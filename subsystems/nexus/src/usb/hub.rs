//! USB Hub
//!
//! USB hub and port management.

use alloc::vec::Vec;

use super::{UsbDeviceId, UsbSpeed};

/// Hub port state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubPortState {
    /// Not connected
    Empty,
    /// Device connected
    Connected,
    /// Suspended
    Suspended,
    /// Over-current
    OverCurrent,
    /// Port disabled
    Disabled,
}

impl HubPortState {
    /// Get state name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Connected => "connected",
            Self::Suspended => "suspended",
            Self::OverCurrent => "over-current",
            Self::Disabled => "disabled",
        }
    }
}

/// Hub port
#[derive(Debug, Clone)]
pub struct HubPort {
    /// Port number
    pub number: u8,
    /// State
    pub state: HubPortState,
    /// Connected device
    pub device: Option<UsbDeviceId>,
    /// Power enabled
    pub power_enabled: bool,
    /// Speed
    pub speed: Option<UsbSpeed>,
}

impl HubPort {
    /// Create new port
    pub fn new(number: u8) -> Self {
        Self {
            number,
            state: HubPortState::Empty,
            device: None,
            power_enabled: true,
            speed: None,
        }
    }

    /// Is occupied
    #[inline]
    pub fn is_occupied(&self) -> bool {
        matches!(
            self.state,
            HubPortState::Connected | HubPortState::Suspended
        )
    }
}

/// USB hub
#[derive(Debug)]
pub struct UsbHub {
    /// Device ID
    pub device_id: UsbDeviceId,
    /// Port count
    pub port_count: u8,
    /// Ports
    pub ports: Vec<HubPort>,
    /// Power per port (mA)
    pub power_per_port: u16,
    /// Is self powered
    pub self_powered: bool,
    /// Hub depth
    pub depth: u8,
    /// MTT (Multi-TT) capable
    pub mtt: bool,
}

impl UsbHub {
    /// Create new hub
    pub fn new(device_id: UsbDeviceId, port_count: u8) -> Self {
        let ports = (1..=port_count).map(HubPort::new).collect();
        Self {
            device_id,
            port_count,
            ports,
            power_per_port: 100,
            self_powered: false,
            depth: 0,
            mtt: false,
        }
    }

    /// Get port
    #[inline(always)]
    pub fn get_port(&self, number: u8) -> Option<&HubPort> {
        self.ports.iter().find(|p| p.number == number)
    }

    /// Get port mutably
    #[inline(always)]
    pub fn get_port_mut(&mut self, number: u8) -> Option<&mut HubPort> {
        self.ports.iter_mut().find(|p| p.number == number)
    }

    /// Connected port count
    #[inline(always)]
    pub fn connected_ports(&self) -> usize {
        self.ports.iter().filter(|p| p.is_occupied()).count()
    }

    /// Available ports
    #[inline]
    pub fn available_ports(&self) -> usize {
        self.ports
            .iter()
            .filter(|p| matches!(p.state, HubPortState::Empty))
            .count()
    }
}
