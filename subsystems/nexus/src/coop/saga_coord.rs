//! # Coop Saga Coordinator
//!
//! Distributed saga pattern coordinator for cooperative transactions:
//! - Multi-step transaction orchestration
//! - Compensating transaction support
//! - Forward recovery and backward recovery
//! - Saga state machine with persistent log
//! - Timeout handling per step
//! - Concurrent saga execution

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Saga status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaStatus {
    Created,
    Running,
    Compensating,
    Completed,
    CompensationComplete,
    Failed,
    PartialFailure,
    TimedOut,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    CompensationPending,
    CompensationRunning,
    Compensated,
    CompensationFailed,
    Skipped,
}

/// Saga recovery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    ForwardRetry,
    BackwardCompensate,
    BestEffort,
}

/// A single step in a saga
#[derive(Debug, Clone)]
pub struct SagaStep {
    pub id: u64,
    pub name_hash: u64,
    pub status: StepStatus,
    pub participant: u64,
    pub timeout_ns: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub result_hash: u64,
    pub compensation_result: u64,
    pub retries: u32,
    pub max_retries: u32,
    pub order: u32,
    pub idempotency_key: u64,
}

impl SagaStep {
    pub fn new(id: u64, name_hash: u64, participant: u64, order: u32, timeout: u64) -> Self {
        let mut ik: u64 = 0xcbf29ce484222325;
        ik ^= id; ik = ik.wrapping_mul(0x100000001b3);
        ik ^= name_hash; ik = ik.wrapping_mul(0x100000001b3);
        Self {
            id, name_hash, status: StepStatus::Pending, participant,
            timeout_ns: timeout, start_ts: 0, end_ts: 0, result_hash: 0,
            compensation_result: 0, retries: 0, max_retries: 3, order,
            idempotency_key: ik,
        }
    }

    #[inline(always)]
    pub fn start(&mut self, ts: u64) { self.status = StepStatus::Running; self.start_ts = ts; }
    #[inline(always)]
    pub fn complete(&mut self, result: u64, ts: u64) { self.status = StepStatus::Completed; self.result_hash = result; self.end_ts = ts; }

    #[inline]
    pub fn fail(&mut self, ts: u64) {
        self.retries += 1;
        if self.retries >= self.max_retries { self.status = StepStatus::Failed; } else { self.status = StepStatus::Pending; }
        self.end_ts = ts;
    }

    #[inline(always)]
    pub fn begin_compensate(&mut self) { self.status = StepStatus::CompensationPending; }
    #[inline(always)]
    pub fn run_compensate(&mut self, ts: u64) { self.status = StepStatus::CompensationRunning; self.start_ts = ts; }
    #[inline(always)]
    pub fn compensated(&mut self, result: u64, ts: u64) { self.status = StepStatus::Compensated; self.compensation_result = result; self.end_ts = ts; }
    #[inline(always)]
    pub fn compensation_fail(&mut self, ts: u64) { self.status = StepStatus::CompensationFailed; self.end_ts = ts; }

    #[inline(always)]
    pub fn is_done(&self) -> bool { matches!(self.status, StepStatus::Completed | StepStatus::Failed | StepStatus::Compensated | StepStatus::CompensationFailed | StepStatus::Skipped) }
    #[inline(always)]
    pub fn is_timed_out(&self, now: u64) -> bool { self.status == StepStatus::Running && now.saturating_sub(self.start_ts) > self.timeout_ns }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.end_ts.saturating_sub(self.start_ts) }
}

