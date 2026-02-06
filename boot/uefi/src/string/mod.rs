//! String and Formatting Utilities
//!
//! UCS-2 string handling, formatting, and conversion utilities for UEFI.

use core::fmt::{self, Write};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum string length
pub const MAX_STRING_LEN: usize = 4096;

/// Null terminator
pub const NUL: u16 = 0;

/// Newline (carriage return)
pub const CR: u16 = 0x000D;

/// Newline (line feed)
pub const LF: u16 = 0x000A;

/// Tab
pub const TAB: u16 = 0x0009;

/// Space
pub const SPACE: u16 = 0x0020;

// =============================================================================
// CHAR16 TYPE
// =============================================================================

/// UEFI character type (UCS-2)
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Char16(u16);

impl Char16 {
    /// Null character
    pub const NUL: Self = Self(0);

    /// Create from u16
    pub const fn from_u16(c: u16) -> Self {
        Self(c)
    }

    /// Convert to u16
    pub const fn to_u16(self) -> u16 {
        self.0
    }

    /// Try create from char
    pub fn try_from_char(c: char) -> Option<Self> {
        if (c as u32) <= 0xFFFF {
            Some(Self(c as u16))
        } else {
            None
        }
    }

    /// Convert to char
    pub fn to_char(self) -> Option<char> {
        char::from_u32(self.0 as u32)
    }

    /// Is ASCII
    pub const fn is_ascii(self) -> bool {
        self.0 < 128
    }

    /// Is alphanumeric
    pub fn is_alphanumeric(self) -> bool {
        self.is_alphabetic() || self.is_numeric()
    }

    /// Is alphabetic
    pub fn is_alphabetic(self) -> bool {
        (self.0 >= 0x41 && self.0 <= 0x5A) || // A-Z
        (self.0 >= 0x61 && self.0 <= 0x7A) // a-z
    }

    /// Is numeric
    pub fn is_numeric(self) -> bool {
        self.0 >= 0x30 && self.0 <= 0x39 // 0-9
    }

    /// Is whitespace
    pub fn is_whitespace(self) -> bool {
        matches!(self.0, 0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x0020)
    }

    /// Is control character
    pub fn is_control(self) -> bool {
        self.0 < 0x20 || (self.0 >= 0x7F && self.0 < 0xA0)
    }

    /// To uppercase
    pub fn to_uppercase(self) -> Self {
        if self.0 >= 0x61 && self.0 <= 0x7A {
            Self(self.0 - 32)
        } else {
            self
        }
    }

    /// To lowercase
    pub fn to_lowercase(self) -> Self {
        if self.0 >= 0x41 && self.0 <= 0x5A {
            Self(self.0 + 32)
        } else {
            self
        }
    }
}

impl From<u16> for Char16 {
    fn from(c: u16) -> Self {
        Self(c)
    }
}

impl From<u8> for Char16 {
    fn from(c: u8) -> Self {
        Self(c as u16)
    }
}

impl From<Char16> for u16 {
    fn from(c: Char16) -> u16 {
        c.0
    }
}

impl fmt::Display for Char16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(c) = self.to_char() {
            write!(f, "{}", c)
        } else {
            write!(f, "\\u{{{:04X}}}", self.0)
        }
    }
}

// =============================================================================
// STRING SLICE
// =============================================================================

/// UCS-2 string slice (null-terminated)
#[repr(transparent)]
pub struct CStr16([u16]);

impl CStr16 {
    /// Create from pointer (unsafe)
    ///
    /// # Safety
    /// Pointer must be valid and null-terminated
    pub unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Self {
        let len = strlen16(ptr);
        let slice = core::slice::from_raw_parts(ptr, len + 1);
        &*(slice as *const [u16] as *const Self)
    }

    /// Create from slice with null terminator
    pub fn from_slice_with_nul(slice: &[u16]) -> Option<&Self> {
        if slice.last() == Some(&0) {
            Some(unsafe { &*(slice as *const [u16] as *const Self) })
        } else {
            None
        }
    }

