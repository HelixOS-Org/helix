//! AEAD (Authenticated Encryption with Associated Data).
//!
//! Provides combined encryption and authentication for
//! tamper-resistant data protection.

use crate::core::types::*;
use crate::crypto::{
    CryptoError, CipherAlgorithm,
    GCM_NONCE_SIZE, GCM_TAG_SIZE,
    CHACHA20_NONCE_SIZE, POLY1305_TAG_SIZE,
    AES_BLOCK_SIZE,
};
use crate::crypto::cipher::{Aes256Context, ChaCha20Context};

// ============================================================================
// Constants
// ============================================================================

/// Maximum AAD (Additional Authenticated Data) size
pub const MAX_AAD_SIZE: usize = 4096;

/// Maximum message size for single AEAD operation
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64 KB

// ============================================================================
// AEAD Result
// ============================================================================

/// Result of AEAD operation.
#[derive(Clone, Copy, Debug)]
pub struct AeadResult {
    /// Success
    pub success: bool,
    /// Output length
    pub output_len: usize,
    /// Error (if any)
    pub error: Option<CryptoError>,
}

impl AeadResult {
    /// Success
    pub fn success(len: usize) -> Self {
        Self {
            success: true,
            output_len: len,
            error: None,
        }
    }
    
    /// Failure
    pub fn failure(error: CryptoError) -> Self {
        Self {
            success: false,
            output_len: 0,
            error: Some(error),
        }
    }
}

// ============================================================================
// GCM Mode (AES-256-GCM)
// ============================================================================

/// GCM multiplication context.
struct GcmMul {
    /// H value (GHASH key)
    h: [u64; 2],
}

impl GcmMul {
    /// Create new context
    fn new() -> Self {
        Self { h: [0; 2] }
    }
    
    /// Initialize with H value
    fn init(&mut self, h_bytes: &[u8; 16]) {
        self.h[0] = u64::from_be_bytes(h_bytes[..8].try_into().unwrap());
        self.h[1] = u64::from_be_bytes(h_bytes[8..].try_into().unwrap());
    }
    
    /// GF(2^128) multiplication
    fn mul(&self, x: &mut [u64; 2]) {
        let mut z = [0u64; 2];
        let mut v = self.h;
        
        for i in 0..2 {
            let mut word = x[i];
            for _ in 0..64 {
                if word & 0x8000000000000000 != 0 {
                    z[0] ^= v[0];
                    z[1] ^= v[1];
                }
                
                // Reduce
                let carry = v[1] & 1;
                v[1] = (v[1] >> 1) | (v[0] << 63);
                v[0] >>= 1;
                
                if carry != 0 {
                    v[0] ^= 0xE100000000000000; // x^128 + x^7 + x^2 + x + 1
                }
                
                word <<= 1;
            }
        }
        
        x[0] = z[0];
        x[1] = z[1];
    }
}

/// GHASH context.
struct GHash {
    /// Multiplication context
    mul: GcmMul,
    /// Current state
    state: [u64; 2],
    /// Buffer for incomplete block
    buffer: [u8; 16],
    /// Buffer length
    buf_len: usize,
}

impl GHash {
    /// Create new context
    fn new() -> Self {
        Self {
            mul: GcmMul::new(),
            state: [0; 2],
            buffer: [0; 16],
            buf_len: 0,
        }
    }
    
    /// Initialize with H
    fn init(&mut self, h: &[u8; 16]) {
        self.mul.init(h);
        self.state = [0; 2];
        self.buf_len = 0;
    }
    
    /// Update with data
    fn update(&mut self, data: &[u8]) {
        let mut offset = 0;
        
        // Handle buffered data
        if self.buf_len > 0 {
            let remaining = 16 - self.buf_len;
            let to_copy = core::cmp::min(remaining, data.len());
            self.buffer[self.buf_len..self.buf_len + to_copy].copy_from_slice(&data[..to_copy]);
            self.buf_len += to_copy;
            offset = to_copy;
            
            if self.buf_len == 16 {
                self.process_block(&self.buffer.clone());
                self.buf_len = 0;
            }
        }
        
        // Process full blocks
        while offset + 16 <= data.len() {
            let block: [u8; 16] = data[offset..offset + 16].try_into().unwrap();
            self.process_block(&block);
            offset += 16;
        }
        
        // Buffer remainder
        if offset < data.len() {
            let remaining = data.len() - offset;
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
            self.buf_len = remaining;
        }
    }
    
