//! Command and Action System
//!
//! This module provides a comprehensive command system for user interactions,
//! keyboard shortcuts, boot operations, and scriptable actions.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                       Command System                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    Input Sources                                 │   │
//! │  │  Keyboard │ Menu │ Console │ Script │ Timer │ External          │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │                              ▼                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Command Dispatcher                             │   │
//! │  │  Parse → Validate → Queue → Execute → Report                    │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │                              ▼                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Action Handlers                                │   │
//! │  │  Boot │ Menu │ Config │ System │ Debug │ Custom                 │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// COMMAND CATEGORIES
// =============================================================================

/// Command category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// Boot-related commands
    Boot,
    /// Menu navigation
    Menu,
    /// Configuration commands
    Config,
    /// System commands
    System,
    /// Debug commands
    Debug,
    /// Network commands
    Network,
    /// Security commands
    Security,
    /// Custom/user commands
    Custom,
}

impl Default for CommandCategory {
    fn default() -> Self {
        Self::System
    }
}

impl fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boot => write!(f, "Boot"),
            Self::Menu => write!(f, "Menu"),
            Self::Config => write!(f, "Configuration"),
            Self::System => write!(f, "System"),
            Self::Debug => write!(f, "Debug"),
            Self::Network => write!(f, "Network"),
            Self::Security => write!(f, "Security"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

// =============================================================================
// COMMAND ID
// =============================================================================

/// Command identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(pub u16);

impl CommandId {
    /// Create new command ID
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CMD_{:04X}", self.0)
    }
}

/// Standard command IDs
pub mod cmd_ids {
    use super::CommandId;

    // Boot commands (0x0001-0x00FF)
    /// Boot the default entry.
    pub const BOOT_DEFAULT: CommandId = CommandId::new(0x0001);
    /// Boot the currently selected entry.
    pub const BOOT_SELECTED: CommandId = CommandId::new(0x0002);
    /// Boot a specific entry by index.
    pub const BOOT_ENTRY: CommandId = CommandId::new(0x0003);
    /// Boot with custom parameters.
    pub const BOOT_CUSTOM: CommandId = CommandId::new(0x0004);
    /// Boot the last used entry.
    pub const BOOT_LAST: CommandId = CommandId::new(0x0005);
    /// Boot once without saving preference.
    pub const BOOT_ONCE: CommandId = CommandId::new(0x0006);
    /// Boot from PXE network.
    pub const BOOT_PXE: CommandId = CommandId::new(0x0010);
    /// Boot from CD/DVD.
    pub const BOOT_CD: CommandId = CommandId::new(0x0011);
    /// Boot from USB device.
    pub const BOOT_USB: CommandId = CommandId::new(0x0012);
    /// Boot into recovery mode.
    pub const BOOT_RECOVERY: CommandId = CommandId::new(0x0020);
    /// Boot into safe mode.
    pub const BOOT_SAFE: CommandId = CommandId::new(0x0021);
    /// Cancel pending boot operation.
    pub const CANCEL_BOOT: CommandId = CommandId::new(0x0030);

    // Menu commands (0x0100-0x01FF)
    /// Navigate menu up.
    pub const MENU_UP: CommandId = CommandId::new(0x0100);
    /// Navigate menu down.
    pub const MENU_DOWN: CommandId = CommandId::new(0x0101);
    /// Navigate menu left.
    pub const MENU_LEFT: CommandId = CommandId::new(0x0102);
    /// Navigate menu right.
    pub const MENU_RIGHT: CommandId = CommandId::new(0x0103);
    /// Select current menu item.
    pub const MENU_SELECT: CommandId = CommandId::new(0x0104);
    /// Go back in menu.
    pub const MENU_BACK: CommandId = CommandId::new(0x0105);
    /// Go to menu home.
    pub const MENU_HOME: CommandId = CommandId::new(0x0106);
    /// Go to menu end.
    pub const MENU_END: CommandId = CommandId::new(0x0107);
    /// Page up in menu.
    pub const MENU_PAGE_UP: CommandId = CommandId::new(0x0108);
    /// Page down in menu.
    pub const MENU_PAGE_DOWN: CommandId = CommandId::new(0x0109);
    /// Refresh menu display.
    pub const MENU_REFRESH: CommandId = CommandId::new(0x0110);
    /// Search in menu.
    pub const MENU_SEARCH: CommandId = CommandId::new(0x0111);
    /// Filter menu items.
    pub const MENU_FILTER: CommandId = CommandId::new(0x0112);
    /// Sort menu items.
    pub const MENU_SORT: CommandId = CommandId::new(0x0113);
    /// Show help.
    pub const MENU_HELP: CommandId = CommandId::new(0x0120);
    /// Show about information.
    pub const MENU_ABOUT: CommandId = CommandId::new(0x0121);

