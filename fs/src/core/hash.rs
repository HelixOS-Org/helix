//! Hash functions and utilities for HelixFS.
//!
//! This module provides various hash implementations used for:
//! - Fast non-cryptographic hashing (XXHash64, CRC32C)
//! - Cryptographic hashing (SHA-256, BLAKE3)
//! - Merkle tree construction
//! - Name hashing for directory entries

use crate::core::types::Hash256;

// ============================================================================
// XXHash64 Implementation
// ============================================================================

/// XXHash64 constants
const PRIME64_1: u64 = 0x9E3779B185EBCA87;
const PRIME64_2: u64 = 0xC2B2AE3D27D4EB4F;
const PRIME64_3: u64 = 0x165667B19E3779F9;
const PRIME64_4: u64 = 0x85EBCA77C2B2AE63;
const PRIME64_5: u64 = 0x27D4EB2F165667C5;

/// Fast non-cryptographic 64-bit hash.
///
/// XXHash64 is extremely fast while providing good distribution.
/// Suitable for hash tables, block checksums, etc.
#[derive(Clone, Debug)]
pub struct XxHash64 {
    acc: [u64; 4],
    buffer: [u8; 32],
    buffer_len: usize,
    total_len: u64,
    seed: u64,
}

impl XxHash64 {
    /// Create a new hasher with seed
    #[inline]
    pub const fn new(seed: u64) -> Self {
        Self {
            acc: [
                seed.wrapping_add(PRIME64_1).wrapping_add(PRIME64_2),
                seed.wrapping_add(PRIME64_2),
                seed,
                seed.wrapping_sub(PRIME64_1),
            ],
            buffer: [0; 32],
            buffer_len: 0,
            total_len: 0,
            seed,
        }
    }

    /// Create a new hasher with seed (alias for new)
    #[inline]
    pub const fn with_seed(seed: u64) -> Self {
        Self::new(seed)
    }

    /// Create hasher with default seed (0)
    #[inline]
    pub const fn default_seed() -> Self {
        Self::new(0)
    }

