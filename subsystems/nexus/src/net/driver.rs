//! Network Driver Features
//!
//! Driver feature detection and management.

use alloc::vec::Vec;

// ============================================================================
// DRIVER FEATURES
// ============================================================================

/// Network driver feature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverFeature {
    /// Receive checksum offload
    RxChecksum,
    /// Transmit checksum offload
    TxChecksum,
    /// Scatter-gather
    SG,
    /// TCP segmentation offload
    Tso,
    /// UDP fragmentation offload
    Ufo,
    /// Generic segmentation offload
    Gso,
    /// Generic receive offload
    Gro,
    /// Large receive offload
    Lro,
    /// Receive hashing
    Rxhash,
    /// Receive VLAN offload
    RxVlan,
    /// Transmit VLAN offload
    TxVlan,
    /// NTUPLE filters
    Ntuple,
    /// Receive all
    RxAll,
    /// High DMA
    Highdma,
    /// TX lockless
    TxLockless,
    /// TX nocache copy
    TxNocacheCopy,
}

impl DriverFeature {
    /// Get feature name
    pub fn name(&self) -> &'static str {
        match self {
            Self::RxChecksum => "rx-checksum",
            Self::TxChecksum => "tx-checksum",
            Self::SG => "scatter-gather",
            Self::Tso => "tcp-segmentation-offload",
            Self::Ufo => "udp-fragmentation-offload",
            Self::Gso => "generic-segmentation-offload",
            Self::Gro => "generic-receive-offload",
            Self::Lro => "large-receive-offload",
            Self::Rxhash => "receive-hashing",
            Self::RxVlan => "rx-vlan-offload",
            Self::TxVlan => "tx-vlan-offload",
            Self::Ntuple => "ntuple-filters",
            Self::RxAll => "rx-all",
            Self::Highdma => "highdma",
            Self::TxLockless => "tx-lockless",
            Self::TxNocacheCopy => "tx-nocache-copy",
        }
    }
}

/// Driver feature set
#[derive(Debug, Clone, Default)]
pub struct DriverFeatures {
    /// Enabled features
    enabled: Vec<DriverFeature>,
    /// Available features
    available: Vec<DriverFeature>,
}

impl DriverFeatures {
    /// Create new feature set
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable feature
    #[inline]
    pub fn enable(&mut self, feature: DriverFeature) {
        if !self.enabled.contains(&feature) {
            self.enabled.push(feature);
        }
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self, feature: DriverFeature) -> bool {
        self.enabled.contains(&feature)
    }

    /// Add available
    #[inline]
    pub fn add_available(&mut self, feature: DriverFeature) {
        if !self.available.contains(&feature) {
            self.available.push(feature);
        }
    }

    /// Is available
    #[inline(always)]
    pub fn is_available(&self, feature: DriverFeature) -> bool {
        self.available.contains(&feature)
    }

    /// Has offload support
    #[inline]
    pub fn has_offloads(&self) -> bool {
        self.is_enabled(DriverFeature::RxChecksum)
            || self.is_enabled(DriverFeature::TxChecksum)
            || self.is_enabled(DriverFeature::Tso)
            || self.is_enabled(DriverFeature::Gro)
    }

    /// Enabled count
    #[inline(always)]
    pub fn enabled_count(&self) -> usize {
        self.enabled.len()
    }

    /// Available count
    #[inline(always)]
    pub fn available_count(&self) -> usize {
        self.available.len()
    }
}
