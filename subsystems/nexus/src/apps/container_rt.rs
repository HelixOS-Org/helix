//! # Apps Container Runtime
//!
//! Application container lifecycle management:
//! - Container creation and destruction
//! - Resource limit enforcement
//! - Image and layer management
//! - Container state machine
//! - Health check orchestration
//! - Network endpoint management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Container state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Creating,
    Created,
    Running,
    Paused,
    Stopped,
    Removing,
    Dead,
}

/// Container restart policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    OnFailure(u32),
    Always,
    UnlessStopped,
}

/// Resource limits for container
#[derive(Debug, Clone)]
pub struct ContainerResources {
    pub cpu_shares: u32,
    pub cpu_quota_us: i64,
    pub cpu_period_us: u64,
    pub memory_limit: u64,
    pub memory_swap: u64,
    pub pids_limit: u32,
    pub io_weight: u16,
    pub cpuset_cpus: Vec<u32>,
}

impl ContainerResources {
    pub fn default_limits() -> Self {
        Self {
            cpu_shares: 1024, cpu_quota_us: -1, cpu_period_us: 100_000,
            memory_limit: u64::MAX, memory_swap: u64::MAX, pids_limit: 4096,
            io_weight: 100, cpuset_cpus: Vec::new(),
        }
    }
}

/// Container network endpoint
#[derive(Debug, Clone)]
pub struct NetEndpoint {
    pub interface: String,
    pub ip_addr: u32,
    pub gateway: u32,
    pub mac: [u8; 6],
    pub mtu: u16,
}

/// Container health check
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub retries: u32,
    pub consecutive_failures: u32,
    pub last_check_ts: u64,
    pub healthy: bool,
}

impl HealthCheck {
    pub fn new(interval: u64, timeout: u64, retries: u32) -> Self {
        Self { interval_ms: interval, timeout_ms: timeout, retries, consecutive_failures: 0, last_check_ts: 0, healthy: true }
    }

    pub fn pass(&mut self, ts: u64) { self.consecutive_failures = 0; self.healthy = true; self.last_check_ts = ts; }
    pub fn fail(&mut self, ts: u64) { self.consecutive_failures += 1; self.last_check_ts = ts; if self.consecutive_failures >= self.retries { self.healthy = false; } }
}

/// Container descriptor
#[derive(Debug, Clone)]
pub struct Container {
    pub id: u64,
    pub name: String,
    pub image: String,
    pub state: ContainerState,
    pub pid: Option<u64>,
    pub resources: ContainerResources,
    pub restart_policy: RestartPolicy,
    pub restart_count: u32,
    pub endpoints: Vec<NetEndpoint>,
    pub health: Option<HealthCheck>,
    pub env_vars: Vec<(String, String)>,
    pub labels: Vec<(String, String)>,
    pub created_at: u64,
    pub started_at: u64,
    pub finished_at: u64,
    pub exit_code: i32,
    pub oom_killed: bool,
    pub cpu_usage_us: u64,
    pub mem_usage: u64,
}

impl Container {
    pub fn new(id: u64, name: String, image: String) -> Self {
        Self {
            id, name, image, state: ContainerState::Creating, pid: None,
            resources: ContainerResources::default_limits(),
            restart_policy: RestartPolicy::Never, restart_count: 0,
            endpoints: Vec::new(), health: None,
            env_vars: Vec::new(), labels: Vec::new(),
            created_at: 0, started_at: 0, finished_at: 0,
            exit_code: 0, oom_killed: false, cpu_usage_us: 0, mem_usage: 0,
        }
    }

    pub fn create(&mut self, ts: u64) { self.state = ContainerState::Created; self.created_at = ts; }
    pub fn start(&mut self, pid: u64, ts: u64) { self.state = ContainerState::Running; self.pid = Some(pid); self.started_at = ts; }
    pub fn pause(&mut self) { self.state = ContainerState::Paused; }
    pub fn unpause(&mut self) { self.state = ContainerState::Running; }
    pub fn stop(&mut self, code: i32, ts: u64) { self.state = ContainerState::Stopped; self.exit_code = code; self.finished_at = ts; self.pid = None; }
    pub fn remove(&mut self) { self.state = ContainerState::Removing; }
    pub fn is_running(&self) -> bool { self.state == ContainerState::Running }
    pub fn uptime_us(&self, now: u64) -> u64 { if self.is_running() { now.saturating_sub(self.started_at) } else { self.finished_at.saturating_sub(self.started_at) } }

