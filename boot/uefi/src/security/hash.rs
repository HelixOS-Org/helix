//! Cryptographic Hash Functions
//!
//! SHA-256, SHA-384, SHA-512 implementations for signature verification.

// =============================================================================
// SHA-256
// =============================================================================

/// SHA-256 hash output size
pub const SHA256_OUTPUT_SIZE: usize = 32;

/// SHA-256 block size
pub const SHA256_BLOCK_SIZE: usize = 64;

/// SHA-256 context
#[derive(Clone)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; SHA256_BLOCK_SIZE],
    buffer_len: usize,
    total_len: u64,
}

impl Sha256 {
    /// Initial hash values (first 32 bits of fractional parts of sqrt of first 8 primes)
    const H: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    /// Round constants (first 32 bits of fractional parts of cube roots of first 64 primes)
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    /// Create new SHA-256 context
    pub fn new() -> Self {
        Self {
            state: Self::H,
            buffer: [0u8; SHA256_BLOCK_SIZE],
            buffer_len: 0,
            total_len: 0,
        }
    }

    /// Update hash with data
    pub fn update(&mut self, data: &[u8]) {
        let mut offset = 0;
        self.total_len += data.len() as u64;

        // Fill buffer if not empty
        if self.buffer_len > 0 {
            let space = SHA256_BLOCK_SIZE - self.buffer_len;
            let to_copy = core::cmp::min(space, data.len());
            self.buffer[self.buffer_len..self.buffer_len + to_copy]
                .copy_from_slice(&data[..to_copy]);
            self.buffer_len += to_copy;
            offset = to_copy;

            if self.buffer_len == SHA256_BLOCK_SIZE {
                self.process_block();
                self.buffer_len = 0;
            }
        }

        // Process full blocks
        while offset + SHA256_BLOCK_SIZE <= data.len() {
            self.buffer
                .copy_from_slice(&data[offset..offset + SHA256_BLOCK_SIZE]);
            self.process_block();
            offset += SHA256_BLOCK_SIZE;
        }

        // Store remainder
        if offset < data.len() {
            let remaining = data.len() - offset;
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
            self.buffer_len = remaining;
        }
    }

    /// Finalize and return hash
    pub fn finalize(mut self) -> [u8; SHA256_OUTPUT_SIZE] {
        // Padding
        let bit_len = self.total_len * 8;

        // Append 0x80
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        // If not enough space for length, process this block and start new one
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            self.process_block();
            self.buffer_len = 0;
        }

        // Pad with zeros
        self.buffer[self.buffer_len..56].fill(0);

        // Append length in bits (big-endian)
        self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
        self.process_block();

        // Convert state to bytes
        let mut output = [0u8; SHA256_OUTPUT_SIZE];
        for (i, &s) in self.state.iter().enumerate() {
            output[i * 4..(i + 1) * 4].copy_from_slice(&s.to_be_bytes());
        }

        output
    }

    /// Process a single block
    fn process_block(&mut self) {
        let mut w = [0u32; 64];

        // Prepare message schedule
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                self.buffer[i * 4],
                self.buffer[i * 4 + 1],
                self.buffer[i * 4 + 2],
                self.buffer[i * 4 + 3],
            ]);
        }

        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        // Working variables
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        // Compression function
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        // Update state
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    /// Compute SHA-256 hash of data in one shot
    pub fn digest(data: &[u8]) -> [u8; SHA256_OUTPUT_SIZE] {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize()
    }
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SHA-384 / SHA-512
// =============================================================================

/// SHA-384 hash output size
pub const SHA384_OUTPUT_SIZE: usize = 48;

/// SHA-512 hash output size
pub const SHA512_OUTPUT_SIZE: usize = 64;

/// SHA-512 block size
pub const SHA512_BLOCK_SIZE: usize = 128;

/// SHA-512 context (also used for SHA-384)
#[derive(Clone)]
pub struct Sha512 {
    state: [u64; 8],
    buffer: [u8; SHA512_BLOCK_SIZE],
    buffer_len: usize,
    total_len: u128,
    is_384: bool,
}

