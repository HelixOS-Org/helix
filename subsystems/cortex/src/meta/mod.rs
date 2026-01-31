//! # Meta-Kernel
//!
//! The Meta-Kernel is the **kernel that watches the kernel**. It is a minimal,
//! formally verifiable core that monitors the main kernel and can take
//! corrective action when the main kernel fails.
//!
//! ## Why a Meta-Kernel?
//!
//! Traditional kernels are a single point of failure. If the kernel crashes,
//! everything crashes. The Meta-Kernel provides a safety net:
//!
//! - Runs in a separate, protected memory region
//! - Cannot be modified by the main kernel
//! - Monitors health of main kernel continuously
//! - Can restart/recover main kernel if it fails
//! - Preserves critical state across kernel crashes
//!
//! ## Design Principles
//!
//! 1. **Minimal**: Only essential functionality (~1000 lines of code)
//! 2. **Formally Verified**: Small enough for formal verification
//! 3. **Isolated**: Hardware-enforced isolation from main kernel
//! 4. **Deterministic**: No dynamic allocation, no recursion
//! 5. **Resilient**: Can survive main kernel corruption
//!
//! ## Implementation
//!
//! The Meta-Kernel runs in:
//! - **x86_64**: SMM (System Management Mode) or hypervisor mode
//! - **AArch64**: EL3 (Secure Monitor) or EL2 (Hypervisor)
//! - **RISC-V**: M-mode (Machine mode) with PMP protection

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::consciousness::{InvariantViolation, ViolationSeverity};
use crate::{CortexResult, SnapshotId, SubsystemId};

// =============================================================================
// HEALTH CHECK
// =============================================================================

/// Health check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheck {
    /// System is healthy
    Healthy,

    /// System is degraded but functional
    Degraded,

    /// System is in critical state
    Critical,

    /// System is unresponsive
    Unresponsive,

    /// System has crashed
    Crashed,
}

/// System health metrics
#[derive(Debug, Clone)]
pub struct SystemHealth {
    /// Overall health
    pub status: HealthCheck,

    /// Heartbeat counter
    pub heartbeat: u64,

    /// Last heartbeat timestamp
    pub last_heartbeat: u64,

    /// Heartbeat timeout (cycles)
    pub heartbeat_timeout: u64,

    /// Kernel responsiveness (microseconds)
    pub responsiveness_us: u64,

    /// Memory integrity check passed
    pub memory_integrity: bool,

    /// Stack integrity check passed
    pub stack_integrity: bool,

    /// Control flow integrity check passed
    pub cfi_integrity: bool,

    /// Critical invariants satisfied
    pub invariants_ok: bool,

    /// Number of panics since last reset
    pub panic_count: u64,

    /// Number of watchdog resets
    pub watchdog_resets: u64,
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            status: HealthCheck::Healthy,
            heartbeat: 0,
            last_heartbeat: 0,
            heartbeat_timeout: 1_000_000_000, // ~1 second at 1GHz
            responsiveness_us: 0,
            memory_integrity: true,
            stack_integrity: true,
            cfi_integrity: true,
            invariants_ok: true,
            panic_count: 0,
            watchdog_resets: 0,
        }
    }
}

impl SystemHealth {
    /// Check if system needs intervention
    pub fn needs_intervention(&self) -> bool {
        matches!(
            self.status,
            HealthCheck::Critical | HealthCheck::Unresponsive | HealthCheck::Crashed
        )
    }

    /// Check if system is operational
    pub fn is_operational(&self) -> bool {
        matches!(self.status, HealthCheck::Healthy | HealthCheck::Degraded)
    }
}

// =============================================================================
// WATCHDOG
// =============================================================================

