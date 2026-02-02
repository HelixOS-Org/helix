//! Console and Text Output
//!
//! Console abstraction for UEFI text and graphics output.

use super::config::LogLevel;

// =============================================================================
// CONSOLE
// =============================================================================

/// Console abstraction
pub struct Console {
    /// Simple Text Output Protocol pointer
    stdout: *mut SimpleTextOutput,
    /// Simple Text Input Protocol pointer
    stdin: *mut SimpleTextInput,
    /// Current log level
    log_level: LogLevel,
    /// Color support
    color_support: bool,
    /// Current foreground color
    fg_color: Color,
    /// Current background color
    bg_color: Color,
}

impl Console {
    /// Create new console
    pub fn new(stdout: *mut SimpleTextOutput, stdin: *mut SimpleTextInput) -> Self {
        Self {
            stdout,
            stdin,
            log_level: LogLevel::Info,
            color_support: true,
            fg_color: Color::White,
            bg_color: Color::Black,
        }
    }

    /// Set log level
    pub fn set_log_level(&mut self, level: LogLevel) {
        self.log_level = level;
    }

    /// Get log level
    pub fn log_level(&self) -> LogLevel {
        self.log_level
    }

    /// Clear screen
    pub fn clear(&self) {
        unsafe {
            if let Some(clear) = (*self.stdout).clear_screen {
                clear(self.stdout);
            }
        }
    }

    /// Reset console
    pub fn reset(&self) {
        unsafe {
            if let Some(reset) = (*self.stdout).reset {
                reset(self.stdout, false);
            }
        }
    }

    /// Enable cursor
    pub fn enable_cursor(&self, visible: bool) {
        unsafe {
            if let Some(enable) = (*self.stdout).enable_cursor {
                enable(self.stdout, visible);
            }
        }
    }

    /// Set cursor position
    pub fn set_cursor(&self, col: usize, row: usize) {
        unsafe {
            if let Some(set_pos) = (*self.stdout).set_cursor_position {
                set_pos(self.stdout, col, row);
            }
        }
    }

    /// Set text attributes
    pub fn set_attribute(&self, fg: Color, bg: Color) {
        let attr = ((bg as usize) << 4) | (fg as usize);
        unsafe {
            if let Some(set_attr) = (*self.stdout).set_attribute {
                set_attr(self.stdout, attr);
            }
        }
    }

    /// Output string
    pub fn output_string(&self, s: &str) {
        // Convert to UCS-2
        let mut buffer = [0u16; 256];
        let mut i = 0;

        for c in s.chars() {
            if i >= buffer.len() - 1 {
                break;
            }
            buffer[i] = c as u16;
            i += 1;
        }
        buffer[i] = 0; // Null terminate

        unsafe {
            if let Some(output) = (*self.stdout).output_string {
                output(self.stdout, buffer.as_ptr());
            }
        }
    }

    /// Print string
    pub fn print(&self, s: &str) {
        self.output_string(s);
    }

    /// Print string with newline
    pub fn println(&self, s: &str) {
        self.output_string(s);
        self.output_string("\r\n");
    }

