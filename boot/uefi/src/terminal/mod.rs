//! Terminal Emulation and Console Protocol for Helix UEFI Bootloader
//!
//! This module provides comprehensive terminal emulation support with
//! ANSI/VT100 escape sequences, Unicode rendering, and advanced console features.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Terminal Subsystem                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Presentation Layer                            │   │
//! │  │  Themes │ Colors │ Fonts │ Glyphs │ Box Drawing                 │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Terminal Emulation                            │   │
//! │  │  VT100 │ VT220 │ ANSI │ xterm │ Linux Console                   │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Console Management                            │   │
//! │  │  Screen Buffer │ Scrollback │ Selection │ History               │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Output Backend                                │   │
//! │  │  Simple Text │ GOP Framebuffer │ Serial │ Debug Port            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]


// =============================================================================
// ANSI ESCAPE SEQUENCES
// =============================================================================

/// ANSI escape sequence start
pub const ESC: u8 = 0x1B;
/// Control Sequence Introducer
pub const CSI: &[u8] = &[0x1B, b'['];
/// Operating System Command
pub const OSC: &[u8] = &[0x1B, b']'];

/// Standard ANSI colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AnsiColor {
    /// Black
    Black = 0,
    /// Red
    Red = 1,
    /// Green
    Green = 2,
    /// Yellow
    Yellow = 3,
    /// Blue
    Blue = 4,
    /// Magenta
    Magenta = 5,
    /// Cyan
    Cyan = 6,
    /// White
    White = 7,
    /// Bright black (gray)
    BrightBlack = 8,
    /// Bright red
    BrightRed = 9,
    /// Bright green
    BrightGreen = 10,
    /// Bright yellow
    BrightYellow = 11,
    /// Bright blue
    BrightBlue = 12,
    /// Bright magenta
    BrightMagenta = 13,
    /// Bright cyan
    BrightCyan = 14,
    /// Bright white
    BrightWhite = 15,
}

impl AnsiColor {
    /// Get RGB values for standard color
    pub const fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            AnsiColor::Black => (0, 0, 0),
            AnsiColor::Red => (170, 0, 0),
            AnsiColor::Green => (0, 170, 0),
            AnsiColor::Yellow => (170, 85, 0),
            AnsiColor::Blue => (0, 0, 170),
            AnsiColor::Magenta => (170, 0, 170),
            AnsiColor::Cyan => (0, 170, 170),
            AnsiColor::White => (170, 170, 170),
            AnsiColor::BrightBlack => (85, 85, 85),
            AnsiColor::BrightRed => (255, 85, 85),
            AnsiColor::BrightGreen => (85, 255, 85),
            AnsiColor::BrightYellow => (255, 255, 85),
            AnsiColor::BrightBlue => (85, 85, 255),
            AnsiColor::BrightMagenta => (255, 85, 255),
            AnsiColor::BrightCyan => (85, 255, 255),
            AnsiColor::BrightWhite => (255, 255, 255),
        }
    }

    /// Get foreground SGR code
    pub const fn fg_code(&self) -> u8 {
        match *self as u8 {
            0..=7 => 30 + *self as u8,
            8..=15 => 90 + (*self as u8 - 8),
            _ => 37,
        }
    }

    /// Get background SGR code
    pub const fn bg_code(&self) -> u8 {
        match *self as u8 {
            0..=7 => 40 + *self as u8,
            8..=15 => 100 + (*self as u8 - 8),
            _ => 47,
        }
    }
}

/// Extended color (256-color palette)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color256(pub u8);

impl Color256 {
    /// Create from 6x6x6 color cube (0-5 for each component)
    pub const fn from_cube(r: u8, g: u8, b: u8) -> Self {
        Self(16 + 36 * r + 6 * g + b)
    }

    /// Create from grayscale (0-23)
    pub const fn from_gray(level: u8) -> Self {
        Self(232 + level)
    }

