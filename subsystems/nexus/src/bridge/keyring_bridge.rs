// SPDX-License-Identifier: GPL-2.0
//! Bridge keyring_bridge — kernel keyring and key management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    User,
    Logon,
    Keyring,
    BigKey,
    Encrypted,
    Trusted,
    Asymmetric,
    DnsCacheEntry,
    Rxrpc,
}

/// Key state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// Valid and usable
    Valid,
    /// Under construction
    UnderConstruction,
    /// Expired
    Expired,
    /// Revoked
    Revoked,
    /// Dead (garbage collectable)
    Dead,
    /// Negative (lookup failed)
    Negative,
}

/// Key permissions
#[derive(Debug, Clone, Copy)]
pub struct KeyPerm(pub u32);

impl KeyPerm {
    pub const POSSESSOR_VIEW: Self = Self(0x01000000);
    pub const POSSESSOR_READ: Self = Self(0x02000000);
    pub const POSSESSOR_WRITE: Self = Self(0x04000000);
    pub const POSSESSOR_SEARCH: Self = Self(0x08000000);
    pub const POSSESSOR_LINK: Self = Self(0x10000000);
    pub const POSSESSOR_SETATTR: Self = Self(0x20000000);
    pub const USER_VIEW: Self = Self(0x010000);
    pub const USER_READ: Self = Self(0x020000);
    pub const USER_WRITE: Self = Self(0x040000);
    pub const GROUP_VIEW: Self = Self(0x0100);
    pub const OTHER_VIEW: Self = Self(0x01);

    pub fn contains(&self, perm: Self) -> bool {
        self.0 & perm.0 != 0
    }
}

/// A kernel key
#[derive(Debug, Clone)]
pub struct KernelKey {
    pub serial: u32,
    pub key_type: KeyType,
    pub description: String,
    pub state: KeyState,
    pub perm: KeyPerm,
    pub uid: u32,
    pub gid: u32,
    pub payload_size: u32,
    pub ref_count: u32,
    pub expiry: Option<u64>,
    pub created: u64,
    pub last_used: u64,
}

impl KernelKey {
    pub fn new(serial: u32, key_type: KeyType, desc: String) -> Self {
        Self {
            serial, key_type, description: desc,
            state: KeyState::Valid,
            perm: KeyPerm(0x3F3F0000),
            uid: 0, gid: 0,
            payload_size: 0, ref_count: 1,
            expiry: None, created: 0, last_used: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        matches!(self.state, KeyState::Valid)
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.expiry.map(|e| now >= e).unwrap_or(false)
    }

    pub fn idle_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_used)
    }

    pub fn is_keyring(&self) -> bool {
        self.key_type == KeyType::Keyring
    }
}

/// A keyring (collection of keys)
#[derive(Debug)]
pub struct Keyring {
    pub serial: u32,
    pub name: String,
    pub keys: Vec<u32>,
    pub max_keys: u32,
    pub max_bytes: u32,
    pub owner_uid: u32,
}

impl Keyring {
    pub fn new(serial: u32, name: String) -> Self {
        Self {
            serial, name, keys: Vec::new(),
            max_keys: 200, max_bytes: 20000,
            owner_uid: 0,
        }
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    pub fn is_full(&self) -> bool {
        self.keys.len() >= self.max_keys as usize
    }

    pub fn link_key(&mut self, serial: u32) -> bool {
        if self.is_full() { return false; }
        if !self.keys.contains(&serial) {
            self.keys.push(serial);
        }
        true
    }

    pub fn unlink_key(&mut self, serial: u32) -> bool {
        let before = self.keys.len();
        self.keys.retain(|&k| k != serial);
        self.keys.len() < before
    }
}

/// Key operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOp {
    Add,
    Update,
    Revoke,
    Invalidate,
    Read,
    Search,
    Link,
    Unlink,
    SetPerm,
}

/// Key event
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub serial: u32,
    pub op: KeyOp,
    pub pid: u32,
    pub result: bool,
    pub timestamp: u64,
}

/// Keyring bridge stats
#[derive(Debug, Clone)]
pub struct KeyringBridgeStats {
    pub total_keys: u32,
    pub total_keyrings: u32,
    pub expired_keys: u32,
    pub revoked_keys: u32,
    pub total_ops: u64,
    pub total_payload_bytes: u64,
}

/// Main keyring bridge
pub struct BridgeKeyring {
    keys: BTreeMap<u32, KernelKey>,
    keyrings: BTreeMap<u32, Keyring>,
    events: Vec<KeyEvent>,
    max_events: usize,
    next_serial: u32,
    stats: KeyringBridgeStats,
}

