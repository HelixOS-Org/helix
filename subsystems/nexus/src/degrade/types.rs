//! Degradation types and severity levels.

/// Type of degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DegradationType {
    /// Performance degradation (latency increase)
    Performance,
    /// Throughput degradation (ops/sec decrease)
    Throughput,
    /// Memory usage increasing
    MemoryUsage,
    /// Memory leak detected
    MemoryLeak,
    /// CPU usage increasing
    CpuUsage,
    /// Error rate increasing
    ErrorRate,
    /// Response time increasing
    ResponseTime,
    /// Queue depth increasing
    QueueDepth,
    /// Connection pool exhaustion
    ConnectionExhaustion,
    /// Handle leak
    HandleLeak,
    /// General resource exhaustion
    ResourceExhaustion,
}

impl DegradationType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::Throughput => "throughput",
            Self::MemoryUsage => "memory_usage",
            Self::MemoryLeak => "memory_leak",
            Self::CpuUsage => "cpu_usage",
            Self::ErrorRate => "error_rate",
            Self::ResponseTime => "response_time",
            Self::QueueDepth => "queue_depth",
            Self::ConnectionExhaustion => "connection_exhaustion",
            Self::HandleLeak => "handle_leak",
            Self::ResourceExhaustion => "resource_exhaustion",
        }
    }

    /// Is this resource-related?
    #[inline]
    pub fn is_resource(&self) -> bool {
        matches!(
            self,
            Self::MemoryUsage
                | Self::MemoryLeak
                | Self::CpuUsage
                | Self::HandleLeak
                | Self::ConnectionExhaustion
                | Self::ResourceExhaustion
        )
    }
}

/// Severity of degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DegradationSeverity {
    /// Minor degradation, monitoring only
    Minor = 0,
    /// Moderate degradation, warning
    Moderate = 1,
    /// Significant degradation, action recommended
    Significant = 2,
    /// Severe degradation, action required
    Severe = 3,
    /// Critical degradation, emergency
    Critical = 4,
}

impl DegradationSeverity {
    /// From percentage degradation
    pub fn from_percentage(pct: f64) -> Self {
        if pct < 10.0 {
            Self::Minor
        } else if pct < 25.0 {
            Self::Moderate
        } else if pct < 50.0 {
            Self::Significant
        } else if pct < 75.0 {
            Self::Severe
        } else {
            Self::Critical
        }
    }

    /// Get name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Minor => "minor",
            Self::Moderate => "moderate",
            Self::Significant => "significant",
            Self::Severe => "severe",
            Self::Critical => "critical",
        }
    }
}
