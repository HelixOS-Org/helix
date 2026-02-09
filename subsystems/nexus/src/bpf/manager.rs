//! BPF Manager
//!
//! Central management for BPF programs and maps.

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    BpfHelperId, BpfHelperInfo, BpfInsn, BpfJit, BpfMapId, BpfMapInfo, BpfMapType, BpfProgId,
    BpfProgInfo, BpfProgState, BpfProgType, BpfVerifier,
};

/// BPF manager
pub struct BpfManager {
    /// Programs
    programs: BTreeMap<BpfProgId, BpfProgInfo>,
    /// Maps
    maps: BTreeMap<BpfMapId, BpfMapInfo>,
    /// Helpers
    helpers: BTreeMap<BpfHelperId, BpfHelperInfo>,
    /// Next program ID
    next_prog_id: AtomicU64,
    /// Next map ID
    next_map_id: AtomicU64,
    /// Verifier
    verifier: BpfVerifier,
    /// JIT compiler
    jit: BpfJit,
}

impl BpfManager {
    /// Create new BPF manager
    pub fn new() -> Self {
        Self {
            programs: BTreeMap::new(),
            maps: BTreeMap::new(),
            helpers: BTreeMap::new(),
            next_prog_id: AtomicU64::new(1),
            next_map_id: AtomicU64::new(1),
            verifier: BpfVerifier::new(),
            jit: BpfJit::new(),
        }
    }

    /// Load program
    pub fn load_program(
        &mut self,
        name: String,
        prog_type: BpfProgType,
        insns: &[BpfInsn],
        timestamp: u64,
    ) -> Result<BpfProgId, String> {
        // Verify program
        let result = self.verifier.verify(insns, prog_type);
        if !result.success {
            return Err(result
                .error
                .unwrap_or_else(|| String::from("Verification failed")));
        }

        let id = BpfProgId::new(self.next_prog_id.fetch_add(1, Ordering::Relaxed) as u32);
        let mut info = BpfProgInfo::new(id, prog_type, name, timestamp);
        info.insn_count = insns.len() as u32;
        info.verified = true;
        info.state = BpfProgState::Loaded;

        // JIT compile
        let jit_result = self.jit.compile(insns);
        if jit_result.success {
            info.jit_size = jit_result.compiled_size;
            info.state = BpfProgState::JitCompiled;
        }

        self.programs.insert(id, info);
        Ok(id)
    }

    /// Unload program
    #[inline(always)]
    pub fn unload_program(&mut self, id: BpfProgId) -> bool {
        self.programs.remove(&id).is_some()
    }

    /// Create map
    pub fn create_map(
        &mut self,
        name: String,
        map_type: BpfMapType,
        key_size: u32,
        value_size: u32,
        max_entries: u32,
        timestamp: u64,
    ) -> BpfMapId {
        let id = BpfMapId::new(self.next_map_id.fetch_add(1, Ordering::Relaxed) as u32);
        let info = BpfMapInfo::new(
            id,
            map_type,
            name,
            key_size,
            value_size,
            max_entries,
            timestamp,
        );
        self.maps.insert(id, info);
        id
    }

    /// Delete map
    #[inline(always)]
    pub fn delete_map(&mut self, id: BpfMapId) -> bool {
        self.maps.remove(&id).is_some()
    }

    /// Get program
    #[inline(always)]
    pub fn get_program(&self, id: BpfProgId) -> Option<&BpfProgInfo> {
        self.programs.get(&id)
    }

    /// Get program mutably
    #[inline(always)]
    pub fn get_program_mut(&mut self, id: BpfProgId) -> Option<&mut BpfProgInfo> {
        self.programs.get_mut(&id)
    }

    /// Get all programs
    #[inline(always)]
    pub fn all_programs(&self) -> impl Iterator<Item = &BpfProgInfo> {
        self.programs.values()
    }

    /// Get map
    #[inline(always)]
    pub fn get_map(&self, id: BpfMapId) -> Option<&BpfMapInfo> {
        self.maps.get(&id)
    }

    /// Get map mutably
    #[inline(always)]
    pub fn get_map_mut(&mut self, id: BpfMapId) -> Option<&mut BpfMapInfo> {
        self.maps.get_mut(&id)
    }

    /// Get all maps
    #[inline(always)]
    pub fn all_maps(&self) -> impl Iterator<Item = &BpfMapInfo> {
        self.maps.values()
    }

    /// Register helper
    #[inline(always)]
    pub fn register_helper(&mut self, info: BpfHelperInfo) {
        self.helpers.insert(info.id, info);
    }

    /// Get helper
    #[inline(always)]
    pub fn get_helper(&self, id: BpfHelperId) -> Option<&BpfHelperInfo> {
        self.helpers.get(&id)
    }

    /// Get verifier
    #[inline(always)]
    pub fn verifier(&self) -> &BpfVerifier {
        &self.verifier
    }

    /// Get verifier mutably
    #[inline(always)]
    pub fn verifier_mut(&mut self) -> &mut BpfVerifier {
        &mut self.verifier
    }

    /// Get JIT compiler
    #[inline(always)]
    pub fn jit(&self) -> &BpfJit {
        &self.jit
    }

    /// Get JIT compiler mutably
    #[inline(always)]
    pub fn jit_mut(&mut self) -> &mut BpfJit {
        &mut self.jit
    }

    /// Count programs
    #[inline(always)]
    pub fn program_count(&self) -> usize {
        self.programs.len()
    }

    /// Count maps
    #[inline(always)]
    pub fn map_count(&self) -> usize {
        self.maps.len()
    }

    /// Count helpers
    #[inline(always)]
    pub fn helper_count(&self) -> usize {
        self.helpers.len()
    }
}

impl Default for BpfManager {
    fn default() -> Self {
        Self::new()
    }
}
