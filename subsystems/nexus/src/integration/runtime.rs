//! NEXUS runtime implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::anomaly::{Anomaly, AnomalyDetector};
use crate::causal::CausalTracker;
use crate::chaos::ChaosEngine;
use crate::config::NexusConfig;
use crate::core::{ComponentId, Nexus, NexusLevel, NexusState, NexusTimestamp};
use crate::error::NexusResult;
use crate::event::{EventBus, NexusEvent, NexusEventKind};
use crate::heal::HealingEngine;
use crate::predict::{CrashPrediction, PredictionEngine};
use crate::quarantine::QuarantineSystem;

use super::health::{HealthCheckResult, HealthProbe, HealthStatus};
use super::hooks::SystemHook;
use super::metrics::{Metric, MetricExporter, MetricValue};

// ============================================================================
// RUNTIME STATS
// ============================================================================

/// Runtime statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Total ticks
    pub tick_count: u64,
    /// Current level
    pub level: NexusLevel,
    /// Current state
    pub state: NexusState,
    /// Predictions made
    pub predictions_made: u64,
    /// Healing success rate
    pub healing_success_rate: f32,
    /// Anomalies detected
    pub anomalies_detected: u64,
    /// Quarantined components
    pub quarantined_components: usize,
    /// Health probes registered
    pub health_probes: usize,
}

// ============================================================================
// NEXUS RUNTIME
// ============================================================================

/// The main NEXUS runtime integrating all subsystems
pub struct NexusRuntime {
    /// Core NEXUS state
    core: Nexus,
    /// Configuration
    config: NexusConfig,
    /// Prediction engine
    prediction: PredictionEngine,
    /// Healing engine
    healing: HealingEngine,
    /// Anomaly detector
    anomaly: AnomalyDetector,
    /// Chaos engine
    chaos: ChaosEngine,
    /// Tracer
    tracer: crate::trace::Tracer,
    /// Causal tracker
    causal: CausalTracker,
    /// Quarantine system
    quarantine: QuarantineSystem,
    /// Event bus
    events: EventBus,
    /// Health probes
    health_probes: Vec<Box<dyn HealthProbe>>,
    /// System hooks
    hooks: Vec<Box<dyn SystemHook>>,
    /// Metric exporters
    exporters: Vec<Box<dyn MetricExporter>>,
    /// Is runtime running
    running: AtomicBool,
    /// Tick counter
    tick_count: u64,
}

impl NexusRuntime {
    /// Create a new NEXUS runtime
    pub fn new(config: NexusConfig) -> Self {
        let tracer_config = crate::trace::TracerConfig {
            buffer_size: config.tracing.buffer_size,
            sample_rate: config.tracing.sample_rate,
            ..Default::default()
        };

        Self {
            core: Nexus::new(),
            config,
            prediction: PredictionEngine::default(),
            healing: HealingEngine::default(),
            anomaly: AnomalyDetector::default(),
            chaos: ChaosEngine::default(),
            tracer: crate::trace::Tracer::new(tracer_config),
            causal: CausalTracker::default(),
            quarantine: QuarantineSystem::default(),
            events: EventBus::new(),
            health_probes: Vec::new(),
            hooks: Vec::new(),
            exporters: Vec::new(),
            running: AtomicBool::new(false),
            tick_count: 0,
        }
    }

