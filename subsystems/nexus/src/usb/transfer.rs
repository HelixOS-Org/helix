//! USB Transfer and Manager
//!
//! USB transfer tracking and device management.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use super::{BusId, EndpointDirection, TransferType, UsbBus, UsbClass, UsbDevice, UsbDeviceId};

/// USB transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    /// Pending
    Pending,
    /// Completed
    Completed,
    /// Error
    Error,
    /// Stall
    Stall,
    /// Cancelled
    Cancelled,
    /// Timeout
    Timeout,
}

impl TransferStatus {
    /// Get status name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Completed => "completed",
            Self::Error => "error",
            Self::Stall => "stall",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
        }
    }

    /// Is success
    #[inline(always)]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// USB transfer record
#[derive(Debug, Clone)]
pub struct UsbTransfer {
    /// Device
    pub device: UsbDeviceId,
    /// Endpoint
    pub endpoint: u8,
    /// Direction
    pub direction: EndpointDirection,
    /// Transfer type
    pub transfer_type: TransferType,
    /// Requested bytes
    pub requested_bytes: u64,
    /// Actual bytes
    pub actual_bytes: u64,
    /// Status
    pub status: TransferStatus,
    /// Start time
    pub start_time: u64,
    /// End time
    pub end_time: u64,
}

impl UsbTransfer {
    /// Create new transfer
    pub fn new(
        device: UsbDeviceId,
        endpoint: u8,
        direction: EndpointDirection,
        transfer_type: TransferType,
        requested_bytes: u64,
    ) -> Self {
        Self {
            device,
            endpoint,
            direction,
            transfer_type,
            requested_bytes,
            actual_bytes: 0,
            status: TransferStatus::Pending,
            start_time: 0,
            end_time: 0,
        }
    }

    /// Duration
    #[inline(always)]
    pub fn duration(&self) -> u64 {
        self.end_time.saturating_sub(self.start_time)
    }

    /// Throughput (bytes/sec)
    #[inline]
    pub fn throughput(&self) -> u64 {
        let duration = self.duration();
        if duration > 0 {
            (self.actual_bytes * 1_000_000_000) / duration
        } else {
            0
        }
    }
}

/// USB manager
pub struct UsbManager {
    /// Buses
    buses: BTreeMap<BusId, UsbBus>,
    /// All devices (flat index)
    all_devices: BTreeMap<UsbDeviceId, UsbDeviceId>,
    /// Transfer history
    transfers: VecDeque<UsbTransfer>,
    /// Max transfers
    max_transfers: usize,
    /// Device count
    device_count: AtomicU32,
    /// Total transfers
    total_transfers: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl UsbManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            buses: BTreeMap::new(),
            all_devices: BTreeMap::new(),
            transfers: VecDeque::new(),
            max_transfers: 10000,
            device_count: AtomicU32::new(0),
            total_transfers: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register bus
    #[inline(always)]
    pub fn register_bus(&mut self, bus: UsbBus) {
        self.buses.insert(bus.id, bus);
    }

    /// Get bus
    #[inline(always)]
    pub fn get_bus(&self, id: BusId) -> Option<&UsbBus> {
        self.buses.get(&id)
    }

    /// Get bus mutably
    #[inline(always)]
    pub fn get_bus_mut(&mut self, id: BusId) -> Option<&mut UsbBus> {
        self.buses.get_mut(&id)
    }

    /// Register device
    #[inline]
    pub fn register_device(&mut self, bus_id: BusId, device: UsbDevice) {
        let id = device.id;
        if let Some(bus) = self.buses.get_mut(&bus_id) {
            bus.add_device(device);
            self.all_devices.insert(id, id);
            self.device_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Find device
    #[inline(always)]
    pub fn find_device(&self, id: UsbDeviceId) -> Option<&UsbDevice> {
        self.buses.get(&id.bus)?.get_device(id.address)
    }

    /// Record transfer
    #[inline]
    pub fn record_transfer(&mut self, transfer: UsbTransfer) {
        self.total_transfers.fetch_add(1, Ordering::Relaxed);

        if self.transfers.len() >= self.max_transfers {
            self.transfers.pop_front();
        }
        self.transfers.push_back(transfer);
    }

    /// Get device count
    #[inline(always)]
    pub fn device_count(&self) -> u32 {
        self.device_count.load(Ordering::Relaxed)
    }

    /// Get bus count
    #[inline(always)]
    pub fn bus_count(&self) -> usize {
        self.buses.len()
    }

    /// Get buses
    #[inline(always)]
    pub fn buses(&self) -> &BTreeMap<BusId, UsbBus> {
        &self.buses
    }

    /// Get devices by class
    #[inline]
    pub fn devices_by_class(&self, class: UsbClass) -> Vec<UsbDeviceId> {
        self.buses
            .values()
            .flat_map(|b| b.devices.values())
            .filter(|d| d.class == class)
            .map(|d| d.id)
            .collect()
    }

    /// Get storage devices
    #[inline(always)]
    pub fn storage_devices(&self) -> Vec<UsbDeviceId> {
        self.devices_by_class(UsbClass::MassStorage)
    }

    /// Get HID devices
    #[inline(always)]
    pub fn hid_devices(&self) -> Vec<UsbDeviceId> {
        self.devices_by_class(UsbClass::Hid)
    }
}

impl Default for UsbManager {
    fn default() -> Self {
        Self::new()
    }
}
