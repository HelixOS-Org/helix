//! # Bridge Emulation
//!
//! Syscall emulation and translation:
//! - Foreign ABI emulation
//! - Syscall number translation tables
//! - Argument marshalling
//! - Return value translation
//! - Emulation performance tracking
//! - Compatibility layers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// EMULATION TARGET
// ============================================================================

/// Foreign ABI target
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmulationTarget {
    /// Linux x86_64 syscalls
    LinuxX86_64,
    /// Linux aarch64 syscalls
    LinuxAarch64,
    /// FreeBSD syscalls
    FreeBsd,
    /// POSIX standard subset
    Posix,
    /// Windows NT syscalls
    WindowsNt,
    /// macOS Mach traps
    MacOsMach,
    /// Custom target
    Custom(u32),
}

/// Emulation accuracy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulationAccuracy {
    /// Full emulation (all features)
    Full,
    /// Partial (common features)
    Partial,
    /// Stub (returns success but no-op)
    Stub,
    /// Unsupported (returns error)
    Unsupported,
}

// ============================================================================
// TRANSLATION TABLE
// ============================================================================

/// Syscall translation entry
#[derive(Debug, Clone)]
pub struct TranslationEntry {
    /// Foreign syscall number
    pub foreign_nr: u32,
    /// Native syscall number
    pub native_nr: u32,
    /// Accuracy
    pub accuracy: EmulationAccuracy,
    /// Argument mapping (foreign index → native index)
    pub arg_mapping: Vec<(u8, u8)>,
    /// Needs argument translation
    pub needs_arg_translation: bool,
    /// Needs return translation
    pub needs_return_translation: bool,
    /// Invocation count
    pub invocations: u64,
    /// Failure count
    pub failures: u64,
}

impl TranslationEntry {
    pub fn new(foreign_nr: u32, native_nr: u32, accuracy: EmulationAccuracy) -> Self {
        Self {
            foreign_nr,
            native_nr,
            accuracy,
            arg_mapping: Vec::new(),
            needs_arg_translation: false,
            needs_return_translation: false,
            invocations: 0,
            failures: 0,
        }
    }

    pub fn with_arg_mapping(mut self, mapping: Vec<(u8, u8)>) -> Self {
        self.arg_mapping = mapping;
        self.needs_arg_translation = true;
        self
    }

    pub fn with_return_translation(mut self) -> Self {
        self.needs_return_translation = true;
        self
    }

    /// Translate arguments
    pub fn translate_args(&self, foreign_args: &[u64]) -> Vec<u64> {
        if !self.needs_arg_translation || self.arg_mapping.is_empty() {
            return foreign_args.to_vec();
        }

        let max_native = self
            .arg_mapping
            .iter()
            .map(|(_, n)| *n as usize + 1)
            .max()
            .unwrap_or(0);
        let mut native_args = alloc::vec![0u64; max_native];

        for &(foreign_idx, native_idx) in &self.arg_mapping {
            if let Some(&val) = foreign_args.get(foreign_idx as usize) {
                if (native_idx as usize) < native_args.len() {
                    native_args[native_idx as usize] = val;
                }
            }
        }

        native_args
    }

    /// Success rate
    pub fn success_rate(&self) -> f64 {
        if self.invocations == 0 {
            return 1.0;
        }
        1.0 - (self.failures as f64 / self.invocations as f64)
    }
}

/// Translation table
#[derive(Debug, Clone)]
pub struct TranslationTable {
    /// Target ABI
    pub target: EmulationTarget,
    /// Entries (foreign_nr → entry)
    pub entries: BTreeMap<u32, TranslationEntry>,
    /// Coverage (fraction of foreign syscalls supported)
    pub coverage: f64,
}

impl TranslationTable {
    pub fn new(target: EmulationTarget) -> Self {
        Self {
            target,
            entries: BTreeMap::new(),
            coverage: 0.0,
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: TranslationEntry) {
        self.entries.insert(entry.foreign_nr, entry);
    }

    /// Lookup translation
    pub fn lookup(&self, foreign_nr: u32) -> Option<&TranslationEntry> {
        self.entries.get(&foreign_nr)
    }

    /// Lookup mutable
    pub fn lookup_mut(&mut self, foreign_nr: u32) -> Option<&mut TranslationEntry> {
        self.entries.get_mut(&foreign_nr)
    }

    /// Supported count
    pub fn supported_count(&self) -> usize {
        self.entries
            .values()
            .filter(|e| !matches!(e.accuracy, EmulationAccuracy::Unsupported))
            .count()
    }
}

// ============================================================================
// ERRNO TRANSLATION
// ============================================================================

/// Errno mapping (foreign → native)
#[derive(Debug, Clone)]
pub struct ErrnoMapping {
    /// Target
    pub target: EmulationTarget,
    /// Mapping (foreign_errno → native_errno)
    pub mapping: BTreeMap<i32, i32>,
    /// Reverse mapping
    pub reverse: BTreeMap<i32, i32>,
}

impl ErrnoMapping {
    pub fn new(target: EmulationTarget) -> Self {
        Self {
            target,
            mapping: BTreeMap::new(),
            reverse: BTreeMap::new(),
        }
    }

