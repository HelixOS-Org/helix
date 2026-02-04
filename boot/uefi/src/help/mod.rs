//! Help and Documentation System
//!
//! This module provides help documentation, keyboard shortcuts,
//! and user guidance for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - Contextual help
//! - Keyboard shortcut reference
//! - Command documentation
//! - Interactive help browser
//! - Quick tips and hints

use core::fmt;

// =============================================================================
// HELP CATEGORIES
// =============================================================================

/// Help category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum HelpCategory {
    /// General help
    #[default]
    General         = 0,
    /// Navigation
    Navigation      = 1,
    /// Boot options
    BootOptions     = 2,
    /// Keyboard shortcuts
    Shortcuts       = 3,
    /// Boot entries
    Entries         = 4,
    /// Configuration
    Configuration   = 5,
    /// Troubleshooting
    Troubleshooting = 6,
    /// Advanced
    Advanced        = 7,
    /// Security
    Security        = 8,
    /// Recovery
    Recovery        = 9,
}

impl fmt::Display for HelpCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HelpCategory::General => write!(f, "General"),
            HelpCategory::Navigation => write!(f, "Navigation"),
            HelpCategory::BootOptions => write!(f, "Boot Options"),
            HelpCategory::Shortcuts => write!(f, "Keyboard Shortcuts"),
            HelpCategory::Entries => write!(f, "Boot Entries"),
            HelpCategory::Configuration => write!(f, "Configuration"),
            HelpCategory::Troubleshooting => write!(f, "Troubleshooting"),
            HelpCategory::Advanced => write!(f, "Advanced"),
            HelpCategory::Security => write!(f, "Security"),
            HelpCategory::Recovery => write!(f, "Recovery"),
        }
    }
}

/// Help category count
pub const HELP_CATEGORY_COUNT: usize = 10;

/// All help categories
pub const HELP_CATEGORIES: [HelpCategory; HELP_CATEGORY_COUNT] = [
    HelpCategory::General,
    HelpCategory::Navigation,
    HelpCategory::BootOptions,
    HelpCategory::Shortcuts,
    HelpCategory::Entries,
    HelpCategory::Configuration,
    HelpCategory::Troubleshooting,
    HelpCategory::Advanced,
    HelpCategory::Security,
    HelpCategory::Recovery,
];

// =============================================================================
// KEYBOARD SHORTCUTS
// =============================================================================

/// Key modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifier(u8);

impl KeyModifier {
    pub const NONE: KeyModifier = KeyModifier(0);
    pub const SHIFT: KeyModifier = KeyModifier(1 << 0);
    pub const CTRL: KeyModifier = KeyModifier(1 << 1);
    pub const ALT: KeyModifier = KeyModifier(1 << 2);

    /// Check if shift is pressed
    pub const fn shift(&self) -> bool {
        self.0 & Self::SHIFT.0 != 0
    }

    /// Check if ctrl is pressed
    pub const fn ctrl(&self) -> bool {
        self.0 & Self::CTRL.0 != 0
    }

    /// Check if alt is pressed
    pub const fn alt(&self) -> bool {
        self.0 & Self::ALT.0 != 0
    }

    /// Combine modifiers
    pub const fn combine(self, other: KeyModifier) -> KeyModifier {
        KeyModifier(self.0 | other.0)
    }
}

