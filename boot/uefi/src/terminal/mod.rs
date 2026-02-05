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
    Black         = 0,
    /// Red
    Red           = 1,
    /// Green
    Green         = 2,
    /// Yellow
    Yellow        = 3,
    /// Blue
    Blue          = 4,
    /// Magenta
    Magenta       = 5,
    /// Cyan
    Cyan          = 6,
    /// White
    White         = 7,
    /// Bright black (gray)
    BrightBlack   = 8,
    /// Bright red
    BrightRed     = 9,
    /// Bright green
    BrightGreen   = 10,
    /// Bright yellow
    BrightYellow  = 11,
    /// Bright blue
    BrightBlue    = 12,
    /// Bright magenta
    BrightMagenta = 13,
    /// Bright cyan
    BrightCyan    = 14,
    /// Bright white
    BrightWhite   = 15,
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
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TermColor {
    /// Default color
    #[default]
    Default,
    /// Standard ANSI color
    Ansi(AnsiColor),
    /// 256-color palette
    Palette(Color256),
    /// True color (24-bit)
    Rgb(TrueColor),
}

// =============================================================================
// TEXT ATTRIBUTES
// =============================================================================

/// Bitflags for text styling options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextStyleFlags(u16);

impl TextStyleFlags {
    /// No style flags
    pub const NONE: Self = Self(0);
    /// Bold/bright
    pub const BOLD: Self = Self(1 << 0);
    /// Dim/faint
    pub const DIM: Self = Self(1 << 1);
    /// Italic
    pub const ITALIC: Self = Self(1 << 2);
    /// Underline
    pub const UNDERLINE: Self = Self(1 << 3);
    /// Slow blink
    pub const BLINK: Self = Self(1 << 4);
    /// Rapid blink
    pub const RAPID_BLINK: Self = Self(1 << 5);
    /// Reverse video
    pub const REVERSE: Self = Self(1 << 6);
    /// Hidden/invisible
    pub const HIDDEN: Self = Self(1 << 7);
    /// Strikethrough
    pub const STRIKETHROUGH: Self = Self(1 << 8);
    /// Double underline
    pub const DOUBLE_UNDERLINE: Self = Self(1 << 9);
    /// Overline
    pub const OVERLINE: Self = Self(1 << 10);

    /// Check if a flag is set
    #[inline]
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Set a flag
    #[inline]
    #[must_use]
    pub const fn with(self, flag: Self) -> Self {
        Self(self.0 | flag.0)
    }

    /// Clear a flag
    #[inline]
    #[must_use]
    pub const fn without(self, flag: Self) -> Self {
        Self(self.0 & !flag.0)
    }

    /// Toggle a flag
    #[inline]
    #[must_use]
    pub const fn toggle(self, flag: Self) -> Self {
        Self(self.0 ^ flag.0)
    }

    /// Check if bold is set
    #[inline]
    pub const fn bold(self) -> bool {
        self.contains(Self::BOLD)
    }

    /// Check if dim is set
    #[inline]
    pub const fn dim(self) -> bool {
        self.contains(Self::DIM)
    }

    /// Check if italic is set
    #[inline]
    pub const fn italic(self) -> bool {
        self.contains(Self::ITALIC)
    }

    /// Check if underline is set
    #[inline]
    pub const fn underline(self) -> bool {
        self.contains(Self::UNDERLINE)
    }

    /// Check if blink is set
    #[inline]
    pub const fn blink(self) -> bool {
        self.contains(Self::BLINK)
    }

    /// Check if rapid blink is set
    #[inline]
    pub const fn rapid_blink(self) -> bool {
        self.contains(Self::RAPID_BLINK)
    }

    /// Check if reverse is set
    #[inline]
    pub const fn reverse(self) -> bool {
        self.contains(Self::REVERSE)
    }

    /// Check if hidden is set
    #[inline]
    pub const fn hidden(self) -> bool {
        self.contains(Self::HIDDEN)
    }

    /// Check if strikethrough is set
    #[inline]
    pub const fn strikethrough(self) -> bool {
        self.contains(Self::STRIKETHROUGH)
    }