/// Log entry for saga persistence
#[derive(Debug, Clone)]
pub struct SagaLogEntry {
    pub saga_id: u64,
    pub step_id: u64,
    pub event: SagaLogEvent,
    pub ts: u64,
    pub data_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SagaLogEvent {
    SagaStarted,
    StepStarted,
    StepCompleted,
    StepFailed,
    CompensationStarted,
    CompensationCompleted,
    CompensationFailed,
    SagaCompleted,
    SagaAborted,
}

/// A saga instance
#[derive(Debug, Clone)]
pub struct Saga {
    pub id: u64,
    pub status: SagaStatus,
    pub steps: Vec<SagaStep>,
    pub recovery: RecoveryMode,
    pub current_step: usize,
    pub create_ts: u64,
    pub end_ts: u64,
    pub log: Vec<SagaLogEntry>,
    pub correlation_id: u64,
}

impl Saga {
    pub fn new(id: u64, recovery: RecoveryMode, ts: u64) -> Self {
        let mut cid: u64 = 0xcbf29ce484222325;
        cid ^= id; cid = cid.wrapping_mul(0x100000001b3);
        cid ^= ts; cid = cid.wrapping_mul(0x100000001b3);
        Self {
            id, status: SagaStatus::Created, steps: Vec::new(),
            recovery, current_step: 0, create_ts: ts, end_ts: 0,
            log: Vec::new(), correlation_id: cid,
        }
    }

    #[inline(always)]
    pub fn add_step(&mut self, step: SagaStep) { self.steps.push(step); }

    #[inline(always)]
    pub fn start(&mut self, ts: u64) {
        self.status = SagaStatus::Running;
        self.log.push(SagaLogEntry { saga_id: self.id, step_id: 0, event: SagaLogEvent::SagaStarted, ts, data_hash: 0 });
    }

    pub fn advance(&mut self, ts: u64) -> Option<u64> {
        if self.status != SagaStatus::Running { return None; }
        if self.current_step >= self.steps.len() {
            self.status = SagaStatus::Completed;
            self.end_ts = ts;
            self.log.push(SagaLogEntry { saga_id: self.id, step_id: 0, event: SagaLogEvent::SagaCompleted, ts, data_hash: 0 });
            return None;
        }
        let step = &mut self.steps[self.current_step];
        if step.status == StepStatus::Pending {
            step.start(ts);
            self.log.push(SagaLogEntry { saga_id: self.id, step_id: step.id, event: SagaLogEvent::StepStarted, ts, data_hash: 0 });
            return Some(step.id);
        }
        if step.status == StepStatus::Completed {
            self.current_step += 1;
            return self.advance(ts);
        }
        if step.status == StepStatus::Failed {
            match self.recovery {
                RecoveryMode::BackwardCompensate => self.begin_compensation(),
                RecoveryMode::ForwardRetry => { step.status = StepStatus::Pending; }
                RecoveryMode::BestEffort => { self.current_step += 1; return self.advance(ts); }
            }
        }
        None
    }

    #[inline]
    pub fn complete_step(&mut self, step_id: u64, result: u64, ts: u64) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == step_id) {
            s.complete(result, ts);
            self.log.push(SagaLogEntry { saga_id: self.id, step_id, event: SagaLogEvent::StepCompleted, ts, data_hash: result });
        }
    }

    #[inline]
    pub fn fail_step(&mut self, step_id: u64, ts: u64) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == step_id) {
            s.fail(ts);
            self.log.push(SagaLogEntry { saga_id: self.id, step_id, event: SagaLogEvent::StepFailed, ts, data_hash: 0 });
        }
    }

    fn begin_compensation(&mut self) {
        self.status = SagaStatus::Compensating;
        for i in (0..self.current_step).rev() {
            if self.steps[i].status == StepStatus::Completed {
                self.steps[i].begin_compensate();
            }
        }
    }

    #[inline(always)]
    pub fn next_compensation(&mut self) -> Option<u64> {
        self.steps.iter().rev().find(|s| s.status == StepStatus::CompensationPending).map(|s| s.id)
    }

    #[inline]
    pub fn compensate_step(&mut self, step_id: u64, result: u64, ts: u64) {
        if let Some(s) = self.steps.iter_mut().find(|s| s.id == step_id) {
            s.compensated(result, ts);
            self.log.push(SagaLogEntry { saga_id: self.id, step_id, event: SagaLogEvent::CompensationCompleted, ts, data_hash: result });
        }
        if self.steps.iter().filter(|s| s.status == StepStatus::CompensationPending || s.status == StepStatus::CompensationRunning).count() == 0 {
            self.status = SagaStatus::CompensationComplete;
            self.end_ts = ts;
        }
    }

    #[inline(always)]
    pub fn is_done(&self) -> bool { matches!(self.status, SagaStatus::Completed | SagaStatus::CompensationComplete | SagaStatus::Failed) }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.end_ts.saturating_sub(self.create_ts) }
    #[inline(always)]
    pub fn step_count(&self) -> usize { self.steps.len() }
    #[inline(always)]
    pub fn completed_steps(&self) -> usize { self.steps.iter().filter(|s| s.status == StepStatus::Completed).count() }
}

