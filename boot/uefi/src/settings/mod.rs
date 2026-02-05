//! Boot Settings and Configuration Management
//!
//! This module provides comprehensive configuration management for the
//! Helix UEFI Bootloader including persistent storage and defaults.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    Settings Management                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
//! │  │  Boot Settings  │  │ Display Settings│  │ Security Settings│        │
//! │  │  • Timeout      │  │  • Resolution   │  │  • SecureBoot   │         │
//! │  │  • Default OS   │  │  • Theme        │  │  • Password     │         │
//! │  │  • Quick Boot   │  │  • Font Size    │  │  • Encryption   │         │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
//! │                                                                         │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
//! │  │ Network Settings│  │ Storage Settings│  │ Debug Settings  │         │
//! │  │  • DHCP         │  │  • Boot Device  │  │  • Logging      │         │
//! │  │  • PXE          │  │  • Partitions   │  │  • Breakpoints  │         │
//! │  │  • HTTP Boot    │  │  • Filesystem   │  │  • Serial       │         │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// SETTING TYPES
// =============================================================================

/// Setting type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingType {
    /// Boolean value
    Boolean,
    /// Integer value
    Integer,
    /// String value
    String,
    /// Selection from list
    Selection,
    /// Password (hidden)
    Password,
    /// Color value
    Color,
    /// File path
    Path,
    /// Key binding
    KeyBind,
    /// Time duration
    Duration,
    /// Range value
    Range,
}

/// Setting visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingVisibility {
    /// Visible in menu
    #[default]
    Visible,
    /// Hidden from menu
    Hidden,
    /// Advanced (requires toggle)
    Advanced,
    /// Expert only
    Expert,
    /// Developer mode
    Developer,
}

/// Setting persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingPersistence {
    /// Volatile (session only)
    Volatile,
    /// Non-volatile (saved)
    #[default]
    NonVolatile,
    /// Write-once
    WriteOnce,
    /// Read-only
    ReadOnly,
}

// =============================================================================
// BOOT SETTINGS
// =============================================================================

/// Boot timeout behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeoutBehavior {
    /// Wait indefinitely
    WaitForever,
    /// Boot default after timeout
    BootDefault,
    /// Boot last selection after timeout
    BootLast,
    /// Countdown with visual indicator
    #[default]
    Countdown,
    /// Boot immediately (no menu)
    Immediate,
}

/// Boot mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BootMode {
    /// Normal boot
    #[default]
    Normal,
    /// Safe mode
    Safe,
    /// Recovery mode
    Recovery,
    /// Diagnostic mode
    Diagnostic,
    /// Setup mode
    Setup,
    /// Firmware update mode
    FirmwareUpdate,
}

impl fmt::Display for BootMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootMode::Normal => write!(f, "Normal"),
            BootMode::Safe => write!(f, "Safe Mode"),
            BootMode::Recovery => write!(f, "Recovery"),
            BootMode::Diagnostic => write!(f, "Diagnostic"),
            BootMode::Setup => write!(f, "Setup"),
            BootMode::FirmwareUpdate => write!(f, "Firmware Update"),
        }
    }
}

/// Quick boot options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuickBootMode {
    /// Disabled
    #[default]
    Disabled,
    /// Skip memory test
    SkipMemoryTest,
    /// Skip device enumeration
    SkipEnumeration,
    /// Minimal initialization
    Minimal,
    /// Ultra-fast (dangerous)
    UltraFast,
}

/// Boot settings structure
#[derive(Debug, Clone, Copy)]
pub struct BootSettings {
    /// Timeout in seconds (0 = immediate, 0xFFFF = wait forever)
    pub timeout_secs: u16,
    /// Timeout behavior
    pub timeout_behavior: TimeoutBehavior,
    /// Default boot entry index
    pub default_entry: u16,
    /// Boot mode
    pub boot_mode: BootMode,
    /// Quick boot mode
    pub quick_boot: QuickBootMode,
    /// Show boot menu
    pub show_menu: bool,
    /// Remember last selection
    pub remember_last: bool,
    /// Enable hotkeys
    pub enable_hotkeys: bool,
    /// Boot order (8 entries max)
    pub boot_order: [u16; 8],
    /// Number of valid boot order entries
    pub boot_order_count: u8,
}

