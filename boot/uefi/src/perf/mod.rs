//! Performance Monitoring and Benchmarking
//!
//! This module provides performance monitoring, timing utilities,
//! and benchmarking capabilities for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - High-resolution timing
//! - Phase timing statistics
//! - Memory performance monitoring
//! - I/O throughput measurement
//! - Boot time analysis
//! - Performance reporting

use core::fmt;

// =============================================================================
// TIMING
// =============================================================================

/// Time source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeSource {
    /// Unknown/unavailable
    #[default]
    Unknown,
    /// TSC (Time Stamp Counter)
    Tsc,
    /// UEFI boot services timer
    UefiTimer,
    /// ACPI PM timer
    AcpiPmTimer,
    /// HPET
    Hpet,
    /// Generic timer (ARM)
    GenericTimer,
}

impl fmt::Display for TimeSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown"),
            Self::Tsc => write!(f, "TSC"),
            Self::UefiTimer => write!(f, "UEFI Timer"),
            Self::AcpiPmTimer => write!(f, "ACPI PM Timer"),
            Self::Hpet => write!(f, "HPET"),
            Self::GenericTimer => write!(f, "Generic Timer"),
        }
    }
}

/// Timer configuration
#[derive(Debug, Clone, Copy, Default)]
pub struct TimerConfig {
    /// Time source
    pub source: TimeSource,
    /// Timer frequency (Hz)
    pub frequency: u64,
    /// Is invariant (constant frequency)
    pub invariant: bool,
    /// Resolution (nanoseconds)
    pub resolution_ns: u32,
    /// Maximum value before wrap
    pub max_value: u64,
}

/// Timestamp
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    /// Raw counter value
    pub raw: u64,
}

impl Timestamp {
    /// Create from raw value
    pub const fn from_raw(raw: u64) -> Self {
        Self { raw }
    }

    /// Create zero timestamp
    pub const fn zero() -> Self {
        Self { raw: 0 }
    }

    /// Get elapsed since start (raw counts)
    pub const fn elapsed_raw(&self, now: Self) -> u64 {
        now.raw.saturating_sub(self.raw)
    }

    /// Get elapsed in nanoseconds
    pub fn elapsed_ns(&self, now: Self, frequency: u64) -> u64 {
        if frequency == 0 {
            return 0;
        }
        let elapsed = self.elapsed_raw(now);
        // Avoid overflow: (elapsed * 1_000_000_000) / frequency
        // Use 128-bit arithmetic simulation
        let ns_per_count = 1_000_000_000u64 / frequency;
        let remainder = 1_000_000_000u64 % frequency;
        elapsed * ns_per_count + (elapsed * remainder) / frequency
    }

    /// Get elapsed in microseconds
    pub fn elapsed_us(&self, now: Self, frequency: u64) -> u64 {
        if frequency == 0 {
            return 0;
        }
        let elapsed = self.elapsed_raw(now);
        (elapsed * 1_000_000) / frequency
    }

    /// Get elapsed in milliseconds
    pub fn elapsed_ms(&self, now: Self, frequency: u64) -> u64 {
        if frequency == 0 {
            return 0;
        }
        let elapsed = self.elapsed_raw(now);
        (elapsed * 1000) / frequency
    }
}

/// Duration measurement
#[derive(Debug, Clone, Copy, Default)]
pub struct Duration {
    /// Duration in nanoseconds
    pub nanos: u64,
}

impl Duration {
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

    /// Get as nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get as microseconds
    pub const fn as_micros(&self) -> u64 {
        self.nanos / 1000
    }

    /// Get as milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Get as seconds
    pub const fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Get subsec nanoseconds
    pub const fn subsec_nanos(&self) -> u32 {
        (self.nanos % 1_000_000_000) as u32
    }

