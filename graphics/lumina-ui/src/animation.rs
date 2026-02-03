//! # UI Animation System
//!
//! Hardware-accelerated animations with easing functions.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::{Color, WidgetId};

/// Animation controller
pub struct AnimationController {
    animations: BTreeMap<AnimationKey, AnimationInstance>,
    time: f64,
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            animations: BTreeMap::new(),
            time: 0.0,
        }
    }

    /// Update all animations
    pub fn update(&mut self, dt: f32) {
        self.time += dt as f64;

        // Remove completed animations
        self.animations
            .retain(|_, anim| anim.elapsed < anim.duration || anim.repeat != RepeatMode::None);

        // Update active animations
        for anim in self.animations.values_mut() {
            anim.elapsed += dt;

            if anim.elapsed >= anim.duration {
                match anim.repeat {
                    RepeatMode::None => {
                        anim.elapsed = anim.duration;
                    },
                    RepeatMode::Loop => {
                        anim.elapsed %= anim.duration;
                    },
                    RepeatMode::PingPong => {
                        anim.elapsed %= anim.duration * 2.0;
                    },
                    RepeatMode::Count(n) => {
                        if anim.repeat_count < n {
                            anim.elapsed %= anim.duration;
                            anim.repeat_count += 1;
                        } else {
                            anim.elapsed = anim.duration;
                        }
                    },
                }
            }
        }
    }

    /// Start a float animation
    pub fn animate_float(
        &mut self,
        id: WidgetId,
        property: &str,
        from: f32,
        to: f32,
        duration: f32,
        easing: Easing,
    ) {
        let key = AnimationKey {
            widget: id,
            property: hash_str(property),
        };

        self.animations.insert(key, AnimationInstance {
            value_type: AnimatedValue::Float { from, to },
            duration,
            elapsed: 0.0,
            delay: 0.0,
            easing,
            repeat: RepeatMode::None,
            repeat_count: 0,
        });
    }

    /// Start a color animation
    pub fn animate_color(
        &mut self,
        id: WidgetId,
        property: &str,
        from: Color,
        to: Color,
        duration: f32,
        easing: Easing,
    ) {
        let key = AnimationKey {
            widget: id,
            property: hash_str(property),
        };

        self.animations.insert(key, AnimationInstance {
            value_type: AnimatedValue::Color { from, to },
            duration,
            elapsed: 0.0,
            delay: 0.0,
            easing,
            repeat: RepeatMode::None,
            repeat_count: 0,
        });
    }

    /// Get current float value
    pub fn get_float(&self, id: WidgetId, property: &str) -> Option<f32> {
        let key = AnimationKey {
            widget: id,
            property: hash_str(property),
        };

        self.animations.get(&key).and_then(|anim| {
            if let AnimatedValue::Float { from, to } = anim.value_type {
                let t = anim.progress();
                let eased = anim.easing.apply(t);
                Some(from + (to - from) * eased)
            } else {
                None
            }
        })
    }

    /// Get current color value
    pub fn get_color(&self, id: WidgetId, property: &str) -> Option<Color> {
        let key = AnimationKey {
            widget: id,
            property: hash_str(property),
        };

        self.animations.get(&key).and_then(|anim| {
            if let AnimatedValue::Color { from, to } = anim.value_type {
                let t = anim.progress();
                let eased = anim.easing.apply(t);
                Some(from.lerp(to, eased))
            } else {
                None
            }
        })
    }

    /// Check if animation is running
    pub fn is_running(&self, id: WidgetId, property: &str) -> bool {
        let key = AnimationKey {
            widget: id,
            property: hash_str(property),
        };
        self.animations.contains_key(&key)
    }

    /// Cancel an animation
    pub fn cancel(&mut self, id: WidgetId, property: &str) {
        let key = AnimationKey {
            widget: id,
            property: hash_str(property),
        };
        self.animations.remove(&key);
    }

    /// Cancel all animations for a widget
    pub fn cancel_all(&mut self, id: WidgetId) {
        self.animations.retain(|k, _| k.widget != id);
    }
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation key
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AnimationKey {
    widget: WidgetId,
    property: u64,
}

/// Animation instance
struct AnimationInstance {
    value_type: AnimatedValue,
    duration: f32,
    elapsed: f32,
    delay: f32,
    easing: Easing,
    repeat: RepeatMode,
    repeat_count: u32,
}

impl AnimationInstance {
    fn progress(&self) -> f32 {
        if self.elapsed < self.delay {
            return 0.0;
        }

        let t = (self.elapsed - self.delay) / self.duration;

        match self.repeat {
            RepeatMode::PingPong => {
                let cycle = t.floor() as u32;
                let frac = t.fract();
                if cycle % 2 == 0 {
                    frac
                } else {
                    1.0 - frac
                }
            },
            _ => t.clamp(0.0, 1.0),
        }
    }
}

