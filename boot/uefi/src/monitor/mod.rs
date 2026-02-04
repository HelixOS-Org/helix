//! Advanced Boot Monitoring and Diagnostics for Helix UEFI Bootloader
//!
//! This module provides comprehensive boot monitoring, performance profiling,
//! and diagnostic capabilities for troubleshooting boot issues.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Boot Monitoring Architecture                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                        Event Collector                           │  │
//! │  │  Timestamps │ Metrics │ Errors │ Warnings │ Resource Usage       │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │         ┌────────────────────┼────────────────────┐                    │
//! │         ▼                    ▼                    ▼                    │
//! │  ┌────────────┐     ┌────────────────┐    ┌─────────────┐             │
//! │  │ Performance│     │    Health     │    │   Trace     │             │
//! │  │  Profiler  │     │   Monitor     │    │   Logger    │             │
//! │  └────────────┘     └────────────────┘    └─────────────┘             │
//! │         │                    │                    │                    │
//! │         ▼                    ▼                    ▼                    │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                      Diagnostic Output                           │  │
//! │  │  Console │ Serial │ Log File │ Network │ NVRAM                   │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - Boot time profiling with microsecond precision
//! - Memory usage tracking
//! - Driver load timing
//! - Error and warning collection
//! - Health status monitoring
//! - Performance metrics

use core::fmt;

// =============================================================================
// BOOT PHASES
// =============================================================================

/// Boot phase identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BootPhase {
    /// Security (SEC) phase
    Sec = 0,
    /// Pre-EFI Initialization
    Pei = 1,
    /// Driver Execution Environment
    Dxe = 2,
    /// Boot Device Selection
    Bds = 3,
    /// Transient System Load
    Tsl = 4,
    /// Runtime
    Rt  = 5,
    /// After Life (post ExitBootServices)
    Al  = 6,
}

impl BootPhase {
    /// Get phase name
    pub const fn name(&self) -> &'static str {
        match self {
            BootPhase::Sec => "SEC",
            BootPhase::Pei => "PEI",
            BootPhase::Dxe => "DXE",
            BootPhase::Bds => "BDS",
            BootPhase::Tsl => "TSL",
            BootPhase::Rt => "RT",
            BootPhase::Al => "AL",
        }
    }

    /// Get phase description
    pub const fn description(&self) -> &'static str {
        match self {
            BootPhase::Sec => "Security Phase",
            BootPhase::Pei => "Pre-EFI Initialization",
            BootPhase::Dxe => "Driver Execution Environment",
            BootPhase::Bds => "Boot Device Selection",
            BootPhase::Tsl => "Transient System Load",
            BootPhase::Rt => "Runtime",
            BootPhase::Al => "After Life",
        }
    }
}

impl fmt::Display for BootPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// BOOT STAGE
// =============================================================================

/// Detailed boot stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStage {
    /// Firmware initialization
    FirmwareInit,
    /// Memory detection
    MemoryDetect,
    /// Memory initialization
    MemoryInit,
    /// CPU initialization
    CpuInit,
    /// Chipset initialization
    ChipsetInit,
    /// Board initialization
    BoardInit,
    /// PCI enumeration
    PciEnumerate,
    /// Console initialization
    ConsoleInit,
    /// Driver loading
    DriverLoad,
    /// Protocol installation
    ProtocolInstall,
    /// Boot option enumeration
    BootOptionEnum,
    /// Boot option selection
    BootOptionSelect,
    /// Boot loader load
    BootLoaderLoad,
    /// Boot loader start
    BootLoaderStart,
    /// Kernel load
    KernelLoad,
    /// Exit boot services
    ExitBootServices,
    /// Kernel start
    KernelStart,
}

