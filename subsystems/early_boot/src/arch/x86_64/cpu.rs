//! # x86_64 CPU Initialization
//!
//! CPU feature detection, control register configuration, and FPU/SIMD setup.

use super::*;
use crate::core::{BootContext, CpuFeatures, CpuState};
use crate::error::{BootError, BootResult};

// =============================================================================
// CPU FEATURE DETECTION
// =============================================================================

/// Detect CPU features and fill in CpuState
pub unsafe fn detect_features(state: &mut CpuState) -> BootResult<()> {
    // Get vendor string
    let (_, ebx, ecx, edx) = cpuid(0, 0);

    // Vendor string is EBX + EDX + ECX
    state.vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
    state.vendor[4..8].copy_from_slice(&edx.to_le_bytes());
    state.vendor[8..12].copy_from_slice(&ecx.to_le_bytes());

    // Get basic CPU info
    let (eax, ebx, ecx, edx) = cpuid(1, 0);

    state.stepping = eax & 0xF;
    state.model = ((eax >> 4) & 0xF) + (((eax >> 16) & 0xF) << 4);
    state.family = ((eax >> 8) & 0xF) + ((eax >> 20) & 0xFF);

    // Logical processor count from CPUID leaf 1
    state.logical_count = ((ebx >> 16) & 0xFF);

    // Initial APIC ID
    state.bsp_id = (ebx >> 24) & 0xFF;

    // Feature flags from leaf 1
    let mut features = CpuFeatures::empty();

    // ECX features
    if ecx & (1 << 0) != 0 {
        features |= CpuFeatures::SSE3;
    }
    if ecx & (1 << 9) != 0 {
        features |= CpuFeatures::SSSE3;
    }
    if ecx & (1 << 19) != 0 {
        features |= CpuFeatures::SSE4_1;
    }
    if ecx & (1 << 20) != 0 {
        features |= CpuFeatures::SSE4_2;
    }
    if ecx & (1 << 26) != 0 {
        features |= CpuFeatures::XSAVE;
    }
    if ecx & (1 << 28) != 0 {
        features |= CpuFeatures::AVX;
    }
    if ecx & (1 << 30) != 0 {
        features |= CpuFeatures::RDRAND;
    }

    // EDX features
    if edx & (1 << 0) != 0 {
        features |= CpuFeatures::FPU;
    }
    if edx & (1 << 4) != 0 {
        features |= CpuFeatures::TSC;
    }
    if edx & (1 << 25) != 0 {
        features |= CpuFeatures::SSE;
    }
    if edx & (1 << 26) != 0 {
        features |= CpuFeatures::SSE2;
    }

    // Extended features from leaf 7
    if cpuid_max_leaf() >= 7 {
        let (_, ebx, ecx, _) = cpuid(7, 0);

        if ebx & (1 << 0) != 0 {
            features |= CpuFeatures::FSGSBASE;
        }
        if ebx & (1 << 5) != 0 {
            features |= CpuFeatures::AVX2;
        }
        if ebx & (1 << 7) != 0 {
            features |= CpuFeatures::SMEP;
        }
        if ebx & (1 << 18) != 0 {
            features |= CpuFeatures::RDSEED;
        }
        if ebx & (1 << 20) != 0 {
            features |= CpuFeatures::SMAP;
        }

        // AVX-512 features
        if ebx & (1 << 16) != 0 {
            features |= CpuFeatures::AVX512;
        }

        if ecx & (1 << 2) != 0 {
            features |= CpuFeatures::UMIP;
        }
        if ecx & (1 << 3) != 0 {
            features |= CpuFeatures::PKU;
        }
        if ecx & (1 << 16) != 0 {
            features |= CpuFeatures::LA57;
        }

        // INVPCID and PCID
        if ebx & (1 << 10) != 0 {
            features |= CpuFeatures::INVPCID;
        }
    }

    // TSC deadline from leaf 1
    if has_cpuid_feature(1, 0, CpuidReg::Ecx, 24) {
        features |= CpuFeatures::TSC_DEADLINE;
    }

    // PCID from leaf 1
    if has_cpuid_feature(1, 0, CpuidReg::Ecx, 17) {
        features |= CpuFeatures::PCID;
    }

    // Atomics (always available on x86_64)
    features |= CpuFeatures::ATOMIC | CpuFeatures::CAS;

    state.features = features;

    // Get CPU brand string if available
    if cpuid_max_extended_leaf() >= 0x8000_0004 {
        let mut brand = [0u8; 48];

        for i in 0..3 {
            let (eax, ebx, ecx, edx) = cpuid(0x8000_0002 + i, 0);
            let offset = i as usize * 16;
            brand[offset..offset + 4].copy_from_slice(&eax.to_le_bytes());
            brand[offset + 4..offset + 8].copy_from_slice(&ebx.to_le_bytes());
            brand[offset + 8..offset + 12].copy_from_slice(&ecx.to_le_bytes());
            brand[offset + 12..offset + 16].copy_from_slice(&edx.to_le_bytes());
        }

        // Trim leading spaces
        let start = brand.iter().position(|&c| c != b' ').unwrap_or(0);
        let len = (48 - start).min(state.model_name.len());
        state.model_name[..len].copy_from_slice(&brand[start..start + len]);
    }

    // Cache information from leaf 4
    detect_cache_info(state);

    // Topology information
    detect_topology(state);

    Ok(())
}

