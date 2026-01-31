//! # MPIDR (Multiprocessor Affinity Register)
//!
//! This module provides utilities for parsing and working with the MPIDR_EL1
//! register, which identifies CPUs in a hierarchical affinity scheme.
//!
//! ## MPIDR Format
//!
//! ```text
//! 63                             40 39   32 31 30 29   25 24 23    16 15     8 7      0
//! ┌─────────────────────────────────┬───────┬──┬──┬───────┬──┬────────┬────────┬────────┐
//! │             RES0                │ Aff3  │1 │U │ RES0  │MT│  Aff2  │  Aff1  │  Aff0  │
//! └─────────────────────────────────┴───────┴──┴──┴───────┴──┴────────┴────────┴────────┘
//!
//! Aff0: Affinity level 0 - typically core within cluster (0-255)
//! Aff1: Affinity level 1 - typically cluster within socket (0-255)
//! Aff2: Affinity level 2 - typically socket within node (0-255)
//! Aff3: Affinity level 3 - typically node (0-255)
//! U:    Uniprocessor (1 = uniprocessor, 0 = multiprocessor)
//! MT:   Multithreading (1 = cores share SMT threads)
//! ```
//!
//! ## Affinity Hierarchy
//!
//! The affinity values form a hierarchical CPU identifier:
//!
//! - **Aff0**: Lowest level, typically individual core or hardware thread
//! - **Aff1**: Cluster of cores sharing L2 cache
//! - **Aff2**: Group of clusters (die or socket)
//! - **Aff3**: NUMA node or system partition
//!
//! ## Common Configurations
//!
//! | Platform        | Aff3   | Aff2   | Aff1    | Aff0      |
//! |-----------------|--------|--------|---------|-----------|
//! | QEMU virt       | 0      | 0      | 0       | CPU index |
//! | Raspberry Pi 4  | 0      | 0      | 0       | Core 0-3  |
//! | AWS Graviton2   | 0      | Socket | Cluster | Core      |
//! | Ampere Altra    | 0      | Socket | Cluster | Core      |

use core::arch::asm;

// ============================================================================
// MPIDR Bit Definitions
// ============================================================================

/// Aff0 mask (bits 0-7)
pub const MPIDR_AFF0_MASK: u64 = 0xFF;

/// Aff0 shift
pub const MPIDR_AFF0_SHIFT: u64 = 0;

/// Aff1 mask (bits 8-15)
pub const MPIDR_AFF1_MASK: u64 = 0xFF << 8;

/// Aff1 shift
pub const MPIDR_AFF1_SHIFT: u64 = 8;

/// Aff2 mask (bits 16-23)
pub const MPIDR_AFF2_MASK: u64 = 0xFF << 16;

/// Aff2 shift
pub const MPIDR_AFF2_SHIFT: u64 = 16;

/// Aff3 mask (bits 32-39)
pub const MPIDR_AFF3_MASK: u64 = 0xFF << 32;

/// Aff3 shift
pub const MPIDR_AFF3_SHIFT: u64 = 32;

/// Uniprocessor bit (bit 30)
pub const MPIDR_UP: u64 = 1 << 30;

/// Multithreading bit (bit 24)
pub const MPIDR_MT: u64 = 1 << 24;

/// Full affinity mask (Aff0-Aff3)
pub const MPIDR_AFFINITY_MASK: u64 =
    MPIDR_AFF0_MASK | MPIDR_AFF1_MASK | MPIDR_AFF2_MASK | MPIDR_AFF3_MASK;

/// Level 0-2 affinity mask (for PSCI)
pub const MPIDR_AFFINITY_LEVEL_MASK: u64 = MPIDR_AFF0_MASK | MPIDR_AFF1_MASK | MPIDR_AFF2_MASK;

// ============================================================================
// MPIDR Structure
// ============================================================================

/// MPIDR register wrapper
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Mpidr(u64);

impl Mpidr {
    /// Create from raw value
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Create from affinity levels
    #[inline]
    pub const fn from_affinity(aff3: u8, aff2: u8, aff1: u8, aff0: u8) -> Self {
        Self(
            (aff0 as u64)
                | ((aff1 as u64) << MPIDR_AFF1_SHIFT)
                | ((aff2 as u64) << MPIDR_AFF2_SHIFT)
                | ((aff3 as u64) << MPIDR_AFF3_SHIFT),
        )
    }

