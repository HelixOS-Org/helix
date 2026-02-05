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
        0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a, 0x510e_527f, 0x9b05_688c, 0x1f83_d9ab,
        0x5be0_cd19,
    ];

    /// Round constants (first 32 bits of fractional parts of cube roots of first 64 primes)
    const K: [u32; 64] = [
        0x428a_2f98, 0x7137_4491, 0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4,
        0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be, 0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe,
        0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc, 0x2de9_2c6f,
        0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7,
        0xc6e0_0bf3, 0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc,
        0x5338_0d13, 0x650a_7354, 0x766a_0abb, 0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b,
        0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585, 0x106a_a070, 0x19a4_c116,
        0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
        0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7,
        0xc671_78f2,
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
        for (i, w_item) in w.iter_mut().enumerate().take(16) {
            *w_item = u32::from_be_bytes([
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
        for (i, (&k_val, &w_val)) in Self::K.iter().zip(w.iter()).enumerate().take(64) {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k_val)
                .wrapping_add(w_val);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            let _ = i; // used for iteration count

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
    _is_384: bool,
}

impl Sha512 {
    /// Initial hash values for SHA-512
    const H512: [u64; 8] = [
        0x6a09_e667_f3bc_c908,
        0xbb67_ae85_84ca_a73b,
        0x3c6e_f372_fe94_f82b,
        0xa54f_f53a_5f1d_36f1,
        0x510e_527f_ade6_82d1,
        0x9b05_688c_2b3e_6c1f,
        0x1f83_d9ab_fb41_bd6b,
        0x5be0_cd19_137e_2179,
    ];

    /// Initial hash values for SHA-384
    const H384: [u64; 8] = [
        0xcbbb_9d5d_c105_9ed8,
        0x629a_292a_367c_d507,
        0x9159_015a_3070_dd17,
        0x152f_ecd8_f70e_5939,
        0x6733_2667_ffc0_0b31,
        0x8eb4_4a87_6858_1511,
        0xdb0c_2e0d_64f9_8fa7,
        0x47b5_481d_befa_4fa4,
    ];

    /// Round constants
    const K: [u64; 80] = [
        0x428a_2f98_d728_ae22,
        0x7137_4491_23ef_65cd,
        0xb5c0_fbcf_ec4d_3b2f,
        0xe9b5_dba5_8189_dbbc,
        0x3956_c25b_f348_b538,
        0x59f1_11f1_b605_d019,
        0x923f_82a4_af19_4f9b,
        0xab1c_5ed5_da6d_8118,
        0xd807_aa98_a303_0242,
        0x1283_5b01_4570_6fbe,
        0x2431_85be_4ee4_b28c,
        0x550c_7dc3_d5ff_b4e2,
        0x72be_5d74_f27b_896f,
        0x80de_b1fe_3b16_96b1,
        0x9bdc_06a7_25c7_1235,
        0xc19b_f174_cf69_2694,
        0xe49b_69c1_9ef1_4ad2,
        0xefbe_4786_384f_25e3,
        0x0fc1_9dc6_8b8c_d5b5,
        0x240c_a1cc_77ac_9c65,
        0x2de9_2c6f_592b_0275,
        0x4a74_84aa_6ea6_e483,
        0x5cb0_a9dc_bd41_fbd4,
        0x76f9_88da_8311_53b5,
        0x983e_5152_ee66_dfab,
        0xa831_c66d_2db4_3210,
        0xb003_27c8_98fb_213f,
        0xbf59_7fc7_beef_0ee4,
        0xc6e0_0bf3_3da8_8fc2,
        0xd5a7_9147_930a_a725,
        0x06ca_6351_e003_826f,
        0x1429_2967_0a0e_6e70,
        0x27b7_0a85_46d2_2ffc,
        0x2e1b_2138_5c26_c926,
        0x4d2c_6dfc_5ac4_2aed,
        0x5338_0d13_9d95_b3df,
        0x650a_7354_8baf_63de,
        0x766a_0abb_3c77_b2a8,
        0x81c2_c92e_47ed_aee6,
        0x9272_2c85_1482_353b,
        0xa2bf_e8a1_4cf1_0364,
        0xa81a_664b_bc42_3001,
        0xc24b_8b70_d0f8_9791,
        0xc76c_51a3_0654_be30,
        0xd192_e819_d6ef_5218,
        0xd699_0624_5565_a910,
        0xf40e_3585_5771_202a,
        0x106a_a070_32bb_d1b8,
        0x19a4_c116_b8d2_d0c8,
        0x1e37_6c08_5141_ab53,
        0x2748_774c_df8e_eb99,
        0x34b0_bcb5_e19b_48a8,
        0x391c_0cb3_c5c9_5a63,
        0x4ed8_aa4a_e341_8acb,
        0x5b9c_ca4f_7763_e373,
        0x682e_6ff3_d6b2_b8a3,
        0x748f_82ee_5def_b2fc,
        0x78a5_636f_4317_2f60,
        0x84c8_7814_a1f0_ab72,
        0x8cc7_0208_1a64_39ec,
        0x90be_fffa_2363_1e28,
        0xa450_6ceb_de82_bde9,
        0xbef9_a3f7_b2c6_7915,
        0xc671_78f2_e372_532b,
        0xca27_3ece_ea26_619c,
        0xd186_b8c7_21c0_c207,
        0xeada_7dd6_cde0_eb1e,
        0xf57d_4f7f_ee6e_d178,
        0x06f0_67aa_7217_6fba,
        0x0a63_7dc5_a2c8_98a6,
        0x113f_9804_bef9_0dae,
        0x1b71_0b35_131c_471b,
        0x28db_77f5_2304_7d84,
        0x32ca_ab7b_40c7_2493,
        0x3c9e_be0a_15c9_bebc,
        0x431d_67c4_9c10_0d4c,
        0x4cc5_d4be_cb3e_42b6,
        0x597f_299c_fc65_7e2a,
        0x5fcb_6fab_3ad6_faec,
        0x6c44_198c_4a47_5817,
    ];

    /// Create new SHA-512 context
    pub fn new() -> Self {
        Self {
            state: Self::H512,
            buffer: [0u8; SHA512_BLOCK_SIZE],
            buffer_len: 0,
            total_len: 0,
            _is_384: false,
        }
    }

    /// Create new SHA-384 context
    pub fn new_384() -> Self {
        Self {
            state: Self::H384,
            buffer: [0u8; SHA512_BLOCK_SIZE],
            buffer_len: 0,
            total_len: 0,
            _is_384: true,
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
        for (i, w_item) in w.iter_mut().enumerate().take(16) {
            *w_item = u64::from_be_bytes([
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
        for (i, (&k_val, &w_val)) in Self::K.iter().zip(w.iter()).enumerate().take(80) {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k_val)
                .wrapping_add(w_val);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            let _ = i; // used for iteration count

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
    _inner_key: [u8; SHA256_BLOCK_SIZE],
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
            _inner_key: inner_key,
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
