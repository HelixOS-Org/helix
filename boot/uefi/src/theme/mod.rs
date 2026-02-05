//! Theme and Appearance System
//!
//! This module provides comprehensive theming, styling, and visual
//! customization capabilities for the Helix UEFI Bootloader.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                       Theme System                                      │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Color Schemes                                  │   │
//! │  │  Dark │ Light │ High Contrast │ Custom │ System                 │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Style Properties                               │   │
//! │  │  Colors │ Fonts │ Spacing │ Borders │ Effects                   │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Component Styles                               │   │
//! │  │  Menu │ Text │ Buttons │ Progress │ Dialogs                     │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// COLOR SYSTEM
// =============================================================================

/// RGBA color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    /// Red component
    pub r: u8,
    /// Green component
    pub g: u8,
    /// Blue component
    pub b: u8,
    /// Alpha component (255 = opaque)
    pub a: u8,
}

impl Color {
    /// Create new color
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create color with alpha
    pub const fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create from RGB hex value
    pub const fn from_rgb(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xFF) as u8,
            g: ((rgb >> 8) & 0xFF) as u8,
            b: (rgb & 0xFF) as u8,
            a: 255,
        }
    }

    /// Create from RGBA hex value
    pub const fn from_rgba(rgba: u32) -> Self {
        Self {
            r: ((rgba >> 24) & 0xFF) as u8,
            g: ((rgba >> 16) & 0xFF) as u8,
            b: ((rgba >> 8) & 0xFF) as u8,
            a: (rgba & 0xFF) as u8,
        }
    }

    /// Convert to RGB u32
    pub const fn to_rgb(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Convert to BGR u32 (for some framebuffers)
    pub const fn to_bgr(&self) -> u32 {
        ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }

    /// Convert to RGBA u32
    pub const fn to_rgba(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Blend with another color
    pub fn blend(&self, other: Color, t: u8) -> Color {
        let t = t as u16;
        let inv_t = 255 - t;
        Color {
            r: (((self.r as u16 * inv_t) + (other.r as u16 * t)) / 255) as u8,
            g: (((self.g as u16 * inv_t) + (other.g as u16 * t)) / 255) as u8,
            b: (((self.b as u16 * inv_t) + (other.b as u16 * t)) / 255) as u8,
            a: (((self.a as u16 * inv_t) + (other.a as u16 * t)) / 255) as u8,
        }
    }

    /// Darken color
    pub fn darken(&self, amount: u8) -> Color {
        let factor = 255 - amount;
        Color {
            r: ((self.r as u16 * factor as u16) / 255) as u8,
            g: ((self.g as u16 * factor as u16) / 255) as u8,
            b: ((self.b as u16 * factor as u16) / 255) as u8,
            a: self.a,
        }
    }

    /// Lighten color
    pub fn lighten(&self, amount: u8) -> Color {
        Color {
            r: self
                .r
                .saturating_add(((255 - self.r) as u16 * amount as u16 / 255) as u8),
            g: self
                .g
                .saturating_add(((255 - self.g) as u16 * amount as u16 / 255) as u8),
            b: self
                .b
                .saturating_add(((255 - self.b) as u16 * amount as u16 / 255) as u8),
            a: self.a,
        }
    }

    /// Check if color is dark
    pub fn is_dark(&self) -> bool {
        // Using relative luminance approximation
        let luminance = (self.r as u32 * 299 + self.g as u32 * 587 + self.b as u32 * 114) / 1000;
        luminance < 128
    }

    /// Get contrasting text color
    pub fn contrast_text(&self) -> Color {
        if self.is_dark() {
            colors::WHITE
        } else {
            colors::BLACK
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "#{:02X}{:02X}{:02X}{:02X}",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

/// Standard colors
pub mod colors {
    use super::Color;

    /// Black color (0, 0, 0)
    pub const BLACK: Color = Color::new(0, 0, 0);
    /// White color (255, 255, 255)
    pub const WHITE: Color = Color::new(255, 255, 255);
    /// Red color (255, 0, 0)
    pub const RED: Color = Color::new(255, 0, 0);
    /// Green color (0, 255, 0)
    pub const GREEN: Color = Color::new(0, 255, 0);
    /// Blue color (0, 0, 255)
    pub const BLUE: Color = Color::new(0, 0, 255);
    /// Yellow color (255, 255, 0)
    pub const YELLOW: Color = Color::new(255, 255, 0);
    /// Cyan color (0, 255, 255)
    pub const CYAN: Color = Color::new(0, 255, 255);
    /// Magenta color (255, 0, 255)
    pub const MAGENTA: Color = Color::new(255, 0, 255);

    /// Gray 900 - darkest gray
    pub const GRAY_900: Color = Color::new(17, 24, 39);
    /// Gray 800
    pub const GRAY_800: Color = Color::new(31, 41, 55);
    /// Gray 700
    pub const GRAY_700: Color = Color::new(55, 65, 81);
    /// Gray 600
    pub const GRAY_600: Color = Color::new(75, 85, 99);
    /// Gray 500 - medium gray
    pub const GRAY_500: Color = Color::new(107, 114, 128);
    /// Gray 400
    pub const GRAY_400: Color = Color::new(156, 163, 175);
    /// Gray 300
    pub const GRAY_300: Color = Color::new(209, 213, 219);
    /// Gray 200
    pub const GRAY_200: Color = Color::new(229, 231, 235);
    /// Gray 100
    pub const GRAY_100: Color = Color::new(243, 244, 246);
    /// Gray 50 - lightest gray
    pub const GRAY_50: Color = Color::new(249, 250, 251);

    /// Success status color (green)
    pub const SUCCESS: Color = Color::new(34, 197, 94);
    /// Warning status color (yellow/amber)
    pub const WARNING: Color = Color::new(234, 179, 8);
    /// Error status color (red)
    pub const ERROR: Color = Color::new(239, 68, 68);
    /// Info status color (blue)
    pub const INFO: Color = Color::new(59, 130, 246);

    /// Primary brand color (blue)
    pub const PRIMARY: Color = Color::new(59, 130, 246);
    /// Secondary brand color (gray)
    pub const SECONDARY: Color = Color::new(107, 114, 128);
    /// Accent brand color (purple)
    pub const ACCENT: Color = Color::new(139, 92, 246);

    /// Fully transparent color
    pub const TRANSPARENT: Color = Color::with_alpha(0, 0, 0, 0);
    /// Semi-transparent black (50% opacity)
    pub const SEMI_TRANSPARENT: Color = Color::with_alpha(0, 0, 0, 128);
}

// =============================================================================
// COLOR SCHEME
// =============================================================================

/// Color scheme type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSchemeType {
    /// Dark mode
    #[default]
    Dark,
    /// Light mode
    Light,
    /// High contrast dark
    HighContrastDark,
    /// High contrast light
    HighContrastLight,
    /// System (follow firmware setting)
    System,
    /// Custom
    Custom,
}

impl fmt::Display for ColorSchemeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorSchemeType::Dark => write!(f, "Dark"),
            ColorSchemeType::Light => write!(f, "Light"),
            ColorSchemeType::HighContrastDark => write!(f, "High Contrast Dark"),
            ColorSchemeType::HighContrastLight => write!(f, "High Contrast Light"),
            ColorSchemeType::System => write!(f, "System"),
            ColorSchemeType::Custom => write!(f, "Custom"),
        }
    }
}

