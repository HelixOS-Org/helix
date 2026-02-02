//! # Model-Specific Register (MSR) Framework
//!
//! Complete MSR definitions and safe accessors for x86_64.
//!
//! ## Overview
//!
//! MSRs are special registers that control various CPU features, performance
//! monitoring, debugging, and system configuration. This module provides:
//!
//! - Type-safe MSR definitions
//! - Raw read/write functions
//! - Structured MSR types with field accessors
//! - Documented register layouts
//!
//! ## Safety
//!
//! MSR access requires ring 0 privilege. Incorrect MSR writes can crash
//! the system or cause undefined behavior. All write operations are unsafe.
//!
//! ## Categories
//!
//! - **System**: EFER, PAT, APIC_BASE
//! - **Segment**: FS_BASE, GS_BASE, KERNEL_GS_BASE
//! - **Syscall**: STAR, LSTAR, CSTAR, SFMASK
//! - **Timing**: TSC, TSC_AUX, PERF_*
//! - **Power**: PERF_CTL, MPERF, APERF
//! - **Virtualization**: VMX_*, SVM_*
//! - **Debugging**: DEBUGCTL, LBR_*

use core::arch::asm;

// =============================================================================
// RAW MSR ACCESS
// =============================================================================

/// Read a Model-Specific Register
///
/// # Safety
/// - Must be in ring 0
/// - MSR must exist (otherwise #GP)
#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Write a Model-Specific Register
///
/// # Safety
/// - Must be in ring 0
/// - MSR must exist and be writable
/// - Value must be valid for the MSR
#[inline]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }
}

// =============================================================================
// MSR ADDRESSES
// =============================================================================

/// MSR address namespace
pub mod addr {
    //! MSR addresses organized by category

    // =========================================================================
    // Time Stamp Counter
    // =========================================================================

    /// Time Stamp Counter
    pub const IA32_TSC: u32 = 0x0000_0010;
    /// TSC auxiliary data (processor ID for RDTSCP)
    pub const IA32_TSC_AUX: u32 = 0xC000_0103;
    /// TSC adjust (for synchronization across cores)
    pub const IA32_TSC_ADJUST: u32 = 0x0000_003B;
    /// TSC deadline for APIC timer
    pub const IA32_TSC_DEADLINE: u32 = 0x0000_06E0;

    // =========================================================================
    // APIC
    // =========================================================================

    /// APIC base address and state
    pub const IA32_APIC_BASE: u32 = 0x0000_001B;
    /// x2APIC registers start (0x800-0x8FF)
    pub const IA32_X2APIC_BASE: u32 = 0x0000_0800;
    /// x2APIC ID
    pub const IA32_X2APIC_APICID: u32 = 0x0000_0802;
    /// x2APIC Version
    pub const IA32_X2APIC_VERSION: u32 = 0x0000_0803;
    /// x2APIC Task Priority
    pub const IA32_X2APIC_TPR: u32 = 0x0000_0808;
    /// x2APIC Processor Priority
    pub const IA32_X2APIC_PPR: u32 = 0x0000_080A;
    /// x2APIC EOI
    pub const IA32_X2APIC_EOI: u32 = 0x0000_080B;
    /// x2APIC Logical Destination
    pub const IA32_X2APIC_LDR: u32 = 0x0000_080D;
    /// x2APIC Spurious Interrupt Vector
    pub const IA32_X2APIC_SIVR: u32 = 0x0000_080F;
    /// x2APIC In-Service Register 0-7
    pub const IA32_X2APIC_ISR0: u32 = 0x0000_0810;
    /// x2APIC Trigger Mode Register 0-7
    pub const IA32_X2APIC_TMR0: u32 = 0x0000_0818;
    /// x2APIC Interrupt Request Register 0-7
    pub const IA32_X2APIC_IRR0: u32 = 0x0000_0820;
    /// x2APIC Error Status
    pub const IA32_X2APIC_ESR: u32 = 0x0000_0828;
    /// x2APIC LVT CMCI
    pub const IA32_X2APIC_LVT_CMCI: u32 = 0x0000_082F;
    /// x2APIC Interrupt Command
    pub const IA32_X2APIC_ICR: u32 = 0x0000_0830;
    /// x2APIC LVT Timer
    pub const IA32_X2APIC_LVT_TIMER: u32 = 0x0000_0832;
    /// x2APIC LVT Thermal Sensor
    pub const IA32_X2APIC_LVT_THERMAL: u32 = 0x0000_0833;
    /// x2APIC LVT Performance Monitoring
    pub const IA32_X2APIC_LVT_PMI: u32 = 0x0000_0834;
    /// x2APIC LVT LINT0
    pub const IA32_X2APIC_LVT_LINT0: u32 = 0x0000_0835;
    /// x2APIC LVT LINT1
    pub const IA32_X2APIC_LVT_LINT1: u32 = 0x0000_0836;
    /// x2APIC LVT Error
    pub const IA32_X2APIC_LVT_ERROR: u32 = 0x0000_0837;
    /// x2APIC Initial Count
    pub const IA32_X2APIC_INIT_COUNT: u32 = 0x0000_0838;
    /// x2APIC Current Count
    pub const IA32_X2APIC_CUR_COUNT: u32 = 0x0000_0839;
    /// x2APIC Divide Configuration
    pub const IA32_X2APIC_DIV_CONF: u32 = 0x0000_083E;
    /// x2APIC Self IPI
    pub const IA32_X2APIC_SELF_IPI: u32 = 0x0000_083F;

