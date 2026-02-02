//! Error Handling and Recovery System
//!
//! This module provides comprehensive error handling, recovery mechanisms,
//! and fallback strategies for the Helix UEFI Bootloader.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Error Handling System                               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Error Categories                               │   │
//! │  │  Boot │ Memory │ Storage │ Network │ Security │ Graphics │ ...  │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Recovery Strategies                            │   │
//! │  │  Retry │ Fallback │ Safe Mode │ Recovery │ Manual                │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Error Reporting                                │   │
//! │  │  Log │ Display │ Beep │ Serial │ Network                        │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

use core::fmt;

// =============================================================================
// ERROR CATEGORIES
// =============================================================================

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Boot process errors
    Boot,
    /// Memory errors
    Memory,
    /// Storage/filesystem errors
    Storage,
    /// Network errors
    Network,
    /// Security errors
    Security,
    /// Graphics/display errors
    Graphics,
    /// Configuration errors
    Config,
    /// Hardware errors
    Hardware,
    /// Protocol errors
    Protocol,
    /// Internal errors
    Internal,
    /// User errors
    User,
}

impl Default for ErrorCategory {
    fn default() -> Self {
        ErrorCategory::Internal
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Boot => write!(f, "Boot"),
            ErrorCategory::Memory => write!(f, "Memory"),
            ErrorCategory::Storage => write!(f, "Storage"),
            ErrorCategory::Network => write!(f, "Network"),
            ErrorCategory::Security => write!(f, "Security"),
            ErrorCategory::Graphics => write!(f, "Graphics"),
            ErrorCategory::Config => write!(f, "Configuration"),
            ErrorCategory::Hardware => write!(f, "Hardware"),
            ErrorCategory::Protocol => write!(f, "Protocol"),
            ErrorCategory::Internal => write!(f, "Internal"),
            ErrorCategory::User => write!(f, "User"),
        }
    }
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Debug information
    Debug,
    /// Informational
    Info,
    /// Warning (non-fatal)
    Warning,
    /// Error (potentially fatal)
    Error,
    /// Critical (likely fatal)
    Critical,
    /// Fatal (unrecoverable)
    Fatal,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Error
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Debug => write!(f, "DEBUG"),
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Critical => write!(f, "CRIT"),
            Severity::Fatal => write!(f, "FATAL"),
        }
    }
}

// =============================================================================
// ERROR CODES
// =============================================================================

/// Error code structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(u32);

impl ErrorCode {
    /// Create new error code
    pub const fn new(category: ErrorCategory, code: u16) -> Self {
        let cat = category as u32;
        Self((cat << 16) | (code as u32))
    }

    /// Get category
    pub fn category(&self) -> ErrorCategory {
        match (self.0 >> 16) as u8 {
            0 => ErrorCategory::Boot,
            1 => ErrorCategory::Memory,
            2 => ErrorCategory::Storage,
            3 => ErrorCategory::Network,
            4 => ErrorCategory::Security,
            5 => ErrorCategory::Graphics,
            6 => ErrorCategory::Config,
            7 => ErrorCategory::Hardware,
            8 => ErrorCategory::Protocol,
            9 => ErrorCategory::Internal,
            _ => ErrorCategory::User,
        }
    }

    /// Get code within category
    pub const fn code(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    /// Get raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04X}", self.0)
    }
}

/// Standard error codes
pub mod codes {
    use super::*;

