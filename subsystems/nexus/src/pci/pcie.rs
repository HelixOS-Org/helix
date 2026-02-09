//! PCIe link and power management.

// ============================================================================
// PCIE LINK
// ============================================================================

/// PCIe link speed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PcieLinkSpeed {
    /// 2.5 GT/s (Gen1)
    Gen1,
    /// 5.0 GT/s (Gen2)
    Gen2,
    /// 8.0 GT/s (Gen3)
    Gen3,
    /// 16.0 GT/s (Gen4)
    Gen4,
    /// 32.0 GT/s (Gen5)
    Gen5,
    /// 64.0 GT/s (Gen6)
    Gen6,
    /// Unknown
    Unknown,
}

impl PcieLinkSpeed {
    /// Get speed name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gen1 => "2.5GT/s",
            Self::Gen2 => "5.0GT/s",
            Self::Gen3 => "8.0GT/s",
            Self::Gen4 => "16.0GT/s",
            Self::Gen5 => "32.0GT/s",
            Self::Gen6 => "64.0GT/s",
            Self::Unknown => "unknown",
        }
    }

    /// From encoding
    #[inline]
    pub fn from_encoding(encoding: u8) -> Self {
        match encoding {
            1 => Self::Gen1,
            2 => Self::Gen2,
            3 => Self::Gen3,
            4 => Self::Gen4,
            5 => Self::Gen5,
            6 => Self::Gen6,
            _ => Self::Unknown,
        }
    }

    /// Get bandwidth per lane (MB/s)
    #[inline]
    pub fn bandwidth_per_lane(&self) -> u32 {
        match self {
            Self::Gen1 => 250,  // 2.5GT/s * 8/10 = 2Gbps = 250MB/s
            Self::Gen2 => 500,  // 5GT/s * 8/10 = 4Gbps = 500MB/s
            Self::Gen3 => 985,  // 8GT/s * 128/130 = ~7.877Gbps = 985MB/s
            Self::Gen4 => 1969, // 16GT/s * 128/130 = ~15.754Gbps = 1969MB/s
            Self::Gen5 => 3938, // 32GT/s * 128/130 = ~31.508Gbps = 3938MB/s
            Self::Gen6 => 7877, // 64GT/s * 128/130 (PAM4) = ~63Gbps = 7877MB/s
            Self::Unknown => 0,
        }
    }
}

/// PCIe link width
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PcieLinkWidth {
    /// x1
    X1,
    /// x2
    X2,
    /// x4
    X4,
    /// x8
    X8,
    /// x12
    X12,
    /// x16
    X16,
    /// x32
    X32,
    /// Unknown
    Unknown,
}

impl PcieLinkWidth {
    /// Get width name
    pub fn name(&self) -> &'static str {
        match self {
            Self::X1 => "x1",
            Self::X2 => "x2",
            Self::X4 => "x4",
            Self::X8 => "x8",
            Self::X12 => "x12",
            Self::X16 => "x16",
            Self::X32 => "x32",
            Self::Unknown => "x?",
        }
    }

    /// From encoding
    pub fn from_encoding(encoding: u8) -> Self {
        match encoding {
            1 => Self::X1,
            2 => Self::X2,
            4 => Self::X4,
            8 => Self::X8,
            12 => Self::X12,
            16 => Self::X16,
            32 => Self::X32,
            _ => Self::Unknown,
        }
    }

    /// Get multiplier
    pub fn lanes(&self) -> u8 {
        match self {
            Self::X1 => 1,
            Self::X2 => 2,
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X12 => 12,
            Self::X16 => 16,
            Self::X32 => 32,
            Self::Unknown => 0,
        }
    }
}

/// PCIe link info
#[derive(Debug, Clone)]
pub struct PcieLink {
    /// Current speed
    pub speed: PcieLinkSpeed,
    /// Current width
    pub width: PcieLinkWidth,
    /// Max speed
    pub max_speed: PcieLinkSpeed,
    /// Max width
    pub max_width: PcieLinkWidth,
    /// Link active
    pub active: bool,
}

impl PcieLink {
    /// Create new link info
    pub fn new() -> Self {
        Self {
            speed: PcieLinkSpeed::Unknown,
            width: PcieLinkWidth::Unknown,
            max_speed: PcieLinkSpeed::Unknown,
            max_width: PcieLinkWidth::Unknown,
            active: false,
        }
    }

    /// Get current bandwidth (MB/s)
    #[inline(always)]
    pub fn bandwidth(&self) -> u32 {
        self.speed.bandwidth_per_lane() * self.width.lanes() as u32
    }

    /// Get max bandwidth (MB/s)
    #[inline(always)]
    pub fn max_bandwidth(&self) -> u32 {
        self.max_speed.bandwidth_per_lane() * self.max_width.lanes() as u32
    }

    /// Efficiency (current/max)
    #[inline]
    pub fn efficiency(&self) -> f32 {
        let max = self.max_bandwidth();
        if max > 0 {
            self.bandwidth() as f32 / max as f32
        } else {
            0.0
        }
    }
}

impl Default for PcieLink {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// POWER STATE
// ============================================================================

/// Power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    /// D0 (fully operational)
    D0,
    /// D1 (light sleep)
    D1,
    /// D2 (deeper sleep)
    D2,
    /// D3hot (deeper still)
    D3Hot,
    /// D3cold (powered off)
    D3Cold,
    /// Unknown
    Unknown,
}

impl PowerState {
    /// Get state name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::D0 => "D0",
            Self::D1 => "D1",
            Self::D2 => "D2",
            Self::D3Hot => "D3hot",
            Self::D3Cold => "D3cold",
            Self::Unknown => "unknown",
        }
    }
}