    /// Update hasher with data
    pub fn update(&mut self, data: &[u8]) {
        let mut input = data;
        self.total_len += data.len() as u64;

        // Fill buffer if we have partial data
        if self.buffer_len > 0 {
            let fill = core::cmp::min(32 - self.buffer_len, input.len());
            self.buffer[self.buffer_len..self.buffer_len + fill].copy_from_slice(&input[..fill]);
            self.buffer_len += fill;
            input = &input[fill..];

            if self.buffer_len == 32 {
                self.process_block(&self.buffer.clone());
                self.buffer_len = 0;
            }
        }

        // Process full blocks
        while input.len() >= 32 {
            let block: [u8; 32] = input[..32].try_into().unwrap();
            self.process_block(&block);
            input = &input[32..];
        }

        // Save remaining data
        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffer_len = input.len();
        }
    }

    /// Write data (alias for update)
    #[inline]
    pub fn write(&mut self, data: &[u8]) {
        self.update(data);
    }

    /// Process a 32-byte block
    #[inline]
    fn process_block(&mut self, block: &[u8; 32]) {
        for i in 0..4 {
            let offset = i * 8;
            let lane = u64::from_le_bytes(block[offset..offset + 8].try_into().unwrap());
            self.acc[i] = Self::round(self.acc[i], lane);
        }
    }

    /// Round function
    #[inline]
    fn round(acc: u64, lane: u64) -> u64 {
        acc.wrapping_add(lane.wrapping_mul(PRIME64_2))
            .rotate_left(31)
            .wrapping_mul(PRIME64_1)
    }

    /// Merge accumulator
    #[inline]
    fn merge_accumulator(acc: u64, val: u64) -> u64 {
        let val = Self::round(0, val);
        acc.bitxor(val)
            .wrapping_mul(PRIME64_1)
            .wrapping_add(PRIME64_4)
    }

    /// Finalize and return hash
    pub fn finish(self) -> u64 {
        let mut hash = if self.total_len >= 32 {
            let mut h = self.acc[0]
                .rotate_left(1)
                .wrapping_add(self.acc[1].rotate_left(7))
                .wrapping_add(self.acc[2].rotate_left(12))
                .wrapping_add(self.acc[3].rotate_left(18));

            h = Self::merge_accumulator(h, self.acc[0]);
            h = Self::merge_accumulator(h, self.acc[1]);
            h = Self::merge_accumulator(h, self.acc[2]);
            h = Self::merge_accumulator(h, self.acc[3]);
            h
        } else {
            self.seed.wrapping_add(PRIME64_5)
        };

        hash = hash.wrapping_add(self.total_len);

        // Process remaining buffer
        let mut i = 0;
        while i + 8 <= self.buffer_len {
            let k = u64::from_le_bytes(self.buffer[i..i + 8].try_into().unwrap());
            hash ^= Self::round(0, k);
            hash = hash
                .rotate_left(27)
                .wrapping_mul(PRIME64_1)
                .wrapping_add(PRIME64_4);
            i += 8;
        }

        if i + 4 <= self.buffer_len {
            let k = u32::from_le_bytes(self.buffer[i..i + 4].try_into().unwrap()) as u64;
            hash ^= k.wrapping_mul(PRIME64_1);
            hash = hash
                .rotate_left(23)
                .wrapping_mul(PRIME64_2)
                .wrapping_add(PRIME64_3);
            i += 4;
        }

        while i < self.buffer_len {
            hash ^= (self.buffer[i] as u64).wrapping_mul(PRIME64_5);
            hash = hash.rotate_left(11).wrapping_mul(PRIME64_1);
            i += 1;
        }

        // Final mix
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(PRIME64_2);
        hash ^= hash >> 29;
        hash = hash.wrapping_mul(PRIME64_3);
        hash ^= hash >> 32;

        hash
    }

    /// One-shot hash of data
    #[inline]
    pub fn hash(data: &[u8]) -> u64 {
        Self::hash_with_seed(data, 0)
    }

    /// One-shot hash with seed
    pub fn hash_with_seed(data: &[u8], seed: u64) -> u64 {
        let mut hasher = Self::new(seed);
        hasher.update(data);
        hasher.finish()
    }
}

impl Default for XxHash64 {
    fn default() -> Self {
        Self::default_seed()
    }
}

// Trait for XOR operation
trait BitXor {
    fn bitxor(self, other: Self) -> Self;
}

impl BitXor for u64 {
    #[inline]
    fn bitxor(self, other: Self) -> Self {
        self ^ other
    }
}

// ============================================================================
// CRC32C Implementation
// ============================================================================

/// CRC32C lookup table (Castagnoli polynomial)
const CRC32C_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let polynomial: u32 = 0x82F63B78; // Castagnoli polynomial (reversed)
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ polynomial;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};

/// CRC32C checksum calculator.
///
/// Uses the Castagnoli polynomial which has better error detection properties
/// and is hardware-accelerated on modern CPUs (though we don't use intrinsics here).
#[derive(Clone, Debug)]
pub struct Crc32c {
    crc: u32,
}

impl Crc32c {
    /// Create new CRC calculator
    #[inline]
    pub const fn new() -> Self {
        Self { crc: !0u32 }
    }