impl Default for BootSettings {
    fn default() -> Self {
        Self {
            timeout_secs: 5,
            timeout_behavior: TimeoutBehavior::Countdown,
            default_entry: 0,
            boot_mode: BootMode::Normal,
            quick_boot: QuickBootMode::Disabled,
            show_menu: true,
            remember_last: false,
            enable_hotkeys: true,
            boot_order: [0; 8],
            boot_order_count: 0,
        }
    }
}

// =============================================================================
// DISPLAY SETTINGS
// =============================================================================

/// Display theme
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayTheme {
    /// Classic VGA style
    Classic,
    /// Dark theme
    Dark,
    /// Light theme
    Light,
    /// High contrast
    HighContrast,
    /// System default
    System,
    /// Custom theme
    Custom,
    /// Helix branded
    #[default]
    Helix,
    /// Minimal
    Minimal,
}

impl fmt::Display for DisplayTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayTheme::Classic => write!(f, "Classic"),
            DisplayTheme::Dark => write!(f, "Dark"),
            DisplayTheme::Light => write!(f, "Light"),
            DisplayTheme::HighContrast => write!(f, "High Contrast"),
            DisplayTheme::System => write!(f, "System"),
            DisplayTheme::Custom => write!(f, "Custom"),
            DisplayTheme::Helix => write!(f, "Helix"),
            DisplayTheme::Minimal => write!(f, "Minimal"),
        }
    }
}

/// Font size
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontSize {
    /// Tiny (8pt equivalent)
    Tiny,
    /// Small (10pt)
    Small,
    /// Normal (12pt)
    #[default]
    Normal,
    /// Large (14pt)
    Large,
    /// Huge (16pt)
    Huge,
    /// Custom pixel height
    Custom(u8),
}

impl FontSize {
    /// Get pixel height
    pub const fn pixel_height(&self) -> u8 {
        match self {
            FontSize::Tiny => 8,
            FontSize::Small => 10,
            FontSize::Normal => 12,
            FontSize::Large => 14,
            FontSize::Huge => 16,
            FontSize::Custom(h) => *h,
        }
    }
}

/// Resolution preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolutionPref {
    /// Native/highest available
    Native,
    /// Auto-detect optimal
    #[default]
    Auto,
    /// Specific resolution
    Fixed {
        /// Width in pixels
        width: u32,
        /// Height in pixels
        height: u32,
    },
    /// Text mode
    TextMode,
    /// Low resolution (safe)
    Low,
}

/// Animation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationLevel {
    /// No animations
    None,
    /// Minimal animations
    Minimal,
    /// Normal animations
    #[default]
    Normal,
    /// Full animations
    Full,
}

/// Display settings structure
#[derive(Debug, Clone, Copy)]
pub struct DisplaySettings {
    /// Theme
    pub theme: DisplayTheme,
    /// Font size
    pub font_size: FontSize,
    /// Resolution preference
    pub resolution: ResolutionPref,
    /// Animation level
    pub animation: AnimationLevel,
    /// Show logo
    pub show_logo: bool,
    /// Show progress bar
    pub show_progress: bool,
    /// Show status messages
    pub show_status: bool,
    /// Console visible
    pub console_visible: bool,
    /// Screen brightness (0-100)
    pub brightness: u8,
    /// Contrast (0-100)
    pub contrast: u8,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            theme: DisplayTheme::Helix,
            font_size: FontSize::Normal,
            resolution: ResolutionPref::Auto,
            animation: AnimationLevel::Normal,
            show_logo: true,
            show_progress: true,
            show_status: true,
            console_visible: false,
            brightness: 100,
            contrast: 50,
        }
    }
}

// =============================================================================
// SECURITY SETTINGS
// =============================================================================

/// Password type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PasswordType {
    /// No password
    #[default]
    None,
    /// Menu password
    Menu,
    /// Boot password
    Boot,
    /// Setup password
    Setup,
    /// Both menu and boot
    Full,
}

/// Secure Boot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecureBootMode {
    /// Disabled
    #[default]
    Disabled,
    /// Standard mode
    Standard,
    /// Custom mode
    Custom,
    /// Deployed mode
    Deployed,
    /// Audit mode
    Audit,
}