impl BridgeKeyring {
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
            keyrings: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            next_serial: 1,
            stats: KeyringBridgeStats {
                total_keys: 0, total_keyrings: 0,
                expired_keys: 0, revoked_keys: 0,
                total_ops: 0, total_payload_bytes: 0,
            },
        }
    }

    pub fn add_key(&mut self, key_type: KeyType, desc: String, payload_size: u32, now: u64) -> u32 {
        let serial = self.next_serial;
        self.next_serial += 1;
        let mut key = KernelKey::new(serial, key_type, desc);
        key.payload_size = payload_size;
        key.created = now;
        key.last_used = now;
        self.stats.total_keys += 1;
        self.stats.total_payload_bytes += payload_size as u64;
        self.keys.insert(serial, key);
        serial
    }

    pub fn create_keyring(&mut self, name: String) -> u32 {
        let serial = self.next_serial;
        self.next_serial += 1;
        let keyring = Keyring::new(serial, name.clone());
        let key = KernelKey::new(serial, KeyType::Keyring, name);
        self.keys.insert(serial, key);
        self.keyrings.insert(serial, keyring);
        self.stats.total_keys += 1;
        self.stats.total_keyrings += 1;
        serial
    }

    pub fn revoke_key(&mut self, serial: u32) -> bool {
        if let Some(key) = self.keys.get_mut(&serial) {
            key.state = KeyState::Revoked;
            self.stats.revoked_keys += 1;
            true
        } else { false }
    }

    pub fn expire_keys(&mut self, now: u64) -> u32 {
        let mut count = 0;
        for key in self.keys.values_mut() {
            if key.state == KeyState::Valid && key.is_expired(now) {
                key.state = KeyState::Expired;
                count += 1;
            }
        }
        self.stats.expired_keys += count;
        count
    }

    pub fn link_to_keyring(&mut self, keyring_serial: u32, key_serial: u32) -> bool {
        if let Some(kr) = self.keyrings.get_mut(&keyring_serial) {
            kr.link_key(key_serial)
        } else { false }
    }

    pub fn search_keyring(&self, keyring_serial: u32, key_type: KeyType, desc: &str) -> Option<u32> {
        let kr = self.keyrings.get(&keyring_serial)?;
        for &serial in &kr.keys {
            if let Some(key) = self.keys.get(&serial) {
                if key.key_type == key_type && key.description == desc && key.is_valid() {
                    return Some(serial);
                }
            }
        }
        None
    }

    pub fn record_event(&mut self, event: KeyEvent) {
        self.stats.total_ops += 1;
        if self.events.len() >= self.max_events { self.events.remove(0); }
        self.events.push(event);
    }

    pub fn stale_keys(&self, now: u64, threshold: u64) -> Vec<u32> {
        self.keys.iter()
            .filter(|(_, k)| k.is_valid() && k.idle_time(now) > threshold)
            .map(|(&s, _)| s)
            .collect()
    }

    pub fn stats(&self) -> &KeyringBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from keyring_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTypeV2 {
    User,
    Logon,
    BigKey,
    Keyring,
    DhCompute,
    Encrypted,
    Trusted,
    Asymmetric,
    Pkcs7,
}

/// Key state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyStateV2 {
    Valid,
    Expired,
    Revoked,
    Dead,
    Negative,
}

/// Key permission bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPermV2(pub u32);

impl KeyPermV2 {
    pub const VIEW: u32 = 1 << 0;
    pub const READ: u32 = 1 << 1;
    pub const WRITE: u32 = 1 << 2;
    pub const SEARCH: u32 = 1 << 3;
    pub const LINK: u32 = 1 << 4;
    pub const SETATTR: u32 = 1 << 5;

    pub fn full() -> Self { Self(0x3F) }
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Key entry
#[derive(Debug)]
pub struct KeyEntryV2 {
    pub serial: u64,
    pub key_type: KeyTypeV2,
    pub state: KeyStateV2,
    pub uid: u32,
    pub gid: u32,
    pub perm: KeyPermV2,
    pub desc_hash: u64,
    pub payload_size: u32,
    pub payload_hash: u64,
    pub created_at: u64,
    pub expires_at: u64,
    pub usage_count: u64,
}

impl KeyEntryV2 {
    pub fn new(serial: u64, ktype: KeyTypeV2, uid: u32, payload_size: u32, now: u64) -> Self {
        Self {
            serial, key_type: ktype, state: KeyStateV2::Valid,
            uid, gid: uid, perm: KeyPermV2::full(), desc_hash: serial,
            payload_size, payload_hash: 0, created_at: now,
            expires_at: 0, usage_count: 0,
        }
    }

    pub fn access(&mut self) { self.usage_count += 1; }
    pub fn revoke(&mut self) { self.state = KeyStateV2::Revoked; }
    pub fn expire(&mut self) { self.state = KeyStateV2::Expired; }

    pub fn check_expiry(&mut self, now: u64) -> bool {
        if self.expires_at > 0 && now >= self.expires_at && self.state == KeyStateV2::Valid {
            self.expire();
            true
        } else { false }
    }

    pub fn is_usable(&self) -> bool { self.state == KeyStateV2::Valid }
}

/// Keyring (container of keys)
#[derive(Debug)]
pub struct KeyringV2 {
    pub serial: u64,
    pub uid: u32,
    pub keys: Vec<u64>,
    pub max_keys: u32,
    pub max_bytes: u64,
    pub current_bytes: u64,
}

impl KeyringV2 {
    pub fn new(serial: u64, uid: u32) -> Self {
        Self { serial, uid, keys: Vec::new(), max_keys: 200, max_bytes: 20000, current_bytes: 0 }
    }

