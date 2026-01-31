//! Hardware and Software Events
//!
//! Event type definitions for performance monitoring.

use alloc::string::String;

// ============================================================================
// HARDWARE EVENTS
// ============================================================================

/// Hardware event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareEvent {
    /// CPU cycles
    CpuCycles,
    /// Instructions retired
    Instructions,
    /// Cache references
    CacheReferences,
    /// Cache misses
    CacheMisses,
    /// Branch instructions
    BranchInstructions,
    /// Branch misses
    BranchMisses,
    /// Bus cycles
    BusCycles,
    /// Stalled cycles (frontend)
    StalledCyclesFrontend,
    /// Stalled cycles (backend)
    StalledCyclesBackend,
    /// Reference cycles
    RefCpuCycles,
}

impl HardwareEvent {
    /// Get event name
    pub fn name(&self) -> &'static str {
        match self {
            Self::CpuCycles => "cpu-cycles",
            Self::Instructions => "instructions",
            Self::CacheReferences => "cache-references",
            Self::CacheMisses => "cache-misses",
            Self::BranchInstructions => "branch-instructions",
            Self::BranchMisses => "branch-misses",
            Self::BusCycles => "bus-cycles",
            Self::StalledCyclesFrontend => "stalled-cycles-frontend",
            Self::StalledCyclesBackend => "stalled-cycles-backend",
            Self::RefCpuCycles => "ref-cycles",
        }
    }

    /// Event code for x86
    pub fn x86_code(&self) -> Option<u64> {
        match self {
            Self::CpuCycles => Some(0x003c),
            Self::Instructions => Some(0x00c0),
            Self::CacheReferences => Some(0x2f4f), // LLC refs
            Self::CacheMisses => Some(0x412e),     // LLC misses
            Self::BranchInstructions => Some(0x00c4),
            Self::BranchMisses => Some(0x00c5),
            _ => None,
        }
    }
}

// ============================================================================
// SOFTWARE EVENTS
// ============================================================================

/// Software event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftwareEvent {
    /// Context switches
    ContextSwitches,
    /// CPU migrations
    CpuMigrations,
    /// Page faults
    PageFaults,
    /// Minor page faults
    MinorFaults,
    /// Major page faults
    MajorFaults,
    /// Alignment faults
    AlignmentFaults,
    /// Emulation faults
    EmulationFaults,
    /// Dummy
    Dummy,
}

impl SoftwareEvent {
    /// Get event name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ContextSwitches => "context-switches",
            Self::CpuMigrations => "cpu-migrations",
            Self::PageFaults => "page-faults",
            Self::MinorFaults => "minor-faults",
            Self::MajorFaults => "major-faults",
            Self::AlignmentFaults => "alignment-faults",
            Self::EmulationFaults => "emulation-faults",
            Self::Dummy => "dummy",
        }
    }
}

// ============================================================================
// CACHE EVENTS
// ============================================================================

/// Cache event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLevel {
    /// L1 Data
    L1D,
    /// L1 Instruction
    L1I,
    /// Last Level Cache
    LL,
    /// Data TLB
    DTLB,
    /// Instruction TLB
    ITLB,
    /// Branch Prediction
    BPU,
    /// Node (NUMA)
    Node,
}

impl CacheLevel {
    /// Get level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::L1D => "L1-dcache",
            Self::L1I => "L1-icache",
            Self::LL => "LLC",
            Self::DTLB => "dTLB",
            Self::ITLB => "iTLB",
            Self::BPU => "branch",
            Self::Node => "node",
        }
    }
}

/// Cache operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheOp {
    /// Read
    Read,
    /// Write
    Write,
    /// Prefetch
    Prefetch,
}

/// Cache result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheResult {
    /// Access (hit or miss)
    Access,
    /// Miss only
    Miss,
}

/// Cache event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheEvent {
    /// Cache level
    pub level: CacheLevel,
    /// Operation
    pub op: CacheOp,
    /// Result
    pub result: CacheResult,
}

// ============================================================================
// EVENT TYPE
// ============================================================================

/// Event type
#[derive(Debug, Clone)]
pub enum EventType {
    /// Hardware event
    Hardware(HardwareEvent),
    /// Software event
    Software(SoftwareEvent),
    /// Cache event
    Cache(CacheEvent),
    /// Tracepoint
    Tracepoint { system: String, name: String },
    /// Raw hardware
    Raw(u64),
}

impl EventType {
    /// Get event name
    pub fn name(&self) -> String {
        match self {
            Self::Hardware(hw) => String::from(hw.name()),
            Self::Software(sw) => String::from(sw.name()),
            Self::Cache(c) => alloc::format!(
                "{}-{}-{}",
                c.level.name(),
                match c.op {
                    CacheOp::Read => "loads",
                    CacheOp::Write => "stores",
                    CacheOp::Prefetch => "prefetches",
                },
                match c.result {
                    CacheResult::Access => "",
                    CacheResult::Miss => "-misses",
                }
            ),
            Self::Tracepoint { system, name } => alloc::format!("{}:{}", system, name),
            Self::Raw(code) => alloc::format!("raw:0x{:x}", code),
        }
    }
}