    // Config commands (0x0200-0x02FF)
    /// Edit configuration.
    pub const CONFIG_EDIT: CommandId = CommandId::new(0x0200);
    /// Save configuration.
    pub const CONFIG_SAVE: CommandId = CommandId::new(0x0201);
    /// Reset configuration to defaults.
    pub const CONFIG_RESET: CommandId = CommandId::new(0x0202);
    /// Import configuration.
    pub const CONFIG_IMPORT: CommandId = CommandId::new(0x0203);
    /// Export configuration.
    pub const CONFIG_EXPORT: CommandId = CommandId::new(0x0204);
    /// Create new boot entry.
    pub const ENTRY_NEW: CommandId = CommandId::new(0x0210);
    /// Edit boot entry.
    pub const ENTRY_EDIT: CommandId = CommandId::new(0x0211);
    /// Delete boot entry.
    pub const ENTRY_DELETE: CommandId = CommandId::new(0x0212);
    /// Copy boot entry.
    pub const ENTRY_COPY: CommandId = CommandId::new(0x0213);
    /// Move entry up in list.
    pub const ENTRY_MOVE_UP: CommandId = CommandId::new(0x0214);
    /// Move entry down in list.
    pub const ENTRY_MOVE_DOWN: CommandId = CommandId::new(0x0215);
    /// Set default boot entry.
    pub const SET_DEFAULT: CommandId = CommandId::new(0x0220);
    /// Set boot timeout.
    pub const SET_TIMEOUT: CommandId = CommandId::new(0x0221);
    /// Set UI theme.
    pub const SET_THEME: CommandId = CommandId::new(0x0222);
    /// Set password protection.
    pub const SET_PASSWORD: CommandId = CommandId::new(0x0230);
    /// Clear password protection.
    pub const CLEAR_PASSWORD: CommandId = CommandId::new(0x0231);

    // System commands (0x0300-0x03FF)
    /// Reboot system.
    pub const REBOOT: CommandId = CommandId::new(0x0300);
    /// Shutdown system.
    pub const SHUTDOWN: CommandId = CommandId::new(0x0301);
    /// Suspend to RAM.
    pub const SUSPEND: CommandId = CommandId::new(0x0302);
    /// Hibernate to disk.
    pub const HIBERNATE: CommandId = CommandId::new(0x0303);
    /// Enter UEFI shell.
    pub const UEFI_SHELL: CommandId = CommandId::new(0x0310);
    /// Enter firmware setup.
    pub const FIRMWARE_SETUP: CommandId = CommandId::new(0x0311);
    /// Exit bootloader.
    pub const EXIT: CommandId = CommandId::new(0x0320);
    /// Quit application.
    pub const QUIT: CommandId = CommandId::new(0x0321);

    // Debug commands (0x0400-0x04FF)
    /// Show debug log.
    pub const DEBUG_LOG: CommandId = CommandId::new(0x0400);
    /// Show debug info.
    pub const DEBUG_INFO: CommandId = CommandId::new(0x0401);
    /// Show memory information.
    pub const DEBUG_MEMORY: CommandId = CommandId::new(0x0402);
    /// Show device list.
    pub const DEBUG_DEVICES: CommandId = CommandId::new(0x0403);
    /// Show UEFI protocols.
    pub const DEBUG_PROTOCOLS: CommandId = CommandId::new(0x0404);
    /// Show UEFI variables.
    pub const DEBUG_VARIABLES: CommandId = CommandId::new(0x0405);
    /// Show memory map.
    pub const DEBUG_MEMMAP: CommandId = CommandId::new(0x0406);
    /// Show ACPI tables.
    pub const DEBUG_ACPI: CommandId = CommandId::new(0x0407);
    /// Show SMBIOS information.
    pub const DEBUG_SMBIOS: CommandId = CommandId::new(0x0408);
    /// Open debug console.
    pub const DEBUG_CONSOLE: CommandId = CommandId::new(0x0410);
    /// Open debug shell.
    pub const DEBUG_SHELL: CommandId = CommandId::new(0x0411);
    /// Set breakpoint.
    pub const DEBUG_BREAK: CommandId = CommandId::new(0x0420);
    /// Step debugger.
    pub const DEBUG_STEP: CommandId = CommandId::new(0x0421);
    /// Continue execution.
    pub const DEBUG_CONTINUE: CommandId = CommandId::new(0x0422);

