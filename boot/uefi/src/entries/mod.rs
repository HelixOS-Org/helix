//! Boot Entries and Menu Management
//!
//! This module provides comprehensive boot entry management, menu system,
//! and boot configuration handling for the Helix UEFI Bootloader.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      Boot Entry Management                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌───────────────────────────────────────────────────────────────────┐ │
//! │  │                     Boot Entry Types                              │ │
//! │  │                                                                   │ │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    │ │
//! │  │  │  Kernel │ │   EFI   │ │  Chain  │ │   PXE   │ │ Recovery│    │ │
//! │  │  │  Linux  │ │  App    │ │  Load   │ │  Boot   │ │  Mode   │    │ │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘    │ │
//! │  └───────────────────────────────────────────────────────────────────┘ │
//! │                                                                         │
//! │  ┌───────────────────────────────────────────────────────────────────┐ │
//! │  │                     Menu System                                   │ │
//! │  │  ┌──────────────────────────────────────────────────────────┐    │ │
//! │  │  │  ┌──────────────────────────────────────────────────┐   │    │ │
//! │  │  │  │ ► Helix OS                                       │   │    │ │
//! │  │  │  │   Windows Boot Manager                           │   │    │ │
//! │  │  │  │   Linux (Recovery Mode)                          │   │    │ │
//! │  │  │  │   UEFI Shell                                     │   │    │ │
//! │  │  │  │   System Setup                                   │   │    │ │
//! │  │  │  └──────────────────────────────────────────────────┘   │    │ │
//! │  │  │  [↑↓] Select  [Enter] Boot  [E] Edit  [F1] Help          │    │ │
//! │  │  └──────────────────────────────────────────────────────────┘    │ │
//! │  └───────────────────────────────────────────────────────────────────┘ │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// BOOT ENTRY TYPES
// =============================================================================

/// Boot entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootEntryType {
    /// Linux kernel with optional initrd
    Linux,
    /// EFI application
    EfiApp,
    /// Chainload another bootloader
    Chainload,
    /// Windows Boot Manager
    Windows,
    /// macOS boot
    MacOs,
    /// BSD systems
    Bsd,
    /// PXE network boot
    Pxe,
    /// HTTP boot
    HttpBoot,
    /// UEFI Shell
    UefiShell,
    /// Recovery mode
    Recovery,
    /// Firmware setup
    FirmwareSetup,
    /// Memory test
    MemoryTest,
    /// Shutdown
    Shutdown,
    /// Reboot
    Reboot,
    /// Custom
    Custom,
}

impl BootEntryType {
    /// Get default icon for entry type
    pub const fn default_icon(&self) -> &'static str {
        match self {
            BootEntryType::Linux => "linux",
            BootEntryType::EfiApp => "efi",
            BootEntryType::Chainload => "chain",
            BootEntryType::Windows => "windows",
            BootEntryType::MacOs => "macos",
            BootEntryType::Bsd => "bsd",
            BootEntryType::Pxe => "network",
            BootEntryType::HttpBoot => "network",
            BootEntryType::UefiShell => "shell",
            BootEntryType::Recovery => "recovery",
            BootEntryType::FirmwareSetup => "setup",
            BootEntryType::MemoryTest => "memory",
            BootEntryType::Shutdown => "power",
            BootEntryType::Reboot => "reboot",
            BootEntryType::Custom => "custom",
        }
    }

    /// Check if entry type is bootable
    pub const fn is_bootable(&self) -> bool {
        !matches!(
            self,
            BootEntryType::Shutdown | BootEntryType::Reboot | BootEntryType::FirmwareSetup
        )
    }

    /// Check if entry requires file path
    pub const fn requires_path(&self) -> bool {
        matches!(
            self,
            BootEntryType::Linux
                | BootEntryType::EfiApp
                | BootEntryType::Chainload
                | BootEntryType::Windows
                | BootEntryType::MacOs
                | BootEntryType::Bsd
                | BootEntryType::UefiShell
        )
    }
}