/// Key code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u16)]
pub enum Key {
    #[default]
    None      = 0,
    // Function keys (UEFI scan codes)
    F1        = 0x0B,
    F2        = 0x0C,
    F3        = 0x100, // Use extended code to avoid conflict with Enter
    F4        = 0x0E,
    F5        = 0x0F,
    F6        = 0x10,
    F7        = 0x11,
    F8        = 0x12,
    F9        = 0x13,
    F10       = 0x14,
    F11       = 0x15,
    F12       = 0x16,
    // Navigation
    Up        = 0x01,
    Down      = 0x02,
    Left      = 0x03,
    Right     = 0x04,
    Home      = 0x05,
    End       = 0x06,
    PageUp    = 0x07,
    PageDown  = 0x101, // Use extended code to avoid conflict with Backspace
    // Action
    Enter     = 0x0D,
    Escape    = 0x1B,
    Tab       = 0x09,
    Space     = 0x20,
    Backspace = 0x08,
    Delete    = 0x7F,
    Insert    = 0x102, // Use extended code to avoid conflict with Tab
    // Letters (for shortcuts)
    A         = 0x41,
    B         = 0x42,
    C         = 0x43,
    D         = 0x44,
    E         = 0x45,
    R         = 0x52,
    S         = 0x53,
    Q         = 0x51,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::None => write!(f, ""),
            Key::F1 => write!(f, "F1"),
            Key::F2 => write!(f, "F2"),
            Key::F3 => write!(f, "F3"),
            Key::F4 => write!(f, "F4"),
            Key::F5 => write!(f, "F5"),
            Key::F6 => write!(f, "F6"),
            Key::F7 => write!(f, "F7"),
            Key::F8 => write!(f, "F8"),
            Key::F9 => write!(f, "F9"),
            Key::F10 => write!(f, "F10"),
            Key::F11 => write!(f, "F11"),
            Key::F12 => write!(f, "F12"),
            Key::Up => write!(f, "↑"),
            Key::Down => write!(f, "↓"),
            Key::Left => write!(f, "←"),
            Key::Right => write!(f, "→"),
            Key::Home => write!(f, "Home"),
            Key::End => write!(f, "End"),
            Key::PageUp => write!(f, "PgUp"),
            Key::PageDown => write!(f, "PgDn"),
            Key::Enter => write!(f, "Enter"),
            Key::Escape => write!(f, "Esc"),
            Key::Tab => write!(f, "Tab"),
            Key::Space => write!(f, "Space"),
            Key::Backspace => write!(f, "Backspace"),
            Key::Delete => write!(f, "Del"),
            Key::Insert => write!(f, "Ins"),
            Key::A => write!(f, "A"),
            Key::B => write!(f, "B"),
            Key::C => write!(f, "C"),
            Key::D => write!(f, "D"),
            Key::E => write!(f, "E"),
            Key::R => write!(f, "R"),
            Key::S => write!(f, "S"),
            Key::Q => write!(f, "Q"),
        }
    }
}

/// Keyboard shortcut
#[derive(Debug, Clone, Copy, Default)]
pub struct Shortcut {
    /// Key
    pub key: Key,
    /// Modifier
    pub modifier: KeyModifier,
}

impl Shortcut {
    /// Create shortcut
    pub const fn new(key: Key, modifier: KeyModifier) -> Self {
        Self { key, modifier }
    }

    /// Create shortcut without modifier
    pub const fn key(key: Key) -> Self {
        Self {
            key,
            modifier: KeyModifier::NONE,
        }
    }

    /// Create with shift
    pub const fn shift(key: Key) -> Self {
        Self {
            key,
            modifier: KeyModifier::SHIFT,
        }
    }

    /// Create with ctrl
    pub const fn ctrl(key: Key) -> Self {
        Self {
            key,
            modifier: KeyModifier::CTRL,
        }
    }

    /// Create with alt
    pub const fn alt(key: Key) -> Self {
        Self {
            key,
            modifier: KeyModifier::ALT,
        }
    }
}

// =============================================================================
// SHORTCUT ENTRIES
// =============================================================================

/// Shortcut documentation entry
#[derive(Debug, Clone, Copy)]
pub struct ShortcutEntry {
    /// Shortcut
    pub shortcut: Shortcut,
    /// Action description
    pub action: &'static str,
    /// Category
    pub category: HelpCategory,
}

/// Navigation shortcuts
pub const NAV_SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        shortcut: Shortcut::key(Key::Up),
        action: "Move selection up",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::Down),
        action: "Move selection down",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::PageUp),
        action: "Page up",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::PageDown),
        action: "Page down",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::Home),
        action: "Go to first entry",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::End),
        action: "Go to last entry",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::Enter),
        action: "Boot selected entry",
        category: HelpCategory::Navigation,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::Escape),
        action: "Cancel / Go back",
        category: HelpCategory::Navigation,
    },
];

