//! Interrupt record tracking
//!
//! This module provides the InterruptRecord struct for capturing detailed
//! information about individual interrupt occurrences.

#![allow(dead_code)]

use super::types::{CpuId, InterruptType, Irq};
use crate::core::NexusTimestamp;

/// Record of an interrupt occurrence
#[derive(Debug, Clone)]
pub struct InterruptRecord {
    /// IRQ number
    pub irq: Irq,
    /// Type of interrupt
    pub irq_type: InterruptType,
    /// CPU that handled it
    pub cpu: CpuId,
    /// Timestamp
    pub timestamp: u64,
    /// Latency in nanoseconds
    pub latency_ns: u64,
    /// Was this expected?
    pub expected: bool,
}

impl InterruptRecord {
    /// Create new interrupt record
    pub fn new(irq: Irq, irq_type: InterruptType, cpu: CpuId) -> Self {
        Self {
            irq,
            irq_type,
            cpu,
            timestamp: NexusTimestamp::now().raw(),
            latency_ns: 0,
            expected: false,
        }
    }

    /// Set latency
    #[inline(always)]
    pub fn with_latency(mut self, latency_ns: u64) -> Self {
        self.latency_ns = latency_ns;
        self
    }

    /// Mark as expected
    #[inline(always)]
    pub fn with_expected(mut self, expected: bool) -> Self {
        self.expected = expected;
        self
    }
}