    /// Read the current CPU's MPIDR
    #[inline]
    pub fn current() -> Self {
        let value: u64;
        unsafe {
            asm!("mrs {}, mpidr_el1", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        Self(value)
    }

    /// Get the raw MPIDR value
    #[inline]
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Get the full affinity value (Aff0-Aff3)
    #[inline]
    pub const fn affinity(self) -> u64 {
        self.0 & MPIDR_AFFINITY_MASK
    }

    /// Get affinity level 0 (core)
    #[inline]
    pub const fn aff0(self) -> u8 {
        (self.0 & MPIDR_AFF0_MASK) as u8
    }

    /// Get affinity level 1 (cluster)
    #[inline]
    pub const fn aff1(self) -> u8 {
        ((self.0 & MPIDR_AFF1_MASK) >> MPIDR_AFF1_SHIFT) as u8
    }

    /// Get affinity level 2 (socket/die)
    #[inline]
    pub const fn aff2(self) -> u8 {
        ((self.0 & MPIDR_AFF2_MASK) >> MPIDR_AFF2_SHIFT) as u8
    }

    /// Get affinity level 3 (node)
    #[inline]
    pub const fn aff3(self) -> u8 {
        ((self.0 & MPIDR_AFF3_MASK) >> MPIDR_AFF3_SHIFT) as u8
    }

    /// Check if this is a uniprocessor system
    #[inline]
    pub const fn is_uniprocessor(self) -> bool {
        (self.0 & MPIDR_UP) != 0
    }

    /// Check if multithreading is indicated
    #[inline]
    pub const fn is_multithreaded(self) -> bool {
        (self.0 & MPIDR_MT) != 0
    }

    /// Get cluster ID (Aff1)
    #[inline]
    pub const fn cluster_id(self) -> u8 {
        self.aff1()
    }

    /// Get core ID within cluster (Aff0)
    #[inline]
    pub const fn core_id(self) -> u8 {
        self.aff0()
    }

    /// Get a linear CPU ID suitable for indexing
    ///
    /// This combines affinity levels into a single value. The formula
    /// may need adjustment for specific platforms.
    #[inline]
    pub const fn linear_id(self) -> u32 {
        // Simple formula: assumes max 256 cores per cluster, 256 clusters per socket, etc.
        // For most systems, only Aff0 and Aff1 matter
        let id = self.aff0() as u32
            + ((self.aff1() as u32) << 8)
            + ((self.aff2() as u32) << 16)
            + ((self.aff3() as u32) << 24);
        id
    }

    /// Get a compact CPU ID assuming a flat topology
    ///
    /// This is useful for systems where CPUs are numbered 0-N in Aff0.
    #[inline]
    pub const fn flat_id(self) -> u32 {
        self.aff0() as u32
    }

    /// Get the affinity value formatted for PSCI calls
    ///
    /// PSCI uses the format: Aff3:Aff2:Aff1:Aff0 in bits 32:24:16:8:0
    #[inline]
    pub const fn psci_affinity(self) -> u64 {
        self.affinity()
    }

    /// Get the affinity value formatted for GICv3 IROUTER
    #[inline]
    pub const fn gicv3_affinity(self) -> u64 {
        // GICv3 uses the same format as MPIDR affinity
        self.affinity()
    }

    /// Check if two MPIDRs are on the same cluster
    #[inline]
    pub const fn same_cluster(self, other: Self) -> bool {
        (self.aff1() == other.aff1())
            && (self.aff2() == other.aff2())
            && (self.aff3() == other.aff3())
    }

    /// Check if two MPIDRs are on the same socket/die
    #[inline]
    pub const fn same_socket(self, other: Self) -> bool {
        (self.aff2() == other.aff2()) && (self.aff3() == other.aff3())
    }

    /// Check if two MPIDRs are on the same NUMA node
    #[inline]
    pub const fn same_node(self, other: Self) -> bool {
        self.aff3() == other.aff3()
    }

    /// Create an MPIDR for testing
    #[cfg(test)]
    pub const fn test(aff0: u8) -> Self {
        Self::from_affinity(0, 0, 0, aff0)
    }
}

impl core::fmt::Debug for Mpidr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Mpidr")
            .field("aff3", &self.aff3())
            .field("aff2", &self.aff2())
            .field("aff1", &self.aff1())
            .field("aff0", &self.aff0())
            .field("mt", &self.is_multithreaded())
            .field("up", &self.is_uniprocessor())
            .finish()
    }
}

impl core::fmt::Display for Mpidr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.aff3(),
            self.aff2(),
            self.aff1(),
            self.aff0()
        )
    }
}

// ============================================================================
// Affinity Level
// ============================================================================

/// Affinity level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AffinityLevel {
    /// Level 0: Core/Thread
    Aff0 = 0,
    /// Level 1: Cluster
    Aff1 = 1,
    /// Level 2: Socket/Die
    Aff2 = 2,
    /// Level 3: Node
    Aff3 = 3,
}

impl AffinityLevel {
    /// Get the bit shift for this level
    pub const fn shift(self) -> u64 {
        match self {
            AffinityLevel::Aff0 => MPIDR_AFF0_SHIFT,
            AffinityLevel::Aff1 => MPIDR_AFF1_SHIFT,
            AffinityLevel::Aff2 => MPIDR_AFF2_SHIFT,
            AffinityLevel::Aff3 => MPIDR_AFF3_SHIFT,
        }
    }

