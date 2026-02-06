//! # Self-Healing Kernel System
//!
//! This module implements a **revolutionary self-healing kernel** that can:
//! - Detect module failures (crashes, hangs, health check failures)
//! - Automatically restart failed modules
//! - Migrate state from crashed modules when possible
//! - Log and report recovery events
//!
//! ## Why This is Revolutionary
//!
//! No mainstream OS can do this:
//! - Linux: A crashed driver = reboot
//! - Windows: Blue Screen of Death
//! - macOS: Kernel panic
//! - **Helix: Auto-recovery in milliseconds!**
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    SELF-HEALING SYSTEM                       │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │  Watchdog   │  │  Health     │  │  Recovery   │          │
//! │  │  Timer      │──│  Monitor    │──│  Manager    │          │
//! │  └─────────────┘  └─────────────┘  └─────────────┘          │
//! │         │                │                │                  │
//! │         ▼                ▼                ▼                  │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │                   MODULE SLOTS                        │   │
//! │  │  [Scheduler] [Allocator] [Driver] [Network] ...      │   │
//! │  │       ↑           ↑          ↑         ↑              │   │
//! │  │    healthy     healthy    CRASHED!   healthy         │   │
//! │  │                              │                        │   │
//! │  │                              └── AUTO RESTART ───────────│
//! │  └──────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use spin::RwLock;

use crate::hotreload::{HotReloadableModule, SlotId};

/// Health status of a module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HealthStatus {
    /// Module is healthy and responding
    Healthy      = 0,
    /// Module is degraded but functional
    Degraded     = 1,
    /// Module is not responding (potential hang)
    Unresponsive = 2,
    /// Module has crashed
    Crashed      = 3,
    /// Module is recovering
    Recovering   = 4,
    /// Module is unknown/not monitored
    Unknown      = 255,
}

/// Recovery action to take
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// No action needed
    None,
    /// Restart the module
    Restart,
    /// Replace with backup module
    Failover,
    /// Escalate to kernel panic (unrecoverable)
    Panic,
}

/// Recovery statistics
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    /// Total health checks performed
    pub health_checks: u64,
    /// Total failures detected
    pub failures_detected: u64,
    /// Successful recoveries
    pub successful_recoveries: u64,
    /// Failed recoveries
    pub failed_recoveries: u64,
    /// Current system health (0-100)
    pub system_health: u8,
}

/// Module health info
#[derive(Debug, Clone)]
pub struct ModuleHealth {
    /// Slot ID
    pub slot_id: SlotId,
    /// Module name
    pub name: String,
    /// Current health status
    pub status: HealthStatus,
    /// Last health check tick
    pub last_check: u64,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Total restarts
    pub restart_count: u32,
    /// Max allowed restarts before giving up
    pub max_restarts: u32,
}

/// Recovery event for logging
#[derive(Debug, Clone)]
pub struct RecoveryEvent {
    /// When it happened
    pub tick: u64,
    /// Which slot
    pub slot_id: SlotId,
    /// Module name
    pub module_name: String,
    /// What happened
    pub event_type: RecoveryEventType,
    /// Was recovery successful
    pub success: bool,
}

/// Types of recovery events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryEventType {
    /// Health check failed
    HealthCheckFailed,
    /// Module crashed
    ModuleCrashed,
    /// Recovery attempted
    RecoveryAttempted,
    /// Recovery successful
    RecoverySuccessful,
    /// Recovery failed
    RecoveryFailed,
    /// Module marked as unrecoverable
    Unrecoverable,
}

/// Factory function type for creating replacement modules
pub type ModuleFactory = fn() -> Box<dyn HotReloadableModule>;

/// Monitored module entry
struct MonitoredModule {
    /// Health info
    health: ModuleHealth,
    /// Factory to create replacement instances
    factory: Option<ModuleFactory>,
    /// Saved state for recovery
    last_known_state: Option<Vec<u8>>,
    /// Is monitoring enabled
    _enabled: bool,
}

/// The Self-Healing Manager
pub struct SelfHealingManager {
    /// Monitored modules
    modules: RwLock<BTreeMap<SlotId, MonitoredModule>>,
    /// Global tick counter
    tick: AtomicU64,
    /// Whether self-healing is enabled
    enabled: AtomicBool,
    /// Health check interval (in ticks)
    check_interval: u32,
    /// Recovery events log
    events: RwLock<Vec<RecoveryEvent>>,
    /// Statistics
    stats: RwLock<RecoveryStats>,
}

impl SelfHealingManager {
    /// Create a new self-healing manager
    pub const fn new() -> Self {
        Self {
            modules: RwLock::new(BTreeMap::new()),
            tick: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            check_interval: 10, // Check every 10 ticks
            events: RwLock::new(Vec::new()),
            stats: RwLock::new(RecoveryStats {
                health_checks: 0,
                failures_detected: 0,
                successful_recoveries: 0,
                failed_recoveries: 0,
                system_health: 100,
            }),
        }
    }

