//! # Timer Subsystem
//!
//! Hardware timer management for system time, scheduling, and delays.
//! Supports PIT, HPET, TSC (x86_64), Generic Timer (AArch64), and CLINT (RISC-V).

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// =============================================================================
// TIME TYPES
// =============================================================================

/// Nanoseconds since boot
pub type Nanoseconds = u64;

/// Timestamp in various units
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub nanos: Nanoseconds,
}

impl Timestamp {
    /// Create from nanoseconds
    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create from microseconds
    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros * 1_000,
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

    /// Get nanoseconds
    pub const fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get microseconds
    pub const fn as_micros(&self) -> u64 {
        self.nanos / 1_000
    }

    /// Get milliseconds
    pub const fn as_millis(&self) -> u64 {
        self.nanos / 1_000_000
    }

    /// Get seconds
    pub const fn as_secs(&self) -> u64 {
        self.nanos / 1_000_000_000
    }

    /// Duration since another timestamp
    pub fn duration_since(&self, earlier: Timestamp) -> Duration {
        Duration {
            nanos: self.nanos.saturating_sub(earlier.nanos),
        }
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self { nanos: 0 }
    }
}

/// Duration type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    pub nanos: u64,
}

impl Duration {
    pub const ZERO: Duration = Duration { nanos: 0 };

    pub const fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    pub const fn from_micros(micros: u64) -> Self {
        Self {
            nanos: micros * 1_000,
        }
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self {
            nanos: millis * 1_000_000,
        }
    }

    pub const fn from_secs(secs: u64) -> Self {
        Self {
            nanos: secs * 1_000_000_000,
        }
    }
}

// =============================================================================
// TIMER SOURCES
// =============================================================================

/// Timer source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerSource {
    // x86_64
    Pit,         // Programmable Interval Timer (legacy)
    Hpet,        // High Precision Event Timer
    Tsc,         // Time Stamp Counter
    TscDeadline, // TSC-deadline mode
    Lapic,       // Local APIC timer

    // AArch64
    GenericTimer, // ARM Generic Timer
    CntpTimer,    // Physical timer
    CntvTimer,    // Virtual timer
    CnthpTimer,   // Hypervisor physical timer

    // RISC-V
    Clint, // Core-Local Interruptor
    Sstc,  // Supervisor Timer Compare

    Unknown,
}

impl Default for TimerSource {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Timer callback type
pub type TimerCallback = fn(id: u64, context: u64);

/// Timer entry
pub struct TimerEntry {
    pub id: u64,
    pub deadline: Timestamp,
    pub callback: TimerCallback,
    pub context: u64,
    pub periodic: bool,
    pub period: Duration,
    pub active: bool,
}

// =============================================================================
// TIMER SUBSYSTEM
// =============================================================================

/// Timer subsystem
///
/// Manages hardware timers for scheduling and delays.
pub struct TimerSubsystem {
    info: SubsystemInfo,
    source: TimerSource,
    frequency: u64,       // Timer frequency in Hz
    ticks: AtomicU64,     // Tick counter
    boot_time: Timestamp, // Time at boot
    calibrated: bool,
    timers: Vec<TimerEntry>,
    next_timer_id: u64,

    // x86_64 specific
    #[cfg(target_arch = "x86_64")]
    tsc_frequency: u64,
    #[cfg(target_arch = "x86_64")]
    hpet_base: u64,
    #[cfg(target_arch = "x86_64")]
    hpet_period: u64,

    // AArch64 specific
    #[cfg(target_arch = "aarch64")]
    cntfrq: u64,

    // RISC-V specific
    #[cfg(target_arch = "riscv64")]
    timebase_freq: u64,
}

static TIMER_DEPS: [Dependency; 1] = [Dependency::required("interrupts")];

impl TimerSubsystem {
    /// Create new timer subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("timers", InitPhase::Core)
                .with_priority(900)
                .with_description("Hardware timer management")
                .with_dependencies(&TIMER_DEPS)
                .provides(PhaseCapabilities::TIMERS)
                .essential(),
            source: TimerSource::Unknown,
            frequency: 0,
            ticks: AtomicU64::new(0),
            boot_time: Timestamp::default(),
            calibrated: false,
            timers: Vec::new(),
            next_timer_id: 1,

