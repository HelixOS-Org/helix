//! Boot Events and Notification System
//!
//! This module provides a comprehensive event-driven architecture for
//! boot process management, including lifecycle events and notifications.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Event System Architecture                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Event Types                                    │   │
//! │  │  Boot │ Progress │ Error │ Device │ Security │ User │ Timer     │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Event Queue                                    │   │
//! │  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐                        │   │
//! │  │  │ E1  │→│ E2  │→│ E3  │→│ E4  │→│ E5  │→ ...                   │   │
//! │  │  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘                        │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Handlers                                       │   │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐                │   │
//! │  │  │ Logger  │ │ Monitor │ │ Display │ │ Action  │                │   │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘                │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// EVENT TYPES
// =============================================================================

/// Event category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventCategory {
    /// Boot lifecycle events
    Boot,
    /// Progress updates
    Progress,
    /// Error and warning events
    Error,
    /// Device events
    Device,
    /// Security events
    Security,
    /// User interaction events
    User,
    /// Timer events
    Timer,
    /// System events
    #[default]
    System,
    /// Custom events
    Custom,
}

impl fmt::Display for EventCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventCategory::Boot => write!(f, "Boot"),
            EventCategory::Progress => write!(f, "Progress"),
            EventCategory::Error => write!(f, "Error"),
            EventCategory::Device => write!(f, "Device"),
            EventCategory::Security => write!(f, "Security"),
            EventCategory::User => write!(f, "User"),
            EventCategory::Timer => write!(f, "Timer"),
            EventCategory::System => write!(f, "System"),
            EventCategory::Custom => write!(f, "Custom"),
        }
    }
}

/// Event priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum EventPriority {
    #[default]
    /// Lowest priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
    /// Immediate (bypass queue)
    Immediate,
}

// =============================================================================
// BOOT LIFECYCLE EVENTS
// =============================================================================

/// Boot phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum BootPhase {
    #[default]
    /// Firmware handoff received
    FirmwareEntry,
    /// Early initialization
    EarlyInit,
    /// Memory map obtained
    MemoryMapReady,
    /// Boot services available
    BootServicesReady,
    /// Console initialized
    ConsoleReady,
    /// Devices enumerated
    DevicesEnumerated,
    /// Filesystems mounted
    FilesystemsMounted,
    /// Configuration loaded
    ConfigLoaded,
    /// Boot menu displayed
    BootMenuShown,
    /// Boot entry selected
    EntrySelected,
    /// Kernel loading
    KernelLoading,
    /// Kernel loaded
    KernelLoaded,
    /// Initrd loading
    InitrdLoading,
    /// Initrd loaded
    InitrdLoaded,
    /// Exit boot services
    ExitBootServices,
    /// Kernel entry
    KernelEntry,
    /// Boot complete
    BootComplete,
}

impl fmt::Display for BootPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootPhase::FirmwareEntry => write!(f, "Firmware Entry"),
            BootPhase::EarlyInit => write!(f, "Early Init"),
            BootPhase::MemoryMapReady => write!(f, "Memory Map Ready"),
            BootPhase::BootServicesReady => write!(f, "Boot Services Ready"),
            BootPhase::ConsoleReady => write!(f, "Console Ready"),
            BootPhase::DevicesEnumerated => write!(f, "Devices Enumerated"),
            BootPhase::FilesystemsMounted => write!(f, "Filesystems Mounted"),
            BootPhase::ConfigLoaded => write!(f, "Config Loaded"),
            BootPhase::BootMenuShown => write!(f, "Boot Menu Shown"),
            BootPhase::EntrySelected => write!(f, "Entry Selected"),
            BootPhase::KernelLoading => write!(f, "Kernel Loading"),
            BootPhase::KernelLoaded => write!(f, "Kernel Loaded"),
            BootPhase::InitrdLoading => write!(f, "Initrd Loading"),
            BootPhase::InitrdLoaded => write!(f, "Initrd Loaded"),
            BootPhase::ExitBootServices => write!(f, "Exit Boot Services"),
            BootPhase::KernelEntry => write!(f, "Kernel Entry"),
            BootPhase::BootComplete => write!(f, "Boot Complete"),
        }
    }
}

