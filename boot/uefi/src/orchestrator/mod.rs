//! Boot Orchestration System
//!
//! This module provides the core orchestration logic for the boot process,
//! coordinating all subsystems to load and launch operating systems.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    Boot Orchestration                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Phase Manager                                  │   │
//! │  │  Init → Discovery → Selection → Load → Handoff                  │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                         │
//! │  ┌────────────┬──────────────┼──────────────┬────────────┐             │
//! │  │            │              │              │            │             │
//! │  ▼            ▼              ▼              ▼            ▼             │
//! │ ┌──────┐  ┌────────┐  ┌───────────┐  ┌───────┐  ┌──────────┐          │
//! │ │Config│  │Devices │  │Menu/User  │  │Loader │  │Handoff   │          │
//! │ └──────┘  └────────┘  └───────────┘  └───────┘  └──────────┘          │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   State Machine                                  │   │
//! │  │  Transitions │ Recovery │ Timeouts │ Logging                    │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]

use core::fmt;

// =============================================================================
// BOOT PHASES
// =============================================================================

/// Boot phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum BootPhase {
    /// Not started
    NotStarted         = 0,
    /// Firmware entry
    FirmwareEntry      = 1,
    /// Early initialization
    EarlyInit          = 2,
    /// Console initialization
    ConsoleInit        = 3,
    /// Memory initialization
    MemoryInit         = 4,
    /// Configuration loading
    ConfigLoad         = 5,
    /// Device discovery
    DeviceDiscovery    = 6,
    /// Boot entry detection
    EntryDetection     = 7,
    /// Security validation
    SecurityValidation = 8,
    /// Menu display
    MenuDisplay        = 9,
    /// User selection
    UserSelection      = 10,
    /// Entry preparation
    EntryPreparation   = 11,
    /// Kernel loading
    KernelLoad         = 12,
    /// Initrd loading
    InitrdLoad         = 13,
    /// Pre-boot hooks
    PreBootHooks       = 14,
    /// Exit boot services
    ExitBootServices   = 15,
    /// Handoff preparation
    HandoffPrep        = 16,
    /// Kernel handoff
    KernelHandoff      = 17,
    /// Boot complete
    BootComplete       = 18,
    /// Boot failed
    BootFailed         = 255,
}

impl Default for BootPhase {
    fn default() -> Self {
        BootPhase::NotStarted
    }
}

impl fmt::Display for BootPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootPhase::NotStarted => write!(f, "Not Started"),
            BootPhase::FirmwareEntry => write!(f, "Firmware Entry"),
            BootPhase::EarlyInit => write!(f, "Early Initialization"),
            BootPhase::ConsoleInit => write!(f, "Console Initialization"),
            BootPhase::MemoryInit => write!(f, "Memory Initialization"),
            BootPhase::ConfigLoad => write!(f, "Configuration Loading"),
            BootPhase::DeviceDiscovery => write!(f, "Device Discovery"),
            BootPhase::EntryDetection => write!(f, "Entry Detection"),
            BootPhase::SecurityValidation => write!(f, "Security Validation"),
            BootPhase::MenuDisplay => write!(f, "Menu Display"),
            BootPhase::UserSelection => write!(f, "User Selection"),
            BootPhase::EntryPreparation => write!(f, "Entry Preparation"),
            BootPhase::KernelLoad => write!(f, "Kernel Loading"),
            BootPhase::InitrdLoad => write!(f, "Initrd Loading"),
            BootPhase::PreBootHooks => write!(f, "Pre-boot Hooks"),
            BootPhase::ExitBootServices => write!(f, "Exit Boot Services"),
            BootPhase::HandoffPrep => write!(f, "Handoff Preparation"),
            BootPhase::KernelHandoff => write!(f, "Kernel Handoff"),
            BootPhase::BootComplete => write!(f, "Boot Complete"),
            BootPhase::BootFailed => write!(f, "Boot Failed"),
        }
    }
}

/// Phase timing information
#[derive(Debug, Clone, Copy, Default)]
pub struct PhaseTiming {
    /// Phase start time (microseconds)
    pub start_us: u64,
    /// Phase end time (microseconds)
    pub end_us: u64,
    /// Duration (microseconds)
    pub duration_us: u64,
}

impl PhaseTiming {
    /// Calculate duration
    pub fn calculate_duration(&mut self) {
        if self.end_us >= self.start_us {
            self.duration_us = self.end_us - self.start_us;
        }
    }
}