/// Key database action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDbAction {
    /// No action
    None,
    /// Clear database
    Clear,
    /// Reset to default
    Reset,
    /// Enroll new key
    Enroll,
    /// Export database
    Export,
    /// Import database
    Import,
}

/// Security settings structure
#[derive(Debug, Clone, Copy)]
pub struct SecuritySettings {
    /// Password type required
    pub password_type: PasswordType,
    /// Password hash (32 bytes)
    pub password_hash: [u8; 32],
    /// Secure Boot mode
    pub secure_boot: SecureBootMode,
    /// Require signed loaders
    pub require_signed: bool,
    /// Enable TPM measurements
    pub tpm_enabled: bool,
    /// Lockout after failures
    pub lockout_enabled: bool,
    /// Lockout threshold (attempts)
    pub lockout_threshold: u8,
    /// Lockout duration (seconds)
    pub lockout_duration: u16,
    /// Auto-lock timeout (seconds, 0 = disabled)
    pub auto_lock_secs: u16,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            password_type: PasswordType::None,
            password_hash: [0u8; 32],
            secure_boot: SecureBootMode::Disabled,
            require_signed: false,
            tpm_enabled: true,
            lockout_enabled: true,
            lockout_threshold: 3,
            lockout_duration: 30,
            auto_lock_secs: 0,
        }
    }
}

// =============================================================================
// NETWORK SETTINGS
// =============================================================================

/// Network mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    /// Disabled
    Disabled,
    /// DHCP
    #[default]
    Dhcp,
    /// Static IP
    Static,
    /// IPv6 only
    Ipv6Only,
    /// Dual stack
    DualStack,
}

/// PXE boot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PxeMode {
    /// Disabled
    #[default]
    Disabled,
    /// IPv4 PXE
    Ipv4,
    /// IPv6 PXE
    Ipv6,
    /// Both
    Both,
    /// HTTP boot
    HttpBoot,
}

/// Network settings structure
#[derive(Debug, Clone, Copy)]
pub struct NetworkSettings {
    /// Network mode
    pub mode: NetworkMode,
    /// PXE mode
    pub pxe: PxeMode,
    /// Enable HTTP boot
    pub http_boot: bool,
    /// DHCP timeout (seconds)
    pub dhcp_timeout: u16,
    /// Retry count
    pub retry_count: u8,
    /// Static IPv4 address (if mode == Static)
    pub static_ip: [u8; 4],
    /// Subnet mask
    pub subnet_mask: [u8; 4],
    /// Gateway
    pub gateway: [u8; 4],
    /// DNS server
    pub dns_server: [u8; 4],
    /// VLAN ID (0 = disabled)
    pub vlan_id: u16,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Dhcp,
            pxe: PxeMode::Disabled,
            http_boot: false,
            dhcp_timeout: 10,
            retry_count: 3,
            static_ip: [0, 0, 0, 0],
            subnet_mask: [255, 255, 255, 0],
            gateway: [0, 0, 0, 0],
            dns_server: [8, 8, 8, 8],
            vlan_id: 0,
        }
    }
}

// =============================================================================
// STORAGE SETTINGS
// =============================================================================

/// Storage scan mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageScanMode {
    /// Scan all devices
    #[default]
    All,
    /// Scan internal only
    InternalOnly,
    /// Scan external only
    ExternalOnly,
    /// Scan first found
    FirstFound,
    /// Scan specified devices
    Specified,
}

/// Filesystem support flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilesystemSupport {
    /// Bitfield of supported filesystems:
    /// - bit 0: FAT12/16/32
    /// - bit 1: ext2/3/4
    /// - bit 2: NTFS
    /// - bit 3: ISO9660
    /// - bit 4: BTRFS
    /// - bit 5: XFS
    /// - bit 6: ZFS
    flags: u8,
}

impl FilesystemSupport {
    /// FAT filesystem flag
    pub const FAT: u8 = 1 << 0;
    /// ext2/3/4 filesystem flag
    pub const EXT: u8 = 1 << 1;
    /// NTFS filesystem flag
    pub const NTFS: u8 = 1 << 2;
    /// ISO9660 filesystem flag
    pub const ISO9660: u8 = 1 << 3;
    /// BTRFS filesystem flag
    pub const BTRFS: u8 = 1 << 4;
    /// XFS filesystem flag
    pub const XFS: u8 = 1 << 5;
    /// ZFS filesystem flag
    pub const ZFS: u8 = 1 << 6;

