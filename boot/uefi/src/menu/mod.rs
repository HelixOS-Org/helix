//! Boot Menu Interface
//!
//! Text and graphical boot menu for selecting boot entries.

use super::config::{BootConfig, BootEntry};
use super::console::{Color, Console, FramebufferConsole, Key};
use alloc::vec::Vec;

// =============================================================================
// BOOT MENU
// =============================================================================

/// Boot menu result
#[derive(Debug, Clone)]
pub enum MenuResult {
    /// Boot selected entry
    Boot(usize),
    /// Enter command line editor
    Editor,
    /// Enter shell
    Shell,
    /// Reboot system
    Reboot,
    /// Shutdown system
    Shutdown,
    /// Continue to default boot
    Continue,
    /// Timeout expired
    Timeout,
}

/// Boot menu state
pub struct BootMenu<'a> {
    /// Console
    console: &'a Console,
    /// Configuration
    config: &'a BootConfig,
    /// Selected entry index
    selected: usize,
    /// Timeout remaining (deciseconds)
    timeout: u32,
    /// Whether menu is visible
    visible: bool,
    /// Editor mode active
    editor_active: bool,
    /// Command line buffer
    cmdline_buffer: [u8; 256],
    /// Command line length
    cmdline_len: usize,
}

impl<'a> BootMenu<'a> {
    /// Create new boot menu
    pub fn new(console: &'a Console, config: &'a BootConfig) -> Self {
        Self {
            console,
            config,
            selected: config.default_entry,
            timeout: config.timeout * 10, // Convert to deciseconds
            visible: config.timeout > 0,
            editor_active: false,
            cmdline_buffer: [0u8; 256],
            cmdline_len: 0,
        }
    }