    /// Convert to RGB
    pub const fn to_rgb(&self) -> (u8, u8, u8) {
        let idx = self.0;
        if idx < 16 {
            // Standard colors
            match idx {
                0 => (0, 0, 0),
                1 => (128, 0, 0),
                2 => (0, 128, 0),
                3 => (128, 128, 0),
                4 => (0, 0, 128),
                5 => (128, 0, 128),
                6 => (0, 128, 128),
                7 => (192, 192, 192),
                8 => (128, 128, 128),
                9 => (255, 0, 0),
                10 => (0, 255, 0),
                11 => (255, 255, 0),
                12 => (0, 0, 255),
                13 => (255, 0, 255),
                14 => (0, 255, 255),
                15 => (255, 255, 255),
                _ => (0, 0, 0),
            }
        } else if idx < 232 {
            // 6x6x6 color cube
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            // Convert 0-5 to color value: 0->0, 1-5 -> 55 + 40*v
            let r_val = if r == 0 { 0 } else { 55 + 40 * r };
            let g_val = if g == 0 { 0 } else { 55 + 40 * g };
            let b_val = if b == 0 { 0 } else { 55 + 40 * b };
            (r_val, g_val, b_val)
        } else {
            // Grayscale
            let gray = 8 + 10 * (idx - 232);
            (gray, gray, gray)
        }
    }
}

/// True color (24-bit RGB)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrueColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl TrueColor {
    /// Create new color
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Black
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// White
    pub const WHITE: Self = Self::new(255, 255, 255);
    /// Default background
    pub const DEFAULT_BG: Self = Self::new(0, 0, 0);
    /// Default foreground
    pub const DEFAULT_FG: Self = Self::new(192, 192, 192);
}

/// Color specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermColor {
    /// Default color
    Default,
    /// Standard ANSI color
    Ansi(AnsiColor),
    /// 256-color palette
    Palette(Color256),
    /// True color (24-bit)
    Rgb(TrueColor),
}

impl Default for TermColor {
    fn default() -> Self {
        TermColor::Default
    }
}

// =============================================================================
// TEXT ATTRIBUTES
// =============================================================================

/// Text attributes (SGR - Select Graphic Rendition)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAttributes {
    /// Foreground color
    pub fg: TermColor,
    /// Background color
    pub bg: TermColor,
    /// Bold/bright
    pub bold: bool,
    /// Dim/faint
    pub dim: bool,
    /// Italic
    pub italic: bool,
    /// Underline
    pub underline: bool,
    /// Slow blink
    pub blink: bool,
    /// Rapid blink
    pub rapid_blink: bool,
    /// Reverse video
    pub reverse: bool,
    /// Hidden/invisible
    pub hidden: bool,
    /// Strikethrough
    pub strikethrough: bool,
    /// Double underline
    pub double_underline: bool,
    /// Overline
    pub overline: bool,
}

impl TextAttributes {
    /// Default attributes
    pub const DEFAULT: Self = Self {
        fg: TermColor::Default,
        bg: TermColor::Default,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        blink: false,
        rapid_blink: false,
        reverse: false,
        hidden: false,
        strikethrough: false,
        double_underline: false,
        overline: false,
    };

    /// Reset all attributes
    pub fn reset(&mut self) {
        *self = Self::DEFAULT;
    }
}

impl Default for TextAttributes {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// =============================================================================
// TERMINAL CELL
// =============================================================================

/// A single character cell in the terminal
#[derive(Debug, Clone, Copy)]
pub struct Cell {
    /// Unicode character (UTF-32)
    pub ch: char,
    /// Text attributes
    pub attr: TextAttributes,
    /// Character width (1 for normal, 2 for wide)
    pub width: u8,
}

impl Cell {
    /// Empty cell
    pub const EMPTY: Self = Self {
        ch: ' ',
        attr: TextAttributes::DEFAULT,
        width: 1,
    };

    /// Create new cell with character
    pub const fn new(ch: char) -> Self {
        Self {
            ch,
            attr: TextAttributes::DEFAULT,
            width: 1,
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::EMPTY
    }
}

// =============================================================================
// CURSOR
// =============================================================================

/// Cursor position
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorPos {
    /// Column (0-based)
    pub col: u16,
    /// Row (0-based)
    pub row: u16,
}

impl CursorPos {
    /// Origin (0, 0)
    pub const ORIGIN: Self = Self { col: 0, row: 0 };

    /// Create new position
    pub const fn new(col: u16, row: u16) -> Self {
        Self { col, row }
    }
}

/// Cursor style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Block cursor
    Block,
    /// Underline cursor
    Underline,
    /// Vertical bar
    Bar,
}

impl Default for CursorStyle {
    fn default() -> Self {
        CursorStyle::Block
    }
}

/// Cursor state
#[derive(Debug, Clone, Copy)]
pub struct CursorState {
    /// Position
    pub pos: CursorPos,
    /// Style
    pub style: CursorStyle,
    /// Visible
    pub visible: bool,
    /// Blinking
    pub blinking: bool,
    /// Blink rate in milliseconds
    pub blink_rate: u16,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            pos: CursorPos::ORIGIN,
            style: CursorStyle::Block,
            visible: true,
            blinking: true,
            blink_rate: 500,
        }
    }
}

