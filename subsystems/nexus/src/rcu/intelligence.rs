//! RCU Intelligence
//!
//! This module provides comprehensive RCU analysis and optimization.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    RcuDomainId, CpuId, GracePeriodId,
    RcuFlavor, RcuDomainState,
    GracePeriodInfo, GracePeriodStats,
    GracePeriodPredictor,
    CallbackInfo, CallbackCoalescer,
    ReaderTracker,
    MemoryPressureLevel, MemoryPressureSample, MemoryPressureAnalyzer,
    ConfigRecommendation, AdaptiveConfigurator,
};

/// RCU domain information
#[derive(Debug, Clone)]
pub struct RcuDomainInfo {
    /// Domain ID
    pub id: RcuDomainId,
    /// Domain name
    pub name: String,
    /// RCU flavor
    pub flavor: RcuFlavor,
    /// Current state
    pub state: RcuDomainState,
    /// Grace period stats
    pub gp_stats: GracePeriodStats,
    /// Online CPU count
    pub cpu_count: u32,
    /// Pending callbacks
    pub pending_callbacks: u64,
    /// Pending memory (bytes)
    pub pending_memory: u64,
}

/// RCU analysis result
#[derive(Debug, Clone)]
pub struct RcuAnalysis {
    /// Domain ID
    pub domain_id: RcuDomainId,
    /// Health score (0-100)
    pub health_score: f32,
    /// Memory pressure level
    pub pressure_level: MemoryPressureLevel,
    /// Detected issues
    pub issues: Vec<RcuIssue>,
    /// Configuration recommendations
    pub recommendations: Vec<ConfigRecommendation>,
}

/// RCU issue type
#[derive(Debug, Clone)]
pub struct RcuIssue {
    /// Issue type
    pub issue_type: RcuIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Recommended action
    pub action: Option<String>,
}

/// RCU issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuIssueType {
    /// Long grace periods
    LongGracePeriods,
    /// High callback backlog
    HighCallbackBacklog,
    /// Memory pressure
    MemoryPressure,
    /// RCU stall
    Stall,
    /// Long critical sections
    LongCriticalSections,
    /// High expedited rate
    HighExpeditedRate,
    /// CPU imbalance
    CpuImbalance,
}

/// RCU Intelligence - comprehensive RCU analysis and optimization
pub struct RcuIntelligence {
    /// Registered domains
    domains: BTreeMap<RcuDomainId, RcuDomainInfo>,
    /// Grace period predictors per domain
    gp_predictors: BTreeMap<RcuDomainId, GracePeriodPredictor>,
    /// Callback coalescers per domain
    callback_coalescers: BTreeMap<RcuDomainId, CallbackCoalescer>,
    /// Reader tracker
    reader_tracker: ReaderTracker,
    /// Memory pressure analyzers per domain
    pressure_analyzers: BTreeMap<RcuDomainId, MemoryPressureAnalyzer>,
    /// Adaptive configurator
    configurator: AdaptiveConfigurator,
    /// Current grace periods
    current_gps: BTreeMap<RcuDomainId, GracePeriodInfo>,
    /// Total callbacks processed
    total_callbacks_processed: AtomicU64,
    /// Total grace periods completed
    total_gps_completed: AtomicU64,
    /// Analysis interval
    analysis_interval_ns: u64,
    /// Last analysis timestamp
    last_analysis_ns: u64,
}

