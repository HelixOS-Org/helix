//! Device Core Types
//!
//! Fundamental types for device management.

/// Device identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub u64);

impl DeviceId {
    /// Create a new device ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Driver identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DriverId(pub u64);

impl DriverId {
    /// Create a new driver ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Bus identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BusId(pub u64);

impl BusId {
    /// Create a new bus ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Device class identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassId(pub u64);

impl ClassId {
    /// Create a new class ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Bus types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusType {
    /// PCI/PCIe bus
    Pci,
    /// USB bus
    Usb,
    /// I2C bus
    I2c,
    /// SPI bus
    Spi,
    /// ACPI bus
    Acpi,
    /// Platform bus
    Platform,
    /// GPIO bus
    Gpio,
    /// SDIO bus
    Sdio,
    /// Virtio bus
    Virtio,
    /// NVMe bus
    Nvme,
    /// CXL bus
    Cxl,
    /// Thunderbolt bus
    Thunderbolt,
    /// Other bus type
    Other,
}

impl BusType {
    /// Check if bus supports hotplug
    pub fn supports_hotplug(&self) -> bool {
        matches!(self, Self::Usb | Self::Thunderbolt | Self::Pci | Self::Sdio)
    }

    /// Check if bus supports power management
    pub fn supports_power_management(&self) -> bool {
        matches!(self, Self::Pci | Self::Usb | Self::Acpi | Self::Platform)
    }

    /// Get typical probe time (microseconds)
    pub fn typical_probe_time_us(&self) -> u64 {
        match self {
            Self::Pci => 100,
            Self::Usb => 500,
            Self::I2c => 50,
            Self::Spi => 20,
            Self::Acpi => 200,
            Self::Platform => 50,
            Self::Gpio => 10,
            Self::Sdio => 300,
            Self::Virtio => 50,
            Self::Nvme => 1000,
            Self::Cxl => 500,
            Self::Thunderbolt => 2000,
            Self::Other => 100,
        }
    }
}

/// Device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device not initialized
    NotInitialized,
    /// Device probing in progress
    Probing,
    /// Device bound to driver
    Bound,
    /// Device suspended
    Suspended,
    /// Device removed
    Removed,
    /// Device in error state
    Error,
    /// Device deferred probe
    DeferredProbe,
}

/// Device power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerState {
    /// Full power (D0)
    D0,
    /// Low power standby (D1)
    D1,
    /// Device specific low power (D2)
    D2,
    /// Device off but powered (D3hot)
    D3Hot,
    /// Device off no power (D3cold)
    D3Cold,
}

impl PowerState {
    /// Get power consumption factor (0.0-1.0)
    pub fn power_factor(&self) -> f32 {
        match self {
            Self::D0 => 1.0,
            Self::D1 => 0.3,
            Self::D2 => 0.1,
            Self::D3Hot => 0.01,
            Self::D3Cold => 0.0,
        }
    }

    /// Get wake latency (microseconds)
    pub fn wake_latency_us(&self) -> u64 {
        match self {
            Self::D0 => 0,
            Self::D1 => 10,
            Self::D2 => 100,
            Self::D3Hot => 1000,
            Self::D3Cold => 100_000,
        }
    }
}
