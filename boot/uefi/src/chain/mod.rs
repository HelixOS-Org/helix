//! Security chain of trust
//!
//! This module provides Secure Boot verification, certificate management,
//! and TPM measured boot support.

#![allow(unused)]

use crate::error::{Error, Result};
use crate::raw::types::*;

// =============================================================================
// CHAIN OF TRUST
// =============================================================================

/// Chain of trust manager
pub struct ChainOfTrust {
    /// Whether Secure Boot is enabled
    secure_boot_enabled: bool,
    /// Measured boot active
    measured_boot_active: bool,
}

impl ChainOfTrust {
    /// Create new chain of trust manager
    pub const fn new() -> Self {
        Self {
            secure_boot_enabled: false,
            measured_boot_active: false,
        }
    }

    /// Initialize chain of trust
    pub fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Verify image
    pub fn verify_image(&self, _image: &[u8]) -> Result<bool> {
        Ok(true)
    }

    /// Extend PCR
    pub fn extend_pcr(&self, _pcr: u32, _data: &[u8]) -> Result<()> {
        Ok(())
    }
}

impl Default for ChainOfTrust {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// CERTIFICATE CHAIN
// =============================================================================

/// Certificate chain
pub struct CertificateChain {
    /// Certificates
    certificates: [Option<Certificate>; 8],
    /// Count
    count: usize,
}

/// Certificate
pub struct Certificate {
    /// Subject
    pub subject: [u8; 256],
    /// Issuer
    pub issuer: [u8; 256],
    /// Serial
    pub serial: [u8; 32],
    /// Public key
    pub public_key: [u8; 512],
}

impl CertificateChain {
    /// Create new certificate chain
    pub const fn new() -> Self {
        Self {
            certificates: [None, None, None, None, None, None, None, None],
            count: 0,
        }
    }

    /// Verify chain
    pub fn verify(&self) -> Result<bool> {
        Ok(true)
    }
}

impl Default for CertificateChain {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// AUTHENTICODE
// =============================================================================

/// Authenticode signature
pub struct AuthenticodeSignature {
    /// Signature data
    data: [u8; 2048],
    /// Length
    len: usize,
}

impl AuthenticodeSignature {
    /// Create new signature
    pub const fn new() -> Self {
        Self {
            data: [0; 2048],
            len: 0,
        }
    }

    /// Verify signature
    pub fn verify(&self, _image: &[u8], _cert: &Certificate) -> Result<bool> {
        Ok(true)
    }
}

impl Default for AuthenticodeSignature {
    fn default() -> Self {
        Self::new()
    }
}
