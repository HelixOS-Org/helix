//! UI Layout and Widget System
//!
//! This module provides a comprehensive UI layout system for the
//! Helix UEFI Bootloader, including widgets, layouts, and rendering.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        UI System                                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Layout Engine                                  │   │
//! │  │  FlexBox │ Grid │ Stack │ Absolute │ Flow                       │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Widgets                                        │   │
//! │  │  Text │ Button │ List │ Input │ Progress │ Image │ Container    │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Rendering                                      │   │
//! │  │  Text │ Shapes │ Images │ Effects │ Compositing                 │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// UNITS AND DIMENSIONS
// =============================================================================

/// Unit type for dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    /// Pixels
    Px(i32),
    /// Percentage of parent
    Percent(u8),
    /// Auto-calculated
    #[default]
    Auto,
    /// Flex fraction
    Fr(u8),
    /// Character width/height (for text mode)
    Ch(u8),
    /// Viewport width percentage
    Vw(u8),
    /// Viewport height percentage
    Vh(u8),
    /// Minimum of two dimensions
    Min,
    /// Maximum of two dimensions
    Max,
}

impl Unit {
    /// Convert to pixels given parent size
    pub fn to_pixels(&self, parent_size: i32, viewport_size: i32, char_size: i32) -> i32 {
        match self {
            Unit::Px(px) => *px,
            Unit::Percent(p) => (parent_size * (*p as i32)) / 100,
            Unit::Auto => parent_size,
            Unit::Fr(f) => (*f as i32) * char_size, // Simplified
            Unit::Ch(c) => (*c as i32) * char_size,
            Unit::Vw(p) => (viewport_size * (*p as i32)) / 100,
            Unit::Vh(p) => (viewport_size * (*p as i32)) / 100,
            Unit::Min | Unit::Max => parent_size,
        }
    }
}

/// Rectangle structure
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Create new rectangle
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create rectangle at origin
    pub const fn sized(width: i32, height: i32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// Check if point is inside
    pub const fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Check if intersects with another rectangle
    pub const fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Get right edge
    pub const fn right(&self) -> i32 {
        self.x + self.width
    }

    /// Get bottom edge
    pub const fn bottom(&self) -> i32 {
        self.y + self.height
    }

    /// Get center X
    pub const fn center_x(&self) -> i32 {
        self.x + self.width / 2
    }

    /// Get center Y
    pub const fn center_y(&self) -> i32 {
        self.y + self.height / 2
    }

    /// Inset rectangle by amount
    pub const fn inset(&self, amount: i32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: self.width - amount * 2,
            height: self.height - amount * 2,
        }
    }
}

/// Point structure
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Size structure
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

/// Padding/Margin structure
#[derive(Debug, Clone, Copy, Default)]
pub struct Spacing {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Spacing {
    /// Uniform spacing
    pub const fn all(value: i32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Horizontal and vertical
    pub const fn symmetric(horizontal: i32, vertical: i32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Horizontal total
    pub const fn horizontal(&self) -> i32 {
        self.left + self.right
    }

    /// Vertical total
    pub const fn vertical(&self) -> i32 {
        self.top + self.bottom
    }
}

// =============================================================================
// LAYOUT TYPES
// =============================================================================

/// Layout direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Horizontal (row)
    Horizontal,
    /// Vertical (column)
    #[default]
    Vertical,
}

/// Alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Start (left/top)
    #[default]
    Start,
    /// Center
    Center,
    /// End (right/bottom)
    End,
    /// Stretch to fill
    Stretch,
    /// Space between items
    SpaceBetween,
    /// Space around items
    SpaceAround,
    /// Space evenly
    SpaceEvenly,
}

/// Content justify
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    /// Start
    #[default]
    Start,
    /// Center
    Center,
    /// End
    End,
    /// Space between
    SpaceBetween,
    /// Space around
    SpaceAround,
    /// Space evenly
    SpaceEvenly,
}