/// Detect cache information
fn detect_cache_info(state: &mut CpuState) {
    if cpuid_max_leaf() < 4 {
        return;
    }

    for index in 0..16 {
        let (eax, ebx, ecx, _) = cpuid(4, index);

        let cache_type = eax & 0x1F;
        if cache_type == 0 {
            break; // No more caches
        }

        let level = (eax >> 5) & 0x7;
        let line_size = (ebx & 0xFFF) + 1;
        let partitions = ((ebx >> 12) & 0x3FF) + 1;
        let ways = ((ebx >> 22) & 0x3FF) + 1;
        let sets = ecx + 1;

        let size_kb = (line_size * partitions * ways * sets) / 1024;

        match (level, cache_type) {
            (1, 1) => state.l1d_cache_kb = size_kb, // L1 Data
            (1, 2) => state.l1i_cache_kb = size_kb, // L1 Instruction
            (2, 3) => state.l2_cache_kb = size_kb,  // L2 Unified
            (3, 3) => state.l3_cache_kb = size_kb,  // L3 Unified
            _ => {},
        }
    }
}

/// Detect CPU topology
fn detect_topology(state: &mut CpuState) {
    // Try x2APIC topology first (leaf 0x0B)
    if cpuid_max_leaf() >= 0x0B {
        let mut level = 0;
        let mut total_threads = 0;
        let mut threads_per_core = 1;

        loop {
            let (_, ebx, ecx, _) = cpuid(0x0B, level);

            let level_type = (ecx >> 8) & 0xFF;
            if level_type == 0 {
                break;
            }

            let processors = ebx & 0xFFFF;

            match level_type {
                1 => {
                    // SMT level
                    threads_per_core = processors as u32;
                },
                2 => {
                    // Core level
                    total_threads = processors as u32;
                },
                _ => {},
            }

            level += 1;
            if level > 10 {
                break;
            }
        }

        if total_threads > 0 {
            state.logical_count = total_threads;
            state.core_count = total_threads / threads_per_core.max(1);
        }
    } else {
        // Fallback: use basic CPUID info
        state.core_count = state.logical_count;
    }

    // Ensure at least 1 core/thread
    if state.core_count == 0 {
        state.core_count = 1;
    }
    if state.logical_count == 0 {
        state.logical_count = 1;
    }
}

// =============================================================================
// FPU/SIMD INITIALIZATION
// =============================================================================

/// Initialize FPU and SIMD extensions
pub unsafe fn init_fpu_simd() -> BootResult<()> {
    // Enable FPU
    let mut cr0 = read_cr0();
    cr0 &= !(CR0_EM | CR0_TS); // Clear EM and TS
    cr0 |= CR0_MP | CR0_NE; // Set MP and NE
    write_cr0(cr0);

    // Enable SSE
    let mut cr4 = read_cr4();
    cr4 |= CR4_OSFXSR | CR4_OSXMMEXCPT;
    write_cr4(cr4);

    // Initialize FPU
    core::arch::asm!("fninit", options(nomem));

    // Enable XSAVE if available
    if has_cpuid_feature(1, 0, CpuidReg::Ecx, 26) {
        cr4 |= CR4_OSXSAVE;
        write_cr4(cr4);

        // Enable x87, SSE, AVX in XCR0
        let mut xcr0 = xgetbv(0);
        xcr0 |= 0x07; // x87, SSE, AVX

        // Enable AVX-512 if available
        if has_cpuid_feature(7, 0, CpuidReg::Ebx, 16) {
            xcr0 |= 0xE0; // AVX-512 opmask, ZMM_Hi256, Hi16_ZMM
        }

        xsetbv(0, xcr0);
    }

    Ok(())
}