    // =========================================================================
    // System Configuration
    // =========================================================================

    /// Extended Feature Enable Register
    pub const IA32_EFER: u32 = 0xC000_0080;
    /// STAR - Syscall Target Address
    pub const IA32_STAR: u32 = 0xC000_0081;
    /// LSTAR - Long Mode Syscall Target
    pub const IA32_LSTAR: u32 = 0xC000_0082;
    /// CSTAR - Compatibility Mode Syscall Target (unused in 64-bit)
    pub const IA32_CSTAR: u32 = 0xC000_0083;
    /// SFMASK - Syscall Flag Mask
    pub const IA32_SFMASK: u32 = 0xC000_0084;
    /// Page Attribute Table
    pub const IA32_PAT: u32 = 0x0000_0277;
    /// Memory Type Range Registers (MTRR)
    pub const IA32_MTRRCAP: u32 = 0x0000_00FE;
    /// MTRR Default Type
    pub const IA32_MTRR_DEF_TYPE: u32 = 0x0000_02FF;
    /// MTRR Physical Base 0-7
    pub const IA32_MTRR_PHYSBASE0: u32 = 0x0000_0200;
    /// MTRR Physical Mask 0-7
    pub const IA32_MTRR_PHYSMASK0: u32 = 0x0000_0201;

    // =========================================================================
    // Segment Bases
    // =========================================================================

    /// FS segment base
    pub const IA32_FS_BASE: u32 = 0xC000_0100;
    /// GS segment base
    pub const IA32_GS_BASE: u32 = 0xC000_0101;
    /// Kernel GS base (swapped by SWAPGS)
    pub const IA32_KERNEL_GS_BASE: u32 = 0xC000_0102;

    // =========================================================================
    // SYSENTER (legacy 32-bit)
    // =========================================================================

    /// SYSENTER CS
    pub const IA32_SYSENTER_CS: u32 = 0x0000_0174;
    /// SYSENTER ESP
    pub const IA32_SYSENTER_ESP: u32 = 0x0000_0175;
    /// SYSENTER EIP
    pub const IA32_SYSENTER_EIP: u32 = 0x0000_0176;

    // =========================================================================
    // Performance Monitoring
    // =========================================================================

    /// Performance Event Select 0-3
    pub const IA32_PERFEVTSEL0: u32 = 0x0000_0186;
    /// Fixed Counter Control
    pub const IA32_FIXED_CTR_CTRL: u32 = 0x0000_038D;
    /// Global Performance Counter Status
    pub const IA32_PERF_GLOBAL_STATUS: u32 = 0x0000_038E;
    /// Global Performance Counter Control
    pub const IA32_PERF_GLOBAL_CTRL: u32 = 0x0000_038F;
    /// Global Performance Counter Overflow Control
    pub const IA32_PERF_GLOBAL_OVF_CTRL: u32 = 0x0000_0390;
    /// Performance Counter 0-7
    pub const IA32_PMC0: u32 = 0x0000_00C1;
    /// Fixed Counter 0-2
    pub const IA32_FIXED_CTR0: u32 = 0x0000_0309;
    /// Performance Capabilities
    pub const IA32_PERF_CAPABILITIES: u32 = 0x0000_0345;

    // =========================================================================
    // Power Management
    // =========================================================================

    /// Maximum Performance Frequency Clock Count
    pub const IA32_MPERF: u32 = 0x0000_00E7;
    /// Actual Performance Frequency Clock Count
    pub const IA32_APERF: u32 = 0x0000_00E8;
    /// Performance Control (P-state)
    pub const IA32_PERF_CTL: u32 = 0x0000_0199;
    /// Performance Status
    pub const IA32_PERF_STATUS: u32 = 0x0000_0198;
    /// Power Control
    pub const IA32_POWER_CTL: u32 = 0x0000_01FC;
    /// Platform Energy Performance Bias
    pub const IA32_ENERGY_PERF_BIAS: u32 = 0x0000_01B0;
    /// Package Thermal Status
    pub const IA32_PACKAGE_THERM_STATUS: u32 = 0x0000_01B1;
    /// Package Thermal Interrupt
    pub const IA32_PACKAGE_THERM_INTERRUPT: u32 = 0x0000_01B2;
    /// HWP Capabilities
    pub const IA32_HWP_CAPABILITIES: u32 = 0x0000_0771;
    /// HWP Request
    pub const IA32_HWP_REQUEST: u32 = 0x0000_0774;

    // =========================================================================
    // Debugging
    // =========================================================================

