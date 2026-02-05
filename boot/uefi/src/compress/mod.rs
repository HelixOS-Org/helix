//! Compression and Decompression
//!
//! Compression algorithms for boot payloads and initrd decompression.

use core::fmt;

// =============================================================================
// COMPRESSION TYPES
// =============================================================================

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// No compression
    None,
    /// DEFLATE (zlib/gzip)
    Deflate,
    /// LZMA
    Lzma,
    /// LZ4
    Lz4,
    /// ZSTD
    Zstd,
    /// EFI Compression (Tiano)
    EfiTiano,
    /// EFI Compression (LZMA)
    EfiLzma,
}

impl CompressionType {
    /// Detect compression type from magic bytes
    pub fn detect(data: &[u8]) -> Self {
        if data.len() < 4 {
            return Self::None;
        }

        // gzip
        if data[0] == 0x1F && data[1] == 0x8B {
            return Self::Deflate;
        }

        // LZMA
        if data.len() >= 5 && data[0..5] == [0x5D, 0x00, 0x00, 0x80, 0x00] {
            return Self::Lzma;
        }

        // LZ4
        if data.len() >= 4 && data[0..4] == [0x04, 0x22, 0x4D, 0x18] {
            return Self::Lz4;
        }

        // ZSTD
        if data.len() >= 4 && data[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
            return Self::Zstd;
        }

        // zlib
        if data[0] == 0x78
            && (data[1] == 0x01 || data[1] == 0x5E || data[1] == 0x9C || data[1] == 0xDA)
        {
            return Self::Deflate;
        }

        Self::None
    }
}

// =============================================================================
// RLE COMPRESSION (SIMPLE)
// =============================================================================

/// RLE compression result
pub struct RleResult {
    /// Output size
    pub size: usize,
    /// Compression ratio (0-100)
    pub ratio: u8,
}

/// RLE compress
pub fn rle_compress(input: &[u8], output: &mut [u8]) -> Option<RleResult> {
    if input.is_empty() {
        return None;
    }

    let mut out_pos = 0;
    let mut in_pos = 0;

    while in_pos < input.len() {
        let byte = input[in_pos];
        let mut run_len = 1usize;

        // Count consecutive bytes
        while in_pos + run_len < input.len() && input[in_pos + run_len] == byte && run_len < 255 {
            run_len += 1;
        }

        if run_len >= 4 || byte == 0x00 {
            // Encode as run
            if out_pos + 3 > output.len() {
                return None;
            }

            output[out_pos] = 0x00; // Escape
            output[out_pos + 1] = run_len as u8;
            output[out_pos + 2] = byte;
            out_pos += 3;
        } else {
            // Encode as literals
            for _ in 0..run_len {
                if out_pos >= output.len() {
                    return None;
                }
                output[out_pos] = byte;
                out_pos += 1;
            }
        }

        in_pos += run_len;
    }

    let ratio = if input.len() > 0 {
        ((input.len() - out_pos) * 100 / input.len()) as u8
    } else {
        0
    };

    Some(RleResult {
        size: out_pos,
        ratio,
    })
}

/// RLE decompress
pub fn rle_decompress(input: &[u8], output: &mut [u8]) -> Option<usize> {
    let mut in_pos = 0;
    let mut out_pos = 0;

    while in_pos < input.len() {
        if input[in_pos] == 0x00 {
            // Run encoding
            if in_pos + 2 >= input.len() {
                return None;
            }

            let run_len = input[in_pos + 1] as usize;
            let byte = input[in_pos + 2];

            if out_pos + run_len > output.len() {
                return None;
            }

            for i in 0..run_len {
                output[out_pos + i] = byte;
            }

            out_pos += run_len;
            in_pos += 3;
        } else {
            // Literal
            if out_pos >= output.len() {
                return None;
            }

            output[out_pos] = input[in_pos];
            out_pos += 1;
            in_pos += 1;
        }
    }

    Some(out_pos)
}

// =============================================================================
// LZ77-STYLE COMPRESSION (SIMPLE)
// =============================================================================

/// Maximum match length
const LZ_MAX_MATCH: usize = 258;