    // Boot errors (0x0000-0x00FF)
    pub const BOOT_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0001);
    pub const BOOT_TIMEOUT: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0002);
    pub const BOOT_CANCELLED: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0003);
    pub const KERNEL_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0010);
    pub const KERNEL_INVALID: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0011);
    pub const KERNEL_LOAD_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0012);
    pub const INITRD_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0020);
    pub const INITRD_LOAD_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0021);
    pub const ENTRY_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0030);
    pub const EXIT_BOOT_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Boot, 0x0040);

    // Memory errors (0x0100-0x01FF)
    pub const OUT_OF_MEMORY: ErrorCode = ErrorCode::new(ErrorCategory::Memory, 0x0001);
    pub const ALLOCATION_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Memory, 0x0002);
    pub const INVALID_ADDRESS: ErrorCode = ErrorCode::new(ErrorCategory::Memory, 0x0003);
    pub const MEMORY_MAP_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Memory, 0x0010);
    pub const PAGE_FAULT: ErrorCode = ErrorCode::new(ErrorCategory::Memory, 0x0020);
    pub const BUFFER_OVERFLOW: ErrorCode = ErrorCode::new(ErrorCategory::Memory, 0x0030);

    // Storage errors (0x0200-0x02FF)
    pub const DEVICE_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0001);
    pub const READ_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0002);
    pub const WRITE_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0003);
    pub const FILESYSTEM_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0010);
    pub const FILE_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0011);
    pub const PARTITION_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0020);
    pub const ESP_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Storage, 0x0021);

    // Network errors (0x0300-0x03FF)
    pub const NO_NETWORK: ErrorCode = ErrorCode::new(ErrorCategory::Network, 0x0001);
    pub const DHCP_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Network, 0x0002);
    pub const DNS_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Network, 0x0003);
    pub const CONNECTION_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Network, 0x0010);
    pub const DOWNLOAD_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Network, 0x0011);
    pub const PXE_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Network, 0x0020);

    // Security errors (0x0400-0x04FF)
    pub const SECURE_BOOT_VIOLATION: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0001);
    pub const SIGNATURE_INVALID: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0002);
    pub const CERTIFICATE_INVALID: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0003);
    pub const TPM_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0010);
    pub const PASSWORD_REQUIRED: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0020);
    pub const PASSWORD_INCORRECT: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0021);
    pub const LOCKOUT_ACTIVE: ErrorCode = ErrorCode::new(ErrorCategory::Security, 0x0022);

    // Graphics errors (0x0500-0x05FF)
    pub const NO_DISPLAY: ErrorCode = ErrorCode::new(ErrorCategory::Graphics, 0x0001);
    pub const MODE_SET_FAILED: ErrorCode = ErrorCode::new(ErrorCategory::Graphics, 0x0002);
    pub const FRAMEBUFFER_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Graphics, 0x0003);

    // Config errors (0x0600-0x06FF)
    pub const CONFIG_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Config, 0x0001);
    pub const CONFIG_INVALID: ErrorCode = ErrorCode::new(ErrorCategory::Config, 0x0002);
    pub const CONFIG_PARSE_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Config, 0x0003);
    pub const VARIABLE_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Config, 0x0010);

    // Hardware errors (0x0700-0x07FF)
    pub const CPU_EXCEPTION: ErrorCode = ErrorCode::new(ErrorCategory::Hardware, 0x0001);
    pub const DEVICE_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Hardware, 0x0002);
    pub const UNSUPPORTED_HARDWARE: ErrorCode = ErrorCode::new(ErrorCategory::Hardware, 0x0003);

    // Protocol errors (0x0800-0x08FF)
    pub const PROTOCOL_NOT_FOUND: ErrorCode = ErrorCode::new(ErrorCategory::Protocol, 0x0001);
    pub const PROTOCOL_ERROR: ErrorCode = ErrorCode::new(ErrorCategory::Protocol, 0x0002);
}

// =============================================================================
// BOOT ERROR
// =============================================================================

/// Maximum error message length
pub const MAX_ERROR_MESSAGE: usize = 256;

/// Boot error structure
#[derive(Debug, Clone)]
pub struct BootError {
    /// Error code
    pub code: ErrorCode,
    /// Severity
    pub severity: Severity,
    /// Message
    pub message: [u8; MAX_ERROR_MESSAGE],
    /// Message length
    pub message_len: usize,
    /// Source file
    pub file: [u8; 64],
    /// File length
    pub file_len: usize,
    /// Source line
    pub line: u32,
    /// Timestamp (microseconds since boot)
    pub timestamp_us: u64,
    /// Recovery attempted
    pub recovery_attempted: bool,
    /// Recovery succeeded
    pub recovery_succeeded: bool,
    /// UEFI status (if applicable)
    pub uefi_status: u64,
}

impl Default for BootError {
    fn default() -> Self {
        Self {
            code: codes::BOOT_FAILED,
            severity: Severity::Error,
            message: [0u8; MAX_ERROR_MESSAGE],
            message_len: 0,
            file: [0u8; 64],
            file_len: 0,
            line: 0,
            timestamp_us: 0,
            recovery_attempted: false,
            recovery_succeeded: false,
            uefi_status: 0,
        }
    }
}

impl BootError {
    /// Create new error
    pub fn new(code: ErrorCode, severity: Severity) -> Self {
        Self {
            code,
            severity,
            ..Default::default()
        }
    }