    /// Update with data
    #[inline]
    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let index = ((self.crc ^ byte as u32) & 0xFF) as usize;
            self.crc = (self.crc >> 8) ^ CRC32C_TABLE[index];
        }
    }

    /// Write data (alias for update)
    #[inline]
    pub fn write(&mut self, data: &[u8]) {
        self.update(data);
    }

    /// Finalize and return checksum
    #[inline]
    pub fn finish(self) -> u32 {
        !self.crc
    }

    /// One-shot checksum
    #[inline]
    pub fn hash(data: &[u8]) -> u32 {
        let mut crc = Self::new();
        crc.update(data);
        crc.finish()
    }

    /// Combine two CRC values
    #[inline]
    pub fn combine(crc1: u32, crc2: u32, len2: usize) -> u32 {
        // Simplified combine - for production, use proper GF(2) matrix multiplication
        let mut crc = crc1;
        let zeros = [0u8; 1024];
        let mut remaining = len2;

        // XOR in the zero-extended CRC
        while remaining > 0 {
            let chunk = core::cmp::min(remaining, 1024);
            let mut temp = Self { crc: !crc };
            temp.update(&zeros[..chunk]);
            crc = !temp.crc;
            remaining -= chunk;
        }

        crc ^ crc2
    }
}

impl Default for Crc32c {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SHA-256 Implementation
// ============================================================================

/// SHA-256 round constants
const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Initial hash values for SHA-256
const SHA256_H: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// SHA-256 cryptographic hash.
///
/// Produces a 256-bit (32-byte) hash suitable for integrity verification.
#[derive(Clone, Debug)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    total_len: u64,
}

impl Sha256 {
    /// Create new SHA-256 hasher
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: SHA256_H,
            buffer: [0; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    /// Update with data
    pub fn update(&mut self, data: &[u8]) {
        let mut input = data;
        self.total_len += data.len() as u64;

        // Fill buffer
        if self.buffer_len > 0 {
            let fill = core::cmp::min(64 - self.buffer_len, input.len());
            self.buffer[self.buffer_len..self.buffer_len + fill].copy_from_slice(&input[..fill]);
            self.buffer_len += fill;
            input = &input[fill..];

            if self.buffer_len == 64 {
                self.process_block(&self.buffer.clone());
                self.buffer_len = 0;
            }
        }

        // Process full blocks
        while input.len() >= 64 {
            let block: [u8; 64] = input[..64].try_into().unwrap();
            self.process_block(&block);
            input = &input[64..];
        }

        // Save remaining
        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffer_len = input.len();
        }
    }

    /// Process a 64-byte block
    fn process_block(&mut self, block: &[u8; 64]) {
        // Message schedule
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
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

        // 64 rounds
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[i])
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

    /// Finalize and return hash
    pub fn finish(mut self) -> Hash256 {
        // Padding
        let total_bits = self.total_len * 8;
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        if self.buffer_len > 56 {
            // Need extra block
            for i in self.buffer_len..64 {
                self.buffer[i] = 0;
            }
            self.process_block(&self.buffer.clone());
            self.buffer_len = 0;
        }

        for i in self.buffer_len..56 {
            self.buffer[i] = 0;
        }
        self.buffer[56..64].copy_from_slice(&total_bits.to_be_bytes());
        self.process_block(&self.buffer.clone());

        // Output
        let mut hash = [0u8; 32];
        for i in 0..8 {
            hash[i * 4..i * 4 + 4].copy_from_slice(&self.state[i].to_be_bytes());
        }
        Hash256::from_bytes(hash)
    }

    /// One-shot hash
    #[inline]
    pub fn hash(data: &[u8]) -> Hash256 {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finish()
    }
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// BLAKE3 Implementation (Simplified)
// ============================================================================

/// BLAKE3 constants
const BLAKE3_IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const BLAKE3_MSG_SCHEDULE: [[usize; 16]; 7] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8],
    [3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1],
    [10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6],
    [12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4],
    [9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7],
    [11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13],
];

/// BLAKE3 chunk flags
const BLAKE3_CHUNK_START: u32 = 1;
const BLAKE3_CHUNK_END: u32 = 2;
const BLAKE3_ROOT: u32 = 8;

/// BLAKE3 cryptographic hash.
///
/// BLAKE3 is a modern, fast cryptographic hash function based on BLAKE2.
/// It's optimized for parallel processing and has excellent performance.
#[derive(Clone, Debug)]
pub struct Blake3 {
    key: [u32; 8],
    chunk_state: Blake3ChunkState,
    cv_stack: [[u32; 8]; 54], // Max tree depth
    cv_stack_len: usize,
}