impl Sha512 {
    /// Initial hash values for SHA-512
    const H512: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    /// Initial hash values for SHA-384
    const H384: [u64; 8] = [
        0xcbbb9d5dc1059ed8,
        0x629a292a367cd507,
        0x9159015a3070dd17,
        0x152fecd8f70e5939,
        0x67332667ffc00b31,
        0x8eb44a8768581511,
        0xdb0c2e0d64f98fa7,
        0x47b5481dbefa4fa4,
    ];

    /// Round constants
    const K: [u64; 80] = [
        0x428a2f98d728ae22,
        0x7137449123ef65cd,
        0xb5c0fbcfec4d3b2f,
        0xe9b5dba58189dbbc,
        0x3956c25bf348b538,
        0x59f111f1b605d019,
        0x923f82a4af194f9b,
        0xab1c5ed5da6d8118,
        0xd807aa98a3030242,
        0x12835b0145706fbe,
        0x243185be4ee4b28c,
        0x550c7dc3d5ffb4e2,
        0x72be5d74f27b896f,
        0x80deb1fe3b1696b1,
        0x9bdc06a725c71235,
        0xc19bf174cf692694,
        0xe49b69c19ef14ad2,
        0xefbe4786384f25e3,
        0x0fc19dc68b8cd5b5,
        0x240ca1cc77ac9c65,
        0x2de92c6f592b0275,
        0x4a7484aa6ea6e483,
        0x5cb0a9dcbd41fbd4,
        0x76f988da831153b5,
        0x983e5152ee66dfab,
        0xa831c66d2db43210,
        0xb00327c898fb213f,
        0xbf597fc7beef0ee4,
        0xc6e00bf33da88fc2,
        0xd5a79147930aa725,
        0x06ca6351e003826f,
        0x142929670a0e6e70,
        0x27b70a8546d22ffc,
        0x2e1b21385c26c926,
        0x4d2c6dfc5ac42aed,
        0x53380d139d95b3df,
        0x650a73548baf63de,
        0x766a0abb3c77b2a8,
        0x81c2c92e47edaee6,
        0x92722c851482353b,
        0xa2bfe8a14cf10364,
        0xa81a664bbc423001,
        0xc24b8b70d0f89791,
        0xc76c51a30654be30,
        0xd192e819d6ef5218,
        0xd69906245565a910,
        0xf40e35855771202a,
        0x106aa07032bbd1b8,
        0x19a4c116b8d2d0c8,
        0x1e376c085141ab53,
        0x2748774cdf8eeb99,
        0x34b0bcb5e19b48a8,
        0x391c0cb3c5c95a63,
        0x4ed8aa4ae3418acb,
        0x5b9cca4f7763e373,
        0x682e6ff3d6b2b8a3,
        0x748f82ee5defb2fc,
        0x78a5636f43172f60,
        0x84c87814a1f0ab72,
        0x8cc702081a6439ec,
        0x90befffa23631e28,
        0xa4506cebde82bde9,
        0xbef9a3f7b2c67915,
        0xc67178f2e372532b,
        0xca273eceea26619c,
        0xd186b8c721c0c207,
        0xeada7dd6cde0eb1e,
        0xf57d4f7fee6ed178,
        0x06f067aa72176fba,
        0x0a637dc5a2c898a6,
        0x113f9804bef90dae,
        0x1b710b35131c471b,
        0x28db77f523047d84,
        0x32caab7b40c72493,
        0x3c9ebe0a15c9bebc,
        0x431d67c49c100d4c,
        0x4cc5d4becb3e42b6,
        0x597f299cfc657e2a,
        0x5fcb6fab3ad6faec,
        0x6c44198c4a475817,
    ];

    /// Create new SHA-512 context
    pub fn new() -> Self {
        Self {
            state: Self::H512,
            buffer: [0u8; SHA512_BLOCK_SIZE],
            buffer_len: 0,
            total_len: 0,
            is_384: false,
        }
    }

    /// Create new SHA-384 context
    pub fn new_384() -> Self {
        Self {
            state: Self::H384,
            buffer: [0u8; SHA512_BLOCK_SIZE],
            buffer_len: 0,
            total_len: 0,
            is_384: true,
        }
    }

