//! Boot Manager Core
//!
//! This module provides the main boot manager logic, coordinating
//! all boot operations for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - Boot process coordination
//! - Entry selection
//! - Configuration loading
//! - State machine management
//! - Error handling integration

use core::fmt;

// =============================================================================
// BOOT MANAGER STATE
// =============================================================================

/// Boot manager state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BootState {
    /// Not initialized
    #[default]
    Uninitialized       = 0,
    /// Initializing
    Initializing        = 1,
    /// Loading configuration
    LoadingConfig       = 2,
    /// Discovering entries
    DiscoveringEntries  = 3,
    /// Displaying menu
    ShowingMenu         = 4,
    /// Waiting for selection
    WaitingForSelection = 5,
    /// Entry selected
    EntrySelected       = 6,
    /// Loading kernel
    LoadingKernel       = 7,
    /// Preparing handoff
    PreparingHandoff    = 8,
    /// Executing handoff
    ExecutingHandoff    = 9,
    /// Boot complete
    BootComplete        = 10,
    /// Error state
    Error               = 255,
}

impl fmt::Display for BootState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootState::Uninitialized => write!(f, "Uninitialized"),
            BootState::Initializing => write!(f, "Initializing"),
            BootState::LoadingConfig => write!(f, "Loading Configuration"),
            BootState::DiscoveringEntries => write!(f, "Discovering Entries"),
            BootState::ShowingMenu => write!(f, "Showing Menu"),
            BootState::WaitingForSelection => write!(f, "Waiting for Selection"),
            BootState::EntrySelected => write!(f, "Entry Selected"),
            BootState::LoadingKernel => write!(f, "Loading Kernel"),
            BootState::PreparingHandoff => write!(f, "Preparing Handoff"),
            BootState::ExecutingHandoff => write!(f, "Executing Handoff"),
            BootState::BootComplete => write!(f, "Boot Complete"),
            BootState::Error => write!(f, "Error"),
        }
    }
}

/// State transition result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionResult {
    /// Transition successful
    Ok,
    /// Invalid transition
    Invalid,
    /// Transition blocked
    Blocked,
    /// Error occurred
    Error(u32),
}

// =============================================================================
// BOOT OPTIONS
// =============================================================================

/// Boot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BootMode {
    /// Normal boot
    #[default]
    Normal,
    /// Safe mode
    Safe,
    /// Recovery mode
    Recovery,
    /// Debug mode
    Debug,
    /// Single user mode
    SingleUser,
    /// Verbose mode
    Verbose,
    /// Rescue mode
    Rescue,
}

impl fmt::Display for BootMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootMode::Normal => write!(f, "Normal"),
            BootMode::Safe => write!(f, "Safe Mode"),
            BootMode::Recovery => write!(f, "Recovery Mode"),
            BootMode::Debug => write!(f, "Debug Mode"),
            BootMode::SingleUser => write!(f, "Single User"),
            BootMode::Verbose => write!(f, "Verbose"),
            BootMode::Rescue => write!(f, "Rescue Mode"),
        }
    }
}

/// Boot flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BootFlags(u32);

impl BootFlags {
    /// No flags
    pub const NONE: BootFlags = BootFlags(0);
    /// Quick boot (skip menu if possible)
    pub const QUICK_BOOT: BootFlags = BootFlags(1 << 0);
    /// Show menu regardless of timeout
    pub const FORCE_MENU: BootFlags = BootFlags(1 << 1);
    /// Enable verbose output
    pub const VERBOSE: BootFlags = BootFlags(1 << 2);
    /// Enable debug mode
    pub const DEBUG: BootFlags = BootFlags(1 << 3);
    /// Boot to recovery
    pub const RECOVERY: BootFlags = BootFlags(1 << 4);
    /// Boot to safe mode
    pub const SAFE_MODE: BootFlags = BootFlags(1 << 5);
    /// Rescan devices
    pub const RESCAN: BootFlags = BootFlags(1 << 6);
    /// Reset configuration
    pub const RESET_CONFIG: BootFlags = BootFlags(1 << 7);
    /// Network boot preferred
    pub const NETWORK_BOOT: BootFlags = BootFlags(1 << 8);
    /// USB boot preferred
    pub const USB_BOOT: BootFlags = BootFlags(1 << 9);
    /// Test mode (don't actually boot)
    pub const TEST_MODE: BootFlags = BootFlags(1 << 10);
    /// Silent boot (minimal output)
    pub const SILENT: BootFlags = BootFlags(1 << 11);