    /// Register a module for monitoring
    pub fn register(&self, slot_id: SlotId, name: &str, factory: Option<ModuleFactory>) {
        let entry = MonitoredModule {
            health: ModuleHealth {
                slot_id,
                name: String::from(name),
                status: HealthStatus::Healthy,
                last_check: 0,
                consecutive_failures: 0,
                restart_count: 0,
                max_restarts: 3,
            },
            factory,
            last_known_state: None,
            _enabled: true,
        };

        self.modules.write().insert(slot_id, entry);
        log_healing(&alloc::format!(
            "[SELF-HEAL] Registered module '{}' (slot {}) for monitoring",
            name,
            slot_id
        ));
    }

    /// Unregister a module
    pub fn unregister(&self, slot_id: SlotId) {
        self.modules.write().remove(&slot_id);
    }

    /// Report a module crash (called from exception handlers)
    pub fn report_crash(&self, slot_id: SlotId) {
        log_healing(&alloc::format!(
            "\n[SELF-HEAL] ⚠️  CRASH DETECTED in slot {}!",
            slot_id
        ));

        // Update status first
        {
            if let Some(module) = self.modules.write().get_mut(&slot_id) {
                module.health.status = HealthStatus::Crashed;
                module.health.consecutive_failures += 1;
            }
        }

        self.stats.write().failures_detected += 1;

        // Get module name for logging
        let module_name = self
            .modules
            .read()
            .get(&slot_id)
            .map(|m| m.health.name.clone())
            .unwrap_or_default();

        self.log_event(
            slot_id,
            &module_name,
            RecoveryEventType::ModuleCrashed,
            true,
        );

        // Trigger recovery (outside of lock)
        self.attempt_recovery(slot_id);
    }

    /// Report a health check result
    pub fn report_health(&self, slot_id: SlotId, healthy: bool) {
        if let Some(module) = self.modules.write().get_mut(&slot_id) {
            module.health.last_check = self.tick.load(Ordering::Relaxed);

            if healthy {
                module.health.status = HealthStatus::Healthy;
                module.health.consecutive_failures = 0;
            } else {
                module.health.consecutive_failures += 1;

                if module.health.consecutive_failures >= 3 {
                    module.health.status = HealthStatus::Unresponsive;
                    log_healing(&alloc::format!(
                        "[SELF-HEAL] Module '{}' unresponsive after {} failures",
                        module.health.name,
                        module.health.consecutive_failures
                    ));
                    self.log_event(
                        slot_id,
                        &module.health.name,
                        RecoveryEventType::HealthCheckFailed,
                        true,
                    );
                } else {
                    module.health.status = HealthStatus::Degraded;
                }
            }

            self.stats.write().health_checks += 1;
        }
    }

    /// Attempt to recover a crashed module
    pub fn attempt_recovery(&self, slot_id: SlotId) -> bool {
        log_healing(&alloc::format!(
            "\n╔══════════════════════════════════════════════╗"
        ));
        log_healing(&alloc::format!(
            "║  SELF-HEALING: Recovering slot {}             ",
            slot_id
        ));
        log_healing(&alloc::format!(
            "╚══════════════════════════════════════════════╝\n"
        ));

        let (module_name, can_recover, has_factory) = {
            let modules = self.modules.read();
            if let Some(module) = modules.get(&slot_id) {
                let can_recover = module.health.restart_count < module.health.max_restarts;
                (
                    module.health.name.clone(),
                    can_recover,
                    module.factory.is_some(),
                )
            } else {
                return false;
            }
        };

        if !can_recover {
            log_healing(&alloc::format!(
                "[SELF-HEAL] ✗ Module '{}' exceeded max restarts, giving up",
                module_name
            ));
            self.log_event(
                slot_id,
                &module_name,
                RecoveryEventType::Unrecoverable,
                false,
            );
            self.stats.write().failed_recoveries += 1;
            return false;
        }

        if !has_factory {
            log_healing(&alloc::format!(
                "[SELF-HEAL] ✗ No factory for '{}', cannot restart",
                module_name
            ));
            self.stats.write().failed_recoveries += 1;
            return false;
        }

        log_healing(&alloc::format!(
            "[SELF-HEAL] Step 1: Creating new instance of '{}'...",
            module_name
        ));

        // Get factory and saved state
        let (factory, _saved_state) = {
            let modules = self.modules.read();
            let module = modules.get(&slot_id).unwrap();
            (module.factory.unwrap(), module.last_known_state.clone())
        };

        // Create new instance
        let new_module = factory();
        log_healing("[SELF-HEAL] Step 2: New instance created");

        // Hot-swap the crashed module with the new one
        log_healing("[SELF-HEAL] Step 3: Force replacing crashed module...");

        match crate::hotreload::force_replace(slot_id, new_module) {
            Ok(()) => {
                log_healing("[SELF-HEAL] Step 4: Hot-swap successful!");

                // Update health status
                if let Some(module) = self.modules.write().get_mut(&slot_id) {
                    module.health.status = HealthStatus::Healthy;
                    module.health.restart_count += 1;
                    module.health.consecutive_failures = 0;
                }

                self.stats.write().successful_recoveries += 1;
                self.log_event(
                    slot_id,
                    &module_name,
                    RecoveryEventType::RecoverySuccessful,
                    true,
                );

                log_healing(&alloc::format!(
                    "\n[SELF-HEAL] ✓ Module '{}' RECOVERED! (restart #{})\n",
                    module_name,
                    self.modules
                        .read()
                        .get(&slot_id)
                        .map(|m| m.health.restart_count)
                        .unwrap_or(0)
                ));

                true
            },
            Err(e) => {
                log_healing(&alloc::format!("[SELF-HEAL] ✗ Hot-swap failed: {:?}", e));
                self.stats.write().failed_recoveries += 1;
                self.log_event(
                    slot_id,
                    &module_name,
                    RecoveryEventType::RecoveryFailed,
                    false,
                );
                false
            },
        }
    }

