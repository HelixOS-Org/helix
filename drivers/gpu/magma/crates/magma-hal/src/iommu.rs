//! # IOMMU Integration
//!
//! GPU IOMMU (SMMU/VT-d) integration for DMA mapping.

use magma_core::{ByteSize, Error, GpuAddr, PhysAddr, Result};

// =============================================================================
// DMA DIRECTION
// =============================================================================

/// DMA transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    /// Host to device
    ToDevice,
    /// Device to host
    FromDevice,
    /// Bidirectional
    Bidirectional,
    /// No DMA transfer (for CPU-only access)
    None,
}

// =============================================================================
// DMA MAPPING
// =============================================================================

/// A DMA mapping for a buffer
#[derive(Debug)]
pub struct DmaMapping {
    /// CPU virtual address
    pub cpu_addr: usize,
    /// DMA address (as seen by GPU/IOMMU)
    pub dma_addr: PhysAddr,
    /// Size of mapping
    pub size: ByteSize,
    /// Transfer direction
    pub direction: DmaDirection,
}

impl DmaMapping {
    /// Check if mapping is coherent (no manual cache operations needed)
    pub fn is_coherent(&self) -> bool {
        // On x86 with IOMMU, DMA is typically coherent
        // On ARM, depends on IOMMU and system configuration
        cfg!(target_arch = "x86_64")
    }
}

// =============================================================================
// IOMMU DOMAIN
// =============================================================================

/// IOMMU domain for GPU isolation
#[derive(Debug)]
pub struct IommuDomain {
    /// Domain ID
    pub domain_id: u32,
    /// Address width (typically 48 bits)
    pub address_bits: u8,
    /// Whether domain supports device isolation
    pub isolated: bool,
}

// =============================================================================
// IOMMU TRAIT
// =============================================================================

/// IOMMU interface trait
pub trait Iommu {
    /// Create a new IOMMU domain for a GPU
    fn create_domain(&mut self) -> Result<IommuDomain>;

    /// Destroy an IOMMU domain
    fn destroy_domain(&mut self, domain: IommuDomain) -> Result<()>;

    /// Map physical memory for DMA
    fn map(
        &mut self,
        domain: &IommuDomain,
        phys_addr: PhysAddr,
        size: ByteSize,
        direction: DmaDirection,
    ) -> Result<DmaMapping>;

    /// Unmap DMA memory
    fn unmap(&mut self, domain: &IommuDomain, mapping: DmaMapping) -> Result<()>;

    /// Allocate coherent DMA memory
    fn alloc_coherent(&mut self, domain: &IommuDomain, size: ByteSize) -> Result<DmaMapping>;

    /// Free coherent DMA memory
    fn free_coherent(&mut self, domain: &IommuDomain, mapping: DmaMapping) -> Result<()>;

    /// Sync DMA buffer for CPU access (before CPU reads device-written data)
    fn sync_for_cpu(&self, mapping: &DmaMapping) -> Result<()>;

    /// Sync DMA buffer for device access (before device reads CPU-written data)
    fn sync_for_device(&self, mapping: &DmaMapping) -> Result<()>;
}

// =============================================================================
// GPU INTERNAL MMU
// =============================================================================

/// GPU internal MMU page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GpuPageSize {
    /// 4KB pages
    Small = 0x1000,
    /// 64KB pages
    Big   = 0x10000,
    /// 128KB pages (Ada+)
    Huge  = 0x20000,
    /// 2MB pages
    Large = 0x200000,
}

/// GPU page table entry flags
bitflags::bitflags! {
    /// GPU PTE flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GpuPteFlags: u64 {
        /// Page is valid
        const VALID = 1 << 0;
        /// Page is writable
        const WRITE = 1 << 1;
        /// Page is readable
        const READ = 1 << 2;
        /// Page is atomic-capable
        const ATOMIC = 1 << 3;
        /// Page is encrypted (for protected content)
        const ENCRYPTED = 1 << 4;
        /// Privilege level (kernel vs user)
        const PRIVILEGE = 1 << 5;
        /// Cache coherent (system memory)
        const COHERENT = 1 << 6;
        /// Volatile (disable caching)
        const VOLATILE = 1 << 7;
    }
}

/// GPU MMU page table level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPtLevel {
    /// Page directory 3 (top level)
    Pd3,
    /// Page directory 2
    Pd2,
    /// Page directory 1
    Pd1,
    /// Page directory 0
    Pd0,
    /// Page table (bottom level)
    Pt,
}

/// GPU MMU interface trait
pub trait GpuMmu {
    /// Create page tables for an address space
    fn create_address_space(&mut self) -> Result<u64>;

    /// Destroy an address space
    fn destroy_address_space(&mut self, asid: u64) -> Result<()>;

    /// Map GPU virtual address to physical
    fn map(
        &mut self,
        asid: u64,
        gpu_addr: GpuAddr,
        phys_addr: PhysAddr,
        size: ByteSize,
        flags: GpuPteFlags,
    ) -> Result<()>;

    /// Unmap GPU virtual address
    fn unmap(&mut self, asid: u64, gpu_addr: GpuAddr, size: ByteSize) -> Result<()>;

    /// Flush TLB for address range
    fn flush_tlb(&mut self, asid: u64, gpu_addr: GpuAddr, size: ByteSize) -> Result<()>;

    /// Flush entire TLB
    fn flush_tlb_all(&mut self, asid: u64) -> Result<()>;
}
