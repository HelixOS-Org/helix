//! # Bridge Envelope Manager
//!
//! Syscall envelope: wrapping/unwrapping syscall arguments
//! with metadata for cross-boundary transport:
//! - Typed argument serialization
//! - Capability attachment
//! - Caller context embedding
//! - Integrity sealing
//! - Version negotiation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// ENVELOPE TYPES
// ============================================================================

/// Envelope version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeVersion {
    /// Version 1 — basic
    V1,
    /// Version 2 — with capabilities
    V2,
    /// Version 3 — with integrity seal
    V3,
}

/// Envelope state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeState {
    /// Open for modification
    Open,
    /// Sealed (immutable)
    Sealed,
    /// Delivered to handler
    Delivered,
    /// Response ready
    Responded,
    /// Expired
    Expired,
}

/// Argument type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    /// Integer value
    Integer,
    /// Pointer to buffer
    Pointer,
    /// File descriptor
    FileDescriptor,
    /// String (null-terminated)
    CString,
    /// Struct (sized)
    Struct,
    /// Array
    Array,
    /// Flags/bitfield
    Flags,
}

/// Argument direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgDirection {
    /// Input to kernel
    In,
    /// Output from kernel
    Out,
    /// Both
    InOut,
}

// ============================================================================
// ARGUMENT DESCRIPTOR
// ============================================================================

/// Typed argument descriptor
#[derive(Debug, Clone)]
pub struct ArgDescriptor {
    /// Argument index (0-5 for x86_64)
    pub index: u8,
    /// Type
    pub arg_type: ArgType,
    /// Direction
    pub direction: ArgDirection,
    /// Raw value
    pub value: u64,
    /// Size (for pointer/struct/array)
    pub size: usize,
    /// Is optional
    pub optional: bool,
    /// Validation passed
    pub validated: bool,
}

impl ArgDescriptor {
    pub fn integer(index: u8, value: u64) -> Self {
        Self {
            index,
            arg_type: ArgType::Integer,
            direction: ArgDirection::In,
            value,
            size: 8,
            optional: false,
            validated: false,
        }
    }

    pub fn pointer(index: u8, addr: u64, size: usize, direction: ArgDirection) -> Self {
        Self {
            index,
            arg_type: ArgType::Pointer,
            direction,
            value: addr,
            size,
            optional: false,
            validated: false,
        }
    }

    pub fn fd(index: u8, fd: u64) -> Self {
        Self {
            index,
            arg_type: ArgType::FileDescriptor,
            direction: ArgDirection::In,
            value: fd,
            size: 4,
            optional: false,
            validated: false,
        }
    }

    /// Mark validated
    pub fn mark_validated(&mut self) {
        self.validated = true;
    }

    /// Is pointer type
    pub fn is_pointer(&self) -> bool {
        matches!(
            self.arg_type,
            ArgType::Pointer | ArgType::CString | ArgType::Struct | ArgType::Array
        )
    }
}

// ============================================================================
// CALLER CONTEXT
// ============================================================================

/// Caller context embedded in envelope
#[derive(Debug, Clone)]
pub struct CallerContext {
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Effective capabilities mask
    pub capabilities: u64,
    /// Namespace ID
    pub namespace_id: u32,
    /// CPU on entry
    pub entry_cpu: u32,
    /// Timestamp
    pub entry_ns: u64,
}

// ============================================================================
// ENVELOPE
// ============================================================================

/// Syscall envelope
#[derive(Debug, Clone)]
pub struct SyscallEnvelope {
    /// Unique envelope ID
    pub id: u64,
    /// Version
    pub version: EnvelopeVersion,
    /// State
    pub state: EnvelopeState,
    /// Syscall number
    pub syscall_nr: u32,
    /// Arguments
    pub args: Vec<ArgDescriptor>,
    /// Caller context
    pub caller: CallerContext,
    /// Integrity seal (FNV-1a hash)
    pub seal: u64,
    /// Creation timestamp
    pub created_ns: u64,
    /// Expiry timestamp
    pub expires_ns: u64,
    /// Priority
    pub priority: u8,
    /// Tags (metadata)
    pub tags: BTreeMap<u64, u64>,
    /// Return value (after completion)
    pub return_value: Option<i64>,
}

impl SyscallEnvelope {
    pub fn new(id: u64, syscall_nr: u32, caller: CallerContext, now: u64) -> Self {
        Self {
            id,
            version: EnvelopeVersion::V3,
            state: EnvelopeState::Open,
            syscall_nr,
            args: Vec::new(),
            caller,
            seal: 0,
            created_ns: now,
            expires_ns: now + 30_000_000_000, // 30s default
            priority: 128,
            tags: BTreeMap::new(),
            return_value: None,
        }
    }

    /// Add argument
    pub fn add_arg(&mut self, arg: ArgDescriptor) {
        if self.state == EnvelopeState::Open {
            self.args.push(arg);
        }
    }

    /// Add tag
    pub fn add_tag(&mut self, key: u64, value: u64) {
        if self.state == EnvelopeState::Open {
            self.tags.insert(key, value);
        }
    }