/// Maximum offset
const LZ_MAX_OFFSET: usize = 32767;

/// Minimum match length
const LZ_MIN_MATCH: usize = 3;

/// LZ token
#[derive(Debug, Clone, Copy)]
enum _LzToken {
    /// Literal byte
    Literal(u8),
    /// Match (offset, length)
    Match { offset: u16, length: u16 },
}

/// Simple LZ encoder state
pub struct LzEncoder<'a> {
    data: &'a [u8],
    pos: usize,
    window_size: usize,
}

impl<'a> LzEncoder<'a> {
    /// Create new encoder
    pub fn new(data: &'a [u8], window_size: usize) -> Self {
        Self {
            data,
            pos: 0,
            window_size: window_size.min(LZ_MAX_OFFSET),
        }
    }

    /// Find best match at current position
    fn find_match(&self) -> Option<(u16, u16)> {
        if self.pos + LZ_MIN_MATCH > self.data.len() {
            return None;
        }

        let window_start = self.pos.saturating_sub(self.window_size);
        let max_len = (self.data.len() - self.pos).min(LZ_MAX_MATCH);

        if max_len < LZ_MIN_MATCH {
            return None;
        }

        let mut best_offset = 0u16;
        let mut best_length = 0u16;

        for offset in 1..=(self.pos - window_start) {
            let match_start = self.pos - offset;
            let mut length = 0;

            while length < max_len
                && self.data[match_start + length] == self.data[self.pos + length]
            {
                length += 1;
            }

            if length >= LZ_MIN_MATCH && length > best_length as usize {
                best_offset = offset as u16;
                best_length = length as u16;
            }
        }

        if best_length >= LZ_MIN_MATCH as u16 {
            Some((best_offset, best_length))
        } else {
            None
        }
    }

    /// Encode to output buffer
    pub fn encode(&mut self, output: &mut [u8]) -> Result<usize, CompressionError> {
        let mut out_pos = 0;

        while self.pos < self.data.len() {
            if let Some((offset, length)) = self.find_match() {
                // Encode match
                if out_pos + 4 > output.len() {
                    return Err(CompressionError::BufferTooSmall);
                }

                output[out_pos] = 0x00; // Match marker
                output[out_pos + 1] = (offset >> 8) as u8;
                output[out_pos + 2] = offset as u8;
                output[out_pos + 3] = length as u8;
                out_pos += 4;
                self.pos += length as usize;
            } else {
                // Encode literal
                if out_pos >= output.len() {
                    return Err(CompressionError::BufferTooSmall);
                }

                let byte = self.data[self.pos];
                if byte == 0x00 {
                    // Escape literal zero
                    if out_pos + 2 > output.len() {
                        return Err(CompressionError::BufferTooSmall);
                    }
                    output[out_pos] = 0x00;
                    output[out_pos + 1] = 0xFF; // Special: literal zero
                    out_pos += 2;
                } else {
                    output[out_pos] = byte;
                    out_pos += 1;
                }
                self.pos += 1;
            }
        }

        Ok(out_pos)
    }
}

/// LZ decoder
pub fn lz_decode(input: &[u8], output: &mut [u8]) -> Result<usize, CompressionError> {
    let mut in_pos = 0;
    let mut out_pos = 0;

    while in_pos < input.len() {
        if input[in_pos] == 0x00 {
            // Match or escaped zero
            if in_pos + 1 >= input.len() {
                return Err(CompressionError::InvalidData);
            }

            if input[in_pos + 1] == 0xFF {
                // Escaped literal zero
                if out_pos >= output.len() {
                    return Err(CompressionError::BufferTooSmall);
                }
                output[out_pos] = 0x00;
                out_pos += 1;
                in_pos += 2;
            } else {
                // Match
                if in_pos + 3 >= input.len() {
                    return Err(CompressionError::InvalidData);
                }

                let offset = ((input[in_pos + 1] as usize) << 8) | (input[in_pos + 2] as usize);
                let length = input[in_pos + 3] as usize;

                if offset > out_pos || offset == 0 {
                    return Err(CompressionError::InvalidData);
                }

                if out_pos + length > output.len() {
                    return Err(CompressionError::BufferTooSmall);
                }

                let match_start = out_pos - offset;
                for i in 0..length {
                    output[out_pos + i] = output[match_start + i];
                }

                out_pos += length;
                in_pos += 4;
            }
        } else {
            // Literal
            if out_pos >= output.len() {
                return Err(CompressionError::BufferTooSmall);
            }

            output[out_pos] = input[in_pos];
            out_pos += 1;
            in_pos += 1;
        }
    }

    Ok(out_pos)
}