impl fmt::Display for BootEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootEntryType::Linux => write!(f, "Linux"),
            BootEntryType::EfiApp => write!(f, "EFI Application"),
            BootEntryType::Chainload => write!(f, "Chainload"),
            BootEntryType::Windows => write!(f, "Windows"),
            BootEntryType::MacOs => write!(f, "macOS"),
            BootEntryType::Bsd => write!(f, "BSD"),
            BootEntryType::Pxe => write!(f, "PXE Boot"),
            BootEntryType::HttpBoot => write!(f, "HTTP Boot"),
            BootEntryType::UefiShell => write!(f, "UEFI Shell"),
            BootEntryType::Recovery => write!(f, "Recovery"),
            BootEntryType::FirmwareSetup => write!(f, "Firmware Setup"),
            BootEntryType::MemoryTest => write!(f, "Memory Test"),
            BootEntryType::Shutdown => write!(f, "Shutdown"),
            BootEntryType::Reboot => write!(f, "Reboot"),
            BootEntryType::Custom => write!(f, "Custom"),
        }
    }
}

// =============================================================================
// BOOT ENTRY FLAGS
// =============================================================================

/// Boot entry flags
#[derive(Debug, Clone, Copy, Default)]
pub struct BootEntryFlags {
    bits: u32,
}

impl BootEntryFlags {
    /// Entry is enabled
    pub const ENABLED: u32 = 1 << 0;
    /// Entry is hidden
    pub const HIDDEN: u32 = 1 << 1;
    /// Entry is the default
    pub const DEFAULT: u32 = 1 << 2;
    /// Entry is read-only
    pub const READONLY: u32 = 1 << 3;
    /// Entry requires password
    pub const PASSWORD: u32 = 1 << 4;
    /// Entry is auto-detected
    pub const AUTODETECT: u32 = 1 << 5;
    /// Entry is user-created
    pub const USER: u32 = 1 << 6;
    /// Entry is temporary
    pub const TEMPORARY: u32 = 1 << 7;
    /// Entry uses Secure Boot
    pub const SECURE: u32 = 1 << 8;
    /// Entry is verified
    pub const VERIFIED: u32 = 1 << 9;
    /// Entry allows editing
    pub const EDITABLE: u32 = 1 << 10;
    /// Entry uses fallback
    pub const FALLBACK: u32 = 1 << 11;

    /// Create new flags
    pub const fn new(bits: u32) -> Self {
        Self { bits }
    }

    /// Empty flags
    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    /// Default enabled entry
    pub const fn default_enabled() -> Self {
        Self {
            bits: Self::ENABLED | Self::EDITABLE,
        }
    }

    /// Check if flag is set
    pub const fn contains(&self, flag: u32) -> bool {
        (self.bits & flag) != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: u32) {
        self.bits |= flag;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: u32) {
        self.bits &= !flag;
    }

    /// Get raw bits
    pub const fn bits(&self) -> u32 {
        self.bits
    }
}

// =============================================================================
// DEVICE PATH
// =============================================================================

/// Device path type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DevicePathType {
    /// Hard disk partition
    HardDisk,
    /// USB device
    Usb,
    /// CD/DVD
    Cdrom,
    /// Network
    Network,
    /// Firmware volume
    FirmwareVolume,
    /// RAM disk
    RamDisk,
    /// File path
    FilePath,
    /// Unknown
    #[default]
    Unknown,
}

/// Partition identifier
#[derive(Debug, Clone, Copy)]
pub struct PartitionId {
    /// Partition GUID
    pub guid: [u8; 16],
    /// Partition number
    pub number: u32,
    /// Partition type
    pub part_type: PartitionType,
}

impl Default for PartitionId {
    fn default() -> Self {
        Self {
            guid: [0u8; 16],
            number: 0,
            part_type: PartitionType::Unknown,
        }
    }
}

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PartitionType {
    /// Unknown
    #[default]
    Unknown,
    /// EFI System Partition
    Esp,
    /// Microsoft Basic Data
    MsBasic,
    /// Linux filesystem
    LinuxFs,
    /// Linux swap
    LinuxSwap,
    /// Linux LVM
    LinuxLvm,
    /// Linux RAID
    LinuxRaid,
    /// Apple HFS+
    AppleHfs,
    /// Apple APFS
    AppleApfs,
}

// =============================================================================
// BOOT ENTRY
// =============================================================================

/// Maximum path length
pub const MAX_PATH_LEN: usize = 256;
/// Maximum title length
pub const MAX_TITLE_LEN: usize = 64;
/// Maximum arguments length
pub const MAX_ARGS_LEN: usize = 512;

