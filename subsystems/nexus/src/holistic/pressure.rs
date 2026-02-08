//! # Holistic Pressure Analysis Engine
//!
//! System-wide resource pressure monitoring (PSI-like):
//! - Pressure stall information tracking
//! - Multi-resource pressure aggregation
//! - Pressure-based throttling decisions
//! - Historical pressure trends
//! - Pressure propagation analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PRESSURE TYPES
// ============================================================================

/// Pressure resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PressureResource {
    /// CPU pressure
    Cpu,
    /// Memory pressure
    Memory,
    /// I/O pressure
    Io,
    /// IRQ pressure
    Irq,
    /// Network pressure
    Network,
}

/// Pressure category (PSI-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureCategory {
    /// Some: at least one task stalled
    Some,
    /// Full: all tasks stalled
    Full,
}

/// Pressure severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PressureSeverity {
    /// None
    None,
    /// Low
    Low,
    /// Moderate
    Moderate,
    /// High
    High,
    /// Critical
    Critical,
}

impl PressureSeverity {
    /// From percentage
    pub fn from_percentage(pct: f64) -> Self {
        if pct < 5.0 {
            Self::None
        } else if pct < 20.0 {
            Self::Low
        } else if pct < 50.0 {
            Self::Moderate
        } else if pct < 80.0 {
            Self::High
        } else {
            Self::Critical
        }
    }

    /// Requires action?
    pub fn requires_action(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

// ============================================================================
// PRESSURE METRICS
// ============================================================================

/// Pressure window (time-based averaging)
#[derive(Debug, Clone)]
pub struct PressureWindow {
    /// Window size (samples)
    pub window_size: usize,
    /// Total stall time in window (ns)
    stall_samples: Vec<u64>,
    /// Total time in window (ns)
    total_samples: Vec<u64>,
    /// Current index
    index: usize,
    /// Filled
    filled: bool,
}

impl PressureWindow {
    pub fn new(size: usize) -> Self {
        Self {
            window_size: size,
            stall_samples: alloc::vec![0u64; size],
            total_samples: alloc::vec![0u64; size],
            index: 0,
            filled: false,
        }
    }

    /// Record a sample
    pub fn record(&mut self, stall_ns: u64, total_ns: u64) {
        self.stall_samples[self.index] = stall_ns;
        self.total_samples[self.index] = total_ns;
        self.index += 1;
        if self.index >= self.window_size {
            self.index = 0;
            self.filled = true;
        }
    }

    /// Average pressure percentage
    pub fn average(&self) -> f64 {
        let count = if self.filled {
            self.window_size
        } else {
            self.index
        };
        if count == 0 {
            return 0.0;
        }

        let total_stall: u64 = self.stall_samples[..count].iter().sum();
        let total_time: u64 = self.total_samples[..count].iter().sum();

        if total_time == 0 {
            0.0
        } else {
            (total_stall as f64 / total_time as f64) * 100.0
        }
    }
}

/// Per-resource pressure tracker
#[derive(Debug, Clone)]
pub struct ResourcePressure {
    /// Resource
    pub resource: PressureResource,
    /// Short window (10s equivalent)
    pub short_window: PressureWindow,
    /// Medium window (60s equivalent)
    pub medium_window: PressureWindow,
    /// Long window (300s equivalent)
    pub long_window: PressureWindow,
    /// Total stall time (ns)
    pub total_stall_ns: u64,
    /// Total elapsed time (ns)
    pub total_elapsed_ns: u64,
    /// Peak pressure (short window)
    pub peak_pressure: f64,
    /// Current some %
    some_pct: f64,
    /// Current full %
    full_pct: f64,
}

impl ResourcePressure {
    pub fn new(resource: PressureResource) -> Self {
        Self {
            resource,
            short_window: PressureWindow::new(10),
            medium_window: PressureWindow::new(60),
            long_window: PressureWindow::new(300),
            total_stall_ns: 0,
            total_elapsed_ns: 0,
            peak_pressure: 0.0,
            some_pct: 0.0,
            full_pct: 0.0,
        }
    }

    /// Record pressure sample
    pub fn record(&mut self, some_stall_ns: u64, full_stall_ns: u64, total_ns: u64) {
        self.short_window.record(some_stall_ns, total_ns);
        self.medium_window.record(some_stall_ns, total_ns);
        self.long_window.record(some_stall_ns, total_ns);

        self.total_stall_ns += some_stall_ns;
        self.total_elapsed_ns += total_ns;

        self.some_pct = self.short_window.average();
        self.full_pct = if total_ns > 0 {
            (full_stall_ns as f64 / total_ns as f64) * 100.0
        } else {
            0.0
        };

        if self.some_pct > self.peak_pressure {
            self.peak_pressure = self.some_pct;
        }
    }

    /// Short-term pressure
    pub fn short_pressure(&self) -> f64 {
        self.short_window.average()
    }

    /// Medium-term pressure
    pub fn medium_pressure(&self) -> f64 {
        self.medium_window.average()
    }

    /// Long-term pressure
    pub fn long_pressure(&self) -> f64 {
        self.long_window.average()
    }

    /// Severity
    pub fn severity(&self) -> PressureSeverity {
        PressureSeverity::from_percentage(self.some_pct)
    }

    /// Trend (positive = increasing pressure)
    pub fn trend(&self) -> f64 {
        self.short_window.average() - self.long_window.average()
    }

    /// Is pressure increasing rapidly?
    pub fn is_spiking(&self) -> bool {
        self.short_window.average() > self.medium_window.average() * 2.0
            && self.short_window.average() > 10.0
    }
}

// ============================================================================
// PRESSURE EVENT
// ============================================================================

/// Pressure event
#[derive(Debug, Clone)]
pub struct PressureEvent {
    /// Resource
    pub resource: PressureResource,
    /// Category
    pub category: PressureCategory,
    /// Severity
    pub severity: PressureSeverity,
    /// Pressure percentage
    pub pressure_pct: f64,
    /// Threshold crossed
    pub threshold: f64,
    /// Timestamp
    pub timestamp: u64,
}

/// Throttle recommendation
#[derive(Debug, Clone)]
pub struct ThrottleRecommendation {
    /// Resource
    pub resource: PressureResource,
    /// Severity
    pub severity: PressureSeverity,
    /// Recommended throttle factor (0.0-1.0)
    pub throttle_factor: f64,
    /// Duration (ns)
    pub duration_ns: u64,
}

// ============================================================================
// PRESSURE ENGINE
// ============================================================================

/// Pressure stats
#[derive(Debug, Clone, Default)]
pub struct HolisticPressureStats {
    /// Resources tracked
    pub resources_tracked: usize,
    /// Events generated
    pub events_generated: u64,
    /// High pressure resources
    pub high_pressure_count: usize,
    /// Maximum pressure across resources
    pub max_pressure: f64,
    /// Throttle recommendations
    pub throttle_count: u64,
}

/// Holistic pressure analysis engine
pub struct HolisticPressureEngine {
    /// Per-resource pressure
    resources: BTreeMap<u8, ResourcePressure>,
    /// Event thresholds
    thresholds: BTreeMap<u8, f64>,
    /// Events
    events: Vec<PressureEvent>,
    /// Max events
    max_events: usize,
    /// Stats
    stats: HolisticPressureStats,
}

impl HolisticPressureEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            resources: BTreeMap::new(),
            thresholds: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            stats: HolisticPressureStats::default(),
        };