/// Color scheme
#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    /// Scheme type
    pub scheme_type: ColorSchemeType,

    // Background colors
    /// Background primary
    pub background: Color,
    /// Background secondary
    pub background_secondary: Color,
    /// Background tertiary
    pub background_tertiary: Color,
    /// Surface color
    pub surface: Color,
    /// Overlay color
    pub overlay: Color,

    // Text colors
    /// Primary text
    pub text_primary: Color,
    /// Secondary text
    pub text_secondary: Color,
    /// Disabled text
    pub text_disabled: Color,
    /// Inverse text
    pub text_inverse: Color,

    // Interactive colors
    /// Primary accent
    pub primary: Color,
    /// Primary hover
    pub primary_hover: Color,
    /// Primary active
    pub primary_active: Color,
    /// Secondary accent
    pub secondary: Color,

    // Status colors
    /// Success
    pub success: Color,
    /// Warning
    pub warning: Color,
    /// Error
    pub error: Color,
    /// Info
    pub info: Color,

    // Border colors
    /// Border color
    pub border: Color,
    /// Border focused
    pub border_focus: Color,

    // Selection
    /// Selected background
    pub selected_bg: Color,
    /// Selected text
    pub selected_text: Color,
    /// Highlight
    pub highlight: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl ColorScheme {
    /// Dark theme
    pub const fn dark() -> Self {
        Self {
            scheme_type: ColorSchemeType::Dark,
            background: colors::GRAY_900,
            background_secondary: colors::GRAY_800,
            background_tertiary: colors::GRAY_700,
            surface: Color::new(24, 32, 48),
            overlay: Color::with_alpha(0, 0, 0, 192),
            text_primary: colors::WHITE,
            text_secondary: colors::GRAY_300,
            text_disabled: colors::GRAY_500,
            text_inverse: colors::GRAY_900,
            primary: Color::new(59, 130, 246),
            primary_hover: Color::new(37, 99, 235),
            primary_active: Color::new(29, 78, 216),
            secondary: colors::GRAY_600,
            success: Color::new(34, 197, 94),
            warning: Color::new(234, 179, 8),
            error: Color::new(239, 68, 68),
            info: Color::new(59, 130, 246),
            border: colors::GRAY_700,
            border_focus: Color::new(59, 130, 246),
            selected_bg: Color::new(30, 64, 175),
            selected_text: colors::WHITE,
            highlight: Color::with_alpha(59, 130, 246, 64),
        }
    }

    /// Light theme
    pub const fn light() -> Self {
        Self {
            scheme_type: ColorSchemeType::Light,
            background: colors::GRAY_50,
            background_secondary: colors::GRAY_100,
            background_tertiary: colors::GRAY_200,
            surface: colors::WHITE,
            overlay: Color::with_alpha(0, 0, 0, 128),
            text_primary: colors::GRAY_900,
            text_secondary: colors::GRAY_600,
            text_disabled: colors::GRAY_400,
            text_inverse: colors::WHITE,
            primary: Color::new(37, 99, 235),
            primary_hover: Color::new(29, 78, 216),
            primary_active: Color::new(30, 64, 175),
            secondary: colors::GRAY_400,
            success: Color::new(22, 163, 74),
            warning: Color::new(202, 138, 4),
            error: Color::new(220, 38, 38),
            info: Color::new(37, 99, 235),
            border: colors::GRAY_300,
            border_focus: Color::new(37, 99, 235),
            selected_bg: Color::new(59, 130, 246),
            selected_text: colors::WHITE,
            highlight: Color::with_alpha(59, 130, 246, 32),
        }
    }

    /// High contrast dark theme
    pub const fn high_contrast_dark() -> Self {
        Self {
            scheme_type: ColorSchemeType::HighContrastDark,
            background: colors::BLACK,
            background_secondary: Color::new(16, 16, 16),
            background_tertiary: Color::new(32, 32, 32),
            surface: Color::new(8, 8, 8),
            overlay: Color::with_alpha(0, 0, 0, 224),
            text_primary: colors::WHITE,
            text_secondary: Color::new(220, 220, 220),
            text_disabled: Color::new(128, 128, 128),
            text_inverse: colors::BLACK,
            primary: Color::new(0, 191, 255),
            primary_hover: Color::new(0, 255, 255),
            primary_active: Color::new(0, 128, 255),
            secondary: Color::new(128, 128, 128),
            success: Color::new(0, 255, 0),
            warning: Color::new(255, 255, 0),
            error: Color::new(255, 0, 0),
            info: Color::new(0, 191, 255),
            border: colors::WHITE,
            border_focus: Color::new(0, 255, 255),
            selected_bg: Color::new(0, 0, 255),
            selected_text: colors::WHITE,
            highlight: Color::with_alpha(255, 255, 0, 64),
        }
    }

    /// High contrast light theme
    pub const fn high_contrast_light() -> Self {
        Self {
            scheme_type: ColorSchemeType::HighContrastLight,
            background: colors::WHITE,
            background_secondary: Color::new(240, 240, 240),
            background_tertiary: Color::new(224, 224, 224),
            surface: colors::WHITE,
            overlay: Color::with_alpha(255, 255, 255, 224),
            text_primary: colors::BLACK,
            text_secondary: Color::new(32, 32, 32),
            text_disabled: Color::new(128, 128, 128),
            text_inverse: colors::WHITE,
            primary: Color::new(0, 0, 192),
            primary_hover: Color::new(0, 0, 128),
            primary_active: Color::new(0, 0, 96),
            secondary: Color::new(96, 96, 96),
            success: Color::new(0, 128, 0),
            warning: Color::new(192, 128, 0),
            error: Color::new(192, 0, 0),
            info: Color::new(0, 0, 192),
            border: colors::BLACK,
            border_focus: Color::new(0, 0, 192),
            selected_bg: Color::new(0, 0, 192),
            selected_text: colors::WHITE,
            highlight: Color::with_alpha(0, 0, 192, 32),
        }
    }
}