    /// Print formatted message
    pub fn printf(&self, fmt: &str, args: &[&dyn core::fmt::Display]) {
        let mut arg_idx = 0;
        let mut chars = fmt.chars().peekable();
        let mut buffer = [0u8; 256];
        let mut buf_idx = 0;

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'}') {
                chars.next(); // consume '}'

                // Flush buffer
                if buf_idx > 0 {
                    if let Ok(s) = core::str::from_utf8(&buffer[..buf_idx]) {
                        self.output_string(s);
                    }
                    buf_idx = 0;
                }

                // Print argument
                if arg_idx < args.len() {
                    // Format argument (simplified)
                    // In real code, use core::fmt::write
                    arg_idx += 1;
                }
            } else {
                let mut encode_buf = [0u8; 4];
                let bytes = c.encode_utf8(&mut encode_buf);
                for &b in bytes.as_bytes() {
                    if buf_idx < buffer.len() {
                        buffer[buf_idx] = b;
                        buf_idx += 1;
                    }
                }
            }
        }

        // Flush remaining
        if buf_idx > 0 {
            if let Ok(s) = core::str::from_utf8(&buffer[..buf_idx]) {
                self.output_string(s);
            }
        }
    }

    /// Print with color
    pub fn print_colored(&self, s: &str, fg: Color) {
        let old_fg = self.fg_color;
        self.set_attribute(fg, self.bg_color);
        self.output_string(s);
        self.set_attribute(old_fg, self.bg_color);
    }

    /// Log at trace level
    pub fn trace(&self, s: &str) {
        if self.log_level as u8 <= LogLevel::Trace as u8 {
            self.print_colored("[TRACE] ", Color::DarkGray);
            self.println(s);
        }
    }

    /// Log at debug level
    pub fn debug(&self, s: &str) {
        if self.log_level as u8 <= LogLevel::Debug as u8 {
            self.print_colored("[DEBUG] ", Color::Cyan);
            self.println(s);
        }
    }

    /// Log at info level
    pub fn info(&self, s: &str) {
        if self.log_level as u8 <= LogLevel::Info as u8 {
            self.print_colored("[INFO]  ", Color::Green);
            self.println(s);
        }
    }

    /// Log at warning level
    pub fn warn(&self, s: &str) {
        if self.log_level as u8 <= LogLevel::Warn as u8 {
            self.print_colored("[WARN]  ", Color::Yellow);
            self.println(s);
        }
    }

    /// Log at error level
    pub fn error(&self, s: &str) {
        if self.log_level as u8 <= LogLevel::Error as u8 {
            self.print_colored("[ERROR] ", Color::Red);
            self.println(s);
        }
    }

    /// Read key
    pub fn read_key(&self) -> Option<Key> {
        let mut key = InputKey {
            scan_code: 0,
            unicode_char: 0,
        };

        unsafe {
            if let Some(read) = (*self.stdin).read_key_stroke {
                let status = read(self.stdin, &mut key);
                if status == 0 {
                    return Some(Key::from_input_key(&key));
                }
            }
        }

        None
    }

    /// Wait for any key
    pub fn wait_for_key(&self) -> Key {
        loop {
            if let Some(key) = self.read_key() {
                return key;
            }
            // Small delay
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
    }

    /// Query text mode
    pub fn query_mode(&self, mode: usize) -> Option<(usize, usize)> {
        let mut cols = 0usize;
        let mut rows = 0usize;

        unsafe {
            if let Some(query) = (*self.stdout).query_mode {
                let status = query(self.stdout, mode, &mut cols, &mut rows);
                if status == 0 {
                    return Some((cols, rows));
                }
            }
        }

        None
    }

    /// Set text mode
    pub fn set_mode(&self, mode: usize) -> bool {
        unsafe {
            if let Some(set) = (*self.stdout).set_mode {
                let status = set(self.stdout, mode);
                return status == 0;
            }
        }
        false
    }

    /// Get cursor position
    pub fn cursor_position(&self) -> (usize, usize) {
        unsafe {
            let col = (*self.stdout).cursor_column as usize;
            let row = (*self.stdout).cursor_row as usize;
            (col, row)
        }
    }
}

// =============================================================================
// PROTOCOL STRUCTURES
// =============================================================================

/// Simple Text Output Protocol (placeholder - would match UEFI spec)
#[repr(C)]
pub struct SimpleTextOutput {
    pub reset: Option<unsafe extern "efiapi" fn(*mut Self, bool) -> usize>,
    pub output_string: Option<unsafe extern "efiapi" fn(*mut Self, *const u16) -> usize>,
    pub test_string: Option<unsafe extern "efiapi" fn(*mut Self, *const u16) -> usize>,
    pub query_mode:
        Option<unsafe extern "efiapi" fn(*mut Self, usize, *mut usize, *mut usize) -> usize>,
    pub set_mode: Option<unsafe extern "efiapi" fn(*mut Self, usize) -> usize>,
    pub set_attribute: Option<unsafe extern "efiapi" fn(*mut Self, usize) -> usize>,
    pub clear_screen: Option<unsafe extern "efiapi" fn(*mut Self) -> usize>,
    pub set_cursor_position: Option<unsafe extern "efiapi" fn(*mut Self, usize, usize) -> usize>,
    pub enable_cursor: Option<unsafe extern "efiapi" fn(*mut Self, bool) -> usize>,
    pub mode: *mut TextMode,
    pub cursor_column: i32,
    pub cursor_row: i32,
}

/// Text mode info
#[repr(C)]
pub struct TextMode {
    pub max_mode: i32,
    pub mode: i32,
    pub attribute: i32,
    pub cursor_column: i32,
    pub cursor_row: i32,
    pub cursor_visible: bool,
}

