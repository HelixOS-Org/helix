//! Validation Layer
//!
//! GPU API validation and error checking.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

// ============================================================================
// Validation Severity
// ============================================================================

/// Validation message severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValidationSeverity {
    /// Verbose (detailed info).
    Verbose,
    /// Information.
    Info,
    /// Warning.
    Warning,
    /// Error.
    Error,
}

impl Default for ValidationSeverity {
    fn default() -> Self {
        ValidationSeverity::Warning
    }
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationSeverity::Verbose => write!(f, "VERBOSE"),
            ValidationSeverity::Info => write!(f, "INFO"),
            ValidationSeverity::Warning => write!(f, "WARNING"),
            ValidationSeverity::Error => write!(f, "ERROR"),
        }
    }
}

// ============================================================================
// Validation Error Kind
// ============================================================================

/// Kind of validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationErrorKind {
    /// Invalid parameter.
    InvalidParameter,
    /// Invalid state.
    InvalidState,
    /// Invalid handle.
    InvalidHandle,
    /// Resource not found.
    ResourceNotFound,
    /// Resource in use.
    ResourceInUse,
    /// Out of memory.
    OutOfMemory,
    /// Feature not supported.
    FeatureNotSupported,
    /// Format not supported.
    FormatNotSupported,
    /// Shader error.
    ShaderError,
    /// Pipeline error.
    PipelineError,
    /// Synchronization error.
    SynchronizationError,
    /// Memory error.
    MemoryError,
    /// Command buffer error.
    CommandBufferError,
    /// Descriptor error.
    DescriptorError,
    /// Render pass error.
    RenderPassError,
    /// Queue error.
    QueueError,
    /// Device lost.
    DeviceLost,
    /// Unknown error.
    Unknown,
}

impl Default for ValidationErrorKind {
    fn default() -> Self {
        ValidationErrorKind::Unknown
    }
}

impl fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationErrorKind::InvalidParameter => write!(f, "Invalid parameter"),
            ValidationErrorKind::InvalidState => write!(f, "Invalid state"),
            ValidationErrorKind::InvalidHandle => write!(f, "Invalid handle"),
            ValidationErrorKind::ResourceNotFound => write!(f, "Resource not found"),
            ValidationErrorKind::ResourceInUse => write!(f, "Resource in use"),
            ValidationErrorKind::OutOfMemory => write!(f, "Out of memory"),
            ValidationErrorKind::FeatureNotSupported => write!(f, "Feature not supported"),
            ValidationErrorKind::FormatNotSupported => write!(f, "Format not supported"),
            ValidationErrorKind::ShaderError => write!(f, "Shader error"),
            ValidationErrorKind::PipelineError => write!(f, "Pipeline error"),
            ValidationErrorKind::SynchronizationError => write!(f, "Synchronization error"),
            ValidationErrorKind::MemoryError => write!(f, "Memory error"),
            ValidationErrorKind::CommandBufferError => write!(f, "Command buffer error"),
            ValidationErrorKind::DescriptorError => write!(f, "Descriptor error"),
            ValidationErrorKind::RenderPassError => write!(f, "Render pass error"),
            ValidationErrorKind::QueueError => write!(f, "Queue error"),
            ValidationErrorKind::DeviceLost => write!(f, "Device lost"),
            ValidationErrorKind::Unknown => write!(f, "Unknown error"),
        }
    }
}

// ============================================================================
// Validation Error
// ============================================================================

/// A validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error kind.
    pub kind: ValidationErrorKind,
    /// Severity.
    pub severity: ValidationSeverity,
    /// Message.
    pub message: String,
    /// Source location (file:line).
    pub location: Option<String>,
    /// Frame index when error occurred.
    pub frame: u64,
    /// Timestamp (relative).
    pub timestamp: u64,
    /// Message ID (for filtering).
    pub message_id: u32,
}

impl ValidationError {
    /// Create a new validation error.
    pub fn new(
        kind: ValidationErrorKind,
        severity: ValidationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            severity,
            message: message.into(),
            location: None,
            frame: 0,
            timestamp: 0,
            message_id: 0,
        }
    }

    /// Create an error.
    pub fn error(kind: ValidationErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, ValidationSeverity::Error, message)
    }

    /// Create a warning.
    pub fn warning(kind: ValidationErrorKind, message: impl Into<String>) -> Self {
        Self::new(kind, ValidationSeverity::Warning, message)
    }

    /// Create an info message.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(
            ValidationErrorKind::Unknown,
            ValidationSeverity::Info,
            message,
        )
    }

    /// Set location.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set frame.
    pub fn with_frame(mut self, frame: u64) -> Self {
        self.frame = frame;
        self
    }

    /// Set message ID.
    pub fn with_message_id(mut self, id: u32) -> Self {
        self.message_id = id;
        self
    }

    /// Check if this is an error.
    pub fn is_error(&self) -> bool {
        self.severity == ValidationSeverity::Error
    }

    /// Check if this is a warning.
    pub fn is_warning(&self) -> bool {
        self.severity == ValidationSeverity::Warning
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.kind, self.message)?;
        if let Some(ref loc) = self.location {
            write!(f, " at {}", loc)?;
        }
        Ok(())
    }
}

