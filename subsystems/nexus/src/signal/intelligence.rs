//! Signal Intelligence
//!
//! Comprehensive signal analysis and optimization engine.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    DeliveryOptimizer, DeliveryState, HandlerProfiler, PatternType, PendingSignal, ProcessId,
    QueueStats, SignalAction, SignalAnalysis, SignalInfo, SignalIssue, SignalIssueType,
    SignalNumber, SignalPatternDetector, SignalQueueManager, SignalRecommendation, ThreadId,
};

/// Signal Intelligence - comprehensive signal analysis and optimization
pub struct SignalIntelligence {
    /// Pattern detector
    pattern_detector: SignalPatternDetector,
    /// Handler profiler
    handler_profiler: HandlerProfiler,
    /// Queue manager
    queue_manager: SignalQueueManager,
    /// Delivery optimizer
    delivery_optimizer: DeliveryOptimizer,
    /// Per-process signal masks
    signal_masks: BTreeMap<ProcessId, u64>,
    /// Total signals sent
    total_sent: AtomicU64,
    /// Total signals delivered
    total_delivered: AtomicU64,
    /// Analysis interval
    analysis_interval_ns: u64,
    /// Last analysis timestamp
    last_analysis_ns: u64,
}

impl SignalIntelligence {
    /// Create new signal intelligence
    pub fn new() -> Self {
        Self {
            pattern_detector: SignalPatternDetector::new(),
            handler_profiler: HandlerProfiler::new(),
            queue_manager: SignalQueueManager::new(64),
            delivery_optimizer: DeliveryOptimizer::new(),
            signal_masks: BTreeMap::new(),
            total_sent: AtomicU64::new(0),
            total_delivered: AtomicU64::new(0),
            analysis_interval_ns: 1_000_000_000, // 1 second
            last_analysis_ns: 0,
        }
    }

    /// Create with custom queue capacity
    #[inline]
    pub fn with_queue_capacity(capacity: usize) -> Self {
        Self {
            queue_manager: SignalQueueManager::new(capacity),
            ..Self::new()
        }
    }

    /// Send signal
    pub fn send_signal(
        &mut self,
        sender: ProcessId,
        receiver: ProcessId,
        signo: SignalNumber,
        timestamp: u64,
    ) -> bool {
        self.total_sent.fetch_add(1, Ordering::Relaxed);

        // Record for pattern detection
        self.pattern_detector
            .record_event(signo, sender, receiver, timestamp);

        // Check if signal is masked
        let mask = self.signal_masks.get(&receiver).copied().unwrap_or(0);
        if (mask & (1 << signo.raw())) != 0 && signo.can_catch() {
            // Signal is blocked
            let info = SignalInfo::new(signo, sender, timestamp);
            let mut pending = PendingSignal::new(info, receiver);
            pending.state = DeliveryState::Blocked;
            return self.queue_manager.enqueue(pending);
        }

        // Enqueue for delivery
        let info = SignalInfo::new(signo, sender, timestamp);
        let pending = PendingSignal::new(info, receiver);
        self.queue_manager.enqueue(pending)
    }

    /// Deliver signal to process
    #[inline]
    pub fn deliver_signal(&mut self, pid: ProcessId) -> Option<PendingSignal> {
        let signal = self.queue_manager.dequeue(pid)?;
        self.total_delivered.fetch_add(1, Ordering::Relaxed);

        // Record handler entry
        self.handler_profiler.record_entry(pid, signal.info.signo);

        Some(signal)
    }

    /// Complete signal delivery
    #[inline]
    pub fn complete_delivery(
        &mut self,
        pid: ProcessId,
        signo: SignalNumber,
        duration_ns: u64,
        failed: bool,
        timestamp: u64,
    ) {
        self.handler_profiler
            .record_exit(pid, signo, duration_ns, failed, timestamp);
        self.queue_manager.mark_delivered(pid, signo);
    }

    /// Set signal mask for process
    #[inline(always)]
    pub fn set_signal_mask(&mut self, pid: ProcessId, mask: u64) {
        self.signal_masks.insert(pid, mask);
    }

    /// Get signal mask for process
    #[inline(always)]
    pub fn get_signal_mask(&self, pid: ProcessId) -> u64 {
        self.signal_masks.get(&pid).copied().unwrap_or(0)
    }

    /// Block signal for process
    #[inline(always)]
    pub fn block_signal(&mut self, pid: ProcessId, signo: SignalNumber) {
        let mask = self.signal_masks.entry(pid).or_default();
        *mask |= 1 << signo.raw();
    }

    /// Unblock signal for process
    #[inline]
    pub fn unblock_signal(&mut self, pid: ProcessId, signo: SignalNumber) {
        if let Some(mask) = self.signal_masks.get_mut(&pid) {
            *mask &= !(1 << signo.raw());
        }
    }

    /// Check if signal is blocked
    #[inline(always)]
    pub fn is_blocked(&self, pid: ProcessId, signo: SignalNumber) -> bool {
        let mask = self.signal_masks.get(&pid).copied().unwrap_or(0);
        (mask & (1 << signo.raw())) != 0
    }

    /// Register thread for delivery optimization
    #[inline(always)]
    pub fn register_thread(&mut self, tid: ThreadId) {
        self.delivery_optimizer.register_thread(tid);
    }