#[derive(Clone, Debug)]
struct Blake3ChunkState {
    cv: [u32; 8],
    chunk_counter: u64,
    buffer: [u8; 64],
    buffer_len: usize,
    blocks_compressed: u8,
    flags: u32,
}

impl Blake3 {
    /// Create new BLAKE3 hasher
    #[inline]
    pub fn new() -> Self {
        Self {
            key: BLAKE3_IV,
            chunk_state: Blake3ChunkState::new(BLAKE3_IV, 0, 0),
            cv_stack: [[0; 8]; 54],
            cv_stack_len: 0,
        }
    }

    /// Create keyed BLAKE3 hasher
    pub fn new_keyed(key: &[u8; 32]) -> Self {
        let mut key_words = [0u32; 8];
        for i in 0..8 {
            key_words[i] = u32::from_le_bytes(key[i * 4..i * 4 + 4].try_into().unwrap());
        }
        Self {
            key: key_words,
            chunk_state: Blake3ChunkState::new(key_words, 0, 0),
            cv_stack: [[0; 8]; 54],
            cv_stack_len: 0,
        }
    }

    /// Update with data
    pub fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            if self.chunk_state.len() == 1024 {
                let chunk_cv = self.chunk_state.output().chaining_value();
                self.push_cv(&chunk_cv);
                self.chunk_state =
                    Blake3ChunkState::new(self.key, self.chunk_state.chunk_counter + 1, 0);
            }

            let want = 1024 - self.chunk_state.len();
            let take = core::cmp::min(want, data.len());
            self.chunk_state.update(&data[..take]);
            data = &data[take..];
        }
    }

    /// Push chaining value to stack, merging as needed
    fn push_cv(&mut self, cv: &[u32; 8]) {
        let mut new_cv = *cv;
        let mut total_chunks = self.chunk_state.chunk_counter + 1;

        while total_chunks & 1 == 0 {
            self.cv_stack_len -= 1;
            new_cv = self.parent_cv(&self.cv_stack[self.cv_stack_len], &new_cv);
            total_chunks >>= 1;
        }

        self.cv_stack[self.cv_stack_len] = new_cv;
        self.cv_stack_len += 1;
    }

    /// Compute parent chaining value
    fn parent_cv(&self, left: &[u32; 8], right: &[u32; 8]) -> [u32; 8] {
        let mut block = [0u8; 64];
        for i in 0..8 {
            block[i * 4..i * 4 + 4].copy_from_slice(&left[i].to_le_bytes());
            block[32 + i * 4..32 + i * 4 + 4].copy_from_slice(&right[i].to_le_bytes());
        }

        let mut cv = self.key;
        Self::compress(&mut cv, &block, 64, 0, 0);
        cv
    }

    /// Finalize and return hash
    pub fn finish(self) -> Hash256 {
        let mut output = self.chunk_state.output();

        // Merge stack
        let mut parent_nodes_remaining = self.cv_stack_len;
        while parent_nodes_remaining > 0 {
            parent_nodes_remaining -= 1;
            output = Blake3Output::parent(
                &self.cv_stack[parent_nodes_remaining],
                &output.chaining_value(),
                self.key,
            );
        }

        // Finalize with ROOT flag
        output.root_hash()
    }

    /// Compression function
    fn compress(cv: &mut [u32; 8], block: &[u8; 64], block_len: u32, counter: u64, flags: u32) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }

        let mut state = [
            cv[0],
            cv[1],
            cv[2],
            cv[3],
            cv[4],
            cv[5],
            cv[6],
            cv[7],
            BLAKE3_IV[0],
            BLAKE3_IV[1],
            BLAKE3_IV[2],
            BLAKE3_IV[3],
            counter as u32,
            (counter >> 32) as u32,
            block_len,
            flags,
        ];

        // 7 rounds
        for s in &BLAKE3_MSG_SCHEDULE {
            // Column step
            Self::g(&mut state, 0, 4, 8, 12, m[s[0]], m[s[1]]);
            Self::g(&mut state, 1, 5, 9, 13, m[s[2]], m[s[3]]);
            Self::g(&mut state, 2, 6, 10, 14, m[s[4]], m[s[5]]);
            Self::g(&mut state, 3, 7, 11, 15, m[s[6]], m[s[7]]);

            // Diagonal step
            Self::g(&mut state, 0, 5, 10, 15, m[s[8]], m[s[9]]);
            Self::g(&mut state, 1, 6, 11, 12, m[s[10]], m[s[11]]);
            Self::g(&mut state, 2, 7, 8, 13, m[s[12]], m[s[13]]);
            Self::g(&mut state, 3, 4, 9, 14, m[s[14]], m[s[15]]);
        }

        // Finalize
        for i in 0..8 {
            cv[i] = state[i] ^ state[i + 8];
        }
    }

    /// G function
    #[inline]
    fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
        state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
        state[d] = (state[d] ^ state[a]).rotate_right(16);
        state[c] = state[c].wrapping_add(state[d]);
        state[b] = (state[b] ^ state[c]).rotate_right(12);
        state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
        state[d] = (state[d] ^ state[a]).rotate_right(8);
        state[c] = state[c].wrapping_add(state[d]);
        state[b] = (state[b] ^ state[c]).rotate_right(7);
    }

    /// One-shot hash
    #[inline]
    pub fn hash(data: &[u8]) -> Hash256 {
        let mut hasher = Self::new();
        hasher.update(data);
        hasher.finish()
    }
}

