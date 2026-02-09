//! # Bridge User Context
//!
//! User-space context management for syscall bridge:
//! - Per-thread register save/restore state
//! - FPU/SSE/AVX state management
//! - Thread-local storage (TLS) tracking
//! - User stack management
//! - Signal frame construction
//! - Kernel entry/exit context tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Register set type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterSet {
    General,
    Floating,
    Sse,
    Avx,
    Avx512,
    Debug,
}

/// General purpose register context (x86-64)
#[derive(Debug, Clone)]
pub struct GpRegs {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
}

impl GpRegs {
    #[inline]
    pub fn zeroed() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0x202, // IF set
            cs: 0x33, ss: 0x2b,
            fs_base: 0, gs_base: 0,
        }
    }

    /// Get syscall arguments from registers
    #[inline(always)]
    pub fn syscall_args(&self) -> [u64; 6] {
        [self.rdi, self.rsi, self.rdx, self.r10, self.r8, self.r9]
    }

    /// Set syscall return value
    #[inline(always)]
    pub fn set_return(&mut self, value: u64) {
        self.rax = value;
    }
}

/// FPU/XSAVE state descriptor
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FpuState {
    pub xsave_size: u32,
    pub features_present: u64,
    pub mxcsr: u32,
    pub fcw: u16,
    pub dirty: bool,
    pub lazy_restore: bool,
}

impl FpuState {
    pub fn new() -> Self {
        Self {
            xsave_size: 576, // legacy FXSAVE
            features_present: 0x3, // x87 + SSE
            mxcsr: 0x1F80,
            fcw: 0x037F,
            dirty: false,
            lazy_restore: true,
        }
    }
}

/// TLS descriptor
#[derive(Debug, Clone)]
pub struct TlsDescriptor {
    pub base_addr: u64,
    pub size: u32,
    pub entry_number: u32,
    pub seg_32bit: bool,
}

/// User stack info
#[derive(Debug, Clone)]
pub struct UserStack {
    pub stack_top: u64,
    pub stack_bottom: u64,
    pub stack_size: u64,
    pub guard_size: u64,
    pub current_sp: u64,
}

impl UserStack {
    pub fn new(top: u64, size: u64) -> Self {
        Self {
            stack_top: top,
            stack_bottom: top.saturating_sub(size),
            stack_size: size,
            guard_size: 4096,
            current_sp: top,
        }
    }

    #[inline(always)]
    pub fn used(&self) -> u64 {
        self.stack_top.saturating_sub(self.current_sp)
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.stack_size == 0 { return 0.0; }
        self.used() as f64 / self.stack_size as f64
    }

    #[inline(always)]
    pub fn is_near_overflow(&self) -> bool {
        self.utilization() > 0.9
    }
}

/// Thread user context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ThreadUserContext {
    pub thread_id: u64,
    pub pid: u64,
    pub gp_regs: GpRegs,
    pub fpu: FpuState,
    pub tls: Vec<TlsDescriptor>,
    pub user_stack: UserStack,
    pub signal_stack: Option<UserStack>,
    pub in_syscall: bool,
    pub syscall_nr: u32,
    pub kernel_entries: u64,
    pub kernel_exits: u64,
    pub total_kernel_time_ns: u64,
    pub last_entry_ts: u64,
}

impl ThreadUserContext {
    pub fn new(thread_id: u64, pid: u64, stack_top: u64, stack_size: u64) -> Self {
        Self {
            thread_id,
            pid,
            gp_regs: GpRegs::zeroed(),
            fpu: FpuState::new(),
            tls: Vec::new(),
            user_stack: UserStack::new(stack_top, stack_size),
            signal_stack: None,
            in_syscall: false,
            syscall_nr: 0,
            kernel_entries: 0,
            kernel_exits: 0,
            total_kernel_time_ns: 0,
            last_entry_ts: 0,
        }
    }

    #[inline]
    pub fn enter_kernel(&mut self, syscall_nr: u32, now: u64) {
        self.in_syscall = true;
        self.syscall_nr = syscall_nr;
        self.kernel_entries += 1;
        self.last_entry_ts = now;
    }

    #[inline]
    pub fn exit_kernel(&mut self, now: u64) {
        self.in_syscall = false;
        self.kernel_exits += 1;
        self.total_kernel_time_ns += now.saturating_sub(self.last_entry_ts);
    }

    #[inline(always)]
    pub fn avg_kernel_time_ns(&self) -> f64 {
        if self.kernel_exits == 0 { return 0.0; }
        self.total_kernel_time_ns as f64 / self.kernel_exits as f64
    }

    #[inline(always)]
    pub fn set_tls(&mut self, desc: TlsDescriptor) {
        self.gp_regs.fs_base = desc.base_addr;
        self.tls.push(desc);
    }
}

/// User context manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeUserContextStats {
    pub tracked_threads: usize,
    pub in_kernel: usize,
    pub fpu_dirty: usize,
    pub near_stack_overflow: usize,
    pub total_kernel_entries: u64,
    pub avg_kernel_time_ns: f64,
}

/// Bridge User Context Manager
#[repr(align(64))]
pub struct BridgeUserContext {
    contexts: BTreeMap<u64, ThreadUserContext>,
    stats: BridgeUserContextStats,
}

impl BridgeUserContext {
    pub fn new() -> Self {
        Self {
            contexts: BTreeMap::new(),
            stats: BridgeUserContextStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_thread(&mut self, ctx: ThreadUserContext) {
        self.contexts.insert(ctx.thread_id, ctx);
        self.recompute();
    }

    #[inline(always)]
    pub fn unregister_thread(&mut self, thread_id: u64) {
        self.contexts.remove(&thread_id);
        self.recompute();
    }

    #[inline]
    pub fn enter_kernel(&mut self, thread_id: u64, syscall_nr: u32, now: u64) {
        if let Some(ctx) = self.contexts.get_mut(&thread_id) {
            ctx.enter_kernel(syscall_nr, now);
        }
    }

    #[inline]
    pub fn exit_kernel(&mut self, thread_id: u64, now: u64) {
        if let Some(ctx) = self.contexts.get_mut(&thread_id) {
            ctx.exit_kernel(now);
        }
        self.recompute();
    }

    #[inline]
    pub fn update_sp(&mut self, thread_id: u64, sp: u64) {
        if let Some(ctx) = self.contexts.get_mut(&thread_id) {
            ctx.user_stack.current_sp = sp;
        }
    }

    #[inline(always)]
    pub fn get_context(&self, thread_id: u64) -> Option<&ThreadUserContext> {
        self.contexts.get(&thread_id)
    }

    #[inline(always)]
    pub fn get_context_mut(&mut self, thread_id: u64) -> Option<&mut ThreadUserContext> {
        self.contexts.get_mut(&thread_id)
    }

    fn recompute(&mut self) {
        self.stats.tracked_threads = self.contexts.len();
        self.stats.in_kernel = self.contexts.values().filter(|c| c.in_syscall).count();
        self.stats.fpu_dirty = self.contexts.values().filter(|c| c.fpu.dirty).count();
        self.stats.near_stack_overflow = self.contexts.values()
            .filter(|c| c.user_stack.is_near_overflow())
            .count();
        self.stats.total_kernel_entries = self.contexts.values().map(|c| c.kernel_entries).sum();
        let total_time: f64 = self.contexts.values().map(|c| c.avg_kernel_time_ns()).sum();
        self.stats.avg_kernel_time_ns = if self.contexts.is_empty() { 0.0 }
            else { total_time / self.contexts.len() as f64 };
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeUserContextStats {
        &self.stats
    }
}