    /// Seal envelope (compute integrity hash)
    pub fn seal(&mut self) {
        if self.state != EnvelopeState::Open {
            return;
        }
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.syscall_nr as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.caller.pid;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.caller.tid;
        hash = hash.wrapping_mul(0x100000001b3);
        for arg in &self.args {
            hash ^= arg.value;
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= arg.index as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.seal = hash;
        self.state = EnvelopeState::Sealed;
    }

    /// Verify seal
    pub fn verify_seal(&self) -> bool {
        if self.state == EnvelopeState::Open {
            return false; // not sealed
        }
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.syscall_nr as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.caller.pid;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.caller.tid;
        hash = hash.wrapping_mul(0x100000001b3);
        for arg in &self.args {
            hash ^= arg.value;
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= arg.index as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash == self.seal
    }

    /// Mark delivered
    pub fn deliver(&mut self) {
        if self.state == EnvelopeState::Sealed {
            self.state = EnvelopeState::Delivered;
        }
    }

    /// Set response
    pub fn respond(&mut self, return_value: i64) {
        if self.state == EnvelopeState::Delivered {
            self.return_value = Some(return_value);
            self.state = EnvelopeState::Responded;
        }
    }

    /// Check expired
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_ns
    }

    /// All args validated?
    pub fn all_validated(&self) -> bool {
        self.args.iter().all(|a| a.validated || a.optional)
    }

    /// Pointer arg count
    pub fn pointer_arg_count(&self) -> usize {
        self.args.iter().filter(|a| a.is_pointer()).count()
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Envelope stats
#[derive(Debug, Clone, Default)]
pub struct BridgeEnvelopeStats {
    /// Envelopes created
    pub created: u64,
    /// Envelopes sealed
    pub sealed: u64,
    /// Envelopes delivered
    pub delivered: u64,
    /// Seal verifications
    pub seal_checks: u64,
    /// Seal failures
    pub seal_failures: u64,
    /// Expired
    pub expired: u64,
}

/// Bridge envelope manager
pub struct BridgeEnvelopeManager {
    /// Active envelopes
    active: BTreeMap<u64, SyscallEnvelope>,
    /// Next ID
    next_id: u64,
    /// Max active
    max_active: usize,
    /// Stats
    stats: BridgeEnvelopeStats,
}

impl BridgeEnvelopeManager {
    pub fn new(max_active: usize) -> Self {
        Self {
            active: BTreeMap::new(),
            next_id: 1,
            max_active,
            stats: BridgeEnvelopeStats::default(),
        }
    }

    /// Create envelope
    pub fn create(&mut self, syscall_nr: u32, caller: CallerContext, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let env = SyscallEnvelope::new(id, syscall_nr, caller, now);
        if self.active.len() >= self.max_active {
            self.evict_oldest();
        }
        self.active.insert(id, env);
        self.stats.created += 1;
        id
    }

    /// Get envelope
    pub fn get(&self, id: u64) -> Option<&SyscallEnvelope> {
        self.active.get(&id)
    }

    /// Get mutable
    pub fn get_mut(&mut self, id: u64) -> Option<&mut SyscallEnvelope> {
        self.active.get_mut(&id)
    }

    /// Seal and verify
    pub fn seal_and_verify(&mut self, id: u64) -> bool {
        if let Some(env) = self.active.get_mut(&id) {
            env.seal();
            self.stats.sealed += 1;
            self.stats.seal_checks += 1;
            if env.verify_seal() {
                true
            } else {
                self.stats.seal_failures += 1;
                false
            }
        } else {
            false
        }
    }

    /// Deliver envelope
    pub fn deliver(&mut self, id: u64) -> bool {
        if let Some(env) = self.active.get_mut(&id) {
            env.deliver();
            self.stats.delivered += 1;
            true
        } else {
            false
        }
    }

    /// Complete with response
    pub fn complete(&mut self, id: u64, return_value: i64) -> bool {
        if let Some(env) = self.active.get_mut(&id) {
            env.respond(return_value);
            true
        } else {
            false
        }
    }

    /// Remove completed
    pub fn remove_completed(&mut self) -> usize {
        let before = self.active.len();
        self.active
            .retain(|_, e| e.state != EnvelopeState::Responded);
        before - self.active.len()
    }

    /// Expire old envelopes
    pub fn expire(&mut self, now: u64) {
        for env in self.active.values_mut() {
            if env.is_expired(now) && env.state != EnvelopeState::Responded {
                env.state = EnvelopeState::Expired;
                self.stats.expired += 1;
            }
        }
        self.active.retain(|_, e| e.state != EnvelopeState::Expired);
    }

    fn evict_oldest(&mut self) {
        let oldest = self.active.keys().next().copied();
        if let Some(k) = oldest {
            self.active.remove(&k);
        }
    }

    /// Active count
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Stats
    pub fn stats(&self) -> &BridgeEnvelopeStats {
        &self.stats
    }
}