    /// Get length (without null terminator)
    pub fn len(&self) -> usize {
        self.0.len().saturating_sub(1)
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// As slice (without null terminator)
    pub fn as_slice(&self) -> &[u16] {
        &self.0[..self.len()]
    }

    /// As slice with null terminator
    pub fn as_slice_with_nul(&self) -> &[u16] {
        &self.0
    }

    /// As pointer
    pub fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    /// Iterate characters
    pub fn chars(&self) -> impl Iterator<Item = Char16> + '_ {
        self.as_slice().iter().map(|&c| Char16(c))
    }

    /// Convert to UTF-8 string
    pub fn to_string(&self) -> String16<256> {
        let mut s = String16::new();
        for c in self.as_slice() {
            s.push_u16(*c);
        }
        s
    }

    /// Equals string
    pub fn eq_str(&self, s: &str) -> bool {
        if self.len() != s.len() {
            return false;
        }

        for (c1, c2) in self.chars().zip(s.chars()) {
            if c1.to_u16() != c2 as u16 {
                return false;
            }
        }

        true
    }

    /// Case-insensitive equals
    pub fn eq_str_ignore_case(&self, s: &str) -> bool {
        if self.len() != s.len() {
            return false;
        }

        for (c1, c2) in self.chars().zip(s.chars()) {
            if c1.to_lowercase().to_u16() != (c2.to_ascii_lowercase() as u16) {
                return false;
            }
        }

        true
    }

    /// Starts with
    pub fn starts_with(&self, prefix: &str) -> bool {
        if prefix.len() > self.len() {
            return false;
        }

        for (c1, c2) in self.chars().zip(prefix.chars()) {
            if c1.to_u16() != c2 as u16 {
                return false;
            }
        }

        true
    }

    /// Ends with
    pub fn ends_with(&self, suffix: &str) -> bool {
        if suffix.len() > self.len() {
            return false;
        }

        let offset = self.len() - suffix.len();
        for (i, c) in suffix.chars().enumerate() {
            if self.0[offset + i] != c as u16 {
                return false;
            }
        }

        true
    }
}

impl fmt::Display for CStr16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.chars() {
            write!(f, "{}", c)?;
        }
        Ok(())
    }
}

impl fmt::Debug for CStr16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        for c in self.chars() {
            write!(f, "{}", c)?;
        }
        write!(f, "\"")
    }
}

// =============================================================================
// OWNED STRING
// =============================================================================

/// Owned UCS-2 string with fixed capacity
#[derive(Clone)]
pub struct String16<const N: usize> {
    /// Buffer (always null-terminated)
    buffer: [u16; N],
    /// Length (without null terminator)
    len: usize,
}

