//! AArch64 Generic Interrupt Controller (GIC)
//!
//! GICv2 and GICv3 support for ARM64.

use crate::raw::types::*;

// =============================================================================
// GIC ARCHITECTURE VERSIONS
// =============================================================================

/// GIC architecture version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GicVersion {
    /// GICv2
    V2,
    /// GICv3
    V3,
    /// GICv4
    V4,
}

// =============================================================================
// INTERRUPT TYPES
// =============================================================================

/// Interrupt types in GIC
pub mod intid {
    /// SGI (Software Generated Interrupt) range: 0-15
    pub const SGI_START: u32 = 0;
    pub const SGI_END: u32 = 15;

    /// PPI (Private Peripheral Interrupt) range: 16-31
    pub const PPI_START: u32 = 16;
    pub const PPI_END: u32 = 31;

    /// SPI (Shared Peripheral Interrupt) range: 32-1019
    pub const SPI_START: u32 = 32;
    pub const SPI_END: u32 = 1019;

    /// Special interrupt IDs
    pub const INTID_SPURIOUS: u32 = 1023;

    /// GICv3 LPI range: 8192+
    pub const LPI_START: u32 = 8192;

    /// Check if interrupt is SGI
    pub fn is_sgi(intid: u32) -> bool {
        intid <= SGI_END
    }

    /// Check if interrupt is PPI
    pub fn is_ppi(intid: u32) -> bool {
        intid >= PPI_START && intid <= PPI_END
    }

    /// Check if interrupt is SPI
    pub fn is_spi(intid: u32) -> bool {
        intid >= SPI_START && intid <= SPI_END
    }

    /// Check if interrupt is LPI
    pub fn is_lpi(intid: u32) -> bool {
        intid >= LPI_START
    }
}

/// Common PPIs
pub mod ppi {
    /// Hypervisor maintenance interrupt
    pub const HYP_MAINT: u32 = 25;
    /// Virtual timer interrupt
    pub const VTIMER: u32 = 27;
    /// Legacy FIQ
    pub const LEGACY_FIQ: u32 = 28;
    /// Secure physical timer
    pub const SECURE_PHYS_TIMER: u32 = 29;
    /// Non-secure physical timer
    pub const NS_PHYS_TIMER: u32 = 30;
    /// Legacy IRQ
    pub const LEGACY_IRQ: u32 = 31;
}

// =============================================================================
// GICV2 DISTRIBUTOR (GICD)
// =============================================================================

/// GICv2 Distributor register offsets
pub mod gicd {
    /// Distributor Control Register
    pub const CTLR: usize = 0x0000;
    /// Interrupt Controller Type Register
    pub const TYPER: usize = 0x0004;
    /// Distributor Implementer Identification Register
    pub const IIDR: usize = 0x0008;
    /// Interrupt Group Registers
    pub const IGROUPR: usize = 0x0080;
    /// Interrupt Set-Enable Registers
    pub const ISENABLER: usize = 0x0100;
    /// Interrupt Clear-Enable Registers
    pub const ICENABLER: usize = 0x0180;
    /// Interrupt Set-Pending Registers
    pub const ISPENDR: usize = 0x0200;
    /// Interrupt Clear-Pending Registers
    pub const ICPENDR: usize = 0x0280;
    /// Interrupt Set-Active Registers
    pub const ISACTIVER: usize = 0x0300;
    /// Interrupt Clear-Active Registers
    pub const ICACTIVER: usize = 0x0380;
    /// Interrupt Priority Registers
    pub const IPRIORITYR: usize = 0x0400;
    /// Interrupt Processor Targets Registers
    pub const ITARGETSR: usize = 0x0800;
    /// Interrupt Configuration Registers
    pub const ICFGR: usize = 0x0C00;
    /// Software Generated Interrupt Register
    pub const SGIR: usize = 0x0F00;
    /// SGI Clear-Pending Registers
    pub const CPENDSGIR: usize = 0x0F10;
    /// SGI Set-Pending Registers
    pub const SPENDSGIR: usize = 0x0F20;
}

/// GICD_CTLR bits
pub mod gicd_ctlr {
    /// Enable Group 0 interrupts
    pub const ENABLE_GRP0: u32 = 1 << 0;
    /// Enable Group 1 interrupts
    pub const ENABLE_GRP1: u32 = 1 << 1;
}