/// Maximum tracked phases
pub const MAX_PHASES: usize = 20;

/// Phase timing tracker
#[derive(Debug)]
pub struct PhaseTracker {
    /// Timings for each phase
    timings: [PhaseTiming; MAX_PHASES],
    /// Current phase
    current: BootPhase,
    /// Boot start time
    boot_start_us: u64,
}

impl Default for PhaseTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseTracker {
    /// Create new tracker
    pub const fn new() -> Self {
        Self {
            timings: [PhaseTiming {
                start_us: 0,
                end_us: 0,
                duration_us: 0,
            }; MAX_PHASES],
            current: BootPhase::NotStarted,
            boot_start_us: 0,
        }
    }

    /// Start tracking boot
    pub fn start(&mut self, timestamp_us: u64) {
        self.boot_start_us = timestamp_us;
        self.enter_phase(BootPhase::FirmwareEntry, timestamp_us);
    }

    /// Enter a new phase
    pub fn enter_phase(&mut self, phase: BootPhase, timestamp_us: u64) {
        // End current phase
        let current_idx = self.current as usize;
        if current_idx < MAX_PHASES {
            self.timings[current_idx].end_us = timestamp_us;
            self.timings[current_idx].calculate_duration();
        }

        // Start new phase
        self.current = phase;
        let phase_idx = phase as usize;
        if phase_idx < MAX_PHASES {
            self.timings[phase_idx].start_us = timestamp_us;
        }
    }

    /// Get current phase
    pub const fn current(&self) -> BootPhase {
        self.current
    }

    /// Get phase timing
    pub fn timing(&self, phase: BootPhase) -> Option<&PhaseTiming> {
        let idx = phase as usize;
        if idx < MAX_PHASES {
            Some(&self.timings[idx])
        } else {
            None
        }
    }

    /// Get total boot time
    pub fn total_time_us(&self, current_time_us: u64) -> u64 {
        current_time_us.saturating_sub(self.boot_start_us)
    }
}

// =============================================================================
// BOOT STATE
// =============================================================================

/// Boot state flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BootStateFlags(u32);

impl BootStateFlags {
    pub const NONE: BootStateFlags = BootStateFlags(0);
    pub const INITIALIZED: BootStateFlags = BootStateFlags(1);
    pub const CONSOLE_READY: BootStateFlags = BootStateFlags(2);
    pub const MEMORY_READY: BootStateFlags = BootStateFlags(4);
    pub const CONFIG_LOADED: BootStateFlags = BootStateFlags(8);
    pub const DEVICES_SCANNED: BootStateFlags = BootStateFlags(16);
    pub const ENTRIES_DETECTED: BootStateFlags = BootStateFlags(32);
    pub const SECURITY_VALIDATED: BootStateFlags = BootStateFlags(64);
    pub const USER_SELECTED: BootStateFlags = BootStateFlags(128);
    pub const KERNEL_LOADED: BootStateFlags = BootStateFlags(256);
    pub const INITRD_LOADED: BootStateFlags = BootStateFlags(512);
    pub const EXIT_SERVICES: BootStateFlags = BootStateFlags(1024);
    pub const HANDOFF_READY: BootStateFlags = BootStateFlags(2048);
    pub const MENU_SKIPPED: BootStateFlags = BootStateFlags(4096);
    pub const QUICK_BOOT: BootStateFlags = BootStateFlags(8192);
    pub const SAFE_MODE: BootStateFlags = BootStateFlags(16384);
    pub const DEBUG_MODE: BootStateFlags = BootStateFlags(32768);
    pub const RECOVERY_MODE: BootStateFlags = BootStateFlags(65536);
    pub const ERROR_OCCURRED: BootStateFlags = BootStateFlags(131072);

    /// Get raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Check flag
    pub const fn has(&self, flag: BootStateFlags) -> bool {
        self.0 & flag.0 != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: BootStateFlags) {
        self.0 |= flag.0;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: BootStateFlags) {
        self.0 &= !flag.0;
    }

    /// Combine flags
    pub const fn with(self, other: BootStateFlags) -> BootStateFlags {
        BootStateFlags(self.0 | other.0)
    }
}