// ============================================================================
// Validation Callback
// ============================================================================

/// Callback for validation messages.
pub type ValidationCallback = fn(&ValidationError);

// ============================================================================
// Validation Filter
// ============================================================================

/// Filter for validation messages.
#[derive(Debug, Clone)]
pub struct ValidationFilter {
    /// Minimum severity.
    pub min_severity: ValidationSeverity,
    /// Enabled error kinds.
    pub enabled_kinds: Vec<ValidationErrorKind>,
    /// Disabled message IDs.
    pub disabled_ids: Vec<u32>,
    /// Maximum messages per kind.
    pub max_messages_per_kind: u32,
}

impl Default for ValidationFilter {
    fn default() -> Self {
        Self {
            min_severity: ValidationSeverity::Warning,
            enabled_kinds: Vec::new(), // Empty = all enabled
            disabled_ids: Vec::new(),
            max_messages_per_kind: 100,
        }
    }
}

impl ValidationFilter {
    /// Create a new filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum severity.
    pub fn with_min_severity(mut self, severity: ValidationSeverity) -> Self {
        self.min_severity = severity;
        self
    }

    /// Enable only specific kinds.
    pub fn with_kinds(mut self, kinds: Vec<ValidationErrorKind>) -> Self {
        self.enabled_kinds = kinds;
        self
    }

    /// Disable specific message IDs.
    pub fn with_disabled_ids(mut self, ids: Vec<u32>) -> Self {
        self.disabled_ids = ids;
        self
    }

    /// Check if an error passes the filter.
    pub fn passes(&self, error: &ValidationError) -> bool {
        // Check severity
        if error.severity < self.min_severity {
            return false;
        }

        // Check disabled IDs
        if self.disabled_ids.contains(&error.message_id) {
            return false;
        }

        // Check enabled kinds
        if !self.enabled_kinds.is_empty() && !self.enabled_kinds.contains(&error.kind) {
            return false;
        }

        true
    }
}

// ============================================================================
// Validation Statistics
// ============================================================================

/// Validation statistics.
#[derive(Debug, Clone, Default)]
pub struct ValidationStatistics {
    /// Total errors.
    pub total_errors: u32,
    /// Total warnings.
    pub total_warnings: u32,
    /// Total info messages.
    pub total_info: u32,
    /// Errors per kind.
    pub errors_per_kind: [u32; 18], // One per ValidationErrorKind
    /// Suppressed messages.
    pub suppressed: u32,
}

impl ValidationStatistics {
    /// Reset statistics.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Record an error.
    pub fn record(&mut self, error: &ValidationError) {
        match error.severity {
            ValidationSeverity::Error => self.total_errors += 1,
            ValidationSeverity::Warning => self.total_warnings += 1,
            ValidationSeverity::Info | ValidationSeverity::Verbose => self.total_info += 1,
        }

        let kind_idx = error.kind as usize;
        if kind_idx < self.errors_per_kind.len() {
            self.errors_per_kind[kind_idx] += 1;
        }
    }

    /// Record a suppressed message.
    pub fn record_suppressed(&mut self) {
        self.suppressed += 1;
    }

    /// Total messages.
    pub fn total(&self) -> u32 {
        self.total_errors + self.total_warnings + self.total_info
    }

    /// Check if has errors.
    pub fn has_errors(&self) -> bool {
        self.total_errors > 0
    }
}

// ============================================================================
// Validation Layer
// ============================================================================

/// Validation layer for GPU API validation.
pub struct ValidationLayer {
    /// Is enabled.
    pub enabled: bool,
    /// Error history.
    errors: VecDeque<ValidationError>,
    /// Maximum errors to keep.
    pub max_errors: usize,
    /// Current frame.
    current_frame: u64,
    /// Callback.
    callback: Option<ValidationCallback>,
    /// Filter.
    pub filter: ValidationFilter,
    /// Statistics.
    pub stats: ValidationStatistics,
    /// Break on error.
    pub break_on_error: bool,
}

impl ValidationLayer {
    /// Create a new validation layer.
    pub fn new() -> Self {
        Self {
            enabled: true,
            errors: VecDeque::new(),
            max_errors: 1000,
            current_frame: 0,
            callback: None,
            filter: ValidationFilter::default(),
            stats: ValidationStatistics::default(),
            break_on_error: false,
        }
    }

