//! Network Manager
//!
//! Central interface and statistics management.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use super::{IfIndex, InterfaceStats, NetworkInterface};

// ============================================================================
// NETWORK MANAGER
// ============================================================================

/// Network manager
pub struct NetworkManager {
    /// Interfaces
    pub(crate) interfaces: BTreeMap<IfIndex, NetworkInterface>,
    /// Interface count
    interface_count: AtomicU32,
    /// Total RX bytes
    total_rx_bytes: AtomicU64,
    /// Total TX bytes
    total_tx_bytes: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl NetworkManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
            interface_count: AtomicU32::new(0),
            total_rx_bytes: AtomicU64::new(0),
            total_tx_bytes: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register interface
    pub fn register_interface(&mut self, interface: NetworkInterface) {
        self.interface_count.fetch_add(1, Ordering::Relaxed);
        self.interfaces.insert(interface.index, interface);
    }

    /// Get interface
    pub fn get_interface(&self, index: IfIndex) -> Option<&NetworkInterface> {
        self.interfaces.get(&index)
    }

    /// Get interface mutably
    pub fn get_interface_mut(&mut self, index: IfIndex) -> Option<&mut NetworkInterface> {
        self.interfaces.get_mut(&index)
    }

    /// Get interface by name
    pub fn get_by_name(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.values().find(|i| i.name == name)
    }

    /// Get physical interfaces
    pub fn physical_interfaces(&self) -> Vec<&NetworkInterface> {
        self.interfaces
            .values()
            .filter(|i| i.if_type.is_physical())
            .collect()
    }

    /// Get virtual interfaces
    pub fn virtual_interfaces(&self) -> Vec<&NetworkInterface> {
        self.interfaces
            .values()
            .filter(|i| i.if_type.is_virtual())
            .collect()
    }

    /// Get running interfaces
    pub fn running_interfaces(&self) -> Vec<&NetworkInterface> {
        self.interfaces
            .values()
            .filter(|i| i.is_running())
            .collect()
    }

    /// Update stats
    pub fn update_stats(&mut self, index: IfIndex, stats: InterfaceStats) {
        if let Some(iface) = self.interfaces.get_mut(&index) {
            let rx_delta = stats.rx_bytes.saturating_sub(iface.stats.rx_bytes);
            let tx_delta = stats.tx_bytes.saturating_sub(iface.stats.tx_bytes);

            self.total_rx_bytes.fetch_add(rx_delta, Ordering::Relaxed);
            self.total_tx_bytes.fetch_add(tx_delta, Ordering::Relaxed);

            iface.stats = stats;
        }
    }

    /// Get interface count
    pub fn interface_count(&self) -> u32 {
        self.interface_count.load(Ordering::Relaxed)
    }

    /// Get total RX bytes
    pub fn total_rx_bytes(&self) -> u64 {
        self.total_rx_bytes.load(Ordering::Relaxed)
    }

    /// Get total TX bytes
    pub fn total_tx_bytes(&self) -> u64 {
        self.total_tx_bytes.load(Ordering::Relaxed)
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}
