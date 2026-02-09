//! Core I/O types.

// ============================================================================
// I/O OPERATION TYPE
// ============================================================================

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOpType {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Flush operation
    Flush,
    /// Discard/Trim operation
    Discard,
    /// Sync operation
    Sync,
}

// ============================================================================
// I/O PRIORITY
// ============================================================================

/// I/O request priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriority {
    /// Idle priority
    Idle     = 0,
    /// Low priority
    Low      = 1,
    /// Normal priority
    Normal   = 2,
    /// High priority
    High     = 3,
    /// Real-time priority
    RealTime = 4,
}

// ============================================================================
// DEVICE TYPE
// ============================================================================

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Hard disk drive
    Hdd,
    /// Solid state drive
    Ssd,
    /// NVMe drive
    Nvme,
    /// RAM disk
    RamDisk,
    /// Network storage
    Network,
    /// Virtual device
    Virtual,
    /// Unknown device
    Unknown,
}

impl DeviceType {
    /// Get typical latency in microseconds
    #[inline]
    pub fn typical_latency_us(&self) -> u64 {
        match self {
            Self::Hdd => 5000,
            Self::Ssd => 100,
            Self::Nvme => 20,
            Self::RamDisk => 1,
            Self::Network => 10000,
            Self::Virtual => 50,
            Self::Unknown => 1000,
        }
    }

    /// Get typical bandwidth in MB/s
    #[inline]
    pub fn typical_bandwidth_mbs(&self) -> u64 {
        match self {
            Self::Hdd => 150,
            Self::Ssd => 500,
            Self::Nvme => 3000,
            Self::RamDisk => 10000,
            Self::Network => 100,
            Self::Virtual => 1000,
            Self::Unknown => 100,
        }
    }

    /// Is sequential access preferred?
    #[inline(always)]
    pub fn prefers_sequential(&self) -> bool {
        matches!(self, Self::Hdd)
    }
}