impl BootStage {
    /// Get stage name
    pub const fn name(&self) -> &'static str {
        match self {
            BootStage::FirmwareInit => "FirmwareInit",
            BootStage::MemoryDetect => "MemoryDetect",
            BootStage::MemoryInit => "MemoryInit",
            BootStage::CpuInit => "CpuInit",
            BootStage::ChipsetInit => "ChipsetInit",
            BootStage::BoardInit => "BoardInit",
            BootStage::PciEnumerate => "PciEnumerate",
            BootStage::ConsoleInit => "ConsoleInit",
            BootStage::DriverLoad => "DriverLoad",
            BootStage::ProtocolInstall => "ProtocolInstall",
            BootStage::BootOptionEnum => "BootOptionEnum",
            BootStage::BootOptionSelect => "BootOptionSelect",
            BootStage::BootLoaderLoad => "BootLoaderLoad",
            BootStage::BootLoaderStart => "BootLoaderStart",
            BootStage::KernelLoad => "KernelLoad",
            BootStage::ExitBootServices => "ExitBootServices",
            BootStage::KernelStart => "KernelStart",
        }
    }

    /// Get expected phase
    pub const fn expected_phase(&self) -> BootPhase {
        match self {
            BootStage::FirmwareInit => BootPhase::Sec,
            BootStage::MemoryDetect | BootStage::MemoryInit => BootPhase::Pei,
            BootStage::CpuInit
            | BootStage::ChipsetInit
            | BootStage::BoardInit
            | BootStage::PciEnumerate
            | BootStage::ConsoleInit
            | BootStage::DriverLoad
            | BootStage::ProtocolInstall => BootPhase::Dxe,
            BootStage::BootOptionEnum | BootStage::BootOptionSelect => BootPhase::Bds,
            BootStage::BootLoaderLoad
            | BootStage::BootLoaderStart
            | BootStage::KernelLoad
            | BootStage::ExitBootServices => BootPhase::Tsl,
            BootStage::KernelStart => BootPhase::Al,
        }
    }
}

impl fmt::Display for BootStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// =============================================================================
// TIMING AND PERFORMANCE
// =============================================================================

/// Timestamp with nanosecond precision
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    /// Seconds since boot
    pub seconds: u64,
    /// Nanoseconds within second
    pub nanoseconds: u32,
}

impl Timestamp {
    /// Zero timestamp
    pub const ZERO: Self = Self {
        seconds: 0,
        nanoseconds: 0,
    };

    /// Create new timestamp
    pub const fn new(seconds: u64, nanoseconds: u32) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }

    /// Create from microseconds
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            seconds: micros / 1_000_000,
            nanoseconds: ((micros % 1_000_000) * 1000) as u32,
        }
    }

    /// Create from milliseconds
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            seconds: millis / 1000,
            nanoseconds: ((millis % 1000) * 1_000_000) as u32,
        }
    }

    /// Convert to microseconds
    pub const fn as_micros(&self) -> u64 {
        self.seconds * 1_000_000 + (self.nanoseconds / 1000) as u64
    }

    /// Convert to milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.seconds * 1000 + (self.nanoseconds / 1_000_000) as u64
    }

    /// Calculate duration from earlier timestamp
    pub fn duration_since(&self, earlier: &Self) -> Duration {
        let total_nanos = (self.seconds as i128 * 1_000_000_000 + self.nanoseconds as i128)
            - (earlier.seconds as i128 * 1_000_000_000 + earlier.nanoseconds as i128);

        if total_nanos < 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos(total_nanos as u64)
        }
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:09}s", self.seconds, self.nanoseconds)
    }
}

/// Duration measurement
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    /// Total nanoseconds
    nanos: u64,
}

impl Duration {
    /// Zero duration
    pub const ZERO: Self = Self { nanos: 0 };

