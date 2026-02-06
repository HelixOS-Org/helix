//! Boot Animation and Splash Screen System for Helix UEFI Bootloader
//!
//! This module provides comprehensive boot animation support including
//! splash screens, progress indicators, and animated logos.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Animation System                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Animation Engine                              │   │
//! │  │  Keyframes │ Easing │ Interpolation │ Timing                    │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Sprite System                                 │   │
//! │  │  Sprites │ Atlas │ Frames │ Layers                              │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Effects                                       │   │
//! │  │  Fade │ Slide │ Zoom │ Rotate │ Particle                        │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Rendering                                     │   │
//! │  │  Compositor │ Blending │ Anti-aliasing │ Double Buffer          │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

// =============================================================================
// ANIMATION TYPES
// =============================================================================

/// Animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    /// Animation not started
    Stopped,
    /// Animation playing
    Playing,
    /// Animation paused
    Paused,
    /// Animation finished
    Finished,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Animation loop mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    /// Play once
    Once,
    /// Loop forever
    Loop,
    /// Loop N times
    Count(u32),
    /// Ping-pong (forward then backward)
    PingPong,
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::Once
    }
}

/// Animation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationDirection {
    /// Forward
    Forward,
    /// Backward
    Backward,
    /// Alternate
    Alternate,
    /// Alternate reverse
    AlternateReverse,
}

impl Default for AnimationDirection {
    fn default() -> Self {
        Self::Forward
    }
}

// =============================================================================
// EASING FUNCTIONS
// =============================================================================

/// Easing function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Easing {
    /// Linear (no easing)
    Linear,
    /// Ease in (accelerate)
    EaseIn,
    /// Ease out (decelerate)
    EaseOut,
    /// Ease in-out
    EaseInOut,
    /// Quadratic ease in
    QuadIn,
    /// Quadratic ease out
    QuadOut,
    /// Quadratic ease in-out
    QuadInOut,
    /// Cubic ease in
    CubicIn,
    /// Cubic ease out
    CubicOut,
    /// Cubic ease in-out
    CubicInOut,
    /// Quartic ease in
    QuartIn,
    /// Quartic ease out
    QuartOut,
    /// Quartic ease in-out
    QuartInOut,
    /// Quintic ease in
    QuintIn,
    /// Quintic ease out
    QuintOut,
    /// Quintic ease in-out
    QuintInOut,
    /// Sinusoidal ease in
    SineIn,
    /// Sinusoidal ease out
    SineOut,
    /// Sinusoidal ease in-out
    SineInOut,
    /// Exponential ease in
    ExpoIn,
    /// Exponential ease out
    ExpoOut,
    /// Exponential ease in-out
    ExpoInOut,
    /// Circular ease in
    CircIn,
    /// Circular ease out
    CircOut,
    /// Circular ease in-out
    CircInOut,
    /// Elastic ease in
    ElasticIn,
    /// Elastic ease out
    ElasticOut,
    /// Elastic ease in-out
    ElasticInOut,
    /// Back ease in (overshoot)
    BackIn,
    /// Back ease out
    BackOut,
    /// Back ease in-out
    BackInOut,
    /// Bounce ease in
    BounceIn,
    /// Bounce ease out
    BounceOut,
    /// Bounce ease in-out
    BounceInOut,
}

impl Default for Easing {
    fn default() -> Self {
        Self::Linear
    }
}