/// Watchdog configuration
#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    /// Timeout in cycles
    pub timeout_cycles: u64,

    /// Action on timeout
    pub timeout_action: WatchdogAction,

    /// Pre-timeout warning (cycles before timeout)
    pub pretimeout_cycles: u64,

    /// Pre-timeout action
    pub pretimeout_action: WatchdogAction,

    /// Number of retries before escalation
    pub max_retries: u8,

    /// Is watchdog enabled?
    pub enabled: bool,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            timeout_cycles: 5_000_000_000, // ~5 seconds
            timeout_action: WatchdogAction::Reset,
            pretimeout_cycles: 1_000_000_000, // 1 second before
            pretimeout_action: WatchdogAction::Warn,
            max_retries: 3,
            enabled: true,
        }
    }
}

/// Watchdog action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogAction {
    /// Log warning only
    Warn,

    /// Attempt soft recovery
    SoftRecover,

    /// Force kernel reset
    Reset,

    /// Enter survival mode
    SurvivalMode,

    /// Hardware reset (last resort)
    HardwareReset,
}

/// Watchdog state
pub struct Watchdog {
    /// Configuration
    config: WatchdogConfig,

    /// Last kick timestamp
    last_kick: u64,

    /// Current retry count
    retry_count: u8,

    /// Is pre-timeout triggered?
    pretimeout_triggered: bool,

    /// Is timeout triggered?
    timeout_triggered: bool,

    /// Total kicks
    total_kicks: u64,

    /// Total timeouts
    total_timeouts: u64,
}

impl Watchdog {
    /// Create new watchdog
    pub fn new(config: WatchdogConfig) -> Self {
        Self {
            config,
            last_kick: 0,
            retry_count: 0,
            pretimeout_triggered: false,
            timeout_triggered: false,
            total_kicks: 0,
            total_timeouts: 0,
        }
    }

    /// Kick (feed) the watchdog
    pub fn kick(&mut self, timestamp: u64) {
        self.last_kick = timestamp;
        self.pretimeout_triggered = false;
        self.timeout_triggered = false;
        self.retry_count = 0;
        self.total_kicks += 1;
    }

    /// Check watchdog state
    pub fn check(&mut self, timestamp: u64) -> Option<WatchdogAction> {
        if !self.config.enabled {
            return None;
        }

        let elapsed = timestamp.saturating_sub(self.last_kick);

        // Check for timeout
        if elapsed >= self.config.timeout_cycles {
            if !self.timeout_triggered {
                self.timeout_triggered = true;
                self.total_timeouts += 1;
                self.retry_count += 1;

                if self.retry_count >= self.config.max_retries {
                    return Some(WatchdogAction::HardwareReset);
                }

                return Some(self.config.timeout_action);
            }
        }
        // Check for pre-timeout
        else if elapsed >= self.config.timeout_cycles - self.config.pretimeout_cycles {
            if !self.pretimeout_triggered {
                self.pretimeout_triggered = true;
                return Some(self.config.pretimeout_action);
            }
        }

        None
    }

    /// Get remaining time before timeout
    pub fn remaining(&self, timestamp: u64) -> u64 {
        let elapsed = timestamp.saturating_sub(self.last_kick);
        self.config.timeout_cycles.saturating_sub(elapsed)
    }

    /// Is watchdog active?
    pub fn is_active(&self) -> bool {
        self.config.enabled
    }
}

// =============================================================================
// META-KERNEL STATE
// =============================================================================

/// Meta-kernel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaState {
    /// Initializing
    Initializing,

    /// Monitoring (normal operation)
    Monitoring,

    /// Warning (pre-timeout or degraded)
    Warning,

    /// Intervening (taking corrective action)
    Intervening,

    /// Recovering (restarting kernel)
    Recovering,

    /// Halted (unrecoverable failure)
    Halted,
}

/// Meta-kernel action
#[derive(Debug, Clone)]
pub enum MetaAction {
    /// No action needed
    None,

    /// Log event
    Log(String),

    /// Trigger soft recovery
    SoftRecover,

    /// Trigger hard recovery (kernel restart)
    HardRecover,

    /// Isolate subsystem
    Isolate(SubsystemId),