/// Boot phase transition event
#[derive(Debug, Clone, Copy)]
pub struct BootPhaseEvent {
    /// Previous phase
    pub from: BootPhase,
    /// New phase
    pub to: BootPhase,
    /// Timestamp (microseconds since boot)
    pub timestamp_us: u64,
    /// Phase duration (microseconds)
    pub duration_us: u64,
}

// =============================================================================
// PROGRESS EVENTS
// =============================================================================

/// Progress event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressType {
    #[default]
    /// Discrete step progress
    Steps,
    /// Percentage progress
    Percentage,
    /// Bytes transferred
    Bytes,
    /// Indeterminate (spinning)
    Indeterminate,
}

/// Progress event
#[derive(Debug, Clone, Copy)]
pub struct ProgressEvent {
    /// Progress type
    pub progress_type: ProgressType,
    /// Current value
    pub current: u64,
    /// Total value (0 for indeterminate)
    pub total: u64,
    /// Operation name
    pub operation: [u8; 64],
    /// Operation name length
    pub operation_len: usize,
    /// Estimated time remaining (seconds, 0 = unknown)
    pub eta_secs: u32,
}

impl Default for ProgressEvent {
    fn default() -> Self {
        Self {
            progress_type: ProgressType::Percentage,
            current: 0,
            total: 100,
            operation: [0u8; 64],
            operation_len: 0,
            eta_secs: 0,
        }
    }
}

impl ProgressEvent {
    /// Calculate percentage (0-100)
    pub fn percentage(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        ((self.current * 100) / self.total).min(100) as u8
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }

    /// Get operation name as string
    pub fn operation_str(&self) -> &str {
        if self.operation_len > 0 {
            core::str::from_utf8(&self.operation[..self.operation_len]).unwrap_or("")
        } else {
            ""
        }
    }
}

// =============================================================================
// ERROR EVENTS
// =============================================================================

/// Error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ErrorSeverity {
    #[default]
    /// Informational
    Info,
    /// Warning (non-fatal)
    Warning,
    /// Error (potentially fatal)
    Error,
    /// Critical (fatal)
    Critical,
    /// Panic (unrecoverable)
    Panic,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "INFO"),
            ErrorSeverity::Warning => write!(f, "WARN"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
            ErrorSeverity::Panic => write!(f, "PANIC"),
        }
    }
}

/// Error event
#[derive(Debug, Clone, Copy)]
pub struct ErrorEvent {
    /// Severity
    pub severity: ErrorSeverity,
    /// Error code
    pub code: u32,
    /// Source module
    pub source: [u8; 32],
    /// Source length
    pub source_len: usize,
    /// Error message
    pub message: [u8; 128],
    /// Message length
    pub message_len: usize,
    /// File name
    pub file: [u8; 64],
    /// File length
    pub file_len: usize,
    /// Line number
    pub line: u32,
}

impl Default for ErrorEvent {
    fn default() -> Self {
        Self {
            severity: ErrorSeverity::Error,
            code: 0,
            source: [0u8; 32],
            source_len: 0,
            message: [0u8; 128],
            message_len: 0,
            file: [0u8; 64],
            file_len: 0,
            line: 0,
        }
    }
}

// =============================================================================
// DEVICE EVENTS
// =============================================================================

/// Device event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceEventType {
    /// Device discovered
    #[default]
    Discovered,
    /// Device attached (hot-plug)
    Attached,
    /// Device detached (hot-unplug)
    Detached,
    /// Device initialized
    Initialized,
    /// Device failed
    Failed,
    /// Device ready
    Ready,
    /// Device error
    Error,
}

/// Device class for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceEventClass {
    #[default]
    /// Storage device
    Storage,
    /// Network device
    Network,
    /// Display device
    Display,
    /// Input device
    Input,
    /// USB device
    Usb,
    /// PCI device
    Pci,
    /// Audio device
    Audio,
    /// Serial device
    Serial,
    /// Unknown device
    Unknown,
}

/// Device event
#[derive(Debug, Clone, Copy)]
pub struct DeviceEvent {
    /// Event type
    pub event_type: DeviceEventType,
    /// Device class
    pub class: DeviceEventClass,
    /// Device ID
    pub device_id: u32,
    /// Device name
    pub name: [u8; 64],
    /// Name length
    pub name_len: usize,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
}