/// Boot entry structure
#[derive(Debug, Clone)]
pub struct BootEntry {
    /// Entry ID
    pub id: u16,
    /// Entry type
    pub entry_type: BootEntryType,
    /// Flags
    pub flags: BootEntryFlags,
    /// Title
    pub title: [u8; MAX_TITLE_LEN],
    /// Title length
    pub title_len: usize,
    /// Path to loader
    pub path: [u8; MAX_PATH_LEN],
    /// Path length
    pub path_len: usize,
    /// Kernel arguments / command line
    pub args: [u8; MAX_ARGS_LEN],
    /// Arguments length
    pub args_len: usize,
    /// Initrd path (for Linux)
    pub initrd: [u8; MAX_PATH_LEN],
    /// Initrd path length
    pub initrd_len: usize,
    /// Device path type
    pub device_type: DevicePathType,
    /// Partition info
    pub partition: PartitionId,
    /// Icon name
    pub icon: [u8; 32],
    /// Icon name length
    pub icon_len: usize,
    /// Hotkey (0 = none)
    pub hotkey: u8,
    /// Boot count
    pub boot_count: u32,
    /// Last boot timestamp
    pub last_boot: u64,
}

impl Default for BootEntry {
    fn default() -> Self {
        Self {
            id: 0,
            entry_type: BootEntryType::Custom,
            flags: BootEntryFlags::default_enabled(),
            title: [0u8; MAX_TITLE_LEN],
            title_len: 0,
            path: [0u8; MAX_PATH_LEN],
            path_len: 0,
            args: [0u8; MAX_ARGS_LEN],
            args_len: 0,
            initrd: [0u8; MAX_PATH_LEN],
            initrd_len: 0,
            device_type: DevicePathType::Unknown,
            partition: PartitionId::default(),
            icon: [0u8; 32],
            icon_len: 0,
            hotkey: 0,
            boot_count: 0,
            last_boot: 0,
        }
    }
}

impl BootEntry {
    /// Create new empty entry
    pub const fn new() -> Self {
        Self {
            id: 0,
            entry_type: BootEntryType::Custom,
            flags: BootEntryFlags::empty(),
            title: [0u8; MAX_TITLE_LEN],
            title_len: 0,
            path: [0u8; MAX_PATH_LEN],
            path_len: 0,
            args: [0u8; MAX_ARGS_LEN],
            args_len: 0,
            initrd: [0u8; MAX_PATH_LEN],
            initrd_len: 0,
            device_type: DevicePathType::Unknown,
            partition: PartitionId {
                guid: [0u8; 16],
                number: 0,
                part_type: PartitionType::Unknown,
            },
            icon: [0u8; 32],
            icon_len: 0,
            hotkey: 0,
            boot_count: 0,
            last_boot: 0,
        }
    }

    /// Check if entry is enabled
    pub const fn is_enabled(&self) -> bool {
        self.flags.contains(BootEntryFlags::ENABLED)
    }

    /// Check if entry is visible
    pub const fn is_visible(&self) -> bool {
        !self.flags.contains(BootEntryFlags::HIDDEN)
    }

    /// Check if entry is default
    pub const fn is_default(&self) -> bool {
        self.flags.contains(BootEntryFlags::DEFAULT)
    }

    /// Check if entry is editable
    pub const fn is_editable(&self) -> bool {
        self.flags.contains(BootEntryFlags::EDITABLE)
            && !self.flags.contains(BootEntryFlags::READONLY)
    }

