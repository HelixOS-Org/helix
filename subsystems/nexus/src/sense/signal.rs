//! Signal Normalizer
//!
//! Converts raw events to normalized signals for the understand domain.

#![allow(dead_code)]

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;
use super::events::{EventData, RawEvent};
use super::probe::ProbeType;

// ============================================================================
// SIGNAL TYPES
// ============================================================================

/// Signal - normalized event ready for understand domain
#[derive(Debug, Clone)]
pub struct Signal {
    /// Signal ID
    pub id: SignalId,
    /// Source probe
    pub source: ProbeId,
    /// Source type
    pub source_type: ProbeType,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Sequence number
    pub sequence: u64,
    /// Signal type
    pub signal_type: SignalType,
    /// Signal value
    pub value: SignalValue,
    /// Metadata
    pub metadata: SignalMetadata,
}

impl Signal {
    /// Is metric signal?
    pub const fn is_metric(&self) -> bool {
        matches!(self.signal_type, SignalType::Metric)
    }

    /// Is error signal?
    pub const fn is_error(&self) -> bool {
        matches!(self.signal_type, SignalType::Error)
    }

    /// Is warning signal?
    pub const fn is_warning(&self) -> bool {
        matches!(self.signal_type, SignalType::Warning)
    }
}

/// Signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    /// Metric sample
    Metric,
    /// State transition
    StateChange,
    /// Error condition
    Error,
    /// Warning condition
    Warning,
    /// Event occurrence
    Event,
    /// Threshold crossed
    Threshold,
}

/// Signal value
#[derive(Debug, Clone)]
pub enum SignalValue {
    /// Numeric value
    Numeric(f64),
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i64),
    /// String value
    String(String),
    /// Multiple values
    Vector(alloc::vec::Vec<f64>),
    /// State value
    State(u32),
}

impl SignalValue {
    /// Try to get as f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Numeric(v) => Some(*v),
            Self::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Try to get as i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            Self::Numeric(v) => Some(*v as i64),
            Self::State(v) => Some(*v as i64),
            _ => None,
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }
}

/// Signal metadata
#[derive(Debug, Clone)]
pub struct SignalMetadata {
    /// CPU where signal originated
    pub cpu: Option<u32>,
    /// Process ID
    pub pid: Option<u32>,
    /// Thread ID
    pub tid: Option<u32>,
    /// Unit of measurement
    pub unit: Option<MetricUnit>,
    /// Tags
    pub tags: Tags,
}

impl Default for SignalMetadata {
    fn default() -> Self {
        Self {
            cpu: None,
            pid: None,
            tid: None,
            unit: None,
            tags: Tags::new(),
        }
    }
}

impl SignalMetadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// With CPU
    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// With process info
    pub fn with_process(mut self, pid: u32, tid: Option<u32>) -> Self {
        self.pid = Some(pid);
        self.tid = tid;
        self
    }

    /// With unit
    pub fn with_unit(mut self, unit: MetricUnit) -> Self {
        self.unit = Some(unit);
        self
    }
}

// ============================================================================
// SIGNAL NORMALIZER
// ============================================================================

/// Signal normalizer - converts raw events to signals
pub struct SignalNormalizer {
    /// Sequence counter
    sequence: AtomicU64,
    /// Signals produced
    signals_produced: AtomicU64,
    /// Events processed
    events_processed: AtomicU64,
}