    /// Analyze process signal handling
    pub fn analyze_process(&self, pid: ProcessId) -> SignalAnalysis {
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();

        // Check queue stats
        if let Some(stats) = self.queue_manager.process_stats(pid) {
            if stats.dropped > 0 {
                health_score -= 20.0;
                issues.push(SignalIssue {
                    issue_type: SignalIssueType::QueueOverflow,
                    severity: 7,
                    description: String::from("Signals dropped due to queue overflow"),
                    signal: None,
                });
            }
        }

        // Check for problematic handlers
        let problematic = self.handler_profiler.get_problematic_handlers();
        for (signo, stats) in &problematic {
            if stats.avg_time_ns() > 10_000_000 {
                // 10ms
                health_score -= 15.0;
                issues.push(SignalIssue {
                    issue_type: SignalIssueType::SlowHandler,
                    severity: 5,
                    description: String::from("Slow signal handler detected"),
                    signal: Some(*signo),
                });
            }
            if stats.unsafe_calls > 0 {
                health_score -= 25.0;
                issues.push(SignalIssue {
                    issue_type: SignalIssueType::UnsafeHandler,
                    severity: 8,
                    description: String::from("Async-signal-unsafe calls in handler"),
                    signal: Some(*signo),
                });
            }
            if stats.nested_handlers > 0 {
                health_score -= 10.0;
                issues.push(SignalIssue {
                    issue_type: SignalIssueType::NestedHandlers,
                    severity: 6,
                    description: String::from("Nested signal handlers detected"),
                    signal: Some(*signo),
                });
            }
        }

        // Check for patterns involving this process
        let patterns: Vec<_> = self
            .pattern_detector
            .get_patterns()
            .iter()
            .filter(|p| p.processes.contains(&pid))
            .cloned()
            .collect();

        for pattern in &patterns {
            if pattern.pattern_type == PatternType::Storm {
                health_score -= 30.0;
                issues.push(SignalIssue {
                    issue_type: SignalIssueType::Storm,
                    severity: 9,
                    description: String::from("Signal storm detected"),
                    signal: None,
                });
            }
        }

        health_score = health_score.max(0.0);

        let recommendations = self.generate_recommendations(&issues);

        SignalAnalysis {
            pid,
            health_score,
            issues,
            patterns,
            recommendations,
        }
    }

    /// Generate recommendations based on issues
    fn generate_recommendations(&self, issues: &[SignalIssue]) -> Vec<SignalRecommendation> {
        let mut recommendations = Vec::new();

        for issue in issues {
            match issue.issue_type {
                SignalIssueType::Storm => {
                    recommendations.push(SignalRecommendation {
                        action: SignalAction::Blocked,
                        signal: issue.signal,
                        expected_improvement: 40.0,
                        reason: String::from("Temporarily block storm signals"),
                    });
                }
                SignalIssueType::SlowHandler => {
                    recommendations.push(SignalRecommendation {
                        action: SignalAction::SigAction,
                        signal: issue.signal,
                        expected_improvement: 20.0,
                        reason: String::from("Optimize signal handler or use SA_NODEFER"),
                    });
                }
                SignalIssueType::UnsafeHandler => {
                    recommendations.push(SignalRecommendation {
                        action: SignalAction::SigAction,
                        signal: issue.signal,
                        expected_improvement: 35.0,
                        reason: String::from("Remove async-signal-unsafe calls from handler"),
                    });
                }
                SignalIssueType::QueueOverflow => {
                    recommendations.push(SignalRecommendation {
                        action: SignalAction::Handler,
                        signal: None,
                        expected_improvement: 25.0,
                        reason: String::from("Process signals faster or increase queue size"),
                    });
                }
                _ => {}
            }
        }

        recommendations
    }

    /// Get pattern detector
    #[inline(always)]
    pub fn pattern_detector(&self) -> &SignalPatternDetector {
        &self.pattern_detector
    }

    /// Get pattern detector mutably
    #[inline(always)]
    pub fn pattern_detector_mut(&mut self) -> &mut SignalPatternDetector {
        &mut self.pattern_detector
    }

    /// Get handler profiler
    #[inline(always)]
    pub fn handler_profiler(&self) -> &HandlerProfiler {
        &self.handler_profiler
    }

    /// Get handler profiler mutably
    #[inline(always)]
    pub fn handler_profiler_mut(&mut self) -> &mut HandlerProfiler {
        &mut self.handler_profiler
    }

    /// Get queue manager
    #[inline(always)]
    pub fn queue_manager(&self) -> &SignalQueueManager {
        &self.queue_manager
    }

    /// Get queue manager mutably
    #[inline(always)]
    pub fn queue_manager_mut(&mut self) -> &mut SignalQueueManager {
        &mut self.queue_manager
    }

    /// Get delivery optimizer
    #[inline(always)]
    pub fn delivery_optimizer(&self) -> &DeliveryOptimizer {
        &self.delivery_optimizer
    }

    /// Get delivery optimizer mutably
    #[inline(always)]
    pub fn delivery_optimizer_mut(&mut self) -> &mut DeliveryOptimizer {
        &mut self.delivery_optimizer
    }

    /// Get total signals sent
    #[inline(always)]
    pub fn total_sent(&self) -> u64 {
        self.total_sent.load(Ordering::Relaxed)
    }

    /// Get total signals delivered
    #[inline(always)]
    pub fn total_delivered(&self) -> u64 {
        self.total_delivered.load(Ordering::Relaxed)
    }

    /// Get queue stats
    #[inline(always)]
    pub fn queue_stats(&self) -> &QueueStats {
        self.queue_manager.global_stats()
    }

    /// Perform periodic maintenance
    #[inline]
    pub fn periodic_maintenance(&mut self, current_time_ns: u64) {
        if current_time_ns - self.last_analysis_ns < self.analysis_interval_ns {
            return;
        }
        self.last_analysis_ns = current_time_ns;

        // Cleanup old patterns
        self.pattern_detector
            .cleanup(60_000_000_000, current_time_ns); // 1 minute
    }

    /// Set analysis interval
    #[inline(always)]
    pub fn set_analysis_interval(&mut self, interval_ns: u64) {
        self.analysis_interval_ns = interval_ns;
    }
}

impl Default for SignalIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