/// Overflow behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// Visible (no clipping)
    Visible,
    /// Hidden (clip)
    #[default]
    Hidden,
    /// Scroll
    Scroll,
    /// Auto (scroll if needed)
    Auto,
}

/// Position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Relative to flow
    #[default]
    Relative,
    /// Absolute positioning
    Absolute,
    /// Fixed to viewport
    Fixed,
    /// Sticky
    Sticky,
}

// =============================================================================
// LAYOUT STYLES
// =============================================================================

/// Layout style configuration
#[derive(Debug, Clone, Copy)]
pub struct LayoutStyle {
    /// Position type
    pub position: Position,
    /// Direction
    pub direction: Direction,
    /// Main axis alignment
    pub justify: Justify,
    /// Cross axis alignment
    pub align: Align,
    /// Align self (override parent)
    pub align_self: Option<Align>,
    /// Width
    pub width: Unit,
    /// Height
    pub height: Unit,
    /// Min width
    pub min_width: Unit,
    /// Max width
    pub max_width: Unit,
    /// Min height
    pub min_height: Unit,
    /// Max height
    pub max_height: Unit,
    /// Margin
    pub margin: Spacing,
    /// Padding
    pub padding: Spacing,
    /// Gap between children
    pub gap: i32,
    /// Flex grow
    pub flex_grow: u8,
    /// Flex shrink
    pub flex_shrink: u8,
    /// Flex wrap
    pub flex_wrap: bool,
    /// Overflow
    pub overflow: Overflow,
    /// Z-index (layer order)
    pub z_index: i16,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            position: Position::Relative,
            direction: Direction::Vertical,
            justify: Justify::Start,
            align: Align::Start,
            align_self: None,
            width: Unit::Auto,
            height: Unit::Auto,
            min_width: Unit::Px(0),
            max_width: Unit::Auto,
            min_height: Unit::Px(0),
            max_height: Unit::Auto,
            margin: Spacing::default(),
            padding: Spacing::default(),
            gap: 0,
            flex_grow: 0,
            flex_shrink: 1,
            flex_wrap: false,
            overflow: Overflow::Hidden,
            z_index: 0,
        }
    }
}

// =============================================================================
// WIDGET TYPES
// =============================================================================

/// Widget type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetType {
    /// Container/group
    Container,
    /// Text label
    Text,
    /// Button
    Button,
    /// List/menu
    List,
    /// List item
    ListItem,
    /// Text input
    Input,
    /// Progress bar
    Progress,
    /// Image
    Image,
    /// Icon
    Icon,
    /// Separator
    Separator,
    /// Spacer
    Spacer,
    /// Checkbox
    Checkbox,
    /// Radio button
    Radio,
    /// Scroll view
    ScrollView,
    /// Custom
    Custom,
}

impl Default for WidgetType {
    fn default() -> Self {
        WidgetType::Container
    }
}

impl fmt::Display for WidgetType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WidgetType::Container => write!(f, "Container"),
            WidgetType::Text => write!(f, "Text"),
            WidgetType::Button => write!(f, "Button"),
            WidgetType::List => write!(f, "List"),
            WidgetType::ListItem => write!(f, "ListItem"),
            WidgetType::Input => write!(f, "Input"),
            WidgetType::Progress => write!(f, "Progress"),
            WidgetType::Image => write!(f, "Image"),
            WidgetType::Icon => write!(f, "Icon"),
            WidgetType::Separator => write!(f, "Separator"),
            WidgetType::Spacer => write!(f, "Spacer"),
            WidgetType::Checkbox => write!(f, "Checkbox"),
            WidgetType::Radio => write!(f, "Radio"),
            WidgetType::ScrollView => write!(f, "ScrollView"),
            WidgetType::Custom => write!(f, "Custom"),
        }
    }
}

/// Widget state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetState {
    /// Normal state
    Normal,
    /// Hovered
    Hovered,
    /// Focused
    Focused,
    /// Pressed/active
    Pressed,
    /// Disabled
    Disabled,
    /// Selected
    Selected,
}