    /// Get message as string
    pub fn message_str(&self) -> &str {
        if self.message_len > 0 {
            core::str::from_utf8(&self.message[..self.message_len]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Set message
    pub fn set_message(&mut self, msg: &str) {
        let bytes = msg.as_bytes();
        let len = bytes.len().min(MAX_ERROR_MESSAGE);
        self.message[..len].copy_from_slice(&bytes[..len]);
        self.message_len = len;
    }

    /// Check if fatal
    pub const fn is_fatal(&self) -> bool {
        matches!(self.severity, Severity::Fatal | Severity::Critical)
    }
}

// =============================================================================
// RECOVERY STRATEGIES
// =============================================================================

/// Recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// No recovery possible
    None,
    /// Retry the operation
    Retry,
    /// Use fallback option
    Fallback,
    /// Enter safe mode
    SafeMode,
    /// Enter recovery mode
    RecoveryMode,
    /// Boot last known good
    LastKnownGood,
    /// Reset to defaults
    ResetDefaults,
    /// Manual intervention required
    Manual,
    /// Reboot system
    Reboot,
    /// Shutdown system
    Shutdown,
    /// Enter UEFI shell
    UefiShell,
    /// Enter firmware setup
    FirmwareSetup,
}

impl Default for RecoveryStrategy {
    fn default() -> Self {
        RecoveryStrategy::None
    }
}

impl fmt::Display for RecoveryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryStrategy::None => write!(f, "None"),
            RecoveryStrategy::Retry => write!(f, "Retry"),
            RecoveryStrategy::Fallback => write!(f, "Fallback"),
            RecoveryStrategy::SafeMode => write!(f, "Safe Mode"),
            RecoveryStrategy::RecoveryMode => write!(f, "Recovery Mode"),
            RecoveryStrategy::LastKnownGood => write!(f, "Last Known Good"),
            RecoveryStrategy::ResetDefaults => write!(f, "Reset Defaults"),
            RecoveryStrategy::Manual => write!(f, "Manual"),
            RecoveryStrategy::Reboot => write!(f, "Reboot"),
            RecoveryStrategy::Shutdown => write!(f, "Shutdown"),
            RecoveryStrategy::UefiShell => write!(f, "UEFI Shell"),
            RecoveryStrategy::FirmwareSetup => write!(f, "Firmware Setup"),
        }
    }
}

/// Recovery action
#[derive(Debug, Clone, Copy)]
pub struct RecoveryAction {
    /// Strategy
    pub strategy: RecoveryStrategy,
    /// Max retry count
    pub max_retries: u8,
    /// Retry delay (milliseconds)
    pub retry_delay_ms: u32,
    /// Fallback entry index
    pub fallback_entry: u16,
    /// Timeout (seconds, 0 = immediate)
    pub timeout_secs: u16,
    /// Auto-execute (no user confirmation)
    pub auto_execute: bool,
}

impl Default for RecoveryAction {
    fn default() -> Self {
        Self {
            strategy: RecoveryStrategy::None,
            max_retries: 3,
            retry_delay_ms: 1000,
            fallback_entry: 0,
            timeout_secs: 10,
            auto_execute: false,
        }
    }
}

/// Get recommended recovery for error
pub fn recommended_recovery(code: ErrorCode) -> RecoveryAction {
    match code {
        codes::BOOT_TIMEOUT => RecoveryAction {
            strategy: RecoveryStrategy::Manual,
            timeout_secs: 30,
            ..Default::default()
        },
        codes::KERNEL_NOT_FOUND | codes::KERNEL_LOAD_FAILED => RecoveryAction {
            strategy: RecoveryStrategy::Fallback,
            timeout_secs: 10,
            ..Default::default()
        },
        codes::OUT_OF_MEMORY => RecoveryAction {
            strategy: RecoveryStrategy::SafeMode,
            timeout_secs: 5,
            auto_execute: true,
            ..Default::default()
        },
        codes::SECURE_BOOT_VIOLATION | codes::SIGNATURE_INVALID => RecoveryAction {
            strategy: RecoveryStrategy::Manual,
            timeout_secs: 0,
            ..Default::default()
        },
        codes::NO_NETWORK | codes::DHCP_FAILED => RecoveryAction {
            strategy: RecoveryStrategy::Retry,
            max_retries: 3,
            retry_delay_ms: 2000,
            ..Default::default()
        },
        codes::CONFIG_INVALID | codes::CONFIG_PARSE_ERROR => RecoveryAction {
            strategy: RecoveryStrategy::ResetDefaults,
            timeout_secs: 10,
            ..Default::default()
        },
        codes::CPU_EXCEPTION | codes::DEVICE_ERROR => RecoveryAction {
            strategy: RecoveryStrategy::Reboot,
            timeout_secs: 5,
            ..Default::default()
        },
        _ => RecoveryAction::default(),
    }
}

