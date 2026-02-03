//! # Style System
//!
//! CSS-like styling for widgets.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::layout::{Dimension, Edges};
use crate::{Color, Rect};

/// Widget style
#[derive(Debug, Clone, Default)]
pub struct Style {
    // Layout
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub min_width: Option<Dimension>,
    pub min_height: Option<Dimension>,
    pub max_width: Option<Dimension>,
    pub max_height: Option<Dimension>,
    pub margin: Option<Edges>,
    pub padding: Option<Edges>,

    // Visual
    pub background: Option<Background>,
    pub border: Option<Border>,
    pub shadow: Option<BoxShadow>,
    pub opacity: Option<f32>,
    pub visibility: Option<Visibility>,

    // Text
    pub color: Option<Color>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub text_align: Option<TextAlign>,
    pub text_decoration: Option<TextDecoration>,

    // Cursor
    pub cursor: Option<Cursor>,

    // Transitions
    pub transition: Option<Transition>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge another style on top
    pub fn merge(&self, other: &Style) -> Style {
        Style {
            width: other.width.or(self.width),
            height: other.height.or(self.height),
            min_width: other.min_width.or(self.min_width),
            min_height: other.min_height.or(self.min_height),
            max_width: other.max_width.or(self.max_width),
            max_height: other.max_height.or(self.max_height),
            margin: other.margin.or(self.margin),
            padding: other.padding.or(self.padding),
            background: other.background.clone().or(self.background.clone()),
            border: other.border.clone().or(self.border.clone()),
            shadow: other.shadow.clone().or(self.shadow.clone()),
            opacity: other.opacity.or(self.opacity),
            visibility: other.visibility.or(self.visibility),
            color: other.color.or(self.color),
            font_size: other.font_size.or(self.font_size),
            font_weight: other.font_weight.or(self.font_weight),
            text_align: other.text_align.or(self.text_align),
            text_decoration: other.text_decoration.or(self.text_decoration),
            cursor: other.cursor.or(self.cursor),
            transition: other.transition.clone().or(self.transition.clone()),
        }
    }
}

/// Background style
#[derive(Debug, Clone)]
pub enum Background {
    Color(Color),
    Gradient(Gradient),
    Image { texture: u64, fit: BackgroundFit },
}

/// Gradient definition
#[derive(Debug, Clone)]
pub struct Gradient {
    pub gradient_type: GradientType,
    pub stops: Vec<GradientStop>,
}

/// Gradient type
#[derive(Debug, Clone, Copy)]
pub enum GradientType {
    Linear { angle: f32 },
    Radial { center: [f32; 2] },
}

/// Gradient stop
#[derive(Debug, Clone)]
pub struct GradientStop {
    pub color: Color,
    pub position: f32,
}

/// Background fit
#[derive(Debug, Clone, Copy)]
pub enum BackgroundFit {
    Fill,
    Contain,
    Cover,
    None,
}

/// Border style
#[derive(Debug, Clone)]
pub struct Border {
    pub width: BorderWidth,
    pub color: Color,
    pub style: BorderStyle,
    pub radius: BorderRadius,
}

/// Border width (can be different per side)
#[derive(Debug, Clone, Copy)]
pub struct BorderWidth {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl BorderWidth {
    pub fn all(width: f32) -> Self {
        Self {
            top: width,
            right: width,
            bottom: width,
            left: width,
        }
    }
}

/// Border style
#[derive(Debug, Clone, Copy)]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
    None,
}

/// Border radius
#[derive(Debug, Clone, Copy)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
}

/// Box shadow
#[derive(Debug, Clone)]
pub struct BoxShadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub spread: f32,
    pub color: Color,
    pub inset: bool,
}

/// Visibility
#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Visible,
    Hidden,
    Collapse,
}

/// Font weight
#[derive(Debug, Clone, Copy)]
pub enum FontWeight {
    Thin,
    Light,
    Regular,
    Medium,
    Semibold,
    Bold,
    Black,
    Numeric(u16),
}

/// Text alignment
#[derive(Debug, Clone, Copy)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Text decoration
#[derive(Debug, Clone, Copy)]
pub enum TextDecoration {
    None,
    Underline,
    LineThrough,
    Overline,
}

