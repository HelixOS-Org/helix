// SPDX-License-Identifier: GPL-2.0
//! Holistic pci_enum — PCI bus enumeration and device discovery.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// PCI device class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciClass {
    Unclassified,
    MassStorage,
    Network,
    Display,
    Multimedia,
    Memory,
    Bridge,
    Communication,
    Peripheral,
    Input,
    Docking,
    Processor,
    SerialBus,
    Wireless,
    Other(u8),
}

impl PciClass {
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => Self::Unclassified, 0x01 => Self::MassStorage,
            0x02 => Self::Network, 0x03 => Self::Display,
            0x04 => Self::Multimedia, 0x05 => Self::Memory,
            0x06 => Self::Bridge, 0x07 => Self::Communication,
            0x08 => Self::Peripheral, 0x09 => Self::Input,
            0x0A => Self::Docking, 0x0B => Self::Processor,
            0x0C => Self::SerialBus, 0x0D => Self::Wireless,
            c => Self::Other(c),
        }
    }
}

/// PCI header type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciHeaderType {
    Standard,
    PciBridge,
    CardBusBridge,
}

/// BDF (Bus/Device/Function)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bdf {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl Bdf {
    pub fn new(bus: u8, dev: u8, func: u8) -> Self { Self { bus, device: dev, function: func } }
    #[inline(always)]
    pub fn as_u32(&self) -> u32 { ((self.bus as u32) << 8) | ((self.device as u32) << 3) | self.function as u32 }
}

/// BAR (Base Address Register)
#[derive(Debug, Clone)]
pub struct PciBar {
    pub index: u8,
    pub base: u64,
    pub size: u64,
    pub is_memory: bool,
    pub is_64bit: bool,
    pub prefetchable: bool,
}

/// PCI device
#[derive(Debug)]
pub struct PciDevice {
    pub bdf: Bdf,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: PciClass,
    pub subclass: u8,
    pub prog_if: u8,
    pub header_type: PciHeaderType,
    pub revision: u8,
    pub irq_line: u8,
    pub irq_pin: u8,
    pub bars: Vec<PciBar>,
    pub msi_capable: bool,
    pub msix_capable: bool,
    pub capabilities: Vec<PciCapability>,
}