impl Default for DeviceEvent {
    fn default() -> Self {
        Self {
            event_type: DeviceEventType::Discovered,
            class: DeviceEventClass::Unknown,
            device_id: 0,
            name: [0u8; 64],
            name_len: 0,
            vendor_id: 0,
            product_id: 0,
        }
    }
}

// =============================================================================
// SECURITY EVENTS
// =============================================================================

/// Security event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecurityEventType {
    /// Secure Boot status change
    #[default]
    SecureBootStatus,
    /// Image verification
    ImageVerification,
    /// Signature check
    SignatureCheck,
    /// TPM measurement
    TpmMeasurement,
    /// Key enrollment
    KeyEnrollment,
    /// Password attempt
    PasswordAttempt,
    /// Lockout triggered
    LockoutTriggered,
    /// Security violation
    Violation,
}

/// Security event
#[derive(Debug, Clone, Copy)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: SecurityEventType,
    /// Success
    pub success: bool,
    /// PCR index (for TPM events)
    pub pcr_index: u8,
    /// Image path (if applicable)
    pub image_path: [u8; 128],
    /// Path length
    pub path_len: usize,
    /// Hash of measured/verified data
    pub hash: [u8; 32],
    /// Additional info
    pub info: u32,
}

impl Default for SecurityEvent {
    fn default() -> Self {
        Self {
            event_type: SecurityEventType::SecureBootStatus,
            success: false,
            pcr_index: 0,
            image_path: [0u8; 128],
            path_len: 0,
            hash: [0u8; 32],
            info: 0,
        }
    }
}

// =============================================================================
// USER EVENTS
// =============================================================================

/// User event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UserEventType {
    /// Key press
    #[default]
    KeyPress,
    /// Key release
    KeyRelease,
    /// Mouse move
    MouseMove,
    /// Mouse button
    MouseButton,
    /// Touch event
    Touch,
    /// Menu selection
    MenuSelect,
    /// Timeout triggered
    Timeout,
    /// Hotkey activated
    Hotkey,
}

/// User input event
#[derive(Debug, Clone, Copy)]
pub struct UserEvent {
    /// Event type
    pub event_type: UserEventType,
    /// Scancode
    pub scancode: u16,
    /// Unicode character
    pub unicode: u16,
    /// Modifier keys
    pub modifiers: u8,
    /// Mouse X position
    pub mouse_x: u16,
    /// Mouse Y position
    pub mouse_y: u16,
    /// Mouse buttons
    pub mouse_buttons: u8,
}

impl Default for UserEvent {
    fn default() -> Self {
        Self {
            event_type: UserEventType::KeyPress,
            scancode: 0,
            unicode: 0,
            modifiers: 0,
            mouse_x: 0,
            mouse_y: 0,
            mouse_buttons: 0,
        }
    }
}

/// Key modifiers
pub mod modifiers {
    /// Shift key
    pub const SHIFT: u8 = 1 << 0;
    /// Control key
    pub const CTRL: u8 = 1 << 1;
    /// Alt key
    pub const ALT: u8 = 1 << 2;
    /// Super/Windows key
    pub const SUPER: u8 = 1 << 3;
    /// Caps Lock active
    pub const CAPS_LOCK: u8 = 1 << 4;
    /// Num Lock active
    pub const NUM_LOCK: u8 = 1 << 5;
}

// =============================================================================
// TIMER EVENTS
// =============================================================================

/// Timer event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerEventType {
    /// One-shot timer fired
    #[default]
    OneShot,
    /// Periodic timer tick
    Periodic,
    /// Timeout expired
    Timeout,
    /// Watchdog warning
    WatchdogWarning,
    /// Watchdog expired
    WatchdogExpired,
}

/// Timer event
#[derive(Debug, Clone, Copy)]
pub struct TimerEvent {
    /// Event type
    pub event_type: TimerEventType,
    /// Timer ID
    pub timer_id: u16,
    /// Interval (milliseconds)
    pub interval_ms: u32,
    /// Elapsed time (milliseconds)
    pub elapsed_ms: u32,
    /// Remaining time (milliseconds)
    pub remaining_ms: u32,
}

impl Default for TimerEvent {
    fn default() -> Self {
        Self {
            event_type: TimerEventType::OneShot,
            timer_id: 0,
            interval_ms: 0,
            elapsed_ms: 0,
            remaining_ms: 0,
        }
    }
}