/// Boot shortcuts
pub const BOOT_SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        shortcut: Shortcut::key(Key::Enter),
        action: "Boot selected entry",
        category: HelpCategory::BootOptions,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::E),
        action: "Edit boot parameters",
        category: HelpCategory::BootOptions,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::D),
        action: "Set as default boot entry",
        category: HelpCategory::BootOptions,
    },
    ShortcutEntry {
        shortcut: Shortcut::ctrl(Key::R),
        action: "Recovery mode",
        category: HelpCategory::BootOptions,
    },
    ShortcutEntry {
        shortcut: Shortcut::ctrl(Key::S),
        action: "Safe mode",
        category: HelpCategory::BootOptions,
    },
];

/// Function key shortcuts
pub const FUNCTION_SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F1),
        action: "Show help",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F2),
        action: "Edit configuration",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F3),
        action: "Command line mode",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F4),
        action: "System information",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F5),
        action: "Refresh / Rescan",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F6),
        action: "Security options",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F7),
        action: "Network boot",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F8),
        action: "Boot options menu",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F9),
        action: "Setup utility",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F10),
        action: "Save and exit",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F11),
        action: "Boot device menu",
        category: HelpCategory::Shortcuts,
    },
    ShortcutEntry {
        shortcut: Shortcut::key(Key::F12),
        action: "Firmware setup",
        category: HelpCategory::Shortcuts,
    },
];

// =============================================================================
// HELP TOPICS
// =============================================================================

/// Help topic
#[derive(Debug, Clone, Copy)]
pub struct HelpTopic {
    /// Topic ID
    pub id: u16,
    /// Title
    pub title: &'static str,
    /// Content
    pub content: &'static str,
    /// Category
    pub category: HelpCategory,
    /// Related topics
    pub related: &'static [u16],
}

/// General help topics
pub const GENERAL_TOPICS: &[HelpTopic] = &[
    HelpTopic {
        id: 1,
        title: "Welcome to Helix Bootloader",
        content: "Helix is a modern UEFI bootloader for the Helix OS Framework.\n\n\
                  Use the arrow keys to navigate the menu and press Enter to boot \
                  the selected operating system.\n\n\
                  Press F1 at any time to access this help system.",
        category: HelpCategory::General,
        related: &[2, 3, 4],
    },
    HelpTopic {
        id: 2,
        title: "Quick Start",
        content: "To boot your operating system:\n\n\
                  1. Use ↑/↓ arrows to select an entry\n\
                  2. Press Enter to boot\n\n\
                  The selected entry will boot automatically after the timeout \
                  expires. Press any key to stop the countdown.\n\n\
                  For advanced options, press F8 or select 'Advanced Options'.",
        category: HelpCategory::General,
        related: &[1, 5],
    },
    HelpTopic {
        id: 3,
        title: "About Helix",
        content: "Helix Bootloader v1.0\n\n\
                  A modern, secure UEFI bootloader designed for the Helix OS Framework.\n\n\
                  Features:\n\
                  • Secure Boot support\n\
                  • Multiple kernel support\n\
                  • Boot parameter editing\n\
                  • System recovery\n\
                  • Network boot\n\
                  • Graphical interface",
        category: HelpCategory::General,
        related: &[1],
    },
];