    // Network commands (0x0500-0x05FF)
    /// Refresh network status.
    pub const NET_REFRESH: CommandId = CommandId::new(0x0500);
    /// Configure network.
    pub const NET_CONFIG: CommandId = CommandId::new(0x0501);
    /// Use DHCP configuration.
    pub const NET_DHCP: CommandId = CommandId::new(0x0502);
    /// Use static IP configuration.
    pub const NET_STATIC: CommandId = CommandId::new(0x0503);
    /// Test network connectivity.
    pub const NET_TEST: CommandId = CommandId::new(0x0510);
    /// Ping network host.
    pub const NET_PING: CommandId = CommandId::new(0x0511);

    // Security commands (0x0600-0x06FF)
    /// Unlock secure boot.
    pub const SEC_UNLOCK: CommandId = CommandId::new(0x0600);
    /// Lock secure boot.
    pub const SEC_LOCK: CommandId = CommandId::new(0x0601);
    /// Verify signature.
    pub const SEC_VERIFY: CommandId = CommandId::new(0x0610);
    /// Enroll security key.
    pub const SEC_ENROLL: CommandId = CommandId::new(0x0611);
    /// Revoke security key.
    pub const SEC_REVOKE: CommandId = CommandId::new(0x0612);
}

// =============================================================================
// KEY BINDINGS
// =============================================================================

/// Key code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCode(pub u16);

impl KeyCode {
    /// Create a new key code from a raw value.
    pub const fn new(code: u16) -> Self {
        Self(code)
    }
}

/// Standard key codes
pub mod keys {
    use super::KeyCode;

    // Special keys
    /// Enter key.
    pub const ENTER: KeyCode = KeyCode::new(0x000D);
    /// Escape key.
    pub const ESCAPE: KeyCode = KeyCode::new(0x001B);
    /// Backspace key.
    pub const BACKSPACE: KeyCode = KeyCode::new(0x0008);
    /// Tab key.
    pub const TAB: KeyCode = KeyCode::new(0x0009);
    /// Space key.
    pub const SPACE: KeyCode = KeyCode::new(0x0020);
    /// Delete key.
    pub const DELETE: KeyCode = KeyCode::new(0x007F);

    // Arrow keys
    /// Up arrow key.
    pub const UP: KeyCode = KeyCode::new(0x0001);
    /// Down arrow key.
    pub const DOWN: KeyCode = KeyCode::new(0x0002);
    /// Left arrow key.
    pub const LEFT: KeyCode = KeyCode::new(0x0003);
    /// Right arrow key.
    pub const RIGHT: KeyCode = KeyCode::new(0x0004);

    // Navigation
    /// Home key.
    pub const HOME: KeyCode = KeyCode::new(0x0005);
    /// End key.
    pub const END: KeyCode = KeyCode::new(0x0006);
    /// Page up key.
    pub const PAGE_UP: KeyCode = KeyCode::new(0x0007);
    /// Page down key.
    pub const PAGE_DOWN: KeyCode = KeyCode::new(0x0008);
    /// Insert key.
    pub const INSERT: KeyCode = KeyCode::new(0x000A);

    // Function keys
    /// F1 function key.
    pub const F1: KeyCode = KeyCode::new(0x0101);
    /// F2 function key.
    pub const F2: KeyCode = KeyCode::new(0x0102);
    /// F3 function key.
    pub const F3: KeyCode = KeyCode::new(0x0103);
    /// F4 function key.
    pub const F4: KeyCode = KeyCode::new(0x0104);
    /// F5 function key.
    pub const F5: KeyCode = KeyCode::new(0x0105);
    /// F6 function key.
    pub const F6: KeyCode = KeyCode::new(0x0106);
    /// F7 function key.
    pub const F7: KeyCode = KeyCode::new(0x0107);
    /// F8 function key.
    pub const F8: KeyCode = KeyCode::new(0x0108);
    /// F9 function key.
    pub const F9: KeyCode = KeyCode::new(0x0109);
    /// F10 function key.
    pub const F10: KeyCode = KeyCode::new(0x010A);
    /// F11 function key.
    pub const F11: KeyCode = KeyCode::new(0x010B);
    /// F12 function key.
    pub const F12: KeyCode = KeyCode::new(0x010C);