// =============================================================================
// UNIFIED EVENT
// =============================================================================

/// Event ID
pub type EventId = u32;

/// Unified event data
#[derive(Debug, Clone, Copy, Default)]
pub enum EventData {
    /// Boot phase event
    Boot(BootPhaseEvent),
    /// Progress event
    Progress(ProgressEvent),
    /// Error event
    Error(ErrorEvent),
    /// Device event
    Device(DeviceEvent),
    /// Security event
    Security(SecurityEvent),
    /// User event
    User(UserEvent),
    /// Timer event
    Timer(TimerEvent),
    /// No data
    #[default]
    None,
}

/// Unified event structure
#[derive(Debug, Clone, Copy)]
pub struct Event {
    /// Event ID (unique)
    pub id: EventId,
    /// Category
    pub category: EventCategory,
    /// Priority
    pub priority: EventPriority,
    /// Timestamp (microseconds since boot)
    pub timestamp_us: u64,
    /// Event data
    pub data: EventData,
    /// Event handled
    pub handled: bool,
    /// Propagate to next handler
    pub propagate: bool,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            id: 0,
            category: EventCategory::System,
            priority: EventPriority::Normal,
            timestamp_us: 0,
            data: EventData::None,
            handled: false,
            propagate: true,
        }
    }
}

impl Event {
    /// Create boot phase event
    pub fn boot_phase(id: EventId, from: BootPhase, to: BootPhase, timestamp_us: u64) -> Self {
        Self {
            id,
            category: EventCategory::Boot,
            priority: EventPriority::High,
            timestamp_us,
            data: EventData::Boot(BootPhaseEvent {
                from,
                to,
                timestamp_us,
                duration_us: 0,
            }),
            handled: false,
            propagate: true,
        }
    }

    /// Create error event
    pub fn error(id: EventId, severity: ErrorSeverity, code: u32, timestamp_us: u64) -> Self {
        let priority = match severity {
            ErrorSeverity::Info => EventPriority::Low,
            ErrorSeverity::Warning => EventPriority::Normal,
            ErrorSeverity::Error => EventPriority::High,
            ErrorSeverity::Critical | ErrorSeverity::Panic => EventPriority::Critical,
        };
        Self {
            id,
            category: EventCategory::Error,
            priority,
            timestamp_us,
            data: EventData::Error(ErrorEvent {
                severity,
                code,
                ..Default::default()
            }),
            handled: false,
            propagate: true,
        }
    }
}

// =============================================================================
// EVENT QUEUE
// =============================================================================

/// Maximum queue size
pub const MAX_QUEUE_SIZE: usize = 64;

/// Event queue
#[derive(Debug)]
pub struct EventQueue {
    /// Events
    events: [Event; MAX_QUEUE_SIZE],
    /// Head (read position)
    head: usize,
    /// Tail (write position)
    tail: usize,
    /// Event count
    count: usize,
    /// Next event ID
    next_id: EventId,
    /// Queue overflow count
    overflow_count: u32,
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EventQueue {
    /// Create new event queue
    #[allow(clippy::large_stack_arrays)]
    pub const fn new() -> Self {
        Self {
            events: [Event {
                id: 0,
                category: EventCategory::System,
                priority: EventPriority::Normal,
                timestamp_us: 0,
                data: EventData::None,
                handled: false,
                propagate: true,
            }; MAX_QUEUE_SIZE],
            head: 0,
            tail: 0,
            count: 0,
            next_id: 1,
            overflow_count: 0,
        }
    }

    /// Check if queue is empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if queue is full
    pub const fn is_full(&self) -> bool {
        self.count >= MAX_QUEUE_SIZE
    }

    /// Get number of events in queue
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Push event to queue
    pub fn push(&mut self, mut event: Event) -> Option<EventId> {
        if self.is_full() {
            self.overflow_count += 1;
            return None;
        }

        event.id = self.next_id;
        self.next_id += 1;

        self.events[self.tail] = event;
        self.tail = (self.tail + 1) % MAX_QUEUE_SIZE;
        self.count += 1;

        Some(event.id)
    }

    /// Pop event from queue
    pub fn pop(&mut self) -> Option<Event> {
        if self.is_empty() {
            return None;
        }

        let event = self.events[self.head];
        self.head = (self.head + 1) % MAX_QUEUE_SIZE;
        self.count -= 1;

        Some(event)
    }

    /// Peek at front event without removing
    pub fn peek(&self) -> Option<&Event> {
        if self.is_empty() {
            None
        } else {
            Some(&self.events[self.head])
        }
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }

    /// Get overflow count
    pub const fn overflow_count(&self) -> u32 {
        self.overflow_count
    }
}

// =============================================================================
// EVENT HANDLER
// =============================================================================

/// Handler result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandlerResult {
    #[default]
    /// Event not handled
    NotHandled,
    /// Event handled, continue propagation
    Handled,
    /// Event consumed, stop propagation
    Consumed,
    /// Error occurred
    Error,
}