// =============================================================================
// TERMINAL MODES
// =============================================================================

/// Terminal emulation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMode {
    /// VT100 compatible
    Vt100,
    /// VT220 compatible
    Vt220,
    /// ANSI compatible
    Ansi,
    /// xterm compatible
    Xterm,
    /// Linux console
    Linux,
    /// Basic text mode
    Basic,
}

impl Default for TerminalMode {
    fn default() -> Self {
        TerminalMode::Xterm
    }
}

/// DEC private modes
pub mod dec_modes {
    pub const CURSOR_KEYS: u16 = 1;       // Application cursor keys
    pub const VT52_MODE: u16 = 2;         // DECANM
    pub const COLUMN_132: u16 = 3;        // 132 column mode
    pub const SMOOTH_SCROLL: u16 = 4;     // Smooth scrolling
    pub const REVERSE_VIDEO: u16 = 5;     // Reverse video
    pub const ORIGIN_MODE: u16 = 6;       // Origin mode
    pub const AUTO_WRAP: u16 = 7;         // Auto-wrap mode
    pub const AUTO_REPEAT: u16 = 8;       // Auto-repeat keys
    pub const MOUSE_X10: u16 = 9;         // X10 mouse reporting
    pub const CURSOR_VISIBLE: u16 = 25;   // Show cursor
    pub const MOUSE_VT200: u16 = 1000;    // VT200 mouse
    pub const MOUSE_HILITE: u16 = 1001;   // Highlight mouse
    pub const MOUSE_CELL: u16 = 1002;     // Cell motion mouse
    pub const MOUSE_ALL: u16 = 1003;      // All motion mouse
    pub const MOUSE_UTF8: u16 = 1005;     // UTF-8 mouse
    pub const MOUSE_SGR: u16 = 1006;      // SGR mouse
    pub const ALT_SCREEN: u16 = 1049;     // Alternate screen
    pub const BRACKETED_PASTE: u16 = 2004; // Bracketed paste
}

/// Terminal mode flags
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalModes {
    /// Application cursor keys
    pub app_cursor: bool,
    /// Application keypad
    pub app_keypad: bool,
    /// Auto-wrap mode
    pub auto_wrap: bool,
    /// Origin mode
    pub origin_mode: bool,
    /// Insert mode
    pub insert_mode: bool,
    /// Line feed mode
    pub line_feed_mode: bool,
    /// Local echo
    pub local_echo: bool,
    /// Reverse wrap
    pub reverse_wrap: bool,
    /// Alternate screen buffer
    pub alt_screen: bool,
    /// Bracketed paste mode
    pub bracketed_paste: bool,
    /// Report focus events
    pub focus_events: bool,
    /// Mouse tracking enabled
    pub mouse_tracking: bool,
    /// Save/restore cursor
    pub save_cursor: bool,
}

// =============================================================================
// SCROLL REGION
// =============================================================================

/// Scroll region definition
#[derive(Debug, Clone, Copy)]
pub struct ScrollRegion {
    /// Top line (0-based, inclusive)
    pub top: u16,
    /// Bottom line (0-based, inclusive)
    pub bottom: u16,
}

impl ScrollRegion {
    /// Create new scroll region
    pub const fn new(top: u16, bottom: u16) -> Self {
        Self { top, bottom }
    }

    /// Full screen region
    pub const fn full_screen(height: u16) -> Self {
        Self { top: 0, bottom: height.saturating_sub(1) }
    }
}

// =============================================================================
// ESCAPE SEQUENCE PARSER
// =============================================================================

/// Parser state for escape sequences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    /// Ground state - normal character processing
    Ground,
    /// Escape received
    Escape,
    /// CSI sequence
    CsiEntry,
    /// CSI parameter
    CsiParam,
    /// CSI intermediate
    CsiIntermediate,
    /// OSC sequence
    OscString,
    /// DCS sequence
    DcsEntry,
    /// DCS parameter
    DcsParam,
    /// DCS passthrough
    DcsPassthrough,
    /// APC sequence
    ApcString,
    /// PM sequence
    PmString,
}

impl Default for ParserState {
    fn default() -> Self {
        ParserState::Ground
    }
}