/// GICD_TYPER fields
pub mod gicd_typer {
    /// Get IT Lines Number (number of supported interrupts / 32 - 1)
    pub fn it_lines_number(typer: u32) -> u32 {
        typer & 0x1F
    }

    /// Get CPU number (number of implemented CPU interfaces - 1)
    pub fn cpu_number(typer: u32) -> u32 {
        (typer >> 5) & 0x7
    }

    /// Security extensions supported
    pub fn security_extn(typer: u32) -> bool {
        (typer & (1 << 10)) != 0
    }

    /// Get LSPI (Lockable SPI count)
    pub fn lspi(typer: u32) -> u32 {
        (typer >> 11) & 0x1F
    }
}

/// GICD_SGIR fields
pub mod gicd_sgir {
    /// Target list filter: Use target list
    pub const TARGET_LIST: u32 = 0 << 24;
    /// Target list filter: All except self
    pub const TARGET_ALL_EXCEPT_SELF: u32 = 1 << 24;
    /// Target list filter: Only self
    pub const TARGET_SELF: u32 = 2 << 24;

    /// Build SGIR value
    pub fn build(intid: u32, target_filter: u32, cpu_target_list: u8) -> u32 {
        (intid & 0xF) | target_filter | ((cpu_target_list as u32) << 16)
    }
}

// =============================================================================
// GICV2 CPU INTERFACE (GICC)
// =============================================================================

/// GICv2 CPU Interface register offsets
pub mod gicc {
    /// CPU Interface Control Register
    pub const CTLR: usize = 0x0000;
    /// Interrupt Priority Mask Register
    pub const PMR: usize = 0x0004;
    /// Binary Point Register
    pub const BPR: usize = 0x0008;
    /// Interrupt Acknowledge Register
    pub const IAR: usize = 0x000C;
    /// End of Interrupt Register
    pub const EOIR: usize = 0x0010;
    /// Running Priority Register
    pub const RPR: usize = 0x0014;
    /// Highest Priority Pending Interrupt Register
    pub const HPPIR: usize = 0x0018;
    /// Aliased Binary Point Register
    pub const ABPR: usize = 0x001C;
    /// Aliased Interrupt Acknowledge Register
    pub const AIAR: usize = 0x0020;
    /// Aliased End of Interrupt Register
    pub const AEOIR: usize = 0x0024;
    /// Aliased Highest Priority Pending Interrupt Register
    pub const AHPPIR: usize = 0x0028;
    /// Active Priorities Registers
    pub const APR: usize = 0x00D0;
    /// Non-secure Active Priorities Registers
    pub const NSAPR: usize = 0x00E0;
    /// CPU Interface Identification Register
    pub const IIDR: usize = 0x00FC;
    /// Deactivate Interrupt Register
    pub const DIR: usize = 0x1000;
}

/// GICC_CTLR bits
pub mod gicc_ctlr {
    /// Enable signaling of Group 0 interrupts
    pub const ENABLE_GRP0: u32 = 1 << 0;
    /// Enable signaling of Group 1 interrupts
    pub const ENABLE_GRP1: u32 = 1 << 1;
    /// Acknowledge interrupt in IAR has EOI side effect
    pub const ACK_CTL: u32 = 1 << 2;
    /// FIQ Enable
    pub const FIQEN: u32 = 1 << 3;
    /// Control access to GICC_BPR
    pub const CBPR: u32 = 1 << 4;
    /// FIQ Bypass Disable Group 0
    pub const FIQ_BYP_DIS_GRP0: u32 = 1 << 5;
    /// IRQ Bypass Disable Group 0
    pub const IRQ_BYP_DIS_GRP0: u32 = 1 << 6;
    /// FIQ Bypass Disable Group 1
    pub const FIQ_BYP_DIS_GRP1: u32 = 1 << 7;
    /// IRQ Bypass Disable Group 1
    pub const IRQ_BYP_DIS_GRP1: u32 = 1 << 8;
    /// EOI mode
    pub const EOIMODE: u32 = 1 << 9;
    /// EOI mode for non-secure
    pub const EOIMODE_NS: u32 = 1 << 10;
}

// =============================================================================
// GICV2 INTERFACE
// =============================================================================

/// GICv2 Distributor interface
pub struct GicDistributor {
    base: VirtualAddress,
}