/// Navigation help topics
pub const NAVIGATION_TOPICS: &[HelpTopic] = &[
    HelpTopic {
        id: 10,
        title: "Menu Navigation",
        content: "Use these keys to navigate the boot menu:\n\n\
                  ↑/↓ - Move selection up/down\n\
                  Page Up/Down - Jump by page\n\
                  Home/End - Jump to first/last entry\n\
                  Enter - Boot selected entry\n\
                  Escape - Cancel or go back\n\
                  Tab - Switch between panels",
        category: HelpCategory::Navigation,
        related: &[11, 12],
    },
    HelpTopic {
        id: 11,
        title: "Submenu Navigation",
        content: "When in a submenu:\n\n\
                  • Press Enter to select an option\n\
                  • Press Escape to return to the previous menu\n\
                  • Press Left/Right to expand/collapse items\n\
                  • Press Space to toggle options",
        category: HelpCategory::Navigation,
        related: &[10],
    },
    HelpTopic {
        id: 12,
        title: "Help Navigation",
        content: "Within the help system:\n\n\
                  • Use arrows to scroll content\n\
                  • Press Tab to switch between topics and content\n\
                  • Press a number to jump to related topics\n\
                  • Press Escape to close help",
        category: HelpCategory::Navigation,
        related: &[10],
    },
];

/// Boot options topics
pub const BOOT_TOPICS: &[HelpTopic] = &[
    HelpTopic {
        id: 20,
        title: "Boot Options",
        content: "Available boot options:\n\n\
                  Normal Boot - Standard boot with default parameters\n\
                  Safe Mode - Boot with minimal drivers\n\
                  Recovery - Access recovery environment\n\
                  Debug - Enable kernel debugging\n\
                  Single User - Boot to single user mode\n\
                  Verbose - Show detailed boot messages",
        category: HelpCategory::BootOptions,
        related: &[21, 22, 23],
    },
    HelpTopic {
        id: 21,
        title: "Edit Boot Parameters",
        content: "Press 'e' to edit boot parameters for the selected entry.\n\n\
                  You can modify:\n\
                  • Kernel command line\n\
                  • Root device\n\
                  • Init process\n\
                  • Module loading\n\n\
                  Changes are temporary unless saved to configuration.\n\
                  Press Ctrl+X to boot with changes, Escape to cancel.",
        category: HelpCategory::BootOptions,
        related: &[20],
    },
    HelpTopic {
        id: 22,
        title: "Safe Mode",
        content: "Safe Mode boots with minimal drivers and services.\n\n\
                  Use Safe Mode when:\n\
                  • The system won't boot normally\n\
                  • You need to troubleshoot driver issues\n\
                  • You want to remove problematic software\n\n\
                  To enter Safe Mode, press Ctrl+S or select from boot options.",
        category: HelpCategory::BootOptions,
        related: &[20, 23],
    },
    HelpTopic {
        id: 23,
        title: "Recovery Mode",
        content: "Recovery Mode provides system recovery tools.\n\n\
                  Available tools:\n\
                  • System restore\n\
                  • Disk repair\n\
                  • Boot repair\n\
                  • Command line\n\
                  • Memory test\n\n\
                  To enter Recovery Mode, press Ctrl+R or select from boot options.",
        category: HelpCategory::BootOptions,
        related: &[20, 22],
    },
];

/// Troubleshooting topics
pub const TROUBLESHOOTING_TOPICS: &[HelpTopic] = &[
    HelpTopic {
        id: 50,
        title: "Boot Problems",
        content: "If your system won't boot:\n\n\
                  1. Try Safe Mode (Ctrl+S)\n\
                  2. Use Recovery Mode (Ctrl+R)\n\
                  3. Check boot parameters (press 'e')\n\
                  4. Verify disk is accessible (F4)\n\
                  5. Check for hardware errors\n\n\
                  If the bootloader itself fails, you may need to \
                  reinstall from recovery media.",
        category: HelpCategory::Troubleshooting,
        related: &[22, 23, 51],
    },
    HelpTopic {
        id: 51,
        title: "Missing Boot Entries",
        content: "If boot entries are missing:\n\n\
                  1. Press F5 to rescan devices\n\
                  2. Check if the disk is connected\n\
                  3. Verify filesystem is supported\n\
                  4. Check configuration file syntax\n\
                  5. Try automatic detection\n\n\
                  Use F3 for command line to manually add entries.",
        category: HelpCategory::Troubleshooting,
        related: &[50, 52],
    },
    HelpTopic {
        id: 52,
        title: "Timeout Issues",
        content: "If timeout isn't working correctly:\n\n\
                  1. Check timeout value in configuration\n\
                  2. Press any key to stop/reset countdown\n\
                  3. Set timeout to -1 to disable auto-boot\n\
                  4. Set timeout to 0 for instant boot\n\n\
                  Timeout is specified in seconds.",
        category: HelpCategory::Troubleshooting,
        related: &[50],
    },
];

