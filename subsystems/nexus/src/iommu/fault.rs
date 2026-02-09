//! IOMMU fault tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::types::{DeviceId, DomainId};

// ============================================================================
// FAULT TYPE
// ============================================================================

/// IOMMU fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultType {
    /// Translation fault
    Translation,
    /// Permission fault
    Permission,
    /// Device fault
    Device,
    /// Page request
    PageRequest,
    /// Invalid descriptor
    InvalidDescriptor,
    /// External abort
    ExternalAbort,
    /// Unknown
    Unknown,
}

impl FaultType {
    /// Get fault name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Translation => "translation",
            Self::Permission => "permission",
            Self::Device => "device",
            Self::PageRequest => "page_request",
            Self::InvalidDescriptor => "invalid_descriptor",
            Self::ExternalAbort => "external_abort",
            Self::Unknown => "unknown",
        }
    }

    /// Is security related
    #[inline(always)]
    pub fn is_security(&self) -> bool {
        matches!(self, Self::Permission | Self::Device)
    }
}

// ============================================================================
// IOMMU FAULT
// ============================================================================

/// IOMMU fault record
#[derive(Debug, Clone)]
pub struct IommuFault {
    /// Timestamp
    pub timestamp: u64,
    /// Device
    pub device: DeviceId,
    /// Domain
    pub domain_id: Option<DomainId>,
    /// Fault type
    pub fault_type: FaultType,
    /// Faulting address
    pub address: u64,
    /// Was read operation
    pub is_read: bool,
    /// Was execute operation
    pub is_exec: bool,
    /// PASID (if applicable)
    pub pasid: Option<u32>,
}

impl IommuFault {
    /// Create new fault
    pub fn new(timestamp: u64, device: DeviceId, fault_type: FaultType, address: u64) -> Self {
        Self {
            timestamp,
            device,
            domain_id: None,
            fault_type,
            address,
            is_read: false,
            is_exec: false,
            pasid: None,
        }
    }
}

// ============================================================================
// FAULT TRACKER
// ============================================================================

/// IOMMU fault tracker
pub struct FaultTracker {
    /// Faults
    faults: VecDeque<IommuFault>,
    /// Max faults
    max_faults: usize,
    /// Fault count by device
    by_device: BTreeMap<DeviceId, u64>,
    /// Fault count by type
    by_type: BTreeMap<FaultType, u64>,
    /// Total faults
    total: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl FaultTracker {
    /// Create new tracker
    pub fn new(max_faults: usize) -> Self {
        Self {
            faults: VecDeque::new(),
            max_faults,
            by_device: BTreeMap::new(),
            by_type: BTreeMap::new(),
            total: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Record fault
    pub fn record(&mut self, fault: IommuFault) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        self.total.fetch_add(1, Ordering::Relaxed);

        match self.by_device.get_mut(&fault.device) {
            Some(count) => *count += 1,
            None => {
                self.by_device.insert(fault.device, 1);
            },
        }
        match self.by_type.get_mut(&fault.fault_type) {
            Some(count) => *count += 1,
            None => {
                self.by_type.insert(fault.fault_type, 1);
            },
        }

        if self.faults.len() >= self.max_faults {
            self.faults.pop_front();
        }
        self.faults.push_back(fault);
    }

    /// Get recent faults
    #[inline(always)]
    pub fn recent(&self, count: usize) -> &[IommuFault] {
        let start = self.faults.len().saturating_sub(count);
        &self.faults[start..]
    }

    /// Get faults for device
    #[inline(always)]
    pub fn for_device(&self, device: DeviceId) -> Vec<&IommuFault> {
        self.faults.iter().filter(|f| f.device == device).collect()
    }

    /// Get total
    #[inline(always)]
    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    /// Get top faulting devices
    #[inline]
    pub fn top_devices(&self, n: usize) -> Vec<(DeviceId, u64)> {
        let mut sorted: Vec<_> = self.by_device.iter().map(|(k, v)| (*k, *v)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Enable/disable
    #[inline(always)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for FaultTracker {
    fn default() -> Self {
        Self::new(10000)
    }
}
