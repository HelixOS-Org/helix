//! Core anomaly types

#![allow(dead_code)]

// ============================================================================
// ANOMALY SEVERITY
// ============================================================================

/// Severity of an anomaly
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnomalySeverity {
    /// Just a warning, might be noise
    Warning,
    /// Minor anomaly, worth investigating
    Minor,
    /// Moderate anomaly, should be addressed
    Moderate,
    /// Serious anomaly, needs attention
    Serious,
    /// Critical anomaly, immediate action needed
    Critical,
}

impl AnomalySeverity {
    /// Get from score (0.0 - 1.0)
    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 {
            Self::Critical
        } else if score >= 0.75 {
            Self::Serious
        } else if score >= 0.5 {
            Self::Moderate
        } else if score >= 0.25 {
            Self::Minor
        } else {
            Self::Warning
        }
    }

    /// Get numeric value (0-4)
    #[inline]
    pub fn value(&self) -> u8 {
        match self {
            Self::Warning => 0,
            Self::Minor => 1,
            Self::Moderate => 2,
            Self::Serious => 3,
            Self::Critical => 4,
        }
    }

    /// Get display name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Warning => "Warning",
            Self::Minor => "Minor",
            Self::Moderate => "Moderate",
            Self::Serious => "Serious",
            Self::Critical => "Critical",
        }
    }
}

// ============================================================================
// ANOMALY TYPE
// ============================================================================

/// Type of anomaly
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyType {
    /// Value spike (sudden increase)
    Spike,
    /// Value drop (sudden decrease)
    Drop,
    /// Value out of expected range
    OutOfRange,
    /// Unusual pattern
    PatternDeviation,
    /// Trend change
    TrendChange,
    /// Periodic anomaly
    PeriodicAnomaly,
    /// Correlation break
    CorrelationBreak,
    /// Missing data
    MissingData,
}

impl AnomalyType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Spike => "Spike",
            Self::Drop => "Drop",
            Self::OutOfRange => "Out of Range",
            Self::PatternDeviation => "Pattern Deviation",
            Self::TrendChange => "Trend Change",
            Self::PeriodicAnomaly => "Periodic Anomaly",
            Self::CorrelationBreak => "Correlation Break",
            Self::MissingData => "Missing Data",
        }
    }
}
