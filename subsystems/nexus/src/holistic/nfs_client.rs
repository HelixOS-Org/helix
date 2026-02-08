// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic NFS client — Network filesystem client state tracking
//!
//! Models NFS v4 client with lease management, delegation tracking,
//! callback channel monitoring, and stale handle recovery.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// NFS protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfsVersion {
    V3,
    V40,
    V41,
    V42,
}

/// NFS delegation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfsDelegationType {
    Read,
    Write,
    None,
}

/// NFS client state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfsClientState {
    Active,
    Expired,
    Reclaiming,
    Disconnected,
    Error,
}

/// An NFS delegation.
#[derive(Debug, Clone)]
pub struct NfsDelegation {
    pub delegation_id: u64,
    pub file_handle_hash: u64,
    pub dtype: NfsDelegationType,
    pub stateid: u64,
    pub recalled: bool,
    pub return_pending: bool,
}

impl NfsDelegation {
    pub fn new(delegation_id: u64, file_handle_hash: u64, dtype: NfsDelegationType) -> Self {
        Self {
            delegation_id,
            file_handle_hash,
            dtype,
            stateid: 0,
            recalled: false,
            return_pending: false,
        }
    }
}

/// An NFS mount instance.
#[derive(Debug, Clone)]
pub struct NfsMountInstance {
    pub mount_id: u64,
    pub server_addr_hash: u64,
    pub version: NfsVersion,
    pub state: NfsClientState,
    pub lease_time_sec: u32,
    pub lease_renew_time: u64,
    pub delegations: Vec<NfsDelegation>,
    pub open_files: u64,
    pub rpc_calls: u64,
    pub rpc_errors: u64,
    pub stale_handles: u64,
    pub callback_channel_ok: bool,
}

impl NfsMountInstance {
    pub fn new(mount_id: u64, version: NfsVersion) -> Self {
        Self {
            mount_id,
            server_addr_hash: 0,
            version,
            state: NfsClientState::Active,
            lease_time_sec: 90,
            lease_renew_time: 0,
            delegations: Vec::new(),
            open_files: 0,
            rpc_calls: 0,
            rpc_errors: 0,
            stale_handles: 0,
            callback_channel_ok: true,
        }
    }

    pub fn renew_lease(&mut self, now: u64) {
        self.lease_renew_time = now;
    }

    pub fn is_lease_expired(&self, now: u64) -> bool {
        now > self.lease_renew_time + self.lease_time_sec as u64
    }

    pub fn add_delegation(&mut self, del: NfsDelegation) {
        self.delegations.push(del);
    }

    pub fn recall_delegation(&mut self, delegation_id: u64) -> bool {
        for d in &mut self.delegations {
            if d.delegation_id == delegation_id {
                d.recalled = true;
                return true;
            }
        }
        false
    }

    pub fn error_rate(&self) -> f64 {
        if self.rpc_calls == 0 {
            return 0.0;
        }
        self.rpc_errors as f64 / self.rpc_calls as f64
    }
}

/// Statistics for NFS client.
#[derive(Debug, Clone)]
pub struct NfsClientStats {
    pub total_mounts: u64,
    pub active_delegations: u64,
    pub total_rpc_calls: u64,
    pub total_rpc_errors: u64,
    pub stale_handle_total: u64,
    pub lease_renewals: u64,
}

/// Main holistic NFS client manager.
pub struct HolisticNfsClient {
    pub mounts: BTreeMap<u64, NfsMountInstance>,
    pub next_mount_id: u64,
    pub next_delegation_id: u64,
    pub stats: NfsClientStats,
}

impl HolisticNfsClient {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            next_mount_id: 1,
            next_delegation_id: 1,
            stats: NfsClientStats {
                total_mounts: 0,
                active_delegations: 0,
                total_rpc_calls: 0,
                total_rpc_errors: 0,
                stale_handle_total: 0,
                lease_renewals: 0,
            },
        }
    }

    pub fn create_mount(&mut self, version: NfsVersion) -> u64 {
        let id = self.next_mount_id;
        self.next_mount_id += 1;
        let mount = NfsMountInstance::new(id, version);
        self.mounts.insert(id, mount);
        self.stats.total_mounts += 1;
        id
    }

    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }
}
