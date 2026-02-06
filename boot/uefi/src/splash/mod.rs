//! Splash Screen and Visual Boot Experience
//!
//! This module provides splash screen rendering, boot animations,
//! and visual boot experience for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - Splash screen rendering
//! - Boot logo display
//! - Progress visualization
//! - Animation system
//! - Transition effects
//! - Brand identity

// =============================================================================
// COLORS
// =============================================================================

/// RGBA color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255, 255 = fully opaque)
    pub a: u8,
}

impl Color {
    /// Create color from RGBA
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create color from RGB (fully opaque)
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create from 32-bit ARGB
    pub const fn from_argb32(argb: u32) -> Self {
        Self {
            a: ((argb >> 24) & 0xFF) as u8,
            r: ((argb >> 16) & 0xFF) as u8,
            g: ((argb >> 8) & 0xFF) as u8,
            b: (argb & 0xFF) as u8,
        }
    }

    /// Create from 32-bit RGB (no alpha)
    pub const fn from_rgb32(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: (rgb & 0xFF) as u8,
            a: 255,
        }
    }

    /// Convert to 32-bit ARGB
    pub const fn to_argb32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Convert to 32-bit RGB
    pub const fn to_rgb32(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Blend with another color
    #[must_use]
    pub fn blend(&self, other: Self, alpha: u8) -> Self {
        let a = alpha as u16;
        let inv_a = 255u16 - a;
        Self {
            r: Self::u16_to_u8(((self.r as u16) * inv_a + (other.r as u16) * a) / 255),
            g: Self::u16_to_u8(((self.g as u16) * inv_a + (other.g as u16) * a) / 255),
            b: Self::u16_to_u8(((self.b as u16) * inv_a + (other.b as u16) * a) / 255),
            a: Self::u16_to_u8(((self.a as u16) * inv_a + (other.a as u16) * a) / 255),
        }
    }

    /// Darken by percentage (0-100)
    #[must_use]
    pub fn darken(&self, percent: u8) -> Self {
        let factor = (100 - percent.min(100)) as u16;
        Self {
            r: Self::u16_to_u8(((self.r as u16) * factor) / 100),
            g: Self::u16_to_u8(((self.g as u16) * factor) / 100),
            b: Self::u16_to_u8(((self.b as u16) * factor) / 100),
            a: self.a,
        }
    }

    /// Lighten by percentage (0-100)
    #[must_use]
    pub fn lighten(&self, percent: u8) -> Self {
        let factor = percent.min(100) as u16;
        Self {
            r: Self::u16_to_u8((self.r as u16) + ((255 - (self.r as u16)) * factor) / 100),
            g: Self::u16_to_u8((self.g as u16) + ((255 - (self.g as u16)) * factor) / 100),
            b: Self::u16_to_u8((self.b as u16) + ((255 - (self.b as u16)) * factor) / 100),
            a: self.a,
        }
    }

    /// Convert u16 to u8, saturating at u8::MAX
    const fn u16_to_u8(value: u16) -> u8 {
        if value > u8::MAX as u16 {
            u8::MAX
        } else {
            value as u8
        }
    }

    /// Transparent
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Black
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White
    pub const WHITE: Self = Self::rgb(255, 255, 255);
}

/// Helix brand colors
pub mod brand {
    use super::Color;

    /// Primary brand color
    pub const PRIMARY: Color = Color::rgb(0x00, 0x7A, 0xCC);
    /// Secondary brand color
    pub const SECONDARY: Color = Color::rgb(0x00, 0x50, 0x8F);
    /// Accent color
    pub const ACCENT: Color = Color::rgb(0x4E, 0xC9, 0xB0);
    /// Background
    pub const BACKGROUND: Color = Color::rgb(0x1E, 0x1E, 0x2E);
    /// Surface
    pub const SURFACE: Color = Color::rgb(0x2D, 0x2D, 0x3D);
    /// Error
    pub const ERROR: Color = Color::rgb(0xF0, 0x56, 0x56);
    /// Warning
    pub const WARNING: Color = Color::rgb(0xFF, 0xB8, 0x6C);
    /// Success
    pub const SUCCESS: Color = Color::rgb(0x50, 0xFA, 0x7B);
    /// Text primary
    pub const TEXT_PRIMARY: Color = Color::rgb(0xF8, 0xF8, 0xF2);
    /// Text secondary
    pub const TEXT_SECONDARY: Color = Color::rgb(0xBD, 0xBD, 0xBD);
    /// Text disabled
    pub const TEXT_DISABLED: Color = Color::rgb(0x6D, 0x6D, 0x7D);
}

