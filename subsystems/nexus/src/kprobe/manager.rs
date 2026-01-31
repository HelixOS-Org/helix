//! Kprobe Manager
//!
//! Managing kprobe registration and lifecycle.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    Architecture, InstructionAnalyzer, KprobeDef, KprobeId, KprobeState, KretprobeDef,
    KretprobeId, ProbeAddress,
};

/// Kprobe manager
pub struct KprobeManager {
    /// Architecture
    arch: Architecture,
    /// Registered kprobes
    kprobes: BTreeMap<KprobeId, KprobeDef>,
    /// Kprobes by address
    by_address: BTreeMap<ProbeAddress, KprobeId>,
    /// Kretprobes
    kretprobes: BTreeMap<KretprobeId, KretprobeDef>,
    /// Next kprobe ID
    next_kprobe_id: AtomicU64,
    /// Next kretprobe ID
    next_kretprobe_id: AtomicU64,
    /// Active kprobes
    active_count: u32,
    /// Total registered
    total_registered: AtomicU64,
    /// Instruction analyzer
    analyzer: InstructionAnalyzer,
}

impl KprobeManager {
    /// Create new kprobe manager
    pub fn new(arch: Architecture) -> Self {
        Self {
            arch,
            kprobes: BTreeMap::new(),
            by_address: BTreeMap::new(),
            kretprobes: BTreeMap::new(),
            next_kprobe_id: AtomicU64::new(1),
            next_kretprobe_id: AtomicU64::new(1),
            active_count: 0,
            total_registered: AtomicU64::new(0),
            analyzer: InstructionAnalyzer::new(arch),
        }
    }

    /// Allocate kprobe ID
    pub fn allocate_kprobe_id(&self) -> KprobeId {
        KprobeId::new(self.next_kprobe_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate kretprobe ID
    pub fn allocate_kretprobe_id(&self) -> KretprobeId {
        KretprobeId::new(self.next_kretprobe_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Register kprobe
    pub fn register(
        &mut self,
        address: ProbeAddress,
        timestamp: u64,
    ) -> Result<KprobeId, &'static str> {
        // Check alignment
        if !address.is_aligned(self.arch.instruction_alignment()) {
            return Err("Address not aligned");
        }

        // Check if already probed
        if self.by_address.contains_key(&address) {
            return Err("Address already probed");
        }

        let id = self.allocate_kprobe_id();
        let def = KprobeDef::new(id, address, timestamp);

        self.by_address.insert(address, id);
        self.kprobes.insert(id, def);
        self.total_registered.fetch_add(1, Ordering::Relaxed);

        Ok(id)
    }

    /// Unregister kprobe
    pub fn unregister(&mut self, id: KprobeId) -> bool {
        if let Some(def) = self.kprobes.remove(&id) {
            self.by_address.remove(&def.address);
            if def.is_armed() {
                self.active_count = self.active_count.saturating_sub(1);
            }
            return true;
        }
        false
    }

    /// Arm kprobe
    pub fn arm(&mut self, id: KprobeId) -> Result<(), &'static str> {
        let def = self.kprobes.get_mut(&id).ok_or("Kprobe not found")?;

        if def.is_armed() {
            return Ok(()); // Already armed
        }

        def.state = KprobeState::Armed;
        self.active_count += 1;

        Ok(())
    }

    /// Disarm kprobe
    pub fn disarm(&mut self, id: KprobeId) -> Result<(), &'static str> {
        let def = self.kprobes.get_mut(&id).ok_or("Kprobe not found")?;

        if !def.is_armed() {
            return Ok(());
        }

        def.state = KprobeState::Disabled;
        self.active_count = self.active_count.saturating_sub(1);

        Ok(())
    }

    /// Register kretprobe
    pub fn register_kretprobe(
        &mut self,
        kprobe_id: KprobeId,
        maxactive: u32,
    ) -> Result<KretprobeId, &'static str> {
        if !self.kprobes.contains_key(&kprobe_id) {
            return Err("Kprobe not found");
        }

        let id = self.allocate_kretprobe_id();
        let def = KretprobeDef::new(id, kprobe_id, maxactive);
        self.kretprobes.insert(id, def);

        Ok(id)
    }

    /// Get kprobe
    pub fn get(&self, id: KprobeId) -> Option<&KprobeDef> {
        self.kprobes.get(&id)
    }

    /// Get kprobe mutably
    pub fn get_mut(&mut self, id: KprobeId) -> Option<&mut KprobeDef> {
        self.kprobes.get_mut(&id)
    }

    /// Get kprobe at address
    pub fn get_at_address(&self, address: ProbeAddress) -> Option<&KprobeDef> {
        self.by_address
            .get(&address)
            .and_then(|id| self.kprobes.get(id))
    }

    /// Get kretprobe
    pub fn get_kretprobe(&self, id: KretprobeId) -> Option<&KretprobeDef> {
        self.kretprobes.get(&id)
    }

    /// Get active count
    pub fn active_count(&self) -> u32 {
        self.active_count
    }

    /// Get total registered
    pub fn total_registered(&self) -> u64 {
        self.total_registered.load(Ordering::Relaxed)
    }

    /// Get instruction analyzer
    pub fn analyzer(&self) -> &InstructionAnalyzer {
        &self.analyzer
    }

    /// Get instruction analyzer mutably
    pub fn analyzer_mut(&mut self) -> &mut InstructionAnalyzer {
        &mut self.analyzer
    }
}