    /// Get the mask for this level
    pub const fn mask(self) -> u64 {
        match self {
            AffinityLevel::Aff0 => MPIDR_AFF0_MASK,
            AffinityLevel::Aff1 => MPIDR_AFF1_MASK,
            AffinityLevel::Aff2 => MPIDR_AFF2_MASK,
            AffinityLevel::Aff3 => MPIDR_AFF3_MASK,
        }
    }

    /// Extract this level from an MPIDR
    pub const fn extract(self, mpidr: Mpidr) -> u8 {
        ((mpidr.value() & self.mask()) >> self.shift()) as u8
    }
}

// ============================================================================
// CPU ID Helpers
// ============================================================================

/// Get the current CPU's affinity as a tuple (aff3, aff2, aff1, aff0)
#[inline]
pub fn current_affinity() -> (u8, u8, u8, u8) {
    let mpidr = Mpidr::current();
    (mpidr.aff3(), mpidr.aff2(), mpidr.aff1(), mpidr.aff0())
}

/// Get the current CPU's core ID (Aff0)
#[inline]
pub fn current_core_id() -> u8 {
    Mpidr::current().aff0()
}

/// Get the current CPU's cluster ID (Aff1)
#[inline]
pub fn current_cluster_id() -> u8 {
    Mpidr::current().aff1()
}

/// Check if we're running on the BSP (CPU 0)
#[inline]
pub fn is_bsp() -> bool {
    Mpidr::current().flat_id() == 0
}

// ============================================================================
// Platform-Specific Helpers
// ============================================================================

/// QEMU virt: CPUs are numbered 0-N in Aff0
pub mod qemu {
    use super::Mpidr;

    /// Get CPU ID for QEMU virt (just Aff0)
    #[inline]
    pub fn cpu_id() -> u32 {
        Mpidr::current().aff0() as u32
    }

    /// Create MPIDR for a given CPU ID
    #[inline]
    pub const fn mpidr_for_cpu(cpu_id: u32) -> Mpidr {
        Mpidr::from_affinity(0, 0, 0, cpu_id as u8)
    }
}

/// Raspberry Pi: 4 cores, all in Aff0
pub mod rpi {
    use super::Mpidr;

    /// Get CPU ID for Raspberry Pi (Aff0)
    #[inline]
    pub fn cpu_id() -> u32 {
        Mpidr::current().aff0() as u32
    }

    /// Create MPIDR for a given core (0-3)
    #[inline]
    pub const fn mpidr_for_core(core: u32) -> Mpidr {
        Mpidr::from_affinity(0, 0, 0, core as u8)
    }
}

/// AWS Graviton: Multiple clusters
pub mod graviton {
    use super::Mpidr;

    /// Get CPU ID for Graviton (cluster * cores_per_cluster + core)
    #[inline]
    pub fn cpu_id(cores_per_cluster: u32) -> u32 {
        let mpidr = Mpidr::current();
        (mpidr.aff1() as u32 * cores_per_cluster) + mpidr.aff0() as u32
    }
}

// ============================================================================
// Topology Discovery
// ============================================================================

/// Discovered CPU topology level
#[derive(Debug, Clone, Copy)]
pub struct TopologyLevel {
    /// Number of entries at this level
    pub count: u32,
    /// Affinity level this represents
    pub level: AffinityLevel,
}

/// Discover the CPU topology from a list of MPIDRs
pub fn discover_topology(mpidrs: &[Mpidr]) -> [TopologyLevel; 4] {
    let mut levels = [
        TopologyLevel {
            count: 0,
            level: AffinityLevel::Aff0,
        },
        TopologyLevel {
            count: 0,
            level: AffinityLevel::Aff1,
        },
        TopologyLevel {
            count: 0,
            level: AffinityLevel::Aff2,
        },
        TopologyLevel {
            count: 0,
            level: AffinityLevel::Aff3,
        },
    ];

    // Find unique values at each level
    let mut seen_aff0 = [false; 256];
    let mut seen_aff1 = [false; 256];
    let mut seen_aff2 = [false; 256];
    let mut seen_aff3 = [false; 256];

    for mpidr in mpidrs {
        if !seen_aff0[mpidr.aff0() as usize] {
            seen_aff0[mpidr.aff0() as usize] = true;
            levels[0].count += 1;
        }
        if !seen_aff1[mpidr.aff1() as usize] {
            seen_aff1[mpidr.aff1() as usize] = true;
            levels[1].count += 1;
        }
        if !seen_aff2[mpidr.aff2() as usize] {
            seen_aff2[mpidr.aff2() as usize] = true;
            levels[2].count += 1;
        }
        if !seen_aff3[mpidr.aff3() as usize] {
            seen_aff3[mpidr.aff3() as usize] = true;
            levels[3].count += 1;
        }
    }

    levels
}
