//! # Driver Subsystem
//!
//! Device driver management, discovery, and initialization.
//! Late phase subsystem for hardware driver loading.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// =============================================================================
// DEVICE TYPES
// =============================================================================

/// Device ID
pub type DeviceId = u64;

/// Driver ID
pub type DriverId = u64;

/// Device class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Unknown,
    // Storage
    BlockDevice,
    CharDevice,
    // Display
    Display,
    Framebuffer,
    // Input
    Keyboard,
    Mouse,
    Touchpad,
    Gamepad,
    // Network
    NetworkInterface,
    WirelessInterface,
    // Audio
    AudioOutput,
    AudioInput,
    // Serial
    SerialPort,
    ParallelPort,
    // Bus
    PciBus,
    UsbBus,
    I2cBus,
    SpiBus,
    // Misc
    Timer,
    Rtc,
    Watchdog,
    Gpio,
    Thermal,
    Power,
}

impl Default for DeviceClass {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Device bus type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    Platform, // Platform/system devices
    Pci,
    PciExpress,
    Usb,
    I2c,
    Spi,
    Acpi,
    DeviceTree,
    Virtual,
}

impl Default for BusType {
    fn default() -> Self {
        Self::Platform
    }
}

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Discovered,
    Probing,
    Active,
    Suspended,
    Error,
    Removed,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::Discovered
    }
}

// =============================================================================
// DEVICE
// =============================================================================

/// Device descriptor
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    pub class: DeviceClass,
    pub bus: BusType,
    pub state: DeviceState,
    pub driver_id: Option<DriverId>,

    // Identification
    pub vendor_id: u32,
    pub device_id: u32,
    pub subsystem_vendor: u32,
    pub subsystem_device: u32,
    pub revision: u8,
    pub class_code: u32,

    // Resources
    pub mmio_regions: Vec<MmioRegion>,
    pub io_ports: Vec<IoPortRange>,
    pub irqs: Vec<u32>,
    pub dma_channels: Vec<u8>,

    // Parent/children
    pub parent: Option<DeviceId>,
    pub children: Vec<DeviceId>,
}

impl Device {
    /// Create new device
    pub fn new(id: DeviceId, name: String, class: DeviceClass, bus: BusType) -> Self {
        Self {
            id,
            name,
            class,
            bus,
            state: DeviceState::Discovered,
            driver_id: None,
            vendor_id: 0,
            device_id: 0,
            subsystem_vendor: 0,
            subsystem_device: 0,
            revision: 0,
            class_code: 0,
            mmio_regions: Vec::new(),
            io_ports: Vec::new(),
            irqs: Vec::new(),
            dma_channels: Vec::new(),
            parent: None,
            children: Vec::new(),
        }
    }

    /// Is device active?
    pub fn is_active(&self) -> bool {
        self.state == DeviceState::Active
    }

    /// Has driver attached?
    pub fn has_driver(&self) -> bool {
        self.driver_id.is_some()
    }
}

/// Memory-mapped I/O region
#[derive(Debug, Clone)]
pub struct MmioRegion {
    pub base: u64,
    pub size: u64,
    pub flags: u32,
}

/// I/O port range
#[derive(Debug, Clone)]
pub struct IoPortRange {
    pub start: u16,
    pub count: u16,
}

// =============================================================================
// DRIVER
// =============================================================================

/// Driver trait
pub trait Driver: Send + Sync {
    /// Get driver info
    fn info(&self) -> &DriverInfo;

    /// Probe device
    fn probe(&mut self, device: &mut Device) -> InitResult<bool>;

    /// Remove device
    fn remove(&mut self, device: &mut Device) -> InitResult<()>;

    /// Suspend device
    fn suspend(&mut self, _device: &mut Device) -> InitResult<()> {
        Ok(())
    }

    /// Resume device
    fn resume(&mut self, _device: &mut Device) -> InitResult<()> {
        Ok(())
    }

    /// Shutdown device
    fn shutdown(&mut self, device: &mut Device) -> InitResult<()> {
        self.remove(device)
    }
}

/// Driver information
#[derive(Debug, Clone)]
pub struct DriverInfo {
    pub id: DriverId,
    pub name: &'static str,
    pub version: &'static str,
    pub author: &'static str,
    pub description: &'static str,
    pub license: &'static str,
    pub class: DeviceClass,
    pub bus: BusType,
    pub match_table: Vec<DeviceMatch>,
}