/// Simple Text Input Protocol (placeholder)
#[repr(C)]
pub struct SimpleTextInput {
    pub reset: Option<unsafe extern "efiapi" fn(*mut Self, bool) -> usize>,
    pub read_key_stroke: Option<unsafe extern "efiapi" fn(*mut Self, *mut InputKey) -> usize>,
    pub wait_for_key: *mut core::ffi::c_void,
}

/// Input key structure
#[repr(C)]
#[derive(Default)]
pub struct InputKey {
    pub scan_code: u16,
    pub unicode_char: u16,
}

// =============================================================================
// COLORS
// =============================================================================

/// Console colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black        = 0x00,
    Blue         = 0x01,
    Green        = 0x02,
    Cyan         = 0x03,
    Red          = 0x04,
    Magenta      = 0x05,
    Brown        = 0x06,
    LightGray    = 0x07,
    DarkGray     = 0x08,
    LightBlue    = 0x09,
    LightGreen   = 0x0A,
    LightCyan    = 0x0B,
    LightRed     = 0x0C,
    LightMagenta = 0x0D,
    Yellow       = 0x0E,
    White        = 0x0F,
}

impl Color {
    /// Convert to attribute byte
    pub fn to_attribute(fg: Self, bg: Self) -> u8 {
        ((bg as u8) << 4) | (fg as u8)
    }
}

// =============================================================================
// KEYBOARD
// =============================================================================

/// Keyboard key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Character key
    Char(char),
    /// Enter key
    Enter,
    /// Escape key
    Escape,
    /// Backspace key
    Backspace,
    /// Tab key
    Tab,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up
    PageUp,
    /// Page Down
    PageDown,
    /// Insert key
    Insert,
    /// Delete key
    Delete,
    /// Function key (F1-F12)
    Function(u8),
    /// Unknown key
    Unknown(u16),
}

impl Key {
    /// Create from UEFI input key
    pub fn from_input_key(key: &InputKey) -> Self {
        // Check scan code first
        match key.scan_code {
            0x01 => Self::Up,
            0x02 => Self::Down,
            0x03 => Self::Right,
            0x04 => Self::Left,
            0x05 => Self::Home,
            0x06 => Self::End,
            0x07 => Self::Insert,
            0x08 => Self::Delete,
            0x09 => Self::PageUp,
            0x0A => Self::PageDown,
            0x0B => Self::Function(1),
            0x0C => Self::Function(2),
            0x0D => Self::Function(3),
            0x0E => Self::Function(4),
            0x0F => Self::Function(5),
            0x10 => Self::Function(6),
            0x11 => Self::Function(7),
            0x12 => Self::Function(8),
            0x13 => Self::Function(9),
            0x14 => Self::Function(10),
            0x15 => Self::Function(11),
            0x16 => Self::Function(12),
            0x17 => Self::Escape,
            0x00 => {
                // Check unicode char
                match key.unicode_char {
                    0x0008 => Self::Backspace,
                    0x0009 => Self::Tab,
                    0x000D => Self::Enter,
                    0x001B => Self::Escape,
                    c if c >= 0x20 && c < 0x7F => Self::Char(c as u8 as char),
                    _ => Self::Unknown(key.unicode_char),
                }
            },
            s => Self::Unknown(s),
        }
    }

    /// Check if printable character
    pub fn is_printable(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    /// Get character if printable
    pub fn as_char(&self) -> Option<char> {
        match self {
            Self::Char(c) => Some(*c),
            _ => None,
        }
    }
}

// =============================================================================
// FRAMEBUFFER CONSOLE
// =============================================================================

/// Framebuffer console for graphical output
pub struct FramebufferConsole {
    /// Framebuffer base address
    fb_base: *mut u32,
    /// Width in pixels
    width: u32,
    /// Height in pixels
    height: u32,
    /// Pixels per scanline
    stride: u32,
    /// Character width
    char_width: u32,
    /// Character height
    char_height: u32,
    /// Current column (in characters)
    cursor_col: u32,
    /// Current row (in characters)
    cursor_row: u32,
    /// Foreground color (ARGB)
    fg_color: u32,
    /// Background color (ARGB)
    bg_color: u32,
    /// Maximum columns
    max_cols: u32,
    /// Maximum rows
    max_rows: u32,
}

impl FramebufferConsole {
    /// Create new framebuffer console
    pub fn new(fb_base: *mut u32, width: u32, height: u32, stride: u32) -> Self {
        let char_width = 8;
        let char_height = 16;

        Self {
            fb_base,
            width,
            height,
            stride,
            char_width,
            char_height,
            cursor_col: 0,
            cursor_row: 0,
            fg_color: 0xFF_FF_FF_FF, // White
            bg_color: 0xFF_00_00_00, // Black
            max_cols: width / char_width,
            max_rows: height / char_height,
        }
    }

