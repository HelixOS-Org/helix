//! # Theme System
//!
//! Complete theming support.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{Color, FontId};

/// Theme manager
pub struct ThemeManager {
    themes: BTreeMap<String, ThemeDefinition>,
    active: String,
    transition: Option<ThemeTransition>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let mut manager = Self {
            themes: BTreeMap::new(),
            active: "dark".into(),
            transition: None,
        };

        manager.register("dark", ThemeDefinition::dark());
        manager.register("light", ThemeDefinition::light());

        manager
    }

    /// Register a theme
    pub fn register(&mut self, name: &str, theme: ThemeDefinition) {
        self.themes.insert(name.into(), theme);
    }

    /// Get active theme
    pub fn active(&self) -> Option<&ThemeDefinition> {
        self.themes.get(&self.active)
    }

    /// Set active theme
    pub fn set_active(&mut self, name: &str) {
        if self.themes.contains_key(name) {
            self.active = name.into();
        }
    }

    /// Set active theme with transition
    pub fn transition_to(&mut self, name: &str, duration: f32) {
        if let (Some(from), Some(to)) = (self.themes.get(&self.active), self.themes.get(name)) {
            self.transition = Some(ThemeTransition {
                from: from.clone(),
                to: to.clone(),
                target_name: name.into(),
                duration,
                elapsed: 0.0,
            });
        }
    }

    /// Update transitions
    pub fn update(&mut self, dt: f32) {
        if let Some(ref mut transition) = self.transition {
            transition.elapsed += dt;
            if transition.elapsed >= transition.duration {
                self.active = transition.target_name.clone();
                self.transition = None;
            }
        }
    }

    /// Get current (possibly interpolated) color
    pub fn get_color(&self, token: &str) -> Color {
        if let Some(ref transition) = self.transition {
            let t = (transition.elapsed / transition.duration).clamp(0.0, 1.0);
            let from = transition.from.get_color(token);
            let to = transition.to.get_color(token);
            from.lerp(to, t)
        } else if let Some(theme) = self.active() {
            theme.get_color(token)
        } else {
            Color::WHITE
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme transition state
struct ThemeTransition {
    from: ThemeDefinition,
    to: ThemeDefinition,
    target_name: String,
    duration: f32,
    elapsed: f32,
}

/// Theme definition
#[derive(Debug, Clone)]
pub struct ThemeDefinition {
    pub name: String,
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub shadows: ShadowTokens,
    pub borders: BorderTokens,
    pub animations: AnimationTokens,
}

impl ThemeDefinition {
    pub fn dark() -> Self {
        Self {
            name: "Dark".into(),
            colors: ColorTokens {
                // Backgrounds
                bg_primary: Color::hex(0x1E1E1E),
                bg_secondary: Color::hex(0x252526),
                bg_tertiary: Color::hex(0x2D2D2D),
                bg_elevated: Color::hex(0x3C3C3C),

                // Foregrounds
                fg_primary: Color::hex(0xCCCCCC),
                fg_secondary: Color::hex(0x969696),
                fg_tertiary: Color::hex(0x6E6E6E),
                fg_disabled: Color::hex(0x4D4D4D),

                // Accents
                accent_primary: Color::hex(0x007ACC),
                accent_secondary: Color::hex(0x0098FF),
                accent_muted: Color::hex(0x094771),

                // Semantic
                success: Color::hex(0x4EC9B0),
                warning: Color::hex(0xCCA700),
                error: Color::hex(0xF44747),
                info: Color::hex(0x3794FF),

                // Borders
                border_default: Color::hex(0x474747),
                border_focused: Color::hex(0x007ACC),
                border_error: Color::hex(0xF44747),

                // Interactive
                hover: Color::rgba(1.0, 1.0, 1.0, 0.1),
                pressed: Color::rgba(1.0, 1.0, 1.0, 0.05),
                selected: Color::hex(0x094771),

                // Overlay
                overlay: Color::rgba(0.0, 0.0, 0.0, 0.5),
                shadow: Color::rgba(0.0, 0.0, 0.0, 0.3),
            },
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            shadows: ShadowTokens::dark(),
            borders: BorderTokens::default(),
            animations: AnimationTokens::default(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".into(),
            colors: ColorTokens {
                // Backgrounds
                bg_primary: Color::hex(0xFFFFFF),
                bg_secondary: Color::hex(0xF3F3F3),
                bg_tertiary: Color::hex(0xE8E8E8),
                bg_elevated: Color::hex(0xFFFFFF),

                // Foregrounds
                fg_primary: Color::hex(0x1E1E1E),
                fg_secondary: Color::hex(0x444444),
                fg_tertiary: Color::hex(0x6E6E6E),
                fg_disabled: Color::hex(0xA0A0A0),

                // Accents
                accent_primary: Color::hex(0x0078D4),
                accent_secondary: Color::hex(0x106EBE),
                accent_muted: Color::hex(0xCCE4F7),

                // Semantic
                success: Color::hex(0x107C10),
                warning: Color::hex(0xF7630C),
                error: Color::hex(0xE51400),
                info: Color::hex(0x0078D4),

                // Borders
                border_default: Color::hex(0xCCCCCC),
                border_focused: Color::hex(0x0078D4),
                border_error: Color::hex(0xE51400),

                // Interactive
                hover: Color::rgba(0.0, 0.0, 0.0, 0.05),
                pressed: Color::rgba(0.0, 0.0, 0.0, 0.1),
                selected: Color::hex(0xCCE4F7),

                // Overlay
                overlay: Color::rgba(0.0, 0.0, 0.0, 0.3),
                shadow: Color::rgba(0.0, 0.0, 0.0, 0.15),
            },
            typography: TypographyTokens::default(),
            spacing: SpacingTokens::default(),
            shadows: ShadowTokens::light(),
            borders: BorderTokens::default(),
            animations: AnimationTokens::default(),
        }
    }

    pub fn get_color(&self, token: &str) -> Color {
        match token {
            "bg.primary" => self.colors.bg_primary,
            "bg.secondary" => self.colors.bg_secondary,
            "bg.tertiary" => self.colors.bg_tertiary,
            "bg.elevated" => self.colors.bg_elevated,
            "fg.primary" => self.colors.fg_primary,
            "fg.secondary" => self.colors.fg_secondary,
            "fg.tertiary" => self.colors.fg_tertiary,
            "fg.disabled" => self.colors.fg_disabled,
            "accent.primary" => self.colors.accent_primary,
            "accent.secondary" => self.colors.accent_secondary,
            "accent.muted" => self.colors.accent_muted,
            "success" => self.colors.success,
            "warning" => self.colors.warning,
            "error" => self.colors.error,
            "info" => self.colors.info,
            "border.default" => self.colors.border_default,
            "border.focused" => self.colors.border_focused,
            "border.error" => self.colors.border_error,
            "hover" => self.colors.hover,
            "pressed" => self.colors.pressed,
            "selected" => self.colors.selected,
            "overlay" => self.colors.overlay,
            "shadow" => self.colors.shadow,
            _ => Color::WHITE,
        }
    }
}

/// Color tokens
#[derive(Debug, Clone)]
pub struct ColorTokens {
    // Backgrounds
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_tertiary: Color,
    pub bg_elevated: Color,

    // Foregrounds
    pub fg_primary: Color,
    pub fg_secondary: Color,
    pub fg_tertiary: Color,
    pub fg_disabled: Color,

    // Accents
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_muted: Color,

    // Semantic
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Borders
    pub border_default: Color,
    pub border_focused: Color,
    pub border_error: Color,

    // Interactive states
    pub hover: Color,
    pub pressed: Color,
    pub selected: Color,

    // Overlays
    pub overlay: Color,
    pub shadow: Color,
}

/// Typography tokens
#[derive(Debug, Clone)]
pub struct TypographyTokens {
    pub font_family: FontId,
    pub font_family_mono: FontId,
    pub font_size_xs: f32,
    pub font_size_sm: f32,
    pub font_size_md: f32,
    pub font_size_lg: f32,
    pub font_size_xl: f32,
    pub font_size_2xl: f32,
    pub line_height_tight: f32,
    pub line_height_normal: f32,
    pub line_height_relaxed: f32,
    pub letter_spacing_tight: f32,
    pub letter_spacing_normal: f32,
    pub letter_spacing_wide: f32,
}

impl Default for TypographyTokens {
    fn default() -> Self {
        Self {
            font_family: FontId(0),
            font_family_mono: FontId(1),
            font_size_xs: 10.0,
            font_size_sm: 12.0,
            font_size_md: 14.0,
            font_size_lg: 16.0,
            font_size_xl: 20.0,
            font_size_2xl: 24.0,
            line_height_tight: 1.2,
            line_height_normal: 1.5,
            line_height_relaxed: 1.8,
            letter_spacing_tight: -0.5,
            letter_spacing_normal: 0.0,
            letter_spacing_wide: 0.5,
        }
    }
}

/// Spacing tokens
#[derive(Debug, Clone)]
pub struct SpacingTokens {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
        }
    }
}

/// Shadow tokens
#[derive(Debug, Clone)]
pub struct ShadowTokens {
    pub sm: Shadow,
    pub md: Shadow,
    pub lg: Shadow,
    pub xl: Shadow,
}

impl ShadowTokens {
    pub fn dark() -> Self {
        Self {
            sm: Shadow {
                offset: [0.0, 1.0],
                blur: 2.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.3),
            },
            md: Shadow {
                offset: [0.0, 4.0],
                blur: 8.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.4),
            },
            lg: Shadow {
                offset: [0.0, 8.0],
                blur: 16.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.5),
            },
            xl: Shadow {
                offset: [0.0, 16.0],
                blur: 32.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.6),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            sm: Shadow {
                offset: [0.0, 1.0],
                blur: 2.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.1),
            },
            md: Shadow {
                offset: [0.0, 4.0],
                blur: 8.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.15),
            },
            lg: Shadow {
                offset: [0.0, 8.0],
                blur: 16.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.2),
            },
            xl: Shadow {
                offset: [0.0, 16.0],
                blur: 32.0,
                color: Color::rgba(0.0, 0.0, 0.0, 0.25),
            },
        }
    }
}

/// Shadow definition
#[derive(Debug, Clone)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: Color,
}

/// Border tokens
#[derive(Debug, Clone)]
pub struct BorderTokens {
    pub width_thin: f32,
    pub width_medium: f32,
    pub width_thick: f32,
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    pub radius_full: f32,
}

impl Default for BorderTokens {
    fn default() -> Self {
        Self {
            width_thin: 1.0,
            width_medium: 2.0,
            width_thick: 4.0,
            radius_sm: 2.0,
            radius_md: 4.0,
            radius_lg: 8.0,
            radius_full: 9999.0,
        }
    }
}

/// Animation tokens
#[derive(Debug, Clone)]
pub struct AnimationTokens {
    pub duration_fast: f32,
    pub duration_normal: f32,
    pub duration_slow: f32,
}

impl Default for AnimationTokens {
    fn default() -> Self {
        Self {
            duration_fast: 0.1,
            duration_normal: 0.2,
            duration_slow: 0.4,
        }
    }
}
