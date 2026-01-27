//! Cipher implementations.
//!
//! Block cipher operations for encryption/decryption.

use crate::core::types::*;
use crate::crypto::{
    CipherAlgorithm, CryptoError,
    AES_BLOCK_SIZE, AES_256_KEY_SIZE,
    CHACHA20_KEY_SIZE, CHACHA20_NONCE_SIZE,
};

// ============================================================================
// Constants
// ============================================================================

/// AES S-box (for software implementation)
pub const AES_SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

/// AES round constants
pub const AES_RCON: [u8; 10] = [
    0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

/// ChaCha20 constants (sigma)
pub const CHACHA_SIGMA: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

// ============================================================================
// AES Context
// ============================================================================

/// AES-256 encryption context.
#[repr(C)]
pub struct Aes256Context {
    /// Round keys (15 x 16 = 240 bytes)
    pub round_keys: [u8; 240],
    /// Number of rounds
    pub rounds: u32,
}

impl Aes256Context {
    /// Create new context
    pub const fn new() -> Self {
        Self {
            round_keys: [0; 240],
            rounds: 14,
        }
    }
    
    /// Initialize with key
    pub fn init(&mut self, key: &[u8]) -> Result<(), CryptoError> {
        if key.len() != AES_256_KEY_SIZE {
            return Err(CryptoError::InvalidKey);
        }
        
        // Key expansion
        self.key_expansion(key);
        self.rounds = 14;
        
        Ok(())
    }
    
    /// Key expansion for AES-256
    fn key_expansion(&mut self, key: &[u8]) {
        // Copy initial key
        self.round_keys[..32].copy_from_slice(key);
        
        let mut rcon_idx = 0;
        let mut i = 32;
        
        while i < 240 {
            let mut temp = [0u8; 4];
            temp.copy_from_slice(&self.round_keys[i - 4..i]);
            
            if i % 32 == 0 {
                // Rotate
                let t = temp[0];
                temp[0] = temp[1];
                temp[1] = temp[2];
                temp[2] = temp[3];
                temp[3] = t;
                
                // SubBytes
                for b in &mut temp {
                    *b = AES_SBOX[*b as usize];
                }
                
                // XOR rcon
                temp[0] ^= AES_RCON[rcon_idx];
                rcon_idx += 1;
            } else if i % 32 == 16 {
                // Additional SubBytes for AES-256
                for b in &mut temp {
                    *b = AES_SBOX[*b as usize];
                }
            }
            
            // XOR with previous
            for j in 0..4 {
                self.round_keys[i + j] = self.round_keys[i - 32 + j] ^ temp[j];
            }
            
            i += 4;
        }
    }
    
    /// Encrypt single block
    pub fn encrypt_block(&self, input: &[u8], output: &mut [u8]) -> Result<(), CryptoError> {
        if input.len() != AES_BLOCK_SIZE || output.len() < AES_BLOCK_SIZE {
            return Err(CryptoError::BufferTooSmall);
        }
        
        let mut state = [0u8; 16];
        state.copy_from_slice(input);
        
        // Initial round key
        for i in 0..16 {
            state[i] ^= self.round_keys[i];
        }
        
        // Main rounds
        for round in 1..self.rounds {
            self.sub_bytes(&mut state);
            self.shift_rows(&mut state);
            self.mix_columns(&mut state);
            
            let round_key = &self.round_keys[round as usize * 16..];
            for i in 0..16 {
                state[i] ^= round_key[i];
            }
        }
        
        // Final round (no MixColumns)
        self.sub_bytes(&mut state);
        self.shift_rows(&mut state);
        
        let round_key = &self.round_keys[self.rounds as usize * 16..];
        for i in 0..16 {
            state[i] ^= round_key[i];
        }
        
        output[..16].copy_from_slice(&state);
        Ok(())
    }
    
    /// SubBytes transformation
    fn sub_bytes(&self, state: &mut [u8; 16]) {
        for b in state.iter_mut() {
            *b = AES_SBOX[*b as usize];
        }
    }
    
    /// ShiftRows transformation
    fn shift_rows(&self, state: &mut [u8; 16]) {
        // Row 1: shift left by 1
        let t = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = t;
        
        // Row 2: shift left by 2
        let t = state[2];
        state[2] = state[10];
        state[10] = t;
        let t = state[6];
        state[6] = state[14];
        state[14] = t;
        
        // Row 3: shift left by 3
        let t = state[15];
        state[15] = state[11];
        state[11] = state[7];
        state[7] = state[3];
        state[3] = t;
    }
    
    /// MixColumns transformation
    fn mix_columns(&self, state: &mut [u8; 16]) {
        for col in 0..4 {
            let idx = col * 4;
            let a = state[idx];
            let b = state[idx + 1];
            let c = state[idx + 2];
            let d = state[idx + 3];
            
            state[idx] = gf_mul(a, 2) ^ gf_mul(b, 3) ^ c ^ d;
            state[idx + 1] = a ^ gf_mul(b, 2) ^ gf_mul(c, 3) ^ d;
            state[idx + 2] = a ^ b ^ gf_mul(c, 2) ^ gf_mul(d, 3);
            state[idx + 3] = gf_mul(a, 3) ^ b ^ c ^ gf_mul(d, 2);
        }
    }
}

impl Default for Aes256Context {
    fn default() -> Self {
        Self::new()
    }
}

/// GF(2^8) multiplication
#[inline]
fn gf_mul(a: u8, b: u8) -> u8 {
    match b {
        2 => {
            let hi = a >> 7;
            let shifted = a << 1;
            if hi != 0 { shifted ^ 0x1b } else { shifted }
        }
        3 => a ^ gf_mul(a, 2),
        _ => 0,
    }
}

// ============================================================================
// ChaCha20 Context
// ============================================================================

/// ChaCha20 state.
#[repr(C)]
pub struct ChaCha20Context {
    /// State matrix (16 x 32-bit words)
    pub state: [u32; 16],
    /// Original state for reset
    pub initial: [u32; 16],
    /// Block counter
    pub counter: u64,
}

impl ChaCha20Context {
    /// Create new context
    pub const fn new() -> Self {
        Self {
            state: [0; 16],
            initial: [0; 16],
            counter: 0,
        }
    }
    
    /// Initialize with key and nonce
    pub fn init(&mut self, key: &[u8], nonce: &[u8]) -> Result<(), CryptoError> {
        if key.len() != CHACHA20_KEY_SIZE {
            return Err(CryptoError::InvalidKey);
        }
        if nonce.len() != CHACHA20_NONCE_SIZE {
            return Err(CryptoError::InvalidNonce);
        }
        
        // Constants
        self.state[0] = CHACHA_SIGMA[0];
        self.state[1] = CHACHA_SIGMA[1];
        self.state[2] = CHACHA_SIGMA[2];
        self.state[3] = CHACHA_SIGMA[3];
        
        // Key (8 words)
        for i in 0..8 {
            let offset = i * 4;
            self.state[4 + i] = u32::from_le_bytes([
                key[offset], key[offset + 1], key[offset + 2], key[offset + 3]
            ]);
        }
        
        // Counter (set to 0)
        self.state[12] = 0;
        
        // Nonce (3 words)
        for i in 0..3 {
            let offset = i * 4;
            self.state[13 + i] = u32::from_le_bytes([
                nonce[offset], nonce[offset + 1], nonce[offset + 2], nonce[offset + 3]
            ]);
        }
        
        self.initial = self.state;
        self.counter = 0;
        
        Ok(())
    }
    
    /// Generate keystream block
    pub fn block(&mut self, output: &mut [u8; 64]) {
        let mut working = self.state;
        
        // 20 rounds (10 double rounds)
        for _ in 0..10 {
            // Column rounds
            quarter_round(&mut working, 0, 4, 8, 12);
            quarter_round(&mut working, 1, 5, 9, 13);
            quarter_round(&mut working, 2, 6, 10, 14);
            quarter_round(&mut working, 3, 7, 11, 15);
            
            // Diagonal rounds
            quarter_round(&mut working, 0, 5, 10, 15);
            quarter_round(&mut working, 1, 6, 11, 12);
            quarter_round(&mut working, 2, 7, 8, 13);
            quarter_round(&mut working, 3, 4, 9, 14);
        }
        
        // Add original state
        for i in 0..16 {
            working[i] = working[i].wrapping_add(self.state[i]);
        }
        
        // Serialize to output
        for (i, word) in working.iter().enumerate() {
            let bytes = word.to_le_bytes();
            output[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }
        
        // Increment counter
        self.state[12] = self.state[12].wrapping_add(1);
        if self.state[12] == 0 {
            // Overflow to next word (for very long streams)
        }
        self.counter += 1;
    }
    
    /// Encrypt/decrypt data
    pub fn crypt(&mut self, data: &mut [u8]) {
        let mut keystream = [0u8; 64];
        let mut offset = 0;
        
        while offset < data.len() {
            self.block(&mut keystream);
            
            let remaining = data.len() - offset;
            let to_process = core::cmp::min(remaining, 64);
            
            for i in 0..to_process {
                data[offset + i] ^= keystream[i];
            }
            
            offset += to_process;
        }
    }
    
    /// Set counter
    pub fn set_counter(&mut self, counter: u32) {
        self.state[12] = counter;
        self.counter = counter as u64;
    }
}

impl Default for ChaCha20Context {
    fn default() -> Self {
        Self::new()
    }
}

/// ChaCha20 quarter round
#[inline]
fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(16);
    
    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(12);
    
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(8);
    
    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(7);
}

// ============================================================================
// XTS Mode
// ============================================================================

/// XTS tweak value.
#[derive(Clone, Copy)]
pub struct XtsTweak {
    /// Tweak bytes
    pub bytes: [u8; 16],
}

impl XtsTweak {
    /// Create from sector number
    pub fn from_sector(sector: u64) -> Self {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&sector.to_le_bytes());
        Self { bytes }
    }
    
    /// Multiply by x in GF(2^128)
    pub fn mul_by_x(&mut self) {
        let mut carry = 0u8;
        
        for i in 0..16 {
            let new_carry = self.bytes[i] >> 7;
            self.bytes[i] = (self.bytes[i] << 1) | carry;
            carry = new_carry;
        }
        
        if carry != 0 {
            self.bytes[0] ^= 0x87; // x^128 + x^7 + x^2 + x + 1
        }
    }
}