    /// Debug Control
    pub const IA32_DEBUGCTL: u32 = 0x0000_01D9;
    /// Last Branch Record TOS
    pub const IA32_LBR_TOS: u32 = 0x0000_01C9;
    /// Last Branch Record From 0-15
    pub const IA32_LBR_FROM_0: u32 = 0x0000_0680;
    /// Last Branch Record To 0-15
    pub const IA32_LBR_TO_0: u32 = 0x0000_06C0;
    /// DS Area
    pub const IA32_DS_AREA: u32 = 0x0000_0600;

    // =========================================================================
    // Machine Check
    // =========================================================================

    /// Machine Check Global Capabilities
    pub const IA32_MCG_CAP: u32 = 0x0000_0179;
    /// Machine Check Global Status
    pub const IA32_MCG_STATUS: u32 = 0x0000_017A;
    /// Machine Check Global Control
    pub const IA32_MCG_CTL: u32 = 0x0000_017B;
    /// Machine Check Bank 0 Control
    pub const IA32_MC0_CTL: u32 = 0x0000_0400;
    /// Machine Check Bank 0 Status
    pub const IA32_MC0_STATUS: u32 = 0x0000_0401;
    /// Machine Check Bank 0 Address
    pub const IA32_MC0_ADDR: u32 = 0x0000_0402;
    /// Machine Check Bank 0 Misc
    pub const IA32_MC0_MISC: u32 = 0x0000_0403;

    // =========================================================================
    // Intel VMX
    // =========================================================================

    /// VMX Basic Capabilities
    pub const IA32_VMX_BASIC: u32 = 0x0000_0480;
    /// VMX Pin-Based Controls
    pub const IA32_VMX_PINBASED_CTLS: u32 = 0x0000_0481;
    /// VMX Processor-Based Controls
    pub const IA32_VMX_PROCBASED_CTLS: u32 = 0x0000_0482;
    /// VMX Exit Controls
    pub const IA32_VMX_EXIT_CTLS: u32 = 0x0000_0483;
    /// VMX Entry Controls
    pub const IA32_VMX_ENTRY_CTLS: u32 = 0x0000_0484;
    /// VMX Miscellaneous Data
    pub const IA32_VMX_MISC: u32 = 0x0000_0485;
    /// VMX CR0 Fixed 0
    pub const IA32_VMX_CR0_FIXED0: u32 = 0x0000_0486;
    /// VMX CR0 Fixed 1
    pub const IA32_VMX_CR0_FIXED1: u32 = 0x0000_0487;
    /// VMX CR4 Fixed 0
    pub const IA32_VMX_CR4_FIXED0: u32 = 0x0000_0488;
    /// VMX CR4 Fixed 1
    pub const IA32_VMX_CR4_FIXED1: u32 = 0x0000_0489;
    /// VMX Secondary Processor-Based Controls
    pub const IA32_VMX_PROCBASED_CTLS2: u32 = 0x0000_048B;
    /// VMX EPT/VPID Capabilities
    pub const IA32_VMX_EPT_VPID_CAP: u32 = 0x0000_048C;
    /// VMX True Pin-Based Controls
    pub const IA32_VMX_TRUE_PINBASED_CTLS: u32 = 0x0000_048D;
    /// VMX True Processor-Based Controls
    pub const IA32_VMX_TRUE_PROCBASED_CTLS: u32 = 0x0000_048E;
    /// VMX True Exit Controls
    pub const IA32_VMX_TRUE_EXIT_CTLS: u32 = 0x0000_048F;
    /// VMX True Entry Controls
    pub const IA32_VMX_TRUE_ENTRY_CTLS: u32 = 0x0000_0490;
    /// VMX VMFUNC Controls
    pub const IA32_VMX_VMFUNC: u32 = 0x0000_0491;

    // =========================================================================
    // AMD SVM
    // =========================================================================

    /// VM Control Register (AMD)
    pub const SVM_VM_CR: u32 = 0xC001_0114;
    /// VM Host Save Physical Address (AMD)
    pub const SVM_VM_HSAVE_PA: u32 = 0xC001_0117;

    // =========================================================================
    // Security & Speculation Mitigations
    // =========================================================================

    /// Speculation Control
    pub const IA32_SPEC_CTRL: u32 = 0x0000_0048;
    /// Prediction Command
    pub const IA32_PRED_CMD: u32 = 0x0000_0049;
    /// Architecture Capabilities
    pub const IA32_ARCH_CAPABILITIES: u32 = 0x0000_010A;
    /// Flush Command
    pub const IA32_FLUSH_CMD: u32 = 0x0000_010B;
    /// Core Capabilities
    pub const IA32_CORE_CAPABILITIES: u32 = 0x0000_00CF;
    /// Extended Feature Disable (AMD)
    pub const AMD64_DE_CFG: u32 = 0xC001_1029;

    // =========================================================================
    // Platform Information
    // =========================================================================

    /// Platform Information
    pub const MSR_PLATFORM_INFO: u32 = 0x0000_00CE;
    /// Miscellaneous Enable
    pub const IA32_MISC_ENABLE: u32 = 0x0000_01A0;
    /// Feature Control (for VMX, SGX enable)
    pub const IA32_FEATURE_CONTROL: u32 = 0x0000_003A;
    /// BIOS Update Trigger
    pub const IA32_BIOS_UPDT_TRIG: u32 = 0x0000_0079;
    /// BIOS Signature
    pub const IA32_BIOS_SIGN_ID: u32 = 0x0000_008B;
    /// SMI Count
    pub const MSR_SMI_COUNT: u32 = 0x0000_0034;
}