impl Easing {
    /// Apply easing function to value t (0.0 to 1.0)
    /// Returns value in range 0.0 to 1.0 (may exceed for elastic/back)
    #[must_use]
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => t * (2.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            },
            Self::QuadIn => t * t,
            Self::QuadOut => t * (2.0 - t),
            Self::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            },
            Self::CubicIn => t * t * t,
            Self::CubicOut => {
                let t1 = t - 1.0;
                t1 * t1 * t1 + 1.0
            },
            Self::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t * t
                } else {
                    let t1 = 2.0 * t - 2.0;
                    (t1 * t1 * t1 + 2.0) / 2.0
                }
            },
            Self::QuartIn => t * t * t * t,
            Self::QuartOut => {
                let t1 = t - 1.0;
                1.0 - t1 * t1 * t1 * t1
            },
            Self::QuartInOut => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    let t1 = t - 1.0;
                    1.0 - 8.0 * t1 * t1 * t1 * t1
                }
            },
            Self::QuintIn => t * t * t * t * t,
            Self::QuintOut => {
                let t1 = t - 1.0;
                1.0 + t1 * t1 * t1 * t1 * t1
            },
            Self::QuintInOut => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    let t1 = 2.0 * t - 2.0;
                    (t1 * t1 * t1 * t1 * t1 + 2.0) / 2.0
                }
            },
            // Approximations for trig-based easing (no libm)
            Self::SineIn => 1.0 - Self::cos_approx(t * core::f32::consts::FRAC_PI_2),
            Self::SineOut => Self::sin_approx(t * core::f32::consts::FRAC_PI_2),
            Self::SineInOut => -(Self::cos_approx(core::f32::consts::PI * t) - 1.0) / 2.0,
            Self::ExpoIn => {
                if t == 0.0 {
                    0.0
                } else {
                    Self::pow2_approx(10.0 * (t - 1.0))
                }
            },
            Self::ExpoOut =>
            {
                #[expect(
                    clippy::float_cmp,
                    reason = "Exact comparison with 1.0 is intentional for boundary condition"
                )]
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - Self::pow2_approx(-10.0 * t)
                }
            },
            Self::ExpoInOut => {
                #[allow(clippy::float_cmp)]
                if t == 0.0 {
                    return 0.0;
                }
                #[allow(clippy::float_cmp)]
                if t == 1.0 {
                    return 1.0;
                }
                if t < 0.5 {
                    Self::pow2_approx(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - Self::pow2_approx(-20.0 * t + 10.0)) / 2.0
                }
            },
            Self::CircIn => 1.0 - Self::sqrt_approx(1.0 - t * t),
            Self::CircOut => Self::sqrt_approx(1.0 - (t - 1.0) * (t - 1.0)),
            Self::CircInOut => {
                if t < 0.5 {
                    (1.0 - Self::sqrt_approx(1.0 - 4.0 * t * t)) / 2.0
                } else {
                    let t1 = 2.0 * t - 2.0;
                    (Self::sqrt_approx(1.0 - t1 * t1) + 1.0) / 2.0
                }
            },
            Self::ElasticIn => {
                #[allow(clippy::float_cmp)] // Exact comparison with boundary values is intentional
                if t == 0.0 {
                    return 0.0;
                }
                #[allow(clippy::float_cmp)] // Exact comparison with boundary values is intentional
                if t == 1.0 {
                    return 1.0;
                }
                let c4 = 2.0 * core::f32::consts::PI / 3.0;
                -Self::pow2_approx(10.0 * t - 10.0) * Self::sin_approx((t * 10.0 - 10.75) * c4)
            },
            Self::ElasticOut => {
                #[allow(clippy::float_cmp)] // Exact comparison with boundary values is intentional
                if t == 0.0 {
                    return 0.0;
                }
                #[allow(clippy::float_cmp)] // Exact comparison with boundary values is intentional
                if t == 1.0 {
                    return 1.0;
                }
                let c4 = 2.0 * core::f32::consts::PI / 3.0;
                Self::pow2_approx(-10.0 * t) * Self::sin_approx((t * 10.0 - 0.75) * c4) + 1.0
            },
            Self::ElasticInOut => {
                #[allow(clippy::float_cmp)] // Exact comparison with boundary values is intentional
                if t == 0.0 {
                    return 0.0;
                }
                #[allow(clippy::float_cmp)] // Exact comparison with boundary values is intentional
                if t == 1.0 {
                    return 1.0;
                }
                let c5 = 2.0 * core::f32::consts::PI / 4.5;
                if t < 0.5 {
                    -(Self::pow2_approx(20.0 * t - 10.0)
                        * Self::sin_approx((20.0 * t - 11.125) * c5))
                        / 2.0
                } else {
                    Self::pow2_approx(-20.0 * t + 10.0) * Self::sin_approx((20.0 * t - 11.125) * c5)
                        / 2.0
                        + 1.0
                }
            },
            Self::BackIn => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            },
            Self::BackOut => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                let t1 = t - 1.0;
                1.0 + c3 * t1 * t1 * t1 + c1 * t1 * t1
            },
            Self::BackInOut => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    (4.0 * t * t * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    let t1 = 2.0 * t - 2.0;
                    (t1 * t1 * ((c2 + 1.0) * t1 + c2) + 2.0) / 2.0
                }
            },
            Self::BounceIn => 1.0 - Self::bounce_out(1.0 - t),
            Self::BounceOut => Self::bounce_out(t),
            Self::BounceInOut => {
                if t < 0.5 {
                    (1.0 - Self::bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + Self::bounce_out(2.0 * t - 1.0)) / 2.0
                }
            },
        }
    }

    /// Bounce out helper
    fn bounce_out(t: f32) -> f32 {
        let n1 = 7.5625;
        let d1 = 2.75;
        if t < 1.0 / d1 {
            n1 * t * t
        } else if t < 2.0 / d1 {
            let t1 = t - 1.5 / d1;
            n1 * t1 * t1 + 0.75
        } else if t < 2.5 / d1 {
            let t1 = t - 2.25 / d1;
            n1 * t1 * t1 + 0.9375
        } else {
            let t1 = t - 2.625 / d1;
            n1 * t1 * t1 + 0.984_375
        }
    }

    /// Approximate sine for `no_std` environments.
    ///
    /// Uses Taylor series approximation.
    fn sin_approx(x: f32) -> f32 {
        // Taylor series approximation
        let x = x % (2.0 * core::f32::consts::PI);
        let x3 = x * x * x;
        let x5 = x3 * x * x;
        let x7 = x5 * x * x;
        x - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0
    }

    /// Approximate cosine for `no_std` environments.
    ///
    /// Uses sine approximation with phase shift.
    fn cos_approx(x: f32) -> f32 {
        Self::sin_approx(x + core::f32::consts::FRAC_PI_2)
    }

    /// Approximate 2^x for `no_std` environments.
    ///
    /// Uses bit manipulation and polynomial approximation.
    #[expect(
        clippy::cast_precision_loss,
        reason = "Precision loss is acceptable for approximation"
    )]
    fn pow2_approx(x: f32) -> f32 {
        // Simple approximation
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Truncation is intentional for integer part extraction"
        )]
        let xi = x as i32;
        let xf = x - xi as f32;
        let base = if xi >= 0 {
            #[expect(clippy::cast_sign_loss, reason = "xi is checked to be non-negative")]
            let shift = xi.min(31) as u32;
            (1u32 << shift) as f32
        } else {
            #[expect(clippy::cast_sign_loss, reason = "negated value is positive")]
            let shift = (-xi).min(31) as u32;
            1.0 / (1u32 << shift) as f32
        };
        base * (1.0 + core::f32::consts::LN_2 * xf + 0.240_226_5 * xf * xf)
    }

    /// Approximate square root for `no_std` environments.
    ///
    /// Uses Newton-Raphson iteration.
    fn sqrt_approx(x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        // Newton-Raphson iteration
        let mut guess = x / 2.0;
        for _ in 0..5 {
            guess = (guess + x / guess) / 2.0;
        }
        guess
    }
}