// =============================================================================
// POINT AND RECT
// =============================================================================

/// 2D point
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

impl Point {
    /// Create point
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Origin (0, 0)
    pub const fn origin() -> Self {
        Self { x: 0, y: 0 }
    }

    /// Add offset
    #[must_use]
    pub const fn offset(&self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// 2D size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Size {
    /// Create size
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Zero size
    pub const fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }

    /// Area
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Rectangle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// X coordinate of top-left corner
    pub x: i32,
    /// Y coordinate of top-left corner
    pub y: i32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Rect {
    /// Create rectangle
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create from position and size
    pub const fn from_pos_size(pos: Point, size: Size) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        }
    }

    /// Get position
    pub const fn position(&self) -> Point {
        Point {
            x: self.x,
            y: self.y,
        }
    }

    /// Get size
    pub const fn size(&self) -> Size {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    /// Right edge
    #[must_use]
    pub const fn right(&self) -> i32 {
        self.x.saturating_add(Self::u32_to_i32_saturated(self.width))
    }

    /// Bottom edge
    #[must_use]
    pub const fn bottom(&self) -> i32 {
        self.y.saturating_add(Self::u32_to_i32_saturated(self.height))
    }

    /// Center point
    #[must_use]
    pub const fn center(&self) -> Point {
        Point {
            x: self.x.saturating_add(Self::u32_to_i32_saturated(self.width / 2)),
            y: self.y.saturating_add(Self::u32_to_i32_saturated(self.height / 2)),
        }
    }

    /// Check if point is inside
    pub const fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    /// Intersect with another rectangle
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if right > x && bottom > y {
            Some(Self::new(
                x,
                y,
                (right - x).unsigned_abs(),
                (bottom - y).unsigned_abs(),
            ))
        } else {
            None
        }
    }

    /// Center within another rectangle
    #[must_use]
    pub fn center_in(&self, container: &Self) -> Self {
        let w_diff = i64::from(container.width) - i64::from(self.width);
        let h_diff = i64::from(container.height) - i64::from(self.height);
        let x = i64::from(container.x) + (w_diff / 2);
        let y = i64::from(container.y) + (h_diff / 2);
        Self::new(
            Self::i64_to_i32_saturated(x),
            Self::i64_to_i32_saturated(y),
            self.width,
            self.height,
        )
    }

    /// Convert u32 to i32, saturating at i32::MAX
    const fn u32_to_i32_saturated(value: u32) -> i32 {
        if value > i32::MAX as u32 {
            i32::MAX
        } else {
            value as i32
        }
    }

    /// Convert i64 to i32, saturating at boundaries
    const fn i64_to_i32_saturated(value: i64) -> i32 {
        if value < i32::MIN as i64 {
            i32::MIN
        } else if value > i32::MAX as i64 {
            i32::MAX
        } else {
            value as i32
        }
    }
}

// =============================================================================
// PIXEL FORMAT
// =============================================================================

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// 32-bit BGRA
    #[default]
    Bgra32,
    /// 32-bit RGBA
    Rgba32,
    /// 24-bit BGR
    Bgr24,
    /// 24-bit RGB
    Rgb24,
    /// 16-bit 565 RGB
    Rgb565,
    /// 8-bit indexed
    Indexed8,
}

impl PixelFormat {
    /// Bytes per pixel
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> u8 {
        match self {
            Self::Bgra32 | Self::Rgba32 => 4,
            Self::Bgr24 | Self::Rgb24 => 3,
            Self::Rgb565 => 2,
            Self::Indexed8 => 1,
        }
    }

    /// Convert color to raw pixel value
    #[must_use]
    pub fn color_to_pixel(&self, color: Color) -> u32 {
        match self {
            Self::Bgra32 => {
                (u32::from(color.a) << 24)
                    | (u32::from(color.r) << 16)
                    | (u32::from(color.g) << 8)
                    | u32::from(color.b)
            },
            Self::Rgba32 => color.to_argb32(),
            Self::Bgr24 | Self::Rgb24 => color.to_rgb32(),
            Self::Rgb565 => {
                let r = (u32::from(color.r) >> 3) & 0x1F;
                let g = (u32::from(color.g) >> 2) & 0x3F;
                let b = (u32::from(color.b) >> 3) & 0x1F;
                (r << 11) | (g << 5) | b
            },
            Self::Indexed8 => u32::from(color.r), // Grayscale
        }
    }
}