// =============================================================================
// EFER - Extended Feature Enable Register
// =============================================================================

bitflags::bitflags! {
    /// EFER - Extended Feature Enable Register (MSR 0xC0000080)
    ///
    /// Controls long mode, syscall, and NX bit.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Efer: u64 {
        /// System Call Extensions (SYSCALL/SYSRET)
        const SCE = 1 << 0;
        /// Long Mode Enable
        const LME = 1 << 8;
        /// Long Mode Active (read-only, set by hardware)
        const LMA = 1 << 10;
        /// No-Execute Enable
        const NXE = 1 << 11;
        /// Secure Virtual Machine Enable (AMD)
        const SVME = 1 << 12;
        /// Long Mode Segment Limit Enable (AMD)
        const LMSLE = 1 << 13;
        /// Fast FXSAVE/FXRSTOR (AMD)
        const FFXSR = 1 << 14;
        /// Translation Cache Extension (AMD)
        const TCE = 1 << 15;
    }
}

impl Efer {
    /// Read current EFER value
    pub fn read() -> Self {
        Self::from_bits_truncate(unsafe { rdmsr(addr::IA32_EFER) })
    }

    /// Write EFER value
    ///
    /// # Safety
    /// Incorrect values can crash the system or disable long mode.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_EFER, self.bits());
        }
    }

    /// Update EFER with a closure
    ///
    /// # Safety
    /// See `write`.
    pub unsafe fn update<F: FnOnce(&mut Self)>(f: F) {
        let mut efer = Self::read();
        f(&mut efer);
        unsafe {
            efer.write();
        }
    }

    /// Check if syscall is enabled
    pub fn syscall_enabled(self) -> bool {
        self.contains(Self::SCE)
    }

    /// Check if long mode is enabled
    pub fn long_mode_enabled(self) -> bool {
        self.contains(Self::LME)
    }

    /// Check if long mode is active
    pub fn long_mode_active(self) -> bool {
        self.contains(Self::LMA)
    }

    /// Check if NX bit is enabled
    pub fn nx_enabled(self) -> bool {
        self.contains(Self::NXE)
    }
}

// =============================================================================
// APIC_BASE - APIC Base Address Register
// =============================================================================

bitflags::bitflags! {
    /// Flags portion of APIC_BASE MSR
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ApicBaseFlags: u64 {
        /// This is the Bootstrap Processor
        const BSP = 1 << 8;
        /// x2APIC Mode Enable
        const X2APIC = 1 << 10;
        /// APIC Global Enable
        const ENABLE = 1 << 11;
    }
}

/// APIC Base register wrapper
#[derive(Debug, Clone, Copy)]
pub struct ApicBase {
    value: u64,
}

impl ApicBase {
    /// Base address mask (bits 12-51)
    const BASE_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    /// Read current APIC base
    pub fn read() -> Self {
        Self {
            value: unsafe { rdmsr(addr::IA32_APIC_BASE) },
        }
    }

    /// Write APIC base
    ///
    /// # Safety
    /// Incorrect values can disable the APIC or cause system instability.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_APIC_BASE, self.value);
        }
    }

    /// Get the APIC base address
    pub fn base_address(self) -> u64 {
        self.value & Self::BASE_MASK
    }

    /// Set the APIC base address
    pub fn set_base_address(&mut self, addr: u64) {
        self.value = (self.value & !Self::BASE_MASK) | (addr & Self::BASE_MASK);
    }

    /// Get flags
    pub fn flags(self) -> ApicBaseFlags {
        ApicBaseFlags::from_bits_truncate(self.value)
    }

    /// Check if this is the bootstrap processor
    pub fn is_bsp(self) -> bool {
        self.flags().contains(ApicBaseFlags::BSP)
    }

    /// Check if APIC is enabled globally
    pub fn is_enabled(self) -> bool {
        self.flags().contains(ApicBaseFlags::ENABLE)
    }

    /// Check if x2APIC mode is enabled
    pub fn is_x2apic(self) -> bool {
        self.flags().contains(ApicBaseFlags::X2APIC)
    }

    /// Enable the APIC
    pub fn enable(&mut self) {
        self.value |= ApicBaseFlags::ENABLE.bits();
    }

    /// Enable x2APIC mode
    pub fn enable_x2apic(&mut self) {
        self.value |= ApicBaseFlags::X2APIC.bits() | ApicBaseFlags::ENABLE.bits();
    }

    /// Disable the APIC (dangerous!)
    pub fn disable(&mut self) {
        self.value &= !ApicBaseFlags::ENABLE.bits();
    }
}

// =============================================================================
// PAT - Page Attribute Table
// =============================================================================

/// Memory type for PAT entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PatMemoryType {
    /// Uncacheable (UC)
    Uncacheable    = 0x00,
    /// Write Combining (WC)
    WriteCombining = 0x01,
    /// Write Through (WT)
    WriteThrough   = 0x04,
    /// Write Protected (WP)
    WriteProtected = 0x05,
    /// Write Back (WB)
    WriteBack      = 0x06,
    /// Uncached (UC-)
    Uncached       = 0x07,
}