    /// Get title as string slice
    pub fn title_str(&self) -> &str {
        if self.title_len > 0 {
            core::str::from_utf8(&self.title[..self.title_len]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Set title from string
    pub fn set_title(&mut self, title: &str) {
        let bytes = title.as_bytes();
        let len = bytes.len().min(MAX_TITLE_LEN);
        self.title[..len].copy_from_slice(&bytes[..len]);
        self.title_len = len;
    }

    /// Get path as string slice
    pub fn path_str(&self) -> &str {
        if self.path_len > 0 {
            core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Set path from string
    pub fn set_path(&mut self, path: &str) {
        let bytes = path.as_bytes();
        let len = bytes.len().min(MAX_PATH_LEN);
        self.path[..len].copy_from_slice(&bytes[..len]);
        self.path_len = len;
    }

    /// Get arguments as string slice
    pub fn args_str(&self) -> &str {
        if self.args_len > 0 {
            core::str::from_utf8(&self.args[..self.args_len]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Set arguments from string
    pub fn set_args(&mut self, args: &str) {
        let bytes = args.as_bytes();
        let len = bytes.len().min(MAX_ARGS_LEN);
        self.args[..len].copy_from_slice(&bytes[..len]);
        self.args_len = len;
    }

    /// Get initrd path as string slice
    pub fn initrd_str(&self) -> &str {
        if self.initrd_len > 0 {
            core::str::from_utf8(&self.initrd[..self.initrd_len]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Set initrd path from string
    pub fn set_initrd(&mut self, initrd: &str) {
        let bytes = initrd.as_bytes();
        let len = bytes.len().min(MAX_PATH_LEN);
        self.initrd[..len].copy_from_slice(&bytes[..len]);
        self.initrd_len = len;
    }
}

// =============================================================================
// LINUX ENTRY BUILDER
// =============================================================================

/// Linux kernel entry builder
#[derive(Debug, Clone)]
pub struct LinuxEntryBuilder {
    entry: BootEntry,
}

impl LinuxEntryBuilder {
    /// Create new Linux entry builder
    pub fn new() -> Self {
        let mut entry = BootEntry::new();
        entry.entry_type = BootEntryType::Linux;
        entry.flags = BootEntryFlags::default_enabled();
        Self { entry }
    }

    /// Set title
    pub fn title(mut self, title: &str) -> Self {
        self.entry.set_title(title);
        self
    }

    /// Set kernel path
    pub fn kernel(mut self, path: &str) -> Self {
        self.entry.set_path(path);
        self
    }

    /// Set initrd path
    pub fn initrd(mut self, path: &str) -> Self {
        self.entry.set_initrd(path);
        self
    }

    /// Set command line
    pub fn cmdline(mut self, args: &str) -> Self {
        self.entry.set_args(args);
        self
    }

    /// Set hotkey
    pub fn hotkey(mut self, key: u8) -> Self {
        self.entry.hotkey = key;
        self
    }

    /// Set as default
    pub fn default(mut self) -> Self {
        self.entry.flags.set(BootEntryFlags::DEFAULT);
        self
    }

    /// Build the entry
    pub fn build(self) -> BootEntry {
        self.entry
    }
}

impl Default for LinuxEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// EFI APP ENTRY BUILDER
// =============================================================================

/// EFI application entry builder
#[derive(Debug, Clone)]
pub struct EfiAppEntryBuilder {
    entry: BootEntry,
}

impl EfiAppEntryBuilder {
    /// Create new EFI app entry builder
    pub fn new() -> Self {
        let mut entry = BootEntry::new();
        entry.entry_type = BootEntryType::EfiApp;
        entry.flags = BootEntryFlags::default_enabled();
        Self { entry }
    }

    /// Set title
    pub fn title(mut self, title: &str) -> Self {
        self.entry.set_title(title);
        self
    }

    /// Set path to EFI application
    pub fn path(mut self, path: &str) -> Self {
        self.entry.set_path(path);
        self
    }

    /// Set arguments
    pub fn args(mut self, args: &str) -> Self {
        self.entry.set_args(args);
        self
    }

    /// Build the entry
    pub fn build(self) -> BootEntry {
        self.entry
    }
}

impl Default for EfiAppEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MENU ITEM
// =============================================================================

/// Menu item state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuItemState {
    /// Normal state
    #[default]
    Normal,
    /// Highlighted/selected
    Selected,
    /// Disabled
    Disabled,
    /// Hidden
    Hidden,
}

/// Menu item type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuItemType {
    /// Boot entry
    #[default]
    BootEntry,
    /// Submenu
    Submenu,
    /// Separator
    Separator,
    /// Action (reboot, shutdown, etc.)
    Action,
    /// Setting toggle
    Toggle,
    /// Text info
    Info,
}

/// Menu item structure
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Item type
    pub item_type: MenuItemType,
    /// State
    pub state: MenuItemState,
    /// Label
    pub label: [u8; MAX_TITLE_LEN],
    /// Label length
    pub label_len: usize,
    /// Description/subtitle
    pub description: [u8; MAX_TITLE_LEN],
    /// Description length
    pub description_len: usize,
    /// Hotkey
    pub hotkey: u8,
    /// Boot entry index (if boot entry)
    pub entry_index: u16,
    /// Submenu ID (if submenu)
    pub submenu_id: u16,
    /// Action ID (if action)
    pub action_id: u16,
    /// Icon name
    pub icon: [u8; 32],
    /// Icon length
    pub icon_len: usize,
}

impl Default for MenuItem {
    fn default() -> Self {
        Self {
            item_type: MenuItemType::BootEntry,
            state: MenuItemState::Normal,
            label: [0u8; MAX_TITLE_LEN],
            label_len: 0,
            description: [0u8; MAX_TITLE_LEN],
            description_len: 0,
            hotkey: 0,
            entry_index: 0,
            submenu_id: 0,
            action_id: 0,
            icon: [0u8; 32],
            icon_len: 0,
        }
    }
}

impl MenuItem {
    /// Create from boot entry
    pub fn from_entry(entry: &BootEntry, index: u16) -> Self {
        let mut item = Self::default();
        item.item_type = MenuItemType::BootEntry;
        item.entry_index = index;
        item.hotkey = entry.hotkey;
        item.label[..entry.title_len].copy_from_slice(&entry.title[..entry.title_len]);
        item.label_len = entry.title_len;
        item.icon[..entry.icon_len].copy_from_slice(&entry.icon[..entry.icon_len]);
        item.icon_len = entry.icon_len;
        item
    }

    /// Create separator
    pub const fn separator() -> Self {
        Self {
            item_type: MenuItemType::Separator,
            state: MenuItemState::Disabled,
            label: [0u8; MAX_TITLE_LEN],
            label_len: 0,
            description: [0u8; MAX_TITLE_LEN],
            description_len: 0,
            hotkey: 0,
            entry_index: 0,
            submenu_id: 0,
            action_id: 0,
            icon: [0u8; 32],
            icon_len: 0,
        }
    }

    /// Check if item is selectable
    pub const fn is_selectable(&self) -> bool {
        !matches!(self.item_type, MenuItemType::Separator | MenuItemType::Info)
            && !matches!(self.state, MenuItemState::Disabled | MenuItemState::Hidden)
    }

    /// Get label as string
    pub fn label_str(&self) -> &str {
        if self.label_len > 0 {
            core::str::from_utf8(&self.label[..self.label_len]).unwrap_or("")
        } else {
            ""
        }
    }
}

// =============================================================================
// MENU ACTIONS
// =============================================================================

/// Menu action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuAction {
    /// No action
    #[default]
    None,
    /// Boot selected entry
    Boot,
    /// Edit selected entry
    Edit,
    /// Show info for selected entry
    Info,
    /// Move selection up
    MoveUp,
    /// Move selection down
    MoveDown,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Go to first
    First,
    /// Go to last
    Last,
    /// Cancel/escape
    Cancel,
    /// Show help
    Help,
    /// Show command line editor
    CommandLine,
    /// Enter submenu
    EnterSubmenu,
    /// Exit submenu
    ExitSubmenu,
    /// Toggle option
    Toggle,
    /// Refresh menu
    Refresh,
    /// Reboot
    Reboot,
    /// Shutdown
    Shutdown,
    /// Firmware setup
    FirmwareSetup,
    /// Recovery mode
    Recovery,
}

// =============================================================================
// MENU CONFIGURATION
// =============================================================================

/// Menu style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuStyle {
    /// Simple list
    List,
    /// Box with border
    #[default]
    Box,
    /// Grid with icons
    Grid,
    /// Minimal
    Minimal,
    /// Full screen
    FullScreen,
}

/// Menu position
#[derive(Debug, Clone, Copy)]
pub struct MenuPosition {
    /// X position (columns)
    pub x: u16,
    /// Y position (rows)
    pub y: u16,
    /// Width (columns)
    pub width: u16,
    /// Height (rows)
    pub height: u16,
}

impl Default for MenuPosition {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 80,
            height: 25,
        }
    }
}

impl MenuPosition {
    /// Create centered menu
    pub const fn centered(width: u16, height: u16, screen_width: u16, screen_height: u16) -> Self {
        Self {
            x: (screen_width.saturating_sub(width)) / 2,
            y: (screen_height.saturating_sub(height)) / 2,
            width,
            height,
        }
    }
}

/// Menu configuration
#[derive(Debug, Clone, Copy)]
pub struct MenuConfig {
    /// Menu style
    pub style: MenuStyle,
    /// Position
    pub position: MenuPosition,
    /// Show hotkeys
    pub show_hotkeys: bool,
    /// Show descriptions
    pub show_descriptions: bool,
    /// Show icons
    pub show_icons: bool,
    /// Show help bar
    pub show_help_bar: bool,
    /// Show timer
    pub show_timer: bool,
    /// Wrap selection
    pub wrap_selection: bool,
    /// Mouse enabled
    pub mouse_enabled: bool,
    /// Timeout (seconds)
    pub timeout: u16,
    /// Selected item colors
    pub selected_fg: u8,
    pub selected_bg: u8,
    /// Normal item colors
    pub normal_fg: u8,
    pub normal_bg: u8,
    /// Disabled item colors
    pub disabled_fg: u8,
    /// Border color
    pub border_fg: u8,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            style: MenuStyle::Box,
            position: MenuPosition::default(),
            show_hotkeys: true,
            show_descriptions: true,
            show_icons: false,
            show_help_bar: true,
            show_timer: true,
            wrap_selection: true,
            mouse_enabled: false,
            timeout: 5,
            selected_fg: 0x0F, // White
            selected_bg: 0x01, // Blue
            normal_fg: 0x07,   // Light gray
            normal_bg: 0x00,   // Black
            disabled_fg: 0x08, // Dark gray
            border_fg: 0x0B,   // Cyan
        }
    }
}

// =============================================================================
// MENU STATE
// =============================================================================

/// Maximum menu items
pub const MAX_MENU_ITEMS: usize = 32;

/// Menu state
#[derive(Debug, Clone)]
pub struct MenuState {
    /// Items
    pub items: [MenuItem; MAX_MENU_ITEMS],
    /// Number of items
    pub item_count: usize,
    /// Currently selected index
    pub selected: usize,
    /// First visible index (for scrolling)
    pub first_visible: usize,
    /// Visible item count
    pub visible_count: usize,
    /// Timeout remaining (seconds)
    pub timeout_remaining: u16,
    /// Menu is active
    pub active: bool,
    /// User has interacted
    pub user_interacted: bool,
    /// Current submenu depth
    pub submenu_depth: u8,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            items: core::array::from_fn(|_| MenuItem::default()),
            item_count: 0,
            selected: 0,
            first_visible: 0,
            visible_count: 10,
            timeout_remaining: 0,
            active: true,
            user_interacted: false,
            submenu_depth: 0,
        }
    }
}