    // Printable characters (use ASCII directly)
    /// Create key code from printable character.
    pub const fn char(c: char) -> KeyCode {
        KeyCode::new(c as u16)
    }
}

/// Key modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    /// No modifiers.
    pub const NONE: Self = Self(0);
    /// Shift modifier.
    pub const SHIFT: Self = Self(1);
    /// Ctrl modifier.
    pub const CTRL: Self = Self(2);
    /// Alt modifier.
    pub const ALT: Self = Self(4);
    /// Logo/Super modifier.
    pub const LOGO: Self = Self(8);

    /// Create from raw value
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    /// Get raw value
    pub const fn raw(&self) -> u8 {
        self.0
    }

    /// Check if shift is pressed
    pub const fn shift(&self) -> bool {
        self.0 & 1 != 0
    }

    /// Check if ctrl is pressed
    pub const fn ctrl(&self) -> bool {
        self.0 & 2 != 0
    }

    /// Check if alt is pressed
    pub const fn alt(&self) -> bool {
        self.0 & 4 != 0
    }

    /// Check if logo is pressed
    pub const fn logo(&self) -> bool {
        self.0 & 8 != 0
    }

    /// Combine modifiers
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Key binding
#[derive(Debug, Clone, Copy)]
pub struct KeyBinding {
    /// Key code
    pub key: KeyCode,
    /// Required modifiers
    pub modifiers: KeyModifiers,
    /// Command to execute
    pub command: CommandId,
    /// Description
    pub description: [u8; 32],
    /// Description length
    pub desc_len: usize,
}

impl KeyBinding {
    /// Create simple binding
    pub const fn simple(key: KeyCode, command: CommandId) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::NONE,
            command,
            description: [0u8; 32],
            desc_len: 0,
        }
    }

    /// Create binding with modifiers
    pub const fn with_mod(key: KeyCode, modifiers: KeyModifiers, command: CommandId) -> Self {
        Self {
            key,
            modifiers,
            command,
            description: [0u8; 32],
            desc_len: 0,
        }
    }
}

/// Standard key bindings
pub const DEFAULT_BINDINGS: &[KeyBinding] = &[
    // Navigation
    KeyBinding::simple(keys::UP, cmd_ids::MENU_UP),
    KeyBinding::simple(keys::DOWN, cmd_ids::MENU_DOWN),
    KeyBinding::simple(keys::LEFT, cmd_ids::MENU_LEFT),
    KeyBinding::simple(keys::RIGHT, cmd_ids::MENU_RIGHT),
    KeyBinding::simple(keys::ENTER, cmd_ids::MENU_SELECT),
    KeyBinding::simple(keys::ESCAPE, cmd_ids::MENU_BACK),
    KeyBinding::simple(keys::HOME, cmd_ids::MENU_HOME),
    KeyBinding::simple(keys::END, cmd_ids::MENU_END),
    KeyBinding::simple(keys::PAGE_UP, cmd_ids::MENU_PAGE_UP),
    KeyBinding::simple(keys::PAGE_DOWN, cmd_ids::MENU_PAGE_DOWN),
    // Function keys
    KeyBinding::simple(keys::F1, cmd_ids::MENU_HELP),
    KeyBinding::simple(keys::F2, cmd_ids::ENTRY_EDIT),
    KeyBinding::simple(keys::F3, cmd_ids::MENU_SEARCH),
    KeyBinding::simple(keys::F5, cmd_ids::MENU_REFRESH),
    KeyBinding::simple(keys::F6, cmd_ids::SET_DEFAULT),
    KeyBinding::simple(keys::F10, cmd_ids::CONFIG_SAVE),
    KeyBinding::simple(keys::F12, cmd_ids::DEBUG_CONSOLE),
    // Boot shortcuts
    KeyBinding::simple(keys::char('b'), cmd_ids::BOOT_SELECTED),
    KeyBinding::simple(keys::char('d'), cmd_ids::BOOT_DEFAULT),
    KeyBinding::simple(keys::char('r'), cmd_ids::BOOT_RECOVERY),
    KeyBinding::simple(keys::char('s'), cmd_ids::BOOT_SAFE),
    KeyBinding::simple(keys::char('e'), cmd_ids::ENTRY_EDIT),
    KeyBinding::simple(keys::char('c'), cmd_ids::DEBUG_CONSOLE),
    // Ctrl combinations
    KeyBinding::with_mod(keys::char('q'), KeyModifiers::CTRL, cmd_ids::QUIT),
    KeyBinding::with_mod(keys::char('r'), KeyModifiers::CTRL, cmd_ids::REBOOT),
    KeyBinding::with_mod(keys::char('s'), KeyModifiers::CTRL, cmd_ids::CONFIG_SAVE),
    KeyBinding::with_mod(keys::char('n'), KeyModifiers::CTRL, cmd_ids::ENTRY_NEW),
    KeyBinding::with_mod(keys::char('d'), KeyModifiers::CTRL, cmd_ids::ENTRY_DELETE),
];