    /// Add duration
    #[must_use]
    pub const fn add(&self, other: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_add(other.nanos),
        }
    }

    /// Subtract duration
    #[must_use]
    pub const fn sub(&self, other: Self) -> Self {
        Self {
            nanos: self.nanos.saturating_sub(other.nanos),
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nanos = self.nanos;
        if nanos < 1000 {
            write!(f, "{nanos}ns")
        } else if nanos < 1_000_000 {
            write!(f, "{}.{:02}µs", nanos / 1000, (nanos % 1000) / 10)
        } else if nanos < 1_000_000_000 {
            write!(
                f,
                "{}.{:02}ms",
                nanos / 1_000_000,
                (nanos % 1_000_000) / 10_000
            )
        } else {
            write!(
                f,
                "{}.{:03}s",
                nanos / 1_000_000_000,
                (nanos % 1_000_000_000) / 1_000_000
            )
        }
    }
}

// =============================================================================
// STOPWATCH
// =============================================================================

/// Stopwatch for measuring elapsed time
#[derive(Debug, Clone, Copy, Default)]
pub struct Stopwatch {
    /// Start timestamp
    start: Timestamp,
    /// Stop timestamp (if stopped)
    stop: Timestamp,
    /// Timer frequency
    frequency: u64,
    /// Is running
    running: bool,
    /// Accumulated time (for pause/resume)
    accumulated_ns: u64,
}

impl Stopwatch {
    /// Create new stopwatch
    pub const fn new(frequency: u64) -> Self {
        Self {
            start: Timestamp::zero(),
            stop: Timestamp::zero(),
            frequency,
            running: false,
            accumulated_ns: 0,
        }
    }

    /// Start the stopwatch
    pub fn start(&mut self, now: Timestamp) {
        self.start = now;
        self.running = true;
    }

    /// Stop the stopwatch
    pub fn stop(&mut self, now: Timestamp) {
        if self.running {
            self.stop = now;
            self.accumulated_ns += self.start.elapsed_ns(now, self.frequency);
            self.running = false;
        }
    }

    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.start = Timestamp::zero();
        self.stop = Timestamp::zero();
        self.accumulated_ns = 0;
        self.running = false;
    }

    /// Get elapsed duration
    pub fn elapsed(&self, now: Timestamp) -> Duration {
        if self.running {
            Duration::from_nanos(self.accumulated_ns + self.start.elapsed_ns(now, self.frequency))
        } else {
            Duration::from_nanos(self.accumulated_ns)
        }
    }

    /// Check if running
    pub const fn is_running(&self) -> bool {
        self.running
    }
}

// =============================================================================
// STATISTICS
// =============================================================================

/// Running statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct Statistics {
    /// Sample count
    pub count: u64,
    /// Sum
    pub sum: u64,
    /// Minimum value
    pub min: u64,
    /// Maximum value
    pub max: u64,
    /// Sum of squares (for variance)
    sum_sq: u128,
}

impl Statistics {
    /// Create new statistics
    pub const fn new() -> Self {
        Self {
            count: 0,
            sum: 0,
            min: u64::MAX,
            max: 0,
            sum_sq: 0,
        }
    }

    /// Add a sample
    pub fn add(&mut self, value: u64) {
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.sum_sq += u128::from(value) * u128::from(value);
    }

    /// Get average
    pub const fn average(&self) -> u64 {
        if self.count == 0 {
            0
        } else {
            self.sum / self.count
        }
    }

    /// Get variance
    pub fn variance(&self) -> u64 {
        if self.count < 2 {
            return 0;
        }
        let mean = u128::from(self.average());
        let mean_sq = mean * mean;
        let avg_sq = self.sum_sq / u128::from(self.count);
        if avg_sq > mean_sq {
            u64::try_from(avg_sq - mean_sq).unwrap_or(u64::MAX)
        } else {
            0
        }
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> u64 {
        // Integer square root of variance
        let var = self.variance();
        if var == 0 {
            return 0;
        }

        // Newton's method for integer square root
        let mut x = var;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + var / x) / 2;
        }
        x
    }

    /// Merge with another statistics
    pub fn merge(&mut self, other: &Self) {
        if other.count == 0 {
            return;
        }
        self.count += other.count;
        self.sum += other.sum;
        if other.min < self.min {
            self.min = other.min;
        }
        if other.max > self.max {
            self.max = other.max;
        }
        self.sum_sq += other.sum_sq;
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// =============================================================================
// PHASE TIMING
// =============================================================================

/// Boot phase ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PhaseId {
    #[default]
    Unknown            = 0,
    FirmwareEntry      = 1,
    EarlyInit          = 2,
    ConsoleInit        = 3,
    MemoryInit         = 4,
    ConfigLoad         = 5,
    DeviceDiscovery    = 6,
    EntryDetection     = 7,
    SecurityValidation = 8,
    MenuDisplay        = 9,
    UserSelection      = 10,
    EntryPreparation   = 11,
    KernelLoad         = 12,
    InitrdLoad         = 13,
    PreBootHooks       = 14,
    ExitBootServices   = 15,
    HandoffPrep        = 16,
    KernelHandoff      = 17,
}

