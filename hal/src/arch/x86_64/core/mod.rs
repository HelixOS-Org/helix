//! # x86_64 Core Module
//!
//! Fundamental CPU control, CPUID, MSRs, and control registers for x86_64 long mode.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                           CORE CPU FRAMEWORK                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
//! │  │   CPUID      │  │    MSRs      │  │  Control     │  │   Features   │ │
//! │  │  Enumeration │  │  Framework   │  │  Registers   │  │  Detection   │ │
//! │  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘ │
//! │                                                                          │
//! │  ┌──────────────┐  ┌──────────────┐                                     │
//! │  │   Cache      │  │    FPU       │                                     │
//! │  │  Control     │  │ SSE/AVX/512  │                                     │
//! │  └──────────────┘  └──────────────┘                                     │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`cpuid`]: Complete CPUID enumeration (features, topology, cache info)
//! - [`msr`]: Model-Specific Registers with type-safe wrappers
//! - [`control_regs`]: CR0, CR2, CR3, CR4, XCR0 management
//! - [`features`]: High-level CPU capability detection
//! - [`cache`]: Cache control, prefetch, memory barriers
//! - [`fpu`]: FPU/SSE/AVX state management (FXSAVE, XSAVE)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_hal::arch::x86_64::core::control_regs::{Cr0, Cr4};
//! use helix_hal::arch::x86_64::core::cpuid::CpuId;
//! use helix_hal::arch::x86_64::core::features::CpuCapabilities;
//! use helix_hal::arch::x86_64::core::msr::{Efer, Tsc};
//!
//! // Detect CPU features
//! let cpuid = CpuId::new();
//! println!("Vendor: {:?}", cpuid.vendor());
//! println!("Has AVX-512: {}", cpuid.has_avx512f());
//!
//! // High-level capability check
//! let caps = CpuCapabilities::detect();
//! if caps.has_avx512() {
//!     // Use AVX-512 optimized code
//! }
//!
//! // Read EFER
//! let efer = Efer::read();
//! assert!(efer.long_mode_active());
//!
//! // Read TSC
//! let tsc = Tsc::read();
//!
//! // Control registers
//! let cr0 = Cr0::read();
//! assert!(cr0.paging_enabled());
//!
//! // Check CR4 features
//! let cr4 = Cr4::read();
//! if cr4.smep_enabled() {
//!     println!("SMEP is active");
//! }
//! ```
//!
//! ## Safety
//!
//! Many operations in this module require ring 0 privilege.
//! Writing to MSRs or control registers can cause system crashes
//! if done incorrectly. All dangerous operations are marked `unsafe`.

// =============================================================================
// SUBMODULES
// =============================================================================

pub mod cache;
pub mod control_regs;
pub mod cpuid;
pub mod features;
pub mod fpu;
pub mod msr;

// =============================================================================
// RE-EXPORTS
// =============================================================================

// CPUID types
// Cache control
pub use cache::{
    align_down_cache_line, align_up_cache_line, cache_line_size, clflush, clflushopt, clwb,
    disable_cache, enable_cache, flush_range, flush_range_opt, invd, is_cache_line_aligned,
    movnti32, movnti64, prefetch, prefetch_range, prefetchw, wbinvd, CacheAligned, CachePadded,
    PrefetchHint, CACHE_LINE_SIZE,
};
// Control registers
pub use control_regs::{
    cli, hlt, invlpg, invlpg_all, invpcid, invpcid_type, lfence, mfence, pause, restore_interrupts,
    sfence, sti, Cr0, Cr2, Cr3, Cr3Flags, Cr4, Cr8, RFlags, Xcr0,
};
pub use cpuid::{
    cpuid as raw_cpuid, cpuid_count as raw_cpuid_count, enumerate_caches, enumerate_topology,
    get_processor_frequency, get_tsc_frequency, get_xsave_info, CacheInfo, CacheType, CpuId,
    CpuIdResult, Feature, TopologyInfo, TopologyLevel, Vendor,
};
// Features
pub use features::{
    generate_report, CpuCapabilities, FeatureCategory, FeatureRequirements, MissingFeature,
};
// FPU/SIMD
pub use fpu::{
    clear_mxcsr_exceptions, clear_ts, detect_save_mode, fclex, fldcw, fninit, fstcw, fstsw, fwait,
    get_mxcsr, init_fpu, set_mxcsr, set_ts, xrstor, xrstors, xsave, xsavec, xsaveopt, xsaves,
    FpuSaveMode, FxSaveArea, MxcsrExceptions, XsaveArea, XsaveHeader, DEFAULT_MXCSR,
    FXSAVE_AREA_SIZE, MXCSR_MASK,
};
// MSR types
pub use msr::{
    addr as msr_addr, ibpb, l1d_flush, rdmsr, swapgs, wrmsr, ApicBase, ApicBaseFlags, Cstar,
    DebugCtl, Efer, FeatureControl, FsBase, GsBase, KernelGsBase, Lstar, MiscEnable, Pat,
    PatMemoryType, PlatformInfo, SfMask, SpecCtrl, Star, Tsc,
};