// =============================================================================
// FRAMEBUFFER
// =============================================================================

/// Framebuffer info
#[derive(Debug, Clone, Copy, Default)]
pub struct FramebufferInfo {
    /// Base address
    pub base: u64,
    /// Size in bytes
    pub size: usize,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Stride (bytes per row)
    pub stride: u32,
    /// Pixel format
    pub format: PixelFormat,
}

impl FramebufferInfo {
    /// Get pixel offset
    pub const fn pixel_offset(&self, x: u32, y: u32) -> usize {
        (y * self.stride + x * self.format.bytes_per_pixel() as u32) as usize
    }

    /// Get dimensions as rect
    pub const fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }
}

/// Draw context
#[derive(Debug)]
pub struct DrawContext {
    /// Framebuffer info
    pub fb: FramebufferInfo,
    /// Clip rectangle
    pub clip: Rect,
    /// Current color
    pub color: Color,
    /// Background color
    pub bg_color: Color,
}

impl DrawContext {
    /// Create new draw context
    pub const fn new(fb: FramebufferInfo) -> Self {
        Self {
            fb,
            clip: Rect::new(0, 0, fb.width, fb.height),
            color: Color::WHITE,
            bg_color: Color::BLACK,
        }
    }

    /// Set clip rectangle
    pub fn set_clip(&mut self, clip: Rect) {
        self.clip = clip;
    }

    /// Set foreground color
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Set background color
    pub fn set_bg_color(&mut self, color: Color) {
        self.bg_color = color;
    }
}

// =============================================================================
// SPLASH SCREEN
// =============================================================================

/// Splash screen style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplashStyle {
    /// Minimal (just logo)
    #[default]
    Minimal,
    /// Standard (logo + progress)
    Standard,
    /// Verbose (logo + progress + status)
    Verbose,
    /// Debug (full info)
    Debug,
}

/// Logo position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogoPosition {
    /// Top center
    TopCenter,
    /// Center
    #[default]
    Center,
    /// Bottom center
    BottomCenter,
    /// Custom position
    Custom(i32, i32),
}

/// Progress bar style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressStyle {
    /// Solid bar
    #[default]
    Solid,
    /// Striped bar
    Striped,
    /// Gradient bar
    Gradient,
    /// Dots
    Dots,
    /// Spinner
    Spinner,
}

/// Splash screen configuration
#[derive(Debug, Clone, Copy)]
pub struct SplashConfig {
    /// Style
    pub style: SplashStyle,
    /// Background color
    pub background: Color,
    /// Logo position
    pub logo_pos: LogoPosition,
    /// Show progress bar
    pub show_progress: bool,
    /// Progress bar style
    pub progress_style: ProgressStyle,
    /// Progress bar color
    pub progress_color: Color,
    /// Progress background color
    pub progress_bg: Color,
    /// Show status text
    pub show_status: bool,
    /// Status text color
    pub status_color: Color,
    /// Show phase indicator
    pub show_phases: bool,
    /// Animation speed (ms per frame)
    pub animation_speed_ms: u32,
}

impl Default for SplashConfig {
    fn default() -> Self {
        Self {
            style: SplashStyle::Standard,
            background: brand::BACKGROUND,
            logo_pos: LogoPosition::Center,
            show_progress: true,
            progress_style: ProgressStyle::Gradient,
            progress_color: brand::PRIMARY,
            progress_bg: brand::SURFACE,
            show_status: true,
            status_color: brand::TEXT_SECONDARY,
            show_phases: false,
            animation_speed_ms: 16,
        }
    }
}

/// Splash screen state
#[derive(Debug, Clone, Copy, Default)]
pub struct SplashState {
    /// Progress (0-100)
    pub progress: u8,
    /// Animation frame
    pub frame: u32,
    /// Current phase index
    pub phase: u8,
    /// Total phases
    pub total_phases: u8,
    /// Is animating
    pub animating: bool,
    /// Time elapsed (ms)
    pub elapsed_ms: u64,
}