/// Boot state
#[derive(Debug, Clone, Copy)]
pub struct BootState {
    /// Current phase
    pub phase: BootPhase,
    /// State flags
    pub flags: BootStateFlags,
    /// Selected entry index
    pub selected_entry: u16,
    /// Boot attempt count
    pub attempt_count: u8,
    /// Max attempts
    pub max_attempts: u8,
    /// Timeout remaining (seconds)
    pub timeout_secs: u16,
    /// Last error code
    pub last_error: u32,
    /// Retry delay (ms)
    pub retry_delay_ms: u32,
}

impl Default for BootState {
    fn default() -> Self {
        Self {
            phase: BootPhase::NotStarted,
            flags: BootStateFlags::NONE,
            selected_entry: 0,
            attempt_count: 0,
            max_attempts: 3,
            timeout_secs: 5,
            last_error: 0,
            retry_delay_ms: 1000,
        }
    }
}

impl BootState {
    /// Check if boot can proceed
    pub fn can_boot(&self) -> bool {
        self.flags.has(BootStateFlags::KERNEL_LOADED)
            && !self.flags.has(BootStateFlags::ERROR_OCCURRED)
    }

    /// Check if in safe mode
    pub fn is_safe_mode(&self) -> bool {
        self.flags.has(BootStateFlags::SAFE_MODE)
    }

    /// Check if in debug mode
    pub fn is_debug_mode(&self) -> bool {
        self.flags.has(BootStateFlags::DEBUG_MODE)
    }

    /// Check if in recovery mode
    pub fn is_recovery_mode(&self) -> bool {
        self.flags.has(BootStateFlags::RECOVERY_MODE)
    }

    /// Set error
    pub fn set_error(&mut self, error_code: u32) {
        self.last_error = error_code;
        self.flags.set(BootStateFlags::ERROR_OCCURRED);
    }

    /// Clear error
    pub fn clear_error(&mut self) {
        self.last_error = 0;
        self.flags.clear(BootStateFlags::ERROR_OCCURRED);
    }
}

// =============================================================================
// BOOT CONTEXT
// =============================================================================

/// Boot context containing all boot-related state
#[derive(Debug)]
pub struct BootContext {
    /// Boot state
    pub state: BootState,
    /// Phase tracker
    pub phases: PhaseTracker,
    /// Boot parameters
    pub params: BootParams,
    /// Hardware info
    pub hardware: HardwareInfo,
    /// Selected kernel info
    pub kernel: KernelInfo,
    /// Initrd info
    pub initrd: InitrdInfo,
    /// Memory layout
    pub memory: MemoryLayout,
    /// Boot log
    pub log: BootLog,
}

impl Default for BootContext {
    fn default() -> Self {
        Self::new()
    }
}

impl BootContext {
    /// Create new boot context
    pub fn new() -> Self {
        Self {
            state: BootState::default(),
            phases: PhaseTracker::new(),
            params: BootParams::default(),
            hardware: HardwareInfo::default(),
            kernel: KernelInfo::default(),
            initrd: InitrdInfo::default(),
            memory: MemoryLayout::default(),
            log: BootLog::new(),
        }
    }

    /// Get current phase
    pub const fn current_phase(&self) -> BootPhase {
        self.state.phase
    }

    /// Transition to new phase
    pub fn transition(&mut self, new_phase: BootPhase, timestamp_us: u64) -> Result<(), BootError> {
        // Validate transition
        if !self.is_valid_transition(new_phase) {
            return Err(BootError::InvalidTransition);
        }

        self.phases.enter_phase(new_phase, timestamp_us);
        self.state.phase = new_phase;

        // Log transition
        self.log.add_entry(LogEntry {
            phase: new_phase,
            timestamp_us,
            message: LogMessage::PhaseEnter,
        });

        Ok(())
    }

    /// Check if transition is valid
    fn is_valid_transition(&self, new_phase: BootPhase) -> bool {
        let current = self.state.phase as u8;
        let new = new_phase as u8;

        // Allow forward progress or failure
        new > current || new_phase == BootPhase::BootFailed
    }
}

/// Boot error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootError {
    /// Invalid phase transition
    InvalidTransition,
    /// Operation not allowed in current phase
    NotAllowed,
    /// Timeout
    Timeout,
    /// Resource not found
    NotFound,
    /// Resource busy
    Busy,
    /// Security error
    Security,
    /// Out of memory
    OutOfMemory,
    /// Generic failure
    Failed,
}

// =============================================================================
// BOOT PARAMETERS
// =============================================================================

