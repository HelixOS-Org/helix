//! Debug Logger
//!
//! Logging for GPU debug messages.

use alloc::{collections::VecDeque, string::String, vec::Vec};
use core::fmt;

// ============================================================================
// Log Level
// ============================================================================

/// Log message level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Trace level (most verbose).
    Trace,
    /// Debug level.
    Debug,
    /// Info level.
    Info,
    /// Warning level.
    Warning,
    /// Error level.
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

// ============================================================================
// Log Category
// ============================================================================

/// Log message category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogCategory {
    /// General.
    General,
    /// Validation.
    Validation,
    /// Performance.
    Performance,
    /// Memory.
    Memory,
    /// Resource.
    Resource,
    /// Pipeline.
    Pipeline,
    /// Shader.
    Shader,
    /// Command.
    Command,
    /// Synchronization.
    Synchronization,
    /// Presentation.
    Presentation,
}

impl Default for LogCategory {
    fn default() -> Self {
        LogCategory::General
    }
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogCategory::General => write!(f, "General"),
            LogCategory::Validation => write!(f, "Validation"),
            LogCategory::Performance => write!(f, "Performance"),
            LogCategory::Memory => write!(f, "Memory"),
            LogCategory::Resource => write!(f, "Resource"),
            LogCategory::Pipeline => write!(f, "Pipeline"),
            LogCategory::Shader => write!(f, "Shader"),
            LogCategory::Command => write!(f, "Command"),
            LogCategory::Synchronization => write!(f, "Synchronization"),
            LogCategory::Presentation => write!(f, "Presentation"),
        }
    }
}

// ============================================================================
// Log Entry
// ============================================================================

/// A log entry.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Log level.
    pub level: LogLevel,
    /// Category.
    pub category: LogCategory,
    /// Message.
    pub message: String,
    /// Frame index.
    pub frame: u64,
    /// Timestamp (relative).
    pub timestamp: u64,
    /// Source location.
    pub location: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(level: LogLevel, category: LogCategory, message: impl Into<String>) -> Self {
        Self {
            level,
            category,
            message: message.into(),
            frame: 0,
            timestamp: 0,
            location: None,
        }
    }

    /// Create a trace entry.
    pub fn trace(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Trace, LogCategory::General, message)
    }

    /// Create a debug entry.
    pub fn debug(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Debug, LogCategory::General, message)
    }

    /// Create an info entry.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, LogCategory::General, message)
    }

    /// Create a warning entry.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warning, LogCategory::General, message)
    }

    /// Create an error entry.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, LogCategory::General, message)
    }

    /// Set category.
    pub fn with_category(mut self, category: LogCategory) -> Self {
        self.category = category;
        self
    }

    /// Set frame.
    pub fn with_frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    /// Set location.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}][{}] {}", self.level, self.category, self.message)?;
        if let Some(ref loc) = self.location {
            write!(f, " ({})", loc)?;
        }
        Ok(())
    }
}

// ============================================================================
// Log Filter
// ============================================================================

/// Filter for log messages.
#[derive(Debug, Clone)]
pub struct LogFilter {
    /// Minimum log level.
    pub min_level: LogLevel,
    /// Enabled categories (empty = all).
    pub enabled_categories: Vec<LogCategory>,
    /// Disabled categories.
    pub disabled_categories: Vec<LogCategory>,
    /// Message filter (substring).
    pub message_filter: Option<String>,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            enabled_categories: Vec::new(),
            disabled_categories: Vec::new(),
            message_filter: None,
        }
    }
}

impl LogFilter {
    /// Create a new filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum level.
    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Enable only specific categories.
    pub fn with_categories(mut self, categories: Vec<LogCategory>) -> Self {
        self.enabled_categories = categories;
        self
    }

    /// Disable specific categories.
    pub fn without_categories(mut self, categories: Vec<LogCategory>) -> Self {
        self.disabled_categories = categories;
        self
    }

    /// Set message filter.
    pub fn with_message_filter(mut self, filter: impl Into<String>) -> Self {
        self.message_filter = Some(filter.into());
        self
    }