/// Maximum number of CSI parameters
pub const MAX_CSI_PARAMS: usize = 16;

/// Maximum parameter string length
pub const MAX_PARAM_STRING: usize = 256;

/// CSI command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsiCommand {
    /// Cursor Up
    CursorUp(u16),
    /// Cursor Down
    CursorDown(u16),
    /// Cursor Forward
    CursorForward(u16),
    /// Cursor Backward
    CursorBack(u16),
    /// Cursor Position
    CursorPosition(u16, u16),
    /// Erase in Display
    EraseDisplay(u8),
    /// Erase in Line
    EraseLine(u8),
    /// Scroll Up
    ScrollUp(u16),
    /// Scroll Down
    ScrollDown(u16),
    /// Insert Lines
    InsertLines(u16),
    /// Delete Lines
    DeleteLines(u16),
    /// Insert Characters
    InsertChars(u16),
    /// Delete Characters
    DeleteChars(u16),
    /// Set Graphics Rendition
    Sgr,
    /// Set Mode
    SetMode(u16),
    /// Reset Mode
    ResetMode(u16),
    /// Device Attributes Request
    DeviceAttributes,
    /// Set Scroll Region
    SetScrollRegion(u16, u16),
    /// Save Cursor
    SaveCursor,
    /// Restore Cursor
    RestoreCursor,
    /// Soft Reset
    SoftReset,
    /// Unknown
    Unknown,
}

// =============================================================================
// SGR ATTRIBUTES
// =============================================================================

/// SGR (Select Graphic Rendition) parameter codes
pub mod sgr {
    pub const RESET: u8 = 0;
    pub const BOLD: u8 = 1;
    pub const DIM: u8 = 2;
    pub const ITALIC: u8 = 3;
    pub const UNDERLINE: u8 = 4;
    pub const SLOW_BLINK: u8 = 5;
    pub const RAPID_BLINK: u8 = 6;
    pub const REVERSE: u8 = 7;
    pub const HIDDEN: u8 = 8;
    pub const STRIKETHROUGH: u8 = 9;

    pub const DEFAULT_FONT: u8 = 10;

    pub const DOUBLE_UNDERLINE: u8 = 21;
    pub const NORMAL_INTENSITY: u8 = 22;
    pub const NOT_ITALIC: u8 = 23;
    pub const NOT_UNDERLINED: u8 = 24;
    pub const NOT_BLINKING: u8 = 25;
    pub const NOT_REVERSED: u8 = 27;
    pub const REVEAL: u8 = 28;
    pub const NOT_STRIKETHROUGH: u8 = 29;

    pub const FG_BLACK: u8 = 30;
    pub const FG_RED: u8 = 31;
    pub const FG_GREEN: u8 = 32;
    pub const FG_YELLOW: u8 = 33;
    pub const FG_BLUE: u8 = 34;
    pub const FG_MAGENTA: u8 = 35;
    pub const FG_CYAN: u8 = 36;
    pub const FG_WHITE: u8 = 37;
    pub const FG_EXTENDED: u8 = 38;
    pub const FG_DEFAULT: u8 = 39;

    pub const BG_BLACK: u8 = 40;
    pub const BG_RED: u8 = 41;
    pub const BG_GREEN: u8 = 42;
    pub const BG_YELLOW: u8 = 43;
    pub const BG_BLUE: u8 = 44;
    pub const BG_MAGENTA: u8 = 45;
    pub const BG_CYAN: u8 = 46;
    pub const BG_WHITE: u8 = 47;
    pub const BG_EXTENDED: u8 = 48;
    pub const BG_DEFAULT: u8 = 49;

    pub const OVERLINE: u8 = 53;
    pub const NOT_OVERLINE: u8 = 55;

    pub const FG_BRIGHT_BLACK: u8 = 90;
    pub const FG_BRIGHT_RED: u8 = 91;
    pub const FG_BRIGHT_GREEN: u8 = 92;
    pub const FG_BRIGHT_YELLOW: u8 = 93;
    pub const FG_BRIGHT_BLUE: u8 = 94;
    pub const FG_BRIGHT_MAGENTA: u8 = 95;
    pub const FG_BRIGHT_CYAN: u8 = 96;
    pub const FG_BRIGHT_WHITE: u8 = 97;