/// Handler type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandlerType {
    #[default]
    /// Logger handler
    Logger,
    /// Monitor/profiler
    Monitor,
    /// Display handler
    Display,
    /// Action handler
    Action,
    /// Error handler
    ErrorHandler,
    /// Custom handler
    Custom,
}

/// Handler registration
#[derive(Debug, Clone, Copy)]
pub struct HandlerRegistration {
    /// Handler ID
    pub id: u16,
    /// Handler type
    pub handler_type: HandlerType,
    /// Event categories to handle
    pub categories: u16,
    /// Minimum priority
    pub min_priority: EventPriority,
    /// Active
    pub active: bool,
}

impl Default for HandlerRegistration {
    fn default() -> Self {
        Self {
            id: 0,
            handler_type: HandlerType::Custom,
            categories: 0xFFFF, // All categories
            min_priority: EventPriority::Low,
            active: true,
        }
    }
}

impl HandlerRegistration {
    /// Check if handler accepts category
    pub fn accepts_category(&self, category: EventCategory) -> bool {
        let bit = 1u16 << (category as u16);
        (self.categories & bit) != 0
    }
}

// =============================================================================
// NOTIFICATION SYSTEM
// =============================================================================

/// Notification level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationLevel {
    #[default]
    /// Silent (no notification)
    Silent,
    /// Visual only
    Visual,
    /// Audio only
    Audio,
    /// Both visual and audio
    Both,
}

/// Notification type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationType {
    #[default]
    /// Information
    Info,
    /// Success
    Success,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Progress update
    Progress,
    /// Question/prompt
    Question,
}

/// Notification structure
#[derive(Debug, Clone, Copy)]
pub struct Notification {
    /// Notification type
    pub notification_type: NotificationType,
    /// Level
    pub level: NotificationLevel,
    /// Title
    pub title: [u8; 64],
    /// Title length
    pub title_len: usize,
    /// Message
    pub message: [u8; 256],
    /// Message length
    pub message_len: usize,
    /// Duration (milliseconds, 0 = until dismissed)
    pub duration_ms: u32,
    /// Has action button
    pub has_action: bool,
    /// Action label
    pub action_label: [u8; 32],
    /// Action label length
    pub action_len: usize,
}

impl Default for Notification {
    fn default() -> Self {
        Self {
            notification_type: NotificationType::Info,
            level: NotificationLevel::Visual,
            title: [0u8; 64],
            title_len: 0,
            message: [0u8; 256],
            message_len: 0,
            duration_ms: 3000,
            has_action: false,
            action_label: [0u8; 32],
            action_len: 0,
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
    fn test_event_queue() {
        let mut queue = EventQueue::new();
        assert!(queue.is_empty());

        let event = Event::default();
        let id = queue.push(event);
        assert!(id.is_some());
        assert_eq!(queue.len(), 1);

        let popped = queue.pop();
        assert!(popped.is_some());
        assert!(queue.is_empty());
    }

    #[test]
    fn test_boot_phase_order() {
        assert!(BootPhase::FirmwareEntry < BootPhase::MemoryMapReady);
        assert!(BootPhase::KernelLoading < BootPhase::BootComplete);
    }

    #[test]
    fn test_progress_event() {
        let progress = ProgressEvent {
            current: 50,
            total: 100,
            ..Default::default()
        };
        assert_eq!(progress.percentage(), 50);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_handler_registration() {
        let handler = HandlerRegistration::default();
        assert!(handler.accepts_category(EventCategory::Boot));
        assert!(handler.accepts_category(EventCategory::Error));
    }

    #[test]
    fn test_error_severity_order() {
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Critical < ErrorSeverity::Panic);
    }
}