/// Boot parameters
#[derive(Debug, Clone, Copy, Default)]
pub struct BootParams {
    /// Timeout (seconds, 0 = immediate, -1 = infinite)
    pub timeout: i16,
    /// Default entry index
    pub default_entry: u16,
    /// Quick boot enabled
    pub quick_boot: bool,
    /// Show menu always
    pub show_menu: bool,
    /// Debug mode
    pub debug_mode: bool,
    /// Safe mode
    pub safe_mode: bool,
    /// Verbose output
    pub verbose: bool,
    /// Serial console enabled
    pub serial_console: bool,
    /// Serial baud rate
    pub serial_baud: u32,
    /// Graphics enabled
    pub graphics: bool,
    /// Splash screen enabled
    pub splash: bool,
    /// Quiet boot
    pub quiet: bool,
    /// No PCI scan
    pub no_pci: bool,
    /// No ACPI
    pub no_acpi: bool,
}

// =============================================================================
// HARDWARE INFO
// =============================================================================

/// Hardware information
#[derive(Debug, Clone, Copy, Default)]
pub struct HardwareInfo {
    /// CPU vendor ID
    pub cpu_vendor: [u8; 16],
    /// CPU vendor length
    pub cpu_vendor_len: usize,
    /// CPU family
    pub cpu_family: u8,
    /// CPU model
    pub cpu_model: u8,
    /// CPU stepping
    pub cpu_stepping: u8,
    /// Logical CPU count
    pub cpu_count: u16,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Available memory (bytes)
    pub available_memory: u64,
    /// Firmware type
    pub firmware_type: FirmwareType,
    /// Secure boot enabled
    pub secure_boot: bool,
    /// TPM available
    pub tpm_available: bool,
    /// TPM version
    pub tpm_version: u8,
}

/// Firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FirmwareType {
    /// Unknown
    #[default]
    Unknown,
    /// UEFI
    Uefi,
    /// Legacy BIOS
    LegacyBios,
}

// =============================================================================
// KERNEL INFO
// =============================================================================

/// Maximum kernel path length
pub const MAX_KERNEL_PATH: usize = 256;

/// Maximum kernel args length
pub const MAX_KERNEL_ARGS: usize = 512;

/// Kernel information
#[derive(Debug, Clone, Copy)]
pub struct KernelInfo {
    /// Kernel path
    pub path: [u8; MAX_KERNEL_PATH],
    /// Path length
    pub path_len: usize,
    /// Kernel arguments
    pub args: [u8; MAX_KERNEL_ARGS],
    /// Args length
    pub args_len: usize,
    /// Load address
    pub load_address: u64,
    /// Entry point
    pub entry_point: u64,
    /// Kernel size
    pub size: u64,
    /// Kernel type
    pub kernel_type: KernelType,
    /// Is valid
    pub valid: bool,
    /// Is loaded
    pub loaded: bool,
}

impl Default for KernelInfo {
    fn default() -> Self {
        Self {
            path: [0u8; MAX_KERNEL_PATH],
            path_len: 0,
            args: [0u8; MAX_KERNEL_ARGS],
            args_len: 0,
            load_address: 0,
            entry_point: 0,
            size: 0,
            kernel_type: KernelType::Unknown,
            valid: false,
            loaded: false,
        }
    }
}

/// Kernel type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KernelType {
    /// Unknown
    #[default]
    Unknown,
    /// Linux kernel
    Linux,
    /// EFI application
    EfiApp,
    /// Windows bootloader
    Windows,
    /// macOS kernel
    MacOS,
    /// BSD kernel
    Bsd,
    /// Multiboot kernel
    Multiboot,
    /// Multiboot2 kernel
    Multiboot2,
    /// ELF executable
    Elf,
    /// PE executable
    Pe,
}

// =============================================================================
// INITRD INFO
// =============================================================================

/// Maximum initrd path length
pub const MAX_INITRD_PATH: usize = 256;

/// Maximum initrd count
pub const MAX_INITRDS: usize = 8;

/// Single initrd info
#[derive(Debug, Clone, Copy)]
pub struct InitrdEntry {
    /// Path
    pub path: [u8; MAX_INITRD_PATH],
    /// Path length
    pub path_len: usize,
    /// Load address
    pub load_address: u64,
    /// Size
    pub size: u64,
    /// Is loaded
    pub loaded: bool,
}

impl Default for InitrdEntry {
    fn default() -> Self {
        Self {
            path: [0u8; MAX_INITRD_PATH],
            path_len: 0,
            load_address: 0,
            size: 0,
            loaded: false,
        }
    }
}

