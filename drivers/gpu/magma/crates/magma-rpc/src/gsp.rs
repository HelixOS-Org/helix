//! # GSP Interface
//!
//! GPU System Processor state machine and initialization.

use magma_core::{Error, GpuGeneration, Result};

// =============================================================================
// GSP STATE
// =============================================================================

/// GSP firmware state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GspState {
    /// Not initialized
    Uninitialized,
    /// Loading firmware
    Loading,
    /// Running boot sequence
    Booting,
    /// Ready for RPC
    Ready,
    /// Error state
    Error,
    /// Suspended
    Suspended,
    /// Shutting down
    ShuttingDown,
}

impl GspState {
    /// Check if GSP is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Check if GSP can accept commands
    pub fn can_accept_commands(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

// =============================================================================
// GSP INFO
// =============================================================================

/// GSP firmware information
#[derive(Debug, Clone)]
pub struct GspInfo {
    /// Firmware version
    pub version: GspVersion,
    /// Supported features
    pub features: GspFeatures,
    /// Number of RPC channels
    pub num_channels: u32,
    /// Maximum message size
    pub max_msg_size: u32,
    /// Current state
    pub state: GspState,
}

/// GSP version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GspVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
    /// Build number
    pub build: u16,
}

impl GspVersion {
    /// Create version from components
    pub const fn new(major: u16, minor: u16, patch: u16, build: u16) -> Self {
        Self {
            major,
            minor,
            patch,
            build,
        }
    }

    /// Parse from 64-bit value
    pub fn from_u64(value: u64) -> Self {
        Self {
            major: ((value >> 48) & 0xFFFF) as u16,
            minor: ((value >> 32) & 0xFFFF) as u16,
            patch: ((value >> 16) & 0xFFFF) as u16,
            build: (value & 0xFFFF) as u16,
        }
    }
}

/// GSP feature flags
bitflags::bitflags! {
    /// GSP supported features
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GspFeatures: u64 {
        /// Basic graphics support
        const GRAPHICS = 1 << 0;
        /// Compute support
        const COMPUTE = 1 << 1;
        /// Display support
        const DISPLAY = 1 << 2;
        /// Video decode
        const NVDEC = 1 << 3;
        /// Video encode
        const NVENC = 1 << 4;
        /// Raytracing
        const RT = 1 << 5;
        /// Tensor cores
        const TENSOR = 1 << 6;
        /// Multi-instance GPU
        const MIG = 1 << 7;
        /// Confidential computing
        const CC = 1 << 8;
        /// NVLink
        const NVLINK = 1 << 9;
        /// Power management
        const POWER_MGMT = 1 << 10;
        /// Thermal management
        const THERMAL_MGMT = 1 << 11;
        /// Memory ECC
        const ECC = 1 << 12;
        /// Page retirement
        const PAGE_RETIRE = 1 << 13;
        /// SR-IOV
        const SRIOV = 1 << 14;
        /// Heterogeneous memory
        const HMM = 1 << 15;
    }
}

// =============================================================================
// GSP BOOT PARAMS
// =============================================================================

/// Parameters for GSP boot
#[derive(Debug, Clone)]
pub struct GspBootParams {
    /// GPU generation
    pub generation: GpuGeneration,
    /// Firmware image address
    pub fw_addr: u64,
    /// Firmware image size
    pub fw_size: u64,
    /// Boot arguments address
    pub args_addr: u64,
    /// Boot arguments size
    pub args_size: u64,
    /// WPR (Write-Protected Region) base
    pub wpr_base: u64,
    /// WPR size
    pub wpr_size: u64,
    /// Heap address
    pub heap_addr: u64,
    /// Heap size
    pub heap_size: u64,
}

// =============================================================================
// GSP FALCON REGISTERS
// =============================================================================

/// GSP Falcon register offsets
pub mod falcon {
    //! Falcon microcontroller registers