    /// Create from nanoseconds
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create from microseconds
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros * 1000,
        }
    }

    /// Create from milliseconds
    pub const fn from_millis(millis: u64) -> Self {
        Self {
            nanos: millis * 1_000_000,
        }
    }

    /// Create from seconds
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nanos: secs * 1_000_000_000,
        }
    }

    /// Get total nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get total microseconds
    pub const fn as_micros(&self) -> u64 {
        self.nanos / 1000
    }

    /// Get total milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Get total seconds
    pub const fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Get fractional milliseconds
    pub const fn subsec_millis(&self) -> u32 {
        ((self.nanos / 1_000_000) % 1000) as u32
    }

    /// Get fractional microseconds
    pub const fn subsec_micros(&self) -> u32 {
        ((self.nanos / 1000) % 1_000_000) as u32
    }

    /// Add durations
    pub const fn add(&self, other: &Self) -> Self {
        Self {
            nanos: self.nanos + other.nanos,
        }
    }

    /// Subtract durations
    pub const fn sub(&self, other: &Self) -> Self {
        Self {
            nanos: self.nanos.saturating_sub(other.nanos),
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.nanos < 1000 {
            write!(f, "{}ns", self.nanos)
        } else if self.nanos < 1_000_000 {
            write!(f, "{}.{:03}µs", self.nanos / 1000, self.nanos % 1000)
        } else if self.nanos < 1_000_000_000 {
            write!(
                f,
                "{}.{:03}ms",
                self.nanos / 1_000_000,
                (self.nanos / 1000) % 1000
            )
        } else {
            write!(
                f,
                "{}.{:03}s",
                self.nanos / 1_000_000_000,
                (self.nanos / 1_000_000) % 1000
            )
        }
    }
}

// =============================================================================
// PERFORMANCE METRICS
// =============================================================================

/// Boot timing record
#[derive(Debug, Clone, Copy)]
pub struct BootTiming {
    /// Boot stage
    pub stage: BootStage,
    /// Start timestamp
    pub start: Timestamp,
    /// End timestamp (None if not completed)
    pub end: Option<Timestamp>,
}

impl BootTiming {
    /// Create new timing record
    pub const fn new(stage: BootStage, start: Timestamp) -> Self {
        Self {
            stage,
            start,
            end: None,
        }
    }

    /// Mark as completed
    pub fn complete(&mut self, end: Timestamp) {
        self.end = Some(end);
    }

    /// Get duration
    pub fn duration(&self) -> Option<Duration> {
        self.end.map(|end| end.duration_since(&self.start))
    }

    /// Check if completed
    pub const fn is_complete(&self) -> bool {
        self.end.is_some()
    }
}

/// Performance counter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterType {
    /// Count
    Count,
    /// Bytes
    Bytes,
    /// Microseconds
    Microseconds,
    /// Percentage (0-10000 = 0-100.00%)
    Percentage,
    /// Pages (4KB)
    Pages,
}

/// Performance counter
#[derive(Debug, Clone, Copy)]
pub struct PerfCounter {
    /// Counter name
    pub name: &'static str,
    /// Counter type
    pub counter_type: CounterType,
    /// Current value
    pub value: u64,
    /// Minimum value seen
    pub min: u64,
    /// Maximum value seen
    pub max: u64,
    /// Accumulator for average
    pub sum: u64,
    /// Sample count
    pub samples: u32,
}

impl PerfCounter {
    /// Create new counter
    pub const fn new(name: &'static str, counter_type: CounterType) -> Self {
        Self {
            name,
            counter_type,
            value: 0,
            min: u64::MAX,
            max: 0,
            sum: 0,
            samples: 0,
        }
    }

    /// Record a value
    pub fn record(&mut self, value: u64) {
        self.value = value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.sum = self.sum.saturating_add(value);
        self.samples = self.samples.saturating_add(1);
    }

    /// Get average
    pub fn average(&self) -> u64 {
        if self.samples == 0 {
            0
        } else {
            self.sum / self.samples as u64
        }
    }

    /// Reset counter
    pub fn reset(&mut self) {
        self.value = 0;
        self.min = u64::MAX;
        self.max = 0;
        self.sum = 0;
        self.samples = 0;
    }
}

impl fmt::Display for PerfCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match self.counter_type {
            CounterType::Count => "",
            CounterType::Bytes => " B",
            CounterType::Microseconds => " µs",
            CounterType::Percentage => "%",
            CounterType::Pages => " pages",
        };
        write!(
            f,
            "{}: {}{} (min:{}, max:{}, avg:{})",
            self.name,
            self.value,
            suffix,
            self.min,
            self.max,
            self.average()
        )
    }
}