impl RcuIntelligence {
    /// Create new RCU intelligence
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            gp_predictors: BTreeMap::new(),
            callback_coalescers: BTreeMap::new(),
            reader_tracker: ReaderTracker::new(),
            pressure_analyzers: BTreeMap::new(),
            configurator: AdaptiveConfigurator::new(),
            current_gps: BTreeMap::new(),
            total_callbacks_processed: AtomicU64::new(0),
            total_gps_completed: AtomicU64::new(0),
            analysis_interval_ns: 1_000_000_000, // 1 second
            last_analysis_ns: 0,
        }
    }

    /// Register RCU domain
    pub fn register_domain(&mut self, id: RcuDomainId, name: String, flavor: RcuFlavor) {
        let info = RcuDomainInfo {
            id,
            name,
            flavor,
            state: RcuDomainState::Idle,
            gp_stats: GracePeriodStats::default(),
            cpu_count: 0,
            pending_callbacks: 0,
            pending_memory: 0,
        };
        self.domains.insert(id, info);
        self.gp_predictors.insert(id, GracePeriodPredictor::new(id));
        self.callback_coalescers
            .insert(id, CallbackCoalescer::new());
        self.pressure_analyzers
            .insert(id, MemoryPressureAnalyzer::new(id));
    }

    /// Register CPU
    pub fn register_cpu(&mut self, cpu_id: CpuId) {
        self.reader_tracker.register_cpu(cpu_id);
        for domain in self.domains.values_mut() {
            domain.cpu_count += 1;
        }
    }

    /// Start grace period
    pub fn start_grace_period(
        &mut self,
        domain_id: RcuDomainId,
        gp_id: GracePeriodId,
        timestamp_ns: u64,
        expedited: bool,
    ) {
        let mut gp = GracePeriodInfo::new(gp_id, domain_id, timestamp_ns);
        gp.expedited = expedited;

        // Get all online CPUs as pending
        for (cpu_id, _) in self.reader_tracker.readers.iter() {
            gp.cpus_pending.push(*cpu_id);
        }

        self.current_gps.insert(domain_id, gp);

        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.state = if expedited {
                RcuDomainState::Expedited
            } else {
                RcuDomainState::Active
            };
            domain.gp_stats.current_gp = Some(gp_id);
        }
    }

    /// Record quiescent state for CPU
    pub fn record_quiescent_state(
        &mut self,
        domain_id: RcuDomainId,
        cpu_id: CpuId,
        timestamp_ns: u64,
    ) {
        self.reader_tracker.record_qs(cpu_id, timestamp_ns);

        if let Some(gp) = self.current_gps.get_mut(&domain_id) {
            gp.cpus_pending.retain(|c| *c != cpu_id);
            if !gp.cpus_qs.contains(&cpu_id) {
                gp.cpus_qs.push(cpu_id);
            }
        }
    }

    /// Complete grace period
    pub fn complete_grace_period(&mut self, domain_id: RcuDomainId, timestamp_ns: u64) {
        if let Some(mut gp) = self.current_gps.remove(&domain_id) {
            gp.end_ns = Some(timestamp_ns);

            // Update predictor
            if let Some(predictor) = self.gp_predictors.get_mut(&domain_id) {
                if let Some(domain) = self.domains.get(&domain_id) {
                    predictor.record_sample(&gp, domain.cpu_count, domain.pending_callbacks);
                }
            }

            // Update domain stats
            if let Some(domain) = self.domains.get_mut(&domain_id) {
                domain.state = RcuDomainState::Idle;
                domain.gp_stats.total_completed += 1;
                domain.gp_stats.current_gp = None;

                if gp.expedited {
                    domain.gp_stats.expedited_count += 1;
                }
                if gp.forced {
                    domain.gp_stats.forced_count += 1;
                }

                if let Some(duration) = gp.duration_ns() {
                    if domain.gp_stats.min_duration_ns == 0
                        || duration < domain.gp_stats.min_duration_ns
                    {
                        domain.gp_stats.min_duration_ns = duration;
                    }
                    if duration > domain.gp_stats.max_duration_ns {
                        domain.gp_stats.max_duration_ns = duration;
                    }
                    // Update average with exponential smoothing
                    let alpha = 0.1;
                    domain.gp_stats.avg_duration_ns = (alpha * duration as f64
                        + (1.0 - alpha) * domain.gp_stats.avg_duration_ns as f64)
                        as u64;
                }
            }

            self.total_gps_completed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Register callback
    pub fn register_callback(&mut self, domain_id: RcuDomainId, callback: CallbackInfo) {
        if let Some(coalescer) = self.callback_coalescers.get_mut(&domain_id) {
            coalescer.add_callback(callback);
        }
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.pending_callbacks += 1;
        }
    }

    /// Process callbacks
    pub fn process_callbacks(&mut self, domain_id: RcuDomainId, count: u64) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.pending_callbacks = domain.pending_callbacks.saturating_sub(count);
        }
        self.total_callbacks_processed
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Record critical section entry
    pub fn record_cs_entry(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        self.reader_tracker.record_cs_entry(cpu_id, timestamp_ns);
    }

    /// Record critical section exit
    pub fn record_cs_exit(&mut self, cpu_id: CpuId, timestamp_ns: u64) {
        self.reader_tracker.record_cs_exit(cpu_id, timestamp_ns);
    }

    /// Update memory pressure
    pub fn update_pressure(
        &mut self,
        domain_id: RcuDomainId,
        timestamp_ns: u64,
        pending_callbacks: u64,
        pending_memory: u64,
        gp_rate: f32,
        callback_rate: f32,
    ) {
        let sample = MemoryPressureSample {
            timestamp_ns,
            pending_callbacks,
            pending_memory_bytes: pending_memory,
            gp_rate,
            callback_rate,
        };

        if let Some(analyzer) = self.pressure_analyzers.get_mut(&domain_id) {
            analyzer.record_sample(sample);
        }

        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.pending_callbacks = pending_callbacks;
            domain.pending_memory = pending_memory;
        }
    }

    /// Analyze domain
    pub fn analyze_domain(
        &mut self,
        domain_id: RcuDomainId,
        current_time_ns: u64,
    ) -> Option<RcuAnalysis> {
        let domain = self.domains.get(&domain_id)?;
        let pressure_analyzer = self.pressure_analyzers.get(&domain_id)?;

        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let pressure_level = pressure_analyzer.current_level();

        // Check memory pressure
        match pressure_level {
            MemoryPressureLevel::Critical => {
                health_score -= 40.0;
                issues.push(RcuIssue {
                    issue_type: RcuIssueType::MemoryPressure,
                    severity: 10,
                    description: String::from("Critical memory pressure"),
                    action: Some(String::from("Force expedited grace periods")),
                });
            },
            MemoryPressureLevel::High => {
                health_score -= 25.0;
                issues.push(RcuIssue {
                    issue_type: RcuIssueType::MemoryPressure,
                    severity: 7,
                    description: String::from("High memory pressure"),
                    action: Some(String::from("Consider expedited grace periods")),
                });
            },
            MemoryPressureLevel::Medium => {
                health_score -= 10.0;
            },
            _ => {},
        }

        // Check grace period duration
        if domain.gp_stats.avg_duration_ns > 100_000_000 {
            // > 100ms
            health_score -= 20.0;
            issues.push(RcuIssue {
                issue_type: RcuIssueType::LongGracePeriods,
                severity: 6,
                description: String::from("Long grace period durations"),
                action: Some(String::from("Check for blocked readers")),
            });
        }

        // Check for stalls
        if domain.gp_stats.stall_count > 0 {
            health_score -= 30.0;
            issues.push(RcuIssue {
                issue_type: RcuIssueType::Stall,
                severity: 9,
                description: String::from("RCU stalls detected"),
                action: Some(String::from("Investigate blocking readers")),
            });
        }

        // Check long critical sections
        let long_cs = self.reader_tracker.long_cs_count();
        if long_cs > 0 {
            health_score -= (long_cs as f32).min(20.0);
            issues.push(RcuIssue {
                issue_type: RcuIssueType::LongCriticalSections,
                severity: 5,
                description: String::from("Long RCU read-side critical sections"),
                action: Some(String::from("Review critical section code")),
            });
        }

        // Check expedited rate
        if domain.gp_stats.total_completed > 0 {
            let expedited_rate =
                domain.gp_stats.expedited_count as f32 / domain.gp_stats.total_completed as f32;
            if expedited_rate > 0.5 {
                health_score -= 10.0;
                issues.push(RcuIssue {
                    issue_type: RcuIssueType::HighExpeditedRate,
                    severity: 4,
                    description: String::from("High expedited grace period rate"),
                    action: Some(String::from("Review callback registration patterns")),
                });
            }
        }

        health_score = health_score.max(0.0);

        // Get configuration recommendations
        self.configurator.analyze(
            &domain.gp_stats,
            pressure_level,
            domain.pending_callbacks,
            current_time_ns,
        );
        let recommendations = self.configurator.recommendations().to_vec();

        Some(RcuAnalysis {
            domain_id,
            health_score,
            pressure_level,
            issues,
            recommendations,
        })
    }

    /// Predict grace period duration
    pub fn predict_gp_duration(&self, domain_id: RcuDomainId, expedited: bool) -> Option<u64> {
        let domain = self.domains.get(&domain_id)?;
        let predictor = self.gp_predictors.get(&domain_id)?;
        Some(predictor.predict_duration(expedited, domain.cpu_count))
    }

    /// Get domain info
    pub fn get_domain(&self, domain_id: RcuDomainId) -> Option<&RcuDomainInfo> {
        self.domains.get(&domain_id)
    }

    /// Get configurator
    pub fn configurator(&self) -> &AdaptiveConfigurator {
        &self.configurator
    }

    /// Get configurator mutably
    pub fn configurator_mut(&mut self) -> &mut AdaptiveConfigurator {
        &mut self.configurator
    }

    /// Get reader tracker
    pub fn reader_tracker(&self) -> &ReaderTracker {
        &self.reader_tracker
    }

    /// Get reader tracker mutably
    pub fn reader_tracker_mut(&mut self) -> &mut ReaderTracker {
        &mut self.reader_tracker
    }

    /// Get total callbacks processed
    pub fn total_callbacks_processed(&self) -> u64 {
        self.total_callbacks_processed.load(Ordering::Relaxed)
    }

    /// Get total grace periods completed
    pub fn total_gps_completed(&self) -> u64 {
        self.total_gps_completed.load(Ordering::Relaxed)
    }

    /// Get domain count
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Perform periodic maintenance
    pub fn periodic_maintenance(&mut self, current_time_ns: u64) {
        if current_time_ns - self.last_analysis_ns < self.analysis_interval_ns {
            return;
        }
        self.last_analysis_ns = current_time_ns;

        // Flush any pending callback batches
        for (domain_id, coalescer) in &mut self.callback_coalescers {
            if coalescer.should_flush(current_time_ns) {
                if let Some(gp) = self.current_gps.get(domain_id) {
                    let _ = coalescer.flush_batch(gp.id, current_time_ns);
                }
            }
        }
    }
}

impl Default for RcuIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