    /// Initialize the runtime
    pub fn init(&mut self) -> NexusResult<()> {
        self.core.init()?;

        // Initialize subsystems
        self.prediction.init_default_features();
        self.prediction.init_default_trees();

        // Enable based on config
        if !self.config.enabled {
            self.core.set_level(NexusLevel::Disabled);
        } else {
            self.core
                .set_level(NexusLevel::from_u8(self.config.level as u8));
        }

        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Shutdown the runtime
    pub fn shutdown(&mut self) -> NexusResult<()> {
        self.running.store(false, Ordering::SeqCst);
        self.core.shutdown()
    }

    /// Run a single tick
    pub fn tick(&mut self) -> NexusResult<()> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let now = NexusTimestamp::now();
        self.tick_count += 1;

        // Call hooks
        for hook in &self.hooks {
            hook.on_tick(now);
        }

        // Run health checks
        self.run_health_checks();

        // Run prediction
        let predictions = self.prediction.predict();
        for prediction in predictions {
            self.handle_prediction(prediction)?;
        }

        // Check for pending releases
        let released = self.quarantine.check_releases();
        for component in released {
            self.emit_event(NexusEventKind::ComponentHealthChanged {
                component,
                old_health: 0.0,
                new_health: 1.0,
            });
        }

        // Process events
        self.events.tick();

        // Update chaos engine
        self.chaos.tick();

        // Cleanup
        if self.tick_count % 100 == 0 {
            self.anomaly.clear_history();
        }

        Ok(())
    }

    /// Run health checks
    fn run_health_checks(&mut self) {
        for probe in &self.health_probes {
            let result = probe.check();

            // Update anomaly detector
            self.anomaly.record(
                &format!("health_{:?}", result.component),
                result.health as f64,
                Some(result.component),
            );

            // Update prediction features
            self.prediction
                .update_feature(2, 1.0 - result.health as f64); // memory_pressure as proxy

            // Check for unhealthy components
            if result.status == HealthStatus::Unhealthy {
                if !self.quarantine.is_quarantined(result.component) {
                    let entry = crate::quarantine::QuarantineEntry::new(
                        result.component,
                        crate::quarantine::QuarantineReason::LowHealth {
                            health: result.health,
                            threshold: 0.5,
                        },
                    );
                    self.quarantine.quarantine(entry);

                    self.emit_event(NexusEventKind::ComponentQuarantined {
                        component: result.component,
                        reason: result.message.unwrap_or_default(),
                    });
                }
            }
        }
    }

    /// Handle a prediction
    fn handle_prediction(&mut self, prediction: CrashPrediction) -> NexusResult<()> {
        // Emit event
        match prediction.kind {
            crate::predict::PredictionKind::Crash => {
                self.emit_event(NexusEventKind::CrashPredicted {
                    confidence: prediction.confidence.value(),
                    time_until_crash_ms: prediction.time_to_failure_ms,
                });
            }
            crate::predict::PredictionKind::OutOfMemory => {
                self.emit_event(NexusEventKind::MemoryLeakDetected {
                    component: prediction.component,
                    rate_bytes_per_sec: 0, // Would need to calculate
                });
            }
            crate::predict::PredictionKind::Deadlock => {
                self.emit_event(NexusEventKind::DeadlockPredicted {
                    components: prediction.component.map(|c| vec![c]).unwrap_or_default(),
                    confidence: prediction.confidence.value(),
                });
            }
            _ => {}
        }

        // Attempt healing if autonomous
        if self.core.level() >= NexusLevel::Autonomous && prediction.confidence.is_high() {
            let result = self.healing.heal_from_prediction(&prediction)?;

            if result.success {
                self.emit_event(NexusEventKind::HealingCompleted {
                    component: result.component,
                    success: true,
                    strategy_used: format!("{:?}", result.strategy),
                });
            }
        }

        Ok(())
    }

    /// Emit an event
    pub fn emit_event(&mut self, kind: NexusEventKind) {
        let event = NexusEvent::new(kind);
        self.events.emit(event);
    }

    /// Record an anomaly
    pub fn record_anomaly(
        &mut self,
        metric: &str,
        value: f64,
        component: Option<ComponentId>,
    ) -> Option<Anomaly> {
        self.anomaly.record(metric, value, component)
    }

    /// Register a health probe
    pub fn register_health_probe(&mut self, probe: Box<dyn HealthProbe>) {
        self.health_probes.push(probe);
    }

    /// Register a system hook
    pub fn register_hook(&mut self, hook: Box<dyn SystemHook>) {
        self.hooks.push(hook);
    }

    /// Register a metric exporter
    pub fn register_exporter(&mut self, exporter: Box<dyn MetricExporter>) {
        self.exporters.push(exporter);
    }

