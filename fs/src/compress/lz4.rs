//! LZ4 Compression
//!
//! Ultra-fast compression algorithm implementation for HelixFS.
//! Optimized for speed over compression ratio.

use crate::core::error::{HfsError, HfsResult};
use super::{CompressionType, Compressor};

// ============================================================================
// Constants
// ============================================================================

/// Minimum match length
const MIN_MATCH: usize = 4;

/// Maximum match length
const MAX_MATCH_LEN: usize = 65535 + MIN_MATCH;

/// Hash table size (power of 2)
const HASH_SIZE_LOG2: usize = 14;
const HASH_SIZE: usize = 1 << HASH_SIZE_LOG2;

/// Acceleration factor (trade compression ratio for speed)
const ACCELERATION: usize = 1;

/// Maximum input size
const MAX_INPUT_SIZE: usize = 0x7E000000;

/// Last N bytes cannot be match start
const MFLIMIT: usize = 12;

/// End of block safety margin
const LASTLITERALS: usize = 5;

/// Match finding limit
const MATCHLIMIT: usize = MFLIMIT + 1;

// ============================================================================
// LZ4 Hash
// ============================================================================

/// Hash function for match finding.
#[inline(always)]
fn hash(sequence: u32) -> usize {
    // Multiplicative hash
    ((sequence.wrapping_mul(2654435761)) >> (32 - HASH_SIZE_LOG2)) as usize
}

/// Read u32 from bytes (little-endian).
#[inline(always)]
fn read_u32(data: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes([
        data[pos],
        data[pos + 1],
        data[pos + 2],
        data[pos + 3],
    ])
}

/// Read u16 from bytes.
#[inline(always)]
fn read_u16(data: &[u8], pos: usize) -> u16 {
    u16::from_le_bytes([data[pos], data[pos + 1]])
}

/// Write u16 to bytes.
#[inline(always)]
fn write_u16(data: &mut [u8], pos: usize, val: u16) {
    let bytes = val.to_le_bytes();
    data[pos] = bytes[0];
    data[pos + 1] = bytes[1];
}

// ============================================================================
// LZ4 Compressor
// ============================================================================

/// LZ4 compressor state.
pub struct Lz4Compressor {
    /// Hash table for match finding
    hash_table: [u16; HASH_SIZE],
    /// High compression mode
    high_compression: bool,
}

impl Lz4Compressor {
    /// Create new LZ4 compressor
    pub fn new() -> Self {
        Self {
            hash_table: [0; HASH_SIZE],
            high_compression: false,
        }
    }
    
    /// Create high compression compressor
    pub fn new_hc() -> Self {
        Self {
            hash_table: [0; HASH_SIZE],
            high_compression: true,
        }
    }
    
    /// Reset hash table
    fn reset(&mut self) {
        self.hash_table.fill(0);
    }
    