// =============================================================================
// FONT CONFIGURATION
// =============================================================================

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    /// Thin (100)
    Thin,
    /// Extra light (200)
    ExtraLight,
    /// Light (300)
    Light,
    /// Normal/Regular (400)
    #[default]
    Normal,
    /// Medium (500)
    Medium,
    /// Semibold (600)
    Semibold,
    /// Bold (700)
    Bold,
    /// Extra bold (800)
    ExtraBold,
    /// Black (900)
    Black,
}

impl FontWeight {
    /// Get numeric weight
    pub const fn weight(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Normal => 400,
            FontWeight::Medium => 500,
            FontWeight::Semibold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    /// Normal
    #[default]
    Normal,
    /// Italic
    Italic,
    /// Oblique
    Oblique,
}

/// Font size preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontSizePreset {
    /// Extra small
    XSmall,
    /// Small
    Small,
    /// Medium (default)
    #[default]
    Medium,
    /// Large
    Large,
    /// Extra large
    XLarge,
    /// 2X large
    XXLarge,
}

impl FontSizePreset {
    /// Get size in pixels (at 96 DPI)
    pub const fn size_px(&self) -> u16 {
        match self {
            FontSizePreset::XSmall => 10,
            FontSizePreset::Small => 12,
            FontSizePreset::Medium => 14,
            FontSizePreset::Large => 16,
            FontSizePreset::XLarge => 20,
            FontSizePreset::XXLarge => 24,
        }
    }
}