// =============================================================================
// MEMORY TRACKING
// =============================================================================

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Available for use
    Available,
    /// Reserved by firmware
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Runtime services code
    RuntimeCode,
    /// Runtime services data
    RuntimeData,
    /// Boot services code
    BootCode,
    /// Boot services data
    BootData,
    /// Loader code
    LoaderCode,
    /// Loader data
    LoaderData,
    /// Unusable
    Unusable,
    /// Persistent memory
    Persistent,
}

/// Memory usage snapshot
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryUsage {
    /// Total available memory
    pub total_available: u64,
    /// Used memory
    pub used: u64,
    /// Free memory
    pub free: u64,
    /// Peak usage
    pub peak: u64,
    /// Number of allocations
    pub allocation_count: u32,
    /// Number of frees
    pub free_count: u32,
    /// Fragmentation percentage (0-100)
    pub fragmentation: u8,
}

impl MemoryUsage {
    /// Calculate usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.total_available == 0 {
            0
        } else {
            ((self.used * 100) / self.total_available) as u8
        }
    }

    /// Check if memory is low
    pub fn is_low(&self) -> bool {
        self.usage_percent() > 90
    }

    /// Check if memory is critical
    pub fn is_critical(&self) -> bool {
        self.usage_percent() > 95
    }
}

impl fmt::Display for MemoryUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memory: {} / {} ({:.1}% used, peak: {}, frag: {}%)",
            format_size(self.used),
            format_size(self.total_available),
            self.usage_percent() as f32,
            format_size(self.peak),
            self.fragmentation
        )
    }
}

/// Format size in human-readable form
fn format_size(bytes: u64) -> &'static str {
    // Simplified for no_std - returns unit only
    if bytes >= 1024 * 1024 * 1024 {
        "GB"
    } else if bytes >= 1024 * 1024 {
        "MB"
    } else if bytes >= 1024 {
        "KB"
    } else {
        "B"
    }
}

// =============================================================================
// HEALTH STATUS
// =============================================================================

/// Health status level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum HealthLevel {
    /// Everything is working correctly
    Healthy  = 0,
    /// Minor issues detected
    Degraded = 1,
    /// Significant problems
    Warning  = 2,
    /// Critical errors
    Critical = 3,
    /// System failed
    Failed   = 4,
}

impl HealthLevel {
    /// Get level name
    pub const fn name(&self) -> &'static str {
        match self {
            HealthLevel::Healthy => "HEALTHY",
            HealthLevel::Degraded => "DEGRADED",
            HealthLevel::Warning => "WARNING",
            HealthLevel::Critical => "CRITICAL",
            HealthLevel::Failed => "FAILED",
        }
    }

    /// Check if action needed
    pub const fn needs_attention(&self) -> bool {
        matches!(
            self,
            HealthLevel::Warning | HealthLevel::Critical | HealthLevel::Failed
        )
    }
}

impl Default for HealthLevel {
    fn default() -> Self {
        HealthLevel::Healthy
    }
}

impl fmt::Display for HealthLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Health check category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCategory {
    /// CPU health
    Cpu,
    /// Memory health
    Memory,
    /// Storage health
    Storage,
    /// Network health
    Network,
    /// Firmware health
    Firmware,
    /// Security health
    Security,
    /// Temperature
    Temperature,
    /// Power
    Power,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Category
    pub category: HealthCategory,
    /// Level
    pub level: HealthLevel,
    /// Message
    pub message: [u8; 128],
    /// Message length
    pub message_len: usize,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl HealthCheck {
    /// Create new health check
    pub const fn new(category: HealthCategory, level: HealthLevel) -> Self {
        Self {
            category,
            level,
            message: [0u8; 128],
            message_len: 0,
            timestamp: Timestamp::ZERO,
        }
    }

    /// Set message
    pub fn set_message(&mut self, msg: &[u8]) {
        let len = msg.len().min(self.message.len());
        self.message[..len].copy_from_slice(&msg[..len]);
        self.message_len = len;
    }
}

// =============================================================================
// EVENT LOGGING
// =============================================================================

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    /// Trace (most verbose)
    Trace    = 0,
    /// Debug
    Debug    = 1,
    /// Info
    Info     = 2,
    /// Warning
    Warning  = 3,
    /// Error
    Error    = 4,
    /// Critical (least verbose)
    Critical = 5,
}

