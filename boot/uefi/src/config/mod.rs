//! Configuration File Parser
//!
//! Parser for bootloader configuration files in TOML-like format.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// CONFIGURATION STRUCTURES
// =============================================================================

/// Boot configuration
#[derive(Debug, Clone)]
pub struct BootConfig {
    /// Timeout before auto-boot (seconds)
    pub timeout: u32,
    /// Default entry index
    pub default_entry: usize,
    /// Enable verbose mode
    pub verbose: bool,
    /// Enable debug mode
    pub debug: bool,
    /// Log level
    pub log_level: LogLevel,
    /// Boot entries
    pub entries: Vec<BootEntry>,
    /// Global kernel parameters
    pub kernel_params: Vec<(String, String)>,
    /// Graphics mode preference
    pub graphics: GraphicsConfig,
    /// Security settings
    pub security: SecurityConfig,
}

impl BootConfig {
    /// Create default configuration
    pub fn new() -> Self {
        Self {
            timeout: 5,
            default_entry: 0,
            verbose: false,
            debug: false,
            log_level: LogLevel::Info,
            entries: Vec::new(),
            kernel_params: Vec::new(),
            graphics: GraphicsConfig::default(),
            security: SecurityConfig::default(),
        }
    }

    /// Parse from string
    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        let mut config = Self::new();
        let mut parser = ConfigParser::new(content);

        while let Some(line) = parser.next_line() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Section header
            if line.starts_with('[') {
                let section = line.trim_matches(|c| c == '[' || c == ']');
                parser.current_section = Some(String::from(section));

                // Handle boot entries
                if section.starts_with("entry.") {
                    let entry_name = section.strip_prefix("entry.").unwrap_or("");
                    config.entries.push(BootEntry::new(entry_name));
                }
                continue;
            }

            // Key-value pair
            if let Some((key, value)) = parse_key_value(line) {
                config.apply_setting(&parser.current_section, key, value)?;
            }
        }

        Ok(config)
    }

    /// Apply a setting
    fn apply_setting(
        &mut self,
        section: &Option<String>,
        key: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        match section.as_deref() {
            None | Some("boot") => match key {
                "timeout" => self.timeout = parse_u32(value)?,
                "default" => self.default_entry = parse_usize(value)?,
                "verbose" => self.verbose = parse_bool(value)?,
                "debug" => self.debug = parse_bool(value)?,
                "log_level" | "loglevel" => self.log_level = LogLevel::from_str(value)?,
                _ => {},
            },
            Some("graphics") => match key {
                "mode" => self.graphics.mode = GraphicsMode::from_str(value)?,
                "width" => self.graphics.width = Some(parse_u32(value)?),
                "height" => self.graphics.height = Some(parse_u32(value)?),
                "depth" | "bpp" => self.graphics.depth = Some(parse_u32(value)?),
                _ => {},
            },
            Some("security") => match key {
                "require_signature" | "require-signature" => {
                    self.security.require_signature = parse_bool(value)?;
                },
                "measure_boot" | "measure-boot" => {
                    self.security.measure_boot = parse_bool(value)?;
                },
                "verify_hash" | "verify-hash" => {
                    self.security.verify_hash = parse_bool(value)?;
                },
                _ => {},
            },
            Some(section) if section.starts_with("entry.") => {
                if let Some(entry) = self.entries.last_mut() {
                    match key {
                        "title" | "name" => entry.title = String::from(value),
                        "kernel" | "linux" => entry.kernel = String::from(value),
                        "initrd" | "initramfs" => entry.initrd = Some(String::from(value)),
                        "cmdline" | "options" | "append" => entry.cmdline = String::from(value),
                        "icon" => entry.icon = Some(String::from(value)),
                        "default" => entry.is_default = parse_bool(value)?,
                        "hidden" => entry.hidden = parse_bool(value)?,
                        _ => {},
                    }
                }
            },
            _ => {},
        }

        Ok(())
    }

    /// Get default entry
    pub fn default_entry(&self) -> Option<&BootEntry> {
        // First, check for explicitly marked default
        for entry in &self.entries {
            if entry.is_default {
                return Some(entry);
            }
        }

        // Otherwise use index
        self.entries.get(self.default_entry)
    }

    /// Get visible entries
    pub fn visible_entries(&self) -> impl Iterator<Item = &BootEntry> {
        self.entries.iter().filter(|e| !e.hidden)
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Boot entry
#[derive(Debug, Clone)]
pub struct BootEntry {
    /// Entry identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Kernel path
    pub kernel: String,
    /// Initrd path
    pub initrd: Option<String>,
    /// Kernel command line
    pub cmdline: String,
    /// Icon path
    pub icon: Option<String>,
    /// Is default entry
    pub is_default: bool,
    /// Is hidden
    pub hidden: bool,
}

impl BootEntry {
    /// Create new entry
    pub fn new(id: &str) -> Self {
        Self {
            id: String::from(id),
            title: String::from(id),
            kernel: String::new(),
            initrd: None,
            cmdline: String::new(),
            icon: None,
            is_default: false,
            hidden: false,
        }
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Trace level
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
    /// Silent
    Silent,
}

impl LogLevel {
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, ConfigError> {
        match s.to_lowercase().as_str() {
            "trace" | "0" => Ok(Self::Trace),
            "debug" | "1" => Ok(Self::Debug),
            "info" | "2" => Ok(Self::Info),
            "warn" | "warning" | "3" => Ok(Self::Warn),
            "error" | "4" => Ok(Self::Error),
            "silent" | "quiet" | "5" => Ok(Self::Silent),
            _ => Err(ConfigError::InvalidValue),
        }
    }
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Graphics configuration
#[derive(Debug, Clone)]
pub struct GraphicsConfig {
    /// Graphics mode
    pub mode: GraphicsMode,
    /// Preferred width
    pub width: Option<u32>,
    /// Preferred height
    pub height: Option<u32>,
    /// Color depth
    pub depth: Option<u32>,
}

impl Default for GraphicsConfig {
    fn default() -> Self {
        Self {
            mode: GraphicsMode::Auto,
            width: None,
            height: None,
            depth: None,
        }
    }
}

/// Graphics mode preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsMode {
    /// Automatic selection
    Auto,
    /// Text mode only
    Text,
    /// Native resolution
    Native,
    /// Specific resolution
    Specific,
    /// Largest available
    Largest,
}

impl GraphicsMode {
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self, ConfigError> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "text" => Ok(Self::Text),
            "native" => Ok(Self::Native),
            "specific" => Ok(Self::Specific),
            "largest" | "max" => Ok(Self::Largest),
            _ => Err(ConfigError::InvalidValue),
        }
    }
}