    /// Run boot menu
    pub fn run(&mut self) -> MenuResult {
        if !self.visible {
            return MenuResult::Continue;
        }

        self.draw();

        loop {
            // Check for key press
            if let Some(key) = self.console.read_key() {
                self.timeout = 0; // Cancel timeout on any key

                match self.handle_key(key) {
                    Some(result) => return result,
                    None => self.draw(),
                }
            }

            // Handle timeout
            if self.timeout > 0 {
                self.timeout -= 1;
                if self.timeout == 0 {
                    return MenuResult::Timeout;
                }

                // Redraw timeout counter every second
                if self.timeout % 10 == 0 {
                    self.draw_timeout();
                }
            }

            // Small delay
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
    }

    /// Handle key press
    fn handle_key(&mut self, key: Key) -> Option<MenuResult> {
        if self.editor_active {
            return self.handle_editor_key(key);
        }

        match key {
            Key::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            Key::Down => {
                let visible_count = self.config.visible_entries().count();
                if self.selected < visible_count.saturating_sub(1) {
                    self.selected += 1;
                }
            }
            Key::Enter => {
                return Some(MenuResult::Boot(self.selected));
            }
            Key::Char('e') | Key::Char('E') => {
                self.enter_editor();
            }
            Key::Char('c') | Key::Char('C') => {
                return Some(MenuResult::Shell);
            }
            Key::Char('r') | Key::Char('R') => {
                return Some(MenuResult::Reboot);
            }
            Key::Char('s') | Key::Char('S') => {
                return Some(MenuResult::Shutdown);
            }
            Key::Escape => {
                return Some(MenuResult::Continue);
            }
            _ => {}
        }

        None
    }

    /// Handle editor key press
    fn handle_editor_key(&mut self, key: Key) -> Option<MenuResult> {
        match key {
            Key::Escape => {
                self.editor_active = false;
            }
            Key::Enter => {
                self.editor_active = false;
                return Some(MenuResult::Boot(self.selected));
            }
            Key::Backspace => {
                if self.cmdline_len > 0 {
                    self.cmdline_len -= 1;
                }
            }
            Key::Char(c) => {
                if self.cmdline_len < self.cmdline_buffer.len() - 1 {
                    self.cmdline_buffer[self.cmdline_len] = c as u8;
                    self.cmdline_len += 1;
                }
            }
            _ => {}
        }

        None
    }

    /// Enter command line editor
    fn enter_editor(&mut self) {
        self.editor_active = true;

        // Copy current entry's cmdline to buffer
        if let Some(entry) = self.config.entries.get(self.selected) {
            let cmdline = entry.cmdline.as_str().as_bytes();
            let len = cmdline.len().min(self.cmdline_buffer.len() - 1);
            self.cmdline_buffer[..len].copy_from_slice(&cmdline[..len]);
            self.cmdline_len = len;
        }
    }

    /// Draw menu
    fn draw(&self) {
        self.console.clear();

        // Banner
        self.draw_banner();

        // Entries
        self.draw_entries();

        // Help text
        self.draw_help();

        // Timeout
        if self.timeout > 0 {
            self.draw_timeout();
        }

        // Editor
        if self.editor_active {
            self.draw_editor();
        }
    }

    /// Draw banner
    fn draw_banner(&self) {
        self.console.println("");
        self.console.print_colored("  ╔═══════════════════════════════════════════════════════╗\r\n", Color::Cyan);
        self.console.print_colored("  ║", Color::Cyan);
        self.console.print_colored("           Helix UEFI Boot Manager v1.0           ", Color::White);
        self.console.print_colored("║\r\n", Color::Cyan);
        self.console.print_colored("  ╚═══════════════════════════════════════════════════════╝\r\n", Color::Cyan);
        self.console.println("");
    }

    /// Draw entries
    fn draw_entries(&self) {
        let visible: Vec<_> = self.config.entries.iter()
            .filter(|e| !e.hidden)
            .collect();

        for (i, entry) in visible.iter().enumerate() {
            let is_selected = i == self.selected;

            if is_selected {
                self.console.print_colored("  ► ", Color::Yellow);
                self.console.set_attribute(Color::Black, Color::White);
                self.console.print(" ");
                self.console.print(entry.title.as_str());
                self.console.print(" ");
                self.console.set_attribute(Color::White, Color::Black);
            } else {
                self.console.print("    ");
                self.console.print(entry.title.as_str());
            }

            self.console.println("");
        }

        self.console.println("");
    }

    /// Draw help text
    fn draw_help(&self) {
        self.console.println("");
        self.console.print_colored("  ─────────────────────────────────────────────────────────\r\n", Color::DarkGray);
        self.console.print_colored("  ↑↓ ", Color::Yellow);
        self.console.print("Select   ");
        self.console.print_colored("Enter ", Color::Yellow);
        self.console.print("Boot   ");
        self.console.print_colored("e ", Color::Yellow);
        self.console.print("Edit   ");
        self.console.print_colored("c ", Color::Yellow);
        self.console.println("Shell");
        self.console.print_colored("  r ", Color::Yellow);
        self.console.print("Reboot   ");
        self.console.print_colored("s ", Color::Yellow);
        self.console.print("Shutdown   ");
        self.console.print_colored("Esc ", Color::Yellow);
        self.console.println("Continue");
    }

    /// Draw timeout
    fn draw_timeout(&self) {
        let seconds = self.timeout / 10;
        self.console.println("");
        self.console.print("  Booting in ");
        self.console.print_colored(&format_number(seconds), Color::Yellow);
        self.console.println(" seconds...");
    }

    /// Draw command line editor
    fn draw_editor(&self) {
        self.console.println("");
        self.console.print_colored("  Command Line Editor:\r\n", Color::Cyan);
        self.console.print("  > ");

        if let Ok(s) = core::str::from_utf8(&self.cmdline_buffer[..self.cmdline_len]) {
            self.console.print(s);
        }

        self.console.print_colored("█", Color::Yellow);
        self.console.println("");
    }

    /// Get selected entry
    pub fn selected_entry(&self) -> Option<&BootEntry> {
        let visible: Vec<_> = self.config.entries.iter()
            .filter(|e| !e.hidden)
            .collect();

        visible.get(self.selected).map(|e| *e)
    }

    /// Get edited command line
    pub fn edited_cmdline(&self) -> Option<&str> {
        if self.cmdline_len > 0 {
            core::str::from_utf8(&self.cmdline_buffer[..self.cmdline_len]).ok()
        } else {
            None
        }
    }
}

// =============================================================================
// GRAPHICAL MENU
// =============================================================================

/// Graphical boot menu with icons
pub struct GraphicalMenu<'a> {
    /// Framebuffer console
    fb: &'a mut FramebufferConsole,
    /// Configuration
    config: &'a BootConfig,
    /// Selected entry
    selected: usize,
    /// Timeout remaining
    timeout: u32,
    /// Menu box position
    box_x: u32,
    box_y: u32,
    box_width: u32,
    box_height: u32,
}

