//! # Holistic Watchdog Manager
//!
//! System-wide watchdog and hang detection:
//! - Soft lockup detection (CPU hogging)
//! - Hard lockup detection (IRQ context)
//! - Hung task detection (D-state processes)
//! - RCU stall detection
//! - Workqueue stall detection
//! - Automated recovery actions

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lockup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockupType {
    SoftLockup,
    HardLockup,
    HungTask,
    RcuStall,
    WorkqueueStall,
    SchedulerStall,
}

/// Recovery action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogRecoveryAction {
    None,
    LogWarning,
    SendSignal,
    KillProcess,
    Panic,
    NmiBacktrace,
    ResetCpu,
}

/// Watchdog state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogState {
    Normal,
    Warning,
    Triggered,
    Recovering,
    Disabled,
}

/// Per-CPU watchdog state
#[derive(Debug, Clone)]
pub struct CpuWatchdog {
    pub cpu_id: u32,
    pub state: WatchdogState,
    pub last_touch_ns: u64,
    pub last_nmi_ns: u64,
    pub soft_lockup_count: u64,
    pub hard_lockup_count: u64,
    pub threshold_ns: u64,
    pub current_task_id: u64,
    pub current_task_ns: u64,
    pub in_nmi: bool,
}

impl CpuWatchdog {
    pub fn new(cpu_id: u32, threshold_ns: u64) -> Self {
        Self {
            cpu_id,
            state: WatchdogState::Normal,
            last_touch_ns: 0,
            last_nmi_ns: 0,
            soft_lockup_count: 0,
            hard_lockup_count: 0,
            threshold_ns,
            current_task_id: 0,
            current_task_ns: 0,
            in_nmi: false,
        }
    }

    pub fn touch(&mut self, now_ns: u64) {
        self.last_touch_ns = now_ns;
        self.state = WatchdogState::Normal;
    }

    pub fn check_soft(&mut self, now_ns: u64) -> bool {
        let elapsed = now_ns.saturating_sub(self.last_touch_ns);
        if elapsed > self.threshold_ns {
            self.soft_lockup_count += 1;
            self.state = WatchdogState::Triggered;
            true
        } else if elapsed > self.threshold_ns / 2 {
            self.state = WatchdogState::Warning;
            false
        } else {
            false
        }
    }

    pub fn check_hard(&mut self, now_ns: u64) -> bool {
        let elapsed = now_ns.saturating_sub(self.last_nmi_ns);
        if elapsed > self.threshold_ns * 2 {
            self.hard_lockup_count += 1;
            self.state = WatchdogState::Triggered;
            true
        } else { false }
    }
}

/// Hung task entry
#[derive(Debug, Clone)]
pub struct HungTaskEntry {
    pub task_id: u64,
    pub in_kernel_ns: u64,
    pub wait_channel_hash: u64,
    pub detected_ns: u64,
    pub state: WatchdogState,
}

impl HungTaskEntry {
    pub fn new(task_id: u64, in_kernel_ns: u64, now_ns: u64) -> Self {
        Self {
            task_id,
            in_kernel_ns,
            wait_channel_hash: 0,
            detected_ns: now_ns,
            state: WatchdogState::Warning,
        }
    }
}

/// RCU stall record
#[derive(Debug, Clone)]
pub struct RcuStallRecord {
    pub cpu_id: u32,
    pub gp_number: u64,
    pub stall_duration_ns: u64,
    pub detected_ns: u64,
    pub blocking_task: Option<u64>,
}

/// Watchdog event for logging
#[derive(Debug, Clone)]
pub struct WatchdogEvent {
    pub timestamp_ns: u64,
    pub lockup_type: LockupType,
    pub cpu_id: Option<u32>,
    pub task_id: Option<u64>,
    pub action_taken: WatchdogRecoveryAction,
    pub duration_ns: u64,
}

/// Holistic Watchdog stats
#[derive(Debug, Clone, Default)]
pub struct HolisticWatchdogMgrStats {
    pub total_cpus: usize,
    pub cpus_normal: usize,
    pub cpus_warning: usize,
    pub cpus_triggered: usize,
    pub total_soft_lockups: u64,
    pub total_hard_lockups: u64,
    pub hung_tasks: usize,
    pub rcu_stalls: usize,
    pub total_events: usize,
}

