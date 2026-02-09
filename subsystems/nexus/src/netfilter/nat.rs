//! NAT Handling
//!
//! Network Address Translation.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ConntrackId, NetworkAddr, Protocol};

/// NAT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// Source NAT
    Snat,
    /// Destination NAT
    Dnat,
    /// Masquerade
    Masquerade,
    /// Redirect
    Redirect,
}

/// NAT mapping
#[derive(Debug, Clone)]
pub struct NatMapping {
    /// NAT type
    pub nat_type: NatType,
    /// Original address/port
    pub original_addr: NetworkAddr,
    /// Original port
    pub original_port: u16,
    /// Translated address/port
    pub translated_addr: NetworkAddr,
    /// Translated port
    pub translated_port: u16,
    /// Protocol
    pub protocol: Protocol,
    /// Conntrack entry
    pub conntrack_id: Option<ConntrackId>,
}

/// NAT table
pub struct NatTable {
    /// SNAT mappings
    snat_mappings: Vec<NatMapping>,
    /// DNAT mappings
    dnat_mappings: Vec<NatMapping>,
    /// Masquerade interface
    masq_interfaces: Vec<String>,
    /// Port range for masquerade
    masq_port_min: u16,
    masq_port_max: u16,
    /// Next available port
    next_port: AtomicU64,
}

impl NatTable {
    /// Create new NAT table
    pub fn new() -> Self {
        Self {
            snat_mappings: Vec::new(),
            dnat_mappings: Vec::new(),
            masq_interfaces: Vec::new(),
            masq_port_min: 32768,
            masq_port_max: 60999,
            next_port: AtomicU64::new(32768),
        }
    }

    /// Add SNAT mapping
    #[inline(always)]
    pub fn add_snat(&mut self, mapping: NatMapping) {
        self.snat_mappings.push(mapping);
    }

    /// Add DNAT mapping
    #[inline(always)]
    pub fn add_dnat(&mut self, mapping: NatMapping) {
        self.dnat_mappings.push(mapping);
    }

    /// Allocate masquerade port
    #[inline]
    pub fn allocate_masq_port(&self) -> u16 {
        let port = self.next_port.fetch_add(1, Ordering::Relaxed) as u16;
        if port > self.masq_port_max {
            self.next_port
                .store(self.masq_port_min as u64, Ordering::Relaxed);
            return self.masq_port_min;
        }
        port
    }

    /// Get SNAT mappings
    #[inline(always)]
    pub fn snat_mappings(&self) -> &[NatMapping] {
        &self.snat_mappings
    }

    /// Get DNAT mappings
    #[inline(always)]
    pub fn dnat_mappings(&self) -> &[NatMapping] {
        &self.dnat_mappings
    }
}

impl Default for NatTable {
    fn default() -> Self {
        Self::new()
    }
}