impl Default for GraphicsMode {
    fn default() -> Self {
        Self::Auto
    }
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Require valid signature
    pub require_signature: bool,
    /// Enable measured boot
    pub measure_boot: bool,
    /// Verify kernel hash
    pub verify_hash: bool,
    /// Expected kernel hash (hex)
    pub kernel_hash: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_signature: false,
            measure_boot: true,
            verify_hash: false,
            kernel_hash: None,
        }
    }
}

// =============================================================================
// PARSER
// =============================================================================

/// Configuration parser
struct ConfigParser<'a> {
    content: &'a str,
    lines: core::str::Lines<'a>,
    current_section: Option<String>,
}

impl<'a> ConfigParser<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            content,
            lines: content.lines(),
            current_section: None,
        }
    }

    fn next_line(&mut self) -> Option<&'a str> {
        self.lines.next()
    }
}

/// Parse key-value pair
fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let mut parts = line.splitn(2, '=');
    let key = parts.next()?.trim();
    let value = parts.next()?.trim();

    // Remove quotes
    let value = value.trim_matches('"').trim_matches('\'');

    Some((key, value))
}

/// Parse u32
fn parse_u32(s: &str) -> Result<u32, ConfigError> {
    s.parse().map_err(|_| ConfigError::InvalidValue)
}

/// Parse usize
fn parse_usize(s: &str) -> Result<usize, ConfigError> {
    s.parse().map_err(|_| ConfigError::InvalidValue)
}

/// Parse bool
fn parse_bool(s: &str) -> Result<bool, ConfigError> {
    match s.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" | "enabled" => Ok(true),
        "false" | "no" | "0" | "off" | "disabled" => Ok(false),
        _ => Err(ConfigError::InvalidValue),
    }
}

// =============================================================================
// ERRORS
// =============================================================================

/// Configuration error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// Invalid syntax
    InvalidSyntax,
    /// Invalid value
    InvalidValue,
    /// Missing required field
    MissingField,
    /// Unknown section
    UnknownSection,
    /// Duplicate entry
    DuplicateEntry,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONFIG: &str = r#"
# Boot configuration
timeout = 5
default = 0
verbose = false

[graphics]
mode = auto
width = 1920
height = 1080

[security]
require_signature = false
measure_boot = true

[entry.helix]
title = "Helix OS"
kernel = \EFI\HELIX\KERNEL
initrd = \EFI\HELIX\INITRD.IMG
cmdline = root=/dev/sda1 quiet

[entry.helix-debug]
title = "Helix OS (Debug)"
kernel = \EFI\HELIX\KERNEL
cmdline = root=/dev/sda1 debug loglevel=7
hidden = false
"#;

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true"), Ok(true));
        assert_eq!(parse_bool("false"), Ok(false));
        assert_eq!(parse_bool("yes"), Ok(true));
        assert_eq!(parse_bool("no"), Ok(false));
        assert_eq!(parse_bool("1"), Ok(true));
        assert_eq!(parse_bool("0"), Ok(false));
    }

    #[test]
    fn test_parse_key_value() {
        let (key, value) = parse_key_value("timeout = 5").unwrap();
        assert_eq!(key, "timeout");
        assert_eq!(value, "5");

        let (key, value) = parse_key_value("title = \"Helix OS\"").unwrap();
        assert_eq!(key, "title");
        assert_eq!(value, "Helix OS");
    }

    #[test]
    fn test_log_level() {
        assert_eq!(LogLevel::from_str("debug"), Ok(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("info"), Ok(LogLevel::Info));
        assert_eq!(LogLevel::from_str("error"), Ok(LogLevel::Error));
    }
}
