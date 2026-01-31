//! Syscall monitoring and pattern detection.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{ThreatSeverity, ThreatType};

// ============================================================================
// SYSCALL MONITOR
// ============================================================================

/// Syscall monitoring and analysis
pub struct SyscallMonitor {
    /// Syscall counts per process
    process_syscalls: BTreeMap<u64, SyscallStats>,
    /// Suspicious syscall patterns
    suspicious_patterns: Vec<SyscallPattern>,
    /// Recent syscalls for sequence detection
    recent_sequence: Vec<(u64, u32, u64)>, // (process, syscall, timestamp)
    /// Max sequence size
    max_sequence: usize,
    /// Total syscalls monitored
    total_syscalls: AtomicU64,
}

/// Per-process syscall statistics
#[derive(Debug, Clone, Default)]
struct SyscallStats {
    /// Counts per syscall number
    counts: BTreeMap<u32, u64>,
    /// Total syscalls
    total: u64,
    /// Last syscall timestamp
    last_timestamp: u64,
    /// Syscall rate (per second)
    rate: f64,
}

/// Suspicious syscall pattern definition
#[derive(Debug, Clone)]
pub struct SyscallPattern {
    /// Pattern name
    pub name: String,
    /// Syscall sequence
    pub sequence: Vec<u32>,
    /// Time window (nanoseconds)
    pub time_window: u64,
    /// Threat type if matched
    pub threat_type: ThreatType,
    /// Severity
    pub severity: ThreatSeverity,
}

impl SyscallMonitor {
    /// Create new syscall monitor
    pub fn new() -> Self {
        let mut monitor = Self {
            process_syscalls: BTreeMap::new(),
            suspicious_patterns: Vec::new(),
            recent_sequence: Vec::new(),
            max_sequence: 1000,
            total_syscalls: AtomicU64::new(0),
        };

        // Add default suspicious patterns
        monitor.add_default_patterns();
        monitor
    }

    /// Add default suspicious patterns
    fn add_default_patterns(&mut self) {
        // Privilege escalation pattern: setuid followed by exec
        self.suspicious_patterns.push(SyscallPattern {
            name: "privilege_escalation".into(),
            sequence: vec![105, 59],  // setuid, execve (Linux x86_64)
            time_window: 100_000_000, // 100ms
            threat_type: ThreatType::PrivilegeEscalation,
            severity: ThreatSeverity::High,
        });

        // Process injection pattern
        self.suspicious_patterns.push(SyscallPattern {
            name: "process_injection".into(),
            sequence: vec![101, 9, 10], // ptrace, mmap, mprotect
            time_window: 500_000_000,   // 500ms
            threat_type: ThreatType::ProcessInjection,
            severity: ThreatSeverity::High,
        });

        // ROP gadget pattern (many small reads)
        // Detected separately via rate analysis
    }

    /// Record syscall
    pub fn record(&mut self, process_id: u64, syscall_num: u32, timestamp: u64) {
        let stats = self.process_syscalls.entry(process_id).or_default();
        *stats.counts.entry(syscall_num).or_insert(0) += 1;
        stats.total += 1;

        // Update rate
        let time_delta = timestamp - stats.last_timestamp;
        if time_delta > 0 {
            let instant_rate = 1_000_000_000.0 / time_delta as f64;
            stats.rate = 0.9 * stats.rate + 0.1 * instant_rate;
        }
        stats.last_timestamp = timestamp;

        // Record in sequence
        self.recent_sequence
            .push((process_id, syscall_num, timestamp));
        if self.recent_sequence.len() > self.max_sequence {
            self.recent_sequence.remove(0);
        }

        self.total_syscalls.fetch_add(1, Ordering::Relaxed);
    }

    /// Check for pattern matches
    pub fn check_patterns(&self, process_id: u64) -> Vec<&SyscallPattern> {
        let mut matches = Vec::new();

        // Get recent syscalls for this process
        let process_sequence: Vec<_> = self
            .recent_sequence
            .iter()
            .filter(|(pid, _, _)| *pid == process_id)
            .collect();

        if process_sequence.is_empty() {
            return matches;
        }

        let now = process_sequence.last().map(|(_, _, t)| *t).unwrap_or(0);

        for pattern in &self.suspicious_patterns {
            // Check if pattern exists within time window
            let window_start = now.saturating_sub(pattern.time_window);
            let sequence_in_window: Vec<_> = process_sequence
                .iter()
                .filter(|(_, _, t)| *t >= window_start)
                .map(|(_, s, _)| *s)
                .collect();

            // Check if pattern is subsequence
            let mut pattern_idx = 0;
            for &syscall in &sequence_in_window {
                if pattern_idx < pattern.sequence.len() && syscall == pattern.sequence[pattern_idx]
                {
                    pattern_idx += 1;
                }
            }

            if pattern_idx == pattern.sequence.len() {
                matches.push(pattern);
            }
        }

        matches
    }

    /// Get syscall rate for process
    pub fn get_rate(&self, process_id: u64) -> f64 {
        self.process_syscalls
            .get(&process_id)
            .map(|s| s.rate)
            .unwrap_or(0.0)
    }

    /// Check for rate anomaly (DoS indicator)
    pub fn check_rate_anomaly(&self, process_id: u64, threshold: f64) -> bool {
        self.get_rate(process_id) > threshold
    }

    /// Get statistics
    pub fn stats(&self) -> SyscallMonitorStats {
        SyscallMonitorStats {
            total_syscalls: self.total_syscalls.load(Ordering::Relaxed),
            monitored_processes: self.process_syscalls.len(),
            pattern_count: self.suspicious_patterns.len(),
        }
    }

    /// Add custom pattern
    pub fn add_pattern(&mut self, pattern: SyscallPattern) {
        self.suspicious_patterns.push(pattern);
    }
}

impl Default for SyscallMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Syscall monitor statistics
#[derive(Debug, Clone)]
pub struct SyscallMonitorStats {
    /// Total syscalls monitored
    pub total_syscalls: u64,
    /// Number of monitored processes
    pub monitored_processes: usize,
    /// Number of patterns
    pub pattern_count: usize,
}