/// Font configuration
#[derive(Debug, Clone, Copy)]
pub struct FontConfig {
    /// Font size
    pub size: u16,
    /// Line height (0 = auto)
    pub line_height: u16,
    /// Font weight
    pub weight: FontWeight,
    /// Font style
    pub style: FontStyle,
    /// Letter spacing (in 1/64 em)
    pub letter_spacing: i8,
    /// Monospace
    pub monospace: bool,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            size: 14,
            line_height: 0,
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            letter_spacing: 0,
            monospace: false,
        }
    }
}

impl FontConfig {
    /// Get effective line height
    pub const fn effective_line_height(&self) -> u16 {
        if self.line_height > 0 {
            self.line_height
        } else {
            // Default: 150% of font size
            (self.size * 3) / 2
        }
    }
}

// =============================================================================
// SPACING AND SIZING
// =============================================================================

/// Spacing preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpacingPreset {
    /// None (0)
    None,
    /// Extra small (4px)
    XSmall,
    /// Small (8px)
    Small,
    /// Medium (16px)
    #[default]
    Medium,
    /// Large (24px)
    Large,
    /// Extra large (32px)
    XLarge,
    /// 2X large (48px)
    XXLarge,
}

impl SpacingPreset {
    /// Get value in pixels
    pub const fn px(&self) -> u16 {
        match self {
            SpacingPreset::None => 0,
            SpacingPreset::XSmall => 4,
            SpacingPreset::Small => 8,
            SpacingPreset::Medium => 16,
            SpacingPreset::Large => 24,
            SpacingPreset::XLarge => 32,
            SpacingPreset::XXLarge => 48,
        }
    }
}