impl PciDevice {
    pub fn new(bdf: Bdf, vendor: u16, device: u16, class: u8, subclass: u8) -> Self {
        Self {
            bdf, vendor_id: vendor, device_id: device,
            class: PciClass::from_code(class), subclass, prog_if: 0,
            header_type: PciHeaderType::Standard, revision: 0,
            irq_line: 0, irq_pin: 0, bars: Vec::new(),
            msi_capable: false, msix_capable: false, capabilities: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn is_bridge(&self) -> bool { self.class == PciClass::Bridge }
    #[inline(always)]
    pub fn add_bar(&mut self, bar: PciBar) { self.bars.push(bar); }
}

/// PCI capability
#[derive(Debug, Clone)]
pub struct PciCapability {
    pub id: u8,
    pub offset: u16,
    pub version: u8,
}

/// PCI bus
#[derive(Debug)]
pub struct PciBus {
    pub bus_number: u8,
    pub parent_bridge: Option<Bdf>,
    pub devices: Vec<Bdf>,
}

impl PciBus {
    pub fn new(num: u8) -> Self { Self { bus_number: num, parent_bridge: None, devices: Vec::new() } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PciEnumStats {
    pub total_buses: u32,
    pub total_devices: u32,
    pub total_functions: u32,
    pub bridges: u32,
    pub msi_capable: u32,
    pub msix_capable: u32,
    pub total_bars: u32,
    pub total_bar_bytes: u64,
}

/// Main PCI enumeration manager
pub struct HolisticPciEnum {
    devices: BTreeMap<u32, PciDevice>,
    buses: BTreeMap<u8, PciBus>,
}

impl HolisticPciEnum {
    pub fn new() -> Self { Self { devices: BTreeMap::new(), buses: BTreeMap::new() } }

    #[inline]
    pub fn discover_device(&mut self, bdf: Bdf, vendor: u16, device: u16, class: u8, subclass: u8) {
        let dev = PciDevice::new(bdf, vendor, device, class, subclass);
        self.devices.insert(bdf.as_u32(), dev);
        self.buses.entry(bdf.bus).or_insert_with(|| PciBus::new(bdf.bus)).devices.push(bdf);
    }

    #[inline(always)]
    pub fn get_device(&self, bdf: &Bdf) -> Option<&PciDevice> {
        self.devices.get(&bdf.as_u32())
    }

    #[inline(always)]
    pub fn devices_by_class(&self, class: PciClass) -> Vec<&PciDevice> {
        self.devices.values().filter(|d| d.class == class).collect()
    }

    pub fn stats(&self) -> PciEnumStats {
        let bridges = self.devices.values().filter(|d| d.is_bridge()).count() as u32;
        let msi = self.devices.values().filter(|d| d.msi_capable).count() as u32;
        let msix = self.devices.values().filter(|d| d.msix_capable).count() as u32;
        let bars: u32 = self.devices.values().map(|d| d.bars.len() as u32).sum();
        let bar_bytes: u64 = self.devices.values().flat_map(|d| &d.bars).map(|b| b.size).sum();
        PciEnumStats {
            total_buses: self.buses.len() as u32, total_devices: self.devices.len() as u32,
            total_functions: self.devices.len() as u32, bridges, msi_capable: msi,
            msix_capable: msix, total_bars: bars, total_bar_bytes: bar_bytes,
        }
    }
}

// ============================================================================
// Merged from pci_enum_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciClassV2 {
    Unclassified,
    MassStorage,
    Network,
    Display,
    Multimedia,
    Memory,
    Bridge,
    Communication,
    SystemPeripheral,
    InputDevice,
    DockingStation,
    Processor,
    SerialBus,
    Wireless,
    Other,
}

/// PCI BAR type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBarType {
    Memory32,
    Memory64,
    Io,
}

/// PCI BAR
#[derive(Debug)]
pub struct PciBar {
    pub index: u8,
    pub bar_type: PciBarType,
    pub base_addr: u64,
    pub size: u64,
    pub prefetchable: bool,
}

/// PCI device v2
#[derive(Debug)]
pub struct PciDeviceV2 {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: PciClassV2,
    pub subclass: u8,
    pub revision: u8,
    pub bars: Vec<PciBar>,
    pub irq: Option<u32>,
    pub msi_capable: bool,
    pub msix_capable: bool,
}

impl PciDeviceV2 {
    pub fn new(bus: u8, dev: u8, func: u8, vid: u16, did: u16, class: PciClassV2) -> Self {
        Self { bus, device: dev, function: func, vendor_id: vid, device_id: did, class, subclass: 0, revision: 0, bars: Vec::new(), irq: None, msi_capable: false, msix_capable: false }
    }
    #[inline(always)]
    pub fn bdf(&self) -> u32 { ((self.bus as u32) << 8) | ((self.device as u32) << 3) | (self.function as u32) }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PciEnumV2Stats {
    pub total_devices: u32,
    pub total_buses: u32,
    pub total_bars: u32,
    pub msi_capable: u32,
    pub msix_capable: u32,
}

/// Main PCI enumerator v2
pub struct HolisticPciEnumV2 {
    devices: BTreeMap<u32, PciDeviceV2>,
    buses_scanned: u32,
}

impl HolisticPciEnumV2 {
    pub fn new() -> Self { Self { devices: BTreeMap::new(), buses_scanned: 0 } }

    #[inline(always)]
    pub fn add_device(&mut self, dev: PciDeviceV2) {
        let bdf = dev.bdf();
        self.devices.insert(bdf, dev);
    }

    #[inline(always)]
    pub fn scan_bus(&mut self, bus: u8) { self.buses_scanned = self.buses_scanned.max(bus as u32 + 1); }

    #[inline(always)]
    pub fn find_by_class(&self, class: PciClassV2) -> Vec<u32> {
        self.devices.iter().filter(|(_, d)| d.class == class).map(|(&bdf, _)| bdf).collect()
    }

    #[inline]
    pub fn stats(&self) -> PciEnumV2Stats {
        let bars: u32 = self.devices.values().map(|d| d.bars.len() as u32).sum();
        let msi = self.devices.values().filter(|d| d.msi_capable).count() as u32;
        let msix = self.devices.values().filter(|d| d.msix_capable).count() as u32;
        PciEnumV2Stats { total_devices: self.devices.len() as u32, total_buses: self.buses_scanned, total_bars: bars, msi_capable: msi, msix_capable: msix }
    }
}
