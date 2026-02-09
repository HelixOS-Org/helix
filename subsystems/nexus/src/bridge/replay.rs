//! # Bridge Replay Engine
//!
//! Syscall replay and record for debugging and testing:
//! - Syscall recording with full arguments
//! - Deterministic replay
//! - Replay divergence detection
//! - Conditional replay (selective syscalls)
//! - Replay checkpointing
//! - Performance comparison (record vs replay)

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RECORDED SYSCALL
// ============================================================================

/// Syscall argument type
#[derive(Debug, Clone)]
pub enum SyscallArg {
    /// Integer argument
    Integer(u64),
    /// Pointer (virtual address)
    Pointer(u64),
    /// Buffer (address + length)
    Buffer(u64, usize),
    /// String (address + length)
    StringArg(u64, usize),
    /// File descriptor
    Fd(i32),
    /// Flags
    Flags(u64),
}

/// Syscall result
#[derive(Debug, Clone)]
pub enum SyscallResult {
    /// Success with return value
    Success(u64),
    /// Error with errno
    Error(i32),
    /// Signal interrupted
    Interrupted,
    /// Timeout
    Timeout,
}

/// Recorded syscall event
#[derive(Debug, Clone)]
pub struct RecordedSyscall {
    /// Sequence number
    pub sequence: u64,
    /// Timestamp (ns)
    pub timestamp_ns: u64,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Arguments
    pub args: Vec<SyscallArg>,
    /// Result
    pub result: SyscallResult,
    /// Duration (ns)
    pub duration_ns: u64,
    /// CPU core
    pub cpu: u32,
}

impl RecordedSyscall {
    pub fn new(sequence: u64, timestamp_ns: u64, pid: u64, tid: u64, syscall_nr: u32) -> Self {
        Self {
            sequence,
            timestamp_ns,
            pid,
            tid,
            syscall_nr,
            args: Vec::new(),
            result: SyscallResult::Success(0),
            duration_ns: 0,
            cpu: 0,
        }
    }

    #[inline(always)]
    pub fn add_arg(&mut self, arg: SyscallArg) {
        self.args.push(arg);
    }

    #[inline(always)]
    pub fn set_result(&mut self, result: SyscallResult, duration_ns: u64) {
        self.result = result;
        self.duration_ns = duration_ns;
    }

    #[inline(always)]
    pub fn is_success(&self) -> bool {
        matches!(self.result, SyscallResult::Success(_))
    }
}

// ============================================================================
// RECORDING SESSION
// ============================================================================

/// Recording filter
#[derive(Debug, Clone)]
pub struct RecordingFilter {
    /// Only these PIDs (empty = all)
    pub pids: Vec<u64>,
    /// Only these syscall numbers (empty = all)
    pub syscall_nrs: Vec<u32>,
    /// Skip fast syscalls under this duration (ns)
    pub min_duration_ns: u64,
    /// Record arguments
    pub record_args: bool,
    /// Record buffers (expensive)
    pub record_buffers: bool,
}

impl RecordingFilter {
    #[inline]
    pub fn all() -> Self {
        Self {
            pids: Vec::new(),
            syscall_nrs: Vec::new(),
            min_duration_ns: 0,
            record_args: true,
            record_buffers: false,
        }
    }

    #[inline]
    pub fn for_pid(pid: u64) -> Self {
        Self {
            pids: alloc::vec![pid],
            syscall_nrs: Vec::new(),
            min_duration_ns: 0,
            record_args: true,
            record_buffers: false,
        }
    }

    /// Check if syscall matches filter
    pub fn matches(&self, pid: u64, syscall_nr: u32, duration_ns: u64) -> bool {
        if !self.pids.is_empty() && !self.pids.contains(&pid) {
            return false;
        }
        if !self.syscall_nrs.is_empty() && !self.syscall_nrs.contains(&syscall_nr) {
            return false;
        }
        if duration_ns < self.min_duration_ns {
            return false;
        }
        true
    }
}