impl Default for WidgetState {
    fn default() -> Self {
        WidgetState::Normal
    }
}

/// Widget ID type
pub type WidgetId = u16;

/// Widget flags
#[derive(Debug, Clone, Copy, Default)]
pub struct WidgetFlags {
    bits: u16,
}

impl WidgetFlags {
    /// Widget is visible
    pub const VISIBLE: u16 = 1 << 0;
    /// Widget is enabled
    pub const ENABLED: u16 = 1 << 1;
    /// Widget is focusable
    pub const FOCUSABLE: u16 = 1 << 2;
    /// Widget is clickable
    pub const CLICKABLE: u16 = 1 << 3;
    /// Widget needs redraw
    pub const DIRTY: u16 = 1 << 4;
    /// Widget layout is dirty
    pub const LAYOUT_DIRTY: u16 = 1 << 5;
    /// Widget has keyboard focus
    pub const HAS_FOCUS: u16 = 1 << 6;
    /// Widget is being pressed
    pub const PRESSED: u16 = 1 << 7;

    /// Default flags (visible and enabled)
    pub const fn default_enabled() -> Self {
        Self {
            bits: Self::VISIBLE | Self::ENABLED,
        }
    }

    /// Create new
    pub const fn new(bits: u16) -> Self {
        Self { bits }
    }

    /// Check flag
    pub const fn contains(&self, flag: u16) -> bool {
        (self.bits & flag) != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: u16) {
        self.bits |= flag;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: u16) {
        self.bits &= !flag;
    }
}

// =============================================================================
// TEXT WIDGET
// =============================================================================

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl Default for TextAlign {
    fn default() -> Self {
        TextAlign::Left
    }
}

/// Text overflow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflow {
    /// Clip text
    Clip,
    /// Show ellipsis (...)
    Ellipsis,
    /// Wrap to next line
    Wrap,
    /// Word wrap
    WordWrap,
}

impl Default for TextOverflow {
    fn default() -> Self {
        TextOverflow::Clip
    }
}

/// Text widget properties
#[derive(Debug, Clone, Copy)]
pub struct TextProps {
    /// Text alignment
    pub align: TextAlign,
    /// Overflow behavior
    pub overflow: TextOverflow,
    /// Font resource ID
    pub font_id: u32,
    /// Font size (0 = use default)
    pub font_size: u8,
    /// Text color
    pub color: u32,
    /// Line height (0 = use font default)
    pub line_height: u8,
    /// Letter spacing
    pub letter_spacing: i8,
    /// Maximum lines (0 = unlimited)
    pub max_lines: u8,
    /// Is bold
    pub bold: bool,
    /// Is italic
    pub italic: bool,
    /// Is underlined
    pub underline: bool,
    /// Is strikethrough
    pub strikethrough: bool,
}

impl Default for TextProps {
    fn default() -> Self {
        Self {
            align: TextAlign::Left,
            overflow: TextOverflow::Clip,
            font_id: 0,
            font_size: 0,
            color: 0xFFFF_FFFF, // White
            line_height: 0,
            letter_spacing: 0,
            max_lines: 0,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

// =============================================================================
// BUTTON WIDGET
// =============================================================================

/// Button variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Primary action
    Primary,
    /// Secondary action
    Secondary,
    /// Outline/ghost
    Outline,
    /// Text only
    Text,
    /// Danger/destructive
    Danger,
    /// Success
    Success,
}

impl Default for ButtonVariant {
    fn default() -> Self {
        ButtonVariant::Primary
    }
}

/// Button size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Normal,
    Large,
}

impl Default for ButtonSize {
    fn default() -> Self {
        ButtonSize::Normal
    }
}

/// Button widget properties
#[derive(Debug, Clone, Copy)]
pub struct ButtonProps {
    /// Variant
    pub variant: ButtonVariant,
    /// Size
    pub size: ButtonSize,
    /// Icon ID (0 = no icon)
    pub icon_id: u32,
    /// Icon position (true = left, false = right)
    pub icon_left: bool,
    /// Has full width
    pub full_width: bool,
    /// Is loading/busy
    pub loading: bool,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Normal,
            icon_id: 0,
            icon_left: true,
            full_width: false,
            loading: false,
        }
    }
}