impl Blake3ChunkState {
    fn new(key: [u32; 8], chunk_counter: u64, flags: u32) -> Self {
        Self {
            cv: key,
            chunk_counter,
            buffer: [0; 64],
            buffer_len: 0,
            blocks_compressed: 0,
            flags,
        }
    }

    fn len(&self) -> usize {
        64 * self.blocks_compressed as usize + self.buffer_len
    }

    fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            if self.buffer_len == 64 {
                let flags = self.flags
                    | if self.blocks_compressed == 0 {
                        BLAKE3_CHUNK_START
                    } else {
                        0
                    };
                Blake3::compress(&mut self.cv, &self.buffer, 64, self.chunk_counter, flags);
                self.blocks_compressed += 1;
                self.buffer_len = 0;
            }

            let take = core::cmp::min(64 - self.buffer_len, data.len());
            self.buffer[self.buffer_len..self.buffer_len + take].copy_from_slice(&data[..take]);
            self.buffer_len += take;
            data = &data[take..];
        }
    }

    fn output(&self) -> Blake3Output {
        let mut flags = self.flags | BLAKE3_CHUNK_END;
        if self.blocks_compressed == 0 {
            flags |= BLAKE3_CHUNK_START;
        }
        Blake3Output {
            input_cv: self.cv,
            block: self.buffer,
            block_len: self.buffer_len as u32,
            counter: self.chunk_counter,
            flags,
        }
    }
}

struct Blake3Output {
    input_cv: [u32; 8],
    block: [u8; 64],
    block_len: u32,
    counter: u64,
    flags: u32,
}

impl Blake3Output {
    fn parent(left_cv: &[u32; 8], right_cv: &[u32; 8], key: [u32; 8]) -> Self {
        let mut block = [0u8; 64];
        for i in 0..8 {
            block[i * 4..i * 4 + 4].copy_from_slice(&left_cv[i].to_le_bytes());
            block[32 + i * 4..32 + i * 4 + 4].copy_from_slice(&right_cv[i].to_le_bytes());
        }
        Self {
            input_cv: key,
            block,
            block_len: 64,
            counter: 0,
            flags: 0,
        }
    }

    fn chaining_value(&self) -> [u32; 8] {
        let mut cv = self.input_cv;
        Blake3::compress(
            &mut cv,
            &self.block,
            self.block_len,
            self.counter,
            self.flags,
        );
        cv
    }