impl SignalNormalizer {
    /// Create new normalizer
    pub fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
            signals_produced: AtomicU64::new(0),
            events_processed: AtomicU64::new(0),
        }
    }

    /// Normalize raw event to signal
    pub fn normalize(&self, event: RawEvent) -> Signal {
        self.events_processed.fetch_add(1, Ordering::Relaxed);

        let (signal_type, value) = self.extract_signal(&event);

        let signal = Signal {
            id: SignalId::generate(),
            source: event.probe_id,
            source_type: event.probe_type,
            timestamp: event.timestamp,
            sequence: self.sequence.fetch_add(1, Ordering::SeqCst),
            signal_type,
            value,
            metadata: SignalMetadata {
                cpu: Some(event.cpu),
                pid: event.pid,
                tid: event.tid,
                unit: None,
                tags: Tags::new(),
            },
        };

        self.signals_produced.fetch_add(1, Ordering::Relaxed);
        signal
    }

    /// Normalize batch of events
    pub fn normalize_batch(&self, events: alloc::vec::Vec<RawEvent>) -> alloc::vec::Vec<Signal> {
        events.into_iter().map(|e| self.normalize(e)).collect()
    }

    /// Extract signal type and value from event
    fn extract_signal(&self, event: &RawEvent) -> (SignalType, SignalValue) {
        match &event.data {
            EventData::CpuSample(s) => (
                SignalType::Metric,
                SignalValue::Numeric(s.busy_percent() as f64),
            ),
            EventData::MemorySample(s) => (
                SignalType::Metric,
                SignalValue::Numeric(s.usage_percent() as f64),
            ),
            EventData::BlockIoEvent(e) => {
                (SignalType::Event, SignalValue::Integer(e.latency_ns as i64))
            },
            EventData::NetworkIoEvent(e) => {
                (SignalType::Metric, SignalValue::Integer(e.bytes as i64))
            },
            EventData::SchedulerEvent(_) => (SignalType::StateChange, SignalValue::Boolean(true)),
            EventData::InterruptEvent(e) => (
                SignalType::Event,
                SignalValue::Integer(e.duration_ns as i64),
            ),
            EventData::SyscallEvent(_) => (SignalType::Event, SignalValue::Boolean(true)),
            EventData::PageFaultEvent(e) => (SignalType::Event, SignalValue::Boolean(e.major)),
            EventData::TimerEvent(e) => (SignalType::Event, SignalValue::Integer(e.slack_ns)),
            EventData::PowerEvent(e) => {
                (SignalType::StateChange, SignalValue::State(e.target_state))
            },
            EventData::ThermalEvent(e) => {
                if e.throttle {
                    (SignalType::Warning, SignalValue::Integer(e.temp_mc as i64))
                } else {
                    (SignalType::Metric, SignalValue::Integer(e.temp_mc as i64))
                }
            },
            EventData::DeviceEvent(_) => (SignalType::StateChange, SignalValue::Boolean(true)),
            EventData::FilesystemEvent(e) => {
                if e.error != 0 {
                    (SignalType::Error, SignalValue::Integer(e.error as i64))
                } else {
                    (
                        SignalType::Event,
                        SignalValue::Integer(e.duration_ns as i64),
                    )
                }
            },
            EventData::SecurityEvent(e) => {
                if e.allowed {
                    (SignalType::Event, SignalValue::Boolean(true))
                } else {
                    (SignalType::Warning, SignalValue::Boolean(false))
                }
            },
            EventData::Metric(m) => match &m.value {
                MetricValue::Counter(v) => (SignalType::Metric, SignalValue::Integer(*v as i64)),
                MetricValue::Gauge(v) => (SignalType::Metric, SignalValue::Numeric(*v)),
                MetricValue::Histogram(_) => (SignalType::Metric, SignalValue::Numeric(0.0)),
            },
            EventData::Raw(_) => (SignalType::Event, SignalValue::Boolean(true)),
        }
    }

    /// Get statistics
    pub fn stats(&self) -> NormalizerStats {
        NormalizerStats {
            events_processed: self.events_processed.load(Ordering::Relaxed),
            signals_produced: self.signals_produced.load(Ordering::Relaxed),
            current_sequence: self.sequence.load(Ordering::Relaxed),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.events_processed.store(0, Ordering::Relaxed);
        self.signals_produced.store(0, Ordering::Relaxed);
    }
}

impl Default for SignalNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalizer statistics
#[derive(Debug, Clone)]
pub struct NormalizerStats {
    /// Events processed
    pub events_processed: u64,
    /// Signals produced
    pub signals_produced: u64,
    /// Current sequence number
    pub current_sequence: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::events::CpuSample;

    #[test]
    fn test_signal_normalizer() {
        let normalizer = SignalNormalizer::new();

        let event = RawEvent::new(
            ProbeId::generate(),
            ProbeType::Cpu,
            EventData::CpuSample(CpuSample {
                cpu_id: 0,
                user: 50,
                system: 25,
                idle: 25,
                iowait: 0,
                irq: 0,
                softirq: 0,
                steal: 0,
                frequency_mhz: 3000,
                temperature: None,
            }),
        );

        let signal = normalizer.normalize(event);
        assert_eq!(signal.signal_type, SignalType::Metric);
        assert_eq!(signal.sequence, 0);
        assert!(signal.is_metric());
    }

    #[test]
    fn test_signal_value() {
        let numeric = SignalValue::Numeric(42.5);
        assert_eq!(numeric.as_f64(), Some(42.5));
        assert_eq!(numeric.as_i64(), Some(42));

        let integer = SignalValue::Integer(100);
        assert_eq!(integer.as_i64(), Some(100));
        assert_eq!(integer.as_f64(), Some(100.0));
    }
}
