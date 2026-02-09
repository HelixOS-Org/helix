//! IOMMU capabilities.

// ============================================================================
// IOMMU CAPABILITIES
// ============================================================================

/// IOMMU capabilities
#[derive(Debug, Clone, Default)]
pub struct IommuCapabilities {
    /// Supports 64-bit addressing
    pub addr_64bit: bool,
    /// Supports nested/2nd level translation
    pub nested: bool,
    /// Supports page request interface
    pub pri: bool,
    /// Supports ATS (Address Translation Services)
    pub ats: bool,
    /// Supports device TLB invalidation
    pub device_tlb: bool,
    /// Supports interrupt remapping
    pub interrupt_remap: bool,
    /// Supports posted interrupts
    pub posted_interrupts: bool,
    /// Maximum address width (bits)
    pub max_addr_width: u8,
    /// Supported page sizes (bitmask)
    pub page_sizes: u32,
    /// Number of domains supported
    pub max_domains: u32,
    /// Number of fault recording registers
    pub num_fault_regs: u8,
    /// Supports queued invalidation
    pub queued_invalidation: bool,
    /// Supports PASID (Process Address Space ID)
    pub pasid: bool,
    /// Supports scalable mode (Intel)
    pub scalable_mode: bool,
    /// Supports DIRTY tracking
    pub dirty_tracking: bool,
}

impl IommuCapabilities {
    /// Create new capabilities
    pub fn new() -> Self {
        Self::default()
    }

    /// Is fully featured
    #[inline(always)]
    pub fn is_full_featured(&self) -> bool {
        self.addr_64bit && self.nested && self.interrupt_remap && self.queued_invalidation
    }

    /// Has isolation support
    #[inline(always)]
    pub fn has_isolation(&self) -> bool {
        self.max_domains > 1
    }
}