impl LogLevel {
    /// Get level name
    pub const fn name(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Critical => "CRIT",
        }
    }

    /// Get level prefix character
    pub const fn prefix(&self) -> char {
        match self {
            LogLevel::Trace => 'T',
            LogLevel::Debug => 'D',
            LogLevel::Info => 'I',
            LogLevel::Warning => 'W',
            LogLevel::Error => 'E',
            LogLevel::Critical => 'C',
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Boot stage
    pub stage: BootStage,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Source module
    pub source: [u8; 32],
    /// Source length
    pub source_len: usize,
    /// Message
    pub message: [u8; 256],
    /// Message length
    pub message_len: usize,
    /// Error code (if any)
    pub error_code: Option<u32>,
}

impl LogEntry {
    /// Create new log entry
    pub const fn new(level: LogLevel, stage: BootStage, timestamp: Timestamp) -> Self {
        Self {
            level,
            stage,
            timestamp,
            source: [0u8; 32],
            source_len: 0,
            message: [0u8; 256],
            message_len: 0,
            error_code: None,
        }
    }

    /// Set source
    pub fn set_source(&mut self, source: &[u8]) {
        let len = source.len().min(self.source.len());
        self.source[..len].copy_from_slice(&source[..len]);
        self.source_len = len;
    }

    /// Set message
    pub fn set_message(&mut self, message: &[u8]) {
        let len = message.len().min(self.message.len());
        self.message[..len].copy_from_slice(&message[..len]);
        self.message_len = len;
    }
}

// =============================================================================
// DIAGNOSTIC CODES
// =============================================================================

/// POST (Power-On Self-Test) code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PostCode {
    /// System power on
    PowerOn          = 0x00,
    /// CPU initialization
    CpuInit          = 0x10,
    /// Early chipset initialization
    ChipsetEarly     = 0x20,
    /// Memory detection
    MemoryDetect     = 0x30,
    /// Memory initialization
    MemoryInit       = 0x31,
    /// Memory test
    MemoryTest       = 0x32,
    /// Cache initialization
    CacheInit        = 0x40,
    /// Microcode update
    MicrocodeUpdate  = 0x50,
    /// PCI initialization
    PciInit          = 0x60,
    /// USB initialization
    UsbInit          = 0x70,
    /// SATA initialization
    SataInit         = 0x71,
    /// NVMe initialization
    NvmeInit         = 0x72,
    /// Video initialization
    VideoInit        = 0x80,
    /// Console initialization
    ConsoleInit      = 0x90,
    /// Driver dispatch
    DriverDispatch   = 0xA0,
    /// BDS entry
    BdsEntry         = 0xB0,
    /// Boot option
    BootOption       = 0xC0,
    /// OS handoff
    OsHandoff        = 0xD0,
    /// Ready to boot
    ReadyToBoot      = 0xE0,
    /// Exit boot services
    ExitBootServices = 0xF0,
    /// Error
    Error            = 0xFF,
}