        let resources = [
            PressureResource::Cpu,
            PressureResource::Memory,
            PressureResource::Io,
            PressureResource::Irq,
            PressureResource::Network,
        ];

        for r in &resources {
            engine
                .resources
                .insert(*r as u8, ResourcePressure::new(*r));
            engine.thresholds.insert(*r as u8, 20.0);
        }

        engine.stats.resources_tracked = engine.resources.len();
        engine
    }

    /// Set threshold for resource
    pub fn set_threshold(&mut self, resource: PressureResource, threshold: f64) {
        self.thresholds.insert(resource as u8, threshold);
    }

    /// Record pressure sample
    pub fn record(
        &mut self,
        resource: PressureResource,
        some_stall_ns: u64,
        full_stall_ns: u64,
        total_ns: u64,
        now: u64,
    ) -> Option<PressureEvent> {
        let rp = self
            .resources
            .entry(resource as u8)
            .or_insert_with(|| ResourcePressure::new(resource));

        rp.record(some_stall_ns, full_stall_ns, total_ns);

        let pressure = rp.short_pressure();
        let threshold = self.thresholds.get(&(resource as u8)).copied().unwrap_or(20.0);

        let event = if pressure > threshold {
            let category = if rp.full_pct > threshold * 0.5 {
                PressureCategory::Full
            } else {
                PressureCategory::Some
            };

            let event = PressureEvent {
                resource,
                category,
                severity: rp.severity(),
                pressure_pct: pressure,
                threshold,
                timestamp: now,
            };

            self.events.push(event.clone());
            if self.events.len() > self.max_events {
                self.events.remove(0);
            }
            self.stats.events_generated += 1;

            Some(event)
        } else {
            None
        };

        self.update_stats();
        event
    }

    /// Get resource pressure
    pub fn resource_pressure(&self, resource: PressureResource) -> Option<&ResourcePressure> {
        self.resources.get(&(resource as u8))
    }

    /// Generate throttle recommendations
    pub fn throttle_recommendations(&mut self) -> Vec<ThrottleRecommendation> {
        let mut recs = Vec::new();

        for rp in self.resources.values() {
            let severity = rp.severity();
            if severity.requires_action() {
                let factor = match severity {
                    PressureSeverity::High => 0.7,
                    PressureSeverity::Critical => 0.3,
                    _ => 1.0,
                };

                let duration = if rp.is_spiking() {
                    100_000_000 // 100ms for spikes
                } else {
                    1_000_000_000 // 1s for sustained
                };

                recs.push(ThrottleRecommendation {
                    resource: rp.resource,
                    severity,
                    throttle_factor: factor,
                    duration_ns: duration,
                });
            }
        }

        self.stats.throttle_count += recs.len() as u64;
        recs
    }

    /// System-wide pressure summary
    pub fn system_pressure(&self) -> f64 {
        if self.resources.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.resources.values().map(|r| r.short_pressure()).sum();
        sum / self.resources.len() as f64
    }

    /// Most pressured resource
    pub fn most_pressured(&self) -> Option<(PressureResource, f64)> {
        self.resources
            .values()
            .max_by(|a, b| {
                a.short_pressure()
                    .partial_cmp(&b.short_pressure())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|r| (r.resource, r.short_pressure()))
    }

    fn update_stats(&mut self) {
        self.stats.high_pressure_count = self
            .resources
            .values()
            .filter(|r| r.severity().requires_action())
            .count();
        self.stats.max_pressure = self
            .resources
            .values()
            .map(|r| r.short_pressure())
            .fold(0.0_f64, |a, b| if a > b { a } else { b });
    }

    /// Stats
    pub fn stats(&self) -> &HolisticPressureStats {
        &self.stats
    }
}