impl MenuState {
    /// Move selection up
    pub fn move_up(&mut self, wrap: bool) {
        if self.selected > 0 {
            self.selected -= 1;
            // Skip non-selectable items
            while self.selected > 0 && !self.items[self.selected].is_selectable() {
                self.selected -= 1;
            }
        } else if wrap && self.item_count > 0 {
            self.selected = self.item_count - 1;
            while self.selected > 0 && !self.items[self.selected].is_selectable() {
                self.selected -= 1;
            }
        }
        self.ensure_visible();
        self.user_interacted = true;
    }

    /// Move selection down
    pub fn move_down(&mut self, wrap: bool) {
        if self.selected + 1 < self.item_count {
            self.selected += 1;
            // Skip non-selectable items
            while self.selected + 1 < self.item_count && !self.items[self.selected].is_selectable()
            {
                self.selected += 1;
            }
        } else if wrap && self.item_count > 0 {
            self.selected = 0;
            while self.selected + 1 < self.item_count && !self.items[self.selected].is_selectable()
            {
                self.selected += 1;
            }
        }
        self.ensure_visible();
        self.user_interacted = true;
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        if self.selected < self.first_visible {
            self.first_visible = self.selected;
        } else if self.selected >= self.first_visible + self.visible_count {
            self.first_visible = self.selected - self.visible_count + 1;
        }
    }

