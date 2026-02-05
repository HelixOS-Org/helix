//! Parsing and Formatting Utilities
//!
//! This module provides comprehensive parsing and formatting utilities
//! for the Helix UEFI Bootloader, including number parsing, string
//! formatting, path manipulation, and configuration parsing.
//!
//! # Features
//!
//! - Number parsing (decimal, hex, binary, octal)
//! - String formatting without std
//! - Path manipulation
//! - Size formatting (KB, MB, GB)
//! - Time formatting
//! - UUID/GUID formatting
//! - Configuration file parsing

use core::fmt;

// =============================================================================
// NUMBER PARSING
// =============================================================================

/// Number parse error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Empty input
    Empty,
    /// Invalid character
    InvalidChar,
    /// Overflow
    Overflow,
    /// Invalid format
    InvalidFormat,
    /// Invalid base
    InvalidBase,
    /// Negative value for unsigned
    Negative,
}

impl Default for ParseError {
    fn default() -> Self {
        ParseError::Empty
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty input"),
            ParseError::InvalidChar => write!(f, "invalid character"),
            ParseError::Overflow => write!(f, "overflow"),
            ParseError::InvalidFormat => write!(f, "invalid format"),
            ParseError::InvalidBase => write!(f, "invalid base"),
            ParseError::Negative => write!(f, "negative value"),
        }
    }
}

/// Parse result
pub type ParseResult<T> = Result<T, ParseError>;

/// Parse unsigned integer with auto-detected base
pub fn parse_u64(s: &str) -> ParseResult<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    let bytes = s.as_bytes();

    // Detect base from prefix
    if bytes.len() >= 2 {
        if bytes[0] == b'0' {
            match bytes[1] {
                b'x' | b'X' => return parse_u64_radix(&s[2..], 16),
                b'b' | b'B' => return parse_u64_radix(&s[2..], 2),
                b'o' | b'O' => return parse_u64_radix(&s[2..], 8),
                _ if bytes[1].is_ascii_digit() => return parse_u64_radix(s, 10),
                _ => return Err(ParseError::InvalidFormat),
            }
        }
    }

    parse_u64_radix(s, 10)
}

/// Parse unsigned integer with specific radix
pub fn parse_u64_radix(s: &str, radix: u32) -> ParseResult<u64> {
    if !(2..=36).contains(&radix) {
        return Err(ParseError::InvalidBase);
    }

    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    let mut result: u64 = 0;
    for b in s.bytes() {
        let digit = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'z' => b - b'a' + 10,
            b'A'..=b'Z' => b - b'A' + 10,
            b'_' => continue, // Allow underscores as separators
            _ => return Err(ParseError::InvalidChar),
        };

        if digit as u32 >= radix {
            return Err(ParseError::InvalidChar);
        }

        result = result
            .checked_mul(radix as u64)
            .ok_or(ParseError::Overflow)?;
        result = result
            .checked_add(digit as u64)
            .ok_or(ParseError::Overflow)?;
    }

    Ok(result)
}

/// Parse signed integer
pub fn parse_i64(s: &str) -> ParseResult<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    let (negative, s) = if s.starts_with('-') {
        (true, &s[1..])
    } else if s.starts_with('+') {
        (false, &s[1..])
    } else {
        (false, s)
    };

    let unsigned = parse_u64(s)?;

    if negative {
        if unsigned > (i64::MAX as u64) + 1 {
            return Err(ParseError::Overflow);
        }
        Ok(-(unsigned as i64))
    } else {
        if unsigned > i64::MAX as u64 {
            return Err(ParseError::Overflow);
        }
        Ok(unsigned as i64)
    }
}

/// Parse boolean
pub fn parse_bool(s: &str) -> ParseResult<bool> {
    let s = s.trim().to_ascii_lowercase_owned();
    match s.as_str() {
        "true" | "yes" | "on" | "1" | "enabled" => Ok(true),
        "false" | "no" | "off" | "0" | "disabled" => Ok(false),
        _ => Err(ParseError::InvalidFormat),
    }
}

/// Helper trait for lowercase without std
trait AsciiLowercase {
    fn to_ascii_lowercase_owned(&self) -> AsciiString;
    fn _as_str(&self) -> &str;
}

impl AsciiLowercase for str {
    fn to_ascii_lowercase_owned(&self) -> AsciiString {
        let mut result = AsciiString::new();
        for b in self.bytes() {
            result.push(b.to_ascii_lowercase());
        }
        result
    }