    /// Falcon ID
    pub const FALCON_ID: u32 = 0x000;
    /// Falcon mailbox 0
    pub const FALCON_MAILBOX0: u32 = 0x040;
    /// Falcon mailbox 1
    pub const FALCON_MAILBOX1: u32 = 0x044;
    /// Falcon IRQSSET (interrupt set)
    pub const FALCON_IRQSSET: u32 = 0x000;
    /// Falcon IRQSCLR (interrupt clear)
    pub const FALCON_IRQSCLR: u32 = 0x004;
    /// Falcon IRQMSET (interrupt mask set)
    pub const FALCON_IRQMSET: u32 = 0x064;
    /// Falcon IRQMCLR (interrupt mask clear)
    pub const FALCON_IRQMCLR: u32 = 0x068;
    /// Falcon IRQDEST (interrupt destination)
    pub const FALCON_IRQDEST: u32 = 0x01C;
    /// Falcon scratch registers base
    pub const FALCON_SCRATCH: u32 = 0x080;
    /// Falcon OS
    pub const FALCON_OS: u32 = 0x080;

    /// Falcon CPUCTL
    pub const FALCON_CPUCTL: u32 = 0x100;
    /// Falcon BOOTVEC
    pub const FALCON_BOOTVEC: u32 = 0x104;
    /// Falcon HWCFG
    pub const FALCON_HWCFG: u32 = 0x108;
    /// Falcon DMACTL
    pub const FALCON_DMACTL: u32 = 0x10C;
    /// Falcon DMATRFBASE
    pub const FALCON_DMATRFBASE: u32 = 0x110;
    /// Falcon DMATRFMOFFS
    pub const FALCON_DMATRFMOFFS: u32 = 0x114;
    /// Falcon DMATRFCMD
    pub const FALCON_DMATRFCMD: u32 = 0x118;
    /// Falcon DMATRFFBOFFS
    pub const FALCON_DMATRFFBOFFS: u32 = 0x11C;

    /// CPUCTL start bit
    pub const CPUCTL_STARTCPU: u32 = 1 << 1;
    /// CPUCTL halt bit
    pub const CPUCTL_HALTED: u32 = 1 << 4;
}

// =============================================================================
// GSP INITIALIZATION SEQUENCE
// =============================================================================

/// GSP initialization steps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GspInitStep {
    /// Validate firmware image
    ValidateFirmware,
    /// Setup WPR (Write-Protected Region)
    SetupWpr,
    /// Load firmware to IMEM
    LoadImem,
    /// Load data to DMEM
    LoadDmem,
    /// Configure Falcon
    ConfigureFalcon,
    /// Start Falcon execution
    StartFalcon,
    /// Wait for boot completion
    WaitBoot,
    /// Setup RPC channels
    SetupRpc,
    /// Verify GSP ready
    VerifyReady,
    /// Complete
    Complete,
}

/// GSP initialization progress
#[derive(Debug)]
pub struct GspInitProgress {
    /// Current step
    pub step: GspInitStep,
    /// Step index (0-based)
    pub step_index: u8,
    /// Total steps
    pub total_steps: u8,
    /// Error if failed
    pub error: Option<Error>,
}

impl GspInitProgress {
    /// Total number of init steps
    pub const TOTAL_STEPS: u8 = 10;

    /// Create initial progress
    pub fn new() -> Self {
        Self {
            step: GspInitStep::ValidateFirmware,
            step_index: 0,
            total_steps: Self::TOTAL_STEPS,
            error: None,
        }
    }

    /// Move to next step
    pub fn next(&mut self) -> bool {
        if self.step == GspInitStep::Complete {
            return false;
        }

        self.step_index += 1;
        self.step = match self.step_index {
            0 => GspInitStep::ValidateFirmware,
            1 => GspInitStep::SetupWpr,
            2 => GspInitStep::LoadImem,
            3 => GspInitStep::LoadDmem,
            4 => GspInitStep::ConfigureFalcon,
            5 => GspInitStep::StartFalcon,
            6 => GspInitStep::WaitBoot,
            7 => GspInitStep::SetupRpc,
            8 => GspInitStep::VerifyReady,
            _ => GspInitStep::Complete,
        };

        true
    }

    /// Mark as failed
    pub fn fail(&mut self, error: Error) {
        self.error = Some(error);
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.step == GspInitStep::Complete && self.error.is_none()
    }

    /// Check if failed
    pub fn is_failed(&self) -> bool {
        self.error.is_some()
    }

    /// Get progress percentage
    pub fn percent(&self) -> u8 {
        ((self.step_index as u16 * 100) / self.total_steps as u16) as u8
    }
}

impl Default for GspInitProgress {
    fn default() -> Self {
        Self::new()
    }
}