// =============================================================================
// KEYFRAME
// =============================================================================

/// Keyframe for animation
#[derive(Debug, Clone, Copy)]
pub struct Keyframe<T: Copy> {
    /// Time offset (0.0 to 1.0)
    pub time: f32,
    /// Value at this keyframe
    pub value: T,
    /// Easing to next keyframe
    pub easing: Easing,
}

impl<T: Copy> Keyframe<T> {
    /// Create new keyframe
    pub const fn new(time: f32, value: T) -> Self {
        Self {
            time,
            value,
            easing: Easing::Linear,
        }
    }

    /// With easing
    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

// =============================================================================
// ANIMATION TIMELINE
// =============================================================================

/// Animation timeline
#[derive(Debug, Clone, Copy)]
pub struct Timeline {
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Loop mode
    pub loop_mode: LoopMode,
    /// Direction
    pub direction: AnimationDirection,
    /// Delay before starting (ms)
    pub delay_ms: u32,
}

impl Timeline {
    /// Create new timeline
    pub const fn new(duration_ms: u32) -> Self {
        Self {
            duration_ms,
            loop_mode: LoopMode::Once,
            direction: AnimationDirection::Forward,
            delay_ms: 0,
        }
    }

    /// With loop mode
    #[must_use]
    pub const fn with_loop(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// With delay
    #[must_use]
    pub const fn with_delay(mut self, delay_ms: u32) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new(1000)
    }
}

// =============================================================================
// TRANSFORM
// =============================================================================

/// 2D Point
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Point2D {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}

impl Point2D {
    /// Zero point (origin)
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Create a new point
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D Size
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size2D {
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

impl Size2D {
    /// Create a new size
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// 2D Transform
#[derive(Debug, Clone, Copy)]
pub struct Transform2D {
    /// Translation
    pub translation: Point2D,
    /// Scale (1.0 = 100%)
    pub scale: Point2D,
    /// Rotation in radians
    pub rotation: f32,
    /// Transform origin
    pub origin: Point2D,
    /// Skew
    pub skew: Point2D,
}

impl Transform2D {
    /// Identity transform
    pub const IDENTITY: Self = Self {
        translation: Point2D::ZERO,
        scale: Point2D { x: 1.0, y: 1.0 },
        rotation: 0.0,
        origin: Point2D::ZERO,
        skew: Point2D::ZERO,
    };

    /// Create translation transform
    pub const fn translate(x: f32, y: f32) -> Self {
        Self {
            translation: Point2D::new(x, y),
            ..Self::IDENTITY
        }
    }

    /// Create scale transform
    pub const fn scale(sx: f32, sy: f32) -> Self {
        Self {
            scale: Point2D::new(sx, sy),
            ..Self::IDENTITY
        }
    }

    /// Create rotation transform
    pub const fn rotate(angle: f32) -> Self {
        Self {
            rotation: angle,
            ..Self::IDENTITY
        }
    }

    /// Interpolate between two transforms
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            translation: Point2D::new(
                self.translation.x + (other.translation.x - self.translation.x) * t,
                self.translation.y + (other.translation.y - self.translation.y) * t,
            ),
            scale: Point2D::new(
                self.scale.x + (other.scale.x - self.scale.x) * t,
                self.scale.y + (other.scale.y - self.scale.y) * t,
            ),
            rotation: self.rotation + (other.rotation - self.rotation) * t,
            origin: Point2D::new(
                self.origin.x + (other.origin.x - self.origin.x) * t,
                self.origin.y + (other.origin.y - self.origin.y) * t,
            ),
            skew: Point2D::new(
                self.skew.x + (other.skew.x - self.skew.x) * t,
                self.skew.y + (other.skew.y - self.skew.y) * t,
            ),
        }
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// =============================================================================
// COLOR ANIMATION
// =============================================================================

/// RGBA Color for animation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnimColor {
    /// Red component
    pub r: u8,
    /// Green component
    pub g: u8,
    /// Blue component
    pub b: u8,
    /// Alpha component
    pub a: u8,
}

impl AnimColor {
    /// Transparent color
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    /// Black color
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    /// White color
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    /// Create new color
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create from RGB (alpha = 255)
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Interpolate between two colors
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "Clamped values are within u8 range"
        )]
        #[expect(clippy::cast_sign_loss, reason = "Clamped values are non-negative")]
        Self {
            r: (f32::from(self.r) + (f32::from(other.r) - f32::from(self.r)) * t) as u8,
            g: (f32::from(self.g) + (f32::from(other.g) - f32::from(self.g)) * t) as u8,
            b: (f32::from(self.b) + (f32::from(other.b) - f32::from(self.b)) * t) as u8,
            a: (f32::from(self.a) + (f32::from(other.a) - f32::from(self.a)) * t) as u8,
        }
    }