    pub const BG_BRIGHT_BLACK: u8 = 100;
    pub const BG_BRIGHT_RED: u8 = 101;
    pub const BG_BRIGHT_GREEN: u8 = 102;
    pub const BG_BRIGHT_YELLOW: u8 = 103;
    pub const BG_BRIGHT_BLUE: u8 = 104;
    pub const BG_BRIGHT_MAGENTA: u8 = 105;
    pub const BG_BRIGHT_CYAN: u8 = 106;
    pub const BG_BRIGHT_WHITE: u8 = 107;
}

// =============================================================================
// BOX DRAWING CHARACTERS
// =============================================================================

/// Box drawing characters
pub mod box_chars {
    // Single lines
    pub const HORIZONTAL: char = '─';
    pub const VERTICAL: char = '│';
    pub const TOP_LEFT: char = '┌';
    pub const TOP_RIGHT: char = '┐';
    pub const BOTTOM_LEFT: char = '└';
    pub const BOTTOM_RIGHT: char = '┘';
    pub const CROSS: char = '┼';
    pub const T_DOWN: char = '┬';
    pub const T_UP: char = '┴';
    pub const T_RIGHT: char = '├';
    pub const T_LEFT: char = '┤';

    // Double lines
    pub const DOUBLE_HORIZONTAL: char = '═';
    pub const DOUBLE_VERTICAL: char = '║';
    pub const DOUBLE_TOP_LEFT: char = '╔';
    pub const DOUBLE_TOP_RIGHT: char = '╗';
    pub const DOUBLE_BOTTOM_LEFT: char = '╚';
    pub const DOUBLE_BOTTOM_RIGHT: char = '╝';
    pub const DOUBLE_CROSS: char = '╬';
    pub const DOUBLE_T_DOWN: char = '╦';
    pub const DOUBLE_T_UP: char = '╩';
    pub const DOUBLE_T_RIGHT: char = '╠';
    pub const DOUBLE_T_LEFT: char = '╣';

    // Rounded corners
    pub const ROUNDED_TOP_LEFT: char = '╭';
    pub const ROUNDED_TOP_RIGHT: char = '╮';
    pub const ROUNDED_BOTTOM_LEFT: char = '╰';
    pub const ROUNDED_BOTTOM_RIGHT: char = '╯';

    // Block elements
    pub const FULL_BLOCK: char = '█';
    pub const UPPER_HALF: char = '▀';
    pub const LOWER_HALF: char = '▄';
    pub const LEFT_HALF: char = '▌';
    pub const RIGHT_HALF: char = '▐';
    pub const LIGHT_SHADE: char = '░';
    pub const MEDIUM_SHADE: char = '▒';
    pub const DARK_SHADE: char = '▓';

    // Progress indicators
    pub const PROGRESS_EMPTY: char = '░';
    pub const PROGRESS_PARTIAL: char = '▓';
    pub const PROGRESS_FULL: char = '█';
}

// =============================================================================
// SPECIAL CHARACTERS
// =============================================================================

/// Special character mappings
pub mod special_chars {
    // Arrows
    pub const ARROW_UP: char = '↑';
    pub const ARROW_DOWN: char = '↓';
    pub const ARROW_LEFT: char = '←';
    pub const ARROW_RIGHT: char = '→';
    pub const ARROW_UP_DOWN: char = '↕';
    pub const ARROW_LEFT_RIGHT: char = '↔';

    // Symbols
    pub const CHECK: char = '✓';
    pub const CROSS: char = '✗';
    pub const BULLET: char = '•';
    pub const DIAMOND: char = '◆';
    pub const STAR: char = '★';
    pub const CIRCLE: char = '●';
    pub const EMPTY_CIRCLE: char = '○';
    pub const SQUARE: char = '■';
    pub const EMPTY_SQUARE: char = '□';

    // Greek letters (often used)
    pub const ALPHA: char = 'α';
    pub const BETA: char = 'β';
    pub const GAMMA: char = 'γ';
    pub const DELTA: char = 'δ';
    pub const PI: char = 'π';
    pub const SIGMA: char = 'σ';
    pub const OMEGA: char = 'ω';

    // Math symbols
    pub const PLUS_MINUS: char = '±';
    pub const INFINITY: char = '∞';
    pub const APPROX: char = '≈';
    pub const NOT_EQUAL: char = '≠';
    pub const LESS_EQUAL: char = '≤';
    pub const GREATER_EQUAL: char = '≥';
    pub const SQRT: char = '√';
    pub const SUM: char = '∑';
    pub const PRODUCT: char = '∏';
}