impl PostCode {
    /// Get code description
    pub const fn description(&self) -> &'static str {
        match self {
            PostCode::PowerOn => "Power On",
            PostCode::CpuInit => "CPU Initialization",
            PostCode::ChipsetEarly => "Early Chipset Init",
            PostCode::MemoryDetect => "Memory Detection",
            PostCode::MemoryInit => "Memory Initialization",
            PostCode::MemoryTest => "Memory Test",
            PostCode::CacheInit => "Cache Initialization",
            PostCode::MicrocodeUpdate => "Microcode Update",
            PostCode::PciInit => "PCI Initialization",
            PostCode::UsbInit => "USB Initialization",
            PostCode::SataInit => "SATA Initialization",
            PostCode::NvmeInit => "NVMe Initialization",
            PostCode::VideoInit => "Video Initialization",
            PostCode::ConsoleInit => "Console Initialization",
            PostCode::DriverDispatch => "Driver Dispatch",
            PostCode::BdsEntry => "BDS Entry",
            PostCode::BootOption => "Boot Option",
            PostCode::OsHandoff => "OS Handoff",
            PostCode::ReadyToBoot => "Ready To Boot",
            PostCode::ExitBootServices => "Exit Boot Services",
            PostCode::Error => "Error",
        }
    }
}

/// Beep code pattern
#[derive(Debug, Clone, Copy)]
pub struct BeepPattern {
    /// Number of beeps
    pub beeps: u8,
    /// Long beeps
    pub long_beeps: u8,
    /// Repeat count
    pub repeats: u8,
}

impl BeepPattern {
    /// Create pattern
    pub const fn new(beeps: u8, long_beeps: u8, repeats: u8) -> Self {
        Self {
            beeps,
            long_beeps,
            repeats,
        }
    }

    /// Memory error pattern
    pub const MEMORY_ERROR: Self = Self::new(3, 0, 3);
    /// Video error pattern
    pub const VIDEO_ERROR: Self = Self::new(1, 3, 1);
    /// Keyboard error pattern
    pub const KEYBOARD_ERROR: Self = Self::new(1, 1, 2);
    /// General error pattern
    pub const GENERAL_ERROR: Self = Self::new(1, 0, 1);
    /// Success pattern
    pub const SUCCESS: Self = Self::new(1, 0, 1);
}

// =============================================================================
// CHECKPOINT SYSTEM
// =============================================================================

/// Boot checkpoint
#[derive(Debug, Clone, Copy)]
pub struct Checkpoint {
    /// Checkpoint ID
    pub id: u32,
    /// Description
    pub description: &'static str,
    /// Expected duration (microseconds)
    pub expected_us: u32,
    /// Actual timestamp
    pub timestamp: Timestamp,
    /// Passed/failed
    pub passed: bool,
}

impl Checkpoint {
    /// Create new checkpoint
    pub const fn new(id: u32, description: &'static str, expected_us: u32) -> Self {
        Self {
            id,
            description,
            expected_us,
            timestamp: Timestamp::ZERO,
            passed: false,
        }
    }

    /// Mark as passed
    pub fn pass(&mut self, timestamp: Timestamp) {
        self.timestamp = timestamp;
        self.passed = true;
    }
}

/// Predefined checkpoints
pub mod checkpoints {
    use super::*;

    /// Firmware loaded
    pub const FIRMWARE_LOADED: Checkpoint = Checkpoint::new(1, "Firmware loaded", 500_000);
    /// Memory initialized
    pub const MEMORY_INIT: Checkpoint = Checkpoint::new(2, "Memory initialized", 1_000_000);
    /// Devices enumerated
    pub const DEVICES_ENUM: Checkpoint = Checkpoint::new(3, "Devices enumerated", 2_000_000);
    /// Console ready
    pub const CONSOLE_READY: Checkpoint = Checkpoint::new(4, "Console ready", 500_000);
    /// Boot menu displayed
    pub const BOOT_MENU: Checkpoint = Checkpoint::new(5, "Boot menu displayed", 100_000);
    /// Bootloader loaded
    pub const BOOTLOADER_LOADED: Checkpoint = Checkpoint::new(6, "Bootloader loaded", 500_000);
    /// Kernel loaded
    pub const KERNEL_LOADED: Checkpoint = Checkpoint::new(7, "Kernel loaded", 1_000_000);
    /// Exit boot services
    pub const EXIT_BOOT_SERVICES: Checkpoint = Checkpoint::new(8, "Exit boot services", 100_000);
}