impl GicDistributor {
    /// Create distributor interface
    ///
    /// # Safety
    /// Base address must be valid and mapped
    pub unsafe fn new(base: VirtualAddress) -> Self {
        Self { base }
    }

    /// Read register
    fn read(&self, offset: usize) -> u32 {
        unsafe {
            let ptr = (self.base + offset as u64).0 as *const u32;
            core::ptr::read_volatile(ptr)
        }
    }

    /// Write register
    fn write(&self, offset: usize, value: u32) {
        unsafe {
            let ptr = (self.base + offset as u64).0 as *mut u32;
            core::ptr::write_volatile(ptr, value);
        }
    }

    /// Enable distributor
    pub fn enable(&self) {
        self.write(gicd::CTLR, gicd_ctlr::ENABLE_GRP0 | gicd_ctlr::ENABLE_GRP1);
    }

    /// Disable distributor
    pub fn disable(&self) {
        self.write(gicd::CTLR, 0);
    }

    /// Get number of supported interrupts
    pub fn num_interrupts(&self) -> u32 {
        let typer = self.read(gicd::TYPER);
        (gicd_typer::it_lines_number(typer) + 1) * 32
    }

    /// Get number of CPU interfaces
    pub fn num_cpus(&self) -> u32 {
        let typer = self.read(gicd::TYPER);
        gicd_typer::cpu_number(typer) + 1
    }

    /// Enable interrupt
    pub fn enable_interrupt(&self, intid: u32) {
        let reg = intid / 32;
        let bit = intid % 32;
        self.write(gicd::ISENABLER + (reg as usize * 4), 1 << bit);
    }

    /// Disable interrupt
    pub fn disable_interrupt(&self, intid: u32) {
        let reg = intid / 32;
        let bit = intid % 32;
        self.write(gicd::ICENABLER + (reg as usize * 4), 1 << bit);
    }

    /// Check if interrupt is enabled
    pub fn is_enabled(&self, intid: u32) -> bool {
        let reg = intid / 32;
        let bit = intid % 32;
        (self.read(gicd::ISENABLER + (reg as usize * 4)) & (1 << bit)) != 0
    }

    /// Set interrupt pending
    pub fn set_pending(&self, intid: u32) {
        let reg = intid / 32;
        let bit = intid % 32;
        self.write(gicd::ISPENDR + (reg as usize * 4), 1 << bit);
    }

    /// Clear interrupt pending
    pub fn clear_pending(&self, intid: u32) {
        let reg = intid / 32;
        let bit = intid % 32;
        self.write(gicd::ICPENDR + (reg as usize * 4), 1 << bit);
    }

    /// Check if interrupt is pending
    pub fn is_pending(&self, intid: u32) -> bool {
        let reg = intid / 32;
        let bit = intid % 32;
        (self.read(gicd::ISPENDR + (reg as usize * 4)) & (1 << bit)) != 0
    }

    /// Set interrupt priority (lower = higher priority)
    pub fn set_priority(&self, intid: u32, priority: u8) {
        let reg = intid / 4;
        let shift = (intid % 4) * 8;
        let offset = gicd::IPRIORITYR + (reg as usize * 4);
        let mut value = self.read(offset);
        value &= !(0xFF << shift);
        value |= (priority as u32) << shift;
        self.write(offset, value);
    }

    /// Get interrupt priority
    pub fn get_priority(&self, intid: u32) -> u8 {
        let reg = intid / 4;
        let shift = (intid % 4) * 8;
        let value = self.read(gicd::IPRIORITYR + (reg as usize * 4));
        ((value >> shift) & 0xFF) as u8
    }

    /// Set interrupt target CPUs (bitmask)
    pub fn set_target(&self, intid: u32, cpu_mask: u8) {
        if intid < 32 {
            return; // SGI/PPI targets are read-only
        }
        let reg = intid / 4;
        let shift = (intid % 4) * 8;
        let offset = gicd::ITARGETSR + (reg as usize * 4);
        let mut value = self.read(offset);
        value &= !(0xFF << shift);
        value |= (cpu_mask as u32) << shift;
        self.write(offset, value);
    }

    /// Get interrupt target CPUs
    pub fn get_target(&self, intid: u32) -> u8 {
        let reg = intid / 4;
        let shift = (intid % 4) * 8;
        let value = self.read(gicd::ITARGETSR + (reg as usize * 4));
        ((value >> shift) & 0xFF) as u8
    }

