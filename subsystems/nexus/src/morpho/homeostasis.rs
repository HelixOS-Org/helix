//! Homeostasis controller for morphogenetic system.

extern crate alloc;

use alloc::collections::BTreeMap;

use super::types::MorphogenType;

/// Homeostasis controller
#[derive(Debug, Clone)]
pub struct HomeostasisController {
    /// Target setpoints for morphogens
    setpoints: BTreeMap<MorphogenType, f64>,
    /// PID gains
    kp: f64,
    ki: f64,
    kd: f64,
    /// Integral error accumulator
    integral: BTreeMap<MorphogenType, f64>,
    /// Previous error
    prev_error: BTreeMap<MorphogenType, f64>,
}

impl HomeostasisController {
    /// Create a new homeostasis controller
    pub fn new() -> Self {
        Self {
            setpoints: BTreeMap::new(),
            kp: 0.5,
            ki: 0.1,
            kd: 0.2,
            integral: BTreeMap::new(),
            prev_error: BTreeMap::new(),
        }
    }

    /// Set target setpoint
    pub fn set_target(&mut self, morph_type: MorphogenType, target: f64) {
        self.setpoints.insert(morph_type, target);
    }

    /// Calculate control signal
    pub fn control(&mut self, morph_type: MorphogenType, current: f64, dt: f64) -> f64 {
        let target = self.setpoints.get(&morph_type).copied().unwrap_or(1.0);
        let error = target - current;

        // Update integral
        let prev_integral = self.integral.get(&morph_type).copied().unwrap_or(0.0);
        let new_integral = prev_integral + error * dt;
        self.integral.insert(morph_type, new_integral);

        // Calculate derivative
        let prev_error = self.prev_error.get(&morph_type).copied().unwrap_or(error);
        let derivative = (error - prev_error) / dt;
        self.prev_error.insert(morph_type, error);

        // PID control
        self.kp * error + self.ki * new_integral + self.kd * derivative
    }
}

impl Default for HomeostasisController {
    fn default() -> Self {
        Self::new()
    }
}