/// Border radius preset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RadiusPreset {
    /// None (square corners)
    None,
    /// Small (2px)
    Small,
    /// Medium (4px)
    #[default]
    Medium,
    /// Large (8px)
    Large,
    /// Extra large (12px)
    XLarge,
    /// Full (half of height)
    Full,
}

impl RadiusPreset {
    /// Get value in pixels
    pub const fn px(&self) -> u16 {
        match self {
            RadiusPreset::None => 0,
            RadiusPreset::Small => 2,
            RadiusPreset::Medium => 4,
            RadiusPreset::Large => 8,
            RadiusPreset::XLarge => 12,
            RadiusPreset::Full => 9999,
        }
    }
}

/// Border width
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderWidth {
    /// None
    None,
    /// Thin (1px)
    #[default]
    Thin,
    /// Medium (2px)
    Medium,
    /// Thick (4px)
    Thick,
}

impl BorderWidth {
    /// Get value in pixels
    pub const fn px(&self) -> u16 {
        match self {
            BorderWidth::None => 0,
            BorderWidth::Thin => 1,
            BorderWidth::Medium => 2,
            BorderWidth::Thick => 4,
        }
    }
}

// =============================================================================
// COMPONENT STYLES
// =============================================================================

/// Button style
#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {
    /// Background color
    pub background: Color,
    /// Background hover
    pub background_hover: Color,
    /// Background active
    pub background_active: Color,
    /// Background disabled
    pub background_disabled: Color,
    /// Text color
    pub text: Color,
    /// Text disabled
    pub text_disabled: Color,
    /// Border color
    pub border: Color,
    /// Border width
    pub border_width: BorderWidth,
    /// Border radius
    pub radius: RadiusPreset,
    /// Padding horizontal
    pub padding_x: SpacingPreset,
    /// Padding vertical
    pub padding_y: SpacingPreset,
    /// Font config
    pub font: FontConfig,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            background: colors::PRIMARY,
            background_hover: Color::new(37, 99, 235),
            background_active: Color::new(29, 78, 216),
            background_disabled: colors::GRAY_500,
            text: colors::WHITE,
            text_disabled: colors::GRAY_300,
            border: colors::TRANSPARENT,
            border_width: BorderWidth::None,
            radius: RadiusPreset::Medium,
            padding_x: SpacingPreset::Medium,
            padding_y: SpacingPreset::Small,
            font: FontConfig::default(),
        }
    }
}

/// Menu item style
#[derive(Debug, Clone, Copy)]
pub struct MenuItemStyle {
    /// Background normal
    pub background: Color,
    /// Background selected
    pub background_selected: Color,
    /// Background hover
    pub background_hover: Color,
    /// Text color
    pub text: Color,
    /// Text selected
    pub text_selected: Color,
    /// Text secondary (for description)
    pub text_secondary: Color,
    /// Icon color
    pub icon: Color,
    /// Icon selected
    pub icon_selected: Color,
    /// Height
    pub height: u16,
    /// Padding
    pub padding: SpacingPreset,
    /// Border radius
    pub radius: RadiusPreset,
    /// Show separator
    pub separator: bool,
    /// Separator color
    pub separator_color: Color,
}