    /// Check if double underline is set
    #[inline]
    pub const fn double_underline(self) -> bool {
        self.contains(Self::DOUBLE_UNDERLINE)
    }

    /// Check if overline is set
    #[inline]
    pub const fn overline(self) -> bool {
        self.contains(Self::OVERLINE)
    }
}

/// Text attributes (SGR - Select Graphic Rendition)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAttributes {
    /// Foreground color
    pub fg: TermColor,
    /// Background color
    pub bg: TermColor,
    /// Style flags (bold, italic, underline, etc.)
    pub flags: TextStyleFlags,
}

impl TextAttributes {
    /// Default attributes
    pub const DEFAULT: Self = Self {
        fg: TermColor::Default,
        bg: TermColor::Default,
        flags: TextStyleFlags::NONE,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    /// Block cursor
    #[default]
    Block,
    /// Underline cursor
    Underline,
    /// Vertical bar
    Bar,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalMode {
    /// VT100 compatible
    Vt100,
    /// VT220 compatible
    Vt220,
    /// ANSI compatible
    Ansi,
    /// xterm compatible
    #[default]
    Xterm,
    /// Linux console
    Linux,
    /// Basic text mode
    Basic,
}

/// DEC private modes
pub mod dec_modes {
    /// Application cursor keys mode
    pub const CURSOR_KEYS: u16 = 1;
    /// VT52 compatibility mode (DECANM)
    pub const VT52_MODE: u16 = 2;
    /// 132 column mode
    pub const COLUMN_132: u16 = 3;
    /// Smooth scrolling mode
    pub const SMOOTH_SCROLL: u16 = 4;
    /// Reverse video mode
    pub const REVERSE_VIDEO: u16 = 5;
    /// Origin mode
    pub const ORIGIN_MODE: u16 = 6;
    /// Auto-wrap mode
    pub const AUTO_WRAP: u16 = 7;
    /// Auto-repeat keys mode
    pub const AUTO_REPEAT: u16 = 8;
    /// X10 mouse reporting mode
    pub const MOUSE_X10: u16 = 9;
    /// Cursor visibility mode
    pub const CURSOR_VISIBLE: u16 = 25;
    /// VT200 mouse tracking mode
    pub const MOUSE_VT200: u16 = 1000;
    /// Highlight mouse tracking mode
    pub const MOUSE_HILITE: u16 = 1001;
    /// Cell motion mouse tracking mode
    pub const MOUSE_CELL: u16 = 1002;
    /// All motion mouse tracking mode
    pub const MOUSE_ALL: u16 = 1003;
    /// UTF-8 mouse encoding mode
    pub const MOUSE_UTF8: u16 = 1005;
    /// SGR mouse encoding mode
    pub const MOUSE_SGR: u16 = 1006;
    /// Alternate screen buffer mode
    pub const ALT_SCREEN: u16 = 1049;
    /// Bracketed paste mode
    pub const BRACKETED_PASTE: u16 = 2004;
}

/// Bitflags for terminal mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TerminalModeFlags(u16);

impl TerminalModeFlags {
    /// No mode flags
    pub const NONE: Self = Self(0);
    /// Application cursor keys
    pub const APP_CURSOR: Self = Self(1 << 0);
    /// Application keypad
    pub const APP_KEYPAD: Self = Self(1 << 1);
    /// Auto-wrap mode
    pub const AUTO_WRAP: Self = Self(1 << 2);
    /// Origin mode
    pub const ORIGIN_MODE: Self = Self(1 << 3);
    /// Insert mode
    pub const INSERT_MODE: Self = Self(1 << 4);
    /// Line feed mode
    pub const LINE_FEED_MODE: Self = Self(1 << 5);
    /// Local echo
    pub const LOCAL_ECHO: Self = Self(1 << 6);
    /// Reverse wrap
    pub const REVERSE_WRAP: Self = Self(1 << 7);
    /// Alternate screen buffer
    pub const ALT_SCREEN: Self = Self(1 << 8);
    /// Bracketed paste mode
    pub const BRACKETED_PASTE: Self = Self(1 << 9);
    /// Report focus events
    pub const FOCUS_EVENTS: Self = Self(1 << 10);
    /// Mouse tracking enabled
    pub const MOUSE_TRACKING: Self = Self(1 << 11);
    /// Save/restore cursor
    pub const SAVE_CURSOR: Self = Self(1 << 12);

    /// Check if a flag is set
    #[inline]
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Set a flag
    #[inline]
    #[must_use]
    pub const fn with(self, flag: Self) -> Self {
        Self(self.0 | flag.0)
    }

    /// Clear a flag
    #[inline]
    #[must_use]
    pub const fn without(self, flag: Self) -> Self {
        Self(self.0 & !flag.0)
    }

    /// Toggle a flag
    #[inline]
    #[must_use]
    pub const fn toggle(self, flag: Self) -> Self {
        Self(self.0 ^ flag.0)
    }

    /// Check if app cursor is set
    #[inline]
    pub const fn app_cursor(self) -> bool {
        self.contains(Self::APP_CURSOR)
    }

    /// Check if app keypad is set
    #[inline]
    pub const fn app_keypad(self) -> bool {
        self.contains(Self::APP_KEYPAD)
    }

    /// Check if auto wrap is set
    #[inline]
    pub const fn auto_wrap(self) -> bool {
        self.contains(Self::AUTO_WRAP)
    }

    /// Check if origin mode is set
    #[inline]
    pub const fn origin_mode(self) -> bool {
        self.contains(Self::ORIGIN_MODE)
    }

    /// Check if insert mode is set
    #[inline]
    pub const fn insert_mode(self) -> bool {
        self.contains(Self::INSERT_MODE)
    }

    /// Check if line feed mode is set
    #[inline]
    pub const fn line_feed_mode(self) -> bool {
        self.contains(Self::LINE_FEED_MODE)
    }

    /// Check if local echo is set
    #[inline]
    pub const fn local_echo(self) -> bool {
        self.contains(Self::LOCAL_ECHO)
    }

    /// Check if reverse wrap is set
    #[inline]
    pub const fn reverse_wrap(self) -> bool {
        self.contains(Self::REVERSE_WRAP)
    }

    /// Check if alt screen is set
    #[inline]
    pub const fn alt_screen(self) -> bool {
        self.contains(Self::ALT_SCREEN)
    }

    /// Check if bracketed paste is set
    #[inline]
    pub const fn bracketed_paste(self) -> bool {
        self.contains(Self::BRACKETED_PASTE)
    }

    /// Check if focus events is set
    #[inline]
    pub const fn focus_events(self) -> bool {
        self.contains(Self::FOCUS_EVENTS)
    }

    /// Check if mouse tracking is set
    #[inline]
    pub const fn mouse_tracking(self) -> bool {
        self.contains(Self::MOUSE_TRACKING)
    }

    /// Check if save cursor is set
    #[inline]
    pub const fn save_cursor(self) -> bool {
        self.contains(Self::SAVE_CURSOR)
    }
}

/// Terminal mode flags
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalModes {
    /// Mode flags
    pub flags: TerminalModeFlags,
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
        Self {
            top: 0,
            bottom: height.saturating_sub(1),
        }
    }
}

// =============================================================================
// ESCAPE SEQUENCE PARSER
// =============================================================================

/// Parser state for escape sequences
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParserState {
    /// Ground state - normal character processing
    #[default]
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
    /// Reset all attributes
    pub const RESET: u8 = 0;
    /// Bold/bright intensity
    pub const BOLD: u8 = 1;
    /// Dim/faint intensity
    pub const DIM: u8 = 2;
    /// Italic text
    pub const ITALIC: u8 = 3;
    /// Underline text
    pub const UNDERLINE: u8 = 4;
    /// Slow blink
    pub const SLOW_BLINK: u8 = 5;
    /// Rapid blink
    pub const RAPID_BLINK: u8 = 6;
    /// Reverse video
    pub const REVERSE: u8 = 7;
    /// Hidden/invisible text
    pub const HIDDEN: u8 = 8;
    /// Strikethrough text
    pub const STRIKETHROUGH: u8 = 9;

    /// Default font
    pub const DEFAULT_FONT: u8 = 10;

    /// Double underline
    pub const DOUBLE_UNDERLINE: u8 = 21;
    /// Normal intensity (not bold/dim)
    pub const NORMAL_INTENSITY: u8 = 22;
    /// Not italic
    pub const NOT_ITALIC: u8 = 23;
    /// Not underlined
    pub const NOT_UNDERLINED: u8 = 24;
    /// Not blinking
    pub const NOT_BLINKING: u8 = 25;
    /// Not reversed
    pub const NOT_REVERSED: u8 = 27;
    /// Reveal hidden text
    pub const REVEAL: u8 = 28;
    /// Not strikethrough
    pub const NOT_STRIKETHROUGH: u8 = 29;

    /// Foreground black
    pub const FG_BLACK: u8 = 30;
    /// Foreground red
    pub const FG_RED: u8 = 31;
    /// Foreground green
    pub const FG_GREEN: u8 = 32;
    /// Foreground yellow
    pub const FG_YELLOW: u8 = 33;
    /// Foreground blue
    pub const FG_BLUE: u8 = 34;
    /// Foreground magenta
    pub const FG_MAGENTA: u8 = 35;
    /// Foreground cyan
    pub const FG_CYAN: u8 = 36;
    /// Foreground white
    pub const FG_WHITE: u8 = 37;
    /// Foreground extended color
    pub const FG_EXTENDED: u8 = 38;
    /// Foreground default color
    pub const FG_DEFAULT: u8 = 39;

    /// Background black
    pub const BG_BLACK: u8 = 40;
    /// Background red
    pub const BG_RED: u8 = 41;
    /// Background green
    pub const BG_GREEN: u8 = 42;
    /// Background yellow
    pub const BG_YELLOW: u8 = 43;
    /// Background blue
    pub const BG_BLUE: u8 = 44;
    /// Background magenta
    pub const BG_MAGENTA: u8 = 45;
    /// Background cyan
    pub const BG_CYAN: u8 = 46;
    /// Background white
    pub const BG_WHITE: u8 = 47;
    /// Background extended color
    pub const BG_EXTENDED: u8 = 48;
    /// Background default color
    pub const BG_DEFAULT: u8 = 49;

    /// Overline text
    pub const OVERLINE: u8 = 53;
    /// Not overlined
    pub const NOT_OVERLINE: u8 = 55;

    /// Foreground bright black
    pub const FG_BRIGHT_BLACK: u8 = 90;
    /// Foreground bright red
    pub const FG_BRIGHT_RED: u8 = 91;
    /// Foreground bright green
    pub const FG_BRIGHT_GREEN: u8 = 92;
    /// Foreground bright yellow
    pub const FG_BRIGHT_YELLOW: u8 = 93;
    /// Foreground bright blue
    pub const FG_BRIGHT_BLUE: u8 = 94;
    /// Foreground bright magenta
    pub const FG_BRIGHT_MAGENTA: u8 = 95;
    /// Foreground bright cyan
    pub const FG_BRIGHT_CYAN: u8 = 96;
    /// Foreground bright white
    pub const FG_BRIGHT_WHITE: u8 = 97;

    /// Background bright black
    pub const BG_BRIGHT_BLACK: u8 = 100;
    /// Background bright red
    pub const BG_BRIGHT_RED: u8 = 101;
    /// Background bright green
    pub const BG_BRIGHT_GREEN: u8 = 102;
    /// Background bright yellow
    pub const BG_BRIGHT_YELLOW: u8 = 103;
    /// Background bright blue
    pub const BG_BRIGHT_BLUE: u8 = 104;
    /// Background bright magenta
    pub const BG_BRIGHT_MAGENTA: u8 = 105;
    /// Background bright cyan
    pub const BG_BRIGHT_CYAN: u8 = 106;
    /// Background bright white
    pub const BG_BRIGHT_WHITE: u8 = 107;
}

// =============================================================================
// BOX DRAWING CHARACTERS
// =============================================================================

/// Box drawing characters
pub mod box_chars {
    /// Horizontal line (─)
    pub const HORIZONTAL: char = '─';
    /// Vertical line (│)
    pub const VERTICAL: char = '│';
    /// Top-left corner (┌)
    pub const TOP_LEFT: char = '┌';
    /// Top-right corner (┐)
    pub const TOP_RIGHT: char = '┐';
    /// Bottom-left corner (└)
    pub const BOTTOM_LEFT: char = '└';
    /// Bottom-right corner (┘)
    pub const BOTTOM_RIGHT: char = '┘';
    /// Cross intersection (┼)
    pub const CROSS: char = '┼';
    /// T-junction pointing down (┬)
    pub const T_DOWN: char = '┬';
    /// T-junction pointing up (┴)
    pub const T_UP: char = '┴';
    /// T-junction pointing right (├)
    pub const T_RIGHT: char = '├';
    /// T-junction pointing left (┤)
    pub const T_LEFT: char = '┤';