// =============================================================================
// HUFFMAN CODING (SIMPLIFIED)
// =============================================================================

/// Huffman node
#[derive(Clone, Copy)]
struct _HuffmanNode {
    /// Symbol (-1 for internal node)
    symbol: i16,
    /// Frequency/weight
    _weight: u32,
    /// Left child index
    _left: u16,
    /// Right child index
    _right: u16,
}

impl _HuffmanNode {
    const fn _leaf(symbol: u8, weight: u32) -> Self {
        Self {
            symbol: symbol as i16,
            _weight: weight,
            _left: 0,
            _right: 0,
        }
    }

    const fn _internal(weight: u32, left: u16, right: u16) -> Self {
        Self {
            symbol: -1,
            _weight: weight,
            _left: left,
            _right: right,
        }
    }

    fn _is_leaf(&self) -> bool {
        self.symbol >= 0
    }
}

/// Simple Huffman code table
pub struct HuffmanTable {
    /// Codes for each symbol (max 15 bits)
    codes: [u16; 256],
    /// Code lengths
    lengths: [u8; 256],
    /// Number of symbols
    num_symbols: usize,
}

impl HuffmanTable {
    /// Create empty table
    pub const fn new() -> Self {
        Self {
            codes: [0; 256],
            lengths: [0; 256],
            num_symbols: 0,
        }
    }

    /// Build from frequency counts
    pub fn build_from_frequencies(frequencies: &[u32; 256]) -> Self {
        let mut table = Self::new();

        // Count non-zero frequencies
        let mut symbols: [(u8, u32); 256] = [(0, 0); 256];
        let mut count = 0;

        for i in 0..256 {
            if frequencies[i] > 0 {
                symbols[count] = (i as u8, frequencies[i]);
                count += 1;
            }
        }

        if count == 0 {
            return table;
        }

        table.num_symbols = count;

        // Sort by frequency
        for i in 0..count {
            for j in i + 1..count {
                if symbols[j].1 < symbols[i].1 {
                    symbols.swap(i, j);
                }
            }
        }

        // Assign codes (simplified: compute log2 using bit manipulation)
        let max_bits = if count <= 1 {
            1
        } else {
            (32 - (count - 1).leading_zeros()) as u8
        };
        let max_bits = max_bits.max(1).min(15);

        for i in 0..count {
            let sym = symbols[i].0;
            table.codes[sym as usize] = i as u16;
            table.lengths[sym as usize] = max_bits;
        }

        table
    }

    /// Get code for symbol
    pub fn encode(&self, symbol: u8) -> Option<(u16, u8)> {
        let len = self.lengths[symbol as usize];
        if len > 0 {
            Some((self.codes[symbol as usize], len))
        } else {
            None
        }
    }
}

// =============================================================================
// BITSTREAM
// =============================================================================

/// Bit reader for decompression
pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    _bit_pos: u8,
    buffer: u32,
    bits_in_buffer: u8,
}