impl SplashState {
    /// Update progress
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
    }

    /// Advance animation frame
    pub fn advance_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Set phase
    pub fn set_phase(&mut self, phase: u8, total: u8) {
        self.phase = phase;
        self.total_phases = total;
        // Calculate progress from phase
        if total > 0 {
            self.progress = ((u16::from(phase) * 100) / u16::from(total)) as u8;
        }
    }
}

/// Maximum status message length
pub const MAX_STATUS_LEN: usize = 128;

/// Splash screen
#[derive(Debug)]
pub struct SplashScreen {
    /// Configuration
    pub config: SplashConfig,
    /// State
    pub state: SplashState,
    /// Status message buffer
    status: [u8; MAX_STATUS_LEN],
    /// Status message length
    status_len: usize,
    /// Screen dimensions
    pub screen_size: Size,
    /// Logo dimensions
    pub logo_size: Size,
    /// Logo data (if embedded)
    logo_data: Option<&'static [u8]>,
}

impl Default for SplashScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl SplashScreen {
    /// Create new splash screen
    pub const fn new() -> Self {
        Self {
            config: SplashConfig {
                style: SplashStyle::Standard,
                background: brand::BACKGROUND,
                logo_pos: LogoPosition::Center,
                show_progress: true,
                progress_style: ProgressStyle::Gradient,
                progress_color: brand::PRIMARY,
                progress_bg: brand::SURFACE,
                show_status: true,
                status_color: brand::TEXT_SECONDARY,
                show_phases: false,
                animation_speed_ms: 16,
            },
            state: SplashState {
                progress: 0,
                frame: 0,
                phase: 0,
                total_phases: 0,
                animating: false,
                elapsed_ms: 0,
            },
            status: [0u8; MAX_STATUS_LEN],
            status_len: 0,
            screen_size: Size::zero(),
            logo_size: Size::new(128, 128),
            logo_data: None,
        }
    }

    /// Initialize with screen size
    pub fn init(&mut self, screen_size: Size) {
        self.screen_size = screen_size;
        self.state = SplashState::default();
        self.state.animating = true;
    }

    /// Set configuration
    pub fn configure(&mut self, config: SplashConfig) {
        self.config = config;
    }

    /// Set logo data
    pub fn set_logo(&mut self, data: &'static [u8], size: Size) {
        self.logo_data = Some(data);
        self.logo_size = size;
    }

    /// Set status message
    pub fn set_status(&mut self, msg: &str) {
        let bytes = msg.as_bytes();
        let len = bytes.len().min(MAX_STATUS_LEN);
        self.status[..len].copy_from_slice(&bytes[..len]);
        self.status_len = len;
    }

    /// Get status message
    pub fn status(&self) -> &str {
        if self.status_len == 0 {
            ""
        } else {
            core::str::from_utf8(&self.status[..self.status_len]).unwrap_or("")
        }
    }

    /// Update with elapsed time
    pub fn update(&mut self, delta_ms: u64) {
        self.state.elapsed_ms += delta_ms;

        // Advance animation frame based on speed
        if self.state.animating && self.config.animation_speed_ms > 0 {
            let frames_per_ms = delta_ms / u64::from(self.config.animation_speed_ms);
            for _ in 0..frames_per_ms {
                self.state.advance_frame();
            }
        }
    }

    /// Set progress
    pub fn set_progress(&mut self, progress: u8) {
        self.state.set_progress(progress);
    }

    /// Set phase
    pub fn set_phase(&mut self, phase: u8, total: u8, status: &str) {
        self.state.set_phase(phase, total);
        self.set_status(status);
    }

    /// Get logo rectangle
    #[must_use]
    pub fn logo_rect(&self) -> Rect {
        let logo = Rect::new(0, 0, self.logo_size.width, self.logo_size.height);
        let screen = Rect::new(0, 0, self.screen_size.width, self.screen_size.height);

        match self.config.logo_pos {
            LogoPosition::TopCenter => {
                let x = (self.screen_size.width - self.logo_size.width) / 2;
                let y = self.screen_size.height / 8;
                Rect::new(
                    i32::try_from(x).unwrap_or(i32::MAX),
                    i32::try_from(y).unwrap_or(i32::MAX),
                    self.logo_size.width,
                    self.logo_size.height,
                )
            },
            LogoPosition::Center => logo.center_in(&screen),
            LogoPosition::BottomCenter => {
                let x = (self.screen_size.width - self.logo_size.width) / 2;
                let y = (self.screen_size.height * 3) / 4;
                Rect::new(
                    i32::try_from(x).unwrap_or(i32::MAX),
                    i32::try_from(y).unwrap_or(i32::MAX),
                    self.logo_size.width,
                    self.logo_size.height,
                )
            },
            LogoPosition::Custom(x, y) => {
                Rect::new(x, y, self.logo_size.width, self.logo_size.height)
            },
        }
    }