impl<const N: usize> String16<N> {
    /// Create empty string
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            len: 0,
        }
    }

    /// Create from ASCII str
    pub fn from_str(s: &str) -> Self {
        let mut result = Self::new();
        for c in s.chars() {
            if !result.push_char(c) {
                break;
            }
        }
        result
    }

    /// Create from u16 slice
    pub fn from_slice(slice: &[u16]) -> Self {
        let mut result = Self::new();
        for &c in slice {
            if c == 0 || !result.push_u16(c) {
                break;
            }
        }
        result
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Capacity
    pub const fn capacity(&self) -> usize {
        N - 1 // Reserve for null terminator
    }

    /// Remaining capacity
    pub fn remaining(&self) -> usize {
        self.capacity() - self.len
    }

    /// Clear
    pub fn clear(&mut self) {
        self.len = 0;
        self.buffer[0] = 0;
    }

    /// Push u16
    pub fn push_u16(&mut self, c: u16) -> bool {
        if self.len >= self.capacity() {
            return false;
        }

        self.buffer[self.len] = c;
        self.len += 1;
        self.buffer[self.len] = 0;
        true
    }

    /// Push char
    pub fn push_char(&mut self, c: char) -> bool {
        if (c as u32) <= 0xFFFF {
            self.push_u16(c as u16)
        } else {
            false
        }
    }

    /// Push str
    pub fn push_str(&mut self, s: &str) -> bool {
        for c in s.chars() {
            if !self.push_char(c) {
                return false;
            }
        }
        true
    }

    /// Pop character
    pub fn pop(&mut self) -> Option<u16> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        let c = self.buffer[self.len];
        self.buffer[self.len] = 0;
        Some(c)
    }

    /// As slice (without null)
    pub fn as_slice(&self) -> &[u16] {
        &self.buffer[..self.len]
    }

    /// As slice with null terminator
    pub fn as_slice_with_nul(&self) -> &[u16] {
        &self.buffer[..=self.len]
    }

    /// As CStr16
    pub fn as_cstr16(&self) -> &CStr16 {
        CStr16::from_slice_with_nul(self.as_slice_with_nul()).unwrap()
    }

    /// As pointer
    pub fn as_ptr(&self) -> *const u16 {
        self.buffer.as_ptr()
    }

    /// As mutable pointer
    pub fn as_mut_ptr(&mut self) -> *mut u16 {
        self.buffer.as_mut_ptr()
    }

    /// Iterate characters
    pub fn chars(&self) -> impl Iterator<Item = Char16> + '_ {
        self.as_slice().iter().map(|&c| Char16(c))
    }

    /// Truncate
    pub fn truncate(&mut self, new_len: usize) {
        if new_len < self.len {
            self.len = new_len;
            self.buffer[self.len] = 0;
        }
    }

    /// Equals str
    pub fn eq_str(&self, s: &str) -> bool {
        self.as_cstr16().eq_str(s)
    }

    /// To uppercase
    pub fn to_uppercase(&self) -> Self {
        let mut result = Self::new();
        for c in self.chars() {
            result.push_u16(c.to_uppercase().to_u16());
        }
        result
    }

    /// To lowercase
    pub fn to_lowercase(&self) -> Self {
        let mut result = Self::new();
        for c in self.chars() {
            result.push_u16(c.to_lowercase().to_u16());
        }
        result
    }

    /// Trim whitespace
    pub fn trim(&self) -> Self {
        let slice = self.as_slice();

        // Find start
        let start = slice
            .iter()
            .position(|&c| !Char16(c).is_whitespace())
            .unwrap_or(slice.len());

        // Find end
        let end = slice
            .iter()
            .rposition(|&c| !Char16(c).is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);

        if start >= end {
            return Self::new();
        }

        Self::from_slice(&slice[start..end])
    }

    /// Contains character
    pub fn contains(&self, c: u16) -> bool {
        self.as_slice().contains(&c)
    }

    /// Find character
    pub fn find(&self, c: u16) -> Option<usize> {
        self.as_slice().iter().position(|&x| x == c)
    }

    /// Find last character
    pub fn rfind(&self, c: u16) -> Option<usize> {
        self.as_slice().iter().rposition(|&x| x == c)
    }

    /// Split at index
    pub fn split_at(&self, mid: usize) -> (Self, Self) {
        let (left, right) = self.as_slice().split_at(mid.min(self.len));
        (Self::from_slice(left), Self::from_slice(right))
    }
}

impl<const N: usize> Default for String16<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> fmt::Display for String16<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for c in self.chars() {
            write!(f, "{}", c)?;
        }
        Ok(())
    }
}

impl<const N: usize> fmt::Debug for String16<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"")?;
        fmt::Display::fmt(self, f)?;
        write!(f, "\"")
    }
}