/// Device matching criteria
#[derive(Debug, Clone)]
pub struct DeviceMatch {
    pub vendor_id: Option<u32>,
    pub device_id: Option<u32>,
    pub class_code: Option<u32>,
    pub class_mask: u32,
}

impl DeviceMatch {
    /// Create match for specific vendor/device
    pub fn vendor_device(vendor: u32, device: u32) -> Self {
        Self {
            vendor_id: Some(vendor),
            device_id: Some(device),
            class_code: None,
            class_mask: 0,
        }
    }

    /// Create match for device class
    pub fn class(class_code: u32, mask: u32) -> Self {
        Self {
            vendor_id: None,
            device_id: None,
            class_code: Some(class_code),
            class_mask: mask,
        }
    }

    /// Check if device matches
    pub fn matches(&self, device: &Device) -> bool {
        if let Some(vendor) = self.vendor_id {
            if device.vendor_id != vendor {
                return false;
            }
        }

        if let Some(dev_id) = self.device_id {
            if device.device_id != dev_id {
                return false;
            }
        }

        if let Some(class) = self.class_code {
            if (device.class_code & self.class_mask) != (class & self.class_mask) {
                return false;
            }
        }

        true
    }
}

/// Registered driver
struct RegisteredDriver {
    info: DriverInfo,
    driver: Box<dyn Driver>,
}

// =============================================================================
// DRIVER SUBSYSTEM
// =============================================================================

/// Driver Subsystem
///
/// Manages device drivers and device discovery.
pub struct DriverSubsystem {
    info: SubsystemInfo,

    // Devices
    devices: Vec<Device>,
    next_device_id: AtomicU64,

    // Drivers
    drivers: Vec<RegisteredDriver>,
    next_driver_id: AtomicU64,

    // Statistics
    devices_discovered: u32,
    devices_active: u32,
    drivers_loaded: u32,
}

static DRIVER_DEPS: [Dependency; 3] = [
    Dependency::required("interrupts"),
    Dependency::required("heap"),
    Dependency::required("ipc"),
];