    /// With alpha
    #[must_use]
    pub const fn with_alpha(mut self, alpha: u8) -> Self {
        self.a = alpha;
        self
    }
}

// =============================================================================
// SPRITE
// =============================================================================

/// Sprite definition
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    /// Source X in texture atlas
    pub src_x: u32,
    /// Source Y in texture atlas
    pub src_y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Hotspot X (pivot point)
    pub hotspot_x: i32,
    /// Hotspot Y (pivot point)
    pub hotspot_y: i32,
}

impl Sprite {
    /// Create new sprite
    pub const fn new(src_x: u32, src_y: u32, width: u32, height: u32) -> Self {
        Self {
            src_x,
            src_y,
            width,
            height,
            hotspot_x: 0,
            hotspot_y: 0,
        }
    }

    /// With hotspot (pivot point)
    #[must_use]
    pub const fn with_hotspot(mut self, x: i32, y: i32) -> Self {
        self.hotspot_x = x;
        self.hotspot_y = y;
        self
    }

    /// With centered hotspot
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "Sprite dimensions are small enough to fit in i32"
    )]
    pub const fn centered(mut self) -> Self {
        self.hotspot_x = (self.width / 2) as i32;
        self.hotspot_y = (self.height / 2) as i32;
        self
    }
}

/// Sprite animation frame
#[derive(Debug, Clone, Copy)]
pub struct SpriteFrame {
    /// Sprite for this frame
    pub sprite: Sprite,
    /// Duration in milliseconds
    pub duration_ms: u32,
}

