//! # Debug Subsystem
//!
//! Debugging and diagnostic facilities for the kernel.
//! Runtime phase subsystem for development and troubleshooting.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// =============================================================================
// LOG LEVELS
// =============================================================================

/// Log level
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info  = 2,
    Warn  = 3,
    Error = 4,
    Fatal = 5,
    Off   = 6,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl LogLevel {
    /// Get level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
            Self::Off => "OFF",
        }
    }

    /// Get ANSI color code
    pub fn color(&self) -> &'static str {
        match self {
            Self::Trace => "\x1b[90m", // Gray
            Self::Debug => "\x1b[36m", // Cyan
            Self::Info => "\x1b[32m",  // Green
            Self::Warn => "\x1b[33m",  // Yellow
            Self::Error => "\x1b[31m", // Red
            Self::Fatal => "\x1b[35m", // Magenta
            Self::Off => "",
        }
    }
}

// =============================================================================
// LOG ENTRY
// =============================================================================

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub module: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl LogEntry {
    /// Create new log entry
    pub fn new(level: LogLevel, module: &str, message: String) -> Self {
        Self {
            timestamp: 0, // Set by logger
            level,
            module: String::from(module),
            message,
            file: None,
            line: None,
        }
    }

    /// With source location
    pub fn with_location(mut self, file: &str, line: u32) -> Self {
        self.file = Some(String::from(file));
        self.line = Some(line);
        self
    }
}

// =============================================================================
// RING BUFFER
// =============================================================================

/// Ring buffer for log storage
pub struct LogRingBuffer {
    entries: VecDeque<LogEntry>,
    capacity: usize,
    dropped: AtomicU64,
}

impl LogRingBuffer {
    /// Create new ring buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            dropped: AtomicU64::new(0),
        }
    }

    /// Push entry
    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.entries.push_back(entry);
    }

    /// Get entries
    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }

    /// Get dropped count
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get recent entries
    pub fn recent(&self, count: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Filter by level
    pub fn filter_level(&self, min_level: LogLevel) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level >= min_level)
            .collect()
    }

    /// Filter by module
    pub fn filter_module(&self, module: &str) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.module.contains(module))
            .collect()
    }
}

// =============================================================================
// BREAKPOINTS
// =============================================================================

/// Breakpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointType {
    Software,   // INT3 on x86
    Hardware,   // Debug registers
    Watchpoint, // Memory watchpoint
}

/// Breakpoint
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub address: u64,
    pub bp_type: BreakpointType,
    pub enabled: bool,
    pub hit_count: u64,
    pub condition: Option<String>,
}

impl Breakpoint {
    /// Create new breakpoint
    pub fn new(id: u32, address: u64, bp_type: BreakpointType) -> Self {
        Self {
            id,
            address,
            bp_type,
            enabled: true,
            hit_count: 0,
            condition: None,
        }
    }
}

// =============================================================================
// STACK TRACE
// =============================================================================

/// Stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub address: u64,
    pub symbol: Option<String>,
    pub offset: u64,
    pub module: Option<String>,
}

/// Stack trace
#[derive(Debug, Clone, Default)]
pub struct StackTrace {
    pub frames: Vec<StackFrame>,
}

impl StackTrace {
    /// Capture current stack trace
    pub fn capture() -> Self {
        let mut frames = Vec::new();

        // Walk stack using frame pointer
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let mut rbp: u64;
            core::arch::asm!("mov {}, rbp", out(reg) rbp, options(nostack));

            for _ in 0..32 {
                // Max 32 frames
                if rbp == 0 || rbp < 0x1000 {
                    break;
                }

                // Return address is at rbp + 8
                let ret_addr = *((rbp + 8) as *const u64);
                if ret_addr == 0 {
                    break;
                }

                frames.push(StackFrame {
                    address: ret_addr,
                    symbol: None,
                    offset: 0,
                    module: None,
                });

                // Next frame pointer
                rbp = *(rbp as *const u64);
            }
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let mut fp: u64;
            core::arch::asm!("mov {}, x29", out(reg) fp, options(nostack));

            for _ in 0..32 {
                if fp == 0 || fp < 0x1000 {
                    break;
                }

                let lr = *((fp + 8) as *const u64);
                if lr == 0 {
                    break;
                }

                frames.push(StackFrame {
                    address: lr,
                    symbol: None,
                    offset: 0,
                    module: None,
                });

                fp = *(fp as *const u64);
            }
        }

        Self { frames }
    }

    /// Get frame count
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