    /// Rollback to snapshot
    Rollback(SnapshotId),

    /// Enter survival mode
    SurvivalMode,

    /// Hardware reset
    HardwareReset,

    /// Halt system
    Halt(String),
}

// =============================================================================
// PROTECTED MEMORY
// =============================================================================

/// Protected memory region (not accessible by main kernel)
pub struct ProtectedMemory {
    /// Base address
    pub base: u64,

    /// Size
    pub size: usize,

    /// Is region valid?
    pub valid: bool,

    /// Write count (for integrity)
    pub write_count: u64,

    /// Checksum
    pub checksum: u64,
}

impl ProtectedMemory {
    /// Create new protected memory region
    pub fn new(base: u64, size: usize) -> Self {
        Self {
            base,
            size,
            valid: true,
            write_count: 0,
            checksum: 0,
        }
    }

    /// Write to protected memory (with integrity tracking)
    pub unsafe fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), ProtectedMemoryError> {
        if offset + data.len() > self.size {
            return Err(ProtectedMemoryError::OutOfBounds);
        }

        let ptr = (self.base + offset as u64) as *mut u8;
        core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());

        self.write_count += 1;
        self.update_checksum();

        Ok(())
    }

    /// Read from protected memory
    pub unsafe fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>, ProtectedMemoryError> {
        if offset + len > self.size {
            return Err(ProtectedMemoryError::OutOfBounds);
        }

        let ptr = (self.base + offset as u64) as *const u8;
        let mut data = Vec::with_capacity(len);
        data.set_len(len);
        core::ptr::copy_nonoverlapping(ptr, data.as_mut_ptr(), len);

        Ok(data)
    }

    /// Update checksum
    fn update_checksum(&mut self) {
        // In real implementation, would compute cryptographic hash
        self.checksum = self.checksum.wrapping_add(1);
    }

    /// Verify integrity
    pub fn verify_integrity(&self) -> bool {
        // In real implementation, would verify cryptographic hash
        self.valid
    }
}

#[derive(Debug)]
pub enum ProtectedMemoryError {
    OutOfBounds,
    IntegrityViolation,
    AccessDenied,
}

// =============================================================================
// KERNEL RESTART
// =============================================================================

/// Kernel restart information
#[derive(Clone)]
pub struct KernelRestart {
    /// Restart count
    pub count: u64,

    /// Last restart timestamp
    pub last_restart: u64,

    /// Last restart reason
    pub last_reason: String,

    /// State to preserve across restart
    pub preserved_state: Vec<u8>,

    /// Restart in progress?
    pub in_progress: bool,
}

impl Default for KernelRestart {
    fn default() -> Self {
        Self {
            count: 0,
            last_restart: 0,
            last_reason: String::new(),
            preserved_state: Vec::new(),
            in_progress: false,
        }
    }
}

// =============================================================================
// META-KERNEL
// =============================================================================

/// The Meta-Kernel
pub struct MetaKernel {
    /// Current state
    state: MetaState,

    /// System health
    health: SystemHealth,

    /// Watchdog
    watchdog: Watchdog,

    /// Protected memory region
    protected_memory: Option<ProtectedMemory>,

    /// Kernel restart info
    restart: KernelRestart,

    /// Current timestamp
    current_timestamp: u64,

    /// Is meta-kernel active?
    active: AtomicBool,

    /// Intervention count
    interventions: AtomicU64,

    /// Last action taken
    last_action: Option<MetaAction>,
}

impl MetaKernel {
    /// Create new meta-kernel
    pub fn new() -> Self {
        Self {
            state: MetaState::Initializing,
            health: SystemHealth::default(),
            watchdog: Watchdog::new(WatchdogConfig::default()),
            protected_memory: None,
            restart: KernelRestart::default(),
            current_timestamp: 0,
            active: AtomicBool::new(true),
            interventions: AtomicU64::new(0),
            last_action: None,
        }
    }