impl DriverSubsystem {
    /// Create new driver subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("drivers", InitPhase::Late)
                .with_priority(900)
                .with_description("Device driver management")
                .with_dependencies(&DRIVER_DEPS)
                .provides(PhaseCapabilities::DRIVERS),
            devices: Vec::new(),
            next_device_id: AtomicU64::new(1),
            drivers: Vec::new(),
            next_driver_id: AtomicU64::new(1),
            devices_discovered: 0,
            devices_active: 0,
            drivers_loaded: 0,
        }
    }

    /// Register device
    pub fn register_device(&mut self, mut device: Device) -> DeviceId {
        device.id = self.next_device_id.fetch_add(1, Ordering::SeqCst);
        let id = device.id;

        self.devices.push(device);
        self.devices_discovered += 1;

        id
    }

    /// Get device by ID
    pub fn get_device(&self, id: DeviceId) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Get device by ID (mutable)
    pub fn get_device_mut(&mut self, id: DeviceId) -> Option<&mut Device> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    /// Find devices by class
    pub fn find_by_class(&self, class: DeviceClass) -> Vec<DeviceId> {
        self.devices
            .iter()
            .filter(|d| d.class == class)
            .map(|d| d.id)
            .collect()
    }

    /// Find devices by bus
    pub fn find_by_bus(&self, bus: BusType) -> Vec<DeviceId> {
        self.devices
            .iter()
            .filter(|d| d.bus == bus)
            .map(|d| d.id)
            .collect()
    }

    /// Register driver
    pub fn register_driver(&mut self, mut info: DriverInfo, driver: Box<dyn Driver>) -> DriverId {
        info.id = self.next_driver_id.fetch_add(1, Ordering::SeqCst);
        let id = info.id;

        self.drivers.push(RegisteredDriver { info, driver });
        self.drivers_loaded += 1;

        id
    }

    /// Probe all devices with registered drivers
    pub fn probe_all(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Probing devices...");

        for device_idx in 0..self.devices.len() {
            let device = &self.devices[device_idx];
            if device.driver_id.is_some() {
                continue; // Already has driver
            }

            // Find matching driver
            let mut matching_driver = None;
            for driver_idx in 0..self.drivers.len() {
                let driver_info = &self.drivers[driver_idx].info;

                for match_entry in &driver_info.match_table {
                    if match_entry.matches(device) {
                        matching_driver = Some(driver_idx);
                        break;
                    }
                }

                if matching_driver.is_some() {
                    break;
                }
            }

            if let Some(driver_idx) = matching_driver {
                let device = &mut self.devices[device_idx];
                let registered = &mut self.drivers[driver_idx];

                device.state = DeviceState::Probing;

                match registered.driver.probe(device) {
                    Ok(true) => {
                        device.driver_id = Some(registered.info.id);
                        device.state = DeviceState::Active;
                        self.devices_active += 1;

                        ctx.debug(alloc::format!(
                            "Device '{}' bound to driver '{}'",
                            device.name,
                            registered.info.name
                        ));
                    },
                    Ok(false) => {
                        device.state = DeviceState::Discovered;
                    },
                    Err(e) => {
                        device.state = DeviceState::Error;
                        ctx.warn(alloc::format!(
                            "Failed to probe device '{}': {:?}",
                            device.name,
                            e
                        ));
                    },
                }
            }
        }

        ctx.info(alloc::format!(
            "Probed {} devices, {} active",
            self.devices_discovered,
            self.devices_active
        ));

        Ok(())
    }

    /// Discover platform devices
    fn discover_platform_devices(&mut self, ctx: &mut InitContext) {
        ctx.debug("Discovering platform devices...");

        // Serial ports (x86_64)
        #[cfg(target_arch = "x86_64")]
        {
            for (i, port) in [0x3F8u16, 0x2F8, 0x3E8, 0x2E8].iter().enumerate() {
                let mut dev = Device::new(
                    0,
                    alloc::format!("serial{}", i),
                    DeviceClass::SerialPort,
                    BusType::Platform,
                );
                dev.io_ports.push(IoPortRange {
                    start: *port,
                    count: 8,
                });
                dev.irqs.push(if i < 2 { 4 } else { 3 });
                self.register_device(dev);
            }
        }

        // Keyboard and mouse
        #[cfg(target_arch = "x86_64")]
        {
            let mut kbd = Device::new(
                0,
                String::from("ps2-keyboard"),
                DeviceClass::Keyboard,
                BusType::Platform,
            );
            kbd.io_ports.push(IoPortRange {
                start: 0x60,
                count: 1,
            });
            kbd.io_ports.push(IoPortRange {
                start: 0x64,
                count: 1,
            });
            kbd.irqs.push(1);
            self.register_device(kbd);

            let mut mouse = Device::new(
                0,
                String::from("ps2-mouse"),
                DeviceClass::Mouse,
                BusType::Platform,
            );
            mouse.irqs.push(12);
            self.register_device(mouse);
        }

        // RTC
        #[cfg(target_arch = "x86_64")]
        {
            let mut rtc = Device::new(0, String::from("rtc"), DeviceClass::Rtc, BusType::Platform);
            rtc.io_ports.push(IoPortRange {
                start: 0x70,
                count: 2,
            });
            rtc.irqs.push(8);
            self.register_device(rtc);
        }
    }

    /// Discover PCI devices
    #[cfg(target_arch = "x86_64")]
    fn discover_pci_devices(&mut self, ctx: &mut InitContext) {
        ctx.debug("Scanning PCI bus...");

        for bus in 0..=255u8 {
            for device in 0..32u8 {
                for function in 0..8u8 {
                    if let Some(dev) = self.probe_pci_device(bus, device, function) {
                        ctx.debug(alloc::format!(
                            "PCI {:02x}:{:02x}.{}: {:04x}:{:04x}",
                            bus,
                            device,
                            function,
                            dev.vendor_id,
                            dev.device_id
                        ));
                        self.register_device(dev);
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn probe_pci_device(&self, bus: u8, device: u8, function: u8) -> Option<Device> {
        let addr = 0x8000_0000u32
            | ((bus as u32) << 16)
            | ((device as u32) << 11)
            | ((function as u32) << 8);

        // Read vendor/device ID
        let config = self.pci_read_config(addr, 0);
        let vendor_id = (config & 0xFFFF) as u32;
        let device_id = ((config >> 16) & 0xFFFF) as u32;

        if vendor_id == 0xFFFF || vendor_id == 0 {
            return None;
        }

        // Read class code
        let class_config = self.pci_read_config(addr, 0x08);
        let class_code = (class_config >> 8) & 0xFFFFFF;
        let revision = (class_config & 0xFF) as u8;

        let device_class = Self::pci_class_to_device_class(class_code);

        let mut dev = Device::new(
            0,
            alloc::format!("pci-{:04x}:{:04x}", vendor_id, device_id),
            device_class,
            BusType::Pci,
        );
        dev.vendor_id = vendor_id;
        dev.device_id = device_id;
        dev.class_code = class_code;
        dev.revision = revision;

        // Read BARs
        for bar_idx in 0..6 {
            let bar_offset = 0x10 + (bar_idx * 4);
            let bar = self.pci_read_config(addr, bar_offset);

            if bar != 0 {
                if (bar & 1) == 0 {
                    // MMIO
                    dev.mmio_regions.push(MmioRegion {
                        base: (bar & !0xF) as u64,
                        size: 0, // Would need to probe size
                        flags: bar & 0xF,
                    });
                } else {
                    // I/O port
                    dev.io_ports.push(IoPortRange {
                        start: (bar & !0x3) as u16,
                        count: 0, // Would need to probe size
                    });
                }
            }
        }

        // Read IRQ
        let irq_config = self.pci_read_config(addr, 0x3C);
        let irq = (irq_config & 0xFF) as u32;
        if irq != 0 && irq != 255 {
            dev.irqs.push(irq);
        }

        Some(dev)
    }

    #[cfg(target_arch = "x86_64")]
    fn pci_read_config(&self, addr: u32, offset: u8) -> u32 {
        let config_addr = addr | (offset as u32);

        unsafe {
            // Write config address
            core::arch::asm!(
                "out dx, eax",
                in("dx") 0xCF8u16,
                in("eax") config_addr,
                options(nostack)
            );

            // Read config data
            let value: u32;
            core::arch::asm!(
                "in eax, dx",
                out("eax") value,
                in("dx") 0xCFCu16,
                options(nostack)
            );
            value
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn pci_class_to_device_class(class_code: u32) -> DeviceClass {
        match (class_code >> 16) & 0xFF {
            0x01 => DeviceClass::BlockDevice,      // Mass storage
            0x02 => DeviceClass::NetworkInterface, // Network
            0x03 => DeviceClass::Display,          // Display
            0x04 => DeviceClass::AudioOutput,      // Multimedia
            0x07 => DeviceClass::SerialPort,       // Simple comm
            0x0C => DeviceClass::UsbBus,           // Serial bus
            _ => DeviceClass::Unknown,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> DriverStats {
        DriverStats {
            devices_discovered: self.devices_discovered,
            devices_active: self.devices_active,
            drivers_loaded: self.drivers_loaded,
        }
    }
}

/// Driver statistics
#[derive(Debug, Clone)]
pub struct DriverStats {
    pub devices_discovered: u32,
    pub devices_active: u32,
    pub drivers_loaded: u32,
}

impl Default for DriverSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for DriverSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing driver subsystem");

        // Discover platform devices
        self.discover_platform_devices(ctx);

        // Discover PCI devices
        #[cfg(target_arch = "x86_64")]
        self.discover_pci_devices(ctx);

        // Probe devices with registered drivers
        self.probe_all(ctx)?;

        let stats = self.stats();
        ctx.info(alloc::format!(
            "Driver subsystem: {} devices, {} drivers",
            stats.devices_discovered,
            stats.drivers_loaded
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Driver subsystem shutdown");

        // Shutdown all active devices (reverse order)
        for device in self.devices.iter_mut().rev() {
            if device.state == DeviceState::Active {
                if let Some(driver_id) = device.driver_id {
                    if let Some(reg) = self.drivers.iter_mut().find(|d| d.info.id == driver_id) {
                        let _ = reg.driver.shutdown(device);
                    }
                }
            }
        }

        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_subsystem() {
        let sub = DriverSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Late);
        assert!(sub.info().provides.contains(PhaseCapabilities::DRIVERS));
    }

    #[test]
    fn test_device_creation() {
        let dev = Device::new(
            1,
            String::from("test"),
            DeviceClass::BlockDevice,
            BusType::Pci,
        );

        assert_eq!(dev.id, 1);
        assert_eq!(dev.name, "test");
        assert_eq!(dev.class, DeviceClass::BlockDevice);
        assert!(!dev.is_active());
        assert!(!dev.has_driver());
    }

    #[test]
    fn test_device_match() {
        let mut dev = Device::new(1, String::from("test"), DeviceClass::Unknown, BusType::Pci);
        dev.vendor_id = 0x8086;
        dev.device_id = 0x1234;

        let match1 = DeviceMatch::vendor_device(0x8086, 0x1234);
        assert!(match1.matches(&dev));

        let match2 = DeviceMatch::vendor_device(0x8086, 0x5678);
        assert!(!match2.matches(&dev));
    }
}
