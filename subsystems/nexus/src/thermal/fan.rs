//! Fan control and monitoring
//!
//! This module provides fan information tracking and control
//! including RPM monitoring, PWM control, and speed percentage calculations.

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::types::CoolingDeviceId;

/// Fan speed mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanMode {
    /// Automatic (controlled by firmware)
    Auto,
    /// Manual
    Manual,
    /// Full speed
    FullSpeed,
}

impl FanMode {
    /// Get mode name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
            Self::FullSpeed => "full_speed",
        }
    }
}

/// Fan info
#[derive(Debug)]
pub struct FanInfo {
    /// Cooling device ID
    pub cooling_device: CoolingDeviceId,
    /// Current RPM
    rpm: AtomicU32,
    /// Minimum RPM
    pub min_rpm: u32,
    /// Maximum RPM
    pub max_rpm: u32,
    /// Mode
    pub mode: FanMode,
    /// PWM value (0-255)
    pwm: AtomicU32,
    /// Enabled
    enabled: AtomicBool,
}

impl FanInfo {
    /// Create new fan info
    pub fn new(cooling_device: CoolingDeviceId) -> Self {
        Self {
            cooling_device,
            rpm: AtomicU32::new(0),
            min_rpm: 0,
            max_rpm: 5000,
            mode: FanMode::Auto,
            pwm: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Get RPM
    #[inline(always)]
    pub fn rpm(&self) -> u32 {
        self.rpm.load(Ordering::Relaxed)
    }

    /// Set RPM
    #[inline(always)]
    pub fn set_rpm(&self, rpm: u32) {
        self.rpm.store(rpm, Ordering::Relaxed);
    }

    /// Get PWM
    #[inline(always)]
    pub fn pwm(&self) -> u32 {
        self.pwm.load(Ordering::Relaxed)
    }

    /// Set PWM
    #[inline(always)]
    pub fn set_pwm(&self, pwm: u32) {
        self.pwm.store(pwm.min(255), Ordering::Relaxed);
    }

    /// Speed percentage
    #[inline]
    pub fn speed_percentage(&self) -> f32 {
        if self.max_rpm == 0 {
            return 0.0;
        }
        (self.rpm() - self.min_rpm) as f32 / (self.max_rpm - self.min_rpm) as f32 * 100.0
    }

    /// Is spinning
    #[inline(always)]
    pub fn is_spinning(&self) -> bool {
        self.rpm() > 0
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Set enabled
    #[inline(always)]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}