// =============================================================================
// PROGRESS INDICATOR
// =============================================================================

/// Boot progress indicator
#[derive(Debug, Clone, Copy)]
pub struct BootProgress {
    /// Current progress (0-100)
    pub percent: u8,
    /// Current stage
    pub stage: BootStage,
    /// Current operation description
    pub operation: &'static str,
    /// Estimated time remaining (ms)
    pub eta_ms: u32,
}

impl BootProgress {
    /// Create new progress indicator
    pub const fn new() -> Self {
        Self {
            percent: 0,
            stage: BootStage::FirmwareInit,
            operation: "Initializing",
            eta_ms: 0,
        }
    }

    /// Update progress
    pub fn update(&mut self, percent: u8, stage: BootStage, operation: &'static str) {
        self.percent = percent.min(100);
        self.stage = stage;
        self.operation = operation;
    }

    /// Check if boot is complete
    pub const fn is_complete(&self) -> bool {
        self.percent >= 100
    }
}

impl Default for BootProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BootProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:3}%] {}: {}",
            self.percent, self.stage, self.operation
        )
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Monitoring error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorError {
    /// Buffer full
    BufferFull,
    /// Invalid timestamp
    InvalidTimestamp,
    /// Counter overflow
    CounterOverflow,
    /// Not initialized
    NotInitialized,
    /// Already initialized
    AlreadyInitialized,
    /// Invalid parameter
    InvalidParameter,
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonitorError::BufferFull => write!(f, "Buffer full"),
            MonitorError::InvalidTimestamp => write!(f, "Invalid timestamp"),
            MonitorError::CounterOverflow => write!(f, "Counter overflow"),
            MonitorError::NotInitialized => write!(f, "Not initialized"),
            MonitorError::AlreadyInitialized => write!(f, "Already initialized"),
            MonitorError::InvalidParameter => write!(f, "Invalid parameter"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp() {
        let ts = Timestamp::from_millis(1500);
        assert_eq!(ts.seconds, 1);
        assert_eq!(ts.nanoseconds, 500_000_000);
        assert_eq!(ts.as_millis(), 1500);
    }

    #[test]
    fn test_duration() {
        let d = Duration::from_millis(1500);
        assert_eq!(d.as_secs(), 1);
        assert_eq!(d.subsec_millis(), 500);
    }

    #[test]
    fn test_duration_since() {
        let start = Timestamp::from_millis(1000);
        let end = Timestamp::from_millis(2500);
        let dur = end.duration_since(&start);
        assert_eq!(dur.as_millis(), 1500);
    }

    #[test]
    fn test_perf_counter() {
        let mut counter = PerfCounter::new("test", CounterType::Bytes);
        counter.record(100);
        counter.record(200);
        counter.record(50);
        assert_eq!(counter.min, 50);
        assert_eq!(counter.max, 200);
        assert_eq!(counter.average(), 116); // (100+200+50)/3 = 116
    }

    #[test]
    fn test_memory_usage() {
        let usage = MemoryUsage {
            total_available: 1024 * 1024 * 100, // 100 MB
            used: 1024 * 1024 * 85,             // 85 MB
            free: 1024 * 1024 * 15,             // 15 MB
            peak: 1024 * 1024 * 90,
            allocation_count: 1000,
            free_count: 500,
            fragmentation: 10,
        };
        assert_eq!(usage.usage_percent(), 85);
        assert!(!usage.is_low());
        assert!(!usage.is_critical());
    }

    #[test]
    fn test_health_level() {
        assert!(!HealthLevel::Healthy.needs_attention());
        assert!(HealthLevel::Warning.needs_attention());
        assert!(HealthLevel::Critical.needs_attention());
    }

    #[test]
    fn test_boot_progress() {
        let mut progress = BootProgress::new();
        assert_eq!(progress.percent, 0);
        progress.update(50, BootStage::DriverLoad, "Loading drivers");
        assert_eq!(progress.percent, 50);
        assert!(!progress.is_complete());
    }
}
