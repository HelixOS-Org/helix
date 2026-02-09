//! # Holistic Feedback Controller
//!
//! System-wide feedback loops for self-tuning:
//! - Multi-variable feedback
//! - Setpoint tracking
//! - Gain scheduling
//! - Stability analysis
//! - Cascade control

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// FEEDBACK TYPES
// ============================================================================

/// Feedback variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeedbackVariable {
    /// CPU utilization
    CpuUtilization,
    /// Memory pressure
    MemoryPressure,
    /// I/O throughput
    IoThroughput,
    /// Latency (p99)
    LatencyP99,
    /// Queue depth
    QueueDepth,
    /// Error rate
    ErrorRate,
    /// Throughput
    Throughput,
    /// Power consumption
    PowerConsumption,
}

/// Controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerType {
    /// Proportional only
    P,
    /// Proportional-Integral
    Pi,
    /// Proportional-Integral-Derivative
    Pid,
    /// On-Off (bang-bang)
    OnOff,
}

/// Gain schedule entry
#[derive(Debug, Clone)]
pub struct GainSchedule {
    /// Operating region lower bound
    pub region_lower: f64,
    /// Operating region upper bound
    pub region_upper: f64,
    /// Kp for this region
    pub kp: f64,
    /// Ki for this region
    pub ki: f64,
    /// Kd for this region
    pub kd: f64,
}

// ============================================================================
// FEEDBACK LOOP
// ============================================================================

/// Feedback controller state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FeedbackControllerState {
    /// Setpoint (target value)
    pub setpoint: f64,
    /// Current process value
    pub process_value: f64,
    /// Error
    pub error: f64,
    /// Previous error
    pub prev_error: f64,
    /// Integral term
    pub integral: f64,
    /// Derivative term
    pub derivative: f64,
    /// Controller output
    pub output: f64,
    /// Output minimum
    pub output_min: f64,
    /// Output maximum
    pub output_max: f64,
    /// Integral windup limit
    pub integral_limit: f64,
}

impl FeedbackControllerState {
    pub fn new(setpoint: f64, out_min: f64, out_max: f64) -> Self {
        Self {
            setpoint,
            process_value: 0.0,
            error: 0.0,
            prev_error: 0.0,
            integral: 0.0,
            derivative: 0.0,
            output: 0.0,
            output_min: out_min,
            output_max: out_max,
            integral_limit: (out_max - out_min) * 0.5,
        }
    }

    /// Clamp output
    fn clamp_output(&self, val: f64) -> f64 {
        if val < self.output_min {
            self.output_min
        } else if val > self.output_max {
            self.output_max
        } else {
            val
        }
    }
}

/// A feedback loop
#[derive(Debug)]
pub struct FeedbackLoop {
    /// Variable being controlled
    pub variable: FeedbackVariable,
    /// Controller type
    pub controller_type: ControllerType,
    /// Gains
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    /// State
    pub state: FeedbackControllerState,
    /// Gain schedules
    schedules: Vec<GainSchedule>,
    /// Update count
    pub update_count: u64,
    /// Settling time estimate
    pub settled: bool,
    /// Error history (recent)
    error_history: VecDeque<f64>,
    /// Max history
    max_history: usize,
}

impl FeedbackLoop {
    pub fn new(
        variable: FeedbackVariable,
        controller_type: ControllerType,
        kp: f64,
        ki: f64,
        kd: f64,
        setpoint: f64,
        out_min: f64,
        out_max: f64,
    ) -> Self {
        Self {
            variable,
            controller_type,
            kp,
            ki,
            kd,
            state: FeedbackControllerState::new(setpoint, out_min, out_max),
            schedules: Vec::new(),
            update_count: 0,
            settled: false,
            error_history: VecDeque::new(),
            max_history: 100,
        }
    }

    /// Add gain schedule
    #[inline(always)]
    pub fn add_schedule(&mut self, schedule: GainSchedule) {
        self.schedules.push(schedule);
    }

    /// Update with new process value
    pub fn update(&mut self, process_value: f64, dt: f64) -> f64 {
        self.state.process_value = process_value;
        self.state.error = self.state.setpoint - process_value;

        // Gain scheduling
        self.apply_gain_schedule(process_value);

        let output = match self.controller_type {
            ControllerType::P => {
                self.kp * self.state.error
            }
            ControllerType::Pi => {
                self.state.integral += self.state.error * dt;
                // Anti-windup
                if self.state.integral > self.state.integral_limit {
                    self.state.integral = self.state.integral_limit;
                } else if self.state.integral < -self.state.integral_limit {
                    self.state.integral = -self.state.integral_limit;
                }
                self.kp * self.state.error + self.ki * self.state.integral
            }
            ControllerType::Pid => {
                self.state.integral += self.state.error * dt;
                if self.state.integral > self.state.integral_limit {
                    self.state.integral = self.state.integral_limit;
                } else if self.state.integral < -self.state.integral_limit {
                    self.state.integral = -self.state.integral_limit;
                }
                if dt > 0.0 {
                    self.state.derivative = (self.state.error - self.state.prev_error) / dt;
                }
                self.kp * self.state.error
                    + self.ki * self.state.integral
                    + self.kd * self.state.derivative
            }
            ControllerType::OnOff => {
                if self.state.error > 0.0 {
                    self.state.output_max
                } else {
                    self.state.output_min
                }
            }
        };

        self.state.prev_error = self.state.error;
        self.state.output = self.state.clamp_output(output);
        self.update_count += 1;

        // Track error
        if self.error_history.len() >= self.max_history {
            self.error_history.pop_front();
        }
        self.error_history.push_back(self.state.error);

        // Check settling
        self.check_settled();

        self.state.output
    }