/// Phase timing entry
#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseEntry {
    /// Phase ID
    pub phase: PhaseId,
    /// Start time (nanoseconds since boot)
    pub start_ns: u64,
    /// End time (nanoseconds since boot)
    pub end_ns: u64,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// Sub-phase count
    pub sub_phases: u8,
    /// Error occurred
    pub had_error: bool,
}

impl PhaseEntry {
    /// Check if complete
    pub const fn is_complete(&self) -> bool {
        self.end_ns > 0
    }

    /// Get duration
    pub const fn duration(&self) -> Duration {
        Duration::from_nanos(self.duration_ns)
    }
}

/// Maximum phases to track
pub const MAX_PHASES: usize = 32;

/// Phase timing tracker
#[derive(Debug)]
pub struct PhaseTimer {
    /// Timer config
    pub config: TimerConfig,
    /// Boot start timestamp
    boot_start: Timestamp,
    /// Phase entries
    phases: [PhaseEntry; MAX_PHASES],
    /// Phase count
    phase_count: usize,
    /// Current phase index
    current_phase: usize,
    /// Total boot time (nanoseconds)
    total_ns: u64,
}

impl Default for PhaseTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseTimer {
    /// Create new phase timer
    pub const fn new() -> Self {
        Self {
            config: TimerConfig {
                source: TimeSource::Unknown,
                frequency: 0,
                invariant: false,
                resolution_ns: 0,
                max_value: 0,
            },
            boot_start: Timestamp::zero(),
            phases: [PhaseEntry {
                phase: PhaseId::Unknown,
                start_ns: 0,
                end_ns: 0,
                duration_ns: 0,
                sub_phases: 0,
                had_error: false,
            }; MAX_PHASES],
            phase_count: 0,
            current_phase: 0,
            total_ns: 0,
        }
    }

    /// Initialize with config
    pub fn init(&mut self, config: TimerConfig, now: Timestamp) {
        self.config = config;
        self.boot_start = now;
    }

    /// Start a phase
    pub fn start_phase(&mut self, phase: PhaseId, now: Timestamp) {
        // End current phase if any
        if self.phase_count > 0 && !self.phases[self.current_phase].is_complete() {
            self.end_phase(now);
        }

        if self.phase_count >= MAX_PHASES {
            return;
        }

        let ns = self.boot_start.elapsed_ns(now, self.config.frequency);

        self.phases[self.phase_count] = PhaseEntry {
            phase,
            start_ns: ns,
            end_ns: 0,
            duration_ns: 0,
            sub_phases: 0,
            had_error: false,
        };
        self.current_phase = self.phase_count;
        self.phase_count += 1;
    }

    /// End current phase
    pub fn end_phase(&mut self, now: Timestamp) {
        if self.phase_count == 0 {
            return;
        }

        let ns = self.boot_start.elapsed_ns(now, self.config.frequency);
        let entry = &mut self.phases[self.current_phase];
        entry.end_ns = ns;
        entry.duration_ns = ns.saturating_sub(entry.start_ns);
        self.total_ns = ns;
    }

    /// Mark current phase as having error
    pub fn mark_error(&mut self) {
        if self.phase_count > 0 {
            self.phases[self.current_phase].had_error = true;
        }
    }

