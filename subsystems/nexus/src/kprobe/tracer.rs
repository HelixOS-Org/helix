//! Function Tracer
//!
//! Tracing function calls via kprobes.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{KprobeContext, KprobeId, ProbeAddress};

/// Function call record
#[derive(Debug, Clone)]
pub struct FunctionCall {
    /// Kprobe ID
    pub kprobe_id: KprobeId,
    /// Entry timestamp
    pub entry_time: u64,
    /// Exit timestamp (if captured)
    pub exit_time: Option<u64>,
    /// Return value (if captured)
    pub return_value: Option<u64>,
    /// Arguments captured
    pub arguments: Vec<u64>,
    /// CPU
    pub cpu: u32,
    /// PID
    pub pid: u64,
    /// Caller address
    pub caller: ProbeAddress,
    /// Depth in call chain
    pub depth: u32,
}

impl FunctionCall {
    /// Create new function call
    pub fn new(kprobe_id: KprobeId, entry_time: u64, cpu: u32, pid: u64) -> Self {
        Self {
            kprobe_id,
            entry_time,
            exit_time: None,
            return_value: None,
            arguments: Vec::new(),
            cpu,
            pid,
            caller: ProbeAddress::new(0),
            depth: 0,
        }
    }

    /// Record exit
    pub fn record_exit(&mut self, exit_time: u64, return_value: u64) {
        self.exit_time = Some(exit_time);
        self.return_value = Some(return_value);
    }

    /// Get duration
    pub fn duration(&self) -> Option<u64> {
        self.exit_time
            .map(|exit| exit.saturating_sub(self.entry_time))
    }
}

/// Function tracer statistics
#[derive(Debug, Clone)]
pub struct FunctionStats {
    /// Symbol name
    pub symbol: String,
    /// Total calls
    pub total_calls: u64,
    /// Total time (ns)
    pub total_time: u64,
    /// Average time (ns)
    pub avg_time: f32,
    /// Min time (ns)
    pub min_time: u64,
    /// Max time (ns)
    pub max_time: u64,
    /// Last call time
    pub last_call: u64,
}

impl FunctionStats {
    /// Create new stats
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            total_calls: 0,
            total_time: 0,
            avg_time: 0.0,
            min_time: u64::MAX,
            max_time: 0,
            last_call: 0,
        }
    }

    /// Update with call
    pub fn update(&mut self, duration: u64, timestamp: u64) {
        self.total_calls += 1;
        self.total_time += duration;
        self.avg_time = self.total_time as f32 / self.total_calls as f32;

        if duration < self.min_time {
            self.min_time = duration;
        }
        if duration > self.max_time {
            self.max_time = duration;
        }

        self.last_call = timestamp;
    }
}

/// Function tracer
pub struct FunctionTracer {
    /// Active calls (per CPU)
    active_calls: BTreeMap<(u32, u64), Vec<FunctionCall>>,
    /// Function statistics
    stats: BTreeMap<KprobeId, FunctionStats>,
    /// Completed calls history
    history: Vec<FunctionCall>,
    /// Max history size
    max_history: usize,
    /// Total calls traced
    total_calls: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl FunctionTracer {
    /// Create new function tracer
    pub fn new(max_history: usize) -> Self {
        Self {
            active_calls: BTreeMap::new(),
            stats: BTreeMap::new(),
            history: Vec::with_capacity(max_history),
            max_history,
            total_calls: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Record function entry
    pub fn record_entry(
        &mut self,
        kprobe_id: KprobeId,
        ctx: &KprobeContext,
        symbol: Option<&str>,
    ) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let key = (ctx.cpu, ctx.pid);
        let depth = self
            .active_calls
            .get(&key)
            .map(|v| v.len() as u32)
            .unwrap_or(0);

        let mut call = FunctionCall::new(kprobe_id, ctx.timestamp, ctx.cpu, ctx.pid);
        call.depth = depth;
        call.caller = ProbeAddress::new(ctx.ip);

        // Capture arguments
        for i in 0..6 {
            call.arguments.push(ctx.arg(i));
        }

        self.active_calls.entry(key).or_default().push(call);
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        // Initialize stats if needed
        if !self.stats.contains_key(&kprobe_id) {
            let sym = symbol
                .map(String::from)
                .unwrap_or_else(|| format!("kprobe_{}", kprobe_id.raw()));
            self.stats.insert(kprobe_id, FunctionStats::new(sym));
        }
    }

    /// Record function exit
    pub fn record_exit(&mut self, kprobe_id: KprobeId, ctx: &KprobeContext) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let key = (ctx.cpu, ctx.pid);

        if let Some(calls) = self.active_calls.get_mut(&key) {
            // Find matching call (last one with same kprobe_id)
            if let Some(idx) = calls
                .iter()
                .rposition(|c| c.kprobe_id == kprobe_id && c.exit_time.is_none())
            {
                let mut call = calls.remove(idx);
                call.record_exit(ctx.timestamp, ctx.return_value());

                // Update stats
                if let (Some(duration), Some(stats)) =
                    (call.duration(), self.stats.get_mut(&kprobe_id))
                {
                    stats.update(duration, ctx.timestamp);
                }

                // Add to history
                if self.history.len() >= self.max_history {
                    self.history.remove(0);
                }
                self.history.push(call);
            }
        }
    }

    /// Get function stats
    pub fn get_stats(&self, kprobe_id: KprobeId) -> Option<&FunctionStats> {
        self.stats.get(&kprobe_id)
    }

    /// Get all stats
    pub fn all_stats(&self) -> impl Iterator<Item = &FunctionStats> {
        self.stats.values()
    }

    /// Get hottest functions
    pub fn hottest_functions(&self, limit: usize) -> Vec<&FunctionStats> {
        let mut sorted: Vec<_> = self.stats.values().collect();
        sorted.sort_by(|a, b| b.total_calls.cmp(&a.total_calls));
        sorted.into_iter().take(limit).collect()
    }

    /// Get slowest functions
    pub fn slowest_functions(&self, limit: usize) -> Vec<&FunctionStats> {
        let mut sorted: Vec<_> = self.stats.values().collect();
        sorted.sort_by(|a, b| {
            b.avg_time
                .partial_cmp(&a.avg_time)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(limit).collect()
    }

    /// Enable/disable
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Get total calls
    pub fn total_calls(&self) -> u64 {
        self.total_calls.load(Ordering::Relaxed)
    }
}