            #[cfg(target_arch = "x86_64")]
            tsc_frequency: 0,
            #[cfg(target_arch = "x86_64")]
            hpet_base: 0,
            #[cfg(target_arch = "x86_64")]
            hpet_period: 0,

            #[cfg(target_arch = "aarch64")]
            cntfrq: 0,

            #[cfg(target_arch = "riscv64")]
            timebase_freq: 0,
        }
    }

    /// Get current time since boot
    pub fn now(&self) -> Timestamp {
        match self.source {
            #[cfg(target_arch = "x86_64")]
            TimerSource::Tsc | TimerSource::TscDeadline => {
                let tsc = Self::read_tsc();
                let nanos = (tsc as u128 * 1_000_000_000) / (self.tsc_frequency as u128);
                Timestamp::from_nanos(nanos as u64)
            },

            #[cfg(target_arch = "x86_64")]
            TimerSource::Hpet => {
                let counter = self.read_hpet_counter();
                let nanos = (counter as u128 * self.hpet_period as u128) / 1_000_000;
                Timestamp::from_nanos(nanos as u64)
            },

            #[cfg(target_arch = "aarch64")]
            TimerSource::GenericTimer | TimerSource::CntpTimer | TimerSource::CntvTimer => {
                let cnt = Self::read_cntvct();
                let nanos = (cnt as u128 * 1_000_000_000) / (self.cntfrq as u128);
                Timestamp::from_nanos(nanos as u64)
            },

            #[cfg(target_arch = "riscv64")]
            TimerSource::Clint | TimerSource::Sstc => {
                let time = Self::read_time();
                let nanos = (time as u128 * 1_000_000_000) / (self.timebase_freq as u128);
                Timestamp::from_nanos(nanos as u64)
            },

            _ => {
                // Fallback: use tick counter
                let ticks = self.ticks.load(Ordering::Relaxed);
                Timestamp::from_nanos(ticks * 1_000_000) // Assume 1ms ticks
            },
        }
    }

    /// Get timer frequency
    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    /// Get tick count
    pub fn ticks(&self) -> u64 {
        self.ticks.load(Ordering::Relaxed)
    }

    /// Increment tick counter (called from timer interrupt)
    pub fn tick(&self) {
        self.ticks.fetch_add(1, Ordering::Relaxed);
    }

    /// Delay for specified duration
    pub fn delay(&self, duration: Duration) {
        let start = self.now();
        let end = Timestamp::from_nanos(start.nanos + duration.nanos);

        while self.now() < end {
            core::hint::spin_loop();
        }
    }

    /// Schedule a one-shot timer
    pub fn schedule_oneshot(
        &mut self,
        deadline: Timestamp,
        callback: TimerCallback,
        context: u64,
    ) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        self.timers.push(TimerEntry {
            id,
            deadline,
            callback,
            context,
            periodic: false,
            period: Duration::ZERO,
            active: true,
        });

        // Sort by deadline
        self.timers.sort_by_key(|t| t.deadline);

        id
    }

    /// Schedule a periodic timer
    pub fn schedule_periodic(
        &mut self,
        period: Duration,
        callback: TimerCallback,
        context: u64,
    ) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;

        let now = self.now();
        self.timers.push(TimerEntry {
            id,
            deadline: Timestamp::from_nanos(now.nanos + period.nanos),
            callback,
            context,
            periodic: true,
            period,
            active: true,
        });

        self.timers.sort_by_key(|t| t.deadline);

        id
    }

    /// Cancel a timer
    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(pos) = self.timers.iter().position(|t| t.id == id) {
            self.timers.remove(pos);
            true
        } else {
            false
        }
    }

    // =========================================================================
    // x86_64 IMPLEMENTATION
    // =========================================================================

    #[cfg(target_arch = "x86_64")]
    fn read_tsc() -> u64 {
        unsafe { core::arch::x86_64::_rdtsc() }
    }

    #[cfg(target_arch = "x86_64")]
    fn read_hpet_counter(&self) -> u64 {
        if self.hpet_base == 0 {
            return 0;
        }

        let counter_ptr = (self.hpet_base + 0xF0) as *const u64;
        unsafe { core::ptr::read_volatile(counter_ptr) }
    }

    #[cfg(target_arch = "x86_64")]
    fn init_x86(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // Try to detect best timer source

        // Check for invariant TSC
        let ext_max = unsafe { core::arch::x86_64::__cpuid(0x80000000) }.eax;
        let has_invariant_tsc = if ext_max >= 0x80000007 {
            let ext7 = unsafe { core::arch::x86_64::__cpuid(0x80000007) };
            (ext7.edx & (1 << 8)) != 0
        } else {
            false
        };

        // Check for TSC deadline
        let leaf1 = unsafe { core::arch::x86_64::__cpuid(1) };
        let has_tsc_deadline = (leaf1.ecx & (1 << 24)) != 0;

        // Calibrate TSC using PIT
        self.calibrate_tsc_with_pit(ctx)?;

        if has_invariant_tsc && has_tsc_deadline {
            self.source = TimerSource::TscDeadline;
            ctx.info("Using TSC-deadline timer");
        } else if has_invariant_tsc {
            self.source = TimerSource::Tsc;
            ctx.info("Using TSC timer");
        } else {
            // Fall back to LAPIC timer
            self.source = TimerSource::Lapic;
            self.init_lapic_timer(ctx)?;
        }

        self.frequency = self.tsc_frequency;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn calibrate_tsc_with_pit(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        const PIT_FREQUENCY: u64 = 1193182;
        const CALIBRATION_MS: u64 = 10;

        // Program PIT channel 2 for calibration
        const PIT_CH2_DATA: u16 = 0x42;
        const PIT_CMD: u16 = 0x43;
        const PIT_CH2_GATE: u16 = 0x61;

        // Calculate countdown value
        let countdown = (PIT_FREQUENCY * CALIBRATION_MS) / 1000;

        unsafe {
            // Gate off, speaker off
            let gate = Self::inb(PIT_CH2_GATE);
            Self::outb(PIT_CH2_GATE, gate & !0x03);

            // Mode 0, binary, channel 2
            Self::outb(PIT_CMD, 0xB0);

            // Load countdown
            Self::outb(PIT_CH2_DATA, (countdown & 0xFF) as u8);
            Self::outb(PIT_CH2_DATA, ((countdown >> 8) & 0xFF) as u8);

            // Start gate
            Self::outb(PIT_CH2_GATE, gate | 0x01);

            // Read TSC start
            let tsc_start = Self::read_tsc();

            // Wait for countdown (poll output bit)
            while (Self::inb(PIT_CH2_GATE) & 0x20) == 0 {
                core::hint::spin_loop();
            }

            // Read TSC end
            let tsc_end = Self::read_tsc();

            // Calculate frequency
            let tsc_diff = tsc_end - tsc_start;
            self.tsc_frequency = (tsc_diff * 1000) / CALIBRATION_MS;

            // Restore gate
            Self::outb(PIT_CH2_GATE, gate);
        }

        ctx.info(alloc::format!(
            "TSC frequency: {} MHz",
            self.tsc_frequency / 1_000_000
        ));

        self.calibrated = true;

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn init_lapic_timer(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.debug("Initializing LAPIC timer");

        // LAPIC timer is configured via memory-mapped registers
        // This is a simplified version

        let lapic_base = 0xFEE0_0000u64;

        // Timer divide configuration (offset 0x3E0)
        let div_ptr = (lapic_base + 0x3E0) as *mut u32;
        unsafe {
            // Divide by 16
            core::ptr::write_volatile(div_ptr, 0x03);
        }

        // Timer vector (offset 0x320)
        let lvt_ptr = (lapic_base + 0x320) as *mut u32;
        unsafe {
            // Vector 0x20, periodic mode
            core::ptr::write_volatile(lvt_ptr, 0x20 | (1 << 17));
        }

        // Initial count (offset 0x380)
        let count_ptr = (lapic_base + 0x380) as *mut u32;
        unsafe {
            // 10ms period (approximate)
            core::ptr::write_volatile(count_ptr, 10_000_000);
        }

        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    fn outb(port: u16, value: u8) {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") port,
                in("al") value,
                options(nostack)
            );
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn inb(port: u16) -> u8 {
        let value: u8;
        unsafe {
            core::arch::asm!(
                "in al, dx",
                out("al") value,
                in("dx") port,
                options(nostack)
            );
        }
        value
    }

    // =========================================================================
    // AArch64 IMPLEMENTATION
    // =========================================================================

    #[cfg(target_arch = "aarch64")]
    fn read_cntvct() -> u64 {
        let cnt: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, cntvct_el0",
                out(reg) cnt,
                options(nostack)
            );
        }
        cnt
    }

    #[cfg(target_arch = "aarch64")]
    fn init_arm(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // Read counter frequency
        unsafe {
            core::arch::asm!(
                "mrs {}, cntfrq_el0",
                out(reg) self.cntfrq,
                options(nostack)
            );
        }

        self.frequency = self.cntfrq;
        self.source = TimerSource::GenericTimer;

        ctx.info(alloc::format!(
            "ARM Generic Timer frequency: {} MHz",
            self.cntfrq / 1_000_000
        ));

        // Enable physical timer
        unsafe {
            // Set CNTV_CTL_EL0.ENABLE = 1
            core::arch::asm!(
                "msr cntp_ctl_el0, {}",
                in(reg) 1u64,
                options(nostack)
            );
        }

        self.calibrated = true;

        Ok(())
    }

    // =========================================================================
    // RISC-V IMPLEMENTATION
    // =========================================================================

    #[cfg(target_arch = "riscv64")]
    fn read_time() -> u64 {
        let time: u64;
        unsafe {
            core::arch::asm!(
                "rdtime {}",
                out(reg) time,
                options(nostack)
            );
        }
        time
    }

    #[cfg(target_arch = "riscv64")]
    fn init_riscv(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // Timebase frequency typically from DTB
        // Common value is 10 MHz
        self.timebase_freq = 10_000_000;
        self.frequency = self.timebase_freq;
        self.source = TimerSource::Clint;

        ctx.info(alloc::format!(
            "RISC-V timer frequency: {} MHz",
            self.timebase_freq / 1_000_000
        ));

        // Enable timer interrupt
        unsafe {
            core::arch::asm!(
                "csrs sie, {}",
                in(reg) (1 << 5), // STIE bit
                options(nostack)
            );
        }

        self.calibrated = true;

        Ok(())
    }
}