// =============================================================================
// PERFORMANCE COUNTERS
// =============================================================================

/// Performance counter
pub struct PerfCounter {
    pub name: String,
    pub value: AtomicU64,
    pub min: AtomicU64,
    pub max: AtomicU64,
    pub samples: AtomicU64,
}

impl PerfCounter {
    /// Create new counter
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            value: AtomicU64::new(0),
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
            samples: AtomicU64::new(0),
        }
    }

    /// Increment
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Add value
    pub fn add(&self, val: u64) {
        self.value.fetch_add(val, Ordering::Relaxed);
    }

    /// Record sample (for timing)
    pub fn record(&self, val: u64) {
        self.value.fetch_add(val, Ordering::Relaxed);
        self.samples.fetch_add(1, Ordering::Relaxed);

        // Update min
        let mut current_min = self.min.load(Ordering::Relaxed);
        while val < current_min {
            match self
                .min
                .compare_exchange(current_min, val, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(v) => current_min = v,
            }
        }

        // Update max
        let mut current_max = self.max.load(Ordering::Relaxed);
        while val > current_max {
            match self
                .max
                .compare_exchange(current_max, val, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(v) => current_max = v,
            }
        }
    }

    /// Get average
    pub fn average(&self) -> u64 {
        let samples = self.samples.load(Ordering::Relaxed);
        if samples == 0 {
            0
        } else {
            self.value.load(Ordering::Relaxed) / samples
        }
    }

    /// Get stats
    pub fn stats(&self) -> CounterStats {
        let samples = self.samples.load(Ordering::Relaxed);
        CounterStats {
            value: self.value.load(Ordering::Relaxed),
            min: if samples > 0 {
                self.min.load(Ordering::Relaxed)
            } else {
                0
            },
            max: self.max.load(Ordering::Relaxed),
            samples,
            average: self.average(),
        }
    }
}

/// Counter statistics
#[derive(Debug, Clone)]
pub struct CounterStats {
    pub value: u64,
    pub min: u64,
    pub max: u64,
    pub samples: u64,
    pub average: u64,
}

// =============================================================================
// DEBUG SUBSYSTEM
// =============================================================================

/// Debug Subsystem
///
/// Provides debugging and diagnostic facilities.
pub struct DebugSubsystem {
    info: SubsystemInfo,

    // Logging
    log_buffer: LogRingBuffer,
    log_level: AtomicU32,
    log_to_console: AtomicBool,
    log_to_serial: AtomicBool,

    // Breakpoints
    breakpoints: Vec<Breakpoint>,
    next_bp_id: u32,
    breakpoints_enabled: AtomicBool,

    // Performance counters
    counters: Vec<PerfCounter>,

    // Debug flags
    debug_mode: AtomicBool,
    panic_on_warn: AtomicBool,
    verbose: AtomicBool,
}

static DEBUG_DEPS: [Dependency; 1] = [Dependency::required("heap")];