    fn _as_str(&self) -> &str {
        self
    }
}

/// Simple ASCII string for no_std
#[derive(Debug, Clone)]
pub struct AsciiString {
    data: [u8; 64],
    len: usize,
}

impl AsciiString {
    /// Create empty string
    pub const fn new() -> Self {
        Self {
            data: [0u8; 64],
            len: 0,
        }
    }

    /// Push byte
    pub fn push(&mut self, b: u8) {
        if self.len < 64 {
            self.data[self.len] = b;
            self.len += 1;
        }
    }

    /// Get as str
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }
}

impl Default for AsciiString {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SIZE PARSING AND FORMATTING
// =============================================================================

/// Size unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeUnit {
    /// Bytes
    Bytes,
    /// Kilobytes (1024)
    KB,
    /// Megabytes
    MB,
    /// Gigabytes
    GB,
    /// Terabytes
    TB,
    /// Petabytes
    PB,
}

impl Default for SizeUnit {
    fn default() -> Self {
        SizeUnit::Bytes
    }
}

impl SizeUnit {
    /// Get multiplier
    pub const fn multiplier(&self) -> u64 {
        match self {
            SizeUnit::Bytes => 1,
            SizeUnit::KB => 1024,
            SizeUnit::MB => 1024 * 1024,
            SizeUnit::GB => 1024 * 1024 * 1024,
            SizeUnit::TB => 1024 * 1024 * 1024 * 1024,
            SizeUnit::PB => 1024 * 1024 * 1024 * 1024 * 1024,
        }
    }

    /// Get suffix
    pub const fn suffix(&self) -> &'static str {
        match self {
            SizeUnit::Bytes => "B",
            SizeUnit::KB => "KB",
            SizeUnit::MB => "MB",
            SizeUnit::GB => "GB",
            SizeUnit::TB => "TB",
            SizeUnit::PB => "PB",
        }
    }
}

/// Parse size with unit
pub fn parse_size(s: &str) -> ParseResult<u64> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    // Find where digits end
    let mut num_end = 0;
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_digit() || c == '_' {
            num_end = i + 1;
        } else {
            break;
        }
    }

    if num_end == 0 {
        return Err(ParseError::InvalidFormat);
    }

    let num_str = &s[..num_end];
    let unit_str = s[num_end..].trim();

    let value = parse_u64(num_str)?;

    let unit = match unit_str.to_ascii_uppercase().as_str() {
        "" | "B" => SizeUnit::Bytes,
        "K" | "KB" | "KIB" => SizeUnit::KB,
        "M" | "MB" | "MIB" => SizeUnit::MB,
        "G" | "GB" | "GIB" => SizeUnit::GB,
        "T" | "TB" | "TIB" => SizeUnit::TB,
        "P" | "PB" | "PIB" => SizeUnit::PB,
        _ => return Err(ParseError::InvalidFormat),
    };

    value
        .checked_mul(unit.multiplier())
        .ok_or(ParseError::Overflow)
}

/// Helper for uppercase
trait _AsciiUppercase {
    fn _to_ascii_uppercase(&self) -> AsciiString;
}

impl _AsciiUppercase for str {
    fn _to_ascii_uppercase(&self) -> AsciiString {
        let mut result = AsciiString::new();
        for b in self.bytes() {
            result.push(b.to_ascii_uppercase());
        }
        result
    }
}

/// Format size with appropriate unit
pub fn format_size(bytes: u64) -> FormattedSize {
    FormattedSize(bytes)
}

/// Formatted size wrapper
#[derive(Debug, Clone, Copy)]
pub struct FormattedSize(pub u64);

impl FormattedSize {
    /// Get best unit
    pub fn best_unit(&self) -> (u64, SizeUnit) {
        if self.0 >= SizeUnit::PB.multiplier() {
            (self.0 / SizeUnit::PB.multiplier(), SizeUnit::PB)
        } else if self.0 >= SizeUnit::TB.multiplier() {
            (self.0 / SizeUnit::TB.multiplier(), SizeUnit::TB)
        } else if self.0 >= SizeUnit::GB.multiplier() {
            (self.0 / SizeUnit::GB.multiplier(), SizeUnit::GB)
        } else if self.0 >= SizeUnit::MB.multiplier() {
            (self.0 / SizeUnit::MB.multiplier(), SizeUnit::MB)
        } else if self.0 >= SizeUnit::KB.multiplier() {
            (self.0 / SizeUnit::KB.multiplier(), SizeUnit::KB)
        } else {
            (self.0, SizeUnit::Bytes)
        }
    }

