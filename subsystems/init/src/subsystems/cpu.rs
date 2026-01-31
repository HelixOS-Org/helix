//! # CPU Subsystem
//!
//! CPU feature detection, initialization, and per-CPU state management.
//! Runs in the Early phase after memory is initialized.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// =============================================================================
// CPU FEATURES
// =============================================================================

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    Intel,
    AMD,
    Arm,
    Apple,
    SiFive,
    Unknown,
}

impl Default for CpuVendor {
    fn default() -> Self {
        Self::Unknown
    }
}

/// CPU features (architecture-independent representation)
#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    // Basic features
    pub fpu: bool,
    pub sse: bool,
    pub sse2: bool,
    pub sse3: bool,
    pub ssse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
    pub avx512: bool,

    // Security features
    pub smep: bool, // Supervisor Mode Execution Prevention
    pub smap: bool, // Supervisor Mode Access Prevention
    pub nx: bool,   // No-Execute bit
    pub umip: bool, // User-Mode Instruction Prevention

    // Virtualization
    pub vmx: bool, // Intel VT-x
    pub svm: bool, // AMD-V

    // Paging features
    pub pae: bool,     // Physical Address Extension
    pub pse: bool,     // Page Size Extension
    pub page1gb: bool, // 1GB pages
    pub pcid: bool,    // Process-Context Identifiers
    pub la57: bool,    // 5-level paging

    // Performance
    pub tsc: bool, // Time Stamp Counter
    pub tsc_deadline: bool,
    pub invariant_tsc: bool,
    pub rdrand: bool,
    pub rdseed: bool,

    // Misc
    pub cmpxchg16b: bool,
    pub popcnt: bool,
    pub xsave: bool,
    pub fsgsbase: bool,
}

// =============================================================================
// PER-CPU DATA
// =============================================================================

/// Maximum supported CPUs
pub const MAX_CPUS: usize = 256;

/// Per-CPU state
#[repr(C)]
pub struct PerCpuData {
    /// CPU ID (local APIC ID or similar)
    pub id: u32,
    /// Is this the BSP?
    pub is_bsp: bool,
    /// Is this CPU online?
    pub online: AtomicBool,
    /// Current CPU state
    pub state: AtomicU32,
    /// Kernel stack pointer
    pub kernel_stack: u64,
    /// TSS pointer (x86_64)
    pub tss_ptr: u64,
    /// Current task pointer
    pub current_task: u64,
    /// Idle task pointer
    pub idle_task: u64,
    /// CPU-local timer ticks
    pub ticks: u64,
    /// Last context switch time
    pub last_switch: u64,
}

impl Default for PerCpuData {
    fn default() -> Self {
        Self {
            id: 0,
            is_bsp: false,
            online: AtomicBool::new(false),
            state: AtomicU32::new(CpuState::Offline as u32),
            kernel_stack: 0,
            tss_ptr: 0,
            current_task: 0,
            idle_task: 0,
            ticks: 0,
            last_switch: 0,
        }
    }
}

/// CPU states
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    Offline  = 0,
    Starting = 1,
    Online   = 2,
    Idle     = 3,
    Running  = 4,
    Halted   = 5,
}

// =============================================================================
// CPU SUBSYSTEM
// =============================================================================

/// CPU subsystem
///
/// Detects CPU features, initializes per-CPU data, and manages SMP.
pub struct CpuSubsystem {
    info: SubsystemInfo,
    vendor: CpuVendor,
    features: CpuFeatures,
    model_name: String,
    cpu_count: u32,
    online_cpus: u32,
    bsp_id: u32,
    per_cpu: Vec<PerCpuData>,
}

static CPU_DEPS: [Dependency; 1] = [Dependency::required("heap")];