impl SpriteFrame {
    /// Create new frame
    pub const fn new(sprite: Sprite, duration_ms: u32) -> Self {
        Self {
            sprite,
            duration_ms,
        }
    }
}

// =============================================================================
// PARTICLE SYSTEM
// =============================================================================

/// Particle properties
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    /// Position
    pub position: Point2D,
    /// Velocity
    pub velocity: Point2D,
    /// Acceleration
    pub acceleration: Point2D,
    /// Color
    pub color: AnimColor,
    /// Size
    pub size: f32,
    /// Lifetime remaining (ms)
    pub lifetime: u32,
    /// Initial lifetime (ms)
    pub initial_lifetime: u32,
    /// Rotation
    pub rotation: f32,
    /// Angular velocity
    pub angular_velocity: f32,
}

impl Particle {
    /// Check if particle is alive
    pub const fn is_alive(&self) -> bool {
        self.lifetime > 0
    }

    /// Get normalized age (0.0 = born, 1.0 = dead)
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "Precision loss is acceptable for age calculation"
    )]
    pub fn age(&self) -> f32 {
        if self.initial_lifetime == 0 {
            return 1.0;
        }
        1.0 - (self.lifetime as f32 / self.initial_lifetime as f32)
    }
}

/// Particle emitter shape
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitterShape {
    /// Point emitter
    Point,
    /// Line emitter
    Line,
    /// Circle emitter
    Circle,
    /// Rectangle emitter
    Rectangle,
    /// Ring emitter
    Ring,
}

impl Default for EmitterShape {
    fn default() -> Self {
        Self::Point
    }
}

/// Particle emitter configuration
#[derive(Debug, Clone, Copy)]
pub struct ParticleEmitterConfig {
    /// Emitter shape
    pub shape: EmitterShape,
    /// Shape size (radius for circle, dimensions for rect)
    pub shape_size: Size2D,
    /// Emission rate (particles per second)
    pub emission_rate: f32,
    /// Particle minimum lifetime (ms)
    pub lifetime_min: u32,
    /// Particle maximum lifetime (ms)
    pub lifetime_max: u32,
    /// Initial velocity minimum
    pub velocity_min: Point2D,
    /// Initial velocity maximum
    pub velocity_max: Point2D,
    /// Acceleration (gravity)
    pub acceleration: Point2D,
    /// Initial size minimum
    pub size_min: f32,
    /// Initial size maximum
    pub size_max: f32,
    /// Size over lifetime multiplier
    pub size_end: f32,
    /// Start color
    pub color_start: AnimColor,
    /// End color
    pub color_end: AnimColor,
    /// Max particles
    pub max_particles: u32,
}

impl Default for ParticleEmitterConfig {
    fn default() -> Self {
        Self {
            shape: EmitterShape::Point,
            shape_size: Size2D::new(0.0, 0.0),
            emission_rate: 10.0,
            lifetime_min: 1000,
            lifetime_max: 2000,
            velocity_min: Point2D::new(-50.0, -100.0),
            velocity_max: Point2D::new(50.0, -50.0),
            acceleration: Point2D::new(0.0, 100.0),
            size_min: 2.0,
            size_max: 4.0,
            size_end: 0.0,
            color_start: AnimColor::WHITE,
            color_end: AnimColor::TRANSPARENT,
            max_particles: 100,
        }
    }
}