    /// Format to buffer
    pub fn format(&self, buf: &mut [u8]) -> usize {
        let (value, unit) = self.best_unit();
        let mut idx = format_u64(value, buf);

        let suffix = unit.suffix();
        for b in suffix.bytes() {
            if idx < buf.len() {
                buf[idx] = b;
                idx += 1;
            }
        }

        idx
    }
}

// =============================================================================
// TIME PARSING AND FORMATTING
// =============================================================================

/// Time unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    /// Nanoseconds
    Nanoseconds,
    /// Microseconds
    Microseconds,
    /// Milliseconds
    Milliseconds,
    /// Seconds
    Seconds,
    /// Minutes
    Minutes,
    /// Hours
    Hours,
    /// Days
    Days,
}

impl Default for TimeUnit {
    fn default() -> Self {
        TimeUnit::Seconds
    }
}

impl TimeUnit {
    /// Get multiplier (in microseconds)
    pub const fn multiplier_us(&self) -> u64 {
        match self {
            TimeUnit::Nanoseconds => 0, // Special case
            TimeUnit::Microseconds => 1,
            TimeUnit::Milliseconds => 1000,
            TimeUnit::Seconds => 1_000_000,
            TimeUnit::Minutes => 60_000_000,
            TimeUnit::Hours => 3_600_000_000,
            TimeUnit::Days => 86_400_000_000,
        }
    }

    /// Get suffix
    pub const fn suffix(&self) -> &'static str {
        match self {
            TimeUnit::Nanoseconds => "ns",
            TimeUnit::Microseconds => "µs",
            TimeUnit::Milliseconds => "ms",
            TimeUnit::Seconds => "s",
            TimeUnit::Minutes => "min",
            TimeUnit::Hours => "h",
            TimeUnit::Days => "d",
        }
    }
}

/// Duration in microseconds
#[derive(Debug, Clone, Copy, Default)]
pub struct Duration {
    /// Microseconds
    pub microseconds: u64,
}

impl Duration {
    /// Create from microseconds
    pub const fn from_us(us: u64) -> Self {
        Self { microseconds: us }
    }

    /// Create from milliseconds
    pub const fn from_ms(ms: u64) -> Self {
        Self {
            microseconds: ms * 1000,
        }
    }

    /// Create from seconds
    pub const fn from_secs(secs: u64) -> Self {
        Self {
            microseconds: secs * 1_000_000,
        }
    }

    /// Get as milliseconds
    pub const fn as_ms(&self) -> u64 {
        self.microseconds / 1000
    }

    /// Get as seconds
    pub const fn as_secs(&self) -> u64 {
        self.microseconds / 1_000_000
    }

    /// Format duration
    pub fn format(&self, buf: &mut [u8]) -> usize {
        let us = self.microseconds;

        if us < 1000 {
            // Microseconds
            let mut idx = format_u64(us, buf);
            if idx + 2 <= buf.len() {
                buf[idx] = b'\xC2'; // µ in UTF-8
                buf[idx + 1] = b'\xB5';
                idx += 2;
            }
            if idx < buf.len() {
                buf[idx] = b's';
                idx += 1;
            }
            idx
        } else if us < 1_000_000 {
            // Milliseconds
            let ms = us / 1000;
            let mut idx = format_u64(ms, buf);
            for b in b"ms" {
                if idx < buf.len() {
                    buf[idx] = *b;
                    idx += 1;
                }
            }
            idx
        } else if us < 60_000_000 {
            // Seconds
            let secs = us / 1_000_000;
            let ms = (us % 1_000_000) / 1000;
            let mut idx = format_u64(secs, buf);
            if ms > 0 {
                if idx < buf.len() {
                    buf[idx] = b'.';
                    idx += 1;
                }
                let ms_3 = (ms * 1000) / 1000; // Normalize
                idx += format_u64(ms_3, &mut buf[idx..]);
            }
            if idx < buf.len() {
                buf[idx] = b's';
                idx += 1;
            }
            idx
        } else {
            // Minutes:Seconds
            let mins = us / 60_000_000;
            let secs = (us % 60_000_000) / 1_000_000;
            let mut idx = format_u64(mins, buf);
            if idx < buf.len() {
                buf[idx] = b':';
                idx += 1;
            }
            if secs < 10 && idx < buf.len() {
                buf[idx] = b'0';
                idx += 1;
            }
            idx += format_u64(secs, &mut buf[idx..]);
            idx
        }
    }
}

