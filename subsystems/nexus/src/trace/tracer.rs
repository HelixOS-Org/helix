//! Main tracer implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::buffer::TraceRingBuffer;
use super::record::TraceRecord;
use super::span::Span;
use super::types::TraceLevel;

// ============================================================================
// TRACER CONFIG
// ============================================================================

/// Tracer configuration
#[derive(Debug, Clone)]
pub struct TracerConfig {
    /// Buffer size
    pub buffer_size: usize,
    /// Minimum trace level
    pub min_level: TraceLevel,
    /// Sample rate (0.0-1.0)
    pub sample_rate: f32,
    /// Enable adaptive sampling
    pub adaptive_sampling: bool,
    /// Target overhead percentage
    pub target_overhead: f32,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            buffer_size: 65536,
            min_level: TraceLevel::Info,
            sample_rate: 1.0,
            adaptive_sampling: true,
            target_overhead: 0.01, // 1%
        }
    }
}

// ============================================================================
// TRACER STATS
// ============================================================================

/// Tracer statistics
#[derive(Debug, Clone)]
pub struct TracerStats {
    /// Buffer slots used
    pub buffer_used: usize,
    /// Buffer capacity
    pub buffer_capacity: usize,
    /// Total records written
    pub total_written: u64,
    /// Total records dropped
    pub total_dropped: u64,
    /// Current sample rate
    pub sample_rate: f32,
}

// ============================================================================
// TRACER
// ============================================================================

/// The main tracer
pub struct Tracer {
    /// Configuration
    config: TracerConfig,
    /// Ring buffer
    buffer: TraceRingBuffer,
    /// Is tracer enabled
    enabled: AtomicBool,
    /// Current sample rate (for adaptive)
    current_sample_rate: f32,
    /// Name hash cache
    name_hashes: Vec<(&'static str, u32)>,
}

impl Tracer {
    /// Create a new tracer
    pub fn new(config: TracerConfig) -> Self {
        let buffer_size = config.buffer_size;
        Self {
            config,
            buffer: TraceRingBuffer::new(buffer_size),
            enabled: AtomicBool::new(true),
            current_sample_rate: 1.0,
            name_hashes: Vec::new(),
        }
    }

    /// Enable tracing
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable tracing
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get name hash
    fn hash_name(&mut self, name: &'static str) -> u32 {
        // Check cache
        for (n, h) in &self.name_hashes {
            if *n == name {
                return *h;
            }
        }

        // Calculate hash (FNV-1a)
        let mut hash = 2166136261u32;
        for byte in name.bytes() {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(16777619);
        }

        // Cache it
        if self.name_hashes.len() < 1024 {
            self.name_hashes.push((name, hash));
        }

        hash
    }

    /// Start a span
    pub fn start_span(&mut self, span: &Span) {
        if !self.is_enabled() {
            return;
        }

        if span.level > self.config.min_level {
            return;
        }

        let hash = self.hash_name(span.name);
        let record = TraceRecord::span_start(span, hash);
        self.buffer.write(record);
    }

    /// End a span
    pub fn end_span(&mut self, span: &Span) {
        if !self.is_enabled() {
            return;
        }

        if span.level > self.config.min_level {
            return;
        }

        let record = TraceRecord::span_end(span);
        self.buffer.write(record);
    }

    /// Read records
    pub fn drain(&mut self) -> Vec<TraceRecord> {
        let mut records = Vec::new();
        while let Some(record) = self.buffer.read() {
            records.push(record);
        }
        records
    }

    /// Get statistics
    pub fn stats(&self) -> TracerStats {
        TracerStats {
            buffer_used: self.buffer.len(),
            buffer_capacity: self.config.buffer_size,
            total_written: self.buffer.total_written(),
            total_dropped: self.buffer.total_dropped(),
            sample_rate: self.current_sample_rate,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &TracerConfig {
        &self.config
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new(TracerConfig::default())
    }
}