// =============================================================================
// COMMAND STRUCTURE
// =============================================================================

/// Command argument
#[derive(Debug, Clone, Copy)]
pub enum CommandArg {
    /// No argument
    None,
    /// Integer argument
    Integer(i64),
    /// Unsigned integer
    Unsigned(u64),
    /// Boolean
    Boolean(bool),
    /// String reference (offset + length)
    String {
        /// Offset into string buffer.
        offset: u16,
        /// Length of string.
        length: u16,
    },
    /// Entry index
    EntryIndex(u16),
    /// Address
    Address(u64),
}

impl Default for CommandArg {
    fn default() -> Self {
        Self::None
    }
}

/// Command flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandFlags(u16);

impl CommandFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Command runs asynchronously.
    pub const ASYNC: Self = Self(1);
    /// Command requires confirmation.
    pub const CONFIRM: Self = Self(2);
    /// Command requires elevated privileges.
    pub const PRIVILEGED: Self = Self(4);
    /// Command can be undone.
    pub const UNDOABLE: Self = Self(8);
    /// Command can be repeated.
    pub const REPEATABLE: Self = Self(16);
    /// Command is hidden from UI.
    pub const HIDDEN: Self = Self(32);
    /// Command is disabled.
    pub const DISABLED: Self = Self(64);

    /// Get raw value
    pub const fn raw(&self) -> u16 {
        self.0
    }

    /// Check flag
    pub const fn has(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    /// Combine flags
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Command to execute
#[derive(Debug, Clone, Copy)]
pub struct Command {
    /// Command ID
    pub id: CommandId,
    /// Category
    pub category: CommandCategory,
    /// Flags
    pub flags: CommandFlags,
    /// Primary argument
    pub arg1: CommandArg,
    /// Secondary argument
    pub arg2: CommandArg,
    /// Timestamp when queued
    pub queued_at: u64,
    /// Source of command
    pub source: CommandSource,
}

impl Default for Command {
    fn default() -> Self {
        Self {
            id: CommandId::new(0),
            category: CommandCategory::System,
            flags: CommandFlags::NONE,
            arg1: CommandArg::None,
            arg2: CommandArg::None,
            queued_at: 0,
            source: CommandSource::Unknown,
        }
    }
}

impl Command {
    /// Create new command
    pub const fn new(id: CommandId, category: CommandCategory) -> Self {
        Self {
            id,
            category,
            flags: CommandFlags::NONE,
            arg1: CommandArg::None,
            arg2: CommandArg::None,
            queued_at: 0,
            source: CommandSource::Unknown,
        }
    }

    /// Create boot command
    pub const fn boot(entry_index: u16) -> Self {
        Self {
            id: cmd_ids::BOOT_ENTRY,
            category: CommandCategory::Boot,
            flags: CommandFlags::NONE,
            arg1: CommandArg::EntryIndex(entry_index),
            arg2: CommandArg::None,
            queued_at: 0,
            source: CommandSource::Unknown,
        }
    }

    /// Check if command requires confirmation
    pub const fn needs_confirm(&self) -> bool {
        self.flags.has(CommandFlags::CONFIRM)
    }

    /// Check if command is privileged
    pub const fn is_privileged(&self) -> bool {
        self.flags.has(CommandFlags::PRIVILEGED)
    }
}

/// Command source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    /// Unknown source
    Unknown,
    /// Keyboard input
    Keyboard,
    /// Menu selection
    Menu,
    /// Console/shell
    Console,
    /// Script
    Script,
    /// Timer event
    Timer,
    /// External request
    External,
    /// System generated
    System,
}

impl Default for CommandSource {
    fn default() -> Self {
        Self::Unknown
    }
}

// =============================================================================
// COMMAND QUEUE
// =============================================================================

/// Maximum queued commands
pub const MAX_COMMAND_QUEUE: usize = 16;