    /// Add mapping
    pub fn add(&mut self, foreign: i32, native: i32) {
        self.mapping.insert(foreign, native);
        self.reverse.insert(native, foreign);
    }

    /// Translate foreign → native
    pub fn to_native(&self, foreign_errno: i32) -> i32 {
        self.mapping.get(&foreign_errno).copied().unwrap_or(foreign_errno)
    }

    /// Translate native → foreign
    pub fn to_foreign(&self, native_errno: i32) -> i32 {
        self.reverse.get(&native_errno).copied().unwrap_or(native_errno)
    }
}

// ============================================================================
// EMULATION CONTEXT
// ============================================================================

/// Per-process emulation context
#[derive(Debug, Clone)]
pub struct EmulationContext {
    /// Process ID
    pub pid: u64,
    /// Target ABI
    pub target: EmulationTarget,
    /// Total syscalls emulated
    pub emulated_calls: u64,
    /// Failed emulations
    pub failed_calls: u64,
    /// Stub calls (no-op)
    pub stub_calls: u64,
    /// Created at
    pub created_at: u64,
}

impl EmulationContext {
    pub fn new(pid: u64, target: EmulationTarget, now: u64) -> Self {
        Self {
            pid,
            target,
            emulated_calls: 0,
            failed_calls: 0,
            stub_calls: 0,
            created_at: now,
        }
    }

    pub fn record_call(&mut self, accuracy: EmulationAccuracy) {
        self.emulated_calls += 1;
        match accuracy {
            EmulationAccuracy::Stub => self.stub_calls += 1,
            EmulationAccuracy::Unsupported => self.failed_calls += 1,
            _ => {}
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.emulated_calls == 0 {
            return 1.0;
        }
        1.0 - (self.failed_calls as f64 / self.emulated_calls as f64)
    }
}

// ============================================================================
// EMULATION MANAGER
// ============================================================================

/// Emulation stats
#[derive(Debug, Clone, Default)]
pub struct EmulationStats {
    /// Active contexts
    pub active_contexts: usize,
    /// Total emulated calls
    pub total_emulated: u64,
    /// Total failures
    pub total_failures: u64,
    /// Translation tables loaded
    pub tables_loaded: usize,
    /// Overall success rate
    pub success_rate: f64,
}

/// Bridge emulation manager
pub struct BridgeEmulationManager {
    /// Translation tables (target → table)
    tables: BTreeMap<u8, TranslationTable>,
    /// Errno mappings
    errno_maps: BTreeMap<u8, ErrnoMapping>,
    /// Active contexts (pid → context)
    contexts: BTreeMap<u64, EmulationContext>,
    /// Stats
    stats: EmulationStats,
}

impl BridgeEmulationManager {
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            errno_maps: BTreeMap::new(),
            contexts: BTreeMap::new(),
            stats: EmulationStats::default(),
        }
    }

    /// Register translation table
    pub fn register_table(&mut self, table: TranslationTable) {
        self.tables.insert(table.target as u8, table);
        self.stats.tables_loaded = self.tables.len();
    }

    /// Register errno mapping
    pub fn register_errno(&mut self, mapping: ErrnoMapping) {
        self.errno_maps.insert(mapping.target as u8, mapping);
    }

    /// Create emulation context for process
    pub fn create_context(&mut self, pid: u64, target: EmulationTarget, now: u64) {
        self.contexts
            .insert(pid, EmulationContext::new(pid, target, now));
        self.stats.active_contexts = self.contexts.len();
    }

    /// Remove context
    pub fn remove_context(&mut self, pid: u64) {
        self.contexts.remove(&pid);
        self.stats.active_contexts = self.contexts.len();
    }

    /// Translate syscall
    pub fn translate(
        &mut self,
        pid: u64,
        foreign_nr: u32,
        foreign_args: &[u64],
    ) -> Option<(u32, Vec<u64>, EmulationAccuracy)> {
        let context = self.contexts.get(&pid)?;
        let target_key = context.target as u8;
        let table = self.tables.get(&target_key)?;
        let entry = table.lookup(foreign_nr)?;

        let native_args = entry.translate_args(foreign_args);
        let accuracy = entry.accuracy;

        // Update stats
        if let Some(ctx) = self.contexts.get_mut(&pid) {
            ctx.record_call(accuracy);
        }

        if let Some(table) = self.tables.get_mut(&target_key) {
            if let Some(entry) = table.lookup_mut(foreign_nr) {
                entry.invocations += 1;
            }
        }

        self.stats.total_emulated += 1;
        if accuracy == EmulationAccuracy::Unsupported {
            self.stats.total_failures += 1;
        }
        if self.stats.total_emulated > 0 {
            self.stats.success_rate =
                1.0 - (self.stats.total_failures as f64 / self.stats.total_emulated as f64);
        }

        Some((entry.native_nr, native_args, accuracy))
    }

    /// Translate errno
    pub fn translate_errno(&self, target: EmulationTarget, native_errno: i32) -> i32 {
        self.errno_maps
            .get(&(target as u8))
            .map(|m| m.to_foreign(native_errno))
            .unwrap_or(native_errno)
    }

    /// Stats
    pub fn stats(&self) -> &EmulationStats {
        &self.stats
    }
}