/// Initrd information
#[derive(Debug, Clone, Copy)]
pub struct InitrdInfo {
    /// Initrd entries
    pub entries: [InitrdEntry; MAX_INITRDS],
    /// Entry count
    pub count: usize,
    /// Combined start address
    pub combined_address: u64,
    /// Combined size
    pub combined_size: u64,
}

impl Default for InitrdInfo {
    fn default() -> Self {
        Self {
            entries: [InitrdEntry::default(); MAX_INITRDS],
            count: 0,
            combined_address: 0,
            combined_size: 0,
        }
    }
}

impl InitrdInfo {
    /// Add initrd
    pub fn add(&mut self, entry: InitrdEntry) -> bool {
        if self.count >= MAX_INITRDS {
            return false;
        }
        self.entries[self.count] = entry;
        self.count += 1;
        true
    }

    /// Check if any initrds present
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if all loaded
    pub fn all_loaded(&self) -> bool {
        self.entries[..self.count].iter().all(|e| e.loaded)
    }
}

// =============================================================================
// MEMORY LAYOUT
// =============================================================================

/// Maximum memory regions
pub const MAX_MEMORY_REGIONS: usize = 64;

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryRegionType {
    /// Available for use
    #[default]
    Available,
    /// Reserved by firmware
    Reserved,
    /// ACPI reclaimable
    AcpiReclaimable,
    /// ACPI NVS
    AcpiNvs,
    /// Memory mapped I/O
    Mmio,
    /// Memory mapped I/O port space
    MmioPortSpace,
    /// Kernel code
    KernelCode,
    /// Kernel data
    KernelData,
    /// Boot services code
    BootServicesCode,
    /// Boot services data
    BootServicesData,
    /// Runtime services code
    RuntimeServicesCode,
    /// Runtime services data
    RuntimeServicesData,
    /// Loader data
    LoaderData,
    /// Initrd
    Initrd,
    /// Framebuffer
    Framebuffer,
}

/// Memory region
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryRegion {
    /// Start address
    pub start: u64,
    /// Size
    pub size: u64,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Attributes
    pub attributes: u64,
}

/// Memory layout
#[derive(Debug)]
pub struct MemoryLayout {
    /// Regions
    pub regions: [MemoryRegion; MAX_MEMORY_REGIONS],
    /// Region count
    pub count: usize,
    /// Total memory
    pub total: u64,
    /// Available memory
    pub available: u64,
    /// Highest address
    pub highest_address: u64,
}

impl Default for MemoryLayout {
    fn default() -> Self {
        Self {
            regions: [MemoryRegion::default(); MAX_MEMORY_REGIONS],
            count: 0,
            total: 0,
            available: 0,
            highest_address: 0,
        }
    }
}

impl MemoryLayout {
    /// Add memory region
    pub fn add(&mut self, region: MemoryRegion) -> bool {
        if self.count >= MAX_MEMORY_REGIONS {
            return false;
        }
        self.regions[self.count] = region;
        self.count += 1;

        // Update totals
        self.total += region.size;
        if region.region_type == MemoryRegionType::Available {
            self.available += region.size;
        }

        let region_end = region.start + region.size;
        if region_end > self.highest_address {
            self.highest_address = region_end;
        }

        true
    }

    /// Find region containing address
    pub fn find(&self, address: u64) -> Option<&MemoryRegion> {
        for i in 0..self.count {
            let region = &self.regions[i];
            if address >= region.start && address < region.start + region.size {
                return Some(region);
            }
        }
        None
    }
}

// =============================================================================
// BOOT LOG
// =============================================================================

/// Maximum log entries
pub const MAX_LOG_ENTRIES: usize = 128;

/// Log message type
#[derive(Debug, Clone, Copy)]
pub enum LogMessage {
    /// Phase entered
    PhaseEnter,
    /// Phase completed
    PhaseComplete,
    /// Error occurred
    Error(u32),
    /// Warning
    Warning(u16),
    /// Info message
    Info(u16),
    /// Device found
    DeviceFound,
    /// Entry detected
    EntryDetected,
    /// User input
    UserInput,
    /// Timeout
    Timeout,
    /// Custom message ID
    Custom(u16),
}