impl CpuSubsystem {
    /// Create new CPU subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("cpu", InitPhase::Early)
                .with_priority(700)
                .with_description("CPU feature detection and SMP")
                .with_dependencies(&CPU_DEPS)
                .provides(PhaseCapabilities::CPU)
                .essential(),
            vendor: CpuVendor::Unknown,
            features: CpuFeatures::default(),
            model_name: String::new(),
            cpu_count: 1,
            online_cpus: 1,
            bsp_id: 0,
            per_cpu: Vec::new(),
        }
    }

    /// Get CPU vendor
    pub fn vendor(&self) -> CpuVendor {
        self.vendor
    }

    /// Get CPU features
    pub fn features(&self) -> &CpuFeatures {
        &self.features
    }

    /// Get CPU model name
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// Get total CPU count
    pub fn cpu_count(&self) -> u32 {
        self.cpu_count
    }

    /// Get online CPU count
    pub fn online_cpus(&self) -> u32 {
        self.online_cpus
    }

    /// Get BSP ID
    pub fn bsp_id(&self) -> u32 {
        self.bsp_id
    }

    /// Get per-CPU data
    pub fn get_cpu(&self, id: u32) -> Option<&PerCpuData> {
        self.per_cpu.iter().find(|c| c.id == id)
    }

    /// Detect CPU features for x86_64
    #[cfg(target_arch = "x86_64")]
    fn detect_features_x86(&mut self) {
        use core::arch::x86_64::__cpuid;

        // Get vendor string
        let vendor = unsafe { __cpuid(0) };
        let vendor_str = unsafe {
            let mut bytes = [0u8; 12];
            bytes[0..4].copy_from_slice(&vendor.ebx.to_le_bytes());
            bytes[4..8].copy_from_slice(&vendor.edx.to_le_bytes());
            bytes[8..12].copy_from_slice(&vendor.ecx.to_le_bytes());
            core::str::from_utf8_unchecked(&bytes)
        };

        self.vendor = match vendor_str {
            "GenuineIntel" => CpuVendor::Intel,
            "AuthenticAMD" => CpuVendor::AMD,
            _ => CpuVendor::Unknown,
        };

        let max_leaf = vendor.eax;

        // Basic features (leaf 1)
        if max_leaf >= 1 {
            let leaf1 = unsafe { __cpuid(1) };

            self.features.fpu = (leaf1.edx & (1 << 0)) != 0;
            self.features.pse = (leaf1.edx & (1 << 3)) != 0;
            self.features.tsc = (leaf1.edx & (1 << 4)) != 0;
            self.features.pae = (leaf1.edx & (1 << 6)) != 0;
            self.features.sse = (leaf1.edx & (1 << 25)) != 0;
            self.features.sse2 = (leaf1.edx & (1 << 26)) != 0;

            self.features.sse3 = (leaf1.ecx & (1 << 0)) != 0;
            self.features.ssse3 = (leaf1.ecx & (1 << 9)) != 0;
            self.features.sse4_1 = (leaf1.ecx & (1 << 19)) != 0;
            self.features.sse4_2 = (leaf1.ecx & (1 << 20)) != 0;
            self.features.popcnt = (leaf1.ecx & (1 << 23)) != 0;
            self.features.xsave = (leaf1.ecx & (1 << 26)) != 0;
            self.features.avx = (leaf1.ecx & (1 << 28)) != 0;
            self.features.rdrand = (leaf1.ecx & (1 << 30)) != 0;

            self.features.vmx = (leaf1.ecx & (1 << 5)) != 0;
            self.features.cmpxchg16b = (leaf1.ecx & (1 << 13)) != 0;
            self.features.pcid = (leaf1.ecx & (1 << 17)) != 0;
            self.features.tsc_deadline = (leaf1.ecx & (1 << 24)) != 0;
        }

        // Extended features (leaf 7)
        if max_leaf >= 7 {
            let leaf7 = unsafe { __cpuid(7) };

            self.features.fsgsbase = (leaf7.ebx & (1 << 0)) != 0;
            self.features.smep = (leaf7.ebx & (1 << 7)) != 0;
            self.features.avx2 = (leaf7.ebx & (1 << 5)) != 0;
            self.features.smap = (leaf7.ebx & (1 << 20)) != 0;
            self.features.rdseed = (leaf7.ebx & (1 << 18)) != 0;
            self.features.avx512 = (leaf7.ebx & (1 << 16)) != 0;
            self.features.umip = (leaf7.ecx & (1 << 2)) != 0;
            self.features.la57 = (leaf7.ecx & (1 << 16)) != 0;
        }

        // Extended leaf for NX, 1GB pages, etc.
        let ext_max = unsafe { __cpuid(0x80000000) }.eax;

        if ext_max >= 0x80000001 {
            let ext1 = unsafe { __cpuid(0x80000001) };
            self.features.nx = (ext1.edx & (1 << 20)) != 0;
            self.features.page1gb = (ext1.edx & (1 << 26)) != 0;
            self.features.svm = (ext1.ecx & (1 << 2)) != 0;
        }

        // Invariant TSC
        if ext_max >= 0x80000007 {
            let ext7 = unsafe { __cpuid(0x80000007) };
            self.features.invariant_tsc = (ext7.edx & (1 << 8)) != 0;
        }

        // Get model name
        if ext_max >= 0x80000004 {
            let mut name_bytes = [0u8; 48];
            for (i, leaf) in [0x80000002, 0x80000003, 0x80000004].iter().enumerate() {
                let cpuid = unsafe { __cpuid(*leaf) };
                let offset = i * 16;
                name_bytes[offset..offset + 4].copy_from_slice(&cpuid.eax.to_le_bytes());
                name_bytes[offset + 4..offset + 8].copy_from_slice(&cpuid.ebx.to_le_bytes());
                name_bytes[offset + 8..offset + 12].copy_from_slice(&cpuid.ecx.to_le_bytes());
                name_bytes[offset + 12..offset + 16].copy_from_slice(&cpuid.edx.to_le_bytes());
            }

            if let Ok(name) = core::str::from_utf8(&name_bytes) {
                self.model_name = String::from(name.trim_end_matches('\0').trim());
            }
        }

        // Get APIC ID (BSP)
        let leaf1 = unsafe { __cpuid(1) };
        self.bsp_id = (leaf1.ebx >> 24) as u32;
    }

    /// Detect CPU features for AArch64
    #[cfg(target_arch = "aarch64")]
    fn detect_features_arm(&mut self) {
        use core::arch::aarch64::__mrs;

        // Read MIDR_EL1 for CPU identification
        let midr: u64;
        unsafe {
            core::arch::asm!("mrs {}, midr_el1", out(reg) midr, options(nostack));
        }

        let implementer = ((midr >> 24) & 0xFF) as u8;
        self.vendor = match implementer {
            0x41 => CpuVendor::Arm,
            0x61 => CpuVendor::Apple,
            _ => CpuVendor::Unknown,
        };

        // Read ID_AA64PFR0_EL1 for feature registers
        let pfr0: u64;
        unsafe {
            core::arch::asm!("mrs {}, id_aa64pfr0_el1", out(reg) pfr0, options(nostack));
        }

        // FP/SIMD support
        let fp = ((pfr0 >> 16) & 0xF) as u8;
        self.features.fpu = fp != 0xF;

        // Read ID_AA64ISAR0_EL1 for instruction set features
        let isar0: u64;
        unsafe {
            core::arch::asm!("mrs {}, id_aa64isar0_el1", out(reg) isar0, options(nostack));
        }

        // RNG support (RNDR/RNDRRS)
        let rndr = ((isar0 >> 60) & 0xF) as u8;
        self.features.rdrand = rndr >= 1;

        // Get MPIDR for CPU ID
        let mpidr: u64;
        unsafe {
            core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nostack));
        }
        self.bsp_id = (mpidr & 0xFF) as u32;

        self.model_name = String::from("ARM Cortex CPU");
    }

    /// Detect CPU features for RISC-V
    #[cfg(target_arch = "riscv64")]
    fn detect_features_riscv(&mut self) {
        // Read mvendorid CSR
        let mvendorid: u64;
        unsafe {
            core::arch::asm!("csrr {}, mvendorid", out(reg) mvendorid, options(nostack));
        }

        self.vendor = match mvendorid {
            0x489 => CpuVendor::SiFive,
            _ => CpuVendor::Unknown,
        };

        // Read misa for extensions
        let misa: u64;
        unsafe {
            core::arch::asm!("csrr {}, misa", out(reg) misa, options(nostack));
        }

        // Check for standard extensions (bits 0-25 = A-Z)
        let extensions = misa & 0x03FFFFFF;

        // D extension (double-precision float)
        self.features.fpu = (extensions & (1 << 3)) != 0;

        // Read mhartid for CPU ID
        let mhartid: u64;
        unsafe {
            core::arch::asm!("csrr {}, mhartid", out(reg) mhartid, options(nostack));
        }
        self.bsp_id = mhartid as u32;

        self.model_name = String::from("RISC-V CPU");
    }

    /// Stub for unsupported architectures
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    fn detect_features_stub(&mut self) {
        self.vendor = CpuVendor::Unknown;
        self.model_name = String::from("Unknown CPU");
    }

    /// Initialize BSP (Bootstrap Processor)
    fn init_bsp(&mut self) {
        let mut bsp_data = PerCpuData::default();
        bsp_data.id = self.bsp_id;
        bsp_data.is_bsp = true;
        bsp_data.online.store(true, Ordering::SeqCst);
        bsp_data
            .state
            .store(CpuState::Online as u32, Ordering::SeqCst);

        self.per_cpu.push(bsp_data);
    }

    /// Enable CPU security features
    fn enable_security_features(&self, ctx: &mut InitContext) {
        #[cfg(target_arch = "x86_64")]
        {
            // Enable SMEP if available
            if self.features.smep {
                unsafe {
                    let mut cr4: u64;
                    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack));
                    cr4 |= 1 << 20; // SMEP bit
                    core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack));
                }
                ctx.debug("Enabled SMEP");
            }

            // Enable SMAP if available
            if self.features.smap {
                unsafe {
                    let mut cr4: u64;
                    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack));
                    cr4 |= 1 << 21; // SMAP bit
                    core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack));
                }
                ctx.debug("Enabled SMAP");
            }

            // Enable UMIP if available
            if self.features.umip {
                unsafe {
                    let mut cr4: u64;
                    core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nostack));
                    cr4 |= 1 << 11; // UMIP bit
                    core::arch::asm!("mov cr4, {}", in(reg) cr4, options(nostack));
                }
                ctx.debug("Enabled UMIP");
            }

            // Enable NX if available (EFER.NXE)
            if self.features.nx {
                unsafe {
                    let mut efer: u64;
                    core::arch::asm!(
                        "rdmsr",
                        in("ecx") 0xC0000080u32, // EFER MSR
                        out("eax") _,
                        out("edx") _,
                        options(nostack)
                    );
                    // Set NXE bit
                    core::arch::asm!(
                        "wrmsr",
                        in("ecx") 0xC0000080u32,
                        in("eax") (1u32 << 11), // NXE
                        in("edx") 0u32,
                        options(nostack)
                    );
                }
                ctx.debug("Enabled NX");
            }
        }
    }
}