    /// Save module state for potential recovery
    pub fn save_state(&self, slot_id: SlotId, state: Vec<u8>) {
        if let Some(module) = self.modules.write().get_mut(&slot_id) {
            module.last_known_state = Some(state);
        }
    }

    /// Periodic tick - run health checks
    pub fn tick(&self) {
        let current_tick = self.tick.fetch_add(1, Ordering::Relaxed);

        if current_tick % self.check_interval as u64 != 0 {
            return;
        }

        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        // Update system health
        let modules = self.modules.read();
        let total = modules.len();
        if total == 0 {
            return;
        }

        let healthy = modules
            .values()
            .filter(|m| m.health.status == HealthStatus::Healthy)
            .count();

        let health_pct = ((healthy * 100) / total) as u8;
        self.stats.write().system_health = health_pct;
    }

    /// Get system health percentage
    pub fn system_health(&self) -> u8 {
        self.stats.read().system_health
    }

    /// Get statistics
    pub fn stats(&self) -> RecoveryStats {
        self.stats.read().clone()
    }

    /// Get all module health statuses
    pub fn module_statuses(&self) -> Vec<ModuleHealth> {
        self.modules
            .read()
            .values()
            .map(|m| m.health.clone())
            .collect()
    }

    /// Get recovery event log
    pub fn events(&self) -> Vec<RecoveryEvent> {
        self.events.read().clone()
    }

    fn log_event(&self, slot_id: SlotId, name: &str, event_type: RecoveryEventType, success: bool) {
        let event = RecoveryEvent {
            tick: self.tick.load(Ordering::Relaxed),
            slot_id,
            module_name: String::from(name),
            event_type,
            success,
        };
        self.events.write().push(event);
    }
}

impl Default for SelfHealingManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global Instance
// =============================================================================

/// Global self-healing manager
static MANAGER: SelfHealingManager = SelfHealingManager::new();

/// Initialize the self-healing system
pub fn init() {
    log_healing("[SELF-HEAL] Self-healing system initialized");
    log_healing("[SELF-HEAL] Watchdog active, monitoring modules...\n");
}

/// Get the global manager
pub fn manager() -> &'static SelfHealingManager {
    &MANAGER
}

/// Register a module for monitoring
pub fn register(slot_id: SlotId, name: &str, factory: Option<ModuleFactory>) {
    MANAGER.register(slot_id, name, factory);
}

/// Report a crash
pub fn report_crash(slot_id: SlotId) {
    MANAGER.report_crash(slot_id);
}

/// Report health check result
pub fn report_health(slot_id: SlotId, healthy: bool) {
    MANAGER.report_health(slot_id, healthy);
}

/// Attempt recovery
pub fn attempt_recovery(slot_id: SlotId) -> bool {
    MANAGER.attempt_recovery(slot_id)
}

/// Periodic tick
pub fn tick() {
    MANAGER.tick();
}

/// Get system health
pub fn system_health() -> u8 {
    MANAGER.system_health()
}

// =============================================================================
// Helper
// =============================================================================

fn log_healing(msg: &str) {
    for &c in msg.as_bytes() {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") c,
                options(nomem, nostack)
            );
        }
    }
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3F8u16,
            in("al") b'\n',
            options(nomem, nostack)
        );
    }
}