impl<const N: usize> Write for String16<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.push_str(s) {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

impl<const N: usize> PartialEq for String16<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<const N: usize> Eq for String16<N> {}

impl<const N: usize> PartialEq<str> for String16<N> {
    fn eq(&self, other: &str) -> bool {
        self.eq_str(other)
    }
}

impl<const N: usize> PartialEq<&str> for String16<N> {
    fn eq(&self, other: &&str) -> bool {
        self.eq_str(*other)
    }
}

// =============================================================================
// FORMATTER
// =============================================================================

/// Number formatting writer
pub struct NumWriter<const N: usize> {
    buffer: [u8; N],
    pos: usize,
}

impl<const N: usize> NumWriter<N> {
    /// Create new
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            pos: N,
        }
    }

    /// Write digit
    fn write_digit(&mut self, digit: u8) {
        if self.pos > 0 {
            self.pos -= 1;
            self.buffer[self.pos] = digit;
        }
    }

    /// Get result
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[self.pos..]).unwrap_or("")
    }

    /// Format unsigned decimal
    pub fn format_u64(mut value: u64) -> Self {
        let mut writer = Self::new();

        if value == 0 {
            writer.write_digit(b'0');
            return writer;
        }

        while value > 0 {
            writer.write_digit(b'0' + (value % 10) as u8);
            value /= 10;
        }

        writer
    }

    /// Format signed decimal
    pub fn format_i64(value: i64) -> Self {
        let mut writer = Self::new();

        let (is_neg, abs_value) = if value < 0 {
            (true, (-(value as i128)) as u64)
        } else {
            (false, value as u64)
        };

        if abs_value == 0 {
            writer.write_digit(b'0');
        } else {
            let mut v = abs_value;
            while v > 0 {
                writer.write_digit(b'0' + (v % 10) as u8);
                v /= 10;
            }
        }

        if is_neg {
            writer.write_digit(b'-');
        }

        writer
    }

    /// Format hexadecimal
    pub fn format_hex(mut value: u64, uppercase: bool) -> Self {
        let mut writer = Self::new();

        if value == 0 {
            writer.write_digit(b'0');
            return writer;
        }

        let base = if uppercase { b'A' } else { b'a' };

        while value > 0 {
            let digit = (value & 0xF) as u8;
            if digit < 10 {
                writer.write_digit(b'0' + digit);
            } else {
                writer.write_digit(base + digit - 10);
            }
            value >>= 4;
        }

        writer
    }

    /// Format hexadecimal with fixed width
    pub fn format_hex_padded(mut value: u64, width: usize, uppercase: bool) -> Self {
        let mut writer = Self::new();

        let base = if uppercase { b'A' } else { b'a' };

        for _ in 0..width {
            let digit = (value & 0xF) as u8;
            if digit < 10 {
                writer.write_digit(b'0' + digit);
            } else {
                writer.write_digit(base + digit - 10);
            }
            value >>= 4;
        }

        writer
    }

    /// Format binary
    pub fn format_binary(mut value: u64) -> Self {
        let mut writer = Self::new();

        if value == 0 {
            writer.write_digit(b'0');
            return writer;
        }

        while value > 0 {
            writer.write_digit(b'0' + (value & 1) as u8);
            value >>= 1;
        }

        writer
    }

    /// Format with thousands separator
    pub fn format_with_separator(mut value: u64, sep: u8) -> Self {
        let mut writer = Self::new();
        let mut count = 0;

        if value == 0 {
            writer.write_digit(b'0');
            return writer;
        }

        while value > 0 {
            if count > 0 && count % 3 == 0 {
                writer.write_digit(sep);
            }
            writer.write_digit(b'0' + (value % 10) as u8);
            value /= 10;
            count += 1;
        }

        writer
    }
}

// =============================================================================
// SIZE FORMATTER
// =============================================================================

/// Format size in bytes
pub fn format_size(bytes: u64) -> SizeDisplay {
    SizeDisplay(bytes)
}

/// Size display wrapper
pub struct SizeDisplay(u64);

impl fmt::Display for SizeDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;
        const GB: u64 = 1024 * 1024 * 1024;
        const TB: u64 = 1024 * 1024 * 1024 * 1024;

        if self.0 >= TB {
            let whole = self.0 / TB;
            let frac = ((self.0 % TB) * 10 / TB) as u8;
            write!(f, "{}.{} TB", whole, frac)
        } else if self.0 >= GB {
            let whole = self.0 / GB;
            let frac = ((self.0 % GB) * 10 / GB) as u8;
            write!(f, "{}.{} GB", whole, frac)
        } else if self.0 >= MB {
            let whole = self.0 / MB;
            let frac = ((self.0 % MB) * 10 / MB) as u8;
            write!(f, "{}.{} MB", whole, frac)
        } else if self.0 >= KB {
            let whole = self.0 / KB;
            let frac = ((self.0 % KB) * 10 / KB) as u8;
            write!(f, "{}.{} KB", whole, frac)
        } else {
            write!(f, "{} B", self.0)
        }
    }
}

