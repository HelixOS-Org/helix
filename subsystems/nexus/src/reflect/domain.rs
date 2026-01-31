//! Reflect Domain — Orchestrator
//!
//! The ReflectDomain is the meta-cognitive layer that observes all
//! other domains, diagnoses issues, and drives continuous improvement.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::calibrator::{Calibrator, CalibratorStats};
use super::diagnostician::{CognitiveFailure, Diagnostician, DiagnosticianStats, FailureId};
use super::evolver::{Evolver, EvolverStats};
use super::insight::{Insight, InsightId, InsightType};
use super::introspector::{Introspector, IntrospectorStats};
use super::metrics::DomainMetrics;
use crate::bus::Domain;
use crate::types::*;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for Reflect domain
#[derive(Debug, Clone)]
pub struct ReflectConfig {
    /// Introspection interval (ticks)
    pub introspection_interval: u64,
    /// Maximum history per domain
    pub max_history: usize,
    /// Maximum failures to track
    pub max_failures: usize,
    /// Maximum suggestions
    pub max_suggestions: usize,
}

impl Default for ReflectConfig {
    fn default() -> Self {
        Self {
            introspection_interval: 100,
            max_history: 1000,
            max_failures: 1000,
            max_suggestions: 1000,
        }
    }
}

impl ReflectConfig {
    /// Create minimal config
    pub fn minimal() -> Self {
        Self {
            introspection_interval: 50,
            max_history: 100,
            max_failures: 100,
            max_suggestions: 100,
        }
    }

    /// Create detailed config
    pub fn detailed() -> Self {
        Self {
            introspection_interval: 10,
            max_history: 10000,
            max_failures: 10000,
            max_suggestions: 5000,
        }
    }
}

// ============================================================================
// REFLECT DOMAIN
// ============================================================================

/// The Reflect domain - meta-cognitive layer
pub struct ReflectDomain {
    /// Domain ID
    id: DomainId,
    /// Configuration
    config: ReflectConfig,
    /// Is running
    running: AtomicBool,
    /// Introspector
    introspector: Introspector,
    /// Calibrator
    calibrator: Calibrator,
    /// Diagnostician
    diagnostician: Diagnostician,
    /// Evolver
    evolver: Evolver,
    /// Total ticks
    total_ticks: AtomicU64,
    /// Insights generated
    insights_generated: AtomicU64,
}

impl ReflectDomain {
    /// Create new Reflect domain
    pub fn new(config: ReflectConfig) -> Self {
        Self {
            id: DomainId::generate(),
            config: config.clone(),
            running: AtomicBool::new(false),
            introspector: Introspector::new(config.max_history),
            calibrator: Calibrator::new(config.max_history),
            diagnostician: Diagnostician::new(config.max_failures),
            evolver: Evolver::new(config.max_suggestions),
            total_ticks: AtomicU64::new(0),
            insights_generated: AtomicU64::new(0),
        }
    }

    /// Get domain ID
    pub fn id(&self) -> DomainId {
        self.id
    }

    /// Is running?
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get configuration
    pub fn config(&self) -> &ReflectConfig {
        &self.config
    }

    /// Get introspector
    pub fn introspector(&self) -> &Introspector {
        &self.introspector
    }

    /// Get calibrator
    pub fn calibrator(&self) -> &Calibrator {
        &self.calibrator
    }

    /// Get mutable calibrator
    pub fn calibrator_mut(&mut self) -> &mut Calibrator {
        &mut self.calibrator
    }

    /// Get diagnostician
    pub fn diagnostician(&self) -> &Diagnostician {
        &self.diagnostician
    }

    /// Get evolver
    pub fn evolver(&self) -> &Evolver {
        &self.evolver
    }