    /// Update hash with data
    pub fn update(&mut self, data: &[u8]) {
        let mut offset = 0;
        self.total_len += data.len() as u128;

        // Fill buffer if not empty
        if self.buffer_len > 0 {
            let space = SHA512_BLOCK_SIZE - self.buffer_len;
            let to_copy = core::cmp::min(space, data.len());
            self.buffer[self.buffer_len..self.buffer_len + to_copy]
                .copy_from_slice(&data[..to_copy]);
            self.buffer_len += to_copy;
            offset = to_copy;

            if self.buffer_len == SHA512_BLOCK_SIZE {
                self.process_block();
                self.buffer_len = 0;
            }
        }

        // Process full blocks
        while offset + SHA512_BLOCK_SIZE <= data.len() {
            self.buffer
                .copy_from_slice(&data[offset..offset + SHA512_BLOCK_SIZE]);
            self.process_block();
            offset += SHA512_BLOCK_SIZE;
        }

        // Store remainder
        if offset < data.len() {
            let remaining = data.len() - offset;
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
            self.buffer_len = remaining;
        }
    }

    /// Finalize and return hash (SHA-512)
    pub fn finalize_512(mut self) -> [u8; SHA512_OUTPUT_SIZE] {
        self.finalize_internal();

        let mut output = [0u8; SHA512_OUTPUT_SIZE];
        for (i, &s) in self.state.iter().enumerate() {
            output[i * 8..(i + 1) * 8].copy_from_slice(&s.to_be_bytes());
        }

        output
    }

    /// Finalize and return hash (SHA-384)
    pub fn finalize_384(mut self) -> [u8; SHA384_OUTPUT_SIZE] {
        self.finalize_internal();

        let mut output = [0u8; SHA384_OUTPUT_SIZE];
        for i in 0..6 {
            output[i * 8..(i + 1) * 8].copy_from_slice(&self.state[i].to_be_bytes());
        }

        output
    }

    fn finalize_internal(&mut self) {
        let bit_len = self.total_len * 8;

        // Append 0x80
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        // If not enough space for length, process this block and start new one
        if self.buffer_len > 112 {
            self.buffer[self.buffer_len..].fill(0);
            self.process_block();
            self.buffer_len = 0;
        }

        // Pad with zeros
        self.buffer[self.buffer_len..112].fill(0);

        // Append length in bits (big-endian, 128-bit)
        self.buffer[112..128].copy_from_slice(&bit_len.to_be_bytes());
        self.process_block();
    }

    /// Process a single block
    fn process_block(&mut self) {
        let mut w = [0u64; 80];

        // Prepare message schedule
        for i in 0..16 {
            w[i] = u64::from_be_bytes([
                self.buffer[i * 8],
                self.buffer[i * 8 + 1],
                self.buffer[i * 8 + 2],
                self.buffer[i * 8 + 3],
                self.buffer[i * 8 + 4],
                self.buffer[i * 8 + 5],
                self.buffer[i * 8 + 6],
                self.buffer[i * 8 + 7],
            ]);
        }

        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        // Working variables
        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        // Compression function
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        // Update state
        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    /// Compute SHA-512 hash of data in one shot
    pub fn digest_512(data: &[u8]) -> [u8; SHA512_OUTPUT_SIZE] {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finalize_512()
    }

    /// Compute SHA-384 hash of data in one shot
    pub fn digest_384(data: &[u8]) -> [u8; SHA384_OUTPUT_SIZE] {
        let mut hasher = Self::new_384();
        hasher.update(data);
        hasher.finalize_384()
    }
}

impl Default for Sha512 {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HMAC
// =============================================================================

/// HMAC-SHA256
pub struct HmacSha256 {
    inner_key: [u8; SHA256_BLOCK_SIZE],
    outer_key: [u8; SHA256_BLOCK_SIZE],
    inner_hasher: Sha256,
}

impl HmacSha256 {
    /// Create new HMAC-SHA256 context
    pub fn new(key: &[u8]) -> Self {
        let mut key_block = [0u8; SHA256_BLOCK_SIZE];

        // If key is longer than block size, hash it
        if key.len() > SHA256_BLOCK_SIZE {
            let hashed = Sha256::digest(key);
            key_block[..SHA256_OUTPUT_SIZE].copy_from_slice(&hashed);
        } else {
            key_block[..key.len()].copy_from_slice(key);
        }

        // Compute inner and outer keys
        let mut inner_key = [0x36u8; SHA256_BLOCK_SIZE];
        let mut outer_key = [0x5cu8; SHA256_BLOCK_SIZE];

        for i in 0..SHA256_BLOCK_SIZE {
            inner_key[i] ^= key_block[i];
            outer_key[i] ^= key_block[i];
        }

        // Initialize inner hasher with inner key
        let mut inner_hasher = Sha256::new();
        inner_hasher.update(&inner_key);

        Self {
            inner_key,
            outer_key,
            inner_hasher,
        }
    }