// =============================================================================
// STRING BUILDER
// =============================================================================

/// Fixed-size string builder
pub struct StringBuilder<const N: usize> {
    buffer: [u8; N],
    len: usize,
}

impl<const N: usize> StringBuilder<N> {
    /// Create new
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            len: 0,
        }
    }

    /// Current length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Remaining capacity
    pub fn remaining(&self) -> usize {
        N - self.len
    }

    /// Clear
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Push byte
    pub fn push_byte(&mut self, b: u8) -> bool {
        if self.len >= N {
            return false;
        }
        self.buffer[self.len] = b;
        self.len += 1;
        true
    }

    /// Push string
    pub fn push_str(&mut self, s: &str) -> bool {
        let bytes = s.as_bytes();
        if self.len + bytes.len() > N {
            return false;
        }
        self.buffer[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        true
    }

    /// Push char
    pub fn push_char(&mut self, c: char) -> bool {
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        self.push_str(encoded)
    }

    /// As string
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("")
    }

    /// As bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }
}

impl<const N: usize> Default for StringBuilder<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Write for StringBuilder<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.push_str(s) {
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

impl<const N: usize> fmt::Display for StringBuilder<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<const N: usize> fmt::Debug for StringBuilder<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get string length (null-terminated u16)
///
/// # Safety
///
/// The caller must ensure the string pointer is valid and null-terminated.
pub unsafe fn strlen16(s: *const u16) -> usize {
    if s.is_null() {
        return 0;
    }

    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
        if len > MAX_STRING_LEN {
            break;
        }
    }
    len
}

/// Get string length (null-terminated u8)
///
/// # Safety
///
/// The caller must ensure the string pointer is valid and null-terminated.
pub unsafe fn strlen8(s: *const u8) -> usize {
    if s.is_null() {
        return 0;
    }

    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
        if len > MAX_STRING_LEN {
            break;
        }
    }
    len
}

/// Compare two u16 strings
///
/// # Safety
///
/// The caller must ensure the string pointer is valid and null-terminated.
pub unsafe fn strcmp16(s1: *const u16, s2: *const u16) -> i32 {
    let mut i = 0;
    loop {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);

        if c1 != c2 {
            return (c1 as i32) - (c2 as i32);
        }

        if c1 == 0 {
            return 0;
        }

        i += 1;
    }
}

/// Copy u16 string
///
/// # Safety
///
/// The caller must ensure the string pointer is valid and null-terminated.
pub unsafe fn strcpy16(dest: *mut u16, src: *const u16) {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dest.add(i) = c;

        if c == 0 {
            break;
        }

        i += 1;
    }
}

/// Convert UTF-8 to UCS-2
pub fn utf8_to_ucs2(utf8: &str, buffer: &mut [u16]) -> usize {
    let mut i = 0;

    for c in utf8.chars() {
        if i >= buffer.len() - 1 {
            break;
        }

        if (c as u32) <= 0xFFFF {
            buffer[i] = c as u16;
            i += 1;
        }
    }

    buffer[i] = 0;
    i
}

/// Convert UCS-2 to UTF-8
pub fn ucs2_to_utf8(ucs2: &[u16], buffer: &mut [u8]) -> usize {
    let mut pos = 0;

    for &c in ucs2 {
        if c == 0 {
            break;
        }

        if let Some(ch) = char::from_u32(c as u32) {
            let mut enc = [0u8; 4];
            let s = ch.encode_utf8(&mut enc);

            if pos + s.len() >= buffer.len() {
                break;
            }

            buffer[pos..pos + s.len()].copy_from_slice(s.as_bytes());
            pos += s.len();
        }
    }

    if pos < buffer.len() {
        buffer[pos] = 0;
    }

    pos
}

/// Check if string is valid ASCII
pub fn is_ascii_str(s: &str) -> bool {
    s.bytes().all(|b| b < 128)
}

