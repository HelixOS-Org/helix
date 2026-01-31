//! Raw Events and Event Data Types
//!
//! Defines the raw events collected by probes and their data payloads.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;
use super::probe::ProbeType;

// ============================================================================
// RAW EVENT
// ============================================================================

/// Raw event from a probe
#[derive(Debug, Clone)]
pub struct RawEvent {
    /// Event ID
    pub id: EventId,
    /// Probe that generated this event
    pub probe_id: ProbeId,
    /// Probe type
    pub probe_type: ProbeType,
    /// Timestamp when event occurred
    pub timestamp: Timestamp,
    /// Sequence number within probe
    pub sequence: u64,
    /// CPU where event occurred
    pub cpu: u32,
    /// Process ID (if applicable)
    pub pid: Option<u32>,
    /// Thread ID (if applicable)
    pub tid: Option<u32>,
    /// Event data
    pub data: EventData,
}

impl RawEvent {
    /// Create new raw event
    pub fn new(probe_id: ProbeId, probe_type: ProbeType, data: EventData) -> Self {
        Self {
            id: EventId::generate(),
            probe_id,
            probe_type,
            timestamp: Timestamp::now(),
            sequence: 0,
            cpu: 0,
            pid: None,
            tid: None,
            data,
        }
    }

    /// With sequence number
    pub fn with_sequence(mut self, seq: u64) -> Self {
        self.sequence = seq;
        self
    }

    /// With CPU
    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = cpu;
        self
    }

    /// With process context
    pub fn with_process(mut self, pid: u32, tid: Option<u32>) -> Self {
        self.pid = Some(pid);
        self.tid = tid;
        self
    }
}

// ============================================================================
// EVENT DATA
// ============================================================================

/// Event data variants
#[derive(Debug, Clone)]
pub enum EventData {
    /// CPU utilization sample
    CpuSample(CpuSample),
    /// Memory usage sample
    MemorySample(MemorySample),
    /// Block I/O operation
    BlockIoEvent(BlockIoEvent),
    /// Network I/O operation
    NetworkIoEvent(NetworkIoEvent),
    /// Scheduler event
    SchedulerEvent(SchedulerEventData),
    /// Interrupt
    InterruptEvent(InterruptEventData),
    /// System call
    SyscallEvent(SyscallEventData),
    /// Page fault
    PageFaultEvent(PageFaultEventData),
    /// Timer tick
    TimerEvent(TimerEventData),
    /// Power state change
    PowerEvent(PowerEventData),
    /// Thermal reading
    ThermalEvent(ThermalEventData),
    /// Device event
    DeviceEvent(DeviceEventData),
    /// Filesystem event
    FilesystemEvent(FilesystemEventData),
    /// Security event
    SecurityEvent(SecurityEventData),
    /// Generic metric
    Metric(MetricSample),
    /// Raw bytes
    Raw(Vec<u8>),
}

// ============================================================================
// CPU PROBE DATA
// ============================================================================

/// CPU sample data
#[derive(Debug, Clone)]
pub struct CpuSample {
    /// CPU ID
    pub cpu_id: u32,
    /// User time (%)
    pub user: u8,
    /// System time (%)
    pub system: u8,
    /// Idle time (%)
    pub idle: u8,
    /// I/O wait (%)
    pub iowait: u8,
    /// IRQ time (%)
    pub irq: u8,
    /// Soft IRQ time (%)
    pub softirq: u8,
    /// Steal time (%)
    pub steal: u8,
    /// Frequency in MHz
    pub frequency_mhz: u32,
    /// Temperature in Celsius
    pub temperature: Option<i16>,
}

impl CpuSample {
    /// Total busy time
    pub fn busy_percent(&self) -> u8 {
        100u8.saturating_sub(self.idle)
    }

    /// Total system overhead
    pub fn overhead_percent(&self) -> u8 {
        self.system.saturating_add(self.irq).saturating_add(self.softirq)
    }
}

// ============================================================================
// MEMORY PROBE DATA
// ============================================================================