/// Command queue
#[derive(Debug)]
pub struct CommandQueue {
    /// Queued commands
    commands: [Command; MAX_COMMAND_QUEUE],
    /// Read index
    read_idx: usize,
    /// Write index
    write_idx: usize,
    /// Count
    count: usize,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandQueue {
    /// Create new queue
    pub const fn new() -> Self {
        Self {
            commands: [Command {
                id: CommandId(0),
                category: CommandCategory::System,
                flags: CommandFlags::NONE,
                arg1: CommandArg::None,
                arg2: CommandArg::None,
                queued_at: 0,
                source: CommandSource::Unknown,
            }; MAX_COMMAND_QUEUE],
            read_idx: 0,
            write_idx: 0,
            count: 0,
        }
    }

    /// Push command
    pub fn push(&mut self, cmd: Command) -> bool {
        if self.count >= MAX_COMMAND_QUEUE {
            return false;
        }
        self.commands[self.write_idx] = cmd;
        self.write_idx = (self.write_idx + 1) % MAX_COMMAND_QUEUE;
        self.count += 1;
        true
    }

    /// Pop command
    pub fn pop(&mut self) -> Option<Command> {
        if self.count == 0 {
            return None;
        }
        let cmd = self.commands[self.read_idx];
        self.read_idx = (self.read_idx + 1) % MAX_COMMAND_QUEUE;
        self.count -= 1;
        Some(cmd)
    }

    /// Peek next command
    pub const fn peek(&self) -> Option<&Command> {
        if self.count == 0 {
            None
        } else {
            Some(&self.commands[self.read_idx])
        }
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Clear queue
    pub fn clear(&mut self) {
        self.read_idx = 0;
        self.write_idx = 0;
        self.count = 0;
    }

    /// Check for pending commands of category
    pub fn has_pending(&self, category: CommandCategory) -> bool {
        for i in 0..self.count {
            let idx = (self.read_idx + i) % MAX_COMMAND_QUEUE;
            if self.commands[idx].category == category {
                return true;
            }
        }
        false
    }
}

// =============================================================================
// COMMAND RESULT
// =============================================================================

/// Command execution result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    /// Success
    Success,
    /// Success with value
    SuccessValue(u64),
    /// Command not found
    NotFound,
    /// Invalid arguments
    InvalidArgs,
    /// Not permitted
    NotPermitted,
    /// Cancelled by user
    Cancelled,
    /// Already in progress
    InProgress,
    /// Failed with error code
    Failed(u32),
    /// Deferred for later
    Deferred,
}

impl Default for CommandResult {
    fn default() -> Self {
        Self::Success
    }
}

impl CommandResult {
    /// Check if successful
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::SuccessValue(_))
    }

    /// Check if failed
    pub const fn is_error(&self) -> bool {
        !self.is_success() && !matches!(self, Self::Deferred | Self::InProgress)
    }
}

// =============================================================================
// COMMAND HANDLER
// =============================================================================

/// Handler function type
pub type HandlerFn = fn(&Command) -> CommandResult;

/// Command handler registration
#[derive(Debug, Clone, Copy)]
pub struct CommandHandler {
    /// Command ID
    pub command_id: CommandId,
    /// Handler function (as address for `no_std`).
    pub handler_addr: usize,
    /// Handler name
    pub name: [u8; 32],
    /// Name length
    pub name_len: usize,
    /// Required flags
    pub required_flags: CommandFlags,
    /// Is enabled
    pub enabled: bool,
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self {
            command_id: CommandId::new(0),
            handler_addr: 0,
            name: [0u8; 32],
            name_len: 0,
            required_flags: CommandFlags::NONE,
            enabled: false,
        }
    }
}

/// Maximum handlers
pub const MAX_HANDLERS: usize = 64;