/// Saga coordinator stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SagaStats {
    pub total_sagas: u64,
    pub completed: u64,
    pub compensated: u64,
    pub failed: u64,
    pub running: u64,
    pub total_steps: u64,
    pub avg_latency_ns: u64,
}

/// Cooperative saga coordinator
pub struct CoopSagaCoord {
    sagas: BTreeMap<u64, Saga>,
    stats: SagaStats,
    next_id: u64,
    next_step_id: u64,
}

impl CoopSagaCoord {
    pub fn new() -> Self {
        Self { sagas: BTreeMap::new(), stats: SagaStats::default(), next_id: 1, next_step_id: 1 }
    }

    #[inline]
    pub fn create_saga(&mut self, recovery: RecoveryMode, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.sagas.insert(id, Saga::new(id, recovery, ts));
        self.stats.total_sagas += 1;
        id
    }

    #[inline]
    pub fn add_step(&mut self, saga_id: u64, name_hash: u64, participant: u64, timeout: u64) -> Option<u64> {
        let sid = self.next_step_id; self.next_step_id += 1;
        let saga = self.sagas.get_mut(&saga_id)?;
        let order = saga.steps.len() as u32;
        saga.add_step(SagaStep::new(sid, name_hash, participant, order, timeout));
        self.stats.total_steps += 1;
        Some(sid)
    }

    #[inline(always)]
    pub fn start(&mut self, saga_id: u64, ts: u64) {
        if let Some(s) = self.sagas.get_mut(&saga_id) { s.start(ts); }
    }

    #[inline(always)]
    pub fn advance(&mut self, saga_id: u64, ts: u64) -> Option<u64> {
        self.sagas.get_mut(&saga_id)?.advance(ts)
    }

    #[inline(always)]
    pub fn complete_step(&mut self, saga_id: u64, step_id: u64, result: u64, ts: u64) {
        if let Some(s) = self.sagas.get_mut(&saga_id) { s.complete_step(step_id, result, ts); }
    }

    #[inline(always)]
    pub fn fail_step(&mut self, saga_id: u64, step_id: u64, ts: u64) {
        if let Some(s) = self.sagas.get_mut(&saga_id) { s.fail_step(step_id, ts); }
    }

    #[inline(always)]
    pub fn compensate_step(&mut self, saga_id: u64, step_id: u64, result: u64, ts: u64) {
        if let Some(s) = self.sagas.get_mut(&saga_id) { s.compensate_step(step_id, result, ts); }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.completed = self.sagas.values().filter(|s| s.status == SagaStatus::Completed).count() as u64;
        self.stats.compensated = self.sagas.values().filter(|s| s.status == SagaStatus::CompensationComplete).count() as u64;
        self.stats.failed = self.sagas.values().filter(|s| s.status == SagaStatus::Failed).count() as u64;
        self.stats.running = self.sagas.values().filter(|s| s.status == SagaStatus::Running).count() as u64;
        let done: Vec<&Saga> = self.sagas.values().filter(|s| s.is_done() && s.end_ts > 0).collect();
        if !done.is_empty() {
            let total: u64 = done.iter().map(|s| s.latency()).sum();
            self.stats.avg_latency_ns = total / done.len() as u64;
        }
    }

    #[inline(always)]
    pub fn saga(&self, id: u64) -> Option<&Saga> { self.sagas.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &SagaStats { &self.stats }
}
