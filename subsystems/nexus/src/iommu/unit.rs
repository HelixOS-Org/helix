//! IOMMU unit representation.

extern crate alloc;

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::capabilities::IommuCapabilities;
use super::domain::{DomainType, IommuDomain};
use super::types::{DeviceId, DomainId, IommuId, IommuState, IommuType};

// ============================================================================
// IOMMU UNIT
// ============================================================================

/// IOMMU unit
#[derive(Debug)]
pub struct IommuUnit {
    /// Unit ID
    pub id: IommuId,
    /// Type
    pub iommu_type: IommuType,
    /// State
    pub state: IommuState,
    /// Capabilities
    pub capabilities: IommuCapabilities,
    /// Base address (MMIO)
    pub base_addr: u64,
    /// Domains
    pub domains: BTreeMap<DomainId, IommuDomain>,
    /// Next domain ID
    next_domain_id: AtomicU64,
    /// Device to domain mapping
    device_domains: BTreeMap<DeviceId, DomainId>,
    /// Translation enabled
    translation_enabled: AtomicBool,
    /// Interrupt remapping enabled
    interrupt_remap_enabled: AtomicBool,
}

impl IommuUnit {
    /// Create new IOMMU unit
    pub fn new(id: IommuId, iommu_type: IommuType) -> Self {
        Self {
            id,
            iommu_type,
            state: IommuState::Initializing,
            capabilities: IommuCapabilities::new(),
            base_addr: 0,
            domains: BTreeMap::new(),
            next_domain_id: AtomicU64::new(1),
            device_domains: BTreeMap::new(),
            translation_enabled: AtomicBool::new(false),
            interrupt_remap_enabled: AtomicBool::new(false),
        }
    }

    /// Create domain
    pub fn create_domain(&mut self, domain_type: DomainType, timestamp: u64) -> DomainId {
        let id = DomainId::new(self.next_domain_id.fetch_add(1, Ordering::Relaxed));
        let domain = IommuDomain::new(id, domain_type, self.id, timestamp);
        self.domains.insert(id, domain);
        id
    }

    /// Get domain
    pub fn get_domain(&self, id: DomainId) -> Option<&IommuDomain> {
        self.domains.get(&id)
    }

    /// Get domain mutably
    pub fn get_domain_mut(&mut self, id: DomainId) -> Option<&mut IommuDomain> {
        self.domains.get_mut(&id)
    }

    /// Attach device to domain
    pub fn attach_device(&mut self, device: DeviceId, domain_id: DomainId) -> bool {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.attach_device(device);
            self.device_domains.insert(device, domain_id);
            true
        } else {
            false
        }
    }

    /// Get device domain
    pub fn get_device_domain(&self, device: DeviceId) -> Option<DomainId> {
        self.device_domains.get(&device).copied()
    }

    /// Enable translation
    pub fn enable_translation(&mut self) {
        self.translation_enabled.store(true, Ordering::Relaxed);
        self.state = IommuState::Enabled;
    }

    /// Is translation enabled
    pub fn is_translation_enabled(&self) -> bool {
        self.translation_enabled.load(Ordering::Relaxed)
    }

    /// Enable interrupt remapping
    pub fn enable_interrupt_remap(&self) {
        self.interrupt_remap_enabled.store(true, Ordering::Relaxed);
    }

    /// Is interrupt remapping enabled
    pub fn is_interrupt_remap_enabled(&self) -> bool {
        self.interrupt_remap_enabled.load(Ordering::Relaxed)
    }

    /// Get domain count
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }
}