// =============================================================================
// LIST WIDGET
// =============================================================================

/// List selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// No selection
    None,
    /// Single selection
    Single,
    /// Multiple selection
    Multiple,
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::Single
    }
}

/// List widget properties
#[derive(Debug, Clone, Copy)]
pub struct ListProps {
    /// Selection mode
    pub selection_mode: SelectionMode,
    /// Item height (0 = auto)
    pub item_height: u16,
    /// Dividers between items
    pub show_dividers: bool,
    /// Alternating row colors
    pub striped: bool,
    /// Show selection highlight
    pub show_highlight: bool,
    /// Wrap selection at edges
    pub wrap_selection: bool,
    /// Max visible items (0 = unlimited)
    pub max_visible: u8,
}

impl Default for ListProps {
    fn default() -> Self {
        Self {
            selection_mode: SelectionMode::Single,
            item_height: 0,
            show_dividers: false,
            striped: false,
            show_highlight: true,
            wrap_selection: true,
            max_visible: 0,
        }
    }
}

// =============================================================================
// PROGRESS WIDGET
// =============================================================================

/// Progress variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressVariant {
    /// Horizontal bar
    Bar,
    /// Circular
    Circular,
    /// Steps/dots
    Steps,
}

impl Default for ProgressVariant {
    fn default() -> Self {
        ProgressVariant::Bar
    }
}

/// Progress widget properties
#[derive(Debug, Clone, Copy)]
pub struct ProgressProps {
    /// Variant
    pub variant: ProgressVariant,
    /// Current value (0-100 or step number)
    pub value: u8,
    /// Total steps (for Steps variant)
    pub total_steps: u8,
    /// Show percentage text
    pub show_text: bool,
    /// Indeterminate mode
    pub indeterminate: bool,
    /// Animation enabled
    pub animated: bool,
    /// Bar height
    pub height: u8,
}

impl Default for ProgressProps {
    fn default() -> Self {
        Self {
            variant: ProgressVariant::Bar,
            value: 0,
            total_steps: 5,
            show_text: false,
            indeterminate: false,
            animated: true,
            height: 4,
        }
    }
}

// =============================================================================
// INPUT WIDGET
// =============================================================================

/// Input type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Single line text
    Text,
    /// Password (masked)
    Password,
    /// Number only
    Number,
    /// Email
    Email,
    /// Search
    Search,
}

impl Default for InputType {
    fn default() -> Self {
        InputType::Text
    }
}

/// Input widget properties
#[derive(Debug, Clone, Copy)]
pub struct InputProps {
    /// Input type
    pub input_type: InputType,
    /// Maximum length
    pub max_length: u16,
    /// Placeholder text (offset in string table)
    pub placeholder_id: u16,
    /// Is read-only
    pub readonly: bool,
    /// Show clear button
    pub clearable: bool,
    /// Mask character for password (0 = default)
    pub mask_char: u8,
}

impl Default for InputProps {
    fn default() -> Self {
        Self {
            input_type: InputType::Text,
            max_length: 256,
            placeholder_id: 0,
            readonly: false,
            clearable: false,
            mask_char: 0,
        }
    }
}

// =============================================================================
// WIDGET STRUCTURE
// =============================================================================

/// Maximum text length in widget
pub const MAX_WIDGET_TEXT: usize = 128;