impl PatMemoryType {
    fn from_u8(value: u8) -> Self {
        match value {
            0x00 => Self::Uncacheable,
            0x01 => Self::WriteCombining,
            0x04 => Self::WriteThrough,
            0x05 => Self::WriteProtected,
            0x06 => Self::WriteBack,
            0x07 => Self::Uncached,
            _ => Self::Uncacheable,
        }
    }
}

/// Page Attribute Table configuration
#[derive(Debug, Clone, Copy)]
pub struct Pat {
    value: u64,
}

impl Pat {
    /// Read current PAT value
    pub fn read() -> Self {
        Self {
            value: unsafe { rdmsr(addr::IA32_PAT) },
        }
    }

    /// Write PAT value
    ///
    /// # Safety
    /// Incorrect values can cause memory corruption.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_PAT, self.value);
        }
    }

    /// Get entry at index (0-7)
    pub fn entry(self, index: usize) -> PatMemoryType {
        assert!(index < 8);
        PatMemoryType::from_u8(((self.value >> (index * 8)) & 0xFF) as u8)
    }

    /// Set entry at index (0-7)
    pub fn set_entry(&mut self, index: usize, mem_type: PatMemoryType) {
        assert!(index < 8);
        let shift = index * 8;
        self.value = (self.value & !(0xFF << shift)) | ((mem_type as u64) << shift);
    }

    /// Create default PAT configuration
    ///
    /// Index | Memory Type
    /// ------|------------
    /// 0     | WB (Write Back)
    /// 1     | WT (Write Through)
    /// 2     | UC- (Uncached)
    /// 3     | UC (Uncacheable)
    /// 4     | WB (Write Back)
    /// 5     | WT (Write Through)
    /// 6     | UC- (Uncached)
    /// 7     | WC (Write Combining)
    pub fn default_config() -> Self {
        Self {
            value: 0x0007_0106_0007_0106,
        }
    }

    /// Create a custom configuration optimized for kernel use
    ///
    /// Index | Memory Type
    /// ------|------------
    /// 0     | WB (Write Back) - Normal memory
    /// 1     | WC (Write Combining) - Framebuffer
    /// 2     | UC- (Uncached) - MMIO
    /// 3     | UC (Uncacheable) - Strict MMIO
    /// 4     | WT (Write Through) - Logging
    /// 5     | WP (Write Protected) - ROM
    /// 6     | WB (Write Back)
    /// 7     | UC- (Uncached)
    pub fn kernel_config() -> Self {
        Self {
            value: 0x0706_0504_0701_0006,
        }
    }
}

// =============================================================================
// SYSCALL CONFIGURATION
// =============================================================================

/// STAR register for SYSCALL/SYSRET
#[derive(Debug, Clone, Copy)]
pub struct Star {
    value: u64,
}

impl Star {
    /// Read current STAR value
    pub fn read() -> Self {
        Self {
            value: unsafe { rdmsr(addr::IA32_STAR) },
        }
    }

    /// Write STAR value
    ///
    /// # Safety
    /// Must set correct segment selectors.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_STAR, self.value);
        }
    }

    /// Create a new STAR configuration
    ///
    /// # Arguments
    /// * `syscall_cs` - CS for SYSCALL (ring 0)
    /// * `sysret_cs` - CS base for SYSRET (ring 3)
    pub fn new(syscall_cs: u16, sysret_cs: u16) -> Self {
        let value = ((sysret_cs as u64) << 48) | ((syscall_cs as u64) << 32);
        Self { value }
    }

    /// Get the SYSCALL CS selector
    pub fn syscall_cs(self) -> u16 {
        ((self.value >> 32) & 0xFFFF) as u16
    }

    /// Get the SYSRET CS selector base
    pub fn sysret_cs(self) -> u16 {
        ((self.value >> 48) & 0xFFFF) as u16
    }
}

/// LSTAR - Long Mode SYSCALL Target
pub struct Lstar;

impl Lstar {
    /// Read current LSTAR value
    pub fn read() -> u64 {
        unsafe { rdmsr(addr::IA32_LSTAR) }
    }

    /// Write LSTAR (syscall entry point)
    ///
    /// # Safety
    /// Must point to valid syscall handler code.
    pub unsafe fn write(addr: u64) {
        unsafe {
            wrmsr(addr::IA32_LSTAR, addr);
        }
    }
}

/// CSTAR - Compatibility Mode SYSCALL Target (32-bit syscalls)
pub struct Cstar;

impl Cstar {
    /// Read current CSTAR value
    pub fn read() -> u64 {
        unsafe { rdmsr(addr::IA32_CSTAR) }
    }

    /// Write CSTAR (32-bit syscall entry point)
    ///
    /// # Safety
    /// Must point to valid syscall handler code.
    pub unsafe fn write(addr: u64) {
        unsafe {
            wrmsr(addr::IA32_CSTAR, addr);
        }
    }
}