    /// Create a new FilesystemSupport with the given flags
    #[must_use]
    pub const fn new(flags: u8) -> Self {
        Self { flags }
    }

    /// Check if FAT is supported
    #[must_use]
    pub const fn fat(&self) -> bool {
        self.flags & Self::FAT != 0
    }

    /// Check if ext2/3/4 is supported
    #[must_use]
    pub const fn ext(&self) -> bool {
        self.flags & Self::EXT != 0
    }

    /// Check if NTFS is supported
    #[must_use]
    pub const fn ntfs(&self) -> bool {
        self.flags & Self::NTFS != 0
    }

    /// Check if ISO9660 is supported
    #[must_use]
    pub const fn iso9660(&self) -> bool {
        self.flags & Self::ISO9660 != 0
    }

    /// Check if BTRFS is supported
    #[must_use]
    pub const fn btrfs(&self) -> bool {
        self.flags & Self::BTRFS != 0
    }

    /// Check if XFS is supported
    #[must_use]
    pub const fn xfs(&self) -> bool {
        self.flags & Self::XFS != 0
    }

    /// Check if ZFS is supported
    #[must_use]
    pub const fn zfs(&self) -> bool {
        self.flags & Self::ZFS != 0
    }
}

impl Default for FilesystemSupport {
    fn default() -> Self {
        Self {
            flags: Self::FAT | Self::EXT | Self::ISO9660,
        }
    }
}

/// Storage settings structure
#[derive(Debug, Clone, Copy)]
pub struct StorageSettings {
    /// Scan mode
    pub scan_mode: StorageScanMode,
    /// Filesystem support
    pub filesystems: FilesystemSupport,
    /// Enable AHCI
    pub ahci_enabled: bool,
    /// Enable `NVMe`
    pub nvme_enabled: bool,
    /// Enable USB storage
    pub usb_storage: bool,
    /// Enable SD/MMC
    pub sdmmc_enabled: bool,
    /// Timeout for device detection (seconds)
    pub detect_timeout: u8,
    /// Cache size (KB)
    pub cache_size_kb: u16,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            scan_mode: StorageScanMode::All,
            filesystems: FilesystemSupport::default(),
            ahci_enabled: true,
            nvme_enabled: true,
            usb_storage: true,
            sdmmc_enabled: true,
            detect_timeout: 5,
            cache_size_kb: 256,
        }
    }
}

// =============================================================================
// DEBUG SETTINGS
// =============================================================================

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LogLevel {
    /// No logging
    Off,
    /// Error only
    Error,
    /// Warnings and errors
    #[default]
    Warning,
    /// Info and above
    Info,
    /// Debug and above
    Debug,
    /// Everything
    Trace,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Off => write!(f, "OFF"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

/// Log output destination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogOutput {
    /// No output
    None,
    /// Console (screen)
    #[default]
    Console,
    /// Serial port
    Serial,
    /// File
    File,
    /// Network (syslog)
    Network,
    /// Memory buffer
    Memory,
    /// All outputs
    All,
}

/// Serial port configuration
#[derive(Debug, Clone, Copy)]
pub struct SerialConfig {
    /// Port (COM1=0x3F8, COM2=0x2F8, etc.)
    pub port: u16,
    /// Baud rate
    pub baud: u32,
    /// Data bits (5-8)
    pub data_bits: u8,
    /// Stop bits (1-2)
    pub stop_bits: u8,
    /// Parity (0=none, 1=odd, 2=even)
    pub parity: u8,
    /// Flow control enabled
    pub flow_control: bool,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: 0x3F8, // COM1
            baud: 115_200,
            data_bits: 8,
            stop_bits: 1,
            parity: 0,
            flow_control: false,
        }
    }
}

/// Debug settings structure
#[derive(Debug, Clone, Copy)]
pub struct DebugSettings {
    /// Log level
    pub log_level: LogLevel,
    /// Log output
    pub log_output: LogOutput,
    /// Serial configuration
    pub serial: SerialConfig,
    /// Debug flags bitfield:
    /// - bit 0: debug_console
    /// - bit 1: break_on_start
    /// - bit 2: pause_before_boot
    /// - bit 3: show_timing
    /// - bit 4: show_memory_map
    /// - bit 5: assertions
    /// - bit 6: stack_canary
    flags: u8,
}