    /// Get phase entry
    pub fn get_phase(&self, phase: PhaseId) -> Option<&PhaseEntry> {
        for i in 0..self.phase_count {
            if self.phases[i].phase == phase {
                return Some(&self.phases[i]);
            }
        }
        None
    }

    /// Get total boot time
    pub const fn total_time(&self) -> Duration {
        Duration::from_nanos(self.total_ns)
    }

    /// Get phase count
    pub const fn len(&self) -> usize {
        self.phase_count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.phase_count == 0
    }

    /// Get slowest phase
    pub fn slowest_phase(&self) -> Option<&PhaseEntry> {
        if self.phase_count == 0 {
            return None;
        }

        let mut slowest_idx = 0;
        let mut max_duration = 0;

        for i in 0..self.phase_count {
            if self.phases[i].duration_ns > max_duration {
                max_duration = self.phases[i].duration_ns;
                slowest_idx = i;
            }
        }

        Some(&self.phases[slowest_idx])
    }
}

// =============================================================================
// MEMORY PERFORMANCE
// =============================================================================

/// Memory operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryOpType {
    #[default]
    Allocate,
    Free,
    MapPages,
    UnmapPages,
    Copy,
    Set,
}

/// Memory performance entry
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryPerfEntry {
    /// Operation type
    pub op_type: MemoryOpType,
    /// Size (bytes)
    pub size: u64,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// Throughput (bytes/sec)
    pub throughput: u64,
}

/// Maximum memory perf entries
pub const MAX_MEM_ENTRIES: usize = 64;

/// Memory performance tracker
#[derive(Debug)]
pub struct MemoryPerf {
    /// Entries
    entries: [MemoryPerfEntry; MAX_MEM_ENTRIES],
    /// Entry count
    count: usize,
    /// Allocation stats
    pub alloc_stats: Statistics,
    /// Free stats
    pub free_stats: Statistics,
    /// Copy stats
    pub copy_stats: Statistics,
    /// Total allocated
    pub total_allocated: u64,
    /// Total freed
    pub total_freed: u64,
    /// Peak usage
    pub peak_usage: u64,
    /// Current usage
    pub current_usage: u64,
}

impl Default for MemoryPerf {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryPerf {
    /// Create new tracker
    pub const fn new() -> Self {
        Self {
            entries: [MemoryPerfEntry {
                op_type: MemoryOpType::Allocate,
                size: 0,
                duration_ns: 0,
                throughput: 0,
            }; MAX_MEM_ENTRIES],
            count: 0,
            alloc_stats: Statistics::new(),
            free_stats: Statistics::new(),
            copy_stats: Statistics::new(),
            total_allocated: 0,
            total_freed: 0,
            peak_usage: 0,
            current_usage: 0,
        }
    }

    /// Record allocation
    pub fn record_alloc(&mut self, size: u64, duration_ns: u64) {
        self.alloc_stats.add(duration_ns);
        self.total_allocated += size;
        self.current_usage += size;
        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }
        self.add_entry(MemoryOpType::Allocate, size, duration_ns);
    }

    /// Record free
    pub fn record_free(&mut self, size: u64, duration_ns: u64) {
        self.free_stats.add(duration_ns);
        self.total_freed += size;
        self.current_usage = self.current_usage.saturating_sub(size);
        self.add_entry(MemoryOpType::Free, size, duration_ns);
    }

    /// Record copy
    pub fn record_copy(&mut self, size: u64, duration_ns: u64) {
        self.copy_stats.add(duration_ns);
        self.add_entry(MemoryOpType::Copy, size, duration_ns);
    }

    /// Add entry
    fn add_entry(&mut self, op_type: MemoryOpType, size: u64, duration_ns: u64) {
        if self.count >= MAX_MEM_ENTRIES {
            return;
        }

        let throughput = if duration_ns > 0 {
            (size * 1_000_000_000) / duration_ns
        } else {
            0
        };

        self.entries[self.count] = MemoryPerfEntry {
            op_type,
            size,
            duration_ns,
            throughput,
        };
        self.count += 1;
    }

