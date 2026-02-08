//! # Holistic System Health Engine
//!
//! System-wide health scoring and monitoring:
//! - Multi-dimensional health scores
//! - Health degradation detection
//! - Root cause analysis
//! - System vitals tracking
//! - Health report generation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// HEALTH TYPES
// ============================================================================

/// Health dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthDimension {
    /// CPU subsystem
    Cpu,
    /// Memory subsystem
    Memory,
    /// I/O subsystem
    Io,
    /// Network subsystem
    Network,
    /// Scheduler
    Scheduler,
    /// Power management
    Power,
    /// Thermal
    Thermal,
    /// Storage
    Storage,
    /// Process management
    ProcessMgmt,
    /// IPC
    Ipc,
}

/// Health level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthLevel {
    /// Optimal (90-100%)
    Optimal,
    /// Good (75-90%)
    Good,
    /// Fair (50-75%)
    Fair,
    /// Degraded (25-50%)
    Degraded,
    /// Critical (0-25%)
    Critical,
}

impl HealthLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.90 {
            Self::Optimal
        } else if score >= 0.75 {
            Self::Good
        } else if score >= 0.50 {
            Self::Fair
        } else if score >= 0.25 {
            Self::Degraded
        } else {
            Self::Critical
        }
    }

    /// Is degraded or worse?
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, Self::Degraded | Self::Critical)
    }
}

// ============================================================================
// HEALTH METRICS
// ============================================================================

/// Vital sign for a dimension
#[derive(Debug, Clone)]
pub struct VitalSign {
    /// Dimension
    pub dimension: HealthDimension,
    /// Current score (0.0-1.0)
    pub score: f64,
    /// Historical scores
    history: Vec<f64>,
    /// Max history
    max_history: usize,
    /// EMA smoothed score
    ema_score: f64,
    /// Alpha for EMA
    alpha: f64,
    /// Trend
    pub trend: f64,
}

impl VitalSign {
    pub fn new(dimension: HealthDimension) -> Self {
        Self {
            dimension,
            score: 1.0,
            history: Vec::new(),
            max_history: 256,
            ema_score: 1.0,
            alpha: 0.2,
            trend: 0.0,
        }
    }

    /// Update score
    pub fn update(&mut self, raw_score: f64) {
        let clamped = if raw_score < 0.0 {
            0.0
        } else if raw_score > 1.0 {
            1.0
        } else {
            raw_score
        };

        let prev_ema = self.ema_score;
        self.ema_score = self.alpha * clamped + (1.0 - self.alpha) * self.ema_score;
        self.trend = self.ema_score - prev_ema;
        self.score = self.ema_score;

        self.history.push(clamped);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Health level
    pub fn level(&self) -> HealthLevel {
        HealthLevel::from_score(self.score)
    }

    /// Is degrading?
    pub fn is_degrading(&self) -> bool {
        self.trend < -0.01
    }

    /// Average over last N samples
    pub fn average_recent(&self, n: usize) -> f64 {
        if self.history.is_empty() {
            return self.score;
        }
        let start = if self.history.len() > n {
            self.history.len() - n
        } else {
            0
        };
        let slice = &self.history[start..];
        slice.iter().sum::<f64>() / slice.len() as f64
    }

    /// Min over last N
    pub fn min_recent(&self, n: usize) -> f64 {
        if self.history.is_empty() {
            return self.score;
        }
        let start = if self.history.len() > n {
            self.history.len() - n
        } else {
            0
        };
        self.history[start..]
            .iter()
            .copied()
            .fold(f64::INFINITY, |a, b| if a < b { a } else { b })
    }

    /// Volatility (std dev over recent)
    pub fn volatility(&self, n: usize) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        let start = if self.history.len() > n {
            self.history.len() - n
        } else {
            0
        };
        let slice = &self.history[start..];
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let var: f64 = slice.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / slice.len() as f64;
        libm::sqrt(var)
    }
}

// ============================================================================
// HEALTH ISSUE
// ============================================================================

/// Detected health issue
#[derive(Debug, Clone)]
pub struct HealthIssue {
    /// Dimension
    pub dimension: HealthDimension,
    /// Severity
    pub level: HealthLevel,
    /// Score at detection
    pub score: f64,
    /// Trend at detection
    pub trend: f64,
    /// Possible causes
    pub possible_causes: Vec<HealthCause>,
    /// Timestamp
    pub detected_at: u64,
}

/// Possible cause
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCause {
    /// Overload
    Overload,
    /// Resource leak
    ResourceLeak,
    /// Contention
    Contention,
    /// Hardware degradation
    HardwareDegradation,
    /// Configuration issue
    Configuration,
    /// External pressure
    ExternalPressure,
    /// Cascading failure
    CascadingFailure,
    /// Unknown
    Unknown,
}

// ============================================================================
// HEALTH REPORT
// ============================================================================

/// System health report
#[derive(Debug, Clone)]
pub struct HealthReport {
    /// Overall score
    pub overall_score: f64,
    /// Overall level
    pub overall_level: HealthLevel,
    /// Per-dimension scores
    pub dimension_scores: BTreeMap<u8, (HealthDimension, f64, HealthLevel)>,
    /// Active issues
    pub issues: Vec<HealthIssue>,
    /// Timestamp
    pub generated_at: u64,
    /// Degrading dimensions
    pub degrading_count: usize,
}