impl DebugSettings {
    /// Debug console flag
    pub const DEBUG_CONSOLE: u8 = 1 << 0;
    /// Break on start flag
    pub const BREAK_ON_START: u8 = 1 << 1;
    /// Pause before boot flag
    pub const PAUSE_BEFORE_BOOT: u8 = 1 << 2;
    /// Show timing flag
    pub const SHOW_TIMING: u8 = 1 << 3;
    /// Show memory map flag
    pub const SHOW_MEMORY_MAP: u8 = 1 << 4;
    /// Assertions enabled flag
    pub const ASSERTIONS: u8 = 1 << 5;
    /// Stack canary enabled flag
    pub const STACK_CANARY: u8 = 1 << 6;

    /// Check if debug console is enabled
    #[must_use]
    pub const fn debug_console(&self) -> bool {
        self.flags & Self::DEBUG_CONSOLE != 0
    }

    /// Check if break on start is enabled
    #[must_use]
    pub const fn break_on_start(&self) -> bool {
        self.flags & Self::BREAK_ON_START != 0
    }

    /// Check if pause before boot is enabled
    #[must_use]
    pub const fn pause_before_boot(&self) -> bool {
        self.flags & Self::PAUSE_BEFORE_BOOT != 0
    }

    /// Check if show timing is enabled
    #[must_use]
    pub const fn show_timing(&self) -> bool {
        self.flags & Self::SHOW_TIMING != 0
    }

    /// Check if show memory map is enabled
    #[must_use]
    pub const fn show_memory_map(&self) -> bool {
        self.flags & Self::SHOW_MEMORY_MAP != 0
    }

    /// Check if assertions are enabled
    #[must_use]
    pub const fn assertions(&self) -> bool {
        self.flags & Self::ASSERTIONS != 0
    }

    /// Check if stack canary is enabled
    #[must_use]
    pub const fn stack_canary(&self) -> bool {
        self.flags & Self::STACK_CANARY != 0
    }
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Warning,
            log_output: LogOutput::Console,
            serial: SerialConfig::default(),
            flags: Self::ASSERTIONS | Self::STACK_CANARY,
        }
    }
}

// =============================================================================
// POWER SETTINGS
// =============================================================================

/// Power behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerBehavior {
    /// Normal boot
    #[default]
    Normal,
    /// Wake on LAN
    WakeOnLan,
    /// Wake on RTC
    WakeOnRtc,
    /// Fast startup
    FastStartup,
    /// Hibernate resume
    HibernateResume,
}

/// Power settings structure
#[derive(Debug, Clone, Copy)]
pub struct PowerSettings {
    /// Power behavior
    pub behavior: PowerBehavior,
    /// Enable Wake-on-LAN
    pub wol_enabled: bool,
    /// Enable Wake-on-RTC
    pub wor_enabled: bool,
    /// RTC wake time (seconds from midnight)
    pub rtc_wake_time: u32,
    /// Battery threshold (percent)
    pub battery_threshold: u8,
    /// Power failure action
    pub power_fail_action: u8,
}

impl Default for PowerSettings {
    fn default() -> Self {
        Self {
            behavior: PowerBehavior::Normal,
            wol_enabled: false,
            wor_enabled: false,
            rtc_wake_time: 0,
            battery_threshold: 5,
            power_fail_action: 0,
        }
    }
}

// =============================================================================
// LOCALE SETTINGS
// =============================================================================

/// Language code (ISO 639-1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LanguageCode {
    /// English language
    #[default]
    English,
    /// French language
    French,
    /// German language
    German,
    /// Spanish language
    Spanish,
    /// Italian language
    Italian,
    /// Portuguese language
    Portuguese,
    /// Russian language
    Russian,
    /// Chinese language
    Chinese,
    /// Japanese language
    Japanese,
    /// Korean language
    Korean,
    /// Arabic language
    Arabic,
    /// Hebrew language
    Hebrew,
    /// Custom language code
    Custom([u8; 2]),
}