    /// Get entry count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// I/O PERFORMANCE
// =============================================================================

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoOpType {
    #[default]
    Read,
    Write,
    Seek,
    Sync,
}

/// I/O performance entry
#[derive(Debug, Clone, Copy, Default)]
pub struct IoPerfEntry {
    /// Operation type
    pub op_type: IoOpType,
    /// Device index
    pub device: u8,
    /// Size (bytes)
    pub size: u64,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// Throughput (bytes/sec)
    pub throughput: u64,
    /// Offset
    pub offset: u64,
}

/// Maximum I/O perf entries
pub const MAX_IO_ENTRIES: usize = 64;

/// I/O performance tracker
#[derive(Debug)]
pub struct IoPerf {
    /// Entries
    entries: [IoPerfEntry; MAX_IO_ENTRIES],
    /// Entry count
    count: usize,
    /// Read stats
    pub read_stats: Statistics,
    /// Write stats
    pub write_stats: Statistics,
    /// Total bytes read
    pub total_read: u64,
    /// Total bytes written
    pub total_written: u64,
    /// Total read time (ns)
    pub total_read_time_ns: u64,
    /// Total write time (ns)
    pub total_write_time_ns: u64,
}

impl Default for IoPerf {
    fn default() -> Self {
        Self::new()
    }
}

impl IoPerf {
    /// Create new tracker
    pub const fn new() -> Self {
        Self {
            entries: [IoPerfEntry {
                op_type: IoOpType::Read,
                device: 0,
                size: 0,
                duration_ns: 0,
                throughput: 0,
                offset: 0,
            }; MAX_IO_ENTRIES],
            count: 0,
            read_stats: Statistics::new(),
            write_stats: Statistics::new(),
            total_read: 0,
            total_written: 0,
            total_read_time_ns: 0,
            total_write_time_ns: 0,
        }
    }

    /// Record read
    pub fn record_read(&mut self, device: u8, offset: u64, size: u64, duration_ns: u64) {
        self.read_stats.add(duration_ns);
        self.total_read += size;
        self.total_read_time_ns += duration_ns;
        self.add_entry(IoOpType::Read, device, offset, size, duration_ns);
    }

    /// Record write
    pub fn record_write(&mut self, device: u8, offset: u64, size: u64, duration_ns: u64) {
        self.write_stats.add(duration_ns);
        self.total_written += size;
        self.total_write_time_ns += duration_ns;
        self.add_entry(IoOpType::Write, device, offset, size, duration_ns);
    }

    /// Add entry
    fn add_entry(
        &mut self,
        op_type: IoOpType,
        device: u8,
        offset: u64,
        size: u64,
        duration_ns: u64,
    ) {
        if self.count >= MAX_IO_ENTRIES {
            return;
        }

        let throughput = if duration_ns > 0 {
            (size * 1_000_000_000) / duration_ns
        } else {
            0
        };

        self.entries[self.count] = IoPerfEntry {
            op_type,
            device,
            size,
            duration_ns,
            throughput,
            offset,
        };
        self.count += 1;
    }

    /// Get average read throughput (bytes/sec)
    pub const fn avg_read_throughput(&self) -> u64 {
        if self.total_read_time_ns == 0 {
            return 0;
        }
        (self.total_read * 1_000_000_000) / self.total_read_time_ns
    }

    /// Get average write throughput (bytes/sec)
    pub const fn avg_write_throughput(&self) -> u64 {
        if self.total_write_time_ns == 0 {
            return 0;
        }
        (self.total_written * 1_000_000_000) / self.total_write_time_ns
    }

    /// Get entry count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// PERFORMANCE REPORT
// =============================================================================

/// Performance report
#[derive(Debug)]
pub struct PerfReport {
    /// Phase timing
    pub phases: PhaseTimer,
    /// Memory performance
    pub memory: MemoryPerf,
    /// I/O performance
    pub io: IoPerf,
    /// Report generation timestamp
    pub generated_ns: u64,
}

impl Default for PerfReport {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfReport {
    /// Create new report
    pub const fn new() -> Self {
        Self {
            phases: PhaseTimer::new(),
            memory: MemoryPerf::new(),
            io: IoPerf::new(),
            generated_ns: 0,
        }
    }