/// Command handler registry
#[derive(Debug)]
pub struct HandlerRegistry {
    /// Registered handlers
    handlers: [CommandHandler; MAX_HANDLERS],
    /// Handler count
    count: usize,
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerRegistry {
    /// Create new registry
    pub const fn new() -> Self {
        Self {
            handlers: [CommandHandler {
                command_id: CommandId(0),
                handler_addr: 0,
                name: [0u8; 32],
                name_len: 0,
                required_flags: CommandFlags::NONE,
                enabled: false,
            }; MAX_HANDLERS],
            count: 0,
        }
    }

    /// Register handler
    pub fn register(&mut self, handler: CommandHandler) -> bool {
        if self.count >= MAX_HANDLERS {
            return false;
        }
        self.handlers[self.count] = handler;
        self.handlers[self.count].enabled = true;
        self.count += 1;
        true
    }

    /// Find handler for command
    pub fn find(&self, command_id: CommandId) -> Option<&CommandHandler> {
        for i in 0..self.count {
            if self.handlers[i].command_id == command_id && self.handlers[i].enabled {
                return Some(&self.handlers[i]);
            }
        }
        None
    }

    /// Get handler count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// ACTION DEFINITIONS
// =============================================================================

/// Boot action details
#[derive(Debug, Clone, Copy, Default)]
pub struct BootAction {
    /// Entry index
    pub entry_index: u16,
    /// Override arguments
    pub override_args: bool,
    /// Custom arguments offset
    pub args_offset: u16,
    /// Custom arguments length
    pub args_len: u16,
    /// Boot once (don't save)
    pub boot_once: bool,
    /// Force safe mode
    pub safe_mode: bool,
    /// Force debug mode
    pub debug_mode: bool,
    /// Timeout override (0 = immediate)
    pub timeout_ms: u32,
}

/// Menu action details
#[derive(Debug, Clone, Copy)]
pub struct MenuAction {
    /// Action type
    pub action_type: MenuActionType,
    /// Target index
    pub target: i16,
    /// Amount (for scroll)
    pub amount: i16,
    /// Search/filter query offset
    pub query_offset: u16,
    /// Query length
    pub query_len: u16,
}

impl Default for MenuAction {
    fn default() -> Self {
        Self {
            action_type: MenuActionType::None,
            target: 0,
            amount: 0,
            query_offset: 0,
            query_len: 0,
        }
    }
}

/// Menu action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuActionType {
    /// No action
    None,
    /// Select item
    Select,
    /// Navigate (relative)
    Navigate,
    /// Go to (absolute)
    GoTo,
    /// Scroll
    Scroll,
    /// Search
    Search,
    /// Filter
    Filter,
    /// Sort
    Sort,
    /// Expand/collapse
    Toggle,
    /// Refresh
    Refresh,
    /// Close
    Close,
}

impl Default for MenuActionType {
    fn default() -> Self {
        Self::None
    }
}

/// System action details
#[derive(Debug, Clone, Copy)]
pub struct SystemAction {
    /// Action type
    pub action_type: SystemActionType,
    /// Target
    pub target: u64,
    /// Flags
    pub flags: u32,
    /// Delay (milliseconds)
    pub delay_ms: u32,
}

impl Default for SystemAction {
    fn default() -> Self {
        Self {
            action_type: SystemActionType::None,
            target: 0,
            flags: 0,
            delay_ms: 0,
        }
    }
}

/// System action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemActionType {
    /// No action
    None,
    /// Reboot
    Reboot,
    /// Shutdown
    Shutdown,
    /// Suspend
    Suspend,
    /// Hibernate
    Hibernate,
    /// Warm reboot
    WarmReboot,
    /// Cold reboot
    ColdReboot,
    /// Reset to defaults
    ResetDefaults,
    /// Enter UEFI shell
    UefiShell,
    /// Enter firmware setup
    FirmwareSetup,
    /// Exit bootloader
    Exit,
}

impl Default for SystemActionType {
    fn default() -> Self {
        Self::None
    }
}

// =============================================================================
// CONSOLE COMMANDS
// =============================================================================

/// Console command
#[derive(Debug, Clone, Copy)]
pub struct ConsoleCommand {
    /// Command name
    pub name: [u8; 16],
    /// Name length
    pub name_len: usize,
    /// Short description
    pub description: [u8; 64],
    /// Description length
    pub desc_len: usize,
    /// Usage string
    pub usage: [u8; 64],
    /// Usage length
    pub usage_len: usize,
    /// Associated command ID
    pub command_id: CommandId,
    /// Minimum arguments
    pub min_args: u8,
    /// Maximum arguments
    pub max_args: u8,
    /// Is hidden
    pub hidden: bool,
}

impl Default for ConsoleCommand {
    fn default() -> Self {
        Self {
            name: [0u8; 16],
            name_len: 0,
            description: [0u8; 64],
            desc_len: 0,
            usage: [0u8; 64],
            usage_len: 0,
            command_id: CommandId::new(0),
            min_args: 0,
            max_args: 0,
            hidden: false,
        }
    }
}