/// Security topics
pub const SECURITY_TOPICS: &[HelpTopic] = &[
    HelpTopic {
        id: 60,
        title: "Secure Boot",
        content: "Secure Boot ensures only trusted code runs at boot time.\n\n\
                  Status: Check F4 for current Secure Boot status\n\n\
                  With Secure Boot enabled:\n\
                  • Only signed bootloaders can run\n\
                  • Only signed kernels can be loaded\n\
                  • Tampered binaries are rejected\n\n\
                  Helix supports Secure Boot with Microsoft or custom keys.",
        category: HelpCategory::Security,
        related: &[61, 62],
    },
    HelpTopic {
        id: 61,
        title: "Password Protection",
        content: "The bootloader can be password protected.\n\n\
                  Protection levels:\n\
                  • None - No password required\n\
                  • Edit - Password required to edit\n\
                  • Full - Password required for all actions\n\n\
                  Set password in configuration file using 'password' directive.",
        category: HelpCategory::Security,
        related: &[60],
    },
    HelpTopic {
        id: 62,
        title: "Trusted Boot",
        content: "Trusted Boot measures each boot component.\n\n\
                  Measurements are stored in TPM PCRs:\n\
                  • PCR 4: Bootloader code\n\
                  • PCR 5: Configuration\n\
                  • PCR 8: Kernel code\n\
                  • PCR 9: Kernel parameters\n\n\
                  These can be used for disk encryption (TPM sealing) \
                  and remote attestation.",
        category: HelpCategory::Security,
        related: &[60],
    },
];

// =============================================================================
// TIPS AND HINTS
// =============================================================================

/// Tip priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[repr(u8)]
pub enum TipPriority {
    /// Low priority
    Low      = 0,
    /// Normal priority
    #[default]
    Normal   = 1,
    /// High priority
    High     = 2,
    /// Critical
    Critical = 3,
}

/// Contextual tip
#[derive(Debug, Clone, Copy)]
pub struct Tip {
    /// Tip ID
    pub id: u16,
    /// Message
    pub message: &'static str,
    /// Priority
    pub priority: TipPriority,
    /// Context (when to show)
    pub context: TipContext,
}

/// Tip context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TipContext {
    /// Always show
    #[default]
    Always,
    /// On first boot
    FirstBoot,
    /// In menu
    InMenu,
    /// When editing
    Editing,
    /// On error
    OnError,
    /// On timeout
    OnTimeout,
    /// In help
    InHelp,
}

/// Standard tips
pub const STANDARD_TIPS: &[Tip] = &[
    Tip {
        id: 1,
        message: "Press F1 for help at any time",
        priority: TipPriority::Normal,
        context: TipContext::InMenu,
    },
    Tip {
        id: 2,
        message: "Press any key to stop the countdown",
        priority: TipPriority::Normal,
        context: TipContext::OnTimeout,
    },
    Tip {
        id: 3,
        message: "Use arrow keys to navigate, Enter to select",
        priority: TipPriority::Normal,
        context: TipContext::FirstBoot,
    },
    Tip {
        id: 4,
        message: "Press 'e' to edit boot parameters",
        priority: TipPriority::Low,
        context: TipContext::InMenu,
    },
    Tip {
        id: 5,
        message: "Press 'd' to set the selected entry as default",
        priority: TipPriority::Low,
        context: TipContext::InMenu,
    },
    Tip {
        id: 6,
        message: "Try Safe Mode (Ctrl+S) if normal boot fails",
        priority: TipPriority::High,
        context: TipContext::OnError,
    },
    Tip {
        id: 7,
        message: "Press F4 to view system information",
        priority: TipPriority::Low,
        context: TipContext::InMenu,
    },
    Tip {
        id: 8,
        message: "Press F5 to rescan for boot entries",
        priority: TipPriority::Normal,
        context: TipContext::InMenu,
    },
    Tip {
        id: 9,
        message: "Use Tab to switch between topics and content",
        priority: TipPriority::Normal,
        context: TipContext::InHelp,
    },
    Tip {
        id: 10,
        message: "Press Escape to close dialogs or go back",
        priority: TipPriority::Normal,
        context: TipContext::Always,
    },
];