    /// Start the domain
    pub fn start(&mut self) -> Result<(), ReflectError> {
        if self.running.load(Ordering::Acquire) {
            return Err(ReflectError::AlreadyRunning);
        }
        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Stop the domain
    pub fn stop(&mut self) -> Result<(), ReflectError> {
        if !self.running.load(Ordering::Acquire) {
            return Err(ReflectError::NotRunning);
        }
        self.running.store(false, Ordering::Release);
        Ok(())
    }

    /// Record domain metrics
    pub fn record_metrics(&mut self, metrics: DomainMetrics) {
        self.introspector.record(metrics);
    }

    /// Record a cognitive failure
    pub fn record_failure(&mut self, failure: CognitiveFailure) -> FailureId {
        self.diagnostician.record_failure(failure)
    }

    /// Process one tick
    pub fn tick(&mut self, _now: Timestamp) -> Vec<Insight> {
        if !self.running.load(Ordering::Acquire) {
            return Vec::new();
        }

        let tick = self.total_ticks.fetch_add(1, Ordering::Relaxed) + 1;

        let mut insights = Vec::new();

        // Periodic introspection
        if tick % self.config.introspection_interval == 0 {
            let health = self.introspector.analyze();
            let calibration = self.calibrator.prediction_accuracy();

            // Generate insights from health
            if health.overall_score < 70 {
                insights.push(Insight {
                    id: InsightId::generate(),
                    insight_type: InsightType::HealthStatus,
                    target_domain: None,
                    title: String::from("Cognitive health degraded"),
                    description: alloc::format!(
                        "Overall health score: {} ({:?})",
                        health.overall_score,
                        health.status
                    ),
                    severity: if health.overall_score < 50 {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    recommendations: vec![String::from("Review domain health details")],
                    created_at: Timestamp::now(),
                    expires_at: None,
                });
            }

            // Generate insights from calibration
            if calibration.calibration_error > 0.15 {
                insights.push(Insight {
                    id: InsightId::generate(),
                    insight_type: InsightType::Calibration,
                    target_domain: None,
                    title: String::from("Calibration drift detected"),
                    description: alloc::format!(
                        "Calibration error: {:.1}%",
                        calibration.calibration_error * 100.0
                    ),
                    severity: Severity::Warning,
                    recommendations: calibration
                        .recommendations
                        .iter()
                        .map(|r| r.reason.clone())
                        .collect(),
                    created_at: Timestamp::now(),
                    expires_at: None,
                });
            }

            // Generate improvement suggestions
            self.evolver.generate_suggestions(&health, &calibration);

            // Convert pending suggestions to insights
            for suggestion in self.evolver.pending() {
                insights.push(Insight {
                    id: InsightId::generate(),
                    insight_type: InsightType::Improvement,
                    target_domain: suggestion.target_domain,
                    title: suggestion.title.clone(),
                    description: suggestion.description.clone(),
                    severity: match suggestion.priority {
                        Priority::Critical => Severity::Error,
                        Priority::High => Severity::Warning,
                        _ => Severity::Info,
                    },
                    recommendations: vec![suggestion.expected_benefit.clone()],
                    created_at: Timestamp::now(),
                    expires_at: None,
                });
            }
        }

        // Find failure patterns
        let patterns = self.diagnostician.find_patterns();
        for pattern in patterns {
            insights.push(Insight {
                id: InsightId::generate(),
                insight_type: InsightType::Diagnosis,
                target_domain: pattern.domains_affected.first().copied(),
                title: alloc::format!("{:?} pattern detected", pattern.pattern_type),
                description: pattern.description,
                severity: Severity::Warning,
                recommendations: vec![String::from("Investigate pattern root cause")],
                created_at: Timestamp::now(),
                expires_at: None,
            });
        }

        self.insights_generated
            .fetch_add(insights.len() as u64, Ordering::Relaxed);

        insights
    }

    /// Get domain statistics
    pub fn stats(&self) -> ReflectStats {
        ReflectStats {
            domain_id: self.id,
            is_running: self.running.load(Ordering::Relaxed),
            total_ticks: self.total_ticks.load(Ordering::Relaxed),
            insights_generated: self.insights_generated.load(Ordering::Relaxed),
            introspector: self.introspector.stats(),
            calibrator: self.calibrator.stats(),
            diagnostician: self.diagnostician.stats(),
            evolver: self.evolver.stats(),
        }
    }
}

impl Default for ReflectDomain {
    fn default() -> Self {
        Self::new(ReflectConfig::default())
    }
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Reflect domain statistics
#[derive(Debug, Clone)]
pub struct ReflectStats {
    /// Domain ID
    pub domain_id: DomainId,
    /// Is running
    pub is_running: bool,
    /// Total ticks
    pub total_ticks: u64,
    /// Insights generated
    pub insights_generated: u64,
    /// Introspector stats
    pub introspector: IntrospectorStats,
    /// Calibrator stats
    pub calibrator: CalibratorStats,
    /// Diagnostician stats
    pub diagnostician: DiagnosticianStats,
    /// Evolver stats
    pub evolver: EvolverStats,
}

// ============================================================================
// ERRORS
// ============================================================================

/// Reflect domain errors
#[derive(Debug)]
pub enum ReflectError {
    /// Domain already running
    AlreadyRunning,
    /// Domain not running
    NotRunning,
    /// Other error
    Other(String),
}

impl ReflectError {
    /// Get error message
    pub fn message(&self) -> &str {
        match self {
            Self::AlreadyRunning => "Domain already running",
            Self::NotRunning => "Domain not running",
            Self::Other(msg) => msg,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflect_domain() {
        let config = ReflectConfig::default();
        let domain = ReflectDomain::new(config);

        assert!(!domain.stats().is_running);
        assert_eq!(domain.stats().insights_generated, 0);
    }

    #[test]
    fn test_reflect_lifecycle() {
        let mut domain = ReflectDomain::default();

        assert!(domain.start().is_ok());
        assert!(domain.is_running());
        assert!(domain.start().is_err()); // Already running

        assert!(domain.stop().is_ok());
        assert!(!domain.is_running());
    }

    #[test]
    fn test_record_metrics() {
        let mut domain = ReflectDomain::default();

        let metrics = DomainMetrics {
            domain: Domain::Sense,
            health_score: 90,
            messages_processed: 1000,
            avg_latency_us: 500,
            p99_latency_us: 2000,
            error_rate: 0.01,
            queue_depth: 10,
            last_tick: 100,
            timestamp: Timestamp::now(),
        };

        domain.record_metrics(metrics);
        assert!(domain.introspector.current(Domain::Sense).is_some());
    }

    #[test]
    fn test_config_variants() {
        let minimal = ReflectConfig::minimal();
        assert_eq!(minimal.max_history, 100);

        let detailed = ReflectConfig::detailed();
        assert_eq!(detailed.max_history, 10000);
    }
}