// =============================================================================
// PATH UTILITIES
// =============================================================================

/// Path separator
pub const PATH_SEP: char = '/';
pub const PATH_SEP_WIN: char = '\\';

/// Maximum path length
pub const MAX_PATH_LEN: usize = 256;

/// Path structure
#[derive(Debug, Clone, Copy)]
pub struct Path {
    /// Path data
    pub data: [u8; MAX_PATH_LEN],
    /// Path length
    pub len: usize,
}

impl Default for Path {
    fn default() -> Self {
        Self {
            data: [0u8; MAX_PATH_LEN],
            len: 0,
        }
    }
}

impl Path {
    /// Create from str
    pub fn from_str(s: &str) -> Self {
        let mut path = Self::default();
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(MAX_PATH_LEN);
        path.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        path.len = copy_len;
        path
    }

    /// Get as str
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("")
    }

    /// Check if absolute
    pub fn is_absolute(&self) -> bool {
        if self.len == 0 {
            return false;
        }
        self.data[0] == b'/' || self.data[0] == b'\\'
    }

    /// Get filename
    pub fn filename(&self) -> &str {
        let s = self.as_str();
        let last_sep = s.rfind(|c| c == '/' || c == '\\');
        match last_sep {
            Some(idx) => &s[idx + 1..],
            None => s,
        }
    }

    /// Get extension
    pub fn extension(&self) -> Option<&str> {
        let filename = self.filename();
        let dot_pos = filename.rfind('.');
        match dot_pos {
            Some(idx) if idx > 0 => Some(&filename[idx + 1..]),
            _ => None,
        }
    }

    /// Get parent directory
    pub fn parent(&self) -> Path {
        let s = self.as_str();
        let last_sep = s.rfind(|c| c == '/' || c == '\\');
        match last_sep {
            Some(0) => Path::from_str("/"),
            Some(idx) => Path::from_str(&s[..idx]),
            None => Path::default(),
        }
    }

    /// Join with another path
    pub fn join(&self, other: &str) -> Path {
        let mut result = *self;

        // Add separator if needed
        if result.len > 0 && !other.is_empty() {
            let last = result.data[result.len - 1];
            let first_other = other.as_bytes()[0];
            if last != b'/' && last != b'\\' && first_other != b'/' && first_other != b'\\' {
                if result.len < MAX_PATH_LEN {
                    result.data[result.len] = b'/';
                    result.len += 1;
                }
            }
        }

        // Append other
        let other_bytes = other.as_bytes();
        let copy_len = other_bytes.len().min(MAX_PATH_LEN - result.len);
        result.data[result.len..result.len + copy_len].copy_from_slice(&other_bytes[..copy_len]);
        result.len += copy_len;

        result
    }

    /// Normalize path (remove . and ..)
    pub fn normalize(&self) -> Path {
        let s = self.as_str();
        let mut result = Path::default();
        let mut components: [&str; 32] = [""; 32];
        let mut comp_count = 0;

        for part in s.split(|c| c == '/' || c == '\\') {
            match part {
                "" | "." => continue,
                ".." => {
                    if comp_count > 0 {
                        comp_count -= 1;
                    }
                },
                _ => {
                    if comp_count < 32 {
                        components[comp_count] = part;
                        comp_count += 1;
                    }
                },
            }
        }

        // Rebuild path
        let is_abs = self.is_absolute();
        if is_abs && result.len < MAX_PATH_LEN {
            result.data[0] = b'/';
            result.len = 1;
        }

        for i in 0..comp_count {
            if i > 0 && result.len < MAX_PATH_LEN {
                result.data[result.len] = b'/';
                result.len += 1;
            }
            let comp = components[i].as_bytes();
            let copy_len = comp.len().min(MAX_PATH_LEN - result.len);
            result.data[result.len..result.len + copy_len].copy_from_slice(&comp[..copy_len]);
            result.len += copy_len;
        }

        result
    }
}

// =============================================================================
// STRING FORMATTING
// =============================================================================

/// Format u64 to buffer, returns bytes written
pub fn format_u64(value: u64, buf: &mut [u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }

    if value == 0 {
        buf[0] = b'0';
        return 1;
    }

    // Calculate digits
    let mut temp = value;
    let mut digits = [0u8; 20];
    let mut digit_count = 0;

    while temp > 0 {
        digits[digit_count] = b'0' + (temp % 10) as u8;
        temp /= 10;
        digit_count += 1;
    }

    // Write in reverse
    let copy_len = digit_count.min(buf.len());
    for i in 0..copy_len {
        buf[i] = digits[digit_count - 1 - i];
    }

    copy_len
}