    /// Get current NEXUS level
    pub fn level(&self) -> NexusLevel {
        self.core.level()
    }

    /// Set NEXUS level
    pub fn set_level(&mut self, level: NexusLevel) {
        self.core.set_level(level);
    }

    /// Get current state
    pub fn state(&self) -> NexusState {
        self.core.state()
    }

    /// Get statistics
    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            tick_count: self.tick_count,
            level: self.core.level(),
            state: self.core.state(),
            predictions_made: self.prediction.recent_predictions().len() as u64,
            healing_success_rate: self.healing.success_rate(),
            anomalies_detected: self.anomaly.total_detected(),
            quarantined_components: self.quarantine.quarantined().len(),
            health_probes: self.health_probes.len(),
        }
    }

    /// Export metrics
    pub fn export_metrics(&self) -> NexusResult<Vec<Metric>> {
        let mut metrics = Vec::new();

        // Core metrics
        metrics.push(Metric {
            name: "nexus_ticks_total".into(),
            description: "Total NEXUS ticks".into(),
            value: MetricValue::Counter(self.tick_count),
            labels: Vec::new(),
            timestamp: NexusTimestamp::now(),
        });

        metrics.push(Metric {
            name: "nexus_level".into(),
            description: "Current NEXUS level".into(),
            value: MetricValue::Gauge(self.core.level() as u8 as f64),
            labels: Vec::new(),
            timestamp: NexusTimestamp::now(),
        });

        metrics.push(Metric {
            name: "nexus_healing_success_rate".into(),
            description: "Healing success rate".into(),
            value: MetricValue::Gauge(self.healing.success_rate() as f64),
            labels: Vec::new(),
            timestamp: NexusTimestamp::now(),
        });

        metrics.push(Metric {
            name: "nexus_quarantined_components".into(),
            description: "Number of quarantined components".into(),
            value: MetricValue::Gauge(self.quarantine.quarantined().len() as f64),
            labels: Vec::new(),
            timestamp: NexusTimestamp::now(),
        });

        // Export to all exporters
        for exporter in &self.exporters {
            exporter.export(&metrics)?;
        }

        Ok(metrics)
    }

    // Accessors for subsystems

    /// Get prediction engine
    pub fn prediction(&self) -> &PredictionEngine {
        &self.prediction
    }

    /// Get mutable prediction engine
    pub fn prediction_mut(&mut self) -> &mut PredictionEngine {
        &mut self.prediction
    }

    /// Get healing engine
    pub fn healing(&self) -> &HealingEngine {
        &self.healing
    }

    /// Get mutable healing engine
    pub fn healing_mut(&mut self) -> &mut HealingEngine {
        &mut self.healing
    }

    /// Get anomaly detector
    pub fn anomaly(&self) -> &AnomalyDetector {
        &self.anomaly
    }

    /// Get mutable anomaly detector
    pub fn anomaly_mut(&mut self) -> &mut AnomalyDetector {
        &mut self.anomaly
    }

    /// Get chaos engine
    pub fn chaos(&self) -> &ChaosEngine {
        &self.chaos
    }

    /// Get mutable chaos engine
    pub fn chaos_mut(&mut self) -> &mut ChaosEngine {
        &mut self.chaos
    }

    /// Get quarantine system
    pub fn quarantine(&self) -> &QuarantineSystem {
        &self.quarantine
    }

    /// Get mutable quarantine system
    pub fn quarantine_mut(&mut self) -> &mut QuarantineSystem {
        &mut self.quarantine
    }

    /// Get tracer
    pub fn tracer(&self) -> &crate::trace::Tracer {
        &self.tracer
    }

    /// Get mutable tracer
    pub fn tracer_mut(&mut self) -> &mut crate::trace::Tracer {
        &mut self.tracer
    }

    /// Get causal tracker
    pub fn causal(&self) -> &CausalTracker {
        &self.causal
    }

    /// Get mutable causal tracker
    pub fn causal_mut(&mut self) -> &mut CausalTracker {
        &mut self.causal
    }
}