/// Holistic Watchdog Manager
pub struct HolisticWatchdogMgr {
    cpu_watchdogs: BTreeMap<u32, CpuWatchdog>,
    hung_tasks: BTreeMap<u64, HungTaskEntry>,
    rcu_stalls: Vec<RcuStallRecord>,
    events: Vec<WatchdogEvent>,
    hung_task_threshold_ns: u64,
    max_events: usize,
    stats: HolisticWatchdogMgrStats,
}

impl HolisticWatchdogMgr {
    pub fn new(hung_task_threshold_ns: u64, max_events: usize) -> Self {
        Self {
            cpu_watchdogs: BTreeMap::new(),
            hung_tasks: BTreeMap::new(),
            rcu_stalls: Vec::new(),
            events: Vec::new(),
            hung_task_threshold_ns,
            max_events,
            stats: HolisticWatchdogMgrStats::default(),
        }
    }

    pub fn register_cpu(&mut self, cpu_id: u32, threshold_ns: u64) {
        self.cpu_watchdogs.entry(cpu_id)
            .or_insert_with(|| CpuWatchdog::new(cpu_id, threshold_ns));
    }

    pub fn touch_cpu(&mut self, cpu_id: u32, now_ns: u64) {
        if let Some(wd) = self.cpu_watchdogs.get_mut(&cpu_id) {
            wd.touch(now_ns);
        }
    }

    /// Run watchdog check on all CPUs
    pub fn check_all(&mut self, now_ns: u64) -> Vec<WatchdogEvent> {
        let mut new_events = Vec::new();

        let cpu_ids: Vec<u32> = self.cpu_watchdogs.keys().copied().collect();
        for cpu_id in cpu_ids {
            let soft = if let Some(wd) = self.cpu_watchdogs.get_mut(&cpu_id) {
                wd.check_soft(now_ns)
            } else { false };

            if soft {
                let action = WatchdogRecoveryAction::LogWarning;
                new_events.push(WatchdogEvent {
                    timestamp_ns: now_ns,
                    lockup_type: LockupType::SoftLockup,
                    cpu_id: Some(cpu_id),
                    task_id: self.cpu_watchdogs.get(&cpu_id).map(|w| w.current_task_id),
                    action_taken: action,
                    duration_ns: now_ns.saturating_sub(
                        self.cpu_watchdogs.get(&cpu_id).map(|w| w.last_touch_ns).unwrap_or(0)
                    ),
                });
            }
        }

        // Check hung tasks
        let hung_ids: Vec<u64> = self.hung_tasks.keys().copied().collect();
        for task_id in hung_ids {
            if let Some(entry) = self.hung_tasks.get_mut(&task_id) {
                if entry.in_kernel_ns > self.hung_task_threshold_ns
                    && entry.state != WatchdogState::Triggered
                {
                    entry.state = WatchdogState::Triggered;
                    new_events.push(WatchdogEvent {
                        timestamp_ns: now_ns,
                        lockup_type: LockupType::HungTask,
                        cpu_id: None,
                        task_id: Some(task_id),
                        action_taken: WatchdogRecoveryAction::LogWarning,
                        duration_ns: entry.in_kernel_ns,
                    });
                }
            }
        }

        for evt in &new_events {
            self.events.push(evt.clone());
        }
        while self.events.len() > self.max_events {
            self.events.remove(0);
        }

        self.recompute();
        new_events
    }

    pub fn report_hung_task(&mut self, task_id: u64, in_kernel_ns: u64, now_ns: u64) {
        self.hung_tasks.entry(task_id)
            .or_insert_with(|| HungTaskEntry::new(task_id, in_kernel_ns, now_ns))
            .in_kernel_ns = in_kernel_ns;
    }

    pub fn clear_hung_task(&mut self, task_id: u64) {
        self.hung_tasks.remove(&task_id);
    }

    pub fn report_rcu_stall(&mut self, stall: RcuStallRecord) {
        self.rcu_stalls.push(stall);
        if self.rcu_stalls.len() > 256 {
            self.rcu_stalls.remove(0);
        }
    }