impl<'a> GraphicalMenu<'a> {
    /// Create new graphical menu
    pub fn new(fb: &'a mut FramebufferConsole, config: &'a BootConfig) -> Self {
        // Calculate centered box
        let box_width = 400;
        let box_height = 300;
        let box_x = 100; // Would be centered based on screen width
        let box_y = 100;

        Self {
            fb,
            config,
            selected: config.default_entry,
            timeout: config.timeout * 10,
            box_x,
            box_y,
            box_width,
            box_height,
        }
    }

    /// Draw menu
    pub fn draw(&mut self) {
        // Clear background
        self.fb.clear();

        // Draw box
        self.draw_box();

        // Draw title
        self.draw_title();

        // Draw entries
        self.draw_entries();

        // Draw footer
        self.draw_footer();
    }

    /// Draw menu box
    fn draw_box(&mut self) {
        // Draw border and background
        // This would use pixel drawing in real implementation
        self.fb.set_colors(0xFF_40_40_40, 0xFF_20_20_20);
    }

    /// Draw title
    fn draw_title(&mut self) {
        self.fb.set_cursor(self.box_x / 8 + 2, self.box_y / 16 + 1);
        self.fb.set_colors(0xFF_00_FF_FF, 0xFF_20_20_20);
        self.fb.print("Helix Boot Manager");
    }

    /// Draw entries
    fn draw_entries(&mut self) {
        let y_start = self.box_y / 16 + 4;

        for (i, entry) in self.config.entries.iter().enumerate() {
            if entry.hidden {
                continue;
            }

            let y = y_start + i as u32;
            self.fb.set_cursor(self.box_x / 8 + 4, y);

            if i == self.selected {
                self.fb.set_colors(0xFF_00_00_00, 0xFF_FF_FF_FF);
                self.fb.print(" ");
                self.fb.print(entry.title.as_str());
                self.fb.print(" ");
                self.fb.set_colors(0xFF_FF_FF_FF, 0xFF_20_20_20);
            } else {
                self.fb.print("  ");
                self.fb.print(entry.title.as_str());
            }
        }
    }

    /// Draw footer
    fn draw_footer(&mut self) {
        let y = self.box_y / 16 + self.box_height / 16 - 2;
        self.fb.set_cursor(self.box_x / 8 + 2, y);
        self.fb.set_colors(0xFF_80_80_80, 0xFF_20_20_20);
        self.fb.print("Use arrows to select, Enter to boot");

        if self.timeout > 0 {
            self.fb.set_cursor(self.box_x / 8 + 2, y + 1);
            self.fb.print("Auto-boot in ");
            let seconds = self.timeout / 10;
            self.fb.print(&format_number(seconds));
            self.fb.print("s");
        }
    }
}

// =============================================================================
// PROGRESS BAR
// =============================================================================

/// Progress bar for boot process
pub struct ProgressBar<'a> {
    console: &'a Console,
    current: u32,
    total: u32,
    width: u32,
    message: &'a str,
}

impl<'a> ProgressBar<'a> {
    /// Create new progress bar
    pub fn new(console: &'a Console, total: u32, width: u32) -> Self {
        Self {
            console,
            current: 0,
            total,
            width,
            message: "",
        }
    }

    /// Set message
    pub fn set_message(&mut self, msg: &'a str) {
        self.message = msg;
        self.draw();
    }

    /// Update progress
    pub fn update(&mut self, current: u32) {
        self.current = current;
        self.draw();
    }

    /// Increment progress
    pub fn increment(&mut self) {
        if self.current < self.total {
            self.current += 1;
            self.draw();
        }
    }