    /// Get currently selected item
    pub fn current_item(&self) -> Option<&MenuItem> {
        if self.selected < self.item_count {
            Some(&self.items[self.selected])
        } else {
            None
        }
    }

    /// Add item to menu
    pub fn add_item(&mut self, item: MenuItem) -> bool {
        if self.item_count < MAX_MENU_ITEMS {
            self.items[self.item_count] = item;
            self.item_count += 1;
            true
        } else {
            false
        }
    }

    /// Clear all items
    pub fn clear(&mut self) {
        self.item_count = 0;
        self.selected = 0;
        self.first_visible = 0;
    }

    /// Find item by hotkey
    pub fn find_by_hotkey(&self, key: u8) -> Option<usize> {
        for (i, item) in self.items[..self.item_count].iter().enumerate() {
            if item.hotkey == key && item.is_selectable() {
                return Some(i);
            }
        }
        None
    }
}

// =============================================================================
// HELP TEXT
// =============================================================================

/// Help text entry
#[derive(Debug, Clone, Copy)]
pub struct HelpEntry {
    /// Key description
    pub key: &'static str,
    /// Action description
    pub action: &'static str,
}

/// Standard help entries
pub const HELP_ENTRIES: &[HelpEntry] = &[
    HelpEntry {
        key: "↑↓",
        action: "Select",
    },
    HelpEntry {
        key: "Enter",
        action: "Boot",
    },
    HelpEntry {
        key: "E",
        action: "Edit",
    },
    HelpEntry {
        key: "C",
        action: "Console",
    },
    HelpEntry {
        key: "F1",
        action: "Help",
    },
    HelpEntry {
        key: "Esc",
        action: "Cancel",
    },
];