    pub fn should_restart(&self) -> bool {
        match self.restart_policy {
            RestartPolicy::Never => false,
            RestartPolicy::OnFailure(max) => self.exit_code != 0 && self.restart_count < max,
            RestartPolicy::Always => true,
            RestartPolicy::UnlessStopped => self.state != ContainerState::Stopped,
        }
    }
}

/// Container runtime stats
#[derive(Debug, Clone, Default)]
pub struct ContainerRuntimeStats {
    pub total_containers: usize,
    pub running_containers: usize,
    pub stopped_containers: usize,
    pub paused_containers: usize,
    pub total_restarts: u64,
    pub oom_kills: u64,
    pub unhealthy: usize,
}

/// Apps container runtime
pub struct AppsContainerRuntime {
    containers: BTreeMap<u64, Container>,
    stats: ContainerRuntimeStats,
    next_id: u64,
}

impl AppsContainerRuntime {
    pub fn new() -> Self { Self { containers: BTreeMap::new(), stats: ContainerRuntimeStats::default(), next_id: 1 } }

    pub fn create(&mut self, name: String, image: String, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut c = Container::new(id, name, image);
        c.create(ts);
        self.containers.insert(id, c);
        id
    }

    pub fn start(&mut self, id: u64, pid: u64, ts: u64) { if let Some(c) = self.containers.get_mut(&id) { c.start(pid, ts); } }
    pub fn stop(&mut self, id: u64, code: i32, ts: u64) { if let Some(c) = self.containers.get_mut(&id) { c.stop(code, ts); } }
    pub fn pause(&mut self, id: u64) { if let Some(c) = self.containers.get_mut(&id) { c.pause(); } }
    pub fn unpause(&mut self, id: u64) { if let Some(c) = self.containers.get_mut(&id) { c.unpause(); } }
    pub fn remove(&mut self, id: u64) { if let Some(c) = self.containers.get_mut(&id) { c.remove(); } self.containers.remove(&id); }

    pub fn set_resources(&mut self, id: u64, res: ContainerResources) { if let Some(c) = self.containers.get_mut(&id) { c.resources = res; } }
    pub fn set_restart_policy(&mut self, id: u64, policy: RestartPolicy) { if let Some(c) = self.containers.get_mut(&id) { c.restart_policy = policy; } }

    pub fn health_check(&mut self, id: u64, passed: bool, ts: u64) {
        if let Some(c) = self.containers.get_mut(&id) {
            if let Some(h) = c.health.as_mut() { if passed { h.pass(ts); } else { h.fail(ts); } }
        }
    }

    pub fn check_restarts(&mut self, ts: u64) -> Vec<u64> {
        let need_restart: Vec<u64> = self.containers.values().filter(|c| c.state == ContainerState::Stopped && c.should_restart()).map(|c| c.id).collect();
        for &id in &need_restart { if let Some(c) = self.containers.get_mut(&id) { c.restart_count += 1; c.state = ContainerState::Created; } }
        need_restart
    }

    pub fn recompute(&mut self) {
        self.stats.total_containers = self.containers.len();
        self.stats.running_containers = self.containers.values().filter(|c| c.is_running()).count();
        self.stats.stopped_containers = self.containers.values().filter(|c| c.state == ContainerState::Stopped).count();
        self.stats.paused_containers = self.containers.values().filter(|c| c.state == ContainerState::Paused).count();
        self.stats.total_restarts = self.containers.values().map(|c| c.restart_count as u64).sum();
        self.stats.oom_kills = self.containers.values().filter(|c| c.oom_killed).count() as u64;
        self.stats.unhealthy = self.containers.values().filter(|c| c.health.as_ref().map(|h| !h.healthy).unwrap_or(false)).count();
    }

    pub fn container(&self, id: u64) -> Option<&Container> { self.containers.get(&id) }
    pub fn stats(&self) -> &ContainerRuntimeStats { &self.stats }
}