/// Recording session
#[derive(Debug, Clone)]
pub struct RecordingSession {
    /// Session ID
    pub id: u64,
    /// Filter
    pub filter: RecordingFilter,
    /// Recorded syscalls
    pub syscalls: Vec<RecordedSyscall>,
    /// Start time
    pub start_time: u64,
    /// End time (0 = ongoing)
    pub end_time: u64,
    /// Max records
    pub max_records: usize,
    /// Dropped records
    pub dropped: u64,
    /// Active
    pub active: bool,
}

impl RecordingSession {
    pub fn new(id: u64, filter: RecordingFilter, start_time: u64) -> Self {
        Self {
            id,
            filter,
            syscalls: Vec::new(),
            start_time,
            end_time: 0,
            max_records: 100_000,
            dropped: 0,
            active: true,
        }
    }

    /// Record a syscall
    pub fn record(&mut self, syscall: RecordedSyscall) -> bool {
        if !self.active {
            return false;
        }
        if !self
            .filter
            .matches(syscall.pid, syscall.syscall_nr, syscall.duration_ns)
        {
            return false;
        }
        if self.syscalls.len() >= self.max_records {
            self.dropped += 1;
            return false;
        }
        self.syscalls.push(syscall);
        true
    }

    /// Stop recording
    #[inline(always)]
    pub fn stop(&mut self, end_time: u64) {
        self.active = false;
        self.end_time = end_time;
    }

    /// Duration
    #[inline]
    pub fn duration_ns(&self) -> u64 {
        if self.end_time > 0 {
            self.end_time.saturating_sub(self.start_time)
        } else {
            0
        }
    }

    /// Syscall rate (per second)
    #[inline]
    pub fn rate(&self) -> f64 {
        let dur_s = self.duration_ns() as f64 / 1_000_000_000.0;
        if dur_s > 0.0 {
            self.syscalls.len() as f64 / dur_s
        } else {
            0.0
        }
    }
}

// ============================================================================
// REPLAY ENGINE
// ============================================================================

/// Replay state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayState {
    /// Ready to start
    Ready,
    /// Replaying
    Running,
    /// Paused
    Paused,
    /// Divergence detected
    Diverged,
    /// Completed
    Complete,
}

/// Replay divergence
#[derive(Debug, Clone)]
pub struct ReplayDivergence {
    /// Sequence number where divergence occurred
    pub sequence: u64,
    /// Expected result
    pub expected: SyscallResult,
    /// Actual result
    pub actual: SyscallResult,
    /// Syscall number
    pub syscall_nr: u32,
}

/// Replay session
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ReplaySession {
    /// Session ID
    pub id: u64,
    /// Source recording
    pub recording_id: u64,
    /// State
    pub state: ReplayState,
    /// Current position
    pub position: usize,
    /// Divergences
    pub divergences: Vec<ReplayDivergence>,
    /// Timing comparison (original_ns, replay_ns)
    pub timing: Vec<(u64, u64)>,
    /// Checkpoints (position → checkpoint data)
    pub checkpoints: BTreeMap<usize, u64>,
}

impl ReplaySession {
    pub fn new(id: u64, recording_id: u64) -> Self {
        Self {
            id,
            recording_id,
            state: ReplayState::Ready,
            position: 0,
            divergences: Vec::new(),
            timing: Vec::new(),
            checkpoints: BTreeMap::new(),
        }
    }

    #[inline(always)]
    pub fn start(&mut self) {
        self.state = ReplayState::Running;
    }

    #[inline]
    pub fn pause(&mut self) {
        if self.state == ReplayState::Running {
            self.state = ReplayState::Paused;
        }
    }

    #[inline]
    pub fn resume(&mut self) {
        if self.state == ReplayState::Paused {
            self.state = ReplayState::Running;
        }
    }