// =============================================================================
// BOOT ORDER
// =============================================================================

/// Maximum boot entries
pub const MAX_BOOT_ENTRIES: usize = 64;

/// Boot order manager
#[derive(Debug, Clone)]
pub struct BootOrder {
    /// Entry IDs in boot order
    pub order: [u16; MAX_BOOT_ENTRIES],
    /// Number of entries
    pub count: usize,
}

impl Default for BootOrder {
    fn default() -> Self {
        Self {
            order: [0; MAX_BOOT_ENTRIES],
            count: 0,
        }
    }
}

impl BootOrder {
    /// Create new boot order
    pub const fn new() -> Self {
        Self {
            order: [0; MAX_BOOT_ENTRIES],
            count: 0,
        }
    }

    /// Add entry to boot order
    pub fn push(&mut self, entry_id: u16) -> bool {
        if self.count < MAX_BOOT_ENTRIES {
            self.order[self.count] = entry_id;
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// Remove entry from boot order
    pub fn remove(&mut self, entry_id: u16) -> bool {
        if let Some(pos) = self.find(entry_id) {
            // Shift remaining entries
            for i in pos..self.count - 1 {
                self.order[i] = self.order[i + 1];
            }
            self.count -= 1;
            true
        } else {
            false
        }
    }

    /// Find entry position
    pub fn find(&self, entry_id: u16) -> Option<usize> {
        self.order[..self.count]
            .iter()
            .position(|&id| id == entry_id)
    }

    /// Move entry to position
    pub fn move_to(&mut self, entry_id: u16, new_pos: usize) -> bool {
        if let Some(old_pos) = self.find(entry_id) {
            let new_pos = new_pos.min(self.count - 1);
            if old_pos != new_pos {
                let id = self.order[old_pos];
                if old_pos < new_pos {
                    for i in old_pos..new_pos {
                        self.order[i] = self.order[i + 1];
                    }
                } else {
                    for i in (new_pos..old_pos).rev() {
                        self.order[i + 1] = self.order[i];
                    }
                }
                self.order[new_pos] = id;
            }
            true
        } else {
            false
        }
    }

    /// Get first entry
    pub fn first(&self) -> Option<u16> {
        if self.count > 0 {
            Some(self.order[0])
        } else {
            None
        }
    }
}

// =============================================================================
// AUTO-DETECTION
// =============================================================================

/// Auto-detected OS type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetectedOs {
    /// Unknown
    #[default]
    Unknown,
    /// Helix OS
    Helix,
    /// Linux distribution
    Linux,
    /// Windows
    Windows,
    /// macOS
    MacOs,
    /// FreeBSD
    FreeBsd,
    /// OpenBSD
    OpenBsd,
    /// NetBSD
    NetBsd,
    /// Other EFI
    OtherEfi,
}

impl fmt::Display for DetectedOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectedOs::Unknown => write!(f, "Unknown"),
            DetectedOs::Helix => write!(f, "Helix OS"),
            DetectedOs::Linux => write!(f, "Linux"),
            DetectedOs::Windows => write!(f, "Windows"),
            DetectedOs::MacOs => write!(f, "macOS"),
            DetectedOs::FreeBsd => write!(f, "FreeBSD"),
            DetectedOs::OpenBsd => write!(f, "OpenBSD"),
            DetectedOs::NetBsd => write!(f, "NetBSD"),
            DetectedOs::OtherEfi => write!(f, "EFI Application"),
        }
    }
}