    /// Clear screen
    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, self.bg_color);
            }
        }
        self.cursor_col = 0;
        self.cursor_row = 0;
    }

    /// Put pixel
    #[inline]
    fn put_pixel(&self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            unsafe {
                let offset = (y * self.stride + x) as isize;
                *self.fb_base.offset(offset) = color;
            }
        }
    }

    /// Draw character
    fn draw_char(&self, c: char, x: u32, y: u32) {
        let glyph = get_glyph(c);

        for row in 0..self.char_height {
            let glyph_row = if row < 16 { glyph[row as usize] } else { 0 };

            for col in 0..self.char_width {
                let pixel_x = x + col;
                let pixel_y = y + row;

                let color = if (glyph_row >> (7 - col)) & 1 != 0 {
                    self.fg_color
                } else {
                    self.bg_color
                };

                self.put_pixel(pixel_x, pixel_y, color);
            }
        }
    }

    /// Print character
    pub fn putc(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_col = 0;
                self.cursor_row += 1;
            },
            '\r' => {
                self.cursor_col = 0;
            },
            '\t' => {
                self.cursor_col = (self.cursor_col + 4) & !3;
            },
            _ => {
                if self.cursor_col >= self.max_cols {
                    self.cursor_col = 0;
                    self.cursor_row += 1;
                }

                if self.cursor_row >= self.max_rows {
                    self.scroll();
                    self.cursor_row = self.max_rows - 1;
                }

                let x = self.cursor_col * self.char_width;
                let y = self.cursor_row * self.char_height;

                self.draw_char(c, x, y);
                self.cursor_col += 1;
            },
        }
    }

    /// Print string
    pub fn print(&mut self, s: &str) {
        for c in s.chars() {
            self.putc(c);
        }
    }

    /// Print with newline
    pub fn println(&mut self, s: &str) {
        self.print(s);
        self.putc('\n');
    }

    /// Scroll screen up
    fn scroll(&mut self) {
        let line_size = self.char_height * self.stride;

        unsafe {
            // Copy lines up
            core::ptr::copy(
                self.fb_base.offset(line_size as isize),
                self.fb_base,
                ((self.max_rows - 1) * line_size) as usize,
            );

            // Clear bottom line
            let bottom_start = (self.max_rows - 1) * self.char_height;
            for y in bottom_start..self.height {
                for x in 0..self.width {
                    self.put_pixel(x, y, self.bg_color);
                }
            }
        }
    }

    /// Set colors
    pub fn set_colors(&mut self, fg: u32, bg: u32) {
        self.fg_color = fg;
        self.bg_color = bg;
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, col: u32, row: u32) {
        self.cursor_col = col.min(self.max_cols - 1);
        self.cursor_row = row.min(self.max_rows - 1);
    }
}

/// Get font glyph for character (8x16 font)
fn get_glyph(c: char) -> [u8; 16] {
    // Simplified - return basic glyph
    // In real implementation, use embedded font data
    let byte = c as u8;

    if byte < 32 || byte > 126 {
        return [0; 16]; // Non-printable
    }

    // Very basic placeholder glyphs
    match c {
        'A'..='Z' => [
            0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        'a'..='z' => [
            0x00, 0x00, 0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        '0'..='9' => [
            0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        ' ' => [0; 16],
        _ => [
            0x00, 0x00, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_from_input() {
        let key = InputKey {
            scan_code: 0x01,
            unicode_char: 0,
        };
        assert_eq!(Key::from_input_key(&key), Key::Up);

        let key = InputKey {
            scan_code: 0,
            unicode_char: 0x41, // 'A'
        };
        assert_eq!(Key::from_input_key(&key), Key::Char('A'));
    }

    #[test]
    fn test_color_attribute() {
        let attr = Color::to_attribute(Color::White, Color::Blue);
        assert_eq!(attr, 0x1F);
    }
}