    /// Double horizontal line (═)
    pub const DOUBLE_HORIZONTAL: char = '═';
    /// Double vertical line (║)
    pub const DOUBLE_VERTICAL: char = '║';
    /// Double top-left corner (╔)
    pub const DOUBLE_TOP_LEFT: char = '╔';
    /// Double top-right corner (╗)
    pub const DOUBLE_TOP_RIGHT: char = '╗';
    /// Double bottom-left corner (╚)
    pub const DOUBLE_BOTTOM_LEFT: char = '╚';
    /// Double bottom-right corner (╝)
    pub const DOUBLE_BOTTOM_RIGHT: char = '╝';
    /// Double cross intersection (╬)
    pub const DOUBLE_CROSS: char = '╬';
    /// Double T-junction pointing down (╦)
    pub const DOUBLE_T_DOWN: char = '╦';
    /// Double T-junction pointing up (╩)
    pub const DOUBLE_T_UP: char = '╩';
    /// Double T-junction pointing right (╠)
    pub const DOUBLE_T_RIGHT: char = '╠';
    /// Double T-junction pointing left (╣)
    pub const DOUBLE_T_LEFT: char = '╣';

    /// Rounded top-left corner (╭)
    pub const ROUNDED_TOP_LEFT: char = '╭';
    /// Rounded top-right corner (╮)
    pub const ROUNDED_TOP_RIGHT: char = '╮';
    /// Rounded bottom-left corner (╰)
    pub const ROUNDED_BOTTOM_LEFT: char = '╰';
    /// Rounded bottom-right corner (╯)
    pub const ROUNDED_BOTTOM_RIGHT: char = '╯';