/// Detection hint
#[derive(Debug, Clone, Copy)]
pub struct DetectionHint {
    /// File path pattern
    pub path_pattern: &'static str,
    /// Detected OS type
    pub os_type: DetectedOs,
    /// Entry type
    pub entry_type: BootEntryType,
    /// Default title
    pub default_title: &'static str,
}

/// Standard detection hints
pub const DETECTION_HINTS: &[DetectionHint] = &[
    DetectionHint {
        path_pattern: "\\EFI\\helix\\",
        os_type: DetectedOs::Helix,
        entry_type: BootEntryType::Linux,
        default_title: "Helix OS",
    },
    DetectionHint {
        path_pattern: "\\EFI\\Microsoft\\Boot\\bootmgfw.efi",
        os_type: DetectedOs::Windows,
        entry_type: BootEntryType::Windows,
        default_title: "Windows Boot Manager",
    },
    DetectionHint {
        path_pattern: "\\EFI\\BOOT\\BOOTX64.EFI",
        os_type: DetectedOs::OtherEfi,
        entry_type: BootEntryType::EfiApp,
        default_title: "EFI Default Loader",
    },
    DetectionHint {
        path_pattern: "\\EFI\\ubuntu\\",
        os_type: DetectedOs::Linux,
        entry_type: BootEntryType::Linux,
        default_title: "Ubuntu",
    },
    DetectionHint {
        path_pattern: "\\EFI\\fedora\\",
        os_type: DetectedOs::Linux,
        entry_type: BootEntryType::Linux,
        default_title: "Fedora",
    },
    DetectionHint {
        path_pattern: "\\EFI\\debian\\",
        os_type: DetectedOs::Linux,
        entry_type: BootEntryType::Linux,
        default_title: "Debian",
    },
    DetectionHint {
        path_pattern: "\\EFI\\arch\\",
        os_type: DetectedOs::Linux,
        entry_type: BootEntryType::Linux,
        default_title: "Arch Linux",
    },
    DetectionHint {
        path_pattern: "\\EFI\\FreeBSD\\",
        os_type: DetectedOs::FreeBsd,
        entry_type: BootEntryType::Bsd,
        default_title: "FreeBSD",
    },
    DetectionHint {
        path_pattern: "\\EFI\\OpenBSD\\",
        os_type: DetectedOs::OpenBsd,
        entry_type: BootEntryType::Bsd,
        default_title: "OpenBSD",
    },
];

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_boot_entry_flags() {
        let mut flags = BootEntryFlags::default_enabled();
        assert!(flags.contains(BootEntryFlags::ENABLED));

        flags.set(BootEntryFlags::DEFAULT);
        assert!(flags.contains(BootEntryFlags::DEFAULT));

        flags.clear(BootEntryFlags::DEFAULT);
        assert!(!flags.contains(BootEntryFlags::DEFAULT));
    }

    #[test]
    fn test_boot_entry_builder() {
        let entry = LinuxEntryBuilder::new()
            .title("Test Linux")
            .kernel("/boot/vmlinuz")
            .initrd("/boot/initrd.img")
            .cmdline("root=/dev/sda1 quiet")
            .default()
            .build();

        assert_eq!(entry.entry_type, BootEntryType::Linux);
        assert!(entry.is_default());
        assert_eq!(entry.title_str(), "Test Linux");
        assert_eq!(entry.path_str(), "/boot/vmlinuz");
    }

    #[test]
    fn test_menu_state() {
        let mut state = MenuState::default();

        let mut item = MenuItem::default();
        item.label[..4].copy_from_slice(b"Test");
        item.label_len = 4;

        assert!(state.add_item(item));
        assert_eq!(state.item_count, 1);
        assert!(state.current_item().is_some());
    }

    #[test]
    fn test_boot_order() {
        let mut order = BootOrder::new();

        order.push(1);
        order.push(2);
        order.push(3);

        assert_eq!(order.count, 3);
        assert_eq!(order.first(), Some(1));

        order.move_to(3, 0);
        assert_eq!(order.order[0], 3);

        order.remove(2);
        assert_eq!(order.count, 2);
    }

    #[test]
    fn test_entry_type() {
        assert!(BootEntryType::Linux.is_bootable());
        assert!(!BootEntryType::Shutdown.is_bootable());
        assert!(BootEntryType::Linux.requires_path());
        assert!(!BootEntryType::Reboot.requires_path());
    }
}