    /// Compress with LZ4 algorithm
    pub fn compress_default(&mut self, input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
        let input_len = input.len();
        
        if input_len == 0 {
            return Ok(0);
        }
        
        if input_len > MAX_INPUT_SIZE {
            return Err(HfsError::TooBig);
        }
        
        let max_output = self.max_compressed_size(input_len);
        if output.len() < max_output {
            return Err(HfsError::BufferTooSmall);
        }
        
        self.reset();
        
        // Input/output positions
        let mut ip = 0; // Input pointer
        let mut op = 0; // Output pointer
        let mut anchor = 0; // Start of literal run
        
        let input_limit = if input_len > MFLIMIT {
            input_len - MFLIMIT
        } else {
            return self.store_literals(input, output);
        };
        
        // First position
        ip += 1;
        
        let step = ACCELERATION;
        
        loop {
            // Find match
            let mut search_match_nb = 1 << 6; // Skip distance increase rate
            let mut forward_ip = ip;
            let mut ref_pos;
            
            // Search for match
            loop {
                let sequence = read_u32(input, forward_ip);
                let h = hash(sequence);
                
                ref_pos = self.hash_table[h] as usize;
                self.hash_table[h] = forward_ip as u16;
                
                ip = forward_ip;
                forward_ip += (search_match_nb >> 6) + step;
                search_match_nb += 1;
                
                if forward_ip > input_limit {
                    break;
                }
                
                // Check match
                if ref_pos + 65535 >= ip
                    && read_u32(input, ref_pos) == sequence
                {
                    // Match found
                    break;
                }
            }
            
            if forward_ip > input_limit {
                break;
            }
            
            // Catch up literals
            let literal_len = ip - anchor;
            
            // Encode literal length
            let token_pos = op;
            op += 1;
            
            if literal_len >= 15 {
                output[token_pos] = 15 << 4;
                let mut remaining = literal_len - 15;
                while remaining >= 255 {
                    output[op] = 255;
                    op += 1;
                    remaining -= 255;
                }
                output[op] = remaining as u8;
                op += 1;
            } else {
                output[token_pos] = (literal_len as u8) << 4;
            }
            
            // Copy literals
            output[op..op + literal_len].copy_from_slice(&input[anchor..anchor + literal_len]);
            op += literal_len;
            
            // Encode offset
            let offset = (ip - ref_pos) as u16;
            write_u16(output, op, offset);
            op += 2;
            
            // Find match length
            let mut match_len = MIN_MATCH;
            let max_match = core::cmp::min(
                input_len - ip - LASTLITERALS,
                MAX_MATCH_LEN - MIN_MATCH,
            );
            
            while match_len < max_match
                && ip + match_len < input_len
                && ref_pos + match_len < input_len
                && input[ip + match_len] == input[ref_pos + match_len]
            {
                match_len += 1;
            }
            
            // Encode match length
            let encoded_match_len = match_len - MIN_MATCH;
            if encoded_match_len >= 15 {
                output[token_pos] |= 15;
                let mut remaining = encoded_match_len - 15;
                while remaining >= 255 {
                    output[op] = 255;
                    op += 1;
                    remaining -= 255;
                }
                output[op] = remaining as u8;
                op += 1;
            } else {
                output[token_pos] |= encoded_match_len as u8;
            }
            
            ip += match_len;
            anchor = ip;
            
            // Update hash table with positions from match
            if ip < input_limit {
                self.hash_table[hash(read_u32(input, ip - 2))] = (ip - 2) as u16;
            }
            
            if ip >= input_limit {
                break;
            }
        }
        
        // Last literals
        let literal_len = input_len - anchor;
        
        if literal_len > 0 {
            // Token
            if literal_len >= 15 {
                output[op] = 15 << 4;
                op += 1;
                let mut remaining = literal_len - 15;
                while remaining >= 255 {
                    output[op] = 255;
                    op += 1;
                    remaining -= 255;
                }
                output[op] = remaining as u8;
                op += 1;
            } else {
                output[op] = (literal_len as u8) << 4;
                op += 1;
            }
            
            // Copy last literals
            output[op..op + literal_len].copy_from_slice(&input[anchor..]);
            op += literal_len;
        }
        
        Ok(op)
    }
    
    /// Store as uncompressed literals
    fn store_literals(&self, input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
        let literal_len = input.len();
        let mut op = 0;
        
        if literal_len >= 15 {
            output[op] = 15 << 4;
            op += 1;
            let mut remaining = literal_len - 15;
            while remaining >= 255 {
                output[op] = 255;
                op += 1;
                remaining -= 255;
            }
            output[op] = remaining as u8;
            op += 1;
        } else {
            output[op] = (literal_len as u8) << 4;
            op += 1;
        }
        
        output[op..op + literal_len].copy_from_slice(input);
        op += literal_len;
        
        Ok(op)
    }
}

impl Default for Lz4Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compressor for Lz4Compressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
        // We need mutable hash table, so create temporary
        let mut temp = Self {
            hash_table: [0; HASH_SIZE],
            high_compression: self.high_compression,
        };
        temp.compress_default(input, output)
    }
    
    fn decompress(&self, input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
        lz4_decompress(input, output)
    }
    
    fn max_compressed_size(&self, input_size: usize) -> usize {
        // LZ4 worst case
        input_size + (input_size / 255) + 16
    }
    
    fn algorithm(&self) -> CompressionType {
        if self.high_compression {
            CompressionType::Lz4Hc
        } else {
            CompressionType::Lz4
        }
    }
}

// ============================================================================
// LZ4 Decompression
// ============================================================================