    /// Get progress bar rectangle
    #[must_use]
    pub fn progress_rect(&self) -> Rect {
        let bar_width = (self.screen_size.width * 2) / 5; // 40% of screen
        let bar_height = 8;
        let x = (self.screen_size.width - bar_width) / 2;
        let y = (self.screen_size.height * 3) / 4; // 75% down

        Rect::new(
            i32::try_from(x).unwrap_or(i32::MAX),
            i32::try_from(y).unwrap_or(i32::MAX),
            bar_width,
            bar_height,
        )
    }

    /// Get status text position
    #[must_use]
    pub fn status_position(&self) -> Point {
        let progress_rect = self.progress_rect();
        Point::new(progress_rect.x, progress_rect.bottom() + 16)
    }

    /// Complete splash (100% progress)
    pub fn complete(&mut self) {
        self.state.progress = 100;
        self.state.animating = false;
        self.set_status("Boot complete");
    }
}

// =============================================================================
// ANIMATIONS
// =============================================================================

/// Animation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationType {
    /// No animation
    #[default]
    None,
    /// Fade in
    FadeIn,
    /// Fade out
    FadeOut,
    /// Slide left
    SlideLeft,
    /// Slide right
    SlideRight,
    /// Slide up
    SlideUp,
    /// Slide down
    SlideDown,
    /// Zoom in
    ZoomIn,
    /// Zoom out
    ZoomOut,
    /// Pulse
    Pulse,
    /// Spin
    Spin,
}

/// Easing function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Linear
    #[default]
    Linear,
    /// Ease in (slow start)
    EaseIn,
    /// Ease out (slow end)
    EaseOut,
    /// Ease in/out
    EaseInOut,
    /// Bounce
    Bounce,
}

impl Easing {
    /// Apply easing to normalized time (0.0 to 1.0 as fixed point)
    #[must_use]
    pub fn apply(&self, t: u16) -> u16 {
        // t is in fixed point: 0 = 0.0, 1000 = 1.0
        let t_squared = (u32::from(t) * u32::from(t)) / 1000;

        match self {
            Self::Linear => t,
            Self::EaseIn => t_squared as u16,
            Self::EaseOut => {
                let inv_t = 1000 - t;
                let inv_sq = (u32::from(inv_t) * u32::from(inv_t)) / 1000;
                1000 - inv_sq as u16
            },
            Self::EaseInOut => {
                if t < 500 {
                    ((t_squared * 2) / 1000) as u16
                } else {
                    let inv_t = 1000 - t;
                    let inv_sq = (u32::from(inv_t) * u32::from(inv_t)) / 1000;
                    (1000 - (inv_sq * 2 / 1000)) as u16
                }
            },
            Self::Bounce => {
                // Simplified bounce
                let base = Self::EaseOut.apply(t);
                if t > 800 {
                    let bounce = (u32::from(t - 800) * 200) / 200;
                    (u32::from(base) + bounce / 4) as u16
                } else {
                    base
                }
            },
        }
    }
}

/// Animation state
#[derive(Debug, Clone, Copy, Default)]
pub struct Animation {
    /// Animation type
    pub anim_type: AnimationType,
    /// Easing function
    pub easing: Easing,
    /// Duration (ms)
    pub duration_ms: u32,
    /// Elapsed time (ms)
    pub elapsed_ms: u32,
    /// Is playing
    pub playing: bool,
    /// Loop animation
    pub loop_anim: bool,
    /// Reverse on loop
    pub reverse: bool,
    /// Current direction (false = forward, true = backward)
    direction: bool,
}

impl Animation {
    /// Create new animation
    pub const fn new(anim_type: AnimationType, duration_ms: u32) -> Self {
        Self {
            anim_type,
            easing: Easing::Linear,
            duration_ms,
            elapsed_ms: 0,
            playing: false,
            loop_anim: false,
            reverse: false,
            direction: false,
        }
    }

