//! Driver performance metrics.

// ============================================================================
// DRIVER METRICS
// ============================================================================

/// Driver performance metrics
#[derive(Debug, Clone, Default)]
pub struct DriverMetrics {
    /// Total operations
    pub total_ops: u64,
    /// Successful operations
    pub successful_ops: u64,
    /// Failed operations
    pub failed_ops: u64,
    /// Average latency (nanoseconds)
    pub avg_latency_ns: f64,
    /// Maximum latency
    pub max_latency_ns: u64,
    /// 99th percentile latency
    pub p99_latency_ns: u64,
    /// CPU time used (nanoseconds)
    pub cpu_time_ns: u64,
    /// Memory used (bytes)
    pub memory_bytes: u64,
    /// Interrupt count
    pub interrupt_count: u64,
    /// DMA transfers
    pub dma_transfers: u64,
    /// DMA bytes
    pub dma_bytes: u64,
}

impl DriverMetrics {
    /// Record operation
    pub fn record_operation(&mut self, success: bool, latency_ns: u64) {
        self.total_ops += 1;
        if success {
            self.successful_ops += 1;
        } else {
            self.failed_ops += 1;
        }

        // Update latency
        let alpha = 0.1;
        self.avg_latency_ns = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;

        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_ops == 0 {
            1.0
        } else {
            self.successful_ops as f64 / self.total_ops as f64
        }
    }

    /// Get failure rate
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }

    /// Get operations per second
    pub fn ops_per_second(&self, uptime_ns: u64) -> f64 {
        if uptime_ns == 0 {
            0.0
        } else {
            self.total_ops as f64 * 1_000_000_000.0 / uptime_ns as f64
        }
    }
}