/// Animated value type
#[derive(Debug, Clone, Copy)]
enum AnimatedValue {
    Float { from: f32, to: f32 },
    Float2 { from: [f32; 2], to: [f32; 2] },
    Float4 { from: [f32; 4], to: [f32; 4] },
    Color { from: Color, to: Color },
}

/// Repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    None,
    Loop,
    PingPong,
    Count(u32),
}

/// Easing function
#[derive(Debug, Clone, Copy)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    Spring { stiffness: f32, damping: f32 },
}

impl Easing {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            },
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => 1.0 - (1.0 - t).powi(2),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            },
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            },
            Easing::EaseInQuart => t * t * t * t,
            Easing::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            Easing::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t.powi(4)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            },
            Easing::EaseInQuint => t.powi(5),
            Easing::EaseOutQuint => 1.0 - (1.0 - t).powi(5),
            Easing::EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t.powi(5)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            },
            Easing::EaseInSine => 1.0 - (t * core::f32::consts::FRAC_PI_2).cos(),
            Easing::EaseOutSine => (t * core::f32::consts::FRAC_PI_2).sin(),
            Easing::EaseInOutSine => -((t * core::f32::consts::PI).cos() - 1.0) / 2.0,
            Easing::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2.0f32.powf(10.0 * t - 10.0)
                }
            },
            Easing::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0f32.powf(-10.0 * t)
                }
            },
            Easing::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    2.0f32.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2.0f32.powf(-20.0 * t + 10.0)) / 2.0
                }
            },
            Easing::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Easing::EaseOutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
            Easing::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            },
            Easing::EaseInBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            },
            Easing::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            },
            Easing::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            },
            Easing::EaseInElastic => {
                let c4 = (2.0 * core::f32::consts::PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    -2.0f32.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * c4).sin()
                }
            },
            Easing::EaseOutElastic => {
                let c4 = (2.0 * core::f32::consts::PI) / 3.0;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    2.0f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            },
            Easing::EaseInOutElastic => {
                let c5 = (2.0 * core::f32::consts::PI) / 4.5;
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    -(2.0f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
                } else {
                    (2.0f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0 + 1.0
                }
            },
            Easing::EaseInBounce => 1.0 - Easing::EaseOutBounce.apply(1.0 - t),
            Easing::EaseOutBounce => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            },
            Easing::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - Easing::EaseOutBounce.apply(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + Easing::EaseOutBounce.apply(2.0 * t - 1.0)) / 2.0
                }
            },
            Easing::Spring { stiffness, damping } => {
                // Simplified spring physics
                let omega = stiffness.sqrt();
                let zeta = *damping / (2.0 * omega);
                if zeta < 1.0 {
                    // Underdamped
                    let omega_d = omega * (1.0 - zeta * zeta).sqrt();
                    1.0 - (-zeta * omega * t).exp() * (omega_d * t).cos()
                } else {
                    // Overdamped
                    1.0 - (-omega * t).exp()
                }
            },
        }
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hash = 0u64;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

/// Animation sequence builder
pub struct AnimationSequence {
    steps: Vec<AnimationStep>,
    current: usize,
    elapsed: f32,
}

impl AnimationSequence {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            current: 0,
            elapsed: 0.0,
        }
    }

    pub fn then(mut self, step: AnimationStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn delay(mut self, duration: f32) -> Self {
        self.steps.push(AnimationStep::Delay(duration));
        self
    }

    pub fn parallel(mut self, steps: Vec<AnimationStep>) -> Self {
        self.steps.push(AnimationStep::Parallel(steps));
        self
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if self.current >= self.steps.len() {
            return true; // Complete
        }

        self.elapsed += dt;

        match &self.steps[self.current] {
            AnimationStep::Delay(duration) => {
                if self.elapsed >= *duration {
                    self.current += 1;
                    self.elapsed = 0.0;
                }
            },
            AnimationStep::Animate { duration, .. } => {
                if self.elapsed >= *duration {
                    self.current += 1;
                    self.elapsed = 0.0;
                }
            },
            AnimationStep::Parallel(steps) => {
                let max_duration = steps
                    .iter()
                    .filter_map(|s| match s {
                        AnimationStep::Animate { duration, .. } => Some(*duration),
                        AnimationStep::Delay(d) => Some(*d),
                        _ => None,
                    })
                    .fold(0.0f32, |a, b| a.max(b));

                if self.elapsed >= max_duration {
                    self.current += 1;
                    self.elapsed = 0.0;
                }
            },
            AnimationStep::Callback(_) => {
                self.current += 1;
                self.elapsed = 0.0;
            },
        }

        false
    }
}

impl Default for AnimationSequence {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation step
pub enum AnimationStep {
    Delay(f32),
    Animate {
        widget: WidgetId,
        property: u64,
        from: f32,
        to: f32,
        duration: f32,
        easing: Easing,
    },
    Parallel(Vec<AnimationStep>),
    Callback(fn()),
}
