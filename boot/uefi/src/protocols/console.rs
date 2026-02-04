//! Console Protocol
//!
//! High-level console I/O abstraction for text input/output.

use super::Protocol;
use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// CONSOLE
// =============================================================================

/// High-level console abstraction
pub struct Console {
    /// System table pointer
    system_table: *mut crate::raw::system_table::EfiSystemTable,
}

impl Console {
    /// Create new console from system table
    ///
    /// # Safety
    /// System table pointer must be valid
    pub unsafe fn new(system_table: *mut crate::raw::system_table::EfiSystemTable) -> Self {
        Self { system_table }
    }

    /// Get from global state
    pub fn get() -> Result<Self> {
        let st = unsafe { crate::services::system_table() };
        Ok(Self {
            system_table: st as *const _ as *mut _,
        })
    }

    // =========================================================================
    // OUTPUT
    // =========================================================================

    /// Write string to console
    pub fn write(&self, s: &str) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        // Convert to UCS-2
        let mut buffer = Vec::with_capacity(s.len() + 1);
        for c in s.chars() {
            if c == '\n' {
                buffer.push('\r' as u16);
            }
            buffer.push(c as u16);
        }
        buffer.push(0);

        let result = unsafe { ((*con_out).output_string)(con_out, buffer.as_ptr()) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write line to console
    pub fn writeln(&self, s: &str) -> Result<()> {
        self.write(s)?;
        self.write("\n")
    }

    /// Write formatted string
    pub fn write_fmt(&self, args: core::fmt::Arguments<'_>) -> Result<()> {
        use core::fmt::Write;
        let mut writer = ConsoleWriter { console: self };
        writer.write_fmt(args).map_err(|_| Error::DeviceError)
    }

    /// Clear screen
    pub fn clear(&self) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let result = unsafe { ((*con_out).clear_screen)(con_out) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Set cursor position
    pub fn set_cursor(&self, col: usize, row: usize) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let result = unsafe { ((*con_out).set_cursor_position)(con_out, col, row) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Enable/disable cursor
    pub fn set_cursor_visible(&self, visible: bool) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let result = unsafe { ((*con_out).enable_cursor)(con_out, visible as u8) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Set text color
    pub fn set_color(&self, foreground: Color, background: Color) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let attribute = foreground.0 | (background.0 << 4);
        let result = unsafe { ((*con_out).set_attribute)(con_out, attribute) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Reset output
    pub fn reset_output(&self) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let result = unsafe { ((*con_out).reset)(con_out, false as u8) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Get output mode
    pub fn output_mode(&self) -> Result<OutputMode> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let mode = unsafe { &*(*con_out).mode };

        Ok(OutputMode {
            columns: mode.cursor_column as usize,
            rows: mode.cursor_row as usize,
            cursor_visible: mode.cursor_visible != 0,
            mode_number: mode.mode as usize,
            max_mode: mode.max_mode as usize,
            attribute: mode.attribute as usize,
        })
    }

    /// Query text mode
    pub fn query_mode(&self, mode: usize) -> Result<(usize, usize)> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let mut columns = 0usize;
        let mut rows = 0usize;

        let result = unsafe { ((*con_out).query_mode)(con_out, mode, &mut columns, &mut rows) };

        if result == Status::SUCCESS {
            Ok((columns, rows))
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Set text mode
    pub fn set_mode(&self, mode: usize) -> Result<()> {
        let con_out = unsafe { (*self.system_table).con_out };
        if con_out.is_null() {
            return Err(Error::NotReady);
        }

        let result = unsafe { ((*con_out).set_mode)(con_out, mode) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Get available modes
    pub fn available_modes(&self) -> Result<Vec<(usize, usize)>> {
        let mode = self.output_mode()?;
        let mut modes = Vec::with_capacity(mode.max_mode);

        for i in 0..mode.max_mode {
            if let Ok((cols, rows)) = self.query_mode(i) {
                modes.push((cols, rows));
            }
        }

        Ok(modes)
    }

    /// Set best mode (highest resolution)
    pub fn set_best_mode(&self) -> Result<(usize, usize)> {
        let modes = self.available_modes()?;

        let best = modes
            .iter()
            .enumerate()
            .max_by_key(|(_, (cols, rows))| cols * rows)
            .map(|(i, &(cols, rows))| (i, cols, rows));

        if let Some((mode_num, cols, rows)) = best {
            self.set_mode(mode_num)?;
            Ok((cols, rows))
        } else {
            Err(Error::NotFound)
        }
    }

    // =========================================================================
    // INPUT
    // =========================================================================

    /// Read key (non-blocking)
    pub fn read_key(&self) -> Result<Option<InputKey>> {
        let con_in = unsafe { (*self.system_table).con_in };
        if con_in.is_null() {
            return Err(Error::NotReady);
        }

        let mut key = crate::raw::types::InputKey {
            scan_code: 0,
            unicode_char: 0,
        };

        let result = unsafe { ((*con_in).read_key_stroke)(con_in, &mut key) };

        match result {
            Status::SUCCESS => Ok(Some(InputKey::from_raw(key))),
            Status::NOT_READY => Ok(None),
            _ => Err(Error::from_status(result)),
        }
    }

    /// Wait for key press (blocking)
    pub fn wait_for_key(&self) -> Result<InputKey> {
        let con_in = unsafe { (*self.system_table).con_in };
        if con_in.is_null() {
            return Err(Error::NotReady);
        }

        // Wait on wait_for_key event
        let event = unsafe { (*con_in).wait_for_key };
        let events = [event];
        let mut index = 0usize;

        let bs = unsafe { crate::services::boot_services() };
        let result = unsafe { ((*bs).wait_for_event)(1, events.as_ptr(), &mut index) };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        // Now read the key
        self.read_key()?.ok_or(Error::NotReady)
    }

    /// Read line with echo
    pub fn read_line(&self, max_len: usize) -> Result<String> {
        let mut buffer = String::with_capacity(max_len);

        loop {
            let key = self.wait_for_key()?;

            match key.unicode_char {
                Some('\r') | Some('\n') => {
                    self.writeln("")?;
                    break;
                },
                Some('\x08') => {
                    // Backspace
                    if !buffer.is_empty() {
                        buffer.pop();
                        self.write("\x08 \x08")?;
                    }
                },
                Some(c) if !c.is_control() => {
                    if buffer.len() < max_len {
                        buffer.push(c);
                        self.write(&alloc::format!("{}", c))?;
                    }
                },
                _ => {
                    // Handle scan codes (arrows, etc.)
                },
            }
        }

        Ok(buffer)
    }

    /// Read line with prompt
    pub fn prompt(&self, prompt: &str, max_len: usize) -> Result<String> {
        self.write(prompt)?;
        self.read_line(max_len)
    }

    /// Read password (no echo)
    pub fn read_password(&self, max_len: usize) -> Result<String> {
        let mut buffer = String::with_capacity(max_len);

        loop {
            let key = self.wait_for_key()?;

            match key.unicode_char {
                Some('\r') | Some('\n') => {
                    self.writeln("")?;
                    break;
                },
                Some('\x08') => {
                    if !buffer.is_empty() {
                        buffer.pop();
                        self.write("\x08 \x08")?;
                    }
                },
                Some(c) if !c.is_control() => {
                    if buffer.len() < max_len {
                        buffer.push(c);
                        self.write("*")?;
                    }
                },
                _ => {},
            }
        }

        Ok(buffer)
    }

    /// Reset input
    pub fn reset_input(&self) -> Result<()> {
        let con_in = unsafe { (*self.system_table).con_in };
        if con_in.is_null() {
            return Err(Error::NotReady);
        }

        let result = unsafe { ((*con_in).reset)(con_in, false as u8) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Flush input buffer
    pub fn flush_input(&self) -> Result<()> {
        while let Ok(Some(_)) = self.read_key() {}
        Ok(())
    }

    // =========================================================================
    // UTILITIES
    // =========================================================================

    /// Print progress bar
    pub fn progress_bar(&self, current: usize, total: usize, width: usize) -> Result<()> {
        let filled = if total > 0 {
            current * width / total
        } else {
            0
        };
        let empty = width.saturating_sub(filled);
        let percent = if total > 0 { current * 100 / total } else { 0 };

        self.write("\r[")?;
        for _ in 0..filled {
            self.write("=")?;
        }
        if filled < width {
            self.write(">")?;
            for _ in 0..empty.saturating_sub(1) {
                self.write(" ")?;
            }
        }
        self.write(&alloc::format!("] {}%", percent))
    }

    /// Print spinner frame
    pub fn spinner(&self, frame: usize) -> Result<()> {
        const FRAMES: [char; 4] = ['|', '/', '-', '\\'];
        let c = FRAMES[frame % FRAMES.len()];
        self.write(&alloc::format!("\r{}", c))
    }

    /// Ask yes/no question
    pub fn confirm(&self, question: &str) -> Result<bool> {
        self.write(&alloc::format!("{} [y/n]: ", question))?;

        loop {
            let key = self.wait_for_key()?;
            match key.unicode_char {
                Some('y') | Some('Y') => {
                    self.writeln("y")?;
                    return Ok(true);
                },
                Some('n') | Some('N') => {
                    self.writeln("n")?;
                    return Ok(false);
                },
                _ => {},
            }
        }
    }

    /// Print menu and get selection
    pub fn menu(&self, title: &str, options: &[&str]) -> Result<usize> {
        self.writeln(title)?;
        self.writeln("")?;

        for (i, option) in options.iter().enumerate() {
            self.writeln(&alloc::format!("  {}. {}", i + 1, option))?;
        }

        self.writeln("")?;

        loop {
            self.write("Select option: ")?;
            let input = self.read_line(10)?;

            if let Ok(num) = input.trim().parse::<usize>() {
                if num >= 1 && num <= options.len() {
                    return Ok(num - 1);
                }
            }

            self.writeln("Invalid selection, try again.")?;
        }
    }
}

impl Protocol for Console {
    const GUID: Guid = Guid::new(0x387477C1, 0x69C7, 0x11D2, [
        0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
    ]);

    fn open(_handle: Handle) -> Result<Self> {
        Console::get()
    }
}

/// Console writer for fmt::Write
struct ConsoleWriter<'a> {
    console: &'a Console,
}

impl<'a> core::fmt::Write for ConsoleWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.console.write(s).map_err(|_| core::fmt::Error)
    }
}

// =============================================================================
// INPUT KEY
// =============================================================================

/// Input key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputKey {
    /// Scan code for special keys
    pub scan_code: ScanCode,
    /// Unicode character (if printable)
    pub unicode_char: Option<char>,
}

impl InputKey {
    /// Create from raw UEFI InputKey
    fn from_raw(raw: crate::raw::types::InputKey) -> Self {
        let unicode_char = if raw.unicode_char != 0 {
            char::from_u32(raw.unicode_char as u32)
        } else {
            None
        };

        Self {
            scan_code: ScanCode(raw.scan_code),
            unicode_char,
        }
    }

    /// Check if this is a printable character
    pub fn is_printable(&self) -> bool {
        self.unicode_char.map(|c| !c.is_control()).unwrap_or(false)
    }

    /// Check if this is a special key (arrow, function, etc.)
    pub fn is_special(&self) -> bool {
        self.scan_code.0 != 0
    }

    /// Check if this is Enter
    pub fn is_enter(&self) -> bool {
        self.unicode_char == Some('\r') || self.unicode_char == Some('\n')
    }

    /// Check if this is Escape
    pub fn is_escape(&self) -> bool {
        self.scan_code == ScanCode::ESCAPE
    }

    /// Check if this is Backspace
    pub fn is_backspace(&self) -> bool {
        self.unicode_char == Some('\x08')
    }

    /// Check if this is Tab
    pub fn is_tab(&self) -> bool {
        self.unicode_char == Some('\t')
    }

    /// Check if this is an arrow key
    pub fn is_arrow(&self) -> bool {
        matches!(
            self.scan_code,
            ScanCode::UP | ScanCode::DOWN | ScanCode::LEFT | ScanCode::RIGHT
        )
    }

    /// Check if this is a function key
    pub fn is_function(&self) -> bool {
        self.scan_code.0 >= ScanCode::F1.0 && self.scan_code.0 <= ScanCode::F12.0
    }
}

// =============================================================================
// SCAN CODE
// =============================================================================

/// Scan code for special keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanCode(pub u16);

impl ScanCode {
    /// No scan code
    pub const NULL: Self = Self(0x00);
    /// Up arrow
    pub const UP: Self = Self(0x01);
    /// Down arrow
    pub const DOWN: Self = Self(0x02);
    /// Right arrow
    pub const RIGHT: Self = Self(0x03);
    /// Left arrow
    pub const LEFT: Self = Self(0x04);
    /// Home
    pub const HOME: Self = Self(0x05);
    /// End
    pub const END: Self = Self(0x06);
    /// Insert
    pub const INSERT: Self = Self(0x07);
    /// Delete
    pub const DELETE: Self = Self(0x08);
    /// Page Up
    pub const PAGE_UP: Self = Self(0x09);
    /// Page Down
    pub const PAGE_DOWN: Self = Self(0x0A);
    /// F1
    pub const F1: Self = Self(0x0B);
    /// F2
    pub const F2: Self = Self(0x0C);
    /// F3
    pub const F3: Self = Self(0x0D);
    /// F4
    pub const F4: Self = Self(0x0E);
    /// F5
    pub const F5: Self = Self(0x0F);
    /// F6
    pub const F6: Self = Self(0x10);
    /// F7
    pub const F7: Self = Self(0x11);
    /// F8
    pub const F8: Self = Self(0x12);
    /// F9
    pub const F9: Self = Self(0x13);
    /// F10
    pub const F10: Self = Self(0x14);
    /// F11
    pub const F11: Self = Self(0x15);
    /// F12
    pub const F12: Self = Self(0x16);
    /// Escape
    pub const ESCAPE: Self = Self(0x17);
    /// Pause
    pub const PAUSE: Self = Self(0x48);
    /// F13
    pub const F13: Self = Self(0x68);
    /// F14
    pub const F14: Self = Self(0x69);
    /// F15
    pub const F15: Self = Self(0x6A);
    /// F16
    pub const F16: Self = Self(0x6B);
    /// F17
    pub const F17: Self = Self(0x6C);
    /// F18
    pub const F18: Self = Self(0x6D);
    /// F19
    pub const F19: Self = Self(0x6E);
    /// F20
    pub const F20: Self = Self(0x6F);
    /// F21
    pub const F21: Self = Self(0x70);
    /// F22
    pub const F22: Self = Self(0x71);
    /// F23
    pub const F23: Self = Self(0x72);
    /// F24
    pub const F24: Self = Self(0x73);
    /// Mute
    pub const MUTE: Self = Self(0x7F);
    /// Volume Up
    pub const VOLUME_UP: Self = Self(0x80);
    /// Volume Down
    pub const VOLUME_DOWN: Self = Self(0x81);
    /// Brightness Up
    pub const BRIGHTNESS_UP: Self = Self(0x100);
    /// Brightness Down
    pub const BRIGHTNESS_DOWN: Self = Self(0x101);
    /// Suspend
    pub const SUSPEND: Self = Self(0x102);
    /// Hibernate
    pub const HIBERNATE: Self = Self(0x103);
    /// Toggle Display
    pub const TOGGLE_DISPLAY: Self = Self(0x104);
    /// Recovery
    pub const RECOVERY: Self = Self(0x105);
    /// Eject
    pub const EJECT: Self = Self(0x106);

    /// Get name of scan code
    pub fn name(&self) -> &'static str {
        match *self {
            Self::NULL => "NULL",
            Self::UP => "Up",
            Self::DOWN => "Down",
            Self::RIGHT => "Right",
            Self::LEFT => "Left",
            Self::HOME => "Home",
            Self::END => "End",
            Self::INSERT => "Insert",
            Self::DELETE => "Delete",
            Self::PAGE_UP => "Page Up",
            Self::PAGE_DOWN => "Page Down",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::ESCAPE => "Escape",
            Self::PAUSE => "Pause",
            Self::MUTE => "Mute",
            Self::VOLUME_UP => "Volume Up",
            Self::VOLUME_DOWN => "Volume Down",
            Self::BRIGHTNESS_UP => "Brightness Up",
            Self::BRIGHTNESS_DOWN => "Brightness Down",
            Self::SUSPEND => "Suspend",
            Self::HIBERNATE => "Hibernate",
            _ => "Unknown",
        }
    }
}

// =============================================================================
// KEY MODIFIERS
// =============================================================================

/// Key modifiers (Shift, Ctrl, Alt, etc.)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers(pub u32);

impl KeyModifiers {
    /// Right Shift pressed
    pub const RIGHT_SHIFT: Self = Self(0x00000001);
    /// Left Shift pressed
    pub const LEFT_SHIFT: Self = Self(0x00000002);
    /// Right Control pressed
    pub const RIGHT_CONTROL: Self = Self(0x00000004);
    /// Left Control pressed
    pub const LEFT_CONTROL: Self = Self(0x00000008);
    /// Right Alt pressed
    pub const RIGHT_ALT: Self = Self(0x00000010);
    /// Left Alt pressed
    pub const LEFT_ALT: Self = Self(0x00000020);
    /// Right Logo pressed
    pub const RIGHT_LOGO: Self = Self(0x00000040);
    /// Left Logo pressed
    pub const LEFT_LOGO: Self = Self(0x00000080);
    /// Menu key pressed
    pub const MENU: Self = Self(0x00000100);
    /// SysReq pressed
    pub const SYS_REQ: Self = Self(0x00000200);

    /// Shift state valid
    pub const SHIFT_STATE_VALID: Self = Self(0x80000000);

    /// Check if any shift is pressed
    pub fn shift(&self) -> bool {
        (self.0 & (Self::LEFT_SHIFT.0 | Self::RIGHT_SHIFT.0)) != 0
    }

    /// Check if any control is pressed
    pub fn control(&self) -> bool {
        (self.0 & (Self::LEFT_CONTROL.0 | Self::RIGHT_CONTROL.0)) != 0
    }

    /// Check if any alt is pressed
    pub fn alt(&self) -> bool {
        (self.0 & (Self::LEFT_ALT.0 | Self::RIGHT_ALT.0)) != 0
    }

    /// Check if any logo is pressed
    pub fn logo(&self) -> bool {
        (self.0 & (Self::LEFT_LOGO.0 | Self::RIGHT_LOGO.0)) != 0
    }

    /// Check if modifiers are valid
    pub fn valid(&self) -> bool {
        (self.0 & Self::SHIFT_STATE_VALID.0) != 0
    }
}

// =============================================================================
// COLORS
// =============================================================================

/// Console color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub usize);

impl Color {
    /// Black
    pub const BLACK: Self = Self(0x00);
    /// Blue
    pub const BLUE: Self = Self(0x01);
    /// Green
    pub const GREEN: Self = Self(0x02);
    /// Cyan
    pub const CYAN: Self = Self(0x03);
    /// Red
    pub const RED: Self = Self(0x04);
    /// Magenta
    pub const MAGENTA: Self = Self(0x05);
    /// Brown
    pub const BROWN: Self = Self(0x06);
    /// Light Gray
    pub const LIGHT_GRAY: Self = Self(0x07);
    /// Dark Gray
    pub const DARK_GRAY: Self = Self(0x08);
    /// Light Blue
    pub const LIGHT_BLUE: Self = Self(0x09);
    /// Light Green
    pub const LIGHT_GREEN: Self = Self(0x0A);
    /// Light Cyan
    pub const LIGHT_CYAN: Self = Self(0x0B);
    /// Light Red
    pub const LIGHT_RED: Self = Self(0x0C);
    /// Light Magenta
    pub const LIGHT_MAGENTA: Self = Self(0x0D);
    /// Yellow
    pub const YELLOW: Self = Self(0x0E);
    /// White
    pub const WHITE: Self = Self(0x0F);

    /// Get color name
    pub fn name(&self) -> &'static str {
        match self.0 {
            0x00 => "Black",
            0x01 => "Blue",
            0x02 => "Green",
            0x03 => "Cyan",
            0x04 => "Red",
            0x05 => "Magenta",
            0x06 => "Brown",
            0x07 => "Light Gray",
            0x08 => "Dark Gray",
            0x09 => "Light Blue",
            0x0A => "Light Green",
            0x0B => "Light Cyan",
            0x0C => "Light Red",
            0x0D => "Light Magenta",
            0x0E => "Yellow",
            0x0F => "White",
            _ => "Unknown",
        }
    }
}

// =============================================================================
// OUTPUT MODE
// =============================================================================

/// Text output mode information
#[derive(Debug, Clone)]
pub struct OutputMode {
    /// Current column
    pub columns: usize,
    /// Current row
    pub rows: usize,
    /// Cursor visible
    pub cursor_visible: bool,
    /// Current mode number
    pub mode_number: usize,
    /// Maximum mode number
    pub max_mode: usize,
    /// Current attribute
    pub attribute: usize,
}

// =============================================================================
// PRINT MACROS
// =============================================================================

/// Print to console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        if let Ok(console) = $crate::protocols::console::Console::get() {
            let _ = console.write_fmt(format_args!($($arg)*));
        }
    }};
}

/// Print line to console
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        if let Ok(console) = $crate::protocols::console::Console::get() {
            let _ = console.write_fmt(format_args!($($arg)*));
            let _ = console.write("\n");
        }
    }};
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_scan_code_names() {
        assert_eq!(ScanCode::UP.name(), "Up");
        assert_eq!(ScanCode::F1.name(), "F1");
        assert_eq!(ScanCode::ESCAPE.name(), "Escape");
    }

    #[test]
    fn test_key_modifiers() {
        let mods = KeyModifiers(KeyModifiers::LEFT_SHIFT.0 | KeyModifiers::LEFT_CONTROL.0);
        assert!(mods.shift());
        assert!(mods.control());
        assert!(!mods.alt());
    }

    #[test]
    fn test_colors() {
        assert_eq!(Color::RED.name(), "Red");
        assert_eq!(Color::WHITE.name(), "White");
    }
}