// =============================================================================
// EFFECTS
// =============================================================================

/// Transition effect type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionEffect {
    /// No transition
    None,
    /// Fade in/out
    Fade,
    /// Slide from direction
    Slide,
    /// Zoom in/out
    Zoom,
    /// Wipe effect
    Wipe,
    /// Dissolve effect
    Dissolve,
    /// Push (new pushes old)
    Push,
    /// Flip 3D
    Flip,
    /// Cube rotation
    Cube,
}

impl Default for TransitionEffect {
    fn default() -> Self {
        Self::Fade
    }
}

/// Transition direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDirection {
    /// Upward direction
    Up,
    /// Downward direction
    Down,
    /// Leftward direction
    Left,
    /// Rightward direction
    Right,
}

impl Default for TransitionDirection {
    fn default() -> Self {
        Self::Left
    }
}

/// Transition configuration
#[derive(Debug, Clone, Copy)]
pub struct Transition {
    /// Effect type
    pub effect: TransitionEffect,
    /// Direction
    pub direction: TransitionDirection,
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Easing function
    pub easing: Easing,
}

impl Transition {
    /// Create new transition
    pub const fn new(effect: TransitionEffect, duration_ms: u32) -> Self {
        Self {
            effect,
            direction: TransitionDirection::Left,
            duration_ms,
            easing: Easing::EaseInOut,
        }
    }

    /// With direction
    #[must_use]
    pub const fn with_direction(mut self, direction: TransitionDirection) -> Self {
        self.direction = direction;
        self
    }

    /// With easing
    #[must_use]
    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new(TransitionEffect::Fade, 300)
    }
}

// =============================================================================
// PROGRESS ANIMATION
// =============================================================================

/// Progress bar style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    /// Simple solid bar
    Solid,
    /// Gradient bar
    Gradient,
    /// Striped bar
    Striped,
    /// Animated striped bar
    AnimatedStripes,
    /// Glowing bar
    Glow,
    /// Segmented bar
    Segmented,
    /// Circular
    Circular,
    /// Semi-circular arc
    Arc,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self::Solid
    }
}

/// Progress indicator configuration
#[derive(Debug, Clone, Copy)]
pub struct ProgressConfig {
    /// Style
    pub style: ProgressStyle,
    /// Background color
    pub bg_color: AnimColor,
    /// Fill color (start)
    pub fill_start: AnimColor,
    /// Fill color (end, for gradient)
    pub fill_end: AnimColor,
    /// Border color
    pub border_color: AnimColor,
    /// Border width
    pub border_width: u8,
    /// Corner radius
    pub corner_radius: u8,
    /// Animation speed (for animated styles)
    pub animation_speed: f32,
    /// Show percentage text
    pub show_text: bool,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            style: ProgressStyle::Gradient,
            bg_color: AnimColor::new(40, 40, 40, 255),
            fill_start: AnimColor::new(0, 120, 255, 255),
            fill_end: AnimColor::new(0, 200, 255, 255),
            border_color: AnimColor::new(80, 80, 80, 255),
            border_width: 1,
            corner_radius: 4,
            animation_speed: 1.0,
            show_text: true,
        }
    }
}

// =============================================================================
// SPINNER ANIMATION
// =============================================================================

/// Spinner style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerStyle {
    /// Simple rotating arc
    Arc,
    /// Dots
    Dots,
    /// Fading dots
    FadingDots,
    /// Spinning circle segments
    Segments,
    /// Pulsing ring
    PulsingRing,
    /// Double arc
    DoubleArc,
    /// Spinning logo
    Logo,
}

impl Default for SpinnerStyle {
    fn default() -> Self {
        Self::Arc
    }
}

/// Spinner configuration
#[derive(Debug, Clone, Copy)]
pub struct SpinnerConfig {
    /// Style
    pub style: SpinnerStyle,
    /// Size (diameter)
    pub size: u32,
    /// Primary color
    pub primary_color: AnimColor,
    /// Secondary color
    pub secondary_color: AnimColor,
    /// Line thickness
    pub thickness: u8,
    /// Rotation speed (degrees per second)
    pub speed: f32,
    /// Number of segments/dots
    pub segments: u8,
}