    pub fn add_key(&mut self, key_serial: u64, size: u32) -> bool {
        if self.keys.len() as u32 >= self.max_keys { return false; }
        if self.current_bytes + size as u64 > self.max_bytes { return false; }
        self.keys.push(key_serial);
        self.current_bytes += size as u64;
        true
    }

    pub fn remove_key(&mut self, serial: u64) {
        self.keys.retain(|&k| k != serial);
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct KeyringV2BridgeStats {
    pub total_keys: u32,
    pub total_keyrings: u32,
    pub valid_keys: u32,
    pub expired_keys: u32,
    pub revoked_keys: u32,
    pub total_payload_bytes: u64,
    pub total_accesses: u64,
}

/// Main keyring v2 bridge
pub struct BridgeKeyringV2 {
    keys: BTreeMap<u64, KeyEntryV2>,
    keyrings: BTreeMap<u64, KeyringV2>,
    next_serial: u64,
}

impl BridgeKeyringV2 {
    pub fn new() -> Self { Self { keys: BTreeMap::new(), keyrings: BTreeMap::new(), next_serial: 1 } }

    pub fn add_key(&mut self, ktype: KeyTypeV2, uid: u32, payload_size: u32, now: u64) -> u64 {
        let serial = self.next_serial; self.next_serial += 1;
        self.keys.insert(serial, KeyEntryV2::new(serial, ktype, uid, payload_size, now));
        serial
    }

    pub fn create_keyring(&mut self, uid: u32) -> u64 {
        let serial = self.next_serial; self.next_serial += 1;
        self.keyrings.insert(serial, KeyringV2::new(serial, uid));
        serial
    }

    pub fn revoke(&mut self, serial: u64) {
        if let Some(k) = self.keys.get_mut(&serial) { k.revoke(); }
    }

    pub fn stats(&self) -> KeyringV2BridgeStats {
        let valid = self.keys.values().filter(|k| k.state == KeyStateV2::Valid).count() as u32;
        let expired = self.keys.values().filter(|k| k.state == KeyStateV2::Expired).count() as u32;
        let revoked = self.keys.values().filter(|k| k.state == KeyStateV2::Revoked).count() as u32;
        let bytes: u64 = self.keys.values().map(|k| k.payload_size as u64).sum();
        let accesses: u64 = self.keys.values().map(|k| k.usage_count).sum();
        KeyringV2BridgeStats {
            total_keys: self.keys.len() as u32, total_keyrings: self.keyrings.len() as u32,
            valid_keys: valid, expired_keys: expired, revoked_keys: revoked,
            total_payload_bytes: bytes, total_accesses: accesses,
        }
    }
}

// ============================================================================
// Merged from keyring_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyV3Type {
    User,
    Logon,
    Keyring,
    BigKey,
    Encrypted,
    Trusted,
    Asymmetric,
    DhParam,
}

/// Key operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyV3Op {
    Add,
    Update,
    Revoke,
    Unlink,
    Search,
    Read,
    SetPerm,
    Describe,
    Instantiate,
    Invalidate,
}

/// Key v3 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyV3Result {
    Success,
    NotFound,
    PermissionDenied,
    Expired,
    Revoked,
    QuotaExceeded,
    Error,
}

/// Key v3 record
#[derive(Debug, Clone)]
pub struct KeyV3Record {
    pub op: KeyV3Op,
    pub key_type: KeyV3Type,
    pub result: KeyV3Result,
    pub key_serial: u32,
    pub desc_hash: u64,
    pub payload_size: u32,
}

impl KeyV3Record {
    pub fn new(op: KeyV3Op, key_type: KeyV3Type, desc: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in desc { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { op, key_type, result: KeyV3Result::Success, key_serial: 0, desc_hash: h, payload_size: 0 }
    }
}

/// Key v3 bridge stats
#[derive(Debug, Clone)]
pub struct KeyV3BridgeStats {
    pub total_ops: u64,
    pub adds: u64,
    pub searches: u64,
    pub revocations: u64,
    pub errors: u64,
}

/// Main bridge keyring v3
#[derive(Debug)]
pub struct BridgeKeyringV3 {
    pub stats: KeyV3BridgeStats,
}

impl BridgeKeyringV3 {
    pub fn new() -> Self {
        Self { stats: KeyV3BridgeStats { total_ops: 0, adds: 0, searches: 0, revocations: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &KeyV3Record) {
        self.stats.total_ops += 1;
        match rec.op {
            KeyV3Op::Add | KeyV3Op::Instantiate => self.stats.adds += 1,
            KeyV3Op::Search | KeyV3Op::Read => self.stats.searches += 1,
            KeyV3Op::Revoke | KeyV3Op::Invalidate => self.stats.revocations += 1,
            _ => {}
        }
        if rec.result != KeyV3Result::Success { self.stats.errors += 1; }
    }
}