// =============================================================================
// ERROR REPORTING
// =============================================================================

/// Error output destination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorOutput {
    /// Console/screen
    Console,
    /// Serial port
    Serial,
    /// Log file
    LogFile,
    /// UEFI variable
    Variable,
    /// Network (syslog)
    Network,
    /// Beep codes
    Beep,
    /// All outputs
    All,
}

impl Default for ErrorOutput {
    fn default() -> Self {
        ErrorOutput::Console
    }
}

/// Error report format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorFormat {
    /// Short one-line
    Short,
    /// Full details
    Full,
    /// Technical (for logs)
    Technical,
    /// User-friendly
    UserFriendly,
}

impl Default for ErrorFormat {
    fn default() -> Self {
        ErrorFormat::UserFriendly
    }
}

/// Error report configuration
#[derive(Debug, Clone, Copy)]
pub struct ErrorReportConfig {
    /// Output destination
    pub output: ErrorOutput,
    /// Report format
    pub format: ErrorFormat,
    /// Minimum severity to report
    pub min_severity: Severity,
    /// Include stack trace
    pub include_stack: bool,
    /// Include timestamp
    pub include_timestamp: bool,
    /// Include source location
    pub include_location: bool,
    /// Beep on error
    pub beep_enabled: bool,
    /// Pause after fatal error
    pub pause_on_fatal: bool,
}

impl Default for ErrorReportConfig {
    fn default() -> Self {
        Self {
            output: ErrorOutput::Console,
            format: ErrorFormat::UserFriendly,
            min_severity: Severity::Warning,
            include_stack: false,
            include_timestamp: true,
            include_location: true,
            beep_enabled: true,
            pause_on_fatal: true,
        }
    }
}

// =============================================================================
// BEEP CODES
// =============================================================================

/// Beep pattern element
#[derive(Debug, Clone, Copy)]
pub struct BeepElement {
    /// Frequency (Hz, 0 = silence)
    pub frequency: u16,
    /// Duration (milliseconds)
    pub duration_ms: u16,
}

/// Standard beep patterns
pub mod beep_patterns {
    use super::BeepElement;

    /// Success beep (short high)
    pub const SUCCESS: &[BeepElement] = &[BeepElement {
        frequency: 1000,
        duration_ms: 100,
    }];

    /// Warning beep (two medium)
    pub const WARNING: &[BeepElement] = &[
        BeepElement {
            frequency: 800,
            duration_ms: 200,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 800,
            duration_ms: 200,
        },
    ];

    /// Error beep (three low)
    pub const ERROR: &[BeepElement] = &[
        BeepElement {
            frequency: 500,
            duration_ms: 300,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 500,
            duration_ms: 300,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 500,
            duration_ms: 300,
        },
    ];

    /// Critical beep (long continuous)
    pub const CRITICAL: &[BeepElement] = &[BeepElement {
        frequency: 440,
        duration_ms: 1000,
    }];

    /// Memory error (1 long, 3 short)
    pub const MEMORY_ERROR: &[BeepElement] = &[
        BeepElement {
            frequency: 1000,
            duration_ms: 500,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 200,
        },
        BeepElement {
            frequency: 1000,
            duration_ms: 150,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 1000,
            duration_ms: 150,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 1000,
            duration_ms: 150,
        },
    ];

    /// Keyboard error (3 short)
    pub const KEYBOARD_ERROR: &[BeepElement] = &[
        BeepElement {
            frequency: 1000,
            duration_ms: 150,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 1000,
            duration_ms: 150,
        },
        BeepElement {
            frequency: 0,
            duration_ms: 100,
        },
        BeepElement {
            frequency: 1000,
            duration_ms: 150,
        },
    ];
}

// =============================================================================
// ERROR LOG
// =============================================================================

/// Maximum log entries
pub const MAX_LOG_ENTRIES: usize = 32;

/// Error log entry
#[derive(Debug, Clone, Copy)]
pub struct LogEntry {
    /// Error code
    pub code: ErrorCode,
    /// Severity
    pub severity: Severity,
    /// Timestamp (microseconds)
    pub timestamp_us: u64,
    /// Short message
    pub message: [u8; 64],
    /// Message length
    pub message_len: u8,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            code: ErrorCode::new(ErrorCategory::Internal, 0),
            severity: Severity::Info,
            timestamp_us: 0,
            message: [0u8; 64],
            message_len: 0,
        }
    }
}