    /// Check if entry passes filter.
    pub fn passes(&self, entry: &LogEntry) -> bool {
        // Check level
        if entry.level < self.min_level {
            return false;
        }

        // Check enabled categories
        if !self.enabled_categories.is_empty()
            && !self.enabled_categories.contains(&entry.category)
        {
            return false;
        }

        // Check disabled categories
        if self.disabled_categories.contains(&entry.category) {
            return false;
        }

        // Check message filter
        if let Some(ref filter) = self.message_filter {
            if !entry.message.contains(filter) {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// Log Buffer
// ============================================================================

/// Circular buffer for log entries.
pub struct LogBuffer {
    /// Entries.
    entries: VecDeque<LogEntry>,
    /// Maximum size.
    max_size: usize,
}

impl LogBuffer {
    /// Create a new log buffer.
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push an entry.
    pub fn push(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_size {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get all entries.
    pub fn entries(&self) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter()
    }

    /// Get recent entries.
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &LogEntry> {
        self.entries.iter().rev().take(count)
    }

    /// Filter entries.
    pub fn filter<'a>(&'a self, filter: &'a LogFilter) -> impl Iterator<Item = &'a LogEntry> {
        self.entries.iter().filter(move |e| filter.passes(e))
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ============================================================================
// Debug Logger
// ============================================================================

/// Callback for log messages.
pub type LogCallback = fn(&LogEntry);

/// Debug logger.
pub struct DebugLogger {
    /// Is enabled.
    pub enabled: bool,
    /// Log buffer.
    buffer: LogBuffer,
    /// Filter.
    pub filter: LogFilter,
    /// Callback.
    callback: Option<LogCallback>,
    /// Current frame.
    current_frame: u64,
    /// Current timestamp.
    current_timestamp: u64,
    /// Error count.
    error_count: u32,
    /// Warning count.
    warning_count: u32,
}

impl DebugLogger {
    /// Create a new logger.
    pub fn new() -> Self {
        Self {
            enabled: true,
            buffer: LogBuffer::default(),
            filter: LogFilter::default(),
            callback: None,
            current_frame: 0,
            current_timestamp: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Enable logging.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable logging.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set callback.
    pub fn set_callback(&mut self, callback: LogCallback) {
        self.callback = Some(callback);
    }

    /// Set frame.
    pub fn set_frame(&mut self, frame: u64) {
        self.current_frame = frame;
    }

    /// Set timestamp.
    pub fn set_timestamp(&mut self, timestamp: u64) {
        self.current_timestamp = timestamp;
    }

    /// Log an entry.
    pub fn log(&mut self, mut entry: LogEntry) {
        if !self.enabled {
            return;
        }

        entry.frame = self.current_frame;
        entry.timestamp = self.current_timestamp;

        // Count errors and warnings
        match entry.level {
            LogLevel::Error => self.error_count += 1,
            LogLevel::Warning => self.warning_count += 1,
            _ => {}
        }

        // Check filter
        if !self.filter.passes(&entry) {
            return;
        }

        // Call callback
        if let Some(callback) = self.callback {
            callback(&entry);
        }

        // Store in buffer
        self.buffer.push(entry);
    }

    /// Log trace.
    pub fn trace(&mut self, message: impl Into<String>) {
        self.log(LogEntry::trace(message));
    }

    /// Log debug.
    pub fn debug(&mut self, message: impl Into<String>) {
        self.log(LogEntry::debug(message));
    }

    /// Log info.
    pub fn info(&mut self, message: impl Into<String>) {
        self.log(LogEntry::info(message));
    }

    /// Log warning.
    pub fn warning(&mut self, message: impl Into<String>) {
        self.log(LogEntry::warning(message));
    }

    /// Log error.
    pub fn error(&mut self, message: impl Into<String>) {
        self.log(LogEntry::error(message));
    }

    /// Log with category.
    pub fn log_category(&mut self, level: LogLevel, category: LogCategory, message: impl Into<String>) {
        self.log(LogEntry::new(level, category, message));
    }

    /// Get log buffer.
    pub fn buffer(&self) -> &LogBuffer {
        &self.buffer
    }

    /// Get recent entries.
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &LogEntry> {
        self.buffer.recent(count)
    }

    /// Get error count.
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Get warning count.
    pub fn warning_count(&self) -> u32 {
        self.warning_count
    }

    /// Reset counts.
    pub fn reset_counts(&mut self) {
        self.error_count = 0;
        self.warning_count = 0;
    }

    /// Clear log buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.reset_counts();
    }

    /// Has errors.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Has warnings.
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }
}

impl Default for DebugLogger {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Logging Macros
// ============================================================================

/// Log trace message.
#[macro_export]
macro_rules! log_trace {
    ($logger:expr, $($arg:tt)*) => {
        $logger.trace(alloc::format!($($arg)*))
    };
}

/// Log debug message.
#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        $logger.debug(alloc::format!($($arg)*))
    };
}

/// Log info message.
#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        $logger.info(alloc::format!($($arg)*))
    };
}

/// Log warning message.
#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $($arg:tt)*) => {
        $logger.warning(alloc::format!($($arg)*))
    };
}

/// Log error message.
#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        $logger.error(alloc::format!($($arg)*))
    };
}