/// Log entry
#[derive(Debug, Clone, Copy)]
pub struct LogEntry {
    /// Phase when logged
    pub phase: BootPhase,
    /// Timestamp
    pub timestamp_us: u64,
    /// Message
    pub message: LogMessage,
}

impl Default for LogEntry {
    fn default() -> Self {
        Self {
            phase: BootPhase::NotStarted,
            timestamp_us: 0,
            message: LogMessage::Info(0),
        }
    }
}

/// Boot log
#[derive(Debug)]
pub struct BootLog {
    /// Entries
    entries: [LogEntry; MAX_LOG_ENTRIES],
    /// Write index
    write_idx: usize,
    /// Entry count
    count: usize,
}

impl Default for BootLog {
    fn default() -> Self {
        Self::new()
    }
}

impl BootLog {
    /// Create new log
    pub const fn new() -> Self {
        Self {
            entries: [LogEntry {
                phase: BootPhase::NotStarted,
                timestamp_us: 0,
                message: LogMessage::Info(0),
            }; MAX_LOG_ENTRIES],
            write_idx: 0,
            count: 0,
        }
    }

    /// Add entry
    pub fn add_entry(&mut self, entry: LogEntry) {
        self.entries[self.write_idx] = entry;
        self.write_idx = (self.write_idx + 1) % MAX_LOG_ENTRIES;
        if self.count < MAX_LOG_ENTRIES {
            self.count += 1;
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

    /// Clear log
    pub fn clear(&mut self) {
        self.write_idx = 0;
        self.count = 0;
    }
}

// =============================================================================
// BOOT HOOKS
// =============================================================================

/// Hook type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookType {
    /// Before phase
    Pre,
    /// After phase
    Post,
}

/// Hook result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// Continue normally
    Continue,
    /// Skip phase
    Skip,
    /// Retry phase
    Retry,
    /// Abort boot
    Abort,
}

impl Default for HookResult {
    fn default() -> Self {
        HookResult::Continue
    }
}

/// Hook registration
#[derive(Debug, Clone, Copy)]
pub struct HookRegistration {
    /// Target phase
    pub phase: BootPhase,
    /// Hook type
    pub hook_type: HookType,
    /// Priority (lower = earlier)
    pub priority: u8,
    /// Handler address
    pub handler_addr: usize,
    /// Enabled
    pub enabled: bool,
}

impl Default for HookRegistration {
    fn default() -> Self {
        Self {
            phase: BootPhase::NotStarted,
            hook_type: HookType::Pre,
            priority: 128,
            handler_addr: 0,
            enabled: false,
        }
    }
}

/// Maximum hooks
pub const MAX_HOOKS: usize = 32;