impl LanguageCode {
    /// Get ISO code
    pub const fn iso_code(&self) -> &'static str {
        match self {
            LanguageCode::English => "en",
            LanguageCode::French => "fr",
            LanguageCode::German => "de",
            LanguageCode::Spanish => "es",
            LanguageCode::Italian => "it",
            LanguageCode::Portuguese => "pt",
            LanguageCode::Russian => "ru",
            LanguageCode::Chinese => "zh",
            LanguageCode::Japanese => "ja",
            LanguageCode::Korean => "ko",
            LanguageCode::Arabic => "ar",
            LanguageCode::Hebrew => "he",
            LanguageCode::Custom(_) => "xx",
        }
    }
}

/// Keyboard layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardLayout {
    /// US QWERTY layout
    #[default]
    UsQwerty,
    /// UK QWERTY layout
    UkQwerty,
    /// French AZERTY layout
    FrAzerty,
    /// German QWERTZ layout
    DeQwertz,
    /// Spanish QWERTY layout
    EsQwerty,
    /// Italian QWERTY layout
    ItQwerty,
    /// Portuguese QWERTY layout
    PtQwerty,
    /// Russian JCUKEN layout
    RuJcuken,
    /// Japanese JIS layout
    JpJis,
    /// Korean Sebeolsik layout
    KoSebeolsik,
}

/// Locale settings structure
#[derive(Debug, Clone, Copy)]
pub struct LocaleSettings {
    /// Language
    pub language: LanguageCode,
    /// Keyboard layout
    pub keyboard: KeyboardLayout,
    /// Time format (12/24)
    pub time_24h: bool,
    /// Date format (0=MDY, 1=DMY, 2=YMD)
    pub date_format: u8,
    /// Timezone offset (minutes from UTC)
    pub timezone_offset: i16,
}

impl Default for LocaleSettings {
    fn default() -> Self {
        Self {
            language: LanguageCode::English,
            keyboard: KeyboardLayout::UsQwerty,
            time_24h: true,
            date_format: 2,
            timezone_offset: 0,
        }
    }
}

// =============================================================================
// MASTER SETTINGS
// =============================================================================

/// Settings version for compatibility
pub const SETTINGS_VERSION: u32 = 1;

/// Settings magic number
pub const SETTINGS_MAGIC: u32 = 0x4845_4C58; // "HELX"

/// Master settings header
#[derive(Debug, Clone, Copy)]
pub struct SettingsHeader {
    /// Magic number
    pub magic: u32,
    /// Version
    pub version: u32,
    /// Size in bytes
    pub size: u32,
    /// CRC32 checksum
    pub checksum: u32,
    /// Flags
    pub flags: u32,
    /// Reserved
    pub reserved: [u32; 3],
}

impl Default for SettingsHeader {
    fn default() -> Self {
        Self {
            magic: SETTINGS_MAGIC,
            version: SETTINGS_VERSION,
            size: 0,
            checksum: 0,
            flags: 0,
            reserved: [0; 3],
        }
    }
}

impl SettingsHeader {
    /// Validate header
    pub const fn is_valid(&self) -> bool {
        self.magic == SETTINGS_MAGIC && self.version == SETTINGS_VERSION
    }
}

/// Complete settings structure
#[derive(Debug, Clone, Copy, Default)]
pub struct Settings {
    /// Header
    pub header: SettingsHeader,
    /// Boot settings
    pub boot: BootSettings,
    /// Display settings
    pub display: DisplaySettings,
    /// Security settings
    pub security: SecuritySettings,
    /// Network settings
    pub network: NetworkSettings,
    /// Storage settings
    pub storage: StorageSettings,
    /// Debug settings
    pub debug: DebugSettings,
    /// Power settings
    pub power: PowerSettings,
    /// Locale settings
    pub locale: LocaleSettings,
}