impl Default for CpuSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for CpuSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing CPU subsystem");

        // Detect CPU features
        #[cfg(target_arch = "x86_64")]
        self.detect_features_x86();

        #[cfg(target_arch = "aarch64")]
        self.detect_features_arm();

        #[cfg(target_arch = "riscv64")]
        self.detect_features_riscv();

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        self.detect_features_stub();

        ctx.info(alloc::format!(
            "CPU: {} ({:?})",
            self.model_name,
            self.vendor
        ));

        // Log important features
        let mut feature_list = alloc::vec![];
        if self.features.sse {
            feature_list.push("SSE");
        }
        if self.features.sse2 {
            feature_list.push("SSE2");
        }
        if self.features.avx {
            feature_list.push("AVX");
        }
        if self.features.avx2 {
            feature_list.push("AVX2");
        }
        if self.features.avx512 {
            feature_list.push("AVX-512");
        }
        if self.features.smep {
            feature_list.push("SMEP");
        }
        if self.features.smap {
            feature_list.push("SMAP");
        }
        if self.features.nx {
            feature_list.push("NX");
        }

        ctx.debug(alloc::format!("Features: {}", feature_list.join(", ")));

        // Enable security features
        self.enable_security_features(ctx);

        // Initialize BSP
        self.init_bsp();
        ctx.debug(alloc::format!("BSP ID: {}", self.bsp_id));

        // TODO: Parse ACPI/DT for CPU count and start APs
        // For now, assume single CPU
        self.cpu_count = 1;
        self.online_cpus = 1;

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("CPU subsystem shutdown");

        // Mark all CPUs as offline (except current)
        for cpu in &self.per_cpu {
            if !cpu.is_bsp {
                cpu.online.store(false, Ordering::SeqCst);
                cpu.state.store(CpuState::Offline as u32, Ordering::SeqCst);
            }
        }

        Ok(())
    }

    fn health_check(&self) -> InitResult<()> {
        // Verify BSP is still online
        if let Some(bsp) = self.per_cpu.iter().find(|c| c.is_bsp) {
            if !bsp.online.load(Ordering::SeqCst) {
                return Err(InitError::new(ErrorKind::SubsystemFailed, "BSP is offline"));
            }
        }
        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_subsystem() {
        let sub = CpuSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Early);
        assert!(sub.info().essential);
        assert_eq!(sub.cpu_count(), 1);
    }

    #[test]
    fn test_per_cpu_data() {
        let data = PerCpuData::default();
        assert_eq!(data.id, 0);
        assert!(!data.is_bsp);
        assert!(!data.online.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cpu_state() {
        assert_eq!(CpuState::Offline as u32, 0);
        assert_eq!(CpuState::Online as u32, 2);
    }
}