/// Standard console commands
pub mod console_cmds {
    /// Boot command name.
    pub const BOOT: &str = "boot";
    /// Help command name.
    pub const HELP: &str = "help";
    /// Info command name.
    pub const INFO: &str = "info";
    /// List command name.
    pub const LIST: &str = "list";
    /// Reboot command name.
    pub const REBOOT: &str = "reboot";
    /// Shutdown command name.
    pub const SHUTDOWN: &str = "shutdown";
    /// Exit command name.
    pub const EXIT: &str = "exit";
    /// Clear command name.
    pub const CLEAR: &str = "clear";
    /// Echo command name.
    pub const ECHO: &str = "echo";
    /// Set command name.
    pub const SET: &str = "set";
    /// Get command name.
    pub const GET: &str = "get";
    /// Memory command name.
    pub const MEMORY: &str = "memory";
    /// Devices command name.
    pub const DEVICES: &str = "devices";
    /// Version command name.
    pub const VERSION: &str = "version";
    /// Debug command name.
    pub const DEBUG: &str = "debug";
}

// =============================================================================
// COMMAND HISTORY
// =============================================================================

/// Maximum history entries
pub const MAX_HISTORY: usize = 32;

/// History entry
#[derive(Debug, Clone, Copy)]
pub struct HistoryEntry {
    /// Command ID
    pub command_id: CommandId,
    /// Result
    pub result: CommandResult,
    /// Timestamp
    pub timestamp: u64,
    /// Duration (microseconds)
    pub duration_us: u32,
}

impl Default for HistoryEntry {
    fn default() -> Self {
        Self {
            command_id: CommandId::new(0),
            result: CommandResult::Success,
            timestamp: 0,
            duration_us: 0,
        }
    }
}

/// Command history
#[derive(Debug)]
pub struct CommandHistory {
    /// Entries
    entries: [HistoryEntry; MAX_HISTORY],
    /// Write index
    write_idx: usize,
    /// Entry count
    count: usize,
    /// Total commands executed
    total_executed: u64,
    /// Total failures
    total_failures: u64,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    /// Create new history
    pub const fn new() -> Self {
        Self {
            entries: [HistoryEntry {
                command_id: CommandId(0),
                result: CommandResult::Success,
                timestamp: 0,
                duration_us: 0,
            }; MAX_HISTORY],
            write_idx: 0,
            count: 0,
            total_executed: 0,
            total_failures: 0,
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: HistoryEntry) {
        self.entries[self.write_idx] = entry;
        self.write_idx = (self.write_idx + 1) % MAX_HISTORY;
        if self.count < MAX_HISTORY {
            self.count += 1;
        }
        self.total_executed += 1;
        if entry.result.is_error() {
            self.total_failures += 1;
        }
    }

    /// Get entry count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get total executed
    pub const fn total_executed(&self) -> u64 {
        self.total_executed
    }

    /// Get total failures
    pub const fn total_failures(&self) -> u64 {
        self.total_failures
    }

    /// Get last entry
    pub const fn last(&self) -> Option<&HistoryEntry> {
        if self.count == 0 {
            None
        } else {
            let idx = if self.write_idx == 0 {
                MAX_HISTORY - 1
            } else {
                self.write_idx - 1
            };
            Some(&self.entries[idx])
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
    fn test_command_id() {
        let id = CommandId::new(0x0100);
        assert_eq!(id.raw(), 0x0100);
    }

    #[test]
    fn test_key_modifiers() {
        let mods = KeyModifiers::CTRL.with(KeyModifiers::SHIFT);
        assert!(mods.ctrl());
        assert!(mods.shift());
        assert!(!mods.alt());
    }

    #[test]
    fn test_command_queue() {
        let mut queue = CommandQueue::new();
        assert!(queue.is_empty());

        let cmd = Command::new(cmd_ids::BOOT_DEFAULT, CommandCategory::Boot);
        assert!(queue.push(cmd));
        assert_eq!(queue.len(), 1);

        let popped = queue.pop();
        assert!(popped.is_some());
        assert!(queue.is_empty());
    }

    #[test]
    fn test_command_result() {
        assert!(CommandResult::Success.is_success());
        assert!(CommandResult::SuccessValue(42).is_success());
        assert!(CommandResult::Failed(1).is_error());
        assert!(!CommandResult::Deferred.is_error());
    }

    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new();
        assert!(history.is_empty());

        history.add(HistoryEntry {
            command_id: cmd_ids::BOOT_DEFAULT,
            result: CommandResult::Success,
            timestamp: 12345,
            duration_us: 100,
        });

        assert_eq!(history.len(), 1);
        assert_eq!(history.total_executed(), 1);
        assert!(history.last().is_some());
    }
}