impl Settings {
    /// Create new settings with defaults
    pub const fn new() -> Self {
        Self {
            header: SettingsHeader {
                magic: SETTINGS_MAGIC,
                version: SETTINGS_VERSION,
                size: 0,
                checksum: 0,
                flags: 0,
                reserved: [0; 3],
            },
            boot: BootSettings {
                timeout_secs: 5,
                timeout_behavior: TimeoutBehavior::Countdown,
                default_entry: 0,
                boot_mode: BootMode::Normal,
                quick_boot: QuickBootMode::Disabled,
                show_menu: true,
                remember_last: false,
                enable_hotkeys: true,
                boot_order: [0; 8],
                boot_order_count: 0,
            },
            display: DisplaySettings {
                theme: DisplayTheme::Helix,
                font_size: FontSize::Normal,
                resolution: ResolutionPref::Auto,
                animation: AnimationLevel::Normal,
                show_logo: true,
                show_progress: true,
                show_status: true,
                console_visible: false,
                brightness: 100,
                contrast: 50,
            },
            security: SecuritySettings {
                password_type: PasswordType::None,
                password_hash: [0u8; 32],
                secure_boot: SecureBootMode::Disabled,
                require_signed: false,
                tpm_enabled: true,
                lockout_enabled: true,
                lockout_threshold: 3,
                lockout_duration: 30,
                auto_lock_secs: 0,
            },
            network: NetworkSettings {
                mode: NetworkMode::Dhcp,
                pxe: PxeMode::Disabled,
                http_boot: false,
                dhcp_timeout: 10,
                retry_count: 3,
                static_ip: [0, 0, 0, 0],
                subnet_mask: [255, 255, 255, 0],
                gateway: [0, 0, 0, 0],
                dns_server: [8, 8, 8, 8],
                vlan_id: 0,
            },
            storage: StorageSettings {
                scan_mode: StorageScanMode::All,
                filesystems: FilesystemSupport {
                    fat: true,
                    ext: true,
                    ntfs: false,
                    iso9660: true,
                    btrfs: false,
                    xfs: false,
                    zfs: false,
                },
                ahci_enabled: true,
                nvme_enabled: true,
                usb_storage: true,
                sdmmc_enabled: true,
                detect_timeout: 5,
                cache_size_kb: 256,
            },
            debug: DebugSettings {
                log_level: LogLevel::Warning,
                log_output: LogOutput::Console,
                serial: SerialConfig {
                    port: 0x3F8,
                    baud: 115_200,
                    data_bits: 8,
                    stop_bits: 1,
                    parity: 0,
                    flow_control: false,
                },
                debug_console: false,
                break_on_start: false,
                pause_before_boot: false,
                show_timing: false,
                show_memory_map: false,
                assertions: true,
                stack_canary: true,
            },
            power: PowerSettings {
                behavior: PowerBehavior::Normal,
                wol_enabled: false,
                wor_enabled: false,
                rtc_wake_time: 0,
                battery_threshold: 5,
                power_fail_action: 0,
            },
            locale: LocaleSettings {
                language: LanguageCode::English,
                keyboard: KeyboardLayout::UsQwerty,
                time_24h: true,
                date_format: 2,
                timezone_offset: 0,
            },
        }
    }

    /// Reset to defaults
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Calculate size
    pub const fn size() -> usize {
        core::mem::size_of::<Settings>()
    }
}

// =============================================================================
// SETTING CHANGE NOTIFICATION
// =============================================================================

/// Setting change event
#[derive(Debug, Clone, Copy)]
pub struct SettingChange {
    /// Category
    pub category: SettingCategory,
    /// Setting ID
    pub id: u16,
    /// Old value
    pub old_value: u64,
    /// New value
    pub new_value: u64,
}

/// Setting category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingCategory {
    /// Boot-related settings
    Boot,
    /// Display-related settings
    Display,
    /// Security-related settings
    Security,
    /// Network-related settings
    Network,
    /// Storage-related settings
    Storage,
    /// Debug-related settings
    Debug,
    /// Power-related settings
    Power,
    /// Locale-related settings
    Locale,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert!(settings.header.is_valid());
        assert_eq!(settings.boot.timeout_secs, 5);
    }

    #[test]
    fn test_settings_new() {
        let settings = Settings::new();
        assert!(settings.header.is_valid());
    }

    #[test]
    fn test_font_size() {
        assert_eq!(FontSize::Normal.pixel_height(), 12);
        assert_eq!(FontSize::Custom(20).pixel_height(), 20);
    }

    #[test]
    fn test_language_code() {
        assert_eq!(LanguageCode::French.iso_code(), "fr");
        assert_eq!(LanguageCode::German.iso_code(), "de");
    }

    #[test]
    fn test_log_level_ord() {
        assert!(LogLevel::Error < LogLevel::Warning);
        assert!(LogLevel::Debug > LogLevel::Info);
    }
}