impl Default for MenuItemStyle {
    fn default() -> Self {
        Self {
            background: colors::TRANSPARENT,
            background_selected: Color::new(30, 64, 175),
            background_hover: Color::with_alpha(255, 255, 255, 16),
            text: colors::WHITE,
            text_selected: colors::WHITE,
            text_secondary: colors::GRAY_400,
            icon: colors::GRAY_400,
            icon_selected: colors::WHITE,
            height: 48,
            padding: SpacingPreset::Medium,
            radius: RadiusPreset::Small,
            separator: false,
            separator_color: colors::GRAY_700,
        }
    }
}

/// Progress bar style
#[derive(Debug, Clone, Copy)]
pub struct ProgressStyle {
    /// Background color
    pub background: Color,
    /// Fill color
    pub fill: Color,
    /// Fill success
    pub fill_success: Color,
    /// Fill error
    pub fill_error: Color,
    /// Border color
    pub border: Color,
    /// Height
    pub height: u16,
    /// Border radius
    pub radius: RadiusPreset,
    /// Animated
    pub animated: bool,
    /// Show percentage
    pub show_percent: bool,
}

impl Default for ProgressStyle {
    fn default() -> Self {
        Self {
            background: colors::GRAY_700,
            fill: colors::PRIMARY,
            fill_success: colors::SUCCESS,
            fill_error: colors::ERROR,
            border: colors::TRANSPARENT,
            height: 8,
            radius: RadiusPreset::Full,
            animated: true,
            show_percent: false,
        }
    }
}

/// Dialog style
#[derive(Debug, Clone, Copy)]
pub struct DialogStyle {
    /// Background color
    pub background: Color,
    /// Border color
    pub border: Color,
    /// Border width
    pub border_width: BorderWidth,
    /// Border radius
    pub radius: RadiusPreset,
    /// Shadow
    pub shadow: bool,
    /// Overlay color
    pub overlay: Color,
    /// Header background
    pub header_bg: Color,
    /// Header text
    pub header_text: Color,
    /// Padding
    pub padding: SpacingPreset,
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self {
            background: colors::GRAY_800,
            border: colors::GRAY_700,
            border_width: BorderWidth::Thin,
            radius: RadiusPreset::Large,
            shadow: true,
            overlay: Color::with_alpha(0, 0, 0, 192),
            header_bg: colors::GRAY_900,
            header_text: colors::WHITE,
            padding: SpacingPreset::Large,
        }
    }
}

// =============================================================================
// THEME
// =============================================================================