impl Default for TimerSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for TimerSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing timer subsystem");

        #[cfg(target_arch = "x86_64")]
        self.init_x86(ctx)?;

        #[cfg(target_arch = "aarch64")]
        self.init_arm(ctx)?;

        #[cfg(target_arch = "riscv64")]
        self.init_riscv(ctx)?;

        self.boot_time = self.now();

        ctx.info(alloc::format!(
            "Timer source: {:?}, frequency: {} Hz",
            self.source,
            self.frequency
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Timer subsystem shutdown");

        // Cancel all timers
        self.timers.clear();

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
    fn test_timestamp() {
        let ts = Timestamp::from_millis(1000);
        assert_eq!(ts.as_secs(), 1);
        assert_eq!(ts.as_millis(), 1000);
        assert_eq!(ts.as_micros(), 1_000_000);
        assert_eq!(ts.as_nanos(), 1_000_000_000);
    }

    #[test]
    fn test_duration() {
        let d = Duration::from_secs(1);
        assert_eq!(d.nanos, 1_000_000_000);

        let d = Duration::from_millis(500);
        assert_eq!(d.nanos, 500_000_000);
    }

    #[test]
    fn test_duration_since() {
        let t1 = Timestamp::from_millis(100);
        let t2 = Timestamp::from_millis(200);

        let d = t2.duration_since(t1);
        assert_eq!(d.nanos, 100_000_000);
    }

    #[test]
    fn test_timer_subsystem() {
        let sub = TimerSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Core);
        assert!(sub.info().provides.contains(PhaseCapabilities::TIMERS));
    }
}