/// Decompress LZ4 data.
pub fn lz4_decompress(input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
    if input.is_empty() {
        return Ok(0);
    }
    
    let input_len = input.len();
    let output_len = output.len();
    
    let mut ip = 0; // Input position
    let mut op = 0; // Output position
    
    loop {
        // Read token
        if ip >= input_len {
            break;
        }
        
        let token = input[ip];
        ip += 1;
        
        // Literal length
        let mut literal_len = ((token >> 4) & 0x0F) as usize;
        
        if literal_len == 15 {
            loop {
                if ip >= input_len {
                    return Err(HfsError::CorruptedData);
                }
                let b = input[ip] as usize;
                ip += 1;
                literal_len += b;
                if b != 255 {
                    break;
                }
            }
        }
        
        // Copy literals
        if literal_len > 0 {
            if ip + literal_len > input_len || op + literal_len > output_len {
                return Err(HfsError::BufferTooSmall);
            }
            
            output[op..op + literal_len].copy_from_slice(&input[ip..ip + literal_len]);
            ip += literal_len;
            op += literal_len;
        }
        
        // Check if end of block
        if ip >= input_len {
            break;
        }
        
        // Read offset
        if ip + 2 > input_len {
            return Err(HfsError::CorruptedData);
        }
        
        let offset = read_u16(input, ip) as usize;
        ip += 2;
        
        if offset == 0 || offset > op {
            return Err(HfsError::CorruptedData);
        }
        
        // Match length
        let mut match_len = (token & 0x0F) as usize + MIN_MATCH;
        
        if (token & 0x0F) == 15 {
            loop {
                if ip >= input_len {
                    return Err(HfsError::CorruptedData);
                }
                let b = input[ip] as usize;
                ip += 1;
                match_len += b;
                if b != 255 {
                    break;
                }
            }
        }
        
        // Copy match
        if op + match_len > output_len {
            return Err(HfsError::BufferTooSmall);
        }
        
        let match_pos = op - offset;
        
        // Handle overlapping copy
        for i in 0..match_len {
            output[op + i] = output[match_pos + i];
        }
        op += match_len;
    }
    
    Ok(op)
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Quick LZ4 compress (allocates hash table).
pub fn lz4_compress(input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
    let mut compressor = Lz4Compressor::new();
    compressor.compress_default(input, output)
}

/// Calculate max compressed size.
pub fn lz4_max_compressed_size(input_size: usize) -> usize {
    input_size + (input_size / 255) + 16
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lz4_roundtrip() {
        let input = b"Hello, World! Hello, World! Hello, World!";
        let mut compressed = [0u8; 256];
        let mut decompressed = [0u8; 256];
        
        let comp_size = lz4_compress(input, &mut compressed).unwrap();
        let decomp_size = lz4_decompress(&compressed[..comp_size], &mut decompressed).unwrap();
        
        assert_eq!(decomp_size, input.len());
        assert_eq!(&decompressed[..decomp_size], input);
    }
    
    #[test]
    fn test_lz4_compressor() {
        let mut compressor = Lz4Compressor::new();
        
        let input = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let mut compressed = [0u8; 256];
        let mut decompressed = [0u8; 256];
        
        let comp_size = compressor.compress_default(input, &mut compressed).unwrap();
        
        // Should compress well
        assert!(comp_size < input.len());
        
        let decomp_size = lz4_decompress(&compressed[..comp_size], &mut decompressed).unwrap();
        assert_eq!(&decompressed[..decomp_size], input);
    }
    
    #[test]
    fn test_lz4_empty() {
        let input: &[u8] = b"";
        let mut compressed = [0u8; 256];
        
        let comp_size = lz4_compress(input, &mut compressed).unwrap();
        assert_eq!(comp_size, 0);
    }
    
    #[test]
    fn test_lz4_small() {
        let input = b"ABC";
        let mut compressed = [0u8; 256];
        let mut decompressed = [0u8; 256];
        
        let comp_size = lz4_compress(input, &mut compressed).unwrap();
        let decomp_size = lz4_decompress(&compressed[..comp_size], &mut decompressed).unwrap();
        
        assert_eq!(&decompressed[..decomp_size], input);
    }
    
    #[test]
    fn test_lz4_max_size() {
        assert_eq!(lz4_max_compressed_size(1000), 1000 + 3 + 16);
        assert_eq!(lz4_max_compressed_size(0), 16);
    }
}