    /// Set easing
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Set looping
    pub fn with_loop(mut self, loop_anim: bool) -> Self {
        self.loop_anim = loop_anim;
        self
    }

    /// Set reverse on loop
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Start playing
    pub fn play(&mut self) {
        self.playing = true;
        self.elapsed_ms = 0;
        self.direction = false;
    }

    /// Stop playing
    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Reset animation
    pub fn reset(&mut self) {
        self.elapsed_ms = 0;
        self.direction = false;
    }

    /// Update with elapsed time
    pub fn update(&mut self, delta_ms: u32) {
        if !self.playing {
            return;
        }

        self.elapsed_ms += delta_ms;

        if self.elapsed_ms >= self.duration_ms {
            if self.loop_anim {
                self.elapsed_ms = 0;
                if self.reverse {
                    self.direction = !self.direction;
                }
            } else {
                self.elapsed_ms = self.duration_ms;
                self.playing = false;
            }
        }
    }

    /// Get progress (0-1000 fixed point)
    pub fn progress(&self) -> u16 {
        if self.duration_ms == 0 {
            return 1000;
        }

        let raw = ((self.elapsed_ms as u64 * 1000) / self.duration_ms as u64) as u16;
        let raw = raw.min(1000);

        let eased = self.easing.apply(raw);

        if self.direction {
            1000 - eased
        } else {
            eased
        }
    }

    /// Check if complete
    pub const fn is_complete(&self) -> bool {
        !self.playing && self.elapsed_ms >= self.duration_ms
    }
}

// =============================================================================
// TRANSITION
// =============================================================================

/// Transition between screens
#[derive(Debug, Clone, Copy, Default)]
pub struct Transition {
    /// Transition type
    pub trans_type: AnimationType,
    /// Duration (ms)
    pub duration_ms: u32,
    /// Elapsed time (ms)
    pub elapsed_ms: u32,
    /// Is active
    pub active: bool,
    /// From screen opacity (0-255)
    pub from_opacity: u8,
    /// To screen opacity (0-255)
    pub to_opacity: u8,
}

impl Transition {
    /// Create fade transition
    pub const fn fade(duration_ms: u32) -> Self {
        Self {
            trans_type: AnimationType::FadeIn,
            duration_ms,
            elapsed_ms: 0,
            active: false,
            from_opacity: 255,
            to_opacity: 0,
        }
    }

    /// Create slide transition
    pub const fn slide(direction: AnimationType, duration_ms: u32) -> Self {
        Self {
            trans_type: direction,
            duration_ms,
            elapsed_ms: 0,
            active: false,
            from_opacity: 255,
            to_opacity: 0,
        }
    }

    /// Start transition
    pub fn start(&mut self) {
        self.active = true;
        self.elapsed_ms = 0;
        self.from_opacity = 255;
        self.to_opacity = 0;
    }

    /// Update transition
    pub fn update(&mut self, delta_ms: u32) {
        if !self.active {
            return;
        }

        self.elapsed_ms += delta_ms;

        if self.elapsed_ms >= self.duration_ms {
            self.elapsed_ms = self.duration_ms;
            self.active = false;
            self.from_opacity = 0;
            self.to_opacity = 255;
        } else {
            let progress = (self.elapsed_ms * 255) / self.duration_ms;
            self.to_opacity = progress as u8;
            self.from_opacity = 255 - self.to_opacity;
        }
    }

    /// Check if complete
    pub const fn is_complete(&self) -> bool {
        !self.active && self.elapsed_ms >= self.duration_ms
    }

    /// Get slide offset
    pub fn slide_offset(&self, screen_size: Size) -> Point {
        if !self.active || self.duration_ms == 0 {
            return Point::origin();
        }

        let progress = (self.elapsed_ms * 1000) / self.duration_ms;
        let eased = Easing::EaseOut.apply(progress as u16);

        match self.trans_type {
            AnimationType::SlideLeft => {
                let offset = ((screen_size.width as u64 * (1000 - eased as u64)) / 1000) as i32;
                Point::new(-offset, 0)
            },
            AnimationType::SlideRight => {
                let offset = ((screen_size.width as u64 * (1000 - eased as u64)) / 1000) as i32;
                Point::new(offset, 0)
            },
            AnimationType::SlideUp => {
                let offset = ((screen_size.height as u64 * (1000 - eased as u64)) / 1000) as i32;
                Point::new(0, -offset)
            },
            AnimationType::SlideDown => {
                let offset = ((screen_size.height as u64 * (1000 - eased as u64)) / 1000) as i32;
                Point::new(0, offset)
            },
            _ => Point::origin(),
        }
    }
}