    /// Process single block
    fn process_block(&mut self, block: &[u8; 16]) {
        let b0 = u64::from_be_bytes(block[..8].try_into().unwrap());
        let b1 = u64::from_be_bytes(block[8..].try_into().unwrap());
        
        self.state[0] ^= b0;
        self.state[1] ^= b1;
        
        self.mul.mul(&mut self.state);
    }
    
    /// Finalize (handles padding)
    fn finish(&mut self) -> [u8; 16] {
        // Pad and process final block if needed
        if self.buf_len > 0 {
            let mut final_block = [0u8; 16];
            final_block[..self.buf_len].copy_from_slice(&self.buffer[..self.buf_len]);
            self.process_block(&final_block);
        }
        
        // Output state
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&self.state[0].to_be_bytes());
        out[8..].copy_from_slice(&self.state[1].to_be_bytes());
        out
    }
}

/// AES-256-GCM context.
pub struct Aes256Gcm {
    /// AES context for counter mode
    aes: Aes256Context,
    /// H value for GHASH
    h: [u8; 16],
    /// Is initialized
    initialized: bool,
}

impl Aes256Gcm {
    /// Create new context
    pub const fn new() -> Self {
        Self {
            aes: Aes256Context::new(),
            h: [0; 16],
            initialized: false,
        }
    }
    
    /// Initialize with key
    pub fn init(&mut self, key: &[u8]) -> Result<(), CryptoError> {
        self.aes.init(key)?;
        
        // Compute H = AES(K, 0^128)
        let zero = [0u8; 16];
        self.aes.encrypt_block(&zero, &mut self.h)?;
        
        self.initialized = true;
        Ok(())
    }
    
