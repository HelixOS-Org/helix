//! # Holistic FPU Context
//!
//! Floating-point unit context management:
//! - Lazy/eager FPU save/restore strategies
//! - SSE/AVX/AVX-512 state tracking per task
//! - Extended state area (XSAVE) management
//! - FPU exception handling and masking
//! - Context switch optimization statistics
//! - Hardware feature detection and capabilities

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// FPU feature level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FpuFeature {
    X87,
    Sse,
    Sse2,
    Sse3,
    Ssse3,
    Sse41,
    Sse42,
    Avx,
    Avx2,
    Avx512,
}

impl FpuFeature {
    #[inline]
    pub fn state_size(&self) -> usize {
        match self {
            Self::X87 => 108,
            Self::Sse | Self::Sse2 | Self::Sse3 | Self::Ssse3 | Self::Sse41 | Self::Sse42 => 512,
            Self::Avx | Self::Avx2 => 832,
            Self::Avx512 => 2688,
        }
    }
}

/// FPU save strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpuStrategy {
    Lazy,
    Eager,
    Hybrid,
}

/// FPU exception type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpuException {
    InvalidOperation,
    DivideByZero,
    Overflow,
    Underflow,
    Precision,
    Denormalized,
    StackFault,
}

/// MXCSR control bits
#[derive(Debug, Clone, Copy)]
pub struct MxcsrFlags {
    pub invalid_op_mask: bool,
    pub denormal_mask: bool,
    pub divide_zero_mask: bool,
    pub overflow_mask: bool,
    pub underflow_mask: bool,
    pub precision_mask: bool,
    pub flush_to_zero: bool,
    pub denormals_are_zero: bool,
    pub rounding_mode: u8,
}

impl Default for MxcsrFlags {
    fn default() -> Self {
        Self {
            invalid_op_mask: true, denormal_mask: true, divide_zero_mask: true,
            overflow_mask: true, underflow_mask: true, precision_mask: true,
            flush_to_zero: false, denormals_are_zero: false, rounding_mode: 0,
        }
    }
}

impl MxcsrFlags {
    pub fn to_bits(&self) -> u32 {
        let mut v = 0u32;
        if self.invalid_op_mask { v |= 1 << 7; }
        if self.denormal_mask { v |= 1 << 8; }
        if self.divide_zero_mask { v |= 1 << 9; }
        if self.overflow_mask { v |= 1 << 10; }
        if self.underflow_mask { v |= 1 << 11; }
        if self.precision_mask { v |= 1 << 12; }
        if self.flush_to_zero { v |= 1 << 15; }
        if self.denormals_are_zero { v |= 1 << 6; }
        v |= (self.rounding_mode as u32 & 0x3) << 13;
        v
    }
}

/// Per-task FPU context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TaskFpuContext {
    pub task_id: u64,
    pub max_feature: FpuFeature,
    pub actual_used: FpuFeature,
    pub state_dirty: bool,
    pub lazy_switches: u64,
    pub eager_saves: u64,
    pub trap_count: u64,
    pub exceptions: Vec<FpuException>,
    pub mxcsr: MxcsrFlags,
    pub xsave_area_size: usize,
    pub last_save_ts: u64,
    pub last_restore_ts: u64,
    pub fpu_time_ns: u64,
}

impl TaskFpuContext {
    pub fn new(task_id: u64, max_feature: FpuFeature) -> Self {
        Self {
            task_id, max_feature, actual_used: FpuFeature::X87,
            state_dirty: false, lazy_switches: 0, eager_saves: 0,
            trap_count: 0, exceptions: Vec::new(), mxcsr: MxcsrFlags::default(),
            xsave_area_size: max_feature.state_size(), last_save_ts: 0,
            last_restore_ts: 0, fpu_time_ns: 0,
        }
    }

    #[inline(always)]
    pub fn mark_dirty(&mut self) { self.state_dirty = true; }

    #[inline]
    pub fn save(&mut self, ts: u64) {
        self.state_dirty = false;
        self.eager_saves += 1;
        self.last_save_ts = ts;
    }

    #[inline(always)]
    pub fn restore(&mut self, ts: u64) {
        self.last_restore_ts = ts;
    }

    #[inline]
    pub fn trap_and_restore(&mut self, ts: u64) {
        self.trap_count += 1;
        self.lazy_switches += 1;
        self.restore(ts);
    }

    #[inline(always)]
    pub fn record_exception(&mut self, exc: FpuException) {
        self.exceptions.push(exc);
    }

    #[inline]
    pub fn upgrade_feature(&mut self, feature: FpuFeature) {
        if feature > self.actual_used {
            self.actual_used = feature;
            self.xsave_area_size = feature.state_size();
        }
    }