/// Hook manager
#[derive(Debug)]
pub struct HookManager {
    /// Registered hooks
    hooks: [HookRegistration; MAX_HOOKS],
    /// Hook count
    count: usize,
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HookManager {
    /// Create new manager
    pub const fn new() -> Self {
        Self {
            hooks: [HookRegistration {
                phase: BootPhase::NotStarted,
                hook_type: HookType::Pre,
                priority: 128,
                handler_addr: 0,
                enabled: false,
            }; MAX_HOOKS],
            count: 0,
        }
    }

    /// Register hook
    pub fn register(&mut self, hook: HookRegistration) -> bool {
        if self.count >= MAX_HOOKS {
            return false;
        }
        self.hooks[self.count] = hook;
        self.count += 1;
        true
    }

    /// Get hooks for phase
    pub fn get_hooks(
        &self,
        phase: BootPhase,
        hook_type: HookType,
    ) -> impl Iterator<Item = &HookRegistration> {
        self.hooks[..self.count]
            .iter()
            .filter(move |h| h.phase == phase && h.hook_type == hook_type && h.enabled)
    }

    /// Get hook count
    pub const fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// =============================================================================
// BOOT ORCHESTRATOR
// =============================================================================

/// Boot orchestrator
#[derive(Debug)]
pub struct BootOrchestrator {
    /// Boot context
    pub context: BootContext,
    /// Hook manager
    pub hooks: HookManager,
    /// Initialized
    initialized: bool,
}

impl Default for BootOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl BootOrchestrator {
    /// Create new orchestrator
    pub fn new() -> Self {
        Self {
            context: BootContext::new(),
            hooks: HookManager::new(),
            initialized: false,
        }
    }

    /// Initialize orchestrator
    pub fn initialize(&mut self, timestamp_us: u64) -> Result<(), BootError> {
        if self.initialized {
            return Err(BootError::NotAllowed);
        }

        self.context.phases.start(timestamp_us);
        self.context.state.flags.set(BootStateFlags::INITIALIZED);
        self.initialized = true;
        Ok(())
    }

    /// Check if initialized
    pub const fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get current phase
    pub const fn current_phase(&self) -> BootPhase {
        self.context.current_phase()
    }

    /// Advance to next phase
    pub fn advance(&mut self, timestamp_us: u64) -> Result<BootPhase, BootError> {
        let next_phase = self.next_phase();
        self.context.transition(next_phase, timestamp_us)?;
        Ok(next_phase)
    }

    /// Determine next phase
    fn next_phase(&self) -> BootPhase {
        match self.context.state.phase {
            BootPhase::NotStarted => BootPhase::FirmwareEntry,
            BootPhase::FirmwareEntry => BootPhase::EarlyInit,
            BootPhase::EarlyInit => BootPhase::ConsoleInit,
            BootPhase::ConsoleInit => BootPhase::MemoryInit,
            BootPhase::MemoryInit => BootPhase::ConfigLoad,
            BootPhase::ConfigLoad => BootPhase::DeviceDiscovery,
            BootPhase::DeviceDiscovery => BootPhase::EntryDetection,
            BootPhase::EntryDetection => BootPhase::SecurityValidation,
            BootPhase::SecurityValidation => {
                if self.context.params.show_menu || !self.context.params.quick_boot {
                    BootPhase::MenuDisplay
                } else {
                    BootPhase::EntryPreparation
                }
            },
            BootPhase::MenuDisplay => BootPhase::UserSelection,
            BootPhase::UserSelection => BootPhase::EntryPreparation,
            BootPhase::EntryPreparation => BootPhase::KernelLoad,
            BootPhase::KernelLoad => BootPhase::InitrdLoad,
            BootPhase::InitrdLoad => BootPhase::PreBootHooks,
            BootPhase::PreBootHooks => BootPhase::ExitBootServices,
            BootPhase::ExitBootServices => BootPhase::HandoffPrep,
            BootPhase::HandoffPrep => BootPhase::KernelHandoff,
            BootPhase::KernelHandoff => BootPhase::BootComplete,
            BootPhase::BootComplete => BootPhase::BootComplete,
            BootPhase::BootFailed => BootPhase::BootFailed,
        }
    }

    /// Handle error
    pub fn handle_error(&mut self, error_code: u32, timestamp_us: u64) {
        self.context.state.set_error(error_code);
        self.context.log.add_entry(LogEntry {
            phase: self.context.state.phase,
            timestamp_us,
            message: LogMessage::Error(error_code),
        });
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_phase_order() {
        assert!(BootPhase::FirmwareEntry < BootPhase::EarlyInit);
        assert!(BootPhase::KernelLoad < BootPhase::BootComplete);
    }

    #[test]
    fn test_phase_tracker() {
        let mut tracker = PhaseTracker::new();
        tracker.start(0);
        assert_eq!(tracker.current(), BootPhase::FirmwareEntry);

        tracker.enter_phase(BootPhase::EarlyInit, 1000);
        assert_eq!(tracker.current(), BootPhase::EarlyInit);
    }

    #[test]
    fn test_boot_state_flags() {
        let mut flags = BootStateFlags::NONE;
        flags.set(BootStateFlags::INITIALIZED);
        assert!(flags.has(BootStateFlags::INITIALIZED));
        assert!(!flags.has(BootStateFlags::KERNEL_LOADED));

        flags.clear(BootStateFlags::INITIALIZED);
        assert!(!flags.has(BootStateFlags::INITIALIZED));
    }

    #[test]
    fn test_memory_layout() {
        let mut layout = MemoryLayout::default();
        layout.add(MemoryRegion {
            start: 0x100000,
            size: 0x1000000,
            region_type: MemoryRegionType::Available,
            attributes: 0,
        });

        assert_eq!(layout.count, 1);
        assert!(layout.find(0x500000).is_some());
        assert!(layout.find(0x0).is_none());
    }

    #[test]
    fn test_boot_orchestrator() {
        let mut orchestrator = BootOrchestrator::new();
        assert!(!orchestrator.is_initialized());

        orchestrator.initialize(0).unwrap();
        assert!(orchestrator.is_initialized());
        assert_eq!(orchestrator.current_phase(), BootPhase::FirmwareEntry);
    }
}