    /// Enable validation.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable validation.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Set callback.
    pub fn set_callback(&mut self, callback: ValidationCallback) {
        self.callback = Some(callback);
    }

    /// Set frame.
    pub fn set_frame(&mut self, frame: u64) {
        self.current_frame = frame;
    }

    /// Report an error.
    pub fn report(&mut self, mut error: ValidationError) {
        if !self.enabled {
            return;
        }

        error.frame = self.current_frame;

        // Check filter
        if !self.filter.passes(&error) {
            self.stats.record_suppressed();
            return;
        }

        // Record statistics
        self.stats.record(&error);

        // Call callback
        if let Some(callback) = self.callback {
            callback(&error);
        }

        // Store error
        if self.errors.len() >= self.max_errors {
            self.errors.pop_front();
        }
        self.errors.push_back(error);
    }

    /// Report an error with kind and message.
    pub fn error(&mut self, kind: ValidationErrorKind, message: impl Into<String>) {
        self.report(ValidationError::error(kind, message));
    }

    /// Report a warning with kind and message.
    pub fn warning(&mut self, kind: ValidationErrorKind, message: impl Into<String>) {
        self.report(ValidationError::warning(kind, message));
    }

    /// Report an info message.
    pub fn info(&mut self, message: impl Into<String>) {
        self.report(ValidationError::info(message));
    }

    /// Get recent errors.
    pub fn recent_errors(&self, count: usize) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter().rev().take(count)
    }

    /// Get all errors.
    pub fn all_errors(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter()
    }

    /// Get errors for current frame.
    pub fn frame_errors(&self) -> impl Iterator<Item = &ValidationError> {
        let frame = self.current_frame;
        self.errors.iter().filter(move |e| e.frame == frame)
    }

    /// Get error count.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Clear errors.
    pub fn clear(&mut self) {
        self.errors.clear();
    }

    /// Get statistics.
    pub fn statistics(&self) -> &ValidationStatistics {
        &self.stats
    }

    /// Reset statistics.
    pub fn reset_statistics(&mut self) {
        self.stats.reset();
    }

    /// Check if has errors.
    pub fn has_errors(&self) -> bool {
        self.stats.has_errors()
    }
}

impl Default for ValidationLayer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Validation Macros
// ============================================================================

/// Check a condition and report if false.
#[macro_export]
macro_rules! validate {
    ($layer:expr, $cond:expr, $kind:expr, $msg:expr) => {
        if !$cond {
            $layer.error($kind, $msg);
        }
    };
}

/// Check a condition and return error if false.
#[macro_export]
macro_rules! validate_or {
    ($layer:expr, $cond:expr, $kind:expr, $msg:expr, $ret:expr) => {
        if !$cond {
            $layer.error($kind, $msg);
            return $ret;
        }
    };
}

// ============================================================================
// Common Validations
// ============================================================================

/// Common validation functions.
pub mod validations {
    use super::*;

    /// Validate buffer size.
    pub fn validate_buffer_size(layer: &mut ValidationLayer, size: u64, max_size: u64) -> bool {
        if size == 0 {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                "Buffer size cannot be zero",
            );
            return false;
        }
        if size > max_size {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                alloc::format!("Buffer size {} exceeds maximum {}", size, max_size),
            );
            return false;
        }
        true
    }

    /// Validate alignment.
    pub fn validate_alignment(layer: &mut ValidationLayer, value: u64, alignment: u64) -> bool {
        if alignment == 0 || !alignment.is_power_of_two() {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                alloc::format!("Invalid alignment: {}", alignment),
            );
            return false;
        }
        if value % alignment != 0 {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                alloc::format!("Value {} is not aligned to {}", value, alignment),
            );
            return false;
        }
        true
    }

    /// Validate offset and size within bounds.
    pub fn validate_bounds(
        layer: &mut ValidationLayer,
        offset: u64,
        size: u64,
        total: u64,
    ) -> bool {
        if offset >= total {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                alloc::format!("Offset {} exceeds total size {}", offset, total),
            );
            return false;
        }
        if offset + size > total {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                alloc::format!(
                    "Range {}..{} exceeds total size {}",
                    offset,
                    offset + size,
                    total
                ),
            );
            return false;
        }
        true
    }

    /// Validate index within bounds.
    pub fn validate_index(layer: &mut ValidationLayer, index: u32, count: u32, name: &str) -> bool {
        if index >= count {
            layer.error(
                ValidationErrorKind::InvalidParameter,
                alloc::format!("{} index {} out of bounds (count: {})", name, index, count),
            );
            return false;
        }
        true
    }
}