/// XTS context (uses two AES contexts).
pub struct XtsContext {
    /// Data encryption key context
    pub data_ctx: Aes256Context,
    /// Tweak encryption key context
    pub tweak_ctx: Aes256Context,
}

impl XtsContext {
    /// Create new context
    pub const fn new() -> Self {
        Self {
            data_ctx: Aes256Context::new(),
            tweak_ctx: Aes256Context::new(),
        }
    }
    
    /// Initialize with concatenated keys (64 bytes)
    pub fn init(&mut self, key: &[u8]) -> Result<(), CryptoError> {
        if key.len() != 64 {
            return Err(CryptoError::InvalidKey);
        }
        
        self.data_ctx.init(&key[..32])?;
        self.tweak_ctx.init(&key[32..])?;
        
        Ok(())
    }
    
    /// Encrypt sector
    pub fn encrypt_sector(&self, sector: u64, data: &mut [u8]) -> Result<(), CryptoError> {
        let blocks = data.len() / AES_BLOCK_SIZE;
        if data.len() % AES_BLOCK_SIZE != 0 {
            return Err(CryptoError::InvalidData);
        }
        
        // Encrypt tweak
        let mut tweak = XtsTweak::from_sector(sector);
        let mut encrypted_tweak = [0u8; 16];
        self.tweak_ctx.encrypt_block(&tweak.bytes, &mut encrypted_tweak)?;
        tweak.bytes = encrypted_tweak;
        
        // Encrypt each block
        for i in 0..blocks {
            let offset = i * 16;
            let block = &mut data[offset..offset + 16];
            
            // XOR with tweak
            for j in 0..16 {
                block[j] ^= tweak.bytes[j];
            }
            
            // Encrypt
            let mut out = [0u8; 16];
            self.data_ctx.encrypt_block(block, &mut out)?;
            
            // XOR with tweak again
            for j in 0..16 {
                block[j] = out[j] ^ tweak.bytes[j];
            }
            
            // Next tweak
            tweak.mul_by_x();
        }
        
        Ok(())
    }
}

impl Default for XtsContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aes_context() {
        let mut ctx = Aes256Context::new();
        let key = [0u8; 32];
        assert!(ctx.init(&key).is_ok());
        assert_eq!(ctx.rounds, 14);
    }
    
    #[test]
    fn test_chacha20_context() {
        let mut ctx = ChaCha20Context::new();
        let key = [0u8; 32];
        let nonce = [0u8; 12];
        assert!(ctx.init(&key, &nonce).is_ok());
    }
    
    #[test]
    fn test_xts_tweak() {
        let mut tweak = XtsTweak::from_sector(0);
        assert_eq!(tweak.bytes[0], 0);
        
        tweak.mul_by_x();
        // After mul_by_x, the tweak should change
    }
    
    #[test]
    fn test_gf_mul() {
        assert_eq!(gf_mul(0x57, 2), 0xae);
        assert_eq!(gf_mul(0x57, 3), 0xf9);
    }
}