    /// Full block (█)
    pub const FULL_BLOCK: char = '█';
    /// Upper half block (▀)
    pub const UPPER_HALF: char = '▀';
    /// Lower half block (▄)
    pub const LOWER_HALF: char = '▄';
    /// Left half block (▌)
    pub const LEFT_HALF: char = '▌';
    /// Right half block (▐)
    pub const RIGHT_HALF: char = '▐';
    /// Light shade (░)
    pub const LIGHT_SHADE: char = '░';
    /// Medium shade (▒)
    pub const MEDIUM_SHADE: char = '▒';
    /// Dark shade (▓)
    pub const DARK_SHADE: char = '▓';

    /// Progress bar empty segment (░)
    pub const PROGRESS_EMPTY: char = '░';
    /// Progress bar partial segment (▓)
    pub const PROGRESS_PARTIAL: char = '▓';
    /// Progress bar full segment (█)
    pub const PROGRESS_FULL: char = '█';
}

// =============================================================================
// SPECIAL CHARACTERS
// =============================================================================

/// Special character mappings
pub mod special_chars {
    /// Up arrow (↑)
    pub const ARROW_UP: char = '↑';
    /// Down arrow (↓)
    pub const ARROW_DOWN: char = '↓';
    /// Left arrow (←)
    pub const ARROW_LEFT: char = '←';
    /// Right arrow (→)
    pub const ARROW_RIGHT: char = '→';
    /// Up-down arrow (↕)
    pub const ARROW_UP_DOWN: char = '↕';
    /// Left-right arrow (↔)
    pub const ARROW_LEFT_RIGHT: char = '↔';