    /// Check flag
    pub const fn has(&self, flag: BootFlags) -> bool {
        self.0 & flag.0 != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: BootFlags) {
        self.0 |= flag.0;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: BootFlags) {
        self.0 &= !flag.0;
    }

    /// Combine flags
    pub const fn combine(self, other: BootFlags) -> BootFlags {
        BootFlags(self.0 | other.0)
    }
}

// =============================================================================
// BOOT ENTRY
// =============================================================================

/// Entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntryType {
    /// Unknown type
    #[default]
    Unknown,
    /// Helix kernel
    HelixKernel,
    /// Linux kernel
    LinuxKernel,
    /// EFI application
    EfiApplication,
    /// Windows bootloader
    WindowsBoot,
    /// macOS bootloader
    MacOSBoot,
    /// Chain load
    ChainLoad,
    /// Recovery
    Recovery,
    /// Submenu
    Submenu,
    /// Power off
    PowerOff,
    /// Reboot
    Reboot,
    /// Firmware setup
    FirmwareSetup,
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntryType::Unknown => write!(f, "Unknown"),
            EntryType::HelixKernel => write!(f, "Helix"),
            EntryType::LinuxKernel => write!(f, "Linux"),
            EntryType::EfiApplication => write!(f, "EFI App"),
            EntryType::WindowsBoot => write!(f, "Windows"),
            EntryType::MacOSBoot => write!(f, "macOS"),
            EntryType::ChainLoad => write!(f, "Chain"),
            EntryType::Recovery => write!(f, "Recovery"),
            EntryType::Submenu => write!(f, "Menu"),
            EntryType::PowerOff => write!(f, "Power Off"),
            EntryType::Reboot => write!(f, "Reboot"),
            EntryType::FirmwareSetup => write!(f, "Setup"),
        }
    }
}

/// Entry flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntryFlags(u16);

impl EntryFlags {
    /// No flags
    pub const NONE: EntryFlags = EntryFlags(0);
    /// Entry is default
    pub const DEFAULT: EntryFlags = EntryFlags(1 << 0);
    /// Entry is hidden
    pub const HIDDEN: EntryFlags = EntryFlags(1 << 1);
    /// Entry is disabled
    pub const DISABLED: EntryFlags = EntryFlags(1 << 2);
    /// Entry is auto-detected
    pub const AUTO_DETECTED: EntryFlags = EntryFlags(1 << 3);
    /// Entry is from config file
    pub const FROM_CONFIG: EntryFlags = EntryFlags(1 << 4);
    /// Entry is fallback
    pub const FALLBACK: EntryFlags = EntryFlags(1 << 5);
    /// Secure boot verified
    pub const VERIFIED: EntryFlags = EntryFlags(1 << 6);
    /// Entry requires password
    pub const PASSWORD_REQUIRED: EntryFlags = EntryFlags(1 << 7);
    /// Entry is network boot
    pub const NETWORK: EntryFlags = EntryFlags(1 << 8);
    /// Entry is removable media
    pub const REMOVABLE: EntryFlags = EntryFlags(1 << 9);

    /// Check flag
    pub const fn has(&self, flag: EntryFlags) -> bool {
        self.0 & flag.0 != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: EntryFlags) {
        self.0 |= flag.0;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: EntryFlags) {
        self.0 &= !flag.0;
    }
}

/// Maximum title length
pub const MAX_TITLE_LEN: usize = 64;

/// Maximum path length
pub const MAX_PATH_LEN: usize = 256;

/// Maximum arguments length
pub const MAX_ARGS_LEN: usize = 512;

/// Boot entry
#[derive(Debug, Clone, Copy)]
pub struct BootEntry {
    /// Entry ID
    pub id: u16,
    /// Entry type
    pub entry_type: EntryType,
    /// Entry flags
    pub flags: EntryFlags,
    /// Title
    pub title: [u8; MAX_TITLE_LEN],
    /// Title length
    pub title_len: usize,
    /// Kernel/loader path
    pub path: [u8; MAX_PATH_LEN],
    /// Path length
    pub path_len: usize,
    /// Kernel arguments
    pub args: [u8; MAX_ARGS_LEN],
    /// Arguments length
    pub args_len: usize,
    /// Initrd path (optional)
    pub initrd_path: [u8; MAX_PATH_LEN],
    /// Initrd path length
    pub initrd_len: usize,
    /// Device index
    pub device_id: u8,
    /// Partition index
    pub partition_id: u8,
    /// Icon index (for UI)
    pub icon: u8,
    /// Sort priority
    pub priority: u8,
}