/// SFMASK - Syscall Flag Mask
pub struct SfMask;

impl SfMask {
    /// Read current SFMASK value
    pub fn read() -> u64 {
        unsafe { rdmsr(addr::IA32_SFMASK) }
    }

    /// Write SFMASK (RFLAGS mask for syscall)
    ///
    /// # Safety
    /// Must be a valid flags mask.
    pub unsafe fn write(mask: u64) {
        unsafe {
            wrmsr(addr::IA32_SFMASK, mask);
        }
    }

    /// Standard mask: disable IF, TF, DF, AC, NT
    pub const STANDARD: u64 = 0x4700;
}

// =============================================================================
// SEGMENT BASES
// =============================================================================

/// FS Base register
pub struct FsBase;

impl FsBase {
    /// Read current FS base
    pub fn read() -> u64 {
        unsafe { rdmsr(addr::IA32_FS_BASE) }
    }

    /// Write FS base
    ///
    /// # Safety
    /// Must be a valid canonical address.
    pub unsafe fn write(base: u64) {
        unsafe {
            wrmsr(addr::IA32_FS_BASE, base);
        }
    }

    /// Write FS base using WRFSBASE (if available)
    ///
    /// # Safety
    /// Requires FSGSBASE feature and CR4.FSGSBASE bit set.
    #[inline]
    pub unsafe fn write_fast(base: u64) {
        unsafe {
            asm!("wrfsbase {}", in(reg) base, options(nostack, preserves_flags));
        }
    }

    /// Read FS base using RDFSBASE (if available)
    ///
    /// # Safety
    /// Requires FSGSBASE feature and CR4.FSGSBASE bit set.
    #[inline]
    pub unsafe fn read_fast() -> u64 {
        let base: u64;
        unsafe {
            asm!("rdfsbase {}", out(reg) base, options(nostack, preserves_flags));
        }
        base
    }
}

/// GS Base register
pub struct GsBase;

impl GsBase {
    /// Read current GS base
    pub fn read() -> u64 {
        unsafe { rdmsr(addr::IA32_GS_BASE) }
    }

    /// Write GS base
    ///
    /// # Safety
    /// Must be a valid canonical address.
    pub unsafe fn write(base: u64) {
        unsafe {
            wrmsr(addr::IA32_GS_BASE, base);
        }
    }

    /// Write GS base using WRGSBASE (if available)
    ///
    /// # Safety
    /// Requires FSGSBASE feature and CR4.FSGSBASE bit set.
    #[inline]
    pub unsafe fn write_fast(base: u64) {
        unsafe {
            asm!("wrgsbase {}", in(reg) base, options(nostack, preserves_flags));
        }
    }

    /// Read GS base using RDGSBASE (if available)
    ///
    /// # Safety
    /// Requires FSGSBASE feature and CR4.FSGSBASE bit set.
    #[inline]
    pub unsafe fn read_fast() -> u64 {
        let base: u64;
        unsafe {
            asm!("rdgsbase {}", out(reg) base, options(nostack, preserves_flags));
        }
        base
    }
}

/// Kernel GS Base (swapped by SWAPGS)
pub struct KernelGsBase;

impl KernelGsBase {
    /// Read kernel GS base
    pub fn read() -> u64 {
        unsafe { rdmsr(addr::IA32_KERNEL_GS_BASE) }
    }

    /// Write kernel GS base
    ///
    /// # Safety
    /// Must be a valid canonical address.
    pub unsafe fn write(base: u64) {
        unsafe {
            wrmsr(addr::IA32_KERNEL_GS_BASE, base);
        }
    }
}

/// Execute SWAPGS instruction
///
/// # Safety
/// Should only be called at syscall/interrupt entry/exit boundaries.
#[inline]
pub unsafe fn swapgs() {
    unsafe {
        asm!("swapgs", options(nostack, preserves_flags));
    }
}

// =============================================================================
// TSC OPERATIONS
// =============================================================================

/// Time Stamp Counter operations
pub struct Tsc;

impl Tsc {
    /// Read TSC value
    #[inline]
    pub fn read() -> u64 {
        let (lo, hi): (u32, u32);
        unsafe {
            asm!(
                "rdtsc",
                out("eax") lo,
                out("edx") hi,
                options(nomem, nostack, preserves_flags)
            );
        }
        ((hi as u64) << 32) | (lo as u64)
    }

    /// Read TSC with processor ID (RDTSCP)
    ///
    /// Returns (tsc, processor_id)
    #[inline]
    pub fn read_with_aux() -> (u64, u32) {
        let (lo, hi, aux): (u32, u32, u32);
        unsafe {
            asm!(
                "rdtscp",
                out("eax") lo,
                out("edx") hi,
                out("ecx") aux,
                options(nomem, nostack, preserves_flags)
            );
        }
        (((hi as u64) << 32) | (lo as u64), aux)
    }

    /// Read TSC with serialization (LFENCE + RDTSC)
    #[inline]
    pub fn read_serialized() -> u64 {
        unsafe {
            asm!("lfence", options(nostack, preserves_flags));
        }
        Self::read()
    }

