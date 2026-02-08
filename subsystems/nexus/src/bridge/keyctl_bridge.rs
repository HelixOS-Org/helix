// SPDX-License-Identifier: GPL-2.0
//! Bridge keyctl_bridge — kernel key management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Key type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    User,
    Logon,
    Keyring,
    BigKey,
    Asymmetric,
    Encrypted,
    Trusted,
}

/// Key permission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPerm {
    View,
    Read,
    Write,
    Search,
    Link,
    SetAttr,
}

/// Key state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Valid,
    Expired,
    Revoked,
    NegativeInstantiated,
    Uninstantiated,
}

/// Kernel key
#[derive(Debug)]
pub struct KernelKey {
    pub serial: u64,
    pub key_type: KeyType,
    pub state: KeyState,
    pub uid: u32,
    pub gid: u32,
    pub perm: u32,
    pub description_hash: u64,
    pub payload_len: u32,
    pub expiry: u64,
    pub created_at: u64,
    pub ref_count: u32,
}

impl KernelKey {
    pub fn new(serial: u64, kt: KeyType, uid: u32, gid: u32, desc_hash: u64, now: u64) -> Self {
        Self { serial, key_type: kt, state: KeyState::Valid, uid, gid, perm: 0x3f3f0000, description_hash: desc_hash, payload_len: 0, expiry: 0, created_at: now, ref_count: 1 }
    }

    pub fn is_expired(&self, now: u64) -> bool { self.expiry > 0 && now >= self.expiry }
}

/// Stats
#[derive(Debug, Clone)]
pub struct KeyctlBridgeStats {
    pub total_keys: u32,
    pub valid_keys: u32,
    pub expired_keys: u32,
    pub revoked_keys: u32,
}

/// Main bridge keyctl
pub struct BridgeKeyctl {
    keys: BTreeMap<u64, KernelKey>,
    next_serial: u64,
}

impl BridgeKeyctl {
    pub fn new() -> Self { Self { keys: BTreeMap::new(), next_serial: 1 } }

    pub fn add_key(&mut self, kt: KeyType, uid: u32, gid: u32, desc_hash: u64, now: u64) -> u64 {
        let serial = self.next_serial; self.next_serial += 1;
        self.keys.insert(serial, KernelKey::new(serial, kt, uid, gid, desc_hash, now));
        serial
    }

    pub fn revoke(&mut self, serial: u64) {
        if let Some(k) = self.keys.get_mut(&serial) { k.state = KeyState::Revoked; }
    }

    pub fn stats(&self) -> KeyctlBridgeStats {
        let valid = self.keys.values().filter(|k| k.state == KeyState::Valid).count() as u32;
        let expired = self.keys.values().filter(|k| k.state == KeyState::Expired).count() as u32;
        let revoked = self.keys.values().filter(|k| k.state == KeyState::Revoked).count() as u32;
        KeyctlBridgeStats { total_keys: self.keys.len() as u32, valid_keys: valid, expired_keys: expired, revoked_keys: revoked }
    }
}