// =============================================================================
// CONSOLE OUTPUT ATTRIBUTES (EFI)
// =============================================================================

/// EFI Console colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EfiColor {
    Black = 0x00,
    Blue = 0x01,
    Green = 0x02,
    Cyan = 0x03,
    Red = 0x04,
    Magenta = 0x05,
    Brown = 0x06,
    LightGray = 0x07,
    DarkGray = 0x08,
    LightBlue = 0x09,
    LightGreen = 0x0A,
    LightCyan = 0x0B,
    LightRed = 0x0C,
    LightMagenta = 0x0D,
    Yellow = 0x0E,
    White = 0x0F,
}

impl EfiColor {
    /// Create attribute byte from foreground and background
    pub const fn make_attr(fg: EfiColor, bg: EfiColor) -> u8 {
        ((bg as u8) << 4) | (fg as u8)
    }

    /// Extract foreground from attribute
    pub const fn from_attr_fg(attr: u8) -> Self {
        // Safe because we mask to 4 bits
        match attr & 0x0F {
            0x00 => EfiColor::Black,
            0x01 => EfiColor::Blue,
            0x02 => EfiColor::Green,
            0x03 => EfiColor::Cyan,
            0x04 => EfiColor::Red,
            0x05 => EfiColor::Magenta,
            0x06 => EfiColor::Brown,
            0x07 => EfiColor::LightGray,
            0x08 => EfiColor::DarkGray,
            0x09 => EfiColor::LightBlue,
            0x0A => EfiColor::LightGreen,
            0x0B => EfiColor::LightCyan,
            0x0C => EfiColor::LightRed,
            0x0D => EfiColor::LightMagenta,
            0x0E => EfiColor::Yellow,
            _ => EfiColor::White,
        }
    }

    /// Convert to ANSI color
    pub const fn to_ansi(&self) -> AnsiColor {
        match self {
            EfiColor::Black => AnsiColor::Black,
            EfiColor::Blue => AnsiColor::Blue,
            EfiColor::Green => AnsiColor::Green,
            EfiColor::Cyan => AnsiColor::Cyan,
            EfiColor::Red => AnsiColor::Red,
            EfiColor::Magenta => AnsiColor::Magenta,
            EfiColor::Brown => AnsiColor::Yellow,
            EfiColor::LightGray => AnsiColor::White,
            EfiColor::DarkGray => AnsiColor::BrightBlack,
            EfiColor::LightBlue => AnsiColor::BrightBlue,
            EfiColor::LightGreen => AnsiColor::BrightGreen,
            EfiColor::LightCyan => AnsiColor::BrightCyan,
            EfiColor::LightRed => AnsiColor::BrightRed,
            EfiColor::LightMagenta => AnsiColor::BrightMagenta,
            EfiColor::Yellow => AnsiColor::BrightYellow,
            EfiColor::White => AnsiColor::BrightWhite,
        }
    }
}

// =============================================================================
// TERMINAL SIZE
// =============================================================================

/// Terminal dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    /// Width in columns
    pub cols: u16,
    /// Height in rows
    pub rows: u16,
}

impl TerminalSize {
    /// Standard 80x25
    pub const VGA_TEXT: Self = Self { cols: 80, rows: 25 };
    /// Standard 80x24
    pub const STANDARD: Self = Self { cols: 80, rows: 24 };
    /// Extended 132x43
    pub const EXTENDED: Self = Self { cols: 132, rows: 43 };
    /// Modern 120x40
    pub const MODERN: Self = Self { cols: 120, rows: 40 };

    /// Create new size
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Total number of cells
    pub const fn total_cells(&self) -> usize {
        (self.cols as usize) * (self.rows as usize)
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::STANDARD
    }
}

// =============================================================================
// CONSOLE MODE
// =============================================================================

/// Console mode descriptor
#[derive(Debug, Clone, Copy)]
pub struct ConsoleMode {
    /// Mode number
    pub mode: u32,
    /// Size
    pub size: TerminalSize,
    /// Attributes supported
    pub attributes: bool,
}

// =============================================================================
// THEME
// =============================================================================

/// Terminal theme
#[derive(Debug, Clone, Copy)]
pub struct TerminalTheme {
    /// Background color
    pub background: TrueColor,
    /// Foreground color
    pub foreground: TrueColor,
    /// Cursor color
    pub cursor: TrueColor,
    /// Selection background
    pub selection_bg: TrueColor,
    /// Selection foreground
    pub selection_fg: TrueColor,
    /// ANSI color palette (16 colors)
    pub palette: [TrueColor; 16],
}