/// Memory sample data
#[derive(Debug, Clone)]
pub struct MemorySample {
    /// Total physical memory in bytes
    pub total: u64,
    /// Used memory in bytes
    pub used: u64,
    /// Free memory in bytes
    pub free: u64,
    /// Available memory in bytes
    pub available: u64,
    /// Buffers in bytes
    pub buffers: u64,
    /// Cached in bytes
    pub cached: u64,
    /// Swap total in bytes
    pub swap_total: u64,
    /// Swap used in bytes
    pub swap_used: u64,
    /// Page faults per second
    pub page_faults: u64,
    /// Major faults per second
    pub major_faults: u64,
}

impl MemorySample {
    /// Usage percentage
    pub fn usage_percent(&self) -> u8 {
        if self.total == 0 {
            0
        } else {
            ((self.used * 100) / self.total) as u8
        }
    }

    /// Swap usage percentage
    pub fn swap_percent(&self) -> u8 {
        if self.swap_total == 0 {
            0
        } else {
            ((self.swap_used * 100) / self.swap_total) as u8
        }
    }

    /// Is memory pressure high?
    pub fn is_pressure_high(&self) -> bool {
        self.usage_percent() > 85 || self.major_faults > 100
    }
}

// ============================================================================
// BLOCK I/O PROBE DATA
// ============================================================================

/// Block I/O event
#[derive(Debug, Clone)]
pub struct BlockIoEvent {
    /// Device ID
    pub device: u32,
    /// Operation type
    pub operation: IoOperation,
    /// Sector number
    pub sector: u64,
    /// Number of sectors
    pub num_sectors: u32,
    /// Latency in nanoseconds
    pub latency_ns: u64,
    /// Error code (0 = success)
    pub error: i32,
}

/// I/O operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOperation {
    Read,
    Write,
    Flush,
    Discard,
    Other,
}

// ============================================================================
// NETWORK I/O PROBE DATA
// ============================================================================

/// Network I/O event
#[derive(Debug, Clone)]
pub struct NetworkIoEvent {
    /// Network interface index
    pub interface: u32,
    /// Direction
    pub direction: NetworkDirection,
    /// Protocol
    pub protocol: NetworkProtocol,
    /// Bytes transferred
    pub bytes: u64,
    /// Packets transferred
    pub packets: u64,
    /// Errors
    pub errors: u32,
    /// Drops
    pub drops: u32,
}

/// Network direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDirection {
    Rx,
    Tx,
}

/// Network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    Icmp,
    Other,
}

// ============================================================================
// SCHEDULER PROBE DATA
// ============================================================================

/// Scheduler event data
#[derive(Debug, Clone)]
pub struct SchedulerEventData {
    /// Event type
    pub event_type: SchedulerEventType,
    /// PID involved
    pub pid: u32,
    /// Previous PID (for switch)
    pub prev_pid: Option<u32>,
    /// CPU involved
    pub cpu: u32,
    /// Priority
    pub priority: i32,
    /// Runtime in nanoseconds
    pub runtime_ns: Option<u64>,
}

/// Scheduler event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerEventType {
    Wakeup,
    Switch,
    Fork,
    Exit,
    Migrate,
    Wait,
}

// ============================================================================
// INTERRUPT PROBE DATA
// ============================================================================

/// Interrupt event data
#[derive(Debug, Clone)]
pub struct InterruptEventData {
    /// IRQ number
    pub irq: u32,
    /// Handler name
    pub handler: Option<String>,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Was handled
    pub handled: bool,
}

// ============================================================================
// SYSCALL PROBE DATA
// ============================================================================

/// System call event data
#[derive(Debug, Clone)]
pub struct SyscallEventData {
    /// System call number
    pub syscall_nr: u32,
    /// Entry or exit
    pub is_entry: bool,
    /// Return value (for exit)
    pub ret: Option<i64>,
    /// Duration in nanoseconds (for exit)
    pub duration_ns: Option<u64>,
}

// ============================================================================
// PAGE FAULT PROBE DATA
// ============================================================================

/// Page fault event data
#[derive(Debug, Clone)]
pub struct PageFaultEventData {
    /// Fault address
    pub address: u64,
    /// Is major fault
    pub major: bool,
    /// Is write fault
    pub write: bool,
    /// Is user-mode fault
    pub user: bool,
    /// Error code
    pub error_code: u64,
}

// ============================================================================
// TIMER PROBE DATA
// ============================================================================