// =============================================================================
// SPINNER
// =============================================================================

/// Spinner style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// Circle dots
    #[default]
    CircleDots,
    /// Rotating arc
    Arc,
    /// Bouncing dots
    BouncingDots,
    /// Pulsing ring
    PulsingRing,
    /// Bars
    Bars,
}

/// Spinner state
#[derive(Debug, Clone, Copy, Default)]
pub struct Spinner {
    /// Style
    pub style: SpinnerStyle,
    /// Size (diameter)
    pub size: u32,
    /// Color
    pub color: Color,
    /// Background color
    pub bg_color: Color,
    /// Animation frame
    pub frame: u32,
    /// Speed (frames per full rotation)
    pub speed: u32,
    /// Active
    pub active: bool,
}

impl Spinner {
    /// Create new spinner
    pub const fn new(style: SpinnerStyle, size: u32) -> Self {
        Self {
            style,
            size,
            color: brand::PRIMARY,
            bg_color: Color::TRANSPARENT,
            frame: 0,
            speed: 60, // 60 frames per rotation
            active: false,
        }
    }

    /// Start spinner
    pub fn start(&mut self) {
        self.active = true;
        self.frame = 0;
    }

    /// Stop spinner
    pub fn stop(&mut self) {
        self.active = false;
    }

    /// Update spinner
    pub fn update(&mut self) {
        if self.active {
            self.frame = (self.frame + 1) % self.speed;
        }
    }

    /// Get rotation angle (0-359 degrees)
    pub fn rotation(&self) -> u16 {
        if self.speed == 0 {
            return 0;
        }
        ((self.frame * 360) / self.speed) as u16
    }

    /// Get number of visible segments (for dot/bar spinners)
    pub fn visible_segments(&self) -> u8 {
        // For styles that show partial segments
        match self.style {
            SpinnerStyle::CircleDots => 8,
            SpinnerStyle::Bars => 12,
            _ => 1,
        }
    }

    /// Get active segment index
    pub fn active_segment(&self) -> u8 {
        let segments = self.visible_segments() as u32;
        if segments == 0 || self.speed == 0 {
            return 0;
        }
        ((self.frame * segments) / self.speed) as u8
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_color() {
        let c = Color::rgb(255, 128, 64);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_color_blend() {
        let white = Color::WHITE;
        let black = Color::BLACK;
        let gray = white.blend(black, 128);
        // Should be approximately 128, 128, 128
        assert!(gray.r > 120 && gray.r < 136);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 10, 100, 100);
        assert!(rect.contains(Point::new(50, 50)));
        assert!(!rect.contains(Point::new(5, 5)));
        assert!(!rect.contains(Point::new(200, 200)));
    }

    #[test]
    fn test_rect_intersect() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        let i = a.intersect(&b).unwrap();
        assert_eq!(i.x, 50);
        assert_eq!(i.y, 50);
        assert_eq!(i.width, 50);
        assert_eq!(i.height, 50);
    }

    #[test]
    fn test_animation() {
        let mut anim = Animation::new(AnimationType::FadeIn, 1000);
        anim.play();

        anim.update(500);
        assert!(anim.playing);
        assert_eq!(anim.progress(), 500);

        anim.update(500);
        assert!(!anim.playing);
        assert_eq!(anim.progress(), 1000);
    }

    #[test]
    fn test_easing() {
        assert_eq!(Easing::Linear.apply(500), 500);
        assert!(Easing::EaseIn.apply(500) < 500);
        assert!(Easing::EaseOut.apply(500) > 500);
    }

    #[test]
    fn test_spinner() {
        let mut spinner = Spinner::new(SpinnerStyle::CircleDots, 32);
        spinner.start();

        for _ in 0..30 {
            spinner.update();
        }

        assert_eq!(spinner.rotation(), 180);
    }

    #[test]
    fn test_splash() {
        let mut splash = SplashScreen::new();
        splash.init(Size::new(1920, 1080));

        splash.set_progress(50);
        assert_eq!(splash.state.progress, 50);

        splash.set_status("Loading kernel...");
        assert_eq!(splash.status(), "Loading kernel...");
    }
}