/// Complete theme
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Theme name
    pub name: [u8; 32],
    /// Name length
    pub name_len: usize,
    /// Color scheme
    pub colors: ColorScheme,
    /// Button style
    pub button: ButtonStyle,
    /// Menu item style
    pub menu_item: MenuItemStyle,
    /// Progress style
    pub progress: ProgressStyle,
    /// Dialog style
    pub dialog: DialogStyle,
    /// Default font
    pub font: FontConfig,
    /// Monospace font
    pub font_mono: FontConfig,
    /// Animation enabled
    pub animations: bool,
    /// Animation speed (1-10, 5 = normal)
    pub animation_speed: u8,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Dark theme
    pub fn dark() -> Self {
        Self {
            name: *b"Helix Dark\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            name_len: 10,
            colors: ColorScheme::dark(),
            button: ButtonStyle::default(),
            menu_item: MenuItemStyle::default(),
            progress: ProgressStyle::default(),
            dialog: DialogStyle::default(),
            font: FontConfig::default(),
            font_mono: FontConfig {
                monospace: true,
                ..Default::default()
            },
            animations: true,
            animation_speed: 5,
        }
    }

    /// Light theme
    pub fn light() -> Self {
        let mut theme = Self::dark();
        theme.name = *b"Helix Light\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        theme.name_len = 11;
        theme.colors = ColorScheme::light();

        // Update component styles for light theme
        theme.button.background = theme.colors.primary;
        theme.button.background_hover = theme.colors.primary_hover;
        theme.button.background_active = theme.colors.primary_active;

        theme.menu_item.text = theme.colors.text_primary;
        theme.menu_item.text_secondary = theme.colors.text_secondary;
        theme.menu_item.background_hover = Color::with_alpha(0, 0, 0, 8);

        theme.progress.background = theme.colors.border;

        theme.dialog.background = theme.colors.surface;
        theme.dialog.border = theme.colors.border;
        theme.dialog.header_bg = theme.colors.background_secondary;
        theme.dialog.header_text = theme.colors.text_primary;

        theme
    }

    /// High contrast dark theme
    pub fn high_contrast_dark() -> Self {
        let mut theme = Self::dark();
        theme.name = *b"High Contrast Dark\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        theme.name_len = 18;
        theme.colors = ColorScheme::high_contrast_dark();
        theme.animations = false;
        theme
    }

    /// Get theme name
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

// =============================================================================
// THEME MANAGER
// =============================================================================

/// Maximum custom themes
pub const MAX_THEMES: usize = 8;

/// Theme manager
#[derive(Debug)]
pub struct ThemeManager {
    /// Current theme
    pub current: Theme,
    /// Available themes
    themes: [Theme; MAX_THEMES],
    /// Theme count
    count: usize,
    /// Current theme index
    current_index: usize,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeManager {
    /// Create new manager with default themes
    pub fn new() -> Self {
        let mut manager = Self {
            current: Theme::dark(),
            themes: [Theme::dark(); MAX_THEMES],
            count: 3,
            current_index: 0,
        };
        manager.themes[0] = Theme::dark();
        manager.themes[1] = Theme::light();
        manager.themes[2] = Theme::high_contrast_dark();
        manager
    }

    /// Add theme
    pub fn add_theme(&mut self, theme: Theme) -> bool {
        if self.count >= MAX_THEMES {
            return false;
        }
        self.themes[self.count] = theme;
        self.count += 1;
        true
    }

    /// Select theme by index
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.count {
            return false;
        }
        self.current = self.themes[index];
        self.current_index = index;
        true
    }

    /// Get theme count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get current index
    pub const fn current_index(&self) -> usize {
        self.current_index
    }

    /// Cycle to next theme
    pub fn next(&mut self) {
        let next = (self.current_index + 1) % self.count;
        self.select(next);
    }

    /// Cycle to previous theme
    pub fn previous(&mut self) {
        let prev = if self.current_index == 0 {
            self.count - 1
        } else {
            self.current_index - 1
        };
        self.select(prev);
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
        let color = Color::new(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_color_from_rgb() {
        let color = Color::from_rgb(0xFF8040);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
    }

    #[test]
    fn test_color_blend() {
        let white = colors::WHITE;
        let black = colors::BLACK;
        let gray = white.blend(black, 128);
        assert!(gray.r > 100 && gray.r < 150);
    }

    #[test]
    fn test_color_is_dark() {
        assert!(colors::BLACK.is_dark());
        assert!(!colors::WHITE.is_dark());
    }

    #[test]
    fn test_color_scheme() {
        let dark = ColorScheme::dark();
        assert_eq!(dark.scheme_type, ColorSchemeType::Dark);

        let light = ColorScheme::light();
        assert_eq!(light.scheme_type, ColorSchemeType::Light);
    }

    #[test]
    fn test_theme_manager() {
        let mut manager = ThemeManager::new();
        assert_eq!(manager.len(), 3);

        manager.next();
        assert_eq!(manager.current_index(), 1);

        manager.previous();
        assert_eq!(manager.current_index(), 0);
    }
}
