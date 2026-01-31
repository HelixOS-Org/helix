//! Block Device Core Types
//!
//! Fundamental types for block device intelligence.

use alloc::format;
use alloc::string::String;

/// Block device major number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Major(pub u32);

/// Block device minor number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Minor(pub u32);

/// Block device ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockDeviceId {
    /// Major number
    pub major: Major,
    /// Minor number
    pub minor: Minor,
}

impl BlockDeviceId {
    /// Create new device ID
    pub const fn new(major: u32, minor: u32) -> Self {
        Self {
            major: Major(major),
            minor: Minor(minor),
        }
    }

    /// Create dev_t value
    pub fn dev_t(&self) -> u64 {
        ((self.major.0 as u64) << 20) | (self.minor.0 as u64)
    }

    /// Format as string
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.major.0, self.minor.0)
    }
}

/// Well-known major numbers
pub mod majors {
    use super::Major;

    pub const RAM: Major = Major(1);
    pub const FLOPPY: Major = Major(2);
    pub const IDE0: Major = Major(3);
    pub const SCSI_DISK0: Major = Major(8);
    pub const SCSI_DISK1: Major = Major(65);
    pub const SCSI_DISK2: Major = Major(66);
    pub const MD: Major = Major(9);
    pub const LOOP: Major = Major(7);
    pub const DEVICE_MAPPER: Major = Major(253);
    pub const NVME: Major = Major(259);
    pub const MMC: Major = Major(179);
    pub const VIRTIO_BLK: Major = Major(254);
}

/// Block device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceType {
    /// Hard disk drive
    Hdd,
    /// Solid state drive
    Ssd,
    /// NVMe drive
    Nvme,
    /// Virtual disk
    Virtual,
    /// Loop device
    Loop,
    /// RAM disk
    Ram,
    /// RAID array
    Raid,
    /// Device mapper
    DeviceMapper,
    /// MMC/SD card
    Mmc,
    /// Floppy
    Floppy,
    /// Optical (CD/DVD)
    Optical,
    /// Unknown
    Unknown,
}

impl BlockDeviceType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Hdd => "hdd",
            Self::Ssd => "ssd",
            Self::Nvme => "nvme",
            Self::Virtual => "virtual",
            Self::Loop => "loop",
            Self::Ram => "ram",
            Self::Raid => "raid",
            Self::DeviceMapper => "dm",
            Self::Mmc => "mmc",
            Self::Floppy => "floppy",
            Self::Optical => "optical",
            Self::Unknown => "unknown",
        }
    }

    /// Is rotational
    pub fn is_rotational(&self) -> bool {
        matches!(self, Self::Hdd | Self::Floppy | Self::Optical)
    }

    /// Is solid state
    pub fn is_solid_state(&self) -> bool {
        matches!(self, Self::Ssd | Self::Nvme | Self::Ram)
    }

    /// Supports trim
    pub fn supports_trim(&self) -> bool {
        matches!(self, Self::Ssd | Self::Nvme)
    }
}

/// Block device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDeviceState {
    /// Active and operational
    Active,
    /// Suspended
    Suspended,
    /// Error
    Error,
    /// Removed
    Removed,
    /// Initializing
    Initializing,
}

impl BlockDeviceState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Error => "error",
            Self::Removed => "removed",
            Self::Initializing => "initializing",
        }
    }
}