    /// Get total boot time
    pub fn boot_time(&self) -> Duration {
        self.phases.total_time()
    }

    /// Get summary
    pub fn summary(&self) -> PerfSummary {
        PerfSummary {
            total_boot_time_ns: self.phases.total_ns,
            phase_count: u8::try_from(self.phases.len()).unwrap_or(u8::MAX),
            slowest_phase: self
                .phases
                .slowest_phase()
                .map_or(PhaseId::Unknown, |p| p.phase),
            slowest_phase_time_ns: self
                .phases
                .slowest_phase()
                .map_or(0, |p| p.duration_ns),
            total_mem_allocated: self.memory.total_allocated,
            peak_mem_usage: self.memory.peak_usage,
            total_io_read: self.io.total_read,
            total_io_write: self.io.total_written,
            avg_read_throughput: self.io.avg_read_throughput(),
            avg_write_throughput: self.io.avg_write_throughput(),
        }
    }
}

/// Performance summary
#[derive(Debug, Clone, Copy, Default)]
pub struct PerfSummary {
    /// Total boot time (nanoseconds)
    pub total_boot_time_ns: u64,
    /// Number of phases
    pub phase_count: u8,
    /// Slowest phase
    pub slowest_phase: PhaseId,
    /// Slowest phase time (nanoseconds)
    pub slowest_phase_time_ns: u64,
    /// Total memory allocated
    pub total_mem_allocated: u64,
    /// Peak memory usage
    pub peak_mem_usage: u64,
    /// Total I/O read
    pub total_io_read: u64,
    /// Total I/O write
    pub total_io_write: u64,
    /// Average read throughput (bytes/sec)
    pub avg_read_throughput: u64,
    /// Average write throughput (bytes/sec)
    pub avg_write_throughput: u64,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_duration() {
        let d = Duration::from_millis(1500);
        assert_eq!(d.as_secs(), 1);
        assert_eq!(d.as_millis(), 1500);
        assert_eq!(d.as_micros(), 1_500_000);
    }

    #[test]
    fn test_timestamp() {
        let start = Timestamp::from_raw(0);
        let end = Timestamp::from_raw(1_000_000);
        let freq = 1_000_000_000; // 1 GHz

        assert_eq!(start.elapsed_us(end, freq), 1);
    }

    #[test]
    fn test_statistics() {
        let mut stats = Statistics::new();
        stats.add(10);
        stats.add(20);
        stats.add(30);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.average(), 20);
        assert_eq!(stats.min, 10);
        assert_eq!(stats.max, 30);
    }

    #[test]
    fn test_stopwatch() {
        let freq = 1_000_000; // 1 MHz
        let mut sw = Stopwatch::new(freq);

        sw.start(Timestamp::from_raw(0));
        assert!(sw.is_running());

        sw.stop(Timestamp::from_raw(1000));
        assert!(!sw.is_running());
    }

    #[test]
    fn test_phase_timer() {
        let mut timer = PhaseTimer::new();
        let config = TimerConfig {
            frequency: 1_000_000_000,
            ..Default::default()
        };

        timer.init(config, Timestamp::from_raw(0));
        timer.start_phase(PhaseId::EarlyInit, Timestamp::from_raw(0));
        timer.end_phase(Timestamp::from_raw(1_000_000));

        assert_eq!(timer.len(), 1);
        assert!(timer.get_phase(PhaseId::EarlyInit).is_some());
    }

    #[test]
    fn test_memory_perf() {
        let mut perf = MemoryPerf::new();

        perf.record_alloc(4096, 1000);
        perf.record_alloc(8192, 2000);
        perf.record_free(4096, 500);

        assert_eq!(perf.total_allocated, 4096 + 8192);
        assert_eq!(perf.total_freed, 4096);
        assert_eq!(perf.current_usage, 8192);
    }
}