impl<'a> BitReader<'a> {
    /// Create new bit reader
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            _bit_pos: 0,
            buffer: 0,
            bits_in_buffer: 0,
        }
    }

    /// Read bits
    pub fn read_bits(&mut self, count: u8) -> Option<u32> {
        if count > 24 {
            return None;
        }

        // Refill buffer
        while self.bits_in_buffer < count && self.pos < self.data.len() {
            self.buffer |= (self.data[self.pos] as u32) << self.bits_in_buffer;
            self.pos += 1;
            self.bits_in_buffer += 8;
        }

        if self.bits_in_buffer < count {
            return None;
        }

        let mask = (1u32 << count) - 1;
        let result = self.buffer & mask;
        self.buffer >>= count;
        self.bits_in_buffer -= count;

        Some(result)
    }

    /// Read single bit
    pub fn read_bit(&mut self) -> Option<bool> {
        self.read_bits(1).map(|b| b != 0)
    }

    /// Bytes remaining
    pub fn bytes_remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Align to byte boundary
    pub fn align(&mut self) {
        let discard = self.bits_in_buffer % 8;
        if discard > 0 {
            self.buffer >>= discard;
            self.bits_in_buffer -= discard;
        }
    }
}

/// Bit writer for compression
pub struct BitWriter<'a> {
    data: &'a mut [u8],
    pos: usize,
    buffer: u32,
    bits_in_buffer: u8,
}

impl<'a> BitWriter<'a> {
    /// Create new bit writer
    pub fn new(data: &'a mut [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buffer: 0,
            bits_in_buffer: 0,
        }
    }

    /// Write bits
    pub fn write_bits(&mut self, value: u32, count: u8) -> bool {
        if count > 24 {
            return false;
        }

        let mask = (1u32 << count) - 1;
        self.buffer |= (value & mask) << self.bits_in_buffer;
        self.bits_in_buffer += count;

        // Flush full bytes
        while self.bits_in_buffer >= 8 {
            if self.pos >= self.data.len() {
                return false;
            }
            self.data[self.pos] = (self.buffer & 0xFF) as u8;
            self.pos += 1;
            self.buffer >>= 8;
            self.bits_in_buffer -= 8;
        }

        true
    }

    /// Flush remaining bits
    pub fn flush(&mut self) -> bool {
        if self.bits_in_buffer > 0 {
            if self.pos >= self.data.len() {
                return false;
            }
            self.data[self.pos] = (self.buffer & 0xFF) as u8;
            self.pos += 1;
            self.buffer = 0;
            self.bits_in_buffer = 0;
        }
        true
    }

    /// Get bytes written
    pub fn bytes_written(&self) -> usize {
        self.pos + if self.bits_in_buffer > 0 { 1 } else { 0 }
    }
}

// =============================================================================
// GZIP HEADER
// =============================================================================

/// Gzip header
#[derive(Debug, Clone)]
pub struct GzipHeader {
    /// Compression method (8 = deflate)
    pub method: u8,
    /// Flags
    pub flags: u8,
    /// Modification time
    pub mtime: u32,
    /// Extra flags
    pub xfl: u8,
    /// OS
    pub os: u8,
    /// Original filename
    pub filename: Option<[u8; 256]>,
    /// Filename length
    pub filename_len: usize,
    /// Comment
    pub comment: Option<[u8; 256]>,
    /// Comment length
    pub comment_len: usize,
    /// Header CRC16
    pub header_crc: Option<u16>,
}

impl GzipHeader {
    /// Flag: text
    pub const FLAG_TEXT: u8 = 0x01;
    /// Flag: header CRC
    pub const FLAG_HCRC: u8 = 0x02;
    /// Flag: extra
    pub const FLAG_EXTRA: u8 = 0x04;
    /// Flag: name
    pub const FLAG_NAME: u8 = 0x08;
    /// Flag: comment
    pub const FLAG_COMMENT: u8 = 0x10;