    /// Apply gain scheduling based on operating region
    fn apply_gain_schedule(&mut self, pv: f64) {
        for schedule in &self.schedules {
            if pv >= schedule.region_lower && pv < schedule.region_upper {
                self.kp = schedule.kp;
                self.ki = schedule.ki;
                self.kd = schedule.kd;
                return;
            }
        }
    }

    /// Check if settled (error within 2% band for 10 samples)
    fn check_settled(&mut self) {
        if self.error_history.len() < 10 {
            self.settled = false;
            return;
        }
        let band = libm::fabs(self.state.setpoint) * 0.02;
        let min_band = 0.01;
        let effective_band = if band < min_band { min_band } else { band };
        let len = self.error_history.len();
        self.settled = self.error_history[len - 10..]
            .iter()
            .all(|&e| libm::fabs(e) < effective_band);
    }

    /// Error variance (recent)
    pub fn error_variance(&self) -> f64 {
        if self.error_history.len() < 2 {
            return 0.0;
        }
        let n = self.error_history.len() as f64;
        let mean: f64 = self.error_history.iter().sum::<f64>() / n;
        let var: f64 = self.error_history.iter().map(|&e| {
            let d = e - mean;
            d * d
        }).sum::<f64>() / (n - 1.0);
        var
    }
}

// ============================================================================
// CASCADE CONTROL
// ============================================================================

/// Cascade controller (outer loop drives inner loop setpoint)
#[derive(Debug)]
pub struct CascadeController {
    /// Outer loop variable
    pub outer_variable: FeedbackVariable,
    /// Inner loop variable
    pub inner_variable: FeedbackVariable,
    /// Linkage factor (outer output * factor = inner setpoint)
    pub linkage_factor: f64,
}

// ============================================================================
// FEEDBACK ENGINE
// ============================================================================

/// Feedback stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticFeedbackStats {
    /// Active loops
    pub active_loops: usize,
    /// Settled loops
    pub settled_loops: usize,
    /// Total updates
    pub total_updates: u64,
}

/// Holistic feedback engine
pub struct HolisticFeedbackEngine {
    /// Feedback loops
    loops: BTreeMap<u8, FeedbackLoop>,
    /// Cascades
    cascades: Vec<CascadeController>,
    /// Stats
    stats: HolisticFeedbackStats,
}

impl HolisticFeedbackEngine {
    pub fn new() -> Self {
        Self {
            loops: BTreeMap::new(),
            cascades: Vec::new(),
            stats: HolisticFeedbackStats::default(),
        }
    }

    /// Add loop
    #[inline]
    pub fn add_loop(&mut self, feedback_loop: FeedbackLoop) {
        let key = feedback_loop.variable as u8;
        self.loops.insert(key, feedback_loop);
        self.update_stats();
    }

    /// Update a loop
    #[inline]
    pub fn update(&mut self, variable: FeedbackVariable, process_value: f64, dt: f64) -> Option<f64> {
        let key = variable as u8;
        if let Some(fl) = self.loops.get_mut(&key) {
            let output = fl.update(process_value, dt);
            self.update_stats();
            Some(output)
        } else {
            None
        }
    }

    /// Set setpoint
    #[inline]
    pub fn set_setpoint(&mut self, variable: FeedbackVariable, setpoint: f64) {
        let key = variable as u8;
        if let Some(fl) = self.loops.get_mut(&key) {
            fl.state.setpoint = setpoint;
        }
    }

    /// Add cascade
    #[inline(always)]
    pub fn add_cascade(&mut self, cascade: CascadeController) {
        self.cascades.push(cascade);
    }

    /// Process cascades (outer drives inner)
    pub fn process_cascades(&mut self) {
        // Collect outputs from outer loops
        let outputs: BTreeMap<u8, f64> = self.loops.iter()
            .map(|(&k, fl)| (k, fl.state.output))
            .collect();
        for cascade in &self.cascades {
            let outer_key = cascade.outer_variable as u8;
            let inner_key = cascade.inner_variable as u8;
            if let Some(&outer_output) = outputs.get(&outer_key) {
                if let Some(inner_loop) = self.loops.get_mut(&inner_key) {
                    inner_loop.state.setpoint = outer_output * cascade.linkage_factor;
                }
            }
        }
    }

    /// Are all loops settled?
    #[inline(always)]
    pub fn all_settled(&self) -> bool {
        self.loops.values().all(|fl| fl.settled)
    }

    fn update_stats(&mut self) {
        self.stats.active_loops = self.loops.len();
        self.stats.settled_loops = self.loops.values().filter(|fl| fl.settled).count();
        self.stats.total_updates = self.loops.values().map(|fl| fl.update_count).sum();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticFeedbackStats {
        &self.stats
    }
}
