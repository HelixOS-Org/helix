//! # Cognitive Insight Engine
//!
//! Generates insights from cognitive data.
//! Pattern recognition and anomaly detection.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// INSIGHT TYPES
// ============================================================================

/// An insight
#[derive(Debug, Clone)]
pub struct Insight {
    /// Insight ID
    pub id: u64,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Category
    pub category: InsightCategory,
    /// Severity
    pub severity: InsightSeverity,
    /// Source domain
    pub source: DomainId,
    /// Evidence
    pub evidence: Vec<Evidence>,
    /// Recommendations
    pub recommendations: Vec<String>,
    /// Confidence (0-1)
    pub confidence: f64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Tags
    pub tags: Vec<String>,
    /// Status
    pub status: InsightStatus,
}

/// Insight category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightCategory {
    /// Performance insight
    Performance,
    /// Behavioral pattern
    Behavior,
    /// Anomaly detection
    Anomaly,
    /// Trend analysis
    Trend,
    /// Correlation discovery
    Correlation,
    /// Prediction
    Prediction,
    /// Optimization opportunity
    Optimization,
    /// Risk identification
    Risk,
}

/// Insight severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InsightSeverity {
    /// Informational
    Info,
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical
    Critical,
}

/// Evidence for an insight
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Evidence type
    pub evidence_type: EvidenceType,
    /// Data
    pub data: EvidenceData,
    /// Weight
    pub weight: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Evidence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    /// Metric value
    Metric,
    /// Event occurrence
    Event,
    /// Pattern match
    Pattern,
    /// Statistical test
    Statistical,
    /// Model output
    Model,
}

/// Evidence data
#[derive(Debug, Clone)]
pub enum EvidenceData {
    /// Scalar value
    Scalar(f64),
    /// Time series
    TimeSeries(Vec<(u64, f64)>),
    /// Text
    Text(String),
    /// Key-value pairs
    Map(BTreeMap<String, String>),
}

/// Insight status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightStatus {
    /// New insight
    New,
    /// Acknowledged
    Acknowledged,
    /// In progress (being acted upon)
    InProgress,
    /// Resolved
    Resolved,
    /// Dismissed
    Dismissed,
}

// ============================================================================
// INSIGHT ENGINE
// ============================================================================

/// Generates and manages insights
pub struct InsightEngine {
    /// Generated insights
    insights: BTreeMap<u64, Insight>,
    /// Patterns
    patterns: Vec<Pattern>,
    /// Anomaly detectors
    detectors: Vec<AnomalyDetector>,
    /// Next insight ID
    next_id: AtomicU64,
    /// Configuration
    config: InsightConfig,
    /// Statistics
    stats: InsightStats,
}

/// Pattern definition
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern ID
    pub id: u64,
    /// Pattern name
    pub name: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Detection threshold
    pub threshold: f64,
    /// Category to assign
    pub category: InsightCategory,
    /// Severity to assign
    pub severity: InsightSeverity,
}

/// Pattern type
#[derive(Debug, Clone)]
pub enum PatternType {
    /// Value exceeds threshold
    ThresholdHigh(f64),
    /// Value below threshold
    ThresholdLow(f64),
    /// Spike detection
    Spike(f64),
    /// Trend detection
    Trend(TrendType),
    /// Cyclical pattern
    Cyclical(u64),
    /// Sequence pattern
    Sequence(Vec<String>),
}

/// Trend type
#[derive(Debug, Clone, Copy)]
pub enum TrendType {
    Increasing,
    Decreasing,
    Stable,
}

/// Anomaly detector
#[derive(Debug, Clone)]
pub struct AnomalyDetector {
    /// Detector ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Detection method
    pub method: AnomalyMethod,
    /// Sensitivity (0-1)
    pub sensitivity: f64,
    /// Historical data for learning
    history: VecDeque<f64>,
    /// Statistics
    mean: f64,
    std_dev: f64,
}