impl Default for SpinnerConfig {
    fn default() -> Self {
        Self {
            style: SpinnerStyle::Arc,
            size: 32,
            primary_color: AnimColor::new(100, 180, 255, 255),
            secondary_color: AnimColor::new(40, 40, 40, 255),
            thickness: 3,
            speed: 360.0,
            segments: 8,
        }
    }
}

// =============================================================================
// SPLASH SCREEN
// =============================================================================

/// Splash screen layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplashLayout {
    /// Logo centered
    Centered,
    /// Logo at top
    Top,
    /// Logo and progress at bottom
    LogoProgress,
    /// Full screen image
    FullScreen,
    /// Split (logo left, info right)
    Split,
}

impl Default for SplashLayout {
    fn default() -> Self {
        Self::LogoProgress
    }
}

/// Splash screen configuration
#[derive(Debug, Clone, Copy)]
pub struct SplashConfig {
    /// Layout
    pub layout: SplashLayout,
    /// Background color
    pub background: AnimColor,
    /// Logo fade in duration
    pub fade_in_ms: u32,
    /// Minimum display time
    pub min_display_ms: u32,
    /// Show progress indicator
    pub show_progress: bool,
    /// Progress configuration
    pub progress: ProgressConfig,
    /// Show spinner
    pub show_spinner: bool,
    /// Spinner configuration
    pub spinner: SpinnerConfig,
    /// Enable logo animation
    pub animate_logo: bool,
}

impl Default for SplashConfig {
    fn default() -> Self {
        Self {
            layout: SplashLayout::LogoProgress,
            background: AnimColor::new(18, 18, 28, 255),
            fade_in_ms: 500,
            min_display_ms: 2000,
            show_progress: true,
            progress: ProgressConfig::default(),
            show_spinner: false,
            spinner: SpinnerConfig::default(),
            animate_logo: true,
        }
    }
}

// =============================================================================
// LAYER
// =============================================================================

/// Layer blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Normal (alpha blend)
    Normal,
    /// Additive
    Add,
    /// Multiply
    Multiply,
    /// Screen
    Screen,
    /// Overlay
    Overlay,
    /// Darken
    Darken,
    /// Lighten
    Lighten,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Layer properties
#[derive(Debug, Clone, Copy)]
pub struct Layer {
    /// Layer ID
    pub id: u32,
    /// Z-order (higher = on top)
    pub z_order: i16,
    /// Visibility
    pub visible: bool,
    /// Opacity (0-255)
    pub opacity: u8,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Transform
    pub transform: Transform2D,
}

impl Layer {
    /// Create new layer
    pub const fn new(id: u32) -> Self {
        Self {
            id,
            z_order: 0,
            visible: true,
            opacity: 255,
            blend_mode: BlendMode::Normal,
            transform: Transform2D::IDENTITY,
        }
    }
}

impl Default for Layer {
    fn default() -> Self {
        Self::new(0)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        assert!((Easing::Linear.apply(0.0) - 0.0).abs() < 0.001);
        assert!((Easing::Linear.apply(0.5) - 0.5).abs() < 0.001);
        assert!((Easing::Linear.apply(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_easing_ease_in() {
        let e = Easing::EaseIn;
        assert!(e.apply(0.5) < 0.5); // Slower at start
    }

    #[test]
    fn test_easing_ease_out() {
        let e = Easing::EaseOut;
        assert!(e.apply(0.5) > 0.5); // Faster at start
    }

    #[test]
    fn test_color_lerp() {
        let a = AnimColor::BLACK;
        let b = AnimColor::WHITE;
        let mid = a.lerp(&b, 0.5);
        assert_eq!(mid.r, 127);
        assert_eq!(mid.g, 127);
        assert_eq!(mid.b, 127);
    }

    #[test]
    fn test_transform_lerp() {
        let a = Transform2D::translate(0.0, 0.0);
        let b = Transform2D::translate(100.0, 100.0);
        let mid = a.lerp(&b, 0.5);
        assert!((mid.translation.x - 50.0).abs() < 0.001);
        assert!((mid.translation.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_particle_age() {
        let p = Particle {
            lifetime: 500,
            initial_lifetime: 1000,
            ..Default::default()
        };
        assert!((p.age() - 0.5).abs() < 0.001);
    }
}