    fn root_hash(&self) -> Hash256 {
        let mut cv = self.input_cv;
        Blake3::compress(
            &mut cv,
            &self.block,
            self.block_len,
            self.counter,
            self.flags | BLAKE3_ROOT,
        );

        let mut hash = [0u8; 32];
        for i in 0..8 {
            hash[i * 4..i * 4 + 4].copy_from_slice(&cv[i].to_le_bytes());
        }
        Hash256::from_bytes(hash)
    }
}

impl Default for Blake3 {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Name Hash (for directory entries)
// ============================================================================

/// Fast name hash for directory entry lookup.
///
/// Uses a variation of FNV-1a optimized for short strings.
#[inline]
pub fn hash_name(name: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in name {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    // Mix to improve distribution
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51afd7ed558ccd);
    hash ^= hash >> 33;
    hash
}

/// Case-insensitive name hash.
#[inline]
pub fn hash_name_casefold(name: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in name {
        // Simple ASCII case folding
        let byte = if byte.is_ascii_uppercase() {
            byte + 32
        } else {
            byte
        };
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51afd7ed558ccd);
    hash ^= hash >> 33;
    hash
}

// ============================================================================
// Merkle Tree Hash Computation
// ============================================================================

/// Compute Merkle root hash for a list of block hashes.
///
/// Uses SHA-256 to combine hashes into a binary tree structure.
pub fn merkle_root(hashes: &[Hash256]) -> Hash256 {
    if hashes.is_empty() {
        return Hash256::ZERO;
    }
    if hashes.len() == 1 {
        return hashes[0];
    }

    // Build tree bottom-up using two buffers
    let mut buffer_a = [Hash256::ZERO; 256];
    let mut buffer_b = [Hash256::ZERO; 256];

    // Copy initial hashes
    let initial_len = hashes.len().min(256);
    buffer_a[..initial_len].copy_from_slice(&hashes[..initial_len]);
    let mut current_len = initial_len;
    let mut use_a = true;

    while current_len > 1 {
        let pairs = (current_len + 1) / 2;
        let (src, dst) = if use_a {
            (&buffer_a, &mut buffer_b)
        } else {
            (&buffer_b, &mut buffer_a)
        };

        for i in 0..pairs {
            let left = &src[i * 2];
            let right = if i * 2 + 1 < current_len {
                &src[i * 2 + 1]
            } else {
                left // Duplicate last if odd
            };

            // Hash(left || right)
            let mut hasher = Sha256::new();
            hasher.update(&left.bytes);
            hasher.update(&right.bytes);
            dst[i] = hasher.finish();
        }

        current_len = pairs;
        use_a = !use_a;
    }

    if use_a {
        buffer_a[0]
    } else {
        buffer_b[0]
    }
}

/// Compute hash of a block for Merkle tree.
#[inline]
pub fn hash_block(data: &[u8]) -> Hash256 {
    Sha256::hash(data)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xxhash64() {
        // Test vectors from XXHash reference implementation
        let hash = XxHash64::hash(b"");
        assert_eq!(hash, 0xef46db3751d8e999);

        let hash = XxHash64::hash(b"a");
        assert_eq!(hash, 0xd24ec4f1a98c6e5b);
    }

    #[test]
    fn test_crc32c() {
        // Test vector: "123456789" should give 0xe3069283
        let crc = Crc32c::hash(b"123456789");
        assert_eq!(crc, 0xe3069283);
    }

    #[test]
    fn test_name_hash() {
        let h1 = hash_name(b"file.txt");
        let h2 = hash_name(b"file.txt");
        let h3 = hash_name(b"file.TXT");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3); // Case sensitive

        // Case-insensitive
        let h4 = hash_name_casefold(b"file.txt");
        let h5 = hash_name_casefold(b"FILE.TXT");
        assert_eq!(h4, h5);
    }
}