    #[inline(always)]
    pub fn exception_count(&self) -> usize { self.exceptions.len() }
}

/// CPU FPU capabilities
#[derive(Debug, Clone)]
pub struct CpuFpuCaps {
    pub cpu_id: u32,
    pub max_feature: FpuFeature,
    pub xsave_support: bool,
    pub xsaveopt_support: bool,
    pub xsavec_support: bool,
    pub xsaves_support: bool,
    pub max_state_size: usize,
    pub compact_format: bool,
}

impl CpuFpuCaps {
    pub fn new(cpu_id: u32, max_feature: FpuFeature) -> Self {
        Self {
            cpu_id, max_feature, xsave_support: true, xsaveopt_support: true,
            xsavec_support: false, xsaves_support: false,
            max_state_size: max_feature.state_size(), compact_format: false,
        }
    }
}

/// FPU context stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FpuContextStats {
    pub tasks_tracked: usize,
    pub total_lazy_switches: u64,
    pub total_eager_saves: u64,
    pub total_traps: u64,
    pub total_exceptions: u64,
    pub avg_state_size: f64,
    pub avx_users: usize,
    pub avx512_users: usize,
}

/// Holistic FPU context manager
#[repr(align(64))]
pub struct HolisticFpuContext {
    tasks: BTreeMap<u64, TaskFpuContext>,
    cpus: BTreeMap<u32, CpuFpuCaps>,
    strategy: FpuStrategy,
    stats: FpuContextStats,
    lazy_threshold: u64,
}

impl HolisticFpuContext {
    pub fn new(strategy: FpuStrategy) -> Self {
        Self {
            tasks: BTreeMap::new(), cpus: BTreeMap::new(),
            strategy, stats: FpuContextStats::default(),
            lazy_threshold: 10,
        }
    }

    #[inline(always)]
    pub fn add_cpu(&mut self, caps: CpuFpuCaps) {
        self.cpus.insert(caps.cpu_id, caps);
    }

    #[inline(always)]
    pub fn register_task(&mut self, task_id: u64, max_feature: FpuFeature) {
        self.tasks.insert(task_id, TaskFpuContext::new(task_id, max_feature));
    }

    #[inline]
    pub fn should_eager_save(&self, task_id: u64) -> bool {
        match self.strategy {
            FpuStrategy::Eager => true,
            FpuStrategy::Lazy => false,
            FpuStrategy::Hybrid => {
                self.tasks.get(&task_id).map(|t| t.trap_count > self.lazy_threshold).unwrap_or(false)
            }
        }
    }

    pub fn context_switch(&mut self, from: u64, to: u64, ts: u64) {
        let eager = self.should_eager_save(from);
        if let Some(ctx) = self.tasks.get_mut(&from) {
            if eager || ctx.state_dirty {
                ctx.save(ts);
            }
        }
        if let Some(ctx) = self.tasks.get_mut(&to) {
            if eager {
                ctx.restore(ts);
            }
        }
    }

    #[inline]
    pub fn handle_fpu_trap(&mut self, task_id: u64, ts: u64) {
        if let Some(ctx) = self.tasks.get_mut(&task_id) {
            ctx.trap_and_restore(ts);
        }
    }

    #[inline]
    pub fn record_exception(&mut self, task_id: u64, exc: FpuException) {
        if let Some(ctx) = self.tasks.get_mut(&task_id) {
            ctx.record_exception(exc);
        }
    }

    #[inline]
    pub fn upgrade_feature(&mut self, task_id: u64, feature: FpuFeature) {
        if let Some(ctx) = self.tasks.get_mut(&task_id) {
            ctx.upgrade_feature(feature);
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.tasks_tracked = self.tasks.len();
        self.stats.total_lazy_switches = self.tasks.values().map(|t| t.lazy_switches).sum();
        self.stats.total_eager_saves = self.tasks.values().map(|t| t.eager_saves).sum();
        self.stats.total_traps = self.tasks.values().map(|t| t.trap_count).sum();
        self.stats.total_exceptions = self.tasks.values().map(|t| t.exception_count() as u64).sum();
        let sizes: Vec<f64> = self.tasks.values().map(|t| t.xsave_area_size as f64).collect();
        self.stats.avg_state_size = if sizes.is_empty() { 0.0 } else { sizes.iter().sum::<f64>() / sizes.len() as f64 };
        self.stats.avx_users = self.tasks.values().filter(|t| t.actual_used >= FpuFeature::Avx).count();
        self.stats.avx512_users = self.tasks.values().filter(|t| t.actual_used >= FpuFeature::Avx512).count();
    }

    #[inline(always)]
    pub fn task(&self, id: u64) -> Option<&TaskFpuContext> { self.tasks.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &FpuContextStats { &self.stats }
}