    #[inline(always)]
    pub fn record_step(&mut self, original_ns: u64, replay_ns: u64) {
        self.timing.push((original_ns, replay_ns));
        self.position += 1;
    }

    #[inline(always)]
    pub fn record_divergence(&mut self, div: ReplayDivergence) {
        self.divergences.push(div);
        self.state = ReplayState::Diverged;
    }

    #[inline(always)]
    pub fn checkpoint(&mut self, data: u64) {
        self.checkpoints.insert(self.position, data);
    }

    #[inline(always)]
    pub fn complete(&mut self) {
        self.state = ReplayState::Complete;
    }

    /// Speedup ratio
    #[inline]
    pub fn speedup(&self) -> f64 {
        if self.timing.is_empty() {
            return 1.0;
        }
        let orig_total: u64 = self.timing.iter().map(|(o, _)| o).sum();
        let replay_total: u64 = self.timing.iter().map(|(_, r)| r).sum();
        if replay_total == 0 {
            return 1.0;
        }
        orig_total as f64 / replay_total as f64
    }

    /// Divergence rate
    #[inline]
    pub fn divergence_rate(&self) -> f64 {
        if self.position == 0 {
            return 0.0;
        }
        self.divergences.len() as f64 / self.position as f64
    }
}

// ============================================================================
// REPLAY MANAGER
// ============================================================================

/// Replay manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ReplayManagerStats {
    /// Active recordings
    pub active_recordings: usize,
    /// Completed recordings
    pub completed_recordings: usize,
    /// Active replays
    pub active_replays: usize,
    /// Total divergences
    pub total_divergences: u64,
}

/// Bridge replay manager
#[repr(align(64))]
pub struct BridgeReplayManager {
    /// Recordings
    recordings: BTreeMap<u64, RecordingSession>,
    /// Replays
    replays: BTreeMap<u64, ReplaySession>,
    /// Next ID
    next_id: u64,
    /// Stats
    stats: ReplayManagerStats,
}

impl BridgeReplayManager {
    pub fn new() -> Self {
        Self {
            recordings: BTreeMap::new(),
            replays: BTreeMap::new(),
            next_id: 1,
            stats: ReplayManagerStats::default(),
        }
    }

    /// Start recording
    #[inline]
    pub fn start_recording(&mut self, filter: RecordingFilter, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.recordings
            .insert(id, RecordingSession::new(id, filter, now));
        self.stats.active_recordings = self.recordings.values().filter(|r| r.active).count();
        id
    }

    /// Stop recording
    #[inline]
    pub fn stop_recording(&mut self, id: u64, now: u64) {
        if let Some(rec) = self.recordings.get_mut(&id) {
            rec.stop(now);
        }
        self.stats.active_recordings = self.recordings.values().filter(|r| r.active).count();
        self.stats.completed_recordings = self.recordings.values().filter(|r| !r.active).count();
    }

    /// Record syscall
    #[inline]
    pub fn record_syscall(&mut self, recording_id: u64, syscall: RecordedSyscall) -> bool {
        if let Some(rec) = self.recordings.get_mut(&recording_id) {
            rec.record(syscall)
        } else {
            false
        }
    }

    /// Start replay
    pub fn start_replay(&mut self, recording_id: u64) -> Option<u64> {
        if !self.recordings.contains_key(&recording_id) {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        let mut session = ReplaySession::new(id, recording_id);
        session.start();
        self.replays.insert(id, session);
        self.stats.active_replays = self
            .replays
            .values()
            .filter(|r| r.state == ReplayState::Running)
            .count();
        Some(id)
    }

    /// Get recording
    #[inline(always)]
    pub fn recording(&self, id: u64) -> Option<&RecordingSession> {
        self.recordings.get(&id)
    }

    /// Get replay
    #[inline(always)]
    pub fn replay(&self, id: u64) -> Option<&ReplaySession> {
        self.replays.get(&id)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &ReplayManagerStats {
        &self.stats
    }
}