impl TerminalTheme {
    /// Solarized Dark theme
    pub const SOLARIZED_DARK: Self = Self {
        background: TrueColor::new(0, 43, 54),
        foreground: TrueColor::new(131, 148, 150),
        cursor: TrueColor::new(131, 148, 150),
        selection_bg: TrueColor::new(7, 54, 66),
        selection_fg: TrueColor::new(147, 161, 161),
        palette: [
            TrueColor::new(7, 54, 66),     // Black
            TrueColor::new(220, 50, 47),   // Red
            TrueColor::new(133, 153, 0),   // Green
            TrueColor::new(181, 137, 0),   // Yellow
            TrueColor::new(38, 139, 210),  // Blue
            TrueColor::new(211, 54, 130),  // Magenta
            TrueColor::new(42, 161, 152),  // Cyan
            TrueColor::new(238, 232, 213), // White
            TrueColor::new(0, 43, 54),     // Bright black
            TrueColor::new(203, 75, 22),   // Bright red
            TrueColor::new(88, 110, 117),  // Bright green
            TrueColor::new(101, 123, 131), // Bright yellow
            TrueColor::new(131, 148, 150), // Bright blue
            TrueColor::new(108, 113, 196), // Bright magenta
            TrueColor::new(147, 161, 161), // Bright cyan
            TrueColor::new(253, 246, 227), // Bright white
        ],
    };

    /// Monokai theme
    pub const MONOKAI: Self = Self {
        background: TrueColor::new(39, 40, 34),
        foreground: TrueColor::new(248, 248, 242),
        cursor: TrueColor::new(248, 248, 242),
        selection_bg: TrueColor::new(73, 72, 62),
        selection_fg: TrueColor::new(248, 248, 242),
        palette: [
            TrueColor::new(39, 40, 34),    // Black
            TrueColor::new(249, 38, 114),  // Red
            TrueColor::new(166, 226, 46),  // Green
            TrueColor::new(244, 191, 117), // Yellow
            TrueColor::new(102, 217, 239), // Blue
            TrueColor::new(174, 129, 255), // Magenta
            TrueColor::new(161, 239, 228), // Cyan
            TrueColor::new(248, 248, 242), // White
            TrueColor::new(117, 113, 94),  // Bright black
            TrueColor::new(249, 38, 114),  // Bright red
            TrueColor::new(166, 226, 46),  // Bright green
            TrueColor::new(244, 191, 117), // Bright yellow
            TrueColor::new(102, 217, 239), // Bright blue
            TrueColor::new(174, 129, 255), // Bright magenta
            TrueColor::new(161, 239, 228), // Bright cyan
            TrueColor::new(249, 248, 245), // Bright white
        ],
    };

    /// Classic VGA theme
    pub const VGA: Self = Self {
        background: TrueColor::new(0, 0, 0),
        foreground: TrueColor::new(170, 170, 170),
        cursor: TrueColor::new(255, 255, 255),
        selection_bg: TrueColor::new(170, 170, 170),
        selection_fg: TrueColor::new(0, 0, 0),
        palette: [
            TrueColor::new(0, 0, 0),       // Black
            TrueColor::new(170, 0, 0),     // Red
            TrueColor::new(0, 170, 0),     // Green
            TrueColor::new(170, 85, 0),    // Yellow (brown)
            TrueColor::new(0, 0, 170),     // Blue
            TrueColor::new(170, 0, 170),   // Magenta
            TrueColor::new(0, 170, 170),   // Cyan
            TrueColor::new(170, 170, 170), // White
            TrueColor::new(85, 85, 85),    // Bright black
            TrueColor::new(255, 85, 85),   // Bright red
            TrueColor::new(85, 255, 85),   // Bright green
            TrueColor::new(255, 255, 85),  // Bright yellow
            TrueColor::new(85, 85, 255),   // Bright blue
            TrueColor::new(255, 85, 255),  // Bright magenta
            TrueColor::new(85, 255, 255),  // Bright cyan
            TrueColor::new(255, 255, 255), // Bright white
        ],
    };