/// Format i64 to buffer
pub fn format_i64(value: i64, buf: &mut [u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }

    if value < 0 {
        buf[0] = b'-';
        1 + format_u64((-value) as u64, &mut buf[1..])
    } else {
        format_u64(value as u64, buf)
    }
}

/// Format u64 as hex to buffer
pub fn format_hex(value: u64, buf: &mut [u8], uppercase: bool) -> usize {
    if buf.is_empty() {
        return 0;
    }

    if value == 0 {
        buf[0] = b'0';
        return 1;
    }

    let hex_chars: &[u8] = if uppercase {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };

    let mut temp = value;
    let mut digits = [0u8; 16];
    let mut digit_count = 0;

    while temp > 0 {
        digits[digit_count] = hex_chars[(temp & 0xF) as usize];
        temp >>= 4;
        digit_count += 1;
    }

    let copy_len = digit_count.min(buf.len());
    for i in 0..copy_len {
        buf[i] = digits[digit_count - 1 - i];
    }

    copy_len
}

/// Format u64 as hex with prefix
pub fn format_hex_prefixed(value: u64, buf: &mut [u8]) -> usize {
    if buf.len() < 2 {
        return 0;
    }
    buf[0] = b'0';
    buf[1] = b'x';
    2 + format_hex(value, &mut buf[2..], false)
}

// =============================================================================
// UUID/GUID FORMATTING
// =============================================================================

/// GUID structure
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Guid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

impl Guid {
    /// Create from parts
    pub const fn new(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Self {
        Self {
            data1,
            data2,
            data3,
            data4,
        }
    }

    /// Format to string buffer (36 + 1 bytes needed)
    pub fn format<'a>(&self, buf: &'a mut [u8; 37]) -> &'a str {
        let hex = b"0123456789ABCDEF";
        let mut idx = 0;

        // Data1 (8 hex digits)
        for i in (0..4).rev() {
            let b = ((self.data1 >> (i * 8)) & 0xFF) as usize;
            buf[idx] = hex[b >> 4];
            buf[idx + 1] = hex[b & 0xF];
            idx += 2;
        }
        buf[idx] = b'-';
        idx += 1;

        // Data2 (4 hex digits)
        for i in (0..2).rev() {
            let b = ((self.data2 >> (i * 8)) & 0xFF) as usize;
            buf[idx] = hex[b >> 4];
            buf[idx + 1] = hex[b & 0xF];
            idx += 2;
        }
        buf[idx] = b'-';
        idx += 1;

        // Data3 (4 hex digits)
        for i in (0..2).rev() {
            let b = ((self.data3 >> (i * 8)) & 0xFF) as usize;
            buf[idx] = hex[b >> 4];
            buf[idx + 1] = hex[b & 0xF];
            idx += 2;
        }
        buf[idx] = b'-';
        idx += 1;

        // Data4[0..2]
        for i in 0..2 {
            buf[idx] = hex[(self.data4[i] >> 4) as usize];
            buf[idx + 1] = hex[(self.data4[i] & 0xF) as usize];
            idx += 2;
        }
        buf[idx] = b'-';
        idx += 1;

        // Data4[2..8]
        for i in 2..8 {
            buf[idx] = hex[(self.data4[i] >> 4) as usize];
            buf[idx + 1] = hex[(self.data4[i] & 0xF) as usize];
            idx += 2;
        }
        buf[idx] = 0;

        core::str::from_utf8(&buf[..36]).unwrap_or("")
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = [0u8; 37];
        let s = self.format(&mut buf);
        write!(f, "{}", s)
    }
}

// =============================================================================
// CONFIG LINE PARSER
// =============================================================================

/// Config line type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLineType {
    /// Empty line
    Empty,
    /// Comment line
    Comment,
    /// Section header [section]
    Section,
    /// Key-value pair
    KeyValue,
    /// Invalid line
    Invalid,
}

impl Default for ConfigLineType {
    fn default() -> Self {
        ConfigLineType::Empty
    }
}

/// Parsed config line
#[derive(Debug, Clone, Copy)]
pub struct ConfigLine {
    /// Line type
    pub line_type: ConfigLineType,
    /// Key or section name
    pub key: [u8; 64],
    /// Key length
    pub key_len: usize,
    /// Value
    pub value: [u8; 128],
    /// Value length
    pub value_len: usize,
}