    /// Write TSC value
    ///
    /// # Safety
    /// Can desynchronize TSC across cores.
    pub unsafe fn write(value: u64) {
        unsafe {
            wrmsr(addr::IA32_TSC, value);
        }
    }

    /// Get TSC_AUX value (processor ID)
    pub fn aux() -> u32 {
        (unsafe { rdmsr(addr::IA32_TSC_AUX) }) as u32
    }

    /// Set TSC_AUX value
    ///
    /// # Safety
    /// Typically set to processor ID.
    pub unsafe fn set_aux(value: u32) {
        unsafe {
            wrmsr(addr::IA32_TSC_AUX, value as u64);
        }
    }

    /// Get TSC adjust value
    pub fn adjust() -> i64 {
        unsafe { rdmsr(addr::IA32_TSC_ADJUST) as i64 }
    }

    /// Set TSC adjust value
    ///
    /// # Safety
    /// Can affect time measurement.
    pub unsafe fn set_adjust(value: i64) {
        unsafe {
            wrmsr(addr::IA32_TSC_ADJUST, value as u64);
        }
    }

    /// Get TSC deadline value
    pub fn deadline() -> u64 {
        unsafe { rdmsr(addr::IA32_TSC_DEADLINE) }
    }

    /// Set TSC deadline
    ///
    /// # Safety
    /// Must have TSC-deadline mode enabled in APIC timer.
    pub unsafe fn set_deadline(value: u64) {
        unsafe {
            wrmsr(addr::IA32_TSC_DEADLINE, value);
        }
    }
}

// =============================================================================
// MISC ENABLE
// =============================================================================

bitflags::bitflags! {
    /// IA32_MISC_ENABLE flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MiscEnable: u64 {
        /// Fast-Strings Enable
        const FAST_STRING = 1 << 0;
        /// TCC (Thermal Control Circuit) Enable
        const TCC = 1 << 1;
        /// Performance Monitoring Available
        const PERF_MON = 1 << 7;
        /// Branch Trace Storage Unavailable
        const BTS_UNAVAIL = 1 << 11;
        /// PEBS Unavailable
        const PEBS_UNAVAIL = 1 << 12;
        /// Enhanced Intel SpeedStep Enable
        const SPEEDSTEP = 1 << 16;
        /// MONITOR/MWAIT Enable
        const MONITOR = 1 << 18;
        /// Limit CPUID Maxval
        const LIMIT_CPUID = 1 << 22;
        /// xTPR Message Disable
        const XTPR_DISABLE = 1 << 23;
        /// XD Bit Disable
        const XD_DISABLE = 1 << 34;
        /// DCU Prefetcher Disable
        const DCU_PREFETCH_DISABLE = 1 << 37;
        /// IP Prefetcher Disable
        const IP_PREFETCH_DISABLE = 1 << 39;
    }
}

impl MiscEnable {
    /// Read current value
    pub fn read() -> Self {
        Self::from_bits_truncate(unsafe { rdmsr(addr::IA32_MISC_ENABLE) })
    }

    /// Write value
    ///
    /// # Safety
    /// Can affect system behavior significantly.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_MISC_ENABLE, self.bits());
        }
    }
}

// =============================================================================
// FEATURE CONTROL
// =============================================================================

bitflags::bitflags! {
    /// IA32_FEATURE_CONTROL flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FeatureControl: u64 {
        /// Lock (cannot be modified after set)
        const LOCK = 1 << 0;
        /// Enable VMX in SMX operation
        const VMX_IN_SMX = 1 << 1;
        /// Enable VMX outside SMX operation
        const VMX_OUTSIDE_SMX = 1 << 2;
        /// SENTER Local Functions Enable
        const SENTER_LOCAL = 0x7F << 8;
        /// SENTER Global Enable
        const SENTER_GLOBAL = 1 << 15;
        /// SGX Launch Control Enable
        const SGX_LAUNCH_CONTROL = 1 << 17;
        /// SGX Global Enable
        const SGX_GLOBAL = 1 << 18;
        /// LMCE On
        const LMCE = 1 << 20;
    }
}

impl FeatureControl {
    /// Read current value
    pub fn read() -> Self {
        Self::from_bits_truncate(unsafe { rdmsr(addr::IA32_FEATURE_CONTROL) })
    }

    /// Write value
    ///
    /// # Safety
    /// Cannot be modified once locked.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_FEATURE_CONTROL, self.bits());
        }
    }

    /// Check if locked
    pub fn is_locked(self) -> bool {
        self.contains(Self::LOCK)
    }

    /// Check if VMX is enabled
    pub fn vmx_enabled(self) -> bool {
        self.contains(Self::VMX_OUTSIDE_SMX)
    }
}

// =============================================================================
// DEBUG CONTROL
// =============================================================================