// ============================================================================
// HEALTH ENGINE
// ============================================================================

/// Health engine stats
#[derive(Debug, Clone, Default)]
pub struct HolisticHealthStats {
    /// Dimensions monitored
    pub dimensions: usize,
    /// Current overall score
    pub overall_score: f64,
    /// Active issues
    pub active_issues: usize,
    /// Reports generated
    pub reports_generated: u64,
    /// Degrading dimensions
    pub degrading_count: usize,
}

/// Holistic health engine
pub struct HolisticHealthEngine {
    /// Vital signs per dimension
    vitals: BTreeMap<u8, VitalSign>,
    /// Issue history
    issues: Vec<HealthIssue>,
    /// Max issues
    max_issues: usize,
    /// Dimension weights for overall score
    weights: BTreeMap<u8, f64>,
    /// Stats
    stats: HolisticHealthStats,
}

impl HolisticHealthEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            vitals: BTreeMap::new(),
            issues: Vec::new(),
            max_issues: 512,
            weights: BTreeMap::new(),
            stats: HolisticHealthStats::default(),
        };

        // Initialize all dimensions
        let dimensions = [
            HealthDimension::Cpu,
            HealthDimension::Memory,
            HealthDimension::Io,
            HealthDimension::Network,
            HealthDimension::Scheduler,
            HealthDimension::Power,
            HealthDimension::Thermal,
            HealthDimension::Storage,
            HealthDimension::ProcessMgmt,
            HealthDimension::Ipc,
        ];

        for dim in &dimensions {
            engine
                .vitals
                .insert(*dim as u8, VitalSign::new(*dim));
            engine.weights.insert(*dim as u8, 1.0);
        }
        // Higher weights for critical dimensions
        engine.weights.insert(HealthDimension::Cpu as u8, 1.5);
        engine.weights.insert(HealthDimension::Memory as u8, 1.5);
        engine.weights.insert(HealthDimension::Scheduler as u8, 1.2);

        engine.stats.dimensions = engine.vitals.len();
        engine
    }

    /// Update a dimension's health
    pub fn update(&mut self, dimension: HealthDimension, score: f64) {
        if let Some(vital) = self.vitals.get_mut(&(dimension as u8)) {
            vital.update(score);
        }
        self.recalculate_overall();
    }

    /// Get vital for dimension
    pub fn vital(&self, dimension: HealthDimension) -> Option<&VitalSign> {
        self.vitals.get(&(dimension as u8))
    }

    /// Overall score (weighted average)
    fn recalculate_overall(&mut self) {
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;
        let mut degrading = 0;

        for (key, vital) in &self.vitals {
            let w = self.weights.get(key).copied().unwrap_or(1.0);
            weighted_sum += vital.score * w;
            total_weight += w;
            if vital.is_degrading() {
                degrading += 1;
            }
        }

        self.stats.overall_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            1.0
        };
        self.stats.degrading_count = degrading;
    }

    /// Detect issues
    pub fn detect_issues(&mut self, now: u64) -> Vec<HealthIssue> {
        let mut new_issues = Vec::new();

        let vitals_snapshot: Vec<(HealthDimension, f64, f64, f64)> = self
            .vitals
            .values()
            .map(|v| (v.dimension, v.score, v.trend, v.volatility(20)))
            .collect();

        for (dim, score, trend, volatility) in vitals_snapshot {
            let level = HealthLevel::from_score(score);
            if level.is_unhealthy() {
                let mut causes = Vec::new();

                // Heuristic cause analysis
                if score < 0.15 && trend < -0.05 {
                    causes.push(HealthCause::CascadingFailure);
                } else if trend < -0.03 {
                    causes.push(HealthCause::ResourceLeak);
                }

                if volatility > 0.2 {
                    causes.push(HealthCause::Contention);
                }

                if score < 0.3 {
                    causes.push(HealthCause::Overload);
                }

                if causes.is_empty() {
                    causes.push(HealthCause::Unknown);
                }

                let issue = HealthIssue {
                    dimension: dim,
                    level,
                    score,
                    trend,
                    possible_causes: causes,
                    detected_at: now,
                };
                new_issues.push(issue);
            }
        }

        self.issues.extend(new_issues.clone());
        if self.issues.len() > self.max_issues {
            let drain = self.issues.len() - self.max_issues;
            self.issues.drain(..drain);
        }
        self.stats.active_issues = new_issues.len();

        new_issues
    }

    /// Generate health report
    pub fn generate_report(&mut self, now: u64) -> HealthReport {
        let issues = self.detect_issues(now);

        let mut dimension_scores = BTreeMap::new();
        for vital in self.vitals.values() {
            dimension_scores.insert(
                vital.dimension as u8,
                (vital.dimension, vital.score, vital.level()),
            );
        }

        let overall_score = self.stats.overall_score;
        let degrading_count = self.stats.degrading_count;

        self.stats.reports_generated += 1;

        HealthReport {
            overall_score,
            overall_level: HealthLevel::from_score(overall_score),
            dimension_scores,
            issues,
            generated_at: now,
            degrading_count,
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticHealthStats {
        &self.stats
    }
}
