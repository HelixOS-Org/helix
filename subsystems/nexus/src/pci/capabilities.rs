//! PCI and PCIe capabilities.

// ============================================================================
// PCI CAPABILITIES
// ============================================================================

/// PCI capability ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(pub u8);

impl CapabilityId {
    // Standard PCI capabilities
    pub const POWER_MGMT: Self = Self(0x01);
    pub const AGP: Self = Self(0x02);
    pub const VPD: Self = Self(0x03);
    pub const SLOT_ID: Self = Self(0x04);
    pub const MSI: Self = Self(0x05);
    pub const HOT_SWAP: Self = Self(0x06);
    pub const PCIX: Self = Self(0x07);
    pub const HYPERTRANSPORT: Self = Self(0x08);
    pub const VENDOR_SPECIFIC: Self = Self(0x09);
    pub const DEBUG_PORT: Self = Self(0x0a);
    pub const CPCI_RC: Self = Self(0x0b);
    pub const HOT_PLUG: Self = Self(0x0c);
    pub const BRIDGE_SUBSYS: Self = Self(0x0d);
    pub const AGP_8X: Self = Self(0x0e);
    pub const SECURE_DEVICE: Self = Self(0x0f);
    pub const PCIE: Self = Self(0x10);
    pub const MSIX: Self = Self(0x11);
    pub const SATA: Self = Self(0x12);
    pub const AF: Self = Self(0x13);
    pub const EA: Self = Self(0x14);
    pub const FPB: Self = Self(0x15);

    /// Get capability name
    pub fn name(&self) -> &'static str {
        match self.0 {
            0x01 => "power_mgmt",
            0x02 => "agp",
            0x03 => "vpd",
            0x04 => "slot_id",
            0x05 => "msi",
            0x06 => "hot_swap",
            0x07 => "pci-x",
            0x08 => "hypertransport",
            0x09 => "vendor",
            0x0a => "debug_port",
            0x0b => "cpci_rc",
            0x0c => "hot_plug",
            0x0d => "bridge_subsys",
            0x0e => "agp_8x",
            0x0f => "secure_device",
            0x10 => "pcie",
            0x11 => "msi-x",
            0x12 => "sata",
            0x13 => "af",
            0x14 => "ea",
            0x15 => "fpb",
            _ => "unknown",
        }
    }
}

/// Extended capability ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtCapabilityId(pub u16);

impl ExtCapabilityId {
    // PCIe extended capabilities
    pub const AER: Self = Self(0x0001);
    pub const VC: Self = Self(0x0002);
    pub const SERIAL_NUM: Self = Self(0x0003);
    pub const POWER_BUDGETING: Self = Self(0x0004);
    pub const RC_LINK_DECL: Self = Self(0x0005);
    pub const RC_INTERNAL_LINK: Self = Self(0x0006);
    pub const RC_EVENT_COLL: Self = Self(0x0007);
    pub const MFVC: Self = Self(0x0008);
    pub const VC2: Self = Self(0x0009);
    pub const RCRB_HEADER: Self = Self(0x000a);
    pub const VENDOR_SPECIFIC: Self = Self(0x000b);
    pub const CAC: Self = Self(0x000c);
    pub const ACS: Self = Self(0x000d);
    pub const ARI: Self = Self(0x000e);
    pub const ATS: Self = Self(0x000f);
    pub const SR_IOV: Self = Self(0x0010);
    pub const MR_IOV: Self = Self(0x0011);
    pub const MCAST: Self = Self(0x0012);
    pub const PAGE_REQUEST: Self = Self(0x0013);
    pub const AMD_XXX: Self = Self(0x0014);
    pub const REBAR: Self = Self(0x0015);
    pub const DPA: Self = Self(0x0016);
    pub const TPH: Self = Self(0x0017);
    pub const LTR: Self = Self(0x0018);
    pub const SEC_PCIE: Self = Self(0x0019);
    pub const PMUX: Self = Self(0x001a);
    pub const PASID: Self = Self(0x001b);
    pub const LN_REQUESTER: Self = Self(0x001c);
    pub const DPC: Self = Self(0x001d);
    pub const L1SS: Self = Self(0x001e);
    pub const PTM: Self = Self(0x001f);
    pub const DVSEC: Self = Self(0x0023);
    pub const DOE: Self = Self(0x002e);

    /// Get capability name
    pub fn name(&self) -> &'static str {
        match self.0 {
            0x0001 => "aer",
            0x0002 => "vc",
            0x0003 => "serial_num",
            0x0004 => "power_budget",
            0x000d => "acs",
            0x000e => "ari",
            0x000f => "ats",
            0x0010 => "sr-iov",
            0x0011 => "mr-iov",
            0x0013 => "page_request",
            0x0015 => "rebar",
            0x0017 => "tph",
            0x0018 => "ltr",
            0x001b => "pasid",
            0x001d => "dpc",
            0x001e => "l1ss",
            0x001f => "ptm",
            0x0023 => "dvsec",
            0x002e => "doe",
            _ => "unknown",
        }
    }
}

/// PCI capability
#[derive(Debug, Clone)]
pub struct PciCapability {
    /// Capability ID
    pub id: CapabilityId,
    /// Offset in config space
    pub offset: u8,
    /// Size
    pub size: u8,
    /// Data (first few bytes)
    pub data: [u8; 8],
}

impl PciCapability {
    /// Create new capability
    pub fn new(id: CapabilityId, offset: u8) -> Self {
        Self {
            id,
            offset,
            size: 0,
            data: [0; 8],
        }
    }
}

/// PCIe extended capability
#[derive(Debug, Clone)]
pub struct ExtCapability {
    /// Capability ID
    pub id: ExtCapabilityId,
    /// Version
    pub version: u8,
    /// Offset in config space
    pub offset: u16,
    /// Next offset
    pub next: u16,
}

impl ExtCapability {
    /// Create new extended capability
    pub fn new(id: ExtCapabilityId, version: u8, offset: u16, next: u16) -> Self {
        Self {
            id,
            version,
            offset,
            next,
        }
    }
}
