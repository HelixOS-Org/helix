//! # Holistic Adaptive Control
//!
//! Adaptive control loops for system-wide optimization:
//! - PID controllers
//! - Model predictive control
//! - Set-point tracking
//! - Feedback linearization
//! - Multi-variable control

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// CONTROL TYPES
// ============================================================================

/// Controlled variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlVariable {
    /// CPU utilization
    CpuUtilization,
    /// Memory pressure
    MemoryPressure,
    /// I/O latency
    IoLatency,
    /// Network throughput
    NetworkThroughput,
    /// Power consumption
    PowerConsumption,
    /// Temperature
    Temperature,
    /// Scheduling latency
    SchedulingLatency,
    /// Queue depth
    QueueDepth,
}

/// Control mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    /// Automatic (closed loop)
    Automatic,
    /// Manual (open loop)
    Manual,
    /// Cascade
    Cascade,
    /// Disabled
    Disabled,
}

// ============================================================================
// PID CONTROLLER
// ============================================================================

/// PID controller
#[derive(Debug, Clone)]
pub struct PidController {
    /// Variable
    pub variable: ControlVariable,
    /// Setpoint
    pub setpoint: f64,
    /// Proportional gain
    pub kp: f64,
    /// Integral gain
    pub ki: f64,
    /// Derivative gain
    pub kd: f64,
    /// Integral accumulator
    integral: f64,
    /// Previous error
    prev_error: f64,
    /// Output limits
    pub output_min: f64,
    pub output_max: f64,
    /// Anti-windup limit
    pub integral_limit: f64,
    /// Last output
    pub last_output: f64,
    /// Last update time
    pub last_update: u64,
}

impl PidController {
    pub fn new(variable: ControlVariable, setpoint: f64, kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            variable,
            setpoint,
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
            output_min: -1.0,
            output_max: 1.0,
            integral_limit: 10.0,
            last_output: 0.0,
            last_update: 0,
        }
    }

    /// Compute control output
    pub fn compute(&mut self, measured: f64, now: u64) -> f64 {
        let error = self.setpoint - measured;

        // Compute dt in seconds
        let dt = if self.last_update == 0 {
            0.001 // Default 1ms
        } else {
            let elapsed = now.saturating_sub(self.last_update);
            elapsed as f64 / 1_000_000_000.0
        };
        if dt <= 0.0 {
            return self.last_output;
        }

        // Proportional
        let p = self.kp * error;

        // Integral with anti-windup
        self.integral += error * dt;
        if self.integral > self.integral_limit {
            self.integral = self.integral_limit;
        } else if self.integral < -self.integral_limit {
            self.integral = -self.integral_limit;
        }
        let i = self.ki * self.integral;

        // Derivative
        let derivative = (error - self.prev_error) / dt;
        let d = self.kd * derivative;

        self.prev_error = error;
        self.last_update = now;

        // Total output with clamping
        let mut output = p + i + d;
        if output > self.output_max {
            output = self.output_max;
        }
        if output < self.output_min {
            output = self.output_min;
        }

        self.last_output = output;
        output
    }

    /// Reset controller state
    #[inline]
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
        self.last_output = 0.0;
        self.last_update = 0;
    }

    /// Current error
    #[inline(always)]
    pub fn error(&self) -> f64 {
        self.prev_error
    }
}

// ============================================================================
// CONTROL LOOP
// ============================================================================

/// Control loop state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlLoopState {
    /// Running
    Running,
    /// Paused
    Paused,
    /// Error
    Error,
}

/// A control loop
#[derive(Debug)]
pub struct ControlLoop {
    /// Name hash
    pub name_hash: u64,
    /// Controller
    pub controller: PidController,
    /// Mode
    pub mode: ControlMode,
    /// State
    pub state: ControlLoopState,
    /// Update interval (ns)
    pub interval_ns: u64,
    /// Last output
    pub last_output: f64,
    /// History of outputs
    pub output_history: VecDeque<f64>,
    /// Max history
    pub max_history: usize,
    /// Stability counter (consecutive outputs within threshold)
    pub stability_counter: u32,
}