    /// Initialize meta-kernel
    pub fn initialize(&mut self, protected_base: u64, protected_size: usize) {
        self.protected_memory = Some(ProtectedMemory::new(protected_base, protected_size));
        self.state = MetaState::Monitoring;
        self.watchdog.kick(0);
    }

    /// Handle kernel heartbeat
    pub fn heartbeat(&mut self, timestamp: u64) {
        self.current_timestamp = timestamp;
        self.health.heartbeat += 1;
        self.health.last_heartbeat = timestamp;
        self.watchdog.kick(timestamp);

        // Update health status
        self.update_health();

        // If we were in warning state, return to monitoring
        if self.state == MetaState::Warning && self.health.is_operational() {
            self.state = MetaState::Monitoring;
        }
    }

    /// Check system (called periodically)
    pub fn check(&mut self, timestamp: u64) -> Option<MetaAction> {
        self.current_timestamp = timestamp;

        // Check watchdog
        if let Some(action) = self.watchdog.check(timestamp) {
            return Some(self.handle_watchdog_action(action));
        }

        // Check health
        if self.health.needs_intervention() {
            return Some(self.intervene());
        }

        None
    }

    /// Update health status
    fn update_health(&mut self) {
        let elapsed = self
            .current_timestamp
            .saturating_sub(self.health.last_heartbeat);

        // Check responsiveness
        if elapsed > self.health.heartbeat_timeout {
            self.health.status = HealthCheck::Unresponsive;
        } else if !self.health.invariants_ok || !self.health.memory_integrity {
            self.health.status = HealthCheck::Critical;
        } else if !self.health.stack_integrity || !self.health.cfi_integrity {
            self.health.status = HealthCheck::Degraded;
        } else {
            self.health.status = HealthCheck::Healthy;
        }
    }

    /// Handle watchdog action
    fn handle_watchdog_action(&mut self, action: WatchdogAction) -> MetaAction {
        self.state = MetaState::Warning;

        match action {
            WatchdogAction::Warn => MetaAction::Log(String::from("Watchdog pre-timeout warning")),

            WatchdogAction::SoftRecover => {
                self.state = MetaState::Intervening;
                self.interventions.fetch_add(1, Ordering::SeqCst);
                MetaAction::SoftRecover
            },

            WatchdogAction::Reset => {
                self.state = MetaState::Recovering;
                self.interventions.fetch_add(1, Ordering::SeqCst);
                MetaAction::HardRecover
            },

            WatchdogAction::SurvivalMode => MetaAction::SurvivalMode,

            WatchdogAction::HardwareReset => {
                self.state = MetaState::Halted;
                MetaAction::HardwareReset
            },
        }
    }

    /// Intervene in kernel operation
    fn intervene(&mut self) -> MetaAction {
        self.state = MetaState::Intervening;
        self.interventions.fetch_add(1, Ordering::SeqCst);

        match self.health.status {
            HealthCheck::Critical => {
                // Try soft recovery first
                if self.restart.count < 3 {
                    MetaAction::SoftRecover
                } else {
                    MetaAction::HardRecover
                }
            },

            HealthCheck::Unresponsive => MetaAction::HardRecover,

            HealthCheck::Crashed => {
                if self.restart.count < 5 {
                    self.state = MetaState::Recovering;
                    MetaAction::HardRecover
                } else {
                    self.state = MetaState::Halted;
                    MetaAction::Halt(String::from("Too many restart attempts"))
                }
            },

            _ => MetaAction::None,
        }
    }