/// Timer event data
#[derive(Debug, Clone)]
pub struct TimerEventData {
    /// Timer ID
    pub timer_id: u64,
    /// Expected expiry
    pub expected_ns: u64,
    /// Actual expiry
    pub actual_ns: u64,
    /// Slack (difference)
    pub slack_ns: i64,
}

// ============================================================================
// POWER PROBE DATA
// ============================================================================

/// Power event data
#[derive(Debug, Clone)]
pub struct PowerEventData {
    /// Event type
    pub event_type: PowerEventType,
    /// CPU involved (if applicable)
    pub cpu: Option<u32>,
    /// Target state
    pub target_state: u32,
    /// Duration (for transitions)
    pub duration_ns: Option<u64>,
}

/// Power event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEventType {
    CpuIdle,
    CpuFrequency,
    Suspend,
    Resume,
}

// ============================================================================
// THERMAL PROBE DATA
// ============================================================================

/// Thermal event data
#[derive(Debug, Clone)]
pub struct ThermalEventData {
    /// Thermal zone
    pub zone: u32,
    /// Temperature in milliCelsius
    pub temp_mc: i32,
    /// Trip point (if triggered)
    pub trip_point: Option<u32>,
    /// Throttle request
    pub throttle: bool,
}

// ============================================================================
// DEVICE PROBE DATA
// ============================================================================

/// Device event data
#[derive(Debug, Clone)]
pub struct DeviceEventData {
    /// Device path
    pub path: String,
    /// Event type
    pub event_type: DeviceEventType,
    /// Driver name
    pub driver: Option<String>,
}

/// Device event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEventType {
    Add,
    Remove,
    Change,
    Bind,
    Unbind,
}

// ============================================================================
// FILESYSTEM PROBE DATA
// ============================================================================

/// Filesystem event data
#[derive(Debug, Clone)]
pub struct FilesystemEventData {
    /// Filesystem type
    pub fstype: String,
    /// Mount point
    pub mount: String,
    /// Operation
    pub operation: FsOperation,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Error code
    pub error: i32,
}

/// Filesystem operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsOperation {
    Mount,
    Unmount,
    Read,
    Write,
    Create,
    Delete,
    Sync,
}

// ============================================================================
// SECURITY PROBE DATA
// ============================================================================

/// Security event data
#[derive(Debug, Clone)]
pub struct SecurityEventData {
    /// Event type
    pub event_type: SecurityEventType,
    /// Subject (user/process)
    pub subject: u32,
    /// Object (file/resource)
    pub object: Option<String>,
    /// Action attempted
    pub action: String,
    /// Result
    pub allowed: bool,
}

/// Security event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEventType {
    AccessCheck,
    CapabilityCheck,
    SelinuxAvc,
    AppArmor,
    Seccomp,
    Audit,
}

// ============================================================================
// METRIC SAMPLE
// ============================================================================

/// Generic metric sample
#[derive(Debug, Clone)]
pub struct MetricSample {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: MetricValue,
    /// Unit
    pub unit: MetricUnit,
    /// Tags
    pub tags: Tags,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_sample() {
        let sample = CpuSample {
            cpu_id: 0,
            user: 30,
            system: 20,
            idle: 50,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            frequency_mhz: 3000,
            temperature: None,
        };
        assert_eq!(sample.busy_percent(), 50);
        assert_eq!(sample.overhead_percent(), 20);
    }

    #[test]
    fn test_memory_sample() {
        let sample = MemorySample {
            total: 100,
            used: 50,
            free: 50,
            available: 50,
            buffers: 0,
            cached: 0,
            swap_total: 100,
            swap_used: 10,
            page_faults: 0,
            major_faults: 0,
        };
        assert_eq!(sample.usage_percent(), 50);
        assert_eq!(sample.swap_percent(), 10);
        assert!(!sample.is_pressure_high());
    }

    #[test]
    fn test_raw_event() {
        let event = RawEvent::new(
            ProbeId::generate(),
            ProbeType::Cpu,
            EventData::Raw(vec![1, 2, 3]),
        )
        .with_cpu(0)
        .with_process(1234, Some(1234));

        assert_eq!(event.cpu, 0);
        assert_eq!(event.pid, Some(1234));
    }
}