// =============================================================================
// HELP BROWSER STATE
// =============================================================================

/// Maximum visible topics in list
pub const MAX_VISIBLE_TOPICS: usize = 16;

/// Help browser panel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpPanel {
    /// Category list
    #[default]
    Categories,
    /// Topic list
    Topics,
    /// Content view
    Content,
}

/// Help browser state
#[derive(Debug, Clone, Copy)]
pub struct HelpBrowser {
    /// Current panel
    pub panel: HelpPanel,
    /// Selected category
    pub category: HelpCategory,
    /// Category selection index
    pub category_index: usize,
    /// Topic selection index
    pub topic_index: usize,
    /// Content scroll offset
    pub scroll_offset: usize,
    /// Search query (simplified)
    pub searching: bool,
    /// History stack
    pub history: [u16; 8],
    /// History depth
    pub history_depth: usize,
}

impl Default for HelpBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpBrowser {
    /// Create new help browser
    pub const fn new() -> Self {
        Self {
            panel: HelpPanel::Categories,
            category: HelpCategory::General,
            category_index: 0,
            topic_index: 0,
            scroll_offset: 0,
            searching: false,
            history: [0; 8],
            history_depth: 0,
        }
    }

    /// Reset browser
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Navigate to category
    pub fn select_category(&mut self, index: usize) {
        if index < HELP_CATEGORY_COUNT {
            self.category_index = index;
            self.category = HELP_CATEGORIES[index];
            self.topic_index = 0;
            self.panel = HelpPanel::Topics;
        }
    }

    /// Navigate to topic by ID
    pub fn goto_topic(&mut self, topic_id: u16) {
        // Push current to history
        if self.history_depth < 8 {
            self.history[self.history_depth] = self.current_topic_id();
            self.history_depth += 1;
        }

        // Find topic and navigate
        // (In real implementation, would search topic tables)
        self.scroll_offset = 0;
        self.panel = HelpPanel::Content;
    }

    /// Go back in history
    pub fn go_back(&mut self) -> bool {
        if self.history_depth > 0 {
            self.history_depth -= 1;
            // Would navigate to history[history_depth]
            true
        } else {
            // Return to topic list
            if self.panel == HelpPanel::Content {
                self.panel = HelpPanel::Topics;
                true
            } else if self.panel == HelpPanel::Topics {
                self.panel = HelpPanel::Categories;
                true
            } else {
                false
            }
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        match self.panel {
            HelpPanel::Categories => {
                if self.category_index > 0 {
                    self.category_index -= 1;
                }
            },
            HelpPanel::Topics => {
                if self.topic_index > 0 {
                    self.topic_index -= 1;
                }
            },
            HelpPanel::Content => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            },
        }
    }

    /// Move selection down
    pub fn move_down(&mut self, max_topics: usize, max_lines: usize) {
        match self.panel {
            HelpPanel::Categories => {
                if self.category_index < HELP_CATEGORY_COUNT - 1 {
                    self.category_index += 1;
                }
            },
            HelpPanel::Topics => {
                if self.topic_index < max_topics.saturating_sub(1) {
                    self.topic_index += 1;
                }
            },
            HelpPanel::Content => {
                if self.scroll_offset < max_lines.saturating_sub(1) {
                    self.scroll_offset += 1;
                }
            },
        }
    }