    /// Update HMAC with data
    pub fn update(&mut self, data: &[u8]) {
        self.inner_hasher.update(data);
    }

    /// Finalize and return MAC
    pub fn finalize(self) -> [u8; SHA256_OUTPUT_SIZE] {
        // Complete inner hash
        let inner_hash = self.inner_hasher.finalize();

        // Outer hash: H(outer_key || inner_hash)
        let mut outer_hasher = Sha256::new();
        outer_hasher.update(&self.outer_key);
        outer_hasher.update(&inner_hash);

        outer_hasher.finalize()
    }

    /// Compute HMAC-SHA256 in one shot
    pub fn mac(key: &[u8], data: &[u8]) -> [u8; SHA256_OUTPUT_SIZE] {
        let mut hmac = Self::new(key);
        hmac.update(data);
        hmac.finalize()
    }
}

// =============================================================================
// HASH ALGORITHM ENUM
// =============================================================================

/// Supported hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HashAlgorithm {
    /// SHA-256
    Sha256 = 0,
    /// SHA-384
    Sha384 = 1,
    /// SHA-512
    Sha512 = 2,
}

impl HashAlgorithm {
    /// Get output size for algorithm
    pub fn output_size(&self) -> usize {
        match self {
            Self::Sha256 => SHA256_OUTPUT_SIZE,
            Self::Sha384 => SHA384_OUTPUT_SIZE,
            Self::Sha512 => SHA512_OUTPUT_SIZE,
        }
    }

    /// Get block size for algorithm
    pub fn block_size(&self) -> usize {
        match self {
            Self::Sha256 => SHA256_BLOCK_SIZE,
            Self::Sha384 | Self::Sha512 => SHA512_BLOCK_SIZE,
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
    fn test_sha256_empty() {
        let hash = Sha256::digest(b"");
        let expected = [
            0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f,
            0xb9, 0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b,
            0x78, 0x52, 0xb8, 0x55,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_sha256_abc() {
        let hash = Sha256::digest(b"abc");
        let expected = [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_sha256_long() {
        let hash = Sha256::digest(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq");
        let expected = [
            0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e,
            0x60, 0x39, 0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4,
            0x19, 0xdb, 0x06, 0xc1,
        ];
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_sha512_empty() {
        let hash = Sha512::digest_512(b"");
        // First 8 bytes of expected hash
        assert_eq!(hash[0], 0xcf);
        assert_eq!(hash[1], 0x83);
        assert_eq!(hash[2], 0xe1);
        assert_eq!(hash[3], 0x35);
    }

    #[test]
    fn test_hmac_sha256() {
        let key = b"key";
        let data = b"The quick brown fox jumps over the lazy dog";
        let mac = HmacSha256::mac(key, data);

        // Expected HMAC
        let expected = [
            0xf7, 0xbc, 0x83, 0xf4, 0x30, 0x53, 0x84, 0x24, 0xb1, 0x32, 0x98, 0xe6, 0xaa, 0x6f,
            0xb1, 0x43, 0xef, 0x4d, 0x59, 0xa1, 0x49, 0x46, 0x17, 0x59, 0x97, 0x47, 0x9d, 0xbc,
            0x2d, 0x1a, 0x3c, 0xd8,
        ];
        assert_eq!(mac, expected);
    }
}
