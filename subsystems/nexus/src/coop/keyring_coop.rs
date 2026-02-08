// SPDX-License-Identifier: GPL-2.0
//! Coop keyring — cooperative key sharing across processes

extern crate alloc;
use alloc::vec::Vec;

/// Keyring coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringCoopEvent {
    KeyShare,
    KeyInherit,
    KeyRevokeBroadcast,
    KeyringLink,
    KeyringUnlink,
}

/// Keyring coop scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringCoopScope {
    Thread,
    Process,
    Session,
    User,
    UserSession,
}

/// Keyring coop record
#[derive(Debug, Clone)]
pub struct KeyringCoopRecord {
    pub event: KeyringCoopEvent,
    pub scope: KeyringCoopScope,
    pub key_serial: u32,
    pub source_pid: u32,
    pub target_count: u32,
}

impl KeyringCoopRecord {
    pub fn new(event: KeyringCoopEvent, scope: KeyringCoopScope) -> Self {
        Self {
            event,
            scope,
            key_serial: 0,
            source_pid: 0,
            target_count: 0,
        }
    }
}

/// Keyring coop stats
#[derive(Debug, Clone)]
pub struct KeyringCoopStats {
    pub total_events: u64,
    pub shares: u64,
    pub inherits: u64,
    pub revoke_broadcasts: u64,
}

/// Main coop keyring
#[derive(Debug)]
pub struct CoopKeyring {
    pub stats: KeyringCoopStats,
}

impl CoopKeyring {
    pub fn new() -> Self {
        Self {
            stats: KeyringCoopStats {
                total_events: 0,
                shares: 0,
                inherits: 0,
                revoke_broadcasts: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &KeyringCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            KeyringCoopEvent::KeyShare => self.stats.shares += 1,
            KeyringCoopEvent::KeyInherit => self.stats.inherits += 1,
            KeyringCoopEvent::KeyRevokeBroadcast => self.stats.revoke_broadcasts += 1,
            _ => {},
        }
    }
}