    /// Parse gzip header
    pub fn parse(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 10 {
            return None;
        }

        // Check magic
        if data[0] != 0x1F || data[1] != 0x8B {
            return None;
        }

        let method = data[2];
        let flags = data[3];
        let mtime = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        let xfl = data[8];
        let os = data[9];

        let mut pos = 10;

        // Skip extra field
        if flags & Self::FLAG_EXTRA != 0 {
            if pos + 2 > data.len() {
                return None;
            }
            let xlen = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2 + xlen;
        }

        // Read filename
        let mut filename = None;
        let mut filename_len = 0;
        if flags & Self::FLAG_NAME != 0 {
            let mut buf = [0u8; 256];
            while pos < data.len() && data[pos] != 0 && filename_len < 255 {
                buf[filename_len] = data[pos];
                filename_len += 1;
                pos += 1;
            }
            if pos < data.len() {
                pos += 1; // Skip null terminator
            }
            filename = Some(buf);
        }

        // Read comment
        let mut comment = None;
        let mut comment_len = 0;
        if flags & Self::FLAG_COMMENT != 0 {
            let mut buf = [0u8; 256];
            while pos < data.len() && data[pos] != 0 && comment_len < 255 {
                buf[comment_len] = data[pos];
                comment_len += 1;
                pos += 1;
            }
            if pos < data.len() {
                pos += 1;
            }
            comment = Some(buf);
        }

        // Read header CRC
        let header_crc = if flags & Self::FLAG_HCRC != 0 {
            if pos + 2 > data.len() {
                return None;
            }
            let crc = u16::from_le_bytes([data[pos], data[pos + 1]]);
            pos += 2;
            Some(crc)
        } else {
            None
        };

        Some((
            Self {
                method,
                flags,
                mtime,
                xfl,
                os,
                filename,
                filename_len,
                comment,
                comment_len,
                header_crc,
            },
            pos,
        ))
    }

    /// Get filename as string
    pub fn filename_str(&self) -> Option<&str> {
        self.filename
            .as_ref()
            .and_then(|f| core::str::from_utf8(&f[..self.filename_len]).ok())
    }
}

// =============================================================================
// COMPRESSION ERROR
// =============================================================================

/// Compression error
#[derive(Debug, Clone)]
pub enum CompressionError {
    /// Invalid data
    InvalidData,
    /// Buffer too small
    BufferTooSmall,
    /// Unsupported format
    UnsupportedFormat,
    /// Checksum mismatch
    ChecksumMismatch,
    /// Incomplete data
    IncompleteData,
}

impl fmt::Display for CompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidData => write!(f, "invalid compressed data"),
            Self::BufferTooSmall => write!(f, "output buffer too small"),
            Self::UnsupportedFormat => write!(f, "unsupported compression format"),
            Self::ChecksumMismatch => write!(f, "checksum mismatch"),
            Self::IncompleteData => write!(f, "incomplete compressed data"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_detection() {
        assert_eq!(
            CompressionType::detect(&[0x1F, 0x8B, 0x08, 0x00]),
            CompressionType::Deflate
        );
        assert_eq!(
            CompressionType::detect(&[0x78, 0x9C, 0x00, 0x00]),
            CompressionType::Deflate
        );
        assert_eq!(
            CompressionType::detect(&[0x28, 0xB5, 0x2F, 0xFD]),
            CompressionType::Zstd
        );
        assert_eq!(
            CompressionType::detect(&[0x04, 0x22, 0x4D, 0x18]),
            CompressionType::Lz4
        );
        assert_eq!(
            CompressionType::detect(&[0x00, 0x00, 0x00, 0x00]),
            CompressionType::None
        );
    }

    #[test]
    fn test_rle() {
        let input = [0x41, 0x41, 0x41, 0x41, 0x41, 0x42, 0x43];
        let mut compressed = [0u8; 32];
        let mut decompressed = [0u8; 32];

        let result = rle_compress(&input, &mut compressed).unwrap();
        let size = rle_decompress(&compressed[..result.size], &mut decompressed).unwrap();

        assert_eq!(&decompressed[..size], &input);
    }

    #[test]
    fn test_bit_reader() {
        let data = [0b10110100, 0b11010010];
        let mut reader = BitReader::new(&data);

        assert_eq!(reader.read_bits(4), Some(0b0100));
        assert_eq!(reader.read_bits(4), Some(0b1011));
        assert_eq!(reader.read_bits(4), Some(0b0010));
    }

    #[test]
    fn test_bit_writer() {
        let mut buffer = [0u8; 4];
        let mut writer = BitWriter::new(&mut buffer);

        assert!(writer.write_bits(0b1010, 4));
        assert!(writer.write_bits(0b1100, 4));
        assert!(writer.flush());

        assert_eq!(buffer[0], 0b11001010);
    }
}
