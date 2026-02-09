//! LSM Manager
//!
//! LSM management and coordination.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Avc, Denial, DenialTracker, HookCategory, HookId, LsmHook, LsmState, LsmType};

/// LSM manager
pub struct LsmManager {
    /// Active LSMs
    pub active_lsms: Vec<(LsmType, LsmState)>,
    /// Hooks
    hooks: BTreeMap<String, LsmHook>,
    /// AVC
    avc: Avc,
    /// Denial tracker
    denial_tracker: DenialTracker,
    /// Next hook ID
    next_hook_id: AtomicU64,
    /// Total hook calls
    total_hook_calls: AtomicU64,
}

impl LsmManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            active_lsms: Vec::new(),
            hooks: BTreeMap::new(),
            avc: Avc::new(10000),
            denial_tracker: DenialTracker::new(10000),
            next_hook_id: AtomicU64::new(1),
            total_hook_calls: AtomicU64::new(0),
        }
    }

    /// Register LSM
    #[inline(always)]
    pub fn register_lsm(&mut self, lsm: LsmType, state: LsmState) {
        self.active_lsms.push((lsm, state));
    }

    /// Register hook
    #[inline]
    pub fn register_hook(
        &mut self,
        name: String,
        category: HookCategory,
        lsm: LsmType,
    ) -> HookId {
        let id = HookId::new(self.next_hook_id.fetch_add(1, Ordering::Relaxed));
        let hook = LsmHook::new(id, name.clone(), category, lsm);
        self.hooks.insert(name, hook);
        id
    }

    /// Record hook call
    #[inline]
    pub fn record_hook_call(&self, hook_name: &str, denied: bool, latency_ns: u64) {
        self.total_hook_calls.fetch_add(1, Ordering::Relaxed);

        if let Some(hook) = self.hooks.get(hook_name) {
            hook.record_call(denied, latency_ns);
        }
    }

    /// Record denial
    #[inline(always)]
    pub fn record_denial(&mut self, denial: Denial) {
        self.denial_tracker.record(denial);
    }

    /// Get AVC
    #[inline(always)]
    pub fn avc(&self) -> &Avc {
        &self.avc
    }

    /// Get AVC mutably
    #[inline(always)]
    pub fn avc_mut(&mut self) -> &mut Avc {
        &mut self.avc
    }

    /// Get denial tracker
    #[inline(always)]
    pub fn denial_tracker(&self) -> &DenialTracker {
        &self.denial_tracker
    }

    /// Get active LSMs
    #[inline(always)]
    pub fn active_lsms(&self) -> &[(LsmType, LsmState)] {
        &self.active_lsms
    }

    /// Is LSM active
    #[inline]
    pub fn is_lsm_active(&self, lsm: LsmType) -> bool {
        self.active_lsms
            .iter()
            .any(|(l, s)| *l == lsm && *s != LsmState::Disabled)
    }

    /// Get hook count
    #[inline(always)]
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Get total hook calls
    #[inline(always)]
    pub fn total_hook_calls(&self) -> u64 {
        self.total_hook_calls.load(Ordering::Relaxed)
    }
}

impl Default for LsmManager {
    fn default() -> Self {
        Self::new()
    }
}
