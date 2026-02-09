//! Core IOMMU types.

extern crate alloc;

// ============================================================================
// IOMMU ID
// ============================================================================

/// IOMMU unit ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IommuId(pub u64);

impl IommuId {
    /// Create new IOMMU ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

// ============================================================================
// DOMAIN ID
// ============================================================================

/// Domain ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainId(pub u64);

impl DomainId {
    /// Create new domain ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

// ============================================================================
// DEVICE ID
// ============================================================================

/// Device ID (BDF - Bus:Device.Function)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId {
    /// Segment
    pub segment: u16,
    /// Bus
    pub bus: u8,
    /// Device
    pub device: u8,
    /// Function
    pub function: u8,
}

impl DeviceId {
    /// Create new device ID
    #[inline]
    pub const fn new(segment: u16, bus: u8, device: u8, function: u8) -> Self {
        Self {
            segment,
            bus,
            device,
            function,
        }
    }

    /// Create from BDF
    #[inline(always)]
    pub const fn from_bdf(bus: u8, device: u8, function: u8) -> Self {
        Self::new(0, bus, device, function)
    }

    /// Get BDF as u16
    #[inline(always)]
    pub fn bdf(&self) -> u16 {
        ((self.bus as u16) << 8) | ((self.device as u16) << 3) | (self.function as u16)
    }
}

impl core::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{}",
            self.segment, self.bus, self.device, self.function
        )
    }
}

// ============================================================================
// IOMMU TYPE
// ============================================================================

/// IOMMU type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuType {
    /// Intel VT-d
    IntelVtd,
    /// AMD-Vi
    AmdVi,
    /// ARM SMMU
    ArmSmmu,
    /// ARM SMMUv3
    ArmSmmuV3,
    /// Apple DART
    AppleDart,
    /// Virtio IOMMU
    VirtioIommu,
    /// Unknown
    Unknown,
}

impl IommuType {
    /// Get type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::IntelVtd => "intel-vtd",
            Self::AmdVi => "amd-vi",
            Self::ArmSmmu => "arm-smmu",
            Self::ArmSmmuV3 => "arm-smmu-v3",
            Self::AppleDart => "apple-dart",
            Self::VirtioIommu => "virtio-iommu",
            Self::Unknown => "unknown",
        }
    }

    /// Supports nested translation
    #[inline(always)]
    pub fn supports_nested(&self) -> bool {
        matches!(self, Self::IntelVtd | Self::AmdVi | Self::ArmSmmuV3)
    }
}

// ============================================================================
// IOMMU STATE
// ============================================================================

/// IOMMU state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuState {
    /// Disabled
    Disabled,
    /// Enabled (translation active)
    Enabled,
    /// Passthrough (identity mapping)
    Passthrough,
    /// Initializing
    Initializing,
    /// Error
    Error,
}

impl IommuState {
    /// Get state name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Enabled => "enabled",
            Self::Passthrough => "passthrough",
            Self::Initializing => "initializing",
            Self::Error => "error",
        }
    }
}