/// Anomaly detection method
#[derive(Debug, Clone, Copy)]
pub enum AnomalyMethod {
    /// Z-score based
    ZScore,
    /// IQR-based
    Iqr,
    /// MAD-based
    Mad,
    /// Isolation forest (simplified)
    IsolationForest,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct InsightConfig {
    /// Maximum insights to keep
    pub max_insights: usize,
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Enable auto-resolution
    pub auto_resolve: bool,
    /// Resolution timeout (ns)
    pub resolution_timeout_ns: u64,
}

impl Default for InsightConfig {
    fn default() -> Self {
        Self {
            max_insights: 10000,
            min_confidence: 0.5,
            auto_resolve: true,
            resolution_timeout_ns: 3600_000_000_000, // 1 hour
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct InsightStats {
    /// Total insights generated
    pub total_generated: u64,
    /// Insights by category
    pub by_category: BTreeMap<String, u64>,
    /// Insights by severity
    pub by_severity: BTreeMap<String, u64>,
    /// Average confidence
    pub avg_confidence: f64,
    /// Patterns matched
    pub patterns_matched: u64,
    /// Anomalies detected
    pub anomalies_detected: u64,
}

impl InsightEngine {
    /// Create a new insight engine
    pub fn new(config: InsightConfig) -> Self {
        Self {
            insights: BTreeMap::new(),
            patterns: Vec::new(),
            detectors: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: InsightStats::default(),
        }
    }

    /// Generate an insight
    pub fn generate(
        &mut self,
        title: &str,
        description: &str,
        category: InsightCategory,
        severity: InsightSeverity,
        source: DomainId,
        evidence: Vec<Evidence>,
        confidence: f64,
    ) -> u64 {
        if confidence < self.config.min_confidence {
            return 0; // Below threshold
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let insight = Insight {
            id,
            title: title.into(),
            description: description.into(),
            category,
            severity,
            source,
            evidence,
            recommendations: Vec::new(),
            confidence,
            timestamp: Timestamp::now(),
            tags: Vec::new(),
            status: InsightStatus::New,
        };

        // Check capacity
        if self.insights.len() >= self.config.max_insights {
            // Remove oldest resolved/dismissed
            let to_remove: Vec<_> = self
                .insights
                .iter()
                .filter(|(_, i)| {
                    matches!(i.status, InsightStatus::Resolved | InsightStatus::Dismissed)
                })
                .map(|(id, _)| *id)
                .take(1)
                .collect();

            for r_id in to_remove {
                self.insights.remove(&r_id);
            }
        }

        self.insights.insert(id, insight);

        // Update stats
        self.stats.total_generated += 1;
        *self
            .stats
            .by_category
            .entry(format!("{:?}", category))
            .or_default() += 1;
        *self
            .stats
            .by_severity
            .entry(format!("{:?}", severity))
            .or_default() += 1;
        self.stats.avg_confidence =
            (self.stats.avg_confidence * (self.stats.total_generated - 1) as f64 + confidence)
                / self.stats.total_generated as f64;

        id
    }

    /// Add a pattern
    #[inline(always)]
    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    /// Add an anomaly detector
    #[inline(always)]
    pub fn add_detector(&mut self, detector: AnomalyDetector) {
        self.detectors.push(detector);
    }

    /// Analyze value against patterns
    pub fn analyze_value(&mut self, name: &str, value: f64, source: DomainId) -> Vec<u64> {
        let mut generated = Vec::new();

        for pattern in &self.patterns {
            let matched = match &pattern.pattern_type {
                PatternType::ThresholdHigh(threshold) => value > *threshold,
                PatternType::ThresholdLow(threshold) => value < *threshold,
                PatternType::Spike(delta) => {
                    // Would need history, simplified check
                    value.abs() > *delta
                },
                _ => false,
            };

            if matched {
                self.stats.patterns_matched += 1;

                let evidence = Evidence {
                    evidence_type: EvidenceType::Metric,
                    data: EvidenceData::Scalar(value),
                    weight: 1.0,
                    timestamp: Timestamp::now(),
                };

                let id = self.generate(
                    &format!("{} matched pattern: {}", name, pattern.name),
                    &format!("Value {} matched pattern threshold", value),
                    pattern.category,
                    pattern.severity,
                    source,
                    vec![evidence],
                    0.8,
                );

                if id > 0 {
                    generated.push(id);
                }
            }
        }

        generated
    }

    /// Detect anomalies
    pub fn detect_anomalies(&mut self, name: &str, value: f64, source: DomainId) -> Vec<u64> {
        let mut generated = Vec::new();

        for detector in &mut self.detectors {
            detector.update(value);

            if let Some(score) = detector.detect(value) {
                if score > detector.sensitivity {
                    self.stats.anomalies_detected += 1;

                    let severity = if score > 0.9 {
                        InsightSeverity::Critical
                    } else if score > 0.7 {
                        InsightSeverity::High
                    } else {
                        InsightSeverity::Medium
                    };

                    let evidence = Evidence {
                        evidence_type: EvidenceType::Statistical,
                        data: EvidenceData::Scalar(score),
                        weight: score,
                        timestamp: Timestamp::now(),
                    };

                    let id = self.generate(
                        &format!("Anomaly detected in {}", name),
                        &format!("Anomaly score: {:.2} using {}", score, detector.name),
                        InsightCategory::Anomaly,
                        severity,
                        source,
                        vec![evidence],
                        score,
                    );

                    if id > 0 {
                        generated.push(id);
                    }
                }
            }
        }

        generated
    }

    /// Analyze trend
    pub fn analyze_trend(&mut self, name: &str, values: &[f64], source: DomainId) -> Option<u64> {
        if values.len() < 3 {
            return None;
        }

        // Simple linear regression
        let n = values.len() as f64;
        let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = values.iter().enumerate().map(|(i, y)| i as f64 * y).sum();
        let sum_xx: f64 = (0..values.len()).map(|i| (i * i) as f64).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);

        let trend = if slope > 0.1 {
            TrendType::Increasing
        } else if slope < -0.1 {
            TrendType::Decreasing
        } else {
            TrendType::Stable
        };

        let (description, severity) = match trend {
            TrendType::Increasing => ("Increasing trend detected", InsightSeverity::Info),
            TrendType::Decreasing => ("Decreasing trend detected", InsightSeverity::Info),
            TrendType::Stable => return None, // Don't report stable trends
        };

        let evidence = Evidence {
            evidence_type: EvidenceType::Statistical,
            data: EvidenceData::Scalar(slope),
            weight: slope.abs().min(1.0),
            timestamp: Timestamp::now(),
        };

        let id = self.generate(
            &format!("Trend in {}", name),
            description,
            InsightCategory::Trend,
            severity,
            source,
            vec![evidence],
            0.7,
        );

        if id > 0 { Some(id) } else { None }
    }

    /// Get insight
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&Insight> {
        self.insights.get(&id)
    }

    /// Get insights by category
    #[inline]
    pub fn by_category(&self, category: InsightCategory) -> Vec<&Insight> {
        self.insights
            .values()
            .filter(|i| i.category == category)
            .collect()
    }

    /// Get insights by severity
    #[inline]
    pub fn by_severity(&self, min_severity: InsightSeverity) -> Vec<&Insight> {
        self.insights
            .values()
            .filter(|i| i.severity >= min_severity)
            .collect()
    }

    /// Get new insights
    #[inline]
    pub fn new_insights(&self) -> Vec<&Insight> {
        self.insights
            .values()
            .filter(|i| i.status == InsightStatus::New)
            .collect()
    }

    /// Update insight status
    #[inline]
    pub fn update_status(&mut self, id: u64, status: InsightStatus) {
        if let Some(insight) = self.insights.get_mut(&id) {
            insight.status = status;
        }
    }

    /// Add recommendation
    #[inline]
    pub fn add_recommendation(&mut self, id: u64, recommendation: &str) {
        if let Some(insight) = self.insights.get_mut(&id) {
            insight.recommendations.push(recommendation.into());
        }
    }

    /// Add tags
    #[inline]
    pub fn add_tags(&mut self, id: u64, tags: &[&str]) {
        if let Some(insight) = self.insights.get_mut(&id) {
            for tag in tags {
                if !insight.tags.contains(&(*tag).into()) {
                    insight.tags.push((*tag).into());
                }
            }
        }
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &InsightStats {
        &self.stats
    }

    /// Get insight count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.insights.len()
    }
}

impl AnomalyDetector {
    /// Create a new detector
    pub fn new(id: u64, name: &str, method: AnomalyMethod, sensitivity: f64) -> Self {
        Self {
            id,
            name: name.into(),
            method,
            sensitivity,
            history: VecDeque::new(),
            mean: 0.0,
            std_dev: 1.0,
        }
    }

    /// Update with new value
    pub fn update(&mut self, value: f64) {
        self.history.push_back(value);

        // Keep last 100 values
        if self.history.len() > 100 {
            self.history.pop_front();
        }

        // Update statistics
        if !self.history.is_empty() {
            let n = self.history.len() as f64;
            self.mean = self.history.iter().sum::<f64>() / n;

            let variance = self
                .history
                .iter()
                .map(|x| (x - self.mean).powi(2))
                .sum::<f64>()
                / n;

            self.std_dev = variance.sqrt().max(0.0001);
        }
    }

    /// Detect anomaly (returns score 0-1, None if not enough data)
    pub fn detect(&self, value: f64) -> Option<f64> {
        if self.history.len() < 10 {
            return None;
        }

        match self.method {
            AnomalyMethod::ZScore => {
                let z = (value - self.mean).abs() / self.std_dev;
                // Convert z-score to 0-1 range (z > 3 is usually anomalous)
                Some((z / 5.0).min(1.0))
            },
            AnomalyMethod::Iqr => {
                let mut sorted = self.history.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

                let q1 = sorted[sorted.len() / 4];
                let q3 = sorted[3 * sorted.len() / 4];
                let iqr = q3 - q1;

                let lower = q1 - 1.5 * iqr;
                let upper = q3 + 1.5 * iqr;

                if value < lower || value > upper {
                    let distance = if value < lower {
                        (lower - value) / iqr
                    } else {
                        (value - upper) / iqr
                    };
                    Some((distance / 3.0).min(1.0))
                } else {
                    Some(0.0)
                }
            },
            AnomalyMethod::Mad => {
                let median = {
                    let mut sorted = self.history.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
                    sorted[sorted.len() / 2]
                };

                let mad: f64 = self.history.iter().map(|x| (x - median).abs()).sum::<f64>()
                    / self.history.len() as f64;

                let deviation = (value - median).abs() / (mad.max(0.0001) * 1.4826);
                Some((deviation / 5.0).min(1.0))
            },
            AnomalyMethod::IsolationForest => {
                // Simplified isolation score
                let z = (value - self.mean).abs() / self.std_dev;
                Some((z / 4.0).min(1.0))
            },
        }
    }
}

impl Default for InsightEngine {
    fn default() -> Self {
        Self::new(InsightConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_generation() {
        let mut engine = InsightEngine::default();
        let domain = DomainId::new(1);

        let id = engine.generate(
            "High CPU Usage",
            "CPU usage exceeded 90%",
            InsightCategory::Performance,
            InsightSeverity::High,
            domain,
            Vec::new(),
            0.85,
        );

        assert!(id > 0);

        let insight = engine.get(id).unwrap();
        assert_eq!(insight.title, "High CPU Usage");
        assert_eq!(insight.category, InsightCategory::Performance);
    }

    #[test]
    fn test_pattern_matching() {
        let mut engine = InsightEngine::default();
        let domain = DomainId::new(1);

        let pattern = Pattern {
            id: 1,
            name: "High Value".into(),
            pattern_type: PatternType::ThresholdHigh(100.0),
            threshold: 0.8,
            category: InsightCategory::Anomaly,
            severity: InsightSeverity::High,
        };

        engine.add_pattern(pattern);

        let insights = engine.analyze_value("metric", 150.0, domain);
        assert!(!insights.is_empty());
    }

    #[test]
    fn test_anomaly_detection() {
        let mut engine = InsightEngine::default();
        let domain = DomainId::new(1);

        let mut detector = AnomalyDetector::new(1, "z_score", AnomalyMethod::ZScore, 0.5);

        // Build up history with normal values
        for i in 0..50 {
            detector.update(100.0 + (i % 5) as f64);
        }

        engine.add_detector(detector);

        // Test with anomalous value
        let insights = engine.detect_anomalies("metric", 200.0, domain);
        assert!(!insights.is_empty());
    }

    #[test]
    fn test_trend_analysis() {
        let mut engine = InsightEngine::default();
        let domain = DomainId::new(1);

        // Increasing trend
        let values: Vec<f64> = (0..10).map(|i| 100.0 + i as f64 * 10.0).collect();

        let insight_id = engine.analyze_trend("growth_metric", &values, domain);
        assert!(insight_id.is_some());

        let insight = engine.get(insight_id.unwrap()).unwrap();
        assert_eq!(insight.category, InsightCategory::Trend);
    }

    #[test]
    fn test_insight_status() {
        let mut engine = InsightEngine::default();
        let domain = DomainId::new(1);

        let id = engine.generate(
            "Test",
            "Test insight",
            InsightCategory::Performance,
            InsightSeverity::Medium,
            domain,
            Vec::new(),
            0.7,
        );

        assert_eq!(engine.get(id).unwrap().status, InsightStatus::New);

        engine.update_status(id, InsightStatus::Acknowledged);
        assert_eq!(engine.get(id).unwrap().status, InsightStatus::Acknowledged);
    }
}