    /// Select current item
    pub fn select(&mut self) {
        match self.panel {
            HelpPanel::Categories => {
                self.category = HELP_CATEGORIES[self.category_index];
                self.topic_index = 0;
                self.panel = HelpPanel::Topics;
            },
            HelpPanel::Topics => {
                self.scroll_offset = 0;
                self.panel = HelpPanel::Content;
            },
            HelpPanel::Content => {
                // No action
            },
        }
    }

    /// Get current topic ID (placeholder)
    fn current_topic_id(&self) -> u16 {
        // Would return actual topic ID
        (self.category as u16) * 10 + self.topic_index as u16
    }

    /// Switch panel
    pub fn switch_panel(&mut self) {
        self.panel = match self.panel {
            HelpPanel::Categories => HelpPanel::Topics,
            HelpPanel::Topics => HelpPanel::Content,
            HelpPanel::Content => HelpPanel::Categories,
        };
    }
}

// =============================================================================
// QUICK REFERENCE
// =============================================================================

/// Quick reference card
#[derive(Debug, Clone, Copy)]
pub struct QuickReference {
    /// Title
    pub title: &'static str,
    /// Entries
    pub entries: &'static [(&'static str, &'static str)],
}

/// Navigation quick reference
pub const NAV_QUICK_REF: QuickReference = QuickReference {
    title: "Navigation",
    entries: &[
        ("↑/↓", "Move selection"),
        ("Enter", "Select/Boot"),
        ("Escape", "Cancel/Back"),
        ("Tab", "Switch panel"),
        ("PgUp/PgDn", "Page scroll"),
    ],
};

/// Function keys quick reference
pub const FKEY_QUICK_REF: QuickReference = QuickReference {
    title: "Function Keys",
    entries: &[
        ("F1", "Help"),
        ("F2", "Edit config"),
        ("F3", "Command line"),
        ("F4", "System info"),
        ("F5", "Refresh"),
        ("F8", "Boot options"),
    ],
};

/// Boot quick reference
pub const BOOT_QUICK_REF: QuickReference = QuickReference {
    title: "Boot Actions",
    entries: &[
        ("e", "Edit parameters"),
        ("d", "Set default"),
        ("Ctrl+S", "Safe mode"),
        ("Ctrl+R", "Recovery"),
    ],
};

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_help_category() {
        assert_eq!(HelpCategory::General as u8, 0);
        assert_eq!(HELP_CATEGORY_COUNT, 10);
    }

    #[test]
    fn test_key_modifier() {
        let mods = KeyModifier::SHIFT.combine(KeyModifier::CTRL);
        assert!(mods.shift());
        assert!(mods.ctrl());
        assert!(!mods.alt());
    }

    #[test]
    fn test_shortcut() {
        let s = Shortcut::ctrl(Key::R);
        assert_eq!(s.key, Key::R);
        assert!(s.modifier.ctrl());
    }

    #[test]
    fn test_help_browser() {
        let mut browser = HelpBrowser::new();

        assert_eq!(browser.panel, HelpPanel::Categories);

        browser.select();
        assert_eq!(browser.panel, HelpPanel::Topics);

        browser.select();
        assert_eq!(browser.panel, HelpPanel::Content);

        browser.go_back();
        assert_eq!(browser.panel, HelpPanel::Topics);
    }

    #[test]
    fn test_help_browser_navigation() {
        let mut browser = HelpBrowser::new();

        browser.move_down(10, 100);
        assert_eq!(browser.category_index, 1);

        browser.move_up();
        assert_eq!(browser.category_index, 0);
    }

    #[test]
    fn test_tips() {
        assert!(!STANDARD_TIPS.is_empty());
        assert!(STANDARD_TIPS.len() >= 10);
    }

    #[test]
    fn test_shortcuts() {
        assert!(!NAV_SHORTCUTS.is_empty());
        assert!(!BOOT_SHORTCUTS.is_empty());
        assert!(!FUNCTION_SHORTCUTS.is_empty());
    }
}