impl Default for ConfigLine {
    fn default() -> Self {
        Self {
            line_type: ConfigLineType::Empty,
            key: [0u8; 64],
            key_len: 0,
            value: [0u8; 128],
            value_len: 0,
        }
    }
}

impl ConfigLine {
    /// Get key as str
    pub fn key_str(&self) -> &str {
        core::str::from_utf8(&self.key[..self.key_len]).unwrap_or("")
    }

    /// Get value as str
    pub fn value_str(&self) -> &str {
        core::str::from_utf8(&self.value[..self.value_len]).unwrap_or("")
    }
}

/// Parse config line
pub fn parse_config_line(line: &str) -> ConfigLine {
    let mut result = ConfigLine::default();
    let line = line.trim();

    if line.is_empty() {
        result.line_type = ConfigLineType::Empty;
        return result;
    }

    // Check for comment
    if line.starts_with('#') || line.starts_with(';') {
        result.line_type = ConfigLineType::Comment;
        return result;
    }

    // Check for section
    if line.starts_with('[') && line.ends_with(']') {
        result.line_type = ConfigLineType::Section;
        let section = &line[1..line.len() - 1];
        let bytes = section.as_bytes();
        let copy_len = bytes.len().min(64);
        result.key[..copy_len].copy_from_slice(&bytes[..copy_len]);
        result.key_len = copy_len;
        return result;
    }

    // Try key=value
    if let Some(eq_pos) = line.find('=') {
        result.line_type = ConfigLineType::KeyValue;

        let key = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();

        // Remove quotes from value if present
        let value = if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };

        let key_bytes = key.as_bytes();
        let key_len = key_bytes.len().min(64);
        result.key[..key_len].copy_from_slice(&key_bytes[..key_len]);
        result.key_len = key_len;

        let value_bytes = value.as_bytes();
        let value_len = value_bytes.len().min(128);
        result.value[..value_len].copy_from_slice(&value_bytes[..value_len]);
        result.value_len = value_len;

        return result;
    }

    result.line_type = ConfigLineType::Invalid;
    result
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_parse_u64() {
        assert_eq!(parse_u64("123").unwrap(), 123);
        assert_eq!(parse_u64("0x1F").unwrap(), 31);
        assert_eq!(parse_u64("0b1010").unwrap(), 10);
        assert_eq!(parse_u64("0o17").unwrap(), 15);
        assert_eq!(parse_u64("1_000_000").unwrap(), 1000000);
    }

    #[test]
    fn test_parse_i64() {
        assert_eq!(parse_i64("-42").unwrap(), -42);
        assert_eq!(parse_i64("+42").unwrap(), 42);
        assert_eq!(parse_i64("0").unwrap(), 0);
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1 MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("2GB").unwrap(), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_format_u64() {
        let mut buf = [0u8; 32];
        let len = format_u64(12345, &mut buf);
        assert_eq!(&buf[..len], b"12345");
    }

    #[test]
    fn test_format_hex() {
        let mut buf = [0u8; 32];
        let len = format_hex(0xFF, &mut buf, true);
        assert_eq!(&buf[..len], b"FF");
    }

    #[test]
    fn test_path() {
        let path = Path::from_str("/boot/kernel.elf");
        assert!(path.is_absolute());
        assert_eq!(path.filename(), "kernel.elf");
        assert_eq!(path.extension(), Some("elf"));
        assert_eq!(path.parent().as_str(), "/boot");
    }

    #[test]
    fn test_path_join() {
        let path = Path::from_str("/boot");
        let joined = path.join("kernel.elf");
        assert_eq!(joined.as_str(), "/boot/kernel.elf");
    }

    #[test]
    fn test_config_line() {
        let line = parse_config_line("key = \"value\"");
        assert_eq!(line.line_type, ConfigLineType::KeyValue);
        assert_eq!(line.key_str(), "key");
        assert_eq!(line.value_str(), "value");

        let section = parse_config_line("[boot]");
        assert_eq!(section.line_type, ConfigLineType::Section);
        assert_eq!(section.key_str(), "boot");
    }

    #[test]
    fn test_guid_format() {
        let guid = Guid::new(0x12345678, 0x9ABC, 0xDEF0, [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
        ]);
        let mut buf = [0u8; 37];
        let s = guid.format(&mut buf);
        assert_eq!(s, "12345678-9ABC-DEF0-1234-56789ABCDEF0");
    }
}