// =============================================================================
// CONTROL REGISTER CONFIGURATION
// =============================================================================

/// Configure control registers for optimal operation
pub unsafe fn configure_control_registers(ctx: &mut BootContext) -> BootResult<()> {
    // CR0: Already set up for protected mode with paging
    let mut cr0 = read_cr0();
    cr0 |= CR0_WP; // Write protect in ring 0
    write_cr0(cr0);

    // CR4: Enable additional features
    let mut cr4 = read_cr4();

    // Always enable
    cr4 |= CR4_PAE | CR4_PGE | CR4_MCE;

    // Enable FSGSBASE if available
    if ctx.cpu_state.features.contains(CpuFeatures::FSGSBASE) {
        cr4 |= CR4_FSGSBASE;
    }

    // Enable PCID if available
    if ctx.cpu_state.features.contains(CpuFeatures::PCID) {
        cr4 |= CR4_PCIDE;
    }

    // Enable SMEP if available
    if has_cpuid_feature(7, 0, CpuidReg::Ebx, 7) {
        cr4 |= CR4_SMEP;
    }

    // Enable SMAP if available
    if has_cpuid_feature(7, 0, CpuidReg::Ebx, 20) {
        cr4 |= CR4_SMAP;
    }

    // Enable UMIP if available
    if has_cpuid_feature(7, 0, CpuidReg::Ecx, 2) {
        cr4 |= CR4_UMIP;
    }

    write_cr4(cr4);

    // EFER: Enable NX and syscall
    let mut efer = rdmsr(MSR_EFER);
    efer |= EFER_NXE | EFER_SCE;
    wrmsr(MSR_EFER, efer);

    // Store updated values
    ctx.arch_data.x86.cr0 = read_cr0();
    ctx.arch_data.x86.cr4 = read_cr4();
    ctx.arch_data.x86.efer = rdmsr(MSR_EFER);

    Ok(())
}

// =============================================================================
// SYSCALL CONFIGURATION
// =============================================================================

/// Enable and configure syscall/sysret
pub unsafe fn enable_syscall() -> BootResult<()> {
    // STAR: segment bases for SYSCALL/SYSRET
    // Low 32 bits: SYSCALL EIP (not used in long mode)
    // Bits 32-47: Kernel CS and SS (CS, CS+8 for SS)
    // Bits 48-63: User CS and SS (CS+16 for 32-bit, CS+8 for SS)
    let star = ((KERNEL_CS as u64) << 32) | ((USER_CS32 as u64) << 48);
    wrmsr(MSR_STAR, star);

    // LSTAR: 64-bit SYSCALL target
    // This should be set to the kernel's syscall handler address
    // For now, we leave it at 0 - kernel will set it up
    // wrmsr(MSR_LSTAR, syscall_handler_address);

    // SFMASK: Flags to clear on SYSCALL
    // Clear IF (interrupts), TF (single step), DF (direction)
    wrmsr(MSR_SFMASK, 0x0000_0000_0004_0700);

    Ok(())
}

// =============================================================================
// CPU UTILITIES
// =============================================================================

/// Get the current CPU's APIC ID
pub fn get_apic_id() -> u32 {
    // Try x2APIC first
    if has_cpuid_feature(1, 0, CpuidReg::Ecx, 21) {
        let (_, _, _, edx) = cpuid(0x0B, 0);
        return edx;
    }

    // Fallback to initial APIC ID from CPUID
    let (_, ebx, _, _) = cpuid(1, 0);
    (ebx >> 24) & 0xFF
}

/// Check if we're running on a hypervisor
pub fn is_hypervisor() -> bool {
    has_cpuid_feature(1, 0, CpuidReg::Ecx, 31)
}

/// Get hypervisor vendor if running under virtualization
pub fn hypervisor_vendor() -> Option<[u8; 12]> {
    if !is_hypervisor() {
        return None;
    }

    let (_, ebx, ecx, edx) = cpuid(0x4000_0000, 0);

    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&ecx.to_le_bytes());
    vendor[8..12].copy_from_slice(&edx.to_le_bytes());

    Some(vendor)
}