    /// Draw progress bar
    fn draw(&self) {
        let percent = if self.total > 0 {
            (self.current * 100) / self.total
        } else {
            0
        };

        let filled = (self.width * self.current) / self.total.max(1);

        self.console.print("\r  ");
        self.console.print(self.message);
        self.console.print(" [");

        for i in 0..self.width {
            if i < filled {
                self.console.print_colored("█", Color::Green);
            } else {
                self.console.print_colored("░", Color::DarkGray);
            }
        }

        self.console.print("] ");
        self.console.print(&format_number(percent));
        self.console.print("%  ");
    }

    /// Complete and print newline
    pub fn finish(&self) {
        self.console.println("");
    }
}

/// Spinner animation
pub struct Spinner<'a> {
    console: &'a Console,
    frame: usize,
    message: &'a str,
}

impl<'a> Spinner<'a> {
    const FRAMES: &'static [char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

    /// Create new spinner
    pub fn new(console: &'a Console, message: &'a str) -> Self {
        Self {
            console,
            frame: 0,
            message,
        }
    }

    /// Advance spinner
    pub fn spin(&mut self) {
        self.frame = (self.frame + 1) % Self::FRAMES.len();
        self.draw();
    }

    /// Draw spinner
    fn draw(&self) {
        self.console.print("\r  ");
        self.console.print_colored(&char_to_str(Self::FRAMES[self.frame]), Color::Cyan);
        self.console.print(" ");
        self.console.print(self.message);
    }

    /// Complete with success
    pub fn success(&self, msg: &str) {
        self.console.print("\r  ");
        self.console.print_colored("✓", Color::Green);
        self.console.print(" ");
        self.console.println(msg);
    }

    /// Complete with failure
    pub fn fail(&self, msg: &str) {
        self.console.print("\r  ");
        self.console.print_colored("✗", Color::Red);
        self.console.print(" ");
        self.console.println(msg);
    }
}

// =============================================================================
// STATUS LINE
// =============================================================================

/// Boot status line
pub struct StatusLine<'a> {
    console: &'a Console,
    row: usize,
}

impl<'a> StatusLine<'a> {
    /// Create new status line
    pub fn new(console: &'a Console, row: usize) -> Self {
        Self { console, row }
    }

    /// Show status message
    pub fn show(&self, msg: &str) {
        self.console.set_cursor(0, self.row);
        self.console.print("  ");
        self.console.print(msg);
        self.clear_to_end();
    }

    /// Show with icon
    pub fn show_with_icon(&self, icon: StatusIcon, msg: &str) {
        self.console.set_cursor(0, self.row);
        self.console.print("  ");

        match icon {
            StatusIcon::Info => self.console.print_colored("ℹ", Color::Cyan),
            StatusIcon::Success => self.console.print_colored("✓", Color::Green),
            StatusIcon::Warning => self.console.print_colored("⚠", Color::Yellow),
            StatusIcon::Error => self.console.print_colored("✗", Color::Red),
            StatusIcon::Loading => self.console.print_colored("◌", Color::LightGray),
        }

        self.console.print(" ");
        self.console.print(msg);
        self.clear_to_end();
    }

    /// Clear line to end
    fn clear_to_end(&self) {
        // Print spaces to clear rest of line
        for _ in 0..40 {
            self.console.print(" ");
        }
    }

    /// Clear status line
    pub fn clear(&self) {
        self.console.set_cursor(0, self.row);
        for _ in 0..80 {
            self.console.print(" ");
        }
    }
}

/// Status icon type
#[derive(Debug, Clone, Copy)]
pub enum StatusIcon {
    Info,
    Success,
    Warning,
    Error,
    Loading,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Format number as string
fn format_number(n: u32) -> &'static str {
    // Very simplified - return static strings
    match n {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        _ => "?",
    }
}

/// Convert char to static str
fn char_to_str(c: char) -> &'static str {
    match c {
        '⠋' => "⠋",
        '⠙' => "⠙",
        '⠹' => "⠹",
        '⠸' => "⠸",
        '⠼' => "⠼",
        '⠴' => "⠴",
        '⠦' => "⠦",
        '⠧' => "⠧",
        '⠇' => "⠇",
        '⠏' => "⠏",
        _ => " ",
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(5), "5");
        assert_eq!(format_number(10), "10");
    }

    #[test]
    fn test_menu_result() {
        let result = MenuResult::Boot(0);
        assert!(matches!(result, MenuResult::Boot(0)));
    }
}