impl Default for BootEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl BootEntry {
    /// Create new boot entry
    pub const fn new() -> Self {
        Self {
            id: 0,
            entry_type: EntryType::Unknown,
            flags: EntryFlags::NONE,
            title: [0; MAX_TITLE_LEN],
            title_len: 0,
            path: [0; MAX_PATH_LEN],
            path_len: 0,
            args: [0; MAX_ARGS_LEN],
            args_len: 0,
            initrd_path: [0; MAX_PATH_LEN],
            initrd_len: 0,
            device_id: 0,
            partition_id: 0,
            icon: 0,
            priority: 128,
        }
    }

    /// Set title
    pub fn set_title(&mut self, title: &str) {
        let bytes = title.as_bytes();
        let len = bytes.len().min(MAX_TITLE_LEN);
        self.title[..len].copy_from_slice(&bytes[..len]);
        self.title_len = len;
    }

    /// Get title
    pub fn title(&self) -> &str {
        core::str::from_utf8(&self.title[..self.title_len]).unwrap_or("")
    }

    /// Set path
    pub fn set_path(&mut self, path: &str) {
        let bytes = path.as_bytes();
        let len = bytes.len().min(MAX_PATH_LEN);
        self.path[..len].copy_from_slice(&bytes[..len]);
        self.path_len = len;
    }

    /// Get path
    pub fn path(&self) -> &str {
        core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("")
    }

    /// Set arguments
    pub fn set_args(&mut self, args: &str) {
        let bytes = args.as_bytes();
        let len = bytes.len().min(MAX_ARGS_LEN);
        self.args[..len].copy_from_slice(&bytes[..len]);
        self.args_len = len;
    }

    /// Get arguments
    pub fn args(&self) -> &str {
        core::str::from_utf8(&self.args[..self.args_len]).unwrap_or("")
    }

    /// Set initrd path
    pub fn set_initrd(&mut self, path: &str) {
        let bytes = path.as_bytes();
        let len = bytes.len().min(MAX_PATH_LEN);
        self.initrd_path[..len].copy_from_slice(&bytes[..len]);
        self.initrd_len = len;
    }

    /// Get initrd path
    pub fn initrd(&self) -> &str {
        core::str::from_utf8(&self.initrd_path[..self.initrd_len]).unwrap_or("")
    }

    /// Check if default
    pub const fn is_default(&self) -> bool {
        self.flags.has(EntryFlags::DEFAULT)
    }

    /// Check if hidden
    pub const fn is_hidden(&self) -> bool {
        self.flags.has(EntryFlags::HIDDEN)
    }

    /// Check if disabled
    pub const fn is_disabled(&self) -> bool {
        self.flags.has(EntryFlags::DISABLED)
    }

    /// Check if bootable (not action entry)
    pub const fn is_bootable(&self) -> bool {
        !matches!(
            self.entry_type,
            EntryType::PowerOff | EntryType::Reboot | EntryType::FirmwareSetup | EntryType::Submenu
        )
    }
}

// =============================================================================
// BOOT ENTRY LIST
// =============================================================================

/// Maximum boot entries
pub const MAX_ENTRIES: usize = 32;

/// Boot entry list
#[derive(Debug)]
pub struct EntryList {
    /// Entries
    entries: [BootEntry; MAX_ENTRIES],
    /// Entry count
    count: usize,
    /// Default entry index
    default_index: usize,
    /// Selected entry index
    selected_index: usize,
}