impl DebugSubsystem {
    /// Create new debug subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("debug", InitPhase::Runtime)
                .with_priority(1000)
                .with_description("Debugging facilities")
                .with_dependencies(&DEBUG_DEPS)
                .provides(PhaseCapabilities::DEBUG),
            log_buffer: LogRingBuffer::new(10000),
            log_level: AtomicU32::new(LogLevel::Info as u32),
            log_to_console: AtomicBool::new(true),
            log_to_serial: AtomicBool::new(true),
            breakpoints: Vec::new(),
            next_bp_id: 1,
            breakpoints_enabled: AtomicBool::new(false),
            counters: Vec::new(),
            debug_mode: AtomicBool::new(false),
            panic_on_warn: AtomicBool::new(false),
            verbose: AtomicBool::new(false),
        }
    }

    /// Log message
    pub fn log(&mut self, entry: LogEntry) {
        let min_level = self.log_level.load(Ordering::Relaxed);
        if (entry.level as u32) < min_level {
            return;
        }

        // Output to console/serial if enabled
        if self.log_to_console.load(Ordering::Relaxed) {
            self.output_log(&entry);
        }

        // Store in buffer
        self.log_buffer.push(entry);
    }

    /// Output log entry
    fn output_log(&self, entry: &LogEntry) {
        #[cfg(target_arch = "x86_64")]
        {
            // Output to serial port (COM1)
            let msg = alloc::format!(
                "[{:>5}] {}: {}\n",
                entry.level.name(),
                entry.module,
                entry.message
            );

            for byte in msg.bytes() {
                Self::serial_write(byte);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn serial_write(byte: u8) {
        const COM1: u16 = 0x3F8;
        unsafe {
            // Wait for transmit buffer empty
            loop {
                let status: u8;
                core::arch::asm!(
                    "in al, dx",
                    out("al") status,
                    in("dx") COM1 + 5,
                    options(nostack)
                );
                if (status & 0x20) != 0 {
                    break;
                }
            }

            // Send byte
            core::arch::asm!(
                "out dx, al",
                in("al") byte,
                in("dx") COM1,
                options(nostack)
            );
        }
    }

    /// Set log level
    pub fn set_log_level(&self, level: LogLevel) {
        self.log_level.store(level as u32, Ordering::Relaxed);
    }

    /// Get log level
    pub fn log_level(&self) -> LogLevel {
        match self.log_level.load(Ordering::Relaxed) {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Fatal,
            _ => LogLevel::Off,
        }
    }

    /// Get log buffer
    pub fn logs(&self) -> &LogRingBuffer {
        &self.log_buffer
    }

    /// Add breakpoint
    pub fn add_breakpoint(&mut self, address: u64, bp_type: BreakpointType) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;

        self.breakpoints.push(Breakpoint::new(id, address, bp_type));
        id
    }

    /// Remove breakpoint
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(pos) = self.breakpoints.iter().position(|b| b.id == id) {
            self.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get breakpoints
    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    /// Create or get counter
    pub fn counter(&mut self, name: &str) -> &PerfCounter {
        if let Some(idx) = self.counters.iter().position(|c| c.name == name) {
            &self.counters[idx]
        } else {
            self.counters.push(PerfCounter::new(name));
            self.counters.last().unwrap()
        }
    }

    /// Get all counters
    pub fn counters(&self) -> &[PerfCounter] {
        &self.counters
    }

    /// Enable debug mode
    pub fn enable_debug_mode(&self) {
        self.debug_mode.store(true, Ordering::SeqCst);
        self.set_log_level(LogLevel::Debug);
    }

    /// Disable debug mode
    pub fn disable_debug_mode(&self) {
        self.debug_mode.store(false, Ordering::SeqCst);
        self.set_log_level(LogLevel::Info);
    }

    /// Is debug mode enabled?
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode.load(Ordering::SeqCst)
    }

    /// Trigger debug break
    pub fn debug_break() {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("int3", options(nostack));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("brk #0", options(nostack));
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!("ebreak", options(nostack));
        }
    }
}

impl Default for DebugSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for DebugSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing debug subsystem");

        // Check for debug mode from config
        if ctx.config().get_bool("debug_mode", false) {
            self.enable_debug_mode();
            ctx.debug("Debug mode enabled");
        }

        // Initialize default counters
        self.counters.push(PerfCounter::new("interrupts"));
        self.counters.push(PerfCounter::new("context_switches"));
        self.counters.push(PerfCounter::new("syscalls"));
        self.counters.push(PerfCounter::new("page_faults"));

        ctx.info(alloc::format!(
            "Debug: log level {:?}, {} counters",
            self.log_level(),
            self.counters.len()
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info(alloc::format!(
            "Debug shutdown: {} log entries, {} dropped",
            self.log_buffer.entries().len(),
            self.log_buffer.dropped()
        ));

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
    fn test_debug_subsystem() {
        let sub = DebugSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Runtime);
        assert!(sub.info().provides.contains(PhaseCapabilities::DEBUG));
    }

    #[test]
    fn test_log_level() {
        assert!(LogLevel::Error > LogLevel::Info);
        assert!(LogLevel::Trace < LogLevel::Debug);
    }

    #[test]
    fn test_ring_buffer() {
        let mut buf = LogRingBuffer::new(3);

        buf.push(LogEntry::new(LogLevel::Info, "test", String::from("1")));
        buf.push(LogEntry::new(LogLevel::Info, "test", String::from("2")));
        buf.push(LogEntry::new(LogLevel::Info, "test", String::from("3")));
        buf.push(LogEntry::new(LogLevel::Info, "test", String::from("4")));

        assert_eq!(buf.entries().len(), 3);
        assert_eq!(buf.dropped(), 1);
    }

    #[test]
    fn test_perf_counter() {
        let counter = PerfCounter::new("test");

        counter.record(100);
        counter.record(200);
        counter.record(300);

        let stats = counter.stats();
        assert_eq!(stats.samples, 3);
        assert_eq!(stats.min, 100);
        assert_eq!(stats.max, 300);
        assert_eq!(stats.average, 200);
    }
}