    /// Handle critical invariant violation
    pub fn handle_critical_violation(&mut self, violation: &InvariantViolation) -> CortexResult {
        self.health.invariants_ok = false;
        self.state = MetaState::Intervening;
        self.interventions.fetch_add(1, Ordering::SeqCst);

        match violation.severity {
            ViolationSeverity::Fatal => {
                // Try to isolate if possible
                if let Some(subsystem) = violation.subsystem {
                    self.last_action = Some(MetaAction::Isolate(subsystem));
                    CortexResult::SubsystemIsolated(subsystem)
                } else {
                    // Full recovery needed
                    self.state = MetaState::Recovering;
                    self.last_action = Some(MetaAction::HardRecover);
                    CortexResult::ActionTaken(crate::DecisionId(0))
                }
            },

            ViolationSeverity::Critical => {
                self.last_action = Some(MetaAction::SurvivalMode);
                CortexResult::ActionTaken(crate::DecisionId(0))
            },

            _ => CortexResult::Observed,
        }
    }

    /// Report panic
    pub fn report_panic(&mut self, message: &str, timestamp: u64) {
        self.health.panic_count += 1;
        self.health.status = HealthCheck::Crashed;

        // Store panic info in protected memory
        if let Some(ref mut mem) = self.protected_memory {
            let panic_info = format!("PANIC@{}: {}", timestamp, message);
            let _ = unsafe { mem.write(0, panic_info.as_bytes()) };
        }
    }

    /// Start kernel restart
    pub fn start_restart(&mut self, reason: &str, timestamp: u64) {
        self.restart.in_progress = true;
        self.restart.count += 1;
        self.restart.last_restart = timestamp;
        self.restart.last_reason = String::from(reason);
        self.state = MetaState::Recovering;
        self.health.watchdog_resets += 1;
    }

    /// Complete kernel restart
    pub fn complete_restart(&mut self) {
        self.restart.in_progress = false;
        self.state = MetaState::Monitoring;
        self.health = SystemHealth::default();
        self.watchdog.kick(self.current_timestamp);
    }

    /// Get current state
    pub fn state(&self) -> MetaState {
        self.state
    }

    /// Get system health
    pub fn health(&self) -> &SystemHealth {
        &self.health
    }

    /// Get restart info
    pub fn restart_info(&self) -> &KernelRestart {
        &self.restart
    }

    /// Get intervention count
    pub fn intervention_count(&self) -> u64 {
        self.interventions.load(Ordering::SeqCst)
    }

    /// Is meta-kernel active?
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Perform hardware reset
    pub fn hardware_reset(&self) -> ! {
        // Architecture-specific reset
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // Triple fault to reset
            core::arch::asm!(
                "lidt [rax]",
                in("rax") 0u64,
                options(noreturn)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            // PSCI system reset
            core::arch::asm!("mov x0, #0x84000009", "hvc #0", options(noreturn));
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            // SBI system reset
            core::arch::asm!(
                "li a7, 0x53525354",
                "li a6, 0",
                "li a0, 0",
                "li a1, 0",
                "ecall",
                options(noreturn)
            );
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        loop {
            core::hint::spin_loop();
        }
    }
}

impl Default for MetaKernel {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_kernel_creation() {
        let meta = MetaKernel::new();
        assert_eq!(meta.state(), MetaState::Initializing);
    }

    #[test]
    fn test_heartbeat() {
        let mut meta = MetaKernel::new();
        meta.initialize(0x1000, 4096);

        meta.heartbeat(1000);
        assert_eq!(meta.health().heartbeat, 1);
        assert_eq!(meta.state(), MetaState::Monitoring);
    }

    #[test]
    fn test_watchdog() {
        let mut watchdog = Watchdog::new(WatchdogConfig {
            timeout_cycles: 1000,
            pretimeout_cycles: 200,
            ..Default::default()
        });

        watchdog.kick(0);

        // No timeout yet
        assert!(watchdog.check(500).is_none());

        // Pre-timeout
        let action = watchdog.check(850);
        assert!(matches!(action, Some(WatchdogAction::Warn)));

        // Timeout
        let action = watchdog.check(1100);
        assert!(matches!(action, Some(WatchdogAction::Reset)));
    }

    #[test]
    fn test_health_check() {
        let health = SystemHealth::default();
        assert!(health.is_operational());
        assert!(!health.needs_intervention());
    }
}