/// Widget structure
#[derive(Debug, Clone)]
pub struct Widget {
    /// Widget ID
    pub id: WidgetId,
    /// Widget type
    pub widget_type: WidgetType,
    /// State
    pub state: WidgetState,
    /// Flags
    pub flags: WidgetFlags,
    /// Layout style
    pub layout: LayoutStyle,
    /// Computed bounds
    pub bounds: Rect,
    /// Content bounds (without padding)
    pub content_bounds: Rect,
    /// Text content
    pub text: [u8; MAX_WIDGET_TEXT],
    /// Text length
    pub text_len: usize,
    /// Parent widget ID (0 = root)
    pub parent: WidgetId,
    /// First child ID (0 = none)
    pub first_child: WidgetId,
    /// Next sibling ID (0 = none)
    pub next_sibling: WidgetId,
    /// Tab index for focus order
    pub tab_index: i16,
    /// Custom data
    pub data: u32,
}

impl Default for Widget {
    fn default() -> Self {
        Self {
            id: 0,
            widget_type: WidgetType::Container,
            state: WidgetState::Normal,
            flags: WidgetFlags::default_enabled(),
            layout: LayoutStyle::default(),
            bounds: Rect::default(),
            content_bounds: Rect::default(),
            text: [0u8; MAX_WIDGET_TEXT],
            text_len: 0,
            parent: 0,
            first_child: 0,
            next_sibling: 0,
            tab_index: 0,
            data: 0,
        }
    }
}

impl Widget {
    /// Create new widget
    pub const fn new(id: WidgetId, widget_type: WidgetType) -> Self {
        Self {
            id,
            widget_type,
            state: WidgetState::Normal,
            flags: WidgetFlags::new(WidgetFlags::VISIBLE | WidgetFlags::ENABLED),
            layout: LayoutStyle {
                position: Position::Relative,
                direction: Direction::Vertical,
                justify: Justify::Start,
                align: Align::Start,
                align_self: None,
                width: Unit::Auto,
                height: Unit::Auto,
                min_width: Unit::Px(0),
                max_width: Unit::Auto,
                min_height: Unit::Px(0),
                max_height: Unit::Auto,
                margin: Spacing {
                    top: 0,
                    right: 0,
                    bottom: 0,
                    left: 0,
                },
                padding: Spacing {
                    top: 0,
                    right: 0,
                    bottom: 0,
                    left: 0,
                },
                gap: 0,
                flex_grow: 0,
                flex_shrink: 1,
                flex_wrap: false,
                overflow: Overflow::Hidden,
                z_index: 0,
            },
            bounds: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            content_bounds: Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            },
            text: [0u8; MAX_WIDGET_TEXT],
            text_len: 0,
            parent: 0,
            first_child: 0,
            next_sibling: 0,
            tab_index: 0,
            data: 0,
        }
    }

    /// Check if visible
    pub const fn is_visible(&self) -> bool {
        self.flags.contains(WidgetFlags::VISIBLE)
    }

    /// Check if enabled
    pub const fn is_enabled(&self) -> bool {
        self.flags.contains(WidgetFlags::ENABLED)
    }

    /// Check if focusable
    pub const fn is_focusable(&self) -> bool {
        self.flags.contains(WidgetFlags::FOCUSABLE) && self.is_enabled()
    }

    /// Get text as string
    pub fn text_str(&self) -> &str {
        if self.text_len > 0 {
            core::str::from_utf8(&self.text[..self.text_len]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Set text
    pub fn set_text(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let len = bytes.len().min(MAX_WIDGET_TEXT);
        self.text[..len].copy_from_slice(&bytes[..len]);
        self.text_len = len;
        self.flags.set(WidgetFlags::DIRTY);
    }
}

// =============================================================================
// SCREEN LAYOUT
// =============================================================================

/// Screen area
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenArea {
    /// Header/title bar
    Header,
    /// Main content area
    Content,
    /// Footer/status bar
    Footer,
    /// Left sidebar
    LeftSidebar,
    /// Right sidebar
    RightSidebar,
    /// Overlay/modal
    Overlay,
    /// Full screen
    FullScreen,
}

impl Default for ScreenArea {
    fn default() -> Self {
        ScreenArea::Content
    }
}

/// Screen layout configuration
#[derive(Debug, Clone, Copy)]
pub struct ScreenLayout {
    /// Viewport width
    pub width: u32,
    /// Viewport height
    pub height: u32,
    /// Header height
    pub header_height: u16,
    /// Footer height
    pub footer_height: u16,
    /// Left sidebar width
    pub left_sidebar_width: u16,
    /// Right sidebar width
    pub right_sidebar_width: u16,
    /// Content padding
    pub content_padding: Spacing,
    /// Has header
    pub has_header: bool,
    /// Has footer
    pub has_footer: bool,
    /// Has left sidebar
    pub has_left_sidebar: bool,
    /// Has right sidebar
    pub has_right_sidebar: bool,
}

impl Default for ScreenLayout {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 768,
            header_height: 48,
            footer_height: 32,
            left_sidebar_width: 0,
            right_sidebar_width: 0,
            content_padding: Spacing::all(16),
            has_header: true,
            has_footer: true,
            has_left_sidebar: false,
            has_right_sidebar: false,
        }
    }
}