impl Default for EntryList {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryList {
    /// Create new entry list
    pub const fn new() -> Self {
        Self {
            entries: [BootEntry::new(); MAX_ENTRIES],
            count: 0,
            default_index: 0,
            selected_index: 0,
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: BootEntry) -> Option<usize> {
        if self.count >= MAX_ENTRIES {
            return None;
        }

        let index = self.count;
        self.entries[index] = entry;
        self.entries[index].id = index as u16;

        // Check if default
        if entry.flags.has(EntryFlags::DEFAULT) {
            self.default_index = index;
        }

        self.count += 1;
        Some(index)
    }

    /// Get entry by index
    pub fn get(&self, index: usize) -> Option<&BootEntry> {
        if index < self.count {
            Some(&self.entries[index])
        } else {
            None
        }
    }

    /// Get mutable entry by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut BootEntry> {
        if index < self.count {
            Some(&mut self.entries[index])
        } else {
            None
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

    /// Get default index
    pub const fn default_index(&self) -> usize {
        self.default_index
    }

    /// Set default entry
    pub fn set_default(&mut self, index: usize) {
        if index < self.count {
            // Clear previous default
            self.entries[self.default_index]
                .flags
                .clear(EntryFlags::DEFAULT);
            // Set new default
            self.entries[index].flags.set(EntryFlags::DEFAULT);
            self.default_index = index;
        }
    }

    /// Get selected index
    pub const fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Select entry
    pub fn select(&mut self, index: usize) {
        if index < self.count {
            self.selected_index = index;
        }
    }

    /// Get selected entry
    pub fn selected(&self) -> Option<&BootEntry> {
        self.get(self.selected_index)
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            // Skip hidden entries
            while self.selected_index > 0 && self.entries[self.selected_index].is_hidden() {
                self.selected_index -= 1;
            }
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.count {
            self.selected_index += 1;
            // Skip hidden entries
            while self.selected_index + 1 < self.count
                && self.entries[self.selected_index].is_hidden()
            {
                self.selected_index += 1;
            }
        }
    }

    /// Find entry by title
    pub fn find_by_title(&self, title: &str) -> Option<usize> {
        for i in 0..self.count {
            if self.entries[i].title() == title {
                return Some(i);
            }
        }
        None
    }

    /// Sort entries by priority
    pub fn sort_by_priority(&mut self) {
        // Simple bubble sort (small array)
        for i in 0..self.count {
            for j in 0..self.count - 1 - i {
                if self.entries[j].priority > self.entries[j + 1].priority {
                    // Swap
                    let tmp = self.entries[j];
                    self.entries[j] = self.entries[j + 1];
                    self.entries[j + 1] = tmp;
                }
            }
        }

        // Update indices
        for i in 0..self.count {
            self.entries[i].id = i as u16;
            if self.entries[i].is_default() {
                self.default_index = i;
            }
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.count = 0;
        self.default_index = 0;
        self.selected_index = 0;
    }

    /// Get visible entry count
    pub fn visible_count(&self) -> usize {
        let mut count = 0;
        for i in 0..self.count {
            if !self.entries[i].is_hidden() {
                count += 1;
            }
        }
        count
    }
}

// =============================================================================
// TIMEOUT
// =============================================================================

/// Timeout state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutState {
    /// Timeout disabled
    Disabled,
    /// Counting down
    Counting(u32),
    /// Paused (user interrupted)
    Paused,
    /// Expired
    Expired,
}

/// Timeout manager
#[derive(Debug, Clone, Copy)]
pub struct Timeout {
    /// Initial timeout (seconds)
    initial: i32,
    /// Remaining time (milliseconds)
    remaining_ms: u32,
    /// State
    state: TimeoutState,
}

impl Default for Timeout {
    fn default() -> Self {
        Self::new(5)
    }
}

impl Timeout {
    /// Create new timeout
    pub const fn new(seconds: i32) -> Self {
        if seconds < 0 {
            Self {
                initial: seconds,
                remaining_ms: 0,
                state: TimeoutState::Disabled,
            }
        } else if seconds == 0 {
            Self {
                initial: 0,
                remaining_ms: 0,
                state: TimeoutState::Expired,
            }
        } else {
            Self {
                initial: seconds,
                remaining_ms: seconds as u32 * 1000,
                state: TimeoutState::Counting(seconds as u32),
            }
        }
    }

    /// Update timeout (call periodically)
    pub fn update(&mut self, elapsed_ms: u32) {
        if let TimeoutState::Counting(_) = self.state {
            if elapsed_ms >= self.remaining_ms {
                self.remaining_ms = 0;
                self.state = TimeoutState::Expired;
            } else {
                self.remaining_ms -= elapsed_ms;
                self.state = TimeoutState::Counting(self.remaining_ms / 1000);
            }
        }
    }

    /// Pause timeout
    pub fn pause(&mut self) {
        if let TimeoutState::Counting(_) = self.state {
            self.state = TimeoutState::Paused;
        }
    }

    /// Resume timeout
    pub fn resume(&mut self) {
        if self.state == TimeoutState::Paused {
            self.state = TimeoutState::Counting(self.remaining_ms / 1000);
        }
    }

    /// Reset timeout
    pub fn reset(&mut self) {
        if self.initial > 0 {
            self.remaining_ms = self.initial as u32 * 1000;
            self.state = TimeoutState::Counting(self.initial as u32);
        }
    }

    /// Get remaining seconds
    pub const fn remaining_secs(&self) -> u32 {
        self.remaining_ms / 1000
    }

    /// Check if expired
    pub const fn is_expired(&self) -> bool {
        matches!(self.state, TimeoutState::Expired)
    }

    /// Check if counting
    pub const fn is_counting(&self) -> bool {
        matches!(self.state, TimeoutState::Counting(_))
    }

    /// Check if disabled
    pub const fn is_disabled(&self) -> bool {
        matches!(self.state, TimeoutState::Disabled)
    }

    /// Get state
    pub const fn state(&self) -> TimeoutState {
        self.state
    }
}

// =============================================================================
// BOOT MANAGER
// =============================================================================

/// Boot manager
#[derive(Debug)]
pub struct BootManager {
    /// Current state
    pub state: BootState,
    /// Boot mode
    pub mode: BootMode,
    /// Boot flags
    pub flags: BootFlags,
    /// Entry list
    pub entries: EntryList,
    /// Timeout
    pub timeout: Timeout,
    /// Last error code
    pub last_error: u32,
    /// Error message buffer
    error_msg: [u8; 128],
    /// Error message length
    error_len: usize,
    /// Boot count (for statistics)
    pub boot_count: u32,
    /// Configuration loaded
    pub config_loaded: bool,
    /// Menu displayed
    pub menu_shown: bool,
}

impl Default for BootManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BootManager {
    /// Create new boot manager
    pub const fn new() -> Self {
        Self {
            state: BootState::Uninitialized,
            mode: BootMode::Normal,
            flags: BootFlags::NONE,
            entries: EntryList::new(),
            timeout: Timeout::new(5),
            last_error: 0,
            error_msg: [0; 128],
            error_len: 0,
            boot_count: 0,
            config_loaded: false,
            menu_shown: false,
        }
    }

    /// Initialize boot manager
    pub fn initialize(&mut self) -> TransitionResult {
        if self.state != BootState::Uninitialized {
            return TransitionResult::Invalid;
        }

        self.state = BootState::Initializing;
        TransitionResult::Ok
    }

    /// Advance to next state
    pub fn advance(&mut self) -> TransitionResult {
        let next = match self.state {
            BootState::Initializing => BootState::LoadingConfig,
            BootState::LoadingConfig => {
                self.config_loaded = true;
                BootState::DiscoveringEntries
            },
            BootState::DiscoveringEntries => BootState::ShowingMenu,
            BootState::ShowingMenu => {
                self.menu_shown = true;
                BootState::WaitingForSelection
            },
            BootState::WaitingForSelection => {
                // Check for timeout or selection
                if self.timeout.is_expired() || self.flags.has(BootFlags::QUICK_BOOT) {
                    BootState::EntrySelected
                } else {
                    return TransitionResult::Blocked;
                }
            },
            BootState::EntrySelected => BootState::LoadingKernel,
            BootState::LoadingKernel => BootState::PreparingHandoff,
            BootState::PreparingHandoff => BootState::ExecutingHandoff,
            BootState::ExecutingHandoff => BootState::BootComplete,
            BootState::BootComplete | BootState::Error => {
                return TransitionResult::Invalid;
            },
            BootState::Uninitialized => {
                return TransitionResult::Invalid;
            },
        };

        self.state = next;
        TransitionResult::Ok
    }

    /// Set error state
    pub fn set_error(&mut self, code: u32, message: &str) {
        self.state = BootState::Error;
        self.last_error = code;

        let bytes = message.as_bytes();
        let len = bytes.len().min(128);
        self.error_msg[..len].copy_from_slice(&bytes[..len]);
        self.error_len = len;
    }

    /// Get error message
    pub fn error_message(&self) -> &str {
        core::str::from_utf8(&self.error_msg[..self.error_len]).unwrap_or("")
    }

    /// Select entry by index
    pub fn select_entry(&mut self, index: usize) -> bool {
        if index < self.entries.len() {
            self.entries.select(index);
            self.timeout.pause();
            true
        } else {
            false
        }
    }

    /// Select and boot entry
    pub fn boot_selected(&mut self) -> TransitionResult {
        if self
            .entries
            .selected()
            .map(|e| e.is_bootable())
            .unwrap_or(false)
        {
            self.state = BootState::EntrySelected;
            TransitionResult::Ok
        } else {
            TransitionResult::Invalid
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: u16) {
        // Pause timeout on any key
        self.timeout.pause();

        match key {
            // Up arrow
            0x01 => self.entries.select_previous(),
            // Down arrow
            0x02 => self.entries.select_next(),
            // Enter
            0x0D => {
                let _ = self.boot_selected();
            },
            // Escape
            0x1B => {
                self.timeout.resume();
            },
            _ => {},
        }
    }

    /// Update (call periodically)
    pub fn update(&mut self, elapsed_ms: u32) {
        if self.state == BootState::WaitingForSelection {
            self.timeout.update(elapsed_ms);

            if self.timeout.is_expired() {
                // Auto-boot default
                self.entries.select(self.entries.default_index());
                let _ = self.advance();
            }
        }
    }

    /// Get selected entry
    pub fn selected_entry(&self) -> Option<&BootEntry> {
        self.entries.selected()
    }

    /// Check if ready to boot
    pub fn ready_to_boot(&self) -> bool {
        self.state == BootState::EntrySelected
            && self
                .entries
                .selected()
                .map(|e| e.is_bootable())
                .unwrap_or(false)
    }

    /// Get progress percentage (0-100)
    pub fn progress(&self) -> u8 {
        match self.state {
            BootState::Uninitialized => 0,
            BootState::Initializing => 10,
            BootState::LoadingConfig => 20,
            BootState::DiscoveringEntries => 30,
            BootState::ShowingMenu => 40,
            BootState::WaitingForSelection => 40,
            BootState::EntrySelected => 50,
            BootState::LoadingKernel => 70,
            BootState::PreparingHandoff => 85,
            BootState::ExecutingHandoff => 95,
            BootState::BootComplete => 100,
            BootState::Error => 0,
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
    fn test_boot_state() {
        let mut mgr = BootManager::new();
        assert_eq!(mgr.state, BootState::Uninitialized);

        assert_eq!(mgr.initialize(), TransitionResult::Ok);
        assert_eq!(mgr.state, BootState::Initializing);
    }

    #[test]
    fn test_boot_entry() {
        let mut entry = BootEntry::new();
        entry.set_title("Helix OS");
        entry.set_path("/boot/helix/kernel");
        entry.set_args("root=/dev/sda1");

        assert_eq!(entry.title(), "Helix OS");
        assert_eq!(entry.path(), "/boot/helix/kernel");
        assert_eq!(entry.args(), "root=/dev/sda1");
    }

    #[test]
    fn test_entry_list() {
        let mut list = EntryList::new();

        let mut entry = BootEntry::new();
        entry.set_title("Test");
        entry.flags.set(EntryFlags::DEFAULT);

        list.add(entry);
        assert_eq!(list.len(), 1);
        assert_eq!(list.default_index(), 0);
    }

    #[test]
    fn test_timeout() {
        let mut timeout = Timeout::new(5);
        assert!(timeout.is_counting());

        timeout.update(2000);
        assert_eq!(timeout.remaining_secs(), 3);

        timeout.pause();
        assert!(!timeout.is_counting());

        timeout.resume();
        assert!(timeout.is_counting());
    }

    #[test]
    fn test_boot_flags() {
        let mut flags = BootFlags::NONE;
        flags.set(BootFlags::VERBOSE);
        flags.set(BootFlags::DEBUG);

        assert!(flags.has(BootFlags::VERBOSE));
        assert!(flags.has(BootFlags::DEBUG));
        assert!(!flags.has(BootFlags::SAFE_MODE));
    }

    #[test]
    fn test_boot_manager_flow() {
        let mut mgr = BootManager::new();

        mgr.initialize();
        assert_eq!(mgr.advance(), TransitionResult::Ok);
        assert_eq!(mgr.state, BootState::LoadingConfig);

        mgr.advance();
        assert_eq!(mgr.state, BootState::DiscoveringEntries);
    }
}
