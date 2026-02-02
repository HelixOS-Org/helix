//! Insight — Reflection output types
//!
//! Insights are the output of the REFLECT domain, representing
//! findings about the cognitive system's health and performance.

use alloc::string::String;
use alloc::vec::Vec;

use crate::bus::Domain;
use crate::types::*;

// ============================================================================
// INSIGHT TYPE
// ============================================================================

/// Insight type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightType {
    /// Health status
    HealthStatus,
    /// Calibration finding
    Calibration,
    /// Failure diagnosis
    Diagnosis,
    /// Improvement suggestion
    Improvement,
    /// Warning
    Warning,
    /// Trend observation
    Trend,
}

impl InsightType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::HealthStatus => "Health Status",
            Self::Calibration => "Calibration",
            Self::Diagnosis => "Diagnosis",
            Self::Improvement => "Improvement",
            Self::Warning => "Warning",
            Self::Trend => "Trend",
        }
    }
}

// ============================================================================
// INSIGHT
// ============================================================================

/// An insight - output of reflection
#[derive(Debug, Clone)]
pub struct Insight {
    /// Insight ID
    pub id: InsightId,
    /// Insight type
    pub insight_type: InsightType,
    /// Target domain (None = system-wide)
    pub target_domain: Option<Domain>,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
    /// Actionable recommendations
    pub recommendations: Vec<String>,
    /// Created at
    pub created_at: Timestamp,
    /// Expires at
    pub expires_at: Option<Timestamp>,
}

impl Insight {
    /// Create new insight
    pub fn new(
        insight_type: InsightType,
        title: impl Into<String>,
        description: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: InsightId::generate(),
            insight_type,
            target_domain: None,
            title: title.into(),
            description: description.into(),
            severity,
            recommendations: Vec::new(),
            created_at: Timestamp::now(),
            expires_at: None,
        }
    }

    /// Set target domain
    pub fn for_domain(mut self, domain: Domain) -> Self {
        self.target_domain = Some(domain);
        self
    }

    /// Add recommendation
    pub fn with_recommendation(mut self, recommendation: impl Into<String>) -> Self {
        self.recommendations.push(recommendation.into());
        self
    }

    /// Set expiration
    pub fn expires_at(mut self, timestamp: Timestamp) -> Self {
        self.expires_at = Some(timestamp);
        self
    }

    /// Set expiration duration from now
    pub fn expires_in(mut self, duration: Duration) -> Self {
        self.expires_at = Some(Timestamp::new(
            Timestamp::now().as_nanos() + duration.as_nanos(),
        ));
        self
    }

    /// Is expired?
    pub fn is_expired(&self, now: Timestamp) -> bool {
        self.expires_at
            .map(|exp| now.as_nanos() > exp.as_nanos())
            .unwrap_or(false)
    }

    /// Is critical?
    pub fn is_critical(&self) -> bool {
        self.severity == Severity::Critical
    }

    /// Is warning or worse?
    pub fn is_warning_or_worse(&self) -> bool {
        matches!(
            self.severity,
            Severity::Critical | Severity::Error | Severity::Warning
        )
    }

    /// Has recommendations?
    pub fn has_recommendations(&self) -> bool {
        !self.recommendations.is_empty()
    }
}

// ============================================================================
// INSIGHT BATCH
// ============================================================================

/// A batch of insights
#[derive(Debug, Clone)]
pub struct InsightBatch {
    /// Insights
    pub insights: Vec<Insight>,
    /// Created at
    pub created_at: Timestamp,
}

impl InsightBatch {
    /// Create new empty batch
    pub fn new() -> Self {
        Self {
            insights: Vec::new(),
            created_at: Timestamp::now(),
        }
    }

    /// Create from insights
    pub fn from(insights: Vec<Insight>) -> Self {
        Self {
            insights,
            created_at: Timestamp::now(),
        }
    }

    /// Add insight
    pub fn add(&mut self, insight: Insight) {
        self.insights.push(insight);
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.insights.is_empty()
    }

    /// Get count
    pub fn len(&self) -> usize {
        self.insights.len()
    }

    /// Get critical insights
    pub fn critical(&self) -> Vec<&Insight> {
        self.insights.iter().filter(|i| i.is_critical()).collect()
    }

    /// Get by type
    pub fn by_type(&self, insight_type: InsightType) -> Vec<&Insight> {
        self.insights
            .iter()
            .filter(|i| i.insight_type == insight_type)
            .collect()
    }

    /// Get by domain
    pub fn by_domain(&self, domain: Domain) -> Vec<&Insight> {
        self.insights
            .iter()
            .filter(|i| i.target_domain == Some(domain))
            .collect()
    }
}

impl Default for InsightBatch {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight() {
        let insight = Insight::new(
            InsightType::Warning,
            "High latency detected",
            "The Reason domain is experiencing high latency",
            Severity::Warning,
        )
        .for_domain(Domain::Reason)
        .with_recommendation("Reduce pattern count");

        assert!(insight.target_domain.is_some());
        assert!(insight.has_recommendations());
        assert!(insight.is_warning_or_worse());
    }

    #[test]
    fn test_insight_expiration() {
        let insight = Insight::new(InsightType::HealthStatus, "Test", "Test", Severity::Info)
            .expires_in(Duration::from_secs(60));

        assert!(!insight.is_expired(Timestamp::now()));
    }

    #[test]
    fn test_insight_batch() {
        let mut batch = InsightBatch::new();

        batch.add(Insight::new(
            InsightType::Warning,
            "Warning 1",
            "Description",
            Severity::Warning,
        ));
        batch.add(Insight::new(
            InsightType::Diagnosis,
            "Diagnosis 1",
            "Description",
            Severity::Info,
        ));

        assert_eq!(batch.len(), 2);
        assert_eq!(batch.by_type(InsightType::Warning).len(), 1);
    }
}