/// Cursor style
#[derive(Debug, Clone, Copy)]
pub enum Cursor {
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
    Grab,
    Grabbing,
    ResizeNS,
    ResizeEW,
    ResizeNESW,
    ResizeNWSE,
    Crosshair,
    Wait,
    Progress,
}

/// Transition
#[derive(Debug, Clone)]
pub struct Transition {
    pub property: TransitionProperty,
    pub duration: f32,
    pub delay: f32,
    pub timing: TimingFunction,
}

/// Transition property
#[derive(Debug, Clone)]
pub enum TransitionProperty {
    All,
    Property(String),
    Properties(Vec<String>),
}

/// Timing function
#[derive(Debug, Clone, Copy)]
pub enum TimingFunction {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f32, f32, f32, f32),
}

/// Stylesheet
pub struct Stylesheet {
    rules: Vec<StyleRule>,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule
    pub fn add_rule(&mut self, selector: &str, style: Style) {
        self.rules.push(StyleRule {
            selector: selector.into(),
            style,
            specificity: calculate_specificity(selector),
        });
    }

    /// Get computed style for selectors
    pub fn compute(&self, selectors: &[&str]) -> Style {
        let mut result = Style::default();

        // Sort rules by specificity
        let mut matching: Vec<_> = self
            .rules
            .iter()
            .filter(|r| selectors.iter().any(|s| matches_selector(&r.selector, s)))
            .collect();

        matching.sort_by_key(|r| r.specificity);

        for rule in matching {
            result = result.merge(&rule.style);
        }

        result
    }
}

impl Default for Stylesheet {
    fn default() -> Self {
        Self::new()
    }
}

/// Style rule
struct StyleRule {
    selector: String,
    style: Style,
    specificity: u32,
}

fn calculate_specificity(selector: &str) -> u32 {
    let mut specificity = 0u32;

    // IDs (#id) = 100
    specificity += selector.matches('#').count() as u32 * 100;

    // Classes (.class) and pseudo-classes = 10
    specificity += selector.matches('.').count() as u32 * 10;
    specificity += selector.matches(':').count() as u32 * 10;

    // Elements = 1
    for part in selector.split(&[' ', '>', '+', '~'][..]) {
        if !part.starts_with('#') && !part.starts_with('.') && !part.starts_with(':') {
            specificity += 1;
        }
    }

    specificity
}

fn matches_selector(rule: &str, selector: &str) -> bool {
    // Simplified matching
    rule == selector || rule == "*"
}

/// Style builder for fluent API
pub struct StyleBuilder {
    style: Style,
}

impl StyleBuilder {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.style.width = Some(Dimension::Pixels(w));
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.style.height = Some(Dimension::Pixels(h));
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(Background::Color(color));
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.style.border = Some(Border {
            width: BorderWidth::all(width),
            color,
            style: BorderStyle::Solid,
            radius: BorderRadius::all(0.0),
        });
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        if let Some(ref mut border) = self.style.border {
            border.radius = BorderRadius::all(radius);
        } else {
            self.style.border = Some(Border {
                width: BorderWidth::all(0.0),
                color: Color::TRANSPARENT,
                style: BorderStyle::None,
                radius: BorderRadius::all(radius),
            });
        }
        self
    }

    pub fn padding(mut self, value: f32) -> Self {
        self.style.padding = Some(Edges::all(value));
        self
    }

    pub fn margin(mut self, value: f32) -> Self {
        self.style.margin = Some(Edges::all(value));
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.style.font_size = Some(size);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.style.opacity = Some(opacity);
        self
    }

    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.style.cursor = Some(cursor);
        self
    }

    pub fn shadow(mut self, offset_x: f32, offset_y: f32, blur: f32, color: Color) -> Self {
        self.style.shadow = Some(BoxShadow {
            offset: [offset_x, offset_y],
            blur,
            spread: 0.0,
            color,
            inset: false,
        });
        self
    }

    pub fn transition(mut self, duration: f32) -> Self {
        self.style.transition = Some(Transition {
            property: TransitionProperty::All,
            duration,
            delay: 0.0,
            timing: TimingFunction::Ease,
        });
        self
    }

    pub fn build(self) -> Style {
        self.style
    }
}

impl Default for StyleBuilder {
    fn default() -> Self {
        Self::new()
    }
}