    /// Configure interrupt trigger (edge vs level)
    pub fn set_trigger(&self, intid: u32, edge_triggered: bool) {
        let reg = intid / 16;
        let shift = (intid % 16) * 2;
        let offset = gicd::ICFGR + (reg as usize * 4);
        let mut value = self.read(offset);
        if edge_triggered {
            value |= 2 << shift;
        } else {
            value &= !(2 << shift);
        }
        self.write(offset, value);
    }

    /// Set interrupt group (0 or 1)
    pub fn set_group(&self, intid: u32, group1: bool) {
        let reg = intid / 32;
        let bit = intid % 32;
        let offset = gicd::IGROUPR + (reg as usize * 4);
        let mut value = self.read(offset);
        if group1 {
            value |= 1 << bit;
        } else {
            value &= !(1 << bit);
        }
        self.write(offset, value);
    }

    /// Send SGI (Software Generated Interrupt)
    pub fn send_sgi(&self, intid: u32, target_filter: u32, cpu_target_list: u8) {
        let sgir = gicd_sgir::build(intid, target_filter, cpu_target_list);
        self.write(gicd::SGIR, sgir);
    }

    /// Send SGI to all CPUs except self
    pub fn send_sgi_all_except_self(&self, intid: u32) {
        self.send_sgi(intid, gicd_sgir::TARGET_ALL_EXCEPT_SELF, 0);
    }

    /// Send SGI to self
    pub fn send_sgi_self(&self, intid: u32) {
        self.send_sgi(intid, gicd_sgir::TARGET_SELF, 0);
    }

    /// Initialize distributor
    pub fn init(&self) {
        // Disable distributor
        self.disable();

        let num_ints = self.num_interrupts();

        // Disable all interrupts
        for i in 0..(num_ints / 32) {
            self.write(gicd::ICENABLER + (i as usize * 4), 0xFFFFFFFF);
        }

        // Clear all pending
        for i in 0..(num_ints / 32) {
            self.write(gicd::ICPENDR + (i as usize * 4), 0xFFFFFFFF);
        }

        // Set default priority (0x80)
        for i in 0..(num_ints / 4) {
            self.write(gicd::IPRIORITYR + (i as usize * 4), 0x80808080);
        }

        // Target all SPIs to CPU 0
        for i in (32 / 4)..(num_ints / 4) {
            self.write(gicd::ITARGETSR + (i as usize * 4), 0x01010101);
        }

        // Set all interrupts to Group 1
        for i in 0..(num_ints / 32) {
            self.write(gicd::IGROUPR + (i as usize * 4), 0xFFFFFFFF);
        }

        // Enable distributor
        self.enable();
    }
}

/// GICv2 CPU Interface
pub struct GicCpuInterface {
    base: VirtualAddress,
}

impl GicCpuInterface {
    /// Create CPU interface
    ///
    /// # Safety
    /// Base address must be valid and mapped
    pub unsafe fn new(base: VirtualAddress) -> Self {
        Self { base }
    }

    /// Read register
    fn read(&self, offset: usize) -> u32 {
        unsafe {
            let ptr = (self.base + offset as u64).0 as *const u32;
            core::ptr::read_volatile(ptr)
        }
    }

    /// Write register
    fn write(&self, offset: usize, value: u32) {
        unsafe {
            let ptr = (self.base + offset as u64).0 as *mut u32;
            core::ptr::write_volatile(ptr, value);
        }
    }

    /// Enable CPU interface
    pub fn enable(&self) {
        self.write(gicc::CTLR, gicc_ctlr::ENABLE_GRP0 | gicc_ctlr::ENABLE_GRP1);
    }

    /// Disable CPU interface
    pub fn disable(&self) {
        self.write(gicc::CTLR, 0);
    }

    /// Set priority mask (allow interrupts with priority < mask)
    pub fn set_priority_mask(&self, priority: u8) {
        self.write(gicc::PMR, priority as u32);
    }

    /// Get priority mask
    pub fn priority_mask(&self) -> u8 {
        self.read(gicc::PMR) as u8
    }

    /// Set binary point register
    pub fn set_binary_point(&self, bpr: u8) {
        self.write(gicc::BPR, bpr as u32);
    }