impl ScreenLayout {
    /// Get area bounds
    pub fn area_bounds(&self, area: ScreenArea) -> Rect {
        let header_h = if self.has_header {
            self.header_height as i32
        } else {
            0
        };
        let footer_h = if self.has_footer {
            self.footer_height as i32
        } else {
            0
        };
        let left_w = if self.has_left_sidebar {
            self.left_sidebar_width as i32
        } else {
            0
        };
        let right_w = if self.has_right_sidebar {
            self.right_sidebar_width as i32
        } else {
            0
        };

        match area {
            ScreenArea::Header => Rect::new(0, 0, self.width as i32, header_h),
            ScreenArea::Footer => Rect::new(
                0,
                self.height as i32 - footer_h,
                self.width as i32,
                footer_h,
            ),
            ScreenArea::LeftSidebar => Rect::new(
                0,
                header_h,
                left_w,
                self.height as i32 - header_h - footer_h,
            ),
            ScreenArea::RightSidebar => Rect::new(
                self.width as i32 - right_w,
                header_h,
                right_w,
                self.height as i32 - header_h - footer_h,
            ),
            ScreenArea::Content => Rect::new(
                left_w,
                header_h,
                self.width as i32 - left_w - right_w,
                self.height as i32 - header_h - footer_h,
            ),
            ScreenArea::Overlay | ScreenArea::FullScreen => {
                Rect::new(0, 0, self.width as i32, self.height as i32)
            },
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_rect() {
        let rect = Rect::new(10, 20, 100, 50);
        assert!(rect.contains(50, 40));
        assert!(!rect.contains(5, 40));
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 70);
    }

    #[test]
    fn test_spacing() {
        let spacing = Spacing::symmetric(10, 20);
        assert_eq!(spacing.horizontal(), 20);
        assert_eq!(spacing.vertical(), 40);
    }

    #[test]
    fn test_widget_flags() {
        let mut flags = WidgetFlags::default_enabled();
        assert!(flags.contains(WidgetFlags::VISIBLE));
        assert!(flags.contains(WidgetFlags::ENABLED));

        flags.set(WidgetFlags::FOCUSABLE);
        assert!(flags.contains(WidgetFlags::FOCUSABLE));

        flags.clear(WidgetFlags::ENABLED);
        assert!(!flags.contains(WidgetFlags::ENABLED));
    }

    #[test]
    fn test_widget() {
        let mut widget = Widget::new(1, WidgetType::Button);
        widget.set_text("Click Me");
        assert_eq!(widget.text_str(), "Click Me");
        assert!(widget.is_visible());
        assert!(widget.is_enabled());
    }

    #[test]
    fn test_screen_layout() {
        let layout = ScreenLayout::default();
        let content = layout.area_bounds(ScreenArea::Content);
        assert!(content.width > 0);
        assert!(content.height > 0);
    }
}