    fn recompute(&mut self) {
        self.stats.total_cpus = self.cpu_watchdogs.len();
        self.stats.cpus_normal = self.cpu_watchdogs.values()
            .filter(|w| w.state == WatchdogState::Normal).count();
        self.stats.cpus_warning = self.cpu_watchdogs.values()
            .filter(|w| w.state == WatchdogState::Warning).count();
        self.stats.cpus_triggered = self.cpu_watchdogs.values()
            .filter(|w| w.state == WatchdogState::Triggered).count();
        self.stats.total_soft_lockups = self.cpu_watchdogs.values()
            .map(|w| w.soft_lockup_count).sum();
        self.stats.total_hard_lockups = self.cpu_watchdogs.values()
            .map(|w| w.hard_lockup_count).sum();
        self.stats.hung_tasks = self.hung_tasks.len();
        self.stats.rcu_stalls = self.rcu_stalls.len();
        self.stats.total_events = self.events.len();
    }

    pub fn cpu_watchdog(&self, id: u32) -> Option<&CpuWatchdog> { self.cpu_watchdogs.get(&id) }
    pub fn stats(&self) -> &HolisticWatchdogMgrStats { &self.stats }
}

// ============================================================================
// Merged from watchdog_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogV2Type {
    Hardware,
    Software,
    NmiWatchdog,
    SoftLockup,
    HardLockup,
}

/// Watchdog state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogV2State {
    Active,
    Stopped,
    Expired,
    Recovering,
}

/// Watchdog instance
#[derive(Debug)]
pub struct WatchdogV2 {
    pub id: u64,
    pub wd_type: WatchdogV2Type,
    pub state: WatchdogV2State,
    pub timeout_ms: u64,
    pub last_ping: u64,
    pub ping_count: u64,
    pub expire_count: u64,
    pub cpu_id: Option<u32>,
    pub pretimeout_ms: u64,
}

impl WatchdogV2 {
    pub fn new(id: u64, wt: WatchdogV2Type, timeout_ms: u64) -> Self {
        Self { id, wd_type: wt, state: WatchdogV2State::Active, timeout_ms, last_ping: 0, ping_count: 0, expire_count: 0, cpu_id: None, pretimeout_ms: timeout_ms / 2 }
    }

    pub fn ping(&mut self, now: u64) { self.last_ping = now; self.ping_count += 1; }

    pub fn check(&mut self, now: u64) -> bool {
        if self.state != WatchdogV2State::Active { return false; }
        if now - self.last_ping > self.timeout_ms { self.state = WatchdogV2State::Expired; self.expire_count += 1; true }
        else { false }
    }

    pub fn is_pretimeout(&self, now: u64) -> bool {
        self.state == WatchdogV2State::Active && (now - self.last_ping) > self.pretimeout_ms
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct WatchdogV2MgrStats {
    pub total_watchdogs: u32,
    pub active: u32,
    pub expired: u32,
    pub total_pings: u64,
    pub total_expires: u64,
}

/// Main watchdog v2 manager
pub struct HolisticWatchdogMgrV2 {
    watchdogs: BTreeMap<u64, WatchdogV2>,
    next_id: u64,
}

impl HolisticWatchdogMgrV2 {
    pub fn new() -> Self { Self { watchdogs: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, wt: WatchdogV2Type, timeout_ms: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.watchdogs.insert(id, WatchdogV2::new(id, wt, timeout_ms));
        id
    }

    pub fn ping(&mut self, id: u64, now: u64) {
        if let Some(w) = self.watchdogs.get_mut(&id) { w.ping(now); }
    }

    pub fn check_all(&mut self, now: u64) -> u32 {
        let mut expired = 0u32;
        for w in self.watchdogs.values_mut() { if w.check(now) { expired += 1; } }
        expired
    }

    pub fn stats(&self) -> WatchdogV2MgrStats {
        let active = self.watchdogs.values().filter(|w| w.state == WatchdogV2State::Active).count() as u32;
        let expired = self.watchdogs.values().filter(|w| w.state == WatchdogV2State::Expired).count() as u32;
        let pings: u64 = self.watchdogs.values().map(|w| w.ping_count).sum();
        let expires: u64 = self.watchdogs.values().map(|w| w.expire_count).sum();
        WatchdogV2MgrStats { total_watchdogs: self.watchdogs.len() as u32, active, expired, total_pings: pings, total_expires: expires }
    }
}