    /// Dracula theme
    pub const DRACULA: Self = Self {
        background: TrueColor::new(40, 42, 54),
        foreground: TrueColor::new(248, 248, 242),
        cursor: TrueColor::new(248, 248, 242),
        selection_bg: TrueColor::new(68, 71, 90),
        selection_fg: TrueColor::new(248, 248, 242),
        palette: [
            TrueColor::new(33, 34, 44),    // Black
            TrueColor::new(255, 85, 85),   // Red
            TrueColor::new(80, 250, 123),  // Green
            TrueColor::new(241, 250, 140), // Yellow
            TrueColor::new(189, 147, 249), // Blue
            TrueColor::new(255, 121, 198), // Magenta
            TrueColor::new(139, 233, 253), // Cyan
            TrueColor::new(248, 248, 242), // White
            TrueColor::new(98, 114, 164),  // Bright black
            TrueColor::new(255, 110, 103), // Bright red
            TrueColor::new(90, 247, 142),  // Bright green
            TrueColor::new(244, 249, 157), // Bright yellow
            TrueColor::new(202, 169, 250), // Bright blue
            TrueColor::new(255, 146, 208), // Bright magenta
            TrueColor::new(154, 237, 254), // Bright cyan
            TrueColor::new(255, 255, 255), // Bright white
        ],
    };

    /// Helix OS theme
    pub const HELIX: Self = Self {
        background: TrueColor::new(18, 18, 28),
        foreground: TrueColor::new(220, 220, 240),
        cursor: TrueColor::new(100, 180, 255),
        selection_bg: TrueColor::new(60, 60, 100),
        selection_fg: TrueColor::new(255, 255, 255),
        palette: [
            TrueColor::new(18, 18, 28),    // Black
            TrueColor::new(255, 100, 100), // Red
            TrueColor::new(100, 255, 150), // Green
            TrueColor::new(255, 220, 100), // Yellow
            TrueColor::new(100, 180, 255), // Blue
            TrueColor::new(200, 100, 255), // Magenta
            TrueColor::new(100, 220, 220), // Cyan
            TrueColor::new(200, 200, 220), // White
            TrueColor::new(80, 80, 100),   // Bright black
            TrueColor::new(255, 150, 150), // Bright red
            TrueColor::new(150, 255, 180), // Bright green
            TrueColor::new(255, 240, 150), // Bright yellow
            TrueColor::new(150, 200, 255), // Bright blue
            TrueColor::new(220, 150, 255), // Bright magenta
            TrueColor::new(150, 240, 240), // Bright cyan
            TrueColor::new(255, 255, 255), // Bright white
        ],
    };
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self::HELIX
    }
}

// =============================================================================
// SERIAL CONSOLE
// =============================================================================

/// Serial port configuration
#[derive(Debug, Clone, Copy)]
pub struct SerialConfig {
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits
    pub data_bits: u8,
    /// Stop bits
    pub stop_bits: u8,
    /// Parity
    pub parity: SerialParity,
    /// Flow control
    pub flow_control: FlowControl,
}

/// Serial parity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialParity {
    None,
    Odd,
    Even,
    Mark,
    Space,
}

/// Flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    None,
    XonXoff,
    Hardware,
}

impl SerialConfig {
    /// Standard 115200 8N1
    pub const STANDARD: Self = Self {
        baud_rate: 115200,
        data_bits: 8,
        stop_bits: 1,
        parity: SerialParity::None,
        flow_control: FlowControl::None,
    };

    /// Debug console (9600 8N1)
    pub const DEBUG: Self = Self {
        baud_rate: 9600,
        data_bits: 8,
        stop_bits: 1,
        parity: SerialParity::None,
        flow_control: FlowControl::None,
    };
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self::STANDARD
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_color() {
        let color = AnsiColor::Red;
        assert_eq!(color.fg_code(), 31);
        assert_eq!(color.bg_code(), 41);
    }

    #[test]
    fn test_color256() {
        // Test color cube
        let color = Color256::from_cube(5, 0, 0);
        assert_eq!(color.0, 196); // Pure red in 6x6x6 cube

        // Test grayscale
        let gray = Color256::from_gray(0);
        assert_eq!(gray.0, 232);
    }

    #[test]
    fn test_cursor_pos() {
        let pos = CursorPos::new(10, 20);
        assert_eq!(pos.col, 10);
        assert_eq!(pos.row, 20);
    }

    #[test]
    fn test_terminal_size() {
        let size = TerminalSize::VGA_TEXT;
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 25);
        assert_eq!(size.total_cells(), 2000);
    }

    #[test]
    fn test_efi_color() {
        let attr = EfiColor::make_attr(EfiColor::White, EfiColor::Blue);
        assert_eq!(attr, 0x1F);
    }
}