    /// Encrypt and authenticate.
    ///
    /// Output format: ciphertext || tag
    pub fn seal(
        &self,
        nonce: &[u8],
        plaintext: &[u8],
        aad: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CryptoError> {
        if !self.initialized {
            return Err(CryptoError::NotInitialized);
        }
        if nonce.len() != GCM_NONCE_SIZE {
            return Err(CryptoError::InvalidNonce);
        }
        if output.len() < plaintext.len() + GCM_TAG_SIZE {
            return Err(CryptoError::BufferTooSmall);
        }
        
        // Initialize counter
        let mut counter = [0u8; 16];
        counter[..12].copy_from_slice(nonce);
        counter[15] = 1;
        
        // Compute E(K, Y0) for final XOR
        let mut e_y0 = [0u8; 16];
        self.aes.encrypt_block(&counter, &mut e_y0)?;
        
        // Initialize GHASH
        let mut ghash = GHash::new();
        ghash.init(&self.h);
        
        // GHASH AAD
        ghash.update(aad);
        
        // Pad AAD to block boundary
        let aad_pad = (16 - (aad.len() % 16)) % 16;
        if aad_pad > 0 {
            ghash.update(&[0u8; 16][..aad_pad]);
        }
        
        // Encrypt plaintext with CTR mode
        let mut ctr_offset = 0;
        while ctr_offset < plaintext.len() {
            // Increment counter
            increment_counter(&mut counter);
            
            // Encrypt counter
            let mut keystream = [0u8; 16];
            self.aes.encrypt_block(&counter, &mut keystream)?;
            
            // XOR with plaintext
            let block_len = core::cmp::min(16, plaintext.len() - ctr_offset);
            for i in 0..block_len {
                output[ctr_offset + i] = plaintext[ctr_offset + i] ^ keystream[i];
            }
            
            ctr_offset += block_len;
        }
        
        // GHASH ciphertext
        ghash.update(&output[..plaintext.len()]);
        
        // Pad ciphertext to block boundary
        let ct_pad = (16 - (plaintext.len() % 16)) % 16;
        if ct_pad > 0 {
            ghash.update(&[0u8; 16][..ct_pad]);
        }
        
        // GHASH length block: [len(AAD) || len(CT)] in bits
        let mut len_block = [0u8; 16];
        len_block[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
        len_block[8..].copy_from_slice(&((plaintext.len() as u64) * 8).to_be_bytes());
        ghash.update(&len_block);
        
        // Get tag
        let mut tag = ghash.finish();
        
        // XOR with E(K, Y0)
        for i in 0..16 {
            tag[i] ^= e_y0[i];
        }
        
        // Append tag to output
        output[plaintext.len()..plaintext.len() + 16].copy_from_slice(&tag);
        
        Ok(plaintext.len() + GCM_TAG_SIZE)
    }
    
    /// Decrypt and verify.
    ///
    /// Input format: ciphertext || tag
    pub fn open(
        &self,
        nonce: &[u8],
        ciphertext: &[u8],
        aad: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CryptoError> {
        if !self.initialized {
            return Err(CryptoError::NotInitialized);
        }
        if nonce.len() != GCM_NONCE_SIZE {
            return Err(CryptoError::InvalidNonce);
        }
        if ciphertext.len() < GCM_TAG_SIZE {
            return Err(CryptoError::InvalidData);
        }
        
        let ct_len = ciphertext.len() - GCM_TAG_SIZE;
        if output.len() < ct_len {
            return Err(CryptoError::BufferTooSmall);
        }
        
        let (ct, tag) = ciphertext.split_at(ct_len);
        
        // Initialize counter
        let mut counter = [0u8; 16];
        counter[..12].copy_from_slice(nonce);
        counter[15] = 1;
        
        // Compute E(K, Y0)
        let mut e_y0 = [0u8; 16];
        self.aes.encrypt_block(&counter, &mut e_y0)?;
        
        // Initialize GHASH
        let mut ghash = GHash::new();
        ghash.init(&self.h);
        
        // GHASH AAD
        ghash.update(aad);
        let aad_pad = (16 - (aad.len() % 16)) % 16;
        if aad_pad > 0 {
            ghash.update(&[0u8; 16][..aad_pad]);
        }
        
        // GHASH ciphertext
        ghash.update(ct);
        let ct_pad = (16 - (ct_len % 16)) % 16;
        if ct_pad > 0 {
            ghash.update(&[0u8; 16][..ct_pad]);
        }
        
        // GHASH length block
        let mut len_block = [0u8; 16];
        len_block[..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
        len_block[8..].copy_from_slice(&((ct_len as u64) * 8).to_be_bytes());
        ghash.update(&len_block);
        
        // Compute expected tag
        let mut expected_tag = ghash.finish();
        for i in 0..16 {
            expected_tag[i] ^= e_y0[i];
        }
        
        // Verify tag (constant time)
        let mut diff = 0u8;
        for i in 0..16 {
            diff |= tag[i] ^ expected_tag[i];
        }
        
        if diff != 0 {
            return Err(CryptoError::AuthFailed);
        }
        
        // Decrypt
        let mut ctr_offset = 0;
        while ctr_offset < ct_len {
            increment_counter(&mut counter);
            
            let mut keystream = [0u8; 16];
            self.aes.encrypt_block(&counter, &mut keystream)?;
            
            let block_len = core::cmp::min(16, ct_len - ctr_offset);
            for i in 0..block_len {
                output[ctr_offset + i] = ct[ctr_offset + i] ^ keystream[i];
            }
            
            ctr_offset += block_len;
        }
        
        Ok(ct_len)
    }
}

impl Default for Aes256Gcm {
    fn default() -> Self {
        Self::new()
    }
}

/// Increment 128-bit counter.
fn increment_counter(counter: &mut [u8; 16]) {
    for i in (0..16).rev() {
        counter[i] = counter[i].wrapping_add(1);
        if counter[i] != 0 {
            break;
        }
    }
}

// ============================================================================
// Poly1305
// ============================================================================

/// Poly1305 state.
#[derive(Clone, Copy)]
pub struct Poly1305 {
    /// r (clamped)
    r: [u32; 5],
    /// h (accumulator)
    h: [u32; 5],
    /// s (final key)
    s: [u32; 4],
    /// Buffer
    buffer: [u8; 16],
    /// Buffer length
    buf_len: usize,
}

impl Poly1305 {
    /// Create new context
    pub fn new() -> Self {
        Self {
            r: [0; 5],
            h: [0; 5],
            s: [0; 4],
            buffer: [0; 16],
            buf_len: 0,
        }
    }
    
    /// Initialize with key (32 bytes: r || s)
    pub fn init(&mut self, key: &[u8; 32]) {
        // r (clamped)
        let r0 = u32::from_le_bytes(key[0..4].try_into().unwrap()) & 0x0fffffff;
        let r1 = u32::from_le_bytes(key[4..8].try_into().unwrap()) & 0x0ffffffc;
        let r2 = u32::from_le_bytes(key[8..12].try_into().unwrap()) & 0x0ffffffc;
        let r3 = u32::from_le_bytes(key[12..16].try_into().unwrap()) & 0x0ffffffc;
        
        self.r[0] = r0 & 0x03ffffff;
        self.r[1] = ((r0 >> 26) | (r1 << 6)) & 0x03ffffff;
        self.r[2] = ((r1 >> 20) | (r2 << 12)) & 0x03ffffff;
        self.r[3] = ((r2 >> 14) | (r3 << 18)) & 0x03ffffff;
        self.r[4] = r3 >> 8;
        
        // s
        self.s[0] = u32::from_le_bytes(key[16..20].try_into().unwrap());
        self.s[1] = u32::from_le_bytes(key[20..24].try_into().unwrap());
        self.s[2] = u32::from_le_bytes(key[24..28].try_into().unwrap());
        self.s[3] = u32::from_le_bytes(key[28..32].try_into().unwrap());
        
        self.h = [0; 5];
        self.buf_len = 0;
    }
    
    /// Update with data
    pub fn update(&mut self, data: &[u8]) {
        let mut offset = 0;
        
        // Handle buffer
        if self.buf_len > 0 {
            let remaining = 16 - self.buf_len;
            let to_copy = core::cmp::min(remaining, data.len());
            self.buffer[self.buf_len..self.buf_len + to_copy].copy_from_slice(&data[..to_copy]);
            self.buf_len += to_copy;
            offset = to_copy;
            
            if self.buf_len == 16 {
                self.process_block(&self.buffer.clone(), false);
                self.buf_len = 0;
            }
        }
        
        // Full blocks
        while offset + 16 <= data.len() {
            let block: [u8; 16] = data[offset..offset + 16].try_into().unwrap();
            self.process_block(&block, false);
            offset += 16;
        }
        
        // Buffer remainder
        if offset < data.len() {
            let remaining = data.len() - offset;
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
            self.buf_len = remaining;
        }
    }
    
    /// Process one block
    fn process_block(&mut self, block: &[u8; 16], is_final: bool) {
        let hibit = if is_final { 0 } else { 1 << 24 };
        
        // Load block into 130-bit number
        let t0 = u32::from_le_bytes(block[0..4].try_into().unwrap());
        let t1 = u32::from_le_bytes(block[4..8].try_into().unwrap());
        let t2 = u32::from_le_bytes(block[8..12].try_into().unwrap());
        let t3 = u32::from_le_bytes(block[12..16].try_into().unwrap());
        
        self.h[0] = self.h[0].wrapping_add(t0 & 0x03ffffff);
        self.h[1] = self.h[1].wrapping_add(((t0 >> 26) | (t1 << 6)) & 0x03ffffff);
        self.h[2] = self.h[2].wrapping_add(((t1 >> 20) | (t2 << 12)) & 0x03ffffff);
        self.h[3] = self.h[3].wrapping_add(((t2 >> 14) | (t3 << 18)) & 0x03ffffff);
        self.h[4] = self.h[4].wrapping_add((t3 >> 8) | hibit);
        
        // h *= r (mod 2^130 - 5)
        // Simplified multiplication
        let mut d = [0u64; 5];
        for i in 0..5 {
            for j in 0..5 {
                let idx = (i + j) % 5;
                let mult = if i + j >= 5 { 5 } else { 1 };
                d[idx] += (self.h[i] as u64) * (self.r[j] as u64) * mult;
            }
        }
        
        // Reduce
        let mut carry = 0u64;
        for i in 0..5 {
            d[i] += carry;
            self.h[i] = (d[i] & 0x03ffffff) as u32;
            carry = d[i] >> 26;
        }
        self.h[0] = self.h[0].wrapping_add((carry * 5) as u32);
    }
    
    /// Finalize and get tag
    pub fn finish(&mut self) -> [u8; 16] {
        // Process remaining bytes
        if self.buf_len > 0 {
            let mut final_block = [0u8; 16];
            final_block[..self.buf_len].copy_from_slice(&self.buffer[..self.buf_len]);
            final_block[self.buf_len] = 1; // Padding
            self.process_block(&final_block, true);
        }
        
        // Final reduction
        let mut h0 = self.h[0];
        let mut h1 = self.h[1];
        let mut h2 = self.h[2];
        let mut h3 = self.h[3];
        let mut h4 = self.h[4];
        
        let mut c = h0 >> 26; h0 &= 0x03ffffff;
        h1 = h1.wrapping_add(c); c = h1 >> 26; h1 &= 0x03ffffff;
        h2 = h2.wrapping_add(c); c = h2 >> 26; h2 &= 0x03ffffff;
        h3 = h3.wrapping_add(c); c = h3 >> 26; h3 &= 0x03ffffff;
        h4 = h4.wrapping_add(c); c = h4 >> 26; h4 &= 0x03ffffff;
        h0 = h0.wrapping_add(c * 5);
        
        // Compute h + s
        let f0 = (h0 as u64) | ((h1 as u64) << 26);
        let f1 = ((h1 as u64) >> 6) | ((h2 as u64) << 20);
        let f2 = ((h2 as u64) >> 12) | ((h3 as u64) << 14);
        let f3 = ((h3 as u64) >> 18) | ((h4 as u64) << 8);
        
        let mut g0 = f0.wrapping_add(self.s[0] as u64);
        let mut g1 = f1.wrapping_add(self.s[1] as u64).wrapping_add(g0 >> 32);
        let mut g2 = f2.wrapping_add(self.s[2] as u64).wrapping_add(g1 >> 32);
        let g3 = f3.wrapping_add(self.s[3] as u64).wrapping_add(g2 >> 32);
        
        g0 &= 0xffffffff;
        g1 &= 0xffffffff;
        g2 &= 0xffffffff;
        
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&(g0 as u32).to_le_bytes());
        out[4..8].copy_from_slice(&(g1 as u32).to_le_bytes());
        out[8..12].copy_from_slice(&(g2 as u32).to_le_bytes());
        out[12..16].copy_from_slice(&(g3 as u32).to_le_bytes());
        
        out
    }
}

impl Default for Poly1305 {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ChaCha20-Poly1305
// ============================================================================

/// ChaCha20-Poly1305 AEAD.
pub struct ChaCha20Poly1305 {
    /// ChaCha20 context
    chacha: ChaCha20Context,
    /// Is initialized
    initialized: bool,
    /// Key
    key: [u8; 32],
}

impl ChaCha20Poly1305 {
    /// Create new context
    pub const fn new() -> Self {
        Self {
            chacha: ChaCha20Context::new(),
            initialized: false,
            key: [0; 32],
        }
    }
    
    /// Initialize with key
    pub fn init(&mut self, key: &[u8]) -> Result<(), CryptoError> {
        if key.len() != 32 {
            return Err(CryptoError::InvalidKey);
        }
        
        self.key.copy_from_slice(key);
        self.initialized = true;
        
        Ok(())
    }
    
    /// Encrypt and authenticate.
    pub fn seal(
        &mut self,
        nonce: &[u8],
        plaintext: &[u8],
        aad: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CryptoError> {
        if !self.initialized {
            return Err(CryptoError::NotInitialized);
        }
        if nonce.len() != CHACHA20_NONCE_SIZE {
            return Err(CryptoError::InvalidNonce);
        }
        if output.len() < plaintext.len() + POLY1305_TAG_SIZE {
            return Err(CryptoError::BufferTooSmall);
        }
        
        // Initialize ChaCha20
        self.chacha.init(&self.key, nonce)?;
        
        // Generate Poly1305 key (first 32 bytes of keystream)
        let mut poly_key = [0u8; 64];
        self.chacha.block(&mut poly_key);
        let poly_key_bytes: [u8; 32] = poly_key[..32].try_into().unwrap();
        
        // Encrypt
        output[..plaintext.len()].copy_from_slice(plaintext);
        self.chacha.set_counter(1);
        self.chacha.crypt(&mut output[..plaintext.len()]);
        
        // Poly1305 MAC
        let mut poly = Poly1305::new();
        poly.init(&poly_key_bytes);
        
        // AAD
        poly.update(aad);
        let aad_pad = (16 - (aad.len() % 16)) % 16;
        if aad_pad > 0 && aad_pad < 16 {
            poly.update(&[0u8; 16][..aad_pad]);
        }
        
        // Ciphertext
        poly.update(&output[..plaintext.len()]);
        let ct_pad = (16 - (plaintext.len() % 16)) % 16;
        if ct_pad > 0 && ct_pad < 16 {
            poly.update(&[0u8; 16][..ct_pad]);
        }
        
        // Lengths
        poly.update(&(aad.len() as u64).to_le_bytes());
        poly.update(&(plaintext.len() as u64).to_le_bytes());
        
        // Tag
        let tag = poly.finish();
        output[plaintext.len()..plaintext.len() + 16].copy_from_slice(&tag);
        
        Ok(plaintext.len() + POLY1305_TAG_SIZE)
    }
    
    /// Decrypt and verify.
    pub fn open(
        &mut self,
        nonce: &[u8],
        ciphertext: &[u8],
        aad: &[u8],
        output: &mut [u8],
    ) -> Result<usize, CryptoError> {
        if !self.initialized {
            return Err(CryptoError::NotInitialized);
        }
        if nonce.len() != CHACHA20_NONCE_SIZE {
            return Err(CryptoError::InvalidNonce);
        }
        if ciphertext.len() < POLY1305_TAG_SIZE {
            return Err(CryptoError::InvalidData);
        }
        
        let ct_len = ciphertext.len() - POLY1305_TAG_SIZE;
        if output.len() < ct_len {
            return Err(CryptoError::BufferTooSmall);
        }
        
        let (ct, tag) = ciphertext.split_at(ct_len);
        
        // Initialize ChaCha20
        self.chacha.init(&self.key, nonce)?;
        
        // Generate Poly1305 key
        let mut poly_key = [0u8; 64];
        self.chacha.block(&mut poly_key);
        let poly_key_bytes: [u8; 32] = poly_key[..32].try_into().unwrap();
        
        // Verify MAC first
        let mut poly = Poly1305::new();
        poly.init(&poly_key_bytes);
        
        poly.update(aad);
        let aad_pad = (16 - (aad.len() % 16)) % 16;
        if aad_pad > 0 && aad_pad < 16 {
            poly.update(&[0u8; 16][..aad_pad]);
        }
        
        poly.update(ct);
        let ct_pad = (16 - (ct_len % 16)) % 16;
        if ct_pad > 0 && ct_pad < 16 {
            poly.update(&[0u8; 16][..ct_pad]);
        }
        
        poly.update(&(aad.len() as u64).to_le_bytes());
        poly.update(&(ct_len as u64).to_le_bytes());
        
        let expected_tag = poly.finish();
        
        // Constant-time comparison
        let mut diff = 0u8;
        for i in 0..16 {
            diff |= tag[i] ^ expected_tag[i];
        }
        
        if diff != 0 {
            return Err(CryptoError::AuthFailed);
        }
        
        // Decrypt
        output[..ct_len].copy_from_slice(ct);
        self.chacha.set_counter(1);
        self.chacha.crypt(&mut output[..ct_len]);
        
        Ok(ct_len)
    }
}

impl Default for ChaCha20Poly1305 {
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
    fn test_aes256_gcm_roundtrip() {
        let mut gcm = Aes256Gcm::new();
        let key = [0u8; 32];
        gcm.init(&key).unwrap();
        
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World!";
        let aad = b"additional data";
        
        let mut ciphertext = [0u8; 64];
        let len = gcm.seal(&nonce, plaintext, aad, &mut ciphertext).unwrap();
        
        let mut decrypted = [0u8; 64];
        let dec_len = gcm.open(&nonce, &ciphertext[..len], aad, &mut decrypted).unwrap();
        
        assert_eq!(&decrypted[..dec_len], plaintext);
    }
    
    #[test]
    fn test_chacha20_poly1305_roundtrip() {
        let mut aead = ChaCha20Poly1305::new();
        let key = [0u8; 32];
        aead.init(&key).unwrap();
        
        let nonce = [0u8; 12];
        let plaintext = b"Hello, World!";
        let aad = b"additional data";
        
        let mut ciphertext = [0u8; 64];
        let len = aead.seal(&nonce, plaintext, aad, &mut ciphertext).unwrap();
        
        let mut decrypted = [0u8; 64];
        let dec_len = aead.open(&nonce, &ciphertext[..len], aad, &mut decrypted).unwrap();
        
        assert_eq!(&decrypted[..dec_len], plaintext);
    }
    
    #[test]
    fn test_gcm_auth_failure() {
        let mut gcm = Aes256Gcm::new();
        let key = [0u8; 32];
        gcm.init(&key).unwrap();
        
        let nonce = [0u8; 12];
        let plaintext = b"test";
        
        let mut ciphertext = [0u8; 32];
        let len = gcm.seal(&nonce, plaintext, &[], &mut ciphertext).unwrap();
        
        // Corrupt ciphertext
        ciphertext[0] ^= 1;
        
        let mut decrypted = [0u8; 32];
        let result = gcm.open(&nonce, &ciphertext[..len], &[], &mut decrypted);
        
        assert!(matches!(result, Err(CryptoError::AuthFailed)));
    }
    
    #[test]
    fn test_poly1305() {
        let mut poly = Poly1305::new();
        let key = [0u8; 32];
        poly.init(&key);
        poly.update(b"test message");
        let tag = poly.finish();
        
        // Tag should be non-zero
        assert!(tag.iter().any(|&b| b != 0));
    }
}