impl ControlLoop {
    pub fn new(name_hash: u64, controller: PidController, interval_ns: u64) -> Self {
        Self {
            name_hash,
            controller,
            mode: ControlMode::Automatic,
            state: ControlLoopState::Running,
            interval_ns,
            last_output: 0.0,
            output_history: VecDeque::new(),
            max_history: 100,
            stability_counter: 0,
        }
    }

    /// Update with new measurement
    pub fn update(&mut self, measured: f64, now: u64) -> f64 {
        if self.mode != ControlMode::Automatic || self.state != ControlLoopState::Running {
            return self.last_output;
        }

        let output = self.controller.compute(measured, now);
        self.output_history.push_back(output);
        if self.output_history.len() > self.max_history {
            self.output_history.pop_front();
        }

        // Track stability
        let diff = libm::fabs(output - self.last_output);
        if diff < 0.01 {
            self.stability_counter += 1;
        } else {
            self.stability_counter = 0;
        }

        self.last_output = output;
        output
    }

    /// Is stable?
    #[inline(always)]
    pub fn is_stable(&self) -> bool {
        self.stability_counter > 10
    }

    /// Pause
    #[inline(always)]
    pub fn pause(&mut self) {
        self.state = ControlLoopState::Paused;
    }

    /// Resume
    #[inline(always)]
    pub fn resume(&mut self) {
        self.state = ControlLoopState::Running;
    }

    /// Output variance
    pub fn output_variance(&self) -> f64 {
        if self.output_history.len() < 2 {
            return 0.0;
        }
        let n = self.output_history.len() as f64;
        let mean: f64 = self.output_history.iter().sum::<f64>() / n;
        let variance: f64 = self
            .output_history
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f64>()
            / (n - 1.0);
        variance
    }
}

// ============================================================================
// ADAPTIVE CONTROLLER
// ============================================================================

/// Adaptive control manager
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticAdaptiveStats {
    /// Active loops
    pub active_loops: usize,
    /// Stable loops
    pub stable_loops: usize,
    /// Total updates
    pub total_updates: u64,
}

/// Holistic adaptive control engine
pub struct HolisticAdaptiveEngine {
    /// Control loops
    loops: BTreeMap<u64, ControlLoop>,
    /// Stats
    stats: HolisticAdaptiveStats,
}

impl HolisticAdaptiveEngine {
    pub fn new() -> Self {
        Self {
            loops: BTreeMap::new(),
            stats: HolisticAdaptiveStats::default(),
        }
    }

    /// Add control loop
    #[inline]
    pub fn add_loop(&mut self, name_hash: u64, controller: PidController, interval_ns: u64) {
        let cloop = ControlLoop::new(name_hash, controller, interval_ns);
        self.loops.insert(name_hash, cloop);
        self.stats.active_loops = self.loops.len();
    }

    /// Update loop with measurement
    #[inline]
    pub fn update(&mut self, name_hash: u64, measured: f64, now: u64) -> Option<f64> {
        if let Some(cloop) = self.loops.get_mut(&name_hash) {
            let output = cloop.update(measured, now);
            self.stats.total_updates += 1;
            self.stats.stable_loops = self.loops.values().filter(|l| l.is_stable()).count();
            Some(output)
        } else {
            None
        }
    }

    /// Set setpoint
    #[inline]
    pub fn set_setpoint(&mut self, name_hash: u64, setpoint: f64) {
        if let Some(cloop) = self.loops.get_mut(&name_hash) {
            cloop.controller.setpoint = setpoint;
        }
    }

    /// Get loop
    #[inline(always)]
    pub fn get_loop(&self, name_hash: u64) -> Option<&ControlLoop> {
        self.loops.get(&name_hash)
    }

    /// All stable?
    #[inline(always)]
    pub fn all_stable(&self) -> bool {
        self.loops.values().all(|l| l.is_stable())
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticAdaptiveStats {
        &self.stats
    }
}