    /// Check mark (✓)
    pub const CHECK: char = '✓';
    /// Cross mark (✗)
    pub const CROSS: char = '✗';
    /// Bullet point (•)
    pub const BULLET: char = '•';
    /// Diamond (◆)
    pub const DIAMOND: char = '◆';
    /// Star (★)
    pub const STAR: char = '★';
    /// Filled circle (●)
    pub const CIRCLE: char = '●';
    /// Empty circle (○)
    pub const EMPTY_CIRCLE: char = '○';
    /// Filled square (■)
    pub const SQUARE: char = '■';
    /// Empty square (□)
    pub const EMPTY_SQUARE: char = '□';

    /// Greek letter alpha (α)
    pub const ALPHA: char = 'α';
    /// Greek letter beta (β)
    pub const BETA: char = 'β';
    /// Greek letter gamma (γ)
    pub const GAMMA: char = 'γ';
    /// Greek letter delta (δ)
    pub const DELTA: char = 'δ';
    /// Greek letter pi (π)
    pub const PI: char = 'π';
    /// Greek letter sigma (σ)
    pub const SIGMA: char = 'σ';
    /// Greek letter omega (ω)
    pub const OMEGA: char = 'ω';

    /// Plus-minus sign (±)
    pub const PLUS_MINUS: char = '±';
    /// Infinity symbol (∞)
    pub const INFINITY: char = '∞';
    /// Approximately equal (≈)
    pub const APPROX: char = '≈';
    /// Not equal (≠)
    pub const NOT_EQUAL: char = '≠';
    /// Less than or equal (≤)
    pub const LESS_EQUAL: char = '≤';
    /// Greater than or equal (≥)
    pub const GREATER_EQUAL: char = '≥';
    /// Square root (√)
    pub const SQRT: char = '√';
    /// Summation (∑)
    pub const SUM: char = '∑';
    /// Product (∏)
    pub const PRODUCT: char = '∏';
}

// =============================================================================
// CONSOLE OUTPUT ATTRIBUTES (EFI)
// =============================================================================

/// EFI Console colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EfiColor {
    /// Black color (0x00)
    Black        = 0x00,
    /// Blue color (0x01)
    Blue         = 0x01,
    /// Green color (0x02)
    Green        = 0x02,
    /// Cyan color (0x03)
    Cyan         = 0x03,
    /// Red color (0x04)
    Red          = 0x04,
    /// Magenta color (0x05)
    Magenta      = 0x05,
    /// Brown color (0x06)
    Brown        = 0x06,
    /// Light gray color (0x07)
    LightGray    = 0x07,
    /// Dark gray color (0x08)
    DarkGray     = 0x08,
    /// Light blue color (0x09)
    LightBlue    = 0x09,
    /// Light green color (0x0A)
    LightGreen   = 0x0A,
    /// Light cyan color (0x0B)
    LightCyan    = 0x0B,
    /// Light red color (0x0C)
    LightRed     = 0x0C,
    /// Light magenta color (0x0D)
    LightMagenta = 0x0D,
    /// Yellow color (0x0E)
    Yellow       = 0x0E,
    /// White color (0x0F)
    White        = 0x0F,
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
    pub const EXTENDED: Self = Self {
        cols: 132,
        rows: 43,
    };
    /// Modern 120x40
    pub const MODERN: Self = Self {
        cols: 120,
        rows: 40,
    };

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
    /// No parity bit
    None,
    /// Odd parity
    Odd,
    /// Even parity
    Even,
    /// Mark parity (always 1)
    Mark,
    /// Space parity (always 0)
    Space,
}

/// Flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl {
    /// No flow control
    None,
    /// Software flow control (XON/XOFF)
    XonXoff,
    /// Hardware flow control (RTS/CTS)
    Hardware,
}

impl SerialConfig {
    /// Standard 115200 8N1
    pub const STANDARD: Self = Self {
        baud_rate: 115_200,
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

#[cfg(all(test, feature = "std"))]
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
