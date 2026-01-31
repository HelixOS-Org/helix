//! Key management types and key manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{KeyId, SecurityStrength};

// ============================================================================
// KEY TYPE AND STATE
// ============================================================================

/// Key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Symmetric key
    Symmetric,
    /// Asymmetric public key
    Public,
    /// Asymmetric private key
    Private,
    /// Key pair
    KeyPair,
    /// Session key
    Session,
    /// Master key
    Master,
    /// Derived key
    Derived,
}

impl KeyType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Symmetric => "symmetric",
            Self::Public => "public",
            Self::Private => "private",
            Self::KeyPair => "keypair",
            Self::Session => "session",
            Self::Master => "master",
            Self::Derived => "derived",
        }
    }
}

/// Key state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// Created but not yet active
    Created,
    /// Active
    Active,
    /// Suspended
    Suspended,
    /// Expired
    Expired,
    /// Revoked
    Revoked,
    /// Destroyed
    Destroyed,
}

impl KeyState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
            Self::Destroyed => "destroyed",
        }
    }

    /// Is usable
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Active)
    }
}

// ============================================================================
// KEY INFO
// ============================================================================

/// Key metadata
#[derive(Debug, Clone)]
pub struct KeyInfo {
    /// Key ID
    pub id: KeyId,
    /// Key type
    pub key_type: KeyType,
    /// Algorithm
    pub algorithm: String,
    /// Key size (bits)
    pub size_bits: u32,
    /// State
    pub state: KeyState,
    /// Created timestamp
    pub created_at: u64,
    /// Expires timestamp
    pub expires_at: Option<u64>,
    /// Last used timestamp
    pub last_used: Option<u64>,
    /// Use count
    pub use_count: AtomicU64,
    /// Owner
    pub owner: Option<String>,
    /// Label
    pub label: Option<String>,
}

impl KeyInfo {
    /// Create new key info
    pub fn new(
        id: KeyId,
        key_type: KeyType,
        algorithm: String,
        size_bits: u32,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            key_type,
            algorithm,
            size_bits,
            state: KeyState::Created,
            created_at,
            expires_at: None,
            last_used: None,
            use_count: AtomicU64::new(0),
            owner: None,
            label: None,
        }
    }

    /// Record use
    pub fn record_use(&mut self, timestamp: u64) {
        self.use_count.fetch_add(1, Ordering::Relaxed);
        self.last_used = Some(timestamp);
    }

    /// Get use count
    pub fn use_count(&self) -> u64 {
        self.use_count.load(Ordering::Relaxed)
    }

    /// Is expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.expires_at
            .map(|exp| current_time >= exp)
            .unwrap_or(false)
    }

    /// Get age (in seconds)
    pub fn age(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.created_at)
    }

    /// Security strength
    pub fn strength(&self) -> SecurityStrength {
        // Approximate based on key size and type
        match self.key_type {
            KeyType::Symmetric => SecurityStrength::new(self.size_bits as u16),
            KeyType::Public | KeyType::Private | KeyType::KeyPair => {
                // RSA keys have much lower strength than their size
                if self.algorithm.contains("rsa") {
                    match self.size_bits {
                        1024 => SecurityStrength::new(80),
                        2048 => SecurityStrength::new(112),
                        3072 => SecurityStrength::new(128),
                        4096 => SecurityStrength::new(152),
                        _ => SecurityStrength::new(128),
                    }
                } else if self.algorithm.contains("ec") || self.algorithm.contains("ed25519") {
                    // ECC has similar strength to key size
                    SecurityStrength::new((self.size_bits / 2) as u16)
                } else {
                    SecurityStrength::new(128)
                }
            }
            _ => SecurityStrength::new(self.size_bits as u16),
        }
    }
}

// ============================================================================
// KEY MANAGER
// ============================================================================

/// Key manager
pub struct KeyManager {
    /// Keys
    keys: BTreeMap<KeyId, KeyInfo>,
    /// Next key ID
    next_id: AtomicU64,
    /// Total keys created
    total_created: AtomicU64,
    /// Active keys
    active_count: AtomicU64,
}

impl KeyManager {
    /// Create new key manager
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            total_created: AtomicU64::new(0),
            active_count: AtomicU64::new(0),
        }
    }

    /// Register key
    pub fn register(
        &mut self,
        key_type: KeyType,
        algorithm: String,
        size_bits: u32,
        timestamp: u64,
    ) -> KeyId {
        let id = KeyId::new(self.next_id.fetch_add(1, Ordering::Relaxed));
        let key = KeyInfo::new(id, key_type, algorithm, size_bits, timestamp);
        self.keys.insert(id, key);
        self.total_created.fetch_add(1, Ordering::Relaxed);
        id
    }

    /// Activate key
    pub fn activate(&mut self, id: KeyId) -> bool {
        if let Some(key) = self.keys.get_mut(&id) {
            if key.state == KeyState::Created {
                key.state = KeyState::Active;
                self.active_count.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Revoke key
    pub fn revoke(&mut self, id: KeyId) -> bool {
        if let Some(key) = self.keys.get_mut(&id) {
            if key.state.is_usable() {
                key.state = KeyState::Revoked;
                self.active_count.fetch_sub(1, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Get key
    pub fn get(&self, id: KeyId) -> Option<&KeyInfo> {
        self.keys.get(&id)
    }

    /// Get key mutably
    pub fn get_mut(&mut self, id: KeyId) -> Option<&mut KeyInfo> {
        self.keys.get_mut(&id)
    }

    /// Get active keys
    pub fn active_keys(&self) -> Vec<&KeyInfo> {
        self.keys.values().filter(|k| k.state.is_usable()).collect()
    }

    /// Get expired keys
    pub fn expired_keys(&self, current_time: u64) -> Vec<&KeyInfo> {
        self.keys
            .values()
            .filter(|k| k.is_expired(current_time))
            .collect()
    }

    /// Total created
    pub fn total_created(&self) -> u64 {
        self.total_created.load(Ordering::Relaxed)
    }

    /// Active count
    pub fn active_count(&self) -> u64 {
        self.active_count.load(Ordering::Relaxed)
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}