/// Error log
#[derive(Debug)]
pub struct ErrorLog {
    /// Entries
    entries: [LogEntry; MAX_LOG_ENTRIES],
    /// Entry count
    count: usize,
    /// Write index (circular)
    write_index: usize,
    /// Error count by severity
    error_counts: [u32; 6],
}

impl Default for ErrorLog {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorLog {
    /// Create new error log
    pub const fn new() -> Self {
        Self {
            entries: [LogEntry {
                code: ErrorCode(0),
                severity: Severity::Info,
                timestamp_us: 0,
                message: [0u8; 64],
                message_len: 0,
            }; MAX_LOG_ENTRIES],
            count: 0,
            write_index: 0,
            error_counts: [0; 6],
        }
    }

    /// Add log entry
    pub fn add(&mut self, entry: LogEntry) {
        self.entries[self.write_index] = entry;
        self.write_index = (self.write_index + 1) % MAX_LOG_ENTRIES;
        if self.count < MAX_LOG_ENTRIES {
            self.count += 1;
        }

        // Update counts
        let severity_idx = entry.severity as usize;
        if severity_idx < 6 {
            self.error_counts[severity_idx] += 1;
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

    /// Get error count for severity
    pub fn count_by_severity(&self, severity: Severity) -> u32 {
        let idx = severity as usize;
        if idx < 6 {
            self.error_counts[idx]
        } else {
            0
        }
    }

    /// Get total error count (Error + Critical + Fatal)
    pub fn total_errors(&self) -> u32 {
        self.error_counts[Severity::Error as usize]
            + self.error_counts[Severity::Critical as usize]
            + self.error_counts[Severity::Fatal as usize]
    }

    /// Clear log
    pub fn clear(&mut self) {
        self.count = 0;
        self.write_index = 0;
        self.error_counts = [0; 6];
    }
}

// =============================================================================
// ERROR SCREEN
// =============================================================================

/// Error screen type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorScreenType {
    /// Simple text message
    Text,
    /// Box with details
    Box,
    /// Full screen with options
    FullScreen,
    /// Blue screen style
    BlueScreen,
    /// Minimal/panic style
    Minimal,
}

impl Default for ErrorScreenType {
    fn default() -> Self {
        ErrorScreenType::Box
    }
}

/// Error screen content
#[derive(Debug, Clone, Copy)]
pub struct ErrorScreen {
    /// Screen type
    pub screen_type: ErrorScreenType,
    /// Title
    pub title: [u8; 64],
    /// Title length
    pub title_len: usize,
    /// Error code to display
    pub error_code: ErrorCode,
    /// Show recovery options
    pub show_options: bool,
    /// Available recovery strategies
    pub options: [RecoveryStrategy; 4],
    /// Number of options
    pub option_count: usize,
    /// Default option index
    pub default_option: usize,
    /// Countdown timer (0 = disabled)
    pub countdown_secs: u16,
}

impl Default for ErrorScreen {
    fn default() -> Self {
        Self {
            screen_type: ErrorScreenType::Box,
            title: [0u8; 64],
            title_len: 0,
            error_code: codes::BOOT_FAILED,
            show_options: true,
            options: [RecoveryStrategy::None; 4],
            option_count: 0,
            default_option: 0,
            countdown_secs: 0,
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code() {
        let code = ErrorCode::new(ErrorCategory::Memory, 0x0001);
        assert_eq!(code.category(), ErrorCategory::Memory);
        assert_eq!(code.code(), 0x0001);
    }

    #[test]
    fn test_boot_error() {
        let mut error = BootError::new(codes::KERNEL_NOT_FOUND, Severity::Critical);
        error.set_message("Kernel image not found");
        assert_eq!(error.message_str(), "Kernel image not found");
        assert!(error.is_fatal());
    }

    #[test]
    fn test_severity_order() {
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
        assert!(Severity::Critical < Severity::Fatal);
    }

    #[test]
    fn test_recovery_action() {
        let action = recommended_recovery(codes::NO_NETWORK);
        assert_eq!(action.strategy, RecoveryStrategy::Retry);
        assert_eq!(action.max_retries, 3);
    }

    #[test]
    fn test_error_log() {
        let mut log = ErrorLog::new();
        assert!(log.is_empty());

        log.add(LogEntry {
            code: codes::BOOT_FAILED,
            severity: Severity::Error,
            ..Default::default()
        });

        assert_eq!(log.len(), 1);
        assert_eq!(log.total_errors(), 1);
    }
}