    /// Acknowledge interrupt (returns INTID)
    pub fn acknowledge(&self) -> u32 {
        self.read(gicc::IAR) & 0x3FF
    }

    /// Acknowledge interrupt with full IAR value
    pub fn acknowledge_full(&self) -> u32 {
        self.read(gicc::IAR)
    }

    /// Signal End Of Interrupt
    pub fn eoi(&self, intid: u32) {
        self.write(gicc::EOIR, intid);
    }

    /// Signal End Of Interrupt with full value
    pub fn eoi_full(&self, iar: u32) {
        self.write(gicc::EOIR, iar);
    }

    /// Get running priority
    pub fn running_priority(&self) -> u8 {
        self.read(gicc::RPR) as u8
    }

    /// Get highest priority pending interrupt
    pub fn highest_pending(&self) -> u32 {
        self.read(gicc::HPPIR) & 0x3FF
    }

    /// Initialize CPU interface
    pub fn init(&self) {
        // Set lowest priority mask (allow all)
        self.set_priority_mask(0xFF);

        // Set binary point to no preemption grouping
        self.set_binary_point(0);

        // Enable CPU interface
        self.enable();
    }
}

// =============================================================================
// GICV3 SYSTEM REGISTERS
// =============================================================================

/// GICv3 system register access
pub mod gicv3 {
    /// Read ICC_IAR0_EL1 (Group 0 Interrupt Acknowledge)
    pub fn read_iar0() -> u32 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_IAR0_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value as u32
    }

    /// Read ICC_IAR1_EL1 (Group 1 Interrupt Acknowledge)
    pub fn read_iar1() -> u32 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_IAR1_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value as u32
    }

    /// Write ICC_EOIR0_EL1 (Group 0 End of Interrupt)
    pub fn write_eoir0(intid: u32) {
        unsafe {
            core::arch::asm!(
                "msr ICC_EOIR0_EL1, {}",
                in(reg) intid as u64,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Write ICC_EOIR1_EL1 (Group 1 End of Interrupt)
    pub fn write_eoir1(intid: u32) {
        unsafe {
            core::arch::asm!(
                "msr ICC_EOIR1_EL1, {}",
                in(reg) intid as u64,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Read ICC_HPPIR0_EL1 (Highest Priority Pending Group 0)
    pub fn read_hppir0() -> u32 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_HPPIR0_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value as u32
    }

    /// Read ICC_HPPIR1_EL1 (Highest Priority Pending Group 1)
    pub fn read_hppir1() -> u32 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_HPPIR1_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value as u32
    }

    /// Write ICC_PMR_EL1 (Priority Mask)
    pub fn write_pmr(priority: u8) {
        unsafe {
            core::arch::asm!(
                "msr ICC_PMR_EL1, {}",
                in(reg) priority as u64,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Read ICC_PMR_EL1
    pub fn read_pmr() -> u8 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_PMR_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value as u8
    }

    /// Write ICC_BPR0_EL1 (Binary Point Register Group 0)
    pub fn write_bpr0(bpr: u8) {
        unsafe {
            core::arch::asm!(
                "msr ICC_BPR0_EL1, {}",
                in(reg) bpr as u64,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Write ICC_BPR1_EL1 (Binary Point Register Group 1)
    pub fn write_bpr1(bpr: u8) {
        unsafe {
            core::arch::asm!(
                "msr ICC_BPR1_EL1, {}",
                in(reg) bpr as u64,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Read ICC_CTLR_EL1 (CPU Interface Control)
    pub fn read_ctlr() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_CTLR_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    /// Write ICC_CTLR_EL1
    pub fn write_ctlr(value: u64) {
        unsafe {
            core::arch::asm!(
                "msr ICC_CTLR_EL1, {}",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Read ICC_IGRPEN0_EL1 (Group 0 Enable)
    pub fn read_igrpen0() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_IGRPEN0_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    /// Write ICC_IGRPEN0_EL1
    pub fn write_igrpen0(value: u64) {
        unsafe {
            core::arch::asm!(
                "msr ICC_IGRPEN0_EL1, {}",
                "isb",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Read ICC_IGRPEN1_EL1 (Group 1 Enable)
    pub fn read_igrpen1() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_IGRPEN1_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    /// Write ICC_IGRPEN1_EL1
    pub fn write_igrpen1(value: u64) {
        unsafe {
            core::arch::asm!(
                "msr ICC_IGRPEN1_EL1, {}",
                "isb",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Write ICC_SGI0R_EL1 (SGI Group 0)
    pub fn write_sgi0(value: u64) {
        unsafe {
            core::arch::asm!(
                "msr ICC_SGI0R_EL1, {}",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Write ICC_SGI1R_EL1 (SGI Group 1)
    pub fn write_sgi1(value: u64) {
        unsafe {
            core::arch::asm!(
                "msr ICC_SGI1R_EL1, {}",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Read ICC_SRE_EL1 (System Register Enable)
    pub fn read_sre() -> u64 {
        let value: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, ICC_SRE_EL1",
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }

    /// Write ICC_SRE_EL1
    pub fn write_sre(value: u64) {
        unsafe {
            core::arch::asm!(
                "msr ICC_SRE_EL1, {}",
                "isb",
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    /// Enable system register interface
    pub fn enable_sre() {
        let sre = read_sre();
        write_sre(sre | 0x7); // SRE, DFB, DIB
    }

    /// Initialize GICv3 CPU interface
    pub fn init_cpu_interface() {
        // Enable system register access
        enable_sre();

        // Set priority mask to lowest
        write_pmr(0xFF);

        // Set binary point
        write_bpr0(0);
        write_bpr1(0);

        // Enable Group 0 and Group 1
        write_igrpen0(1);
        write_igrpen1(1);
    }

    /// Acknowledge Group 1 interrupt
    pub fn acknowledge() -> u32 {
        read_iar1() & 0xFFFFFF
    }

    /// End of Interrupt for Group 1
    pub fn eoi(intid: u32) {
        write_eoir1(intid);
    }

    /// Send SGI to CPU list
    pub fn send_sgi(intid: u32, target_list: u16, aff3: u8, aff2: u8, aff1: u8) {
        let sgi = (intid as u64 & 0xF) << 24
            | (aff3 as u64) << 48
            | (aff2 as u64) << 32
            | (aff1 as u64) << 16
            | (target_list as u64);
        write_sgi1(sgi);
    }

    /// Send SGI to all except self
    pub fn send_sgi_all_except_self(intid: u32) {
        let sgi = (intid as u64 & 0xF) << 24 | (1 << 40); // IRM bit
        write_sgi1(sgi);
    }
}

// =============================================================================
// GIC CONFIGURATION
// =============================================================================

/// GIC configuration
#[derive(Debug, Clone)]
pub struct GicConfig {
    /// Distributor base address
    pub distributor_base: PhysicalAddress,
    /// CPU interface base address (GICv2)
    pub cpu_interface_base: Option<PhysicalAddress>,
    /// Redistributor base address (GICv3)
    pub redistributor_base: Option<PhysicalAddress>,
    /// GIC version
    pub version: GicVersion,
}

impl GicConfig {
    /// Create GICv2 configuration
    pub fn gicv2(distributor: PhysicalAddress, cpu_interface: PhysicalAddress) -> Self {
        Self {
            distributor_base: distributor,
            cpu_interface_base: Some(cpu_interface),
            redistributor_base: None,
            version: GicVersion::V2,
        }
    }

    /// Create GICv3 configuration
    pub fn gicv3(distributor: PhysicalAddress, redistributor: PhysicalAddress) -> Self {
        Self {
            distributor_base: distributor,
            cpu_interface_base: None,
            redistributor_base: Some(redistributor),
            version: GicVersion::V3,
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intid_classification() {
        assert!(intid::is_sgi(0));
        assert!(intid::is_sgi(15));
        assert!(!intid::is_sgi(16));

        assert!(!intid::is_ppi(15));
        assert!(intid::is_ppi(16));
        assert!(intid::is_ppi(31));

        assert!(!intid::is_spi(31));
        assert!(intid::is_spi(32));
        assert!(intid::is_spi(100));
    }

    #[test]
    fn test_sgir_build() {
        let sgir = gicd_sgir::build(5, gicd_sgir::TARGET_ALL_EXCEPT_SELF, 0);
        assert_eq!(sgir & 0xF, 5);
        assert_eq!(sgir & (3 << 24), gicd_sgir::TARGET_ALL_EXCEPT_SELF);
    }
}