/// Parse unsigned integer from string
pub fn parse_u64(s: &str) -> Option<u64> {
    let s = s.trim();

    if s.is_empty() {
        return None;
    }

    // Check for hex prefix
    if s.starts_with("0x") || s.starts_with("0X") {
        return parse_hex_u64(&s[2..]);
    }

    // Check for binary prefix
    if s.starts_with("0b") || s.starts_with("0B") {
        return parse_binary_u64(&s[2..]);
    }

    let mut result = 0u64;

    for c in s.chars() {
        if !c.is_ascii_digit() {
            return None;
        }

        result = result.checked_mul(10)?;
        result = result.checked_add((c as u8 - b'0') as u64)?;
    }

    Some(result)
}

/// Parse hexadecimal
pub fn parse_hex_u64(s: &str) -> Option<u64> {
    let s = s.trim();

    if s.is_empty() {
        return None;
    }

    let mut result = 0u64;

    for c in s.chars() {
        let digit = match c {
            '0'..='9' => c as u8 - b'0',
            'a'..='f' => c as u8 - b'a' + 10,
            'A'..='F' => c as u8 - b'A' + 10,
            _ => return None,
        };

        result = result.checked_mul(16)?;
        result = result.checked_add(digit as u64)?;
    }

    Some(result)
}

/// Parse binary
pub fn parse_binary_u64(s: &str) -> Option<u64> {
    let s = s.trim();

    if s.is_empty() {
        return None;
    }

    let mut result = 0u64;

    for c in s.chars() {
        let digit = match c {
            '0' => 0,
            '1' => 1,
            _ => return None,
        };

        result = result.checked_mul(2)?;
        result = result.checked_add(digit)?;
    }

    Some(result)
}

// =============================================================================
// PATH UTILITIES
// =============================================================================

/// Extract filename from path
pub fn path_filename(path: &str) -> &str {
    path.rsplit('\\')
        .next()
        .or_else(|| path.rsplit('/').next())
        .unwrap_or(path)
}

/// Extract directory from path
pub fn path_directory(path: &str) -> &str {
    if let Some(pos) = path.rfind(|c| c == '\\' || c == '/') {
        &path[..pos]
    } else {
        ""
    }
}

/// Extract extension from path
pub fn path_extension(path: &str) -> Option<&str> {
    let filename = path_filename(path);
    if let Some(pos) = filename.rfind('.') {
        if pos > 0 {
            return Some(&filename[pos + 1..]);
        }
    }
    None
}

/// Join path components
pub fn path_join<const N: usize>(base: &str, component: &str) -> StringBuilder<N> {
    let mut result = StringBuilder::new();
    result.push_str(base);

    if !base.is_empty() && !base.ends_with('\\') && !base.ends_with('/') {
        result.push_char('\\');
    }

    result.push_str(component);
    result
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_char16() {
        let c = Char16::from_u16(b'A' as u16);
        assert!(c.is_alphabetic());
        assert!(c.is_ascii());
        assert_eq!(c.to_lowercase().to_u16(), b'a' as u16);
    }

    #[test]
    fn test_string16() {
        let mut s: String16<32> = String16::new();
        s.push_str("Hello");
        assert_eq!(s.len(), 5);
        assert!(s.eq_str("Hello"));
    }

    #[test]
    fn test_num_writer() {
        let w = NumWriter::<32>::format_u64(12345);
        assert_eq!(w.as_str(), "12345");

        let w = NumWriter::<32>::format_hex(0xABCD, true);
        assert_eq!(w.as_str(), "ABCD");

        let w = NumWriter::<32>::format_i64(-42);
        assert_eq!(w.as_str(), "-42");
    }

    #[test]
    fn test_parse() {
        assert_eq!(parse_u64("12345"), Some(12345));
        assert_eq!(parse_u64("0x1A"), Some(26));
        assert_eq!(parse_u64("0b1010"), Some(10));
        assert_eq!(parse_u64("invalid"), None);
    }

    #[test]
    fn test_path() {
        assert_eq!(path_filename("\\EFI\\BOOT\\bootx64.efi"), "bootx64.efi");
        assert_eq!(path_directory("\\EFI\\BOOT\\bootx64.efi"), "\\EFI\\BOOT");
        assert_eq!(path_extension("bootx64.efi"), Some("efi"));
    }
}