bitflags::bitflags! {
    /// IA32_DEBUGCTL flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DebugCtl: u64 {
        /// Last Branch Record
        const LBR = 1 << 0;
        /// Branch Trace Flag
        const BTF = 1 << 1;
        /// Performance Monitoring / Breakpoint Pin
        const PBTI = 1 << 2;
        /// Performance Monitoring / Breakpoint Pin 1
        const PBTI1 = 1 << 3;
        /// Trace messages enable
        const TR = 1 << 6;
        /// Branch trace store
        const BTS = 1 << 7;
        /// Branch trace interrupt
        const BTINT = 1 << 8;
        /// Branch trace off in CPL > 0
        const BTS_OFF_OS = 1 << 9;
        /// Branch trace off in CPL = 0
        const BTS_OFF_USER = 1 << 10;
        /// Freeze LBR on PMI
        const FREEZE_LBRS_ON_PMI = 1 << 11;
        /// Freeze PerfMon on PMI
        const FREEZE_PERFMON_ON_PMI = 1 << 12;
        /// Enable uncore PMI
        const UNCORE_PMI_EN = 1 << 13;
        /// Freeze while SMM
        const FREEZE_WHILE_SMM = 1 << 14;
        /// RTM Debug
        const RTM_DEBUG = 1 << 15;
    }
}

impl DebugCtl {
    /// Read current value
    pub fn read() -> Self {
        Self::from_bits_truncate(unsafe { rdmsr(addr::IA32_DEBUGCTL) })
    }

    /// Write value
    ///
    /// # Safety
    /// Can affect debugging and profiling behavior.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_DEBUGCTL, self.bits());
        }
    }
}

// =============================================================================
// SPECULATION CONTROL
// =============================================================================

bitflags::bitflags! {
    /// IA32_SPEC_CTRL flags (speculation mitigations)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecCtrl: u64 {
        /// Indirect Branch Restricted Speculation
        const IBRS = 1 << 0;
        /// Single Thread Indirect Branch Predictors
        const STIBP = 1 << 1;
        /// Speculative Store Bypass Disable
        const SSBD = 1 << 2;
        /// Intel PSFD
        const PSFD = 1 << 7;
    }
}

impl SpecCtrl {
    /// Read current value
    pub fn read() -> Self {
        Self::from_bits_truncate(unsafe { rdmsr(addr::IA32_SPEC_CTRL) })
    }

    /// Write value
    ///
    /// # Safety
    /// Can affect performance.
    pub unsafe fn write(self) {
        unsafe {
            wrmsr(addr::IA32_SPEC_CTRL, self.bits());
        }
    }
}

/// Indirect Branch Prediction Barrier
///
/// # Safety
/// This is a heavyweight operation.
pub unsafe fn ibpb() {
    unsafe {
        wrmsr(addr::IA32_PRED_CMD, 1);
    }
}

/// L1D cache flush
///
/// # Safety
/// This is a heavyweight operation.
pub unsafe fn l1d_flush() {
    unsafe {
        wrmsr(addr::IA32_FLUSH_CMD, 1);
    }
}

// =============================================================================
// PLATFORM INFO
// =============================================================================

/// Platform information from MSR_PLATFORM_INFO
#[derive(Debug, Clone, Copy)]
pub struct PlatformInfo {
    value: u64,
}

impl PlatformInfo {
    /// Read platform info
    pub fn read() -> Self {
        Self {
            value: unsafe { rdmsr(addr::MSR_PLATFORM_INFO) },
        }
    }

    /// Get maximum non-turbo ratio
    pub fn max_non_turbo_ratio(self) -> u8 {
        ((self.value >> 8) & 0xFF) as u8
    }

    /// Get programmable ratio limit for turbo mode
    pub fn programmable_ratio_limit(self) -> bool {
        (self.value & (1 << 28)) != 0
    }

    /// Get programmable TDP limit for turbo mode
    pub fn programmable_tdp_limit(self) -> bool {
        (self.value & (1 << 29)) != 0
    }

    /// Get low power mode support
    pub fn low_power_mode_support(self) -> bool {
        (self.value & (1 << 32)) != 0
    }

    /// Get maximum efficiency ratio
    pub fn max_efficiency_ratio(self) -> u8 {
        ((self.value >> 40) & 0xFF) as u8
    }
}

// =============================================================================
// TYPED MSR TRAIT
// =============================================================================

/// Trait for typed MSR access
pub trait Msr: Sized {
    /// MSR address
    const ADDRESS: u32;

    /// Read the MSR value
    fn read() -> Self;

    /// Write the MSR value
    ///
    /// # Safety
    /// Must provide valid value for this MSR.
    unsafe fn write(&self);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efer_read() {
        let efer = Efer::read();
        // In long mode, LMA should be set
        assert!(efer.long_mode_active());
    }

    #[test]
    fn test_apic_base_read() {
        let apic_base = ApicBase::read();
        // APIC should be enabled
        assert!(apic_base.is_enabled());
        // Base address should be valid (typically 0xFEE00000)
        assert!(apic_base.base_address() > 0);
    }

    #[test]
    fn test_tsc_read() {
        let tsc1 = Tsc::read();
        let tsc2 = Tsc::read();
        // TSC should be monotonically increasing
        assert!(tsc2 >= tsc1);
    }

    #[test]
    fn test_pat_entries() {
        let mut pat = Pat::default_config();
        pat.set_entry(0, PatMemoryType::WriteBack);
        assert_eq!(pat.entry(0), PatMemoryType::WriteBack);
    }
}
