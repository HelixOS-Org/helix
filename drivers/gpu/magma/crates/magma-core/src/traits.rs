//! # MAGMA Core Traits
//!
//! Foundational traits that define the driver's architecture.
//!
//! These traits enable:
//! - Hardware abstraction across GPU generations
//! - Compile-time polymorphism (no vtables in hot paths)
//! - Clear separation of concerns
//!
//! ## Trait Hierarchy
//!
//! ```text
//! GpuDevice
//!    │
//!    ├── GpuEngine (Graphics, Compute, Copy, Video)
//!    │      │
//!    │      └── EngineCommands
//!    │
//!    ├── MemoryAllocator
//!    │      │
//!    │      ├── VramAllocator
//!    │      └── SysmemAllocator
//!    │
//!    └── CommandSubmitter
//!           │
//!           └── CommandRing
//! ```

use crate::error::Result;
use crate::types::*;

// =============================================================================
// GPU DEVICE TRAIT
// =============================================================================

/// Core GPU device trait
///
/// This is the primary abstraction over a physical GPU device.
/// Generation-specific implementations provide concrete behavior.
pub trait GpuDevice: Sized + Send + Sync {
    /// The memory allocator type for this device
    type Allocator: MemoryAllocator;

    /// The command submitter type
    type Submitter: CommandSubmitter;

    /// The GSP interface type (if supported)
    type Gsp: GspInterface;

    /// Get the device generation
    fn generation(&self) -> GpuGeneration;

    /// Get the device ID
    fn device_id(&self) -> GpuDeviceId;

    /// Get the PCI address
    fn pci_address(&self) -> PciAddr;

    /// Get total VRAM size
    fn vram_size(&self) -> ByteSize;

    /// Get available VRAM
    fn vram_available(&self) -> ByteSize;

    /// Get the memory allocator
    fn allocator(&self) -> &Self::Allocator;

    /// Get the command submitter
    fn submitter(&self) -> &Self::Submitter;

    /// Get the GSP interface
    fn gsp(&self) -> &Self::Gsp;

    /// Check if device is in a healthy state
    fn is_healthy(&self) -> bool;

    /// Perform a GPU reset if possible
    fn reset(&self) -> Result<()>;
}

// =============================================================================
// GPU ENGINE TRAITS
// =============================================================================

/// GPU engine type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineType {
    /// 3D Graphics engine (GR)
    Graphics,
    /// Compute engine (CUDA)
    Compute,
    /// Copy engine (CE/DMA)
    Copy,
    /// Video decode (NVDEC)
    VideoDecode,
    /// Video encode (NVENC)
    VideoEncode,
    /// Display engine
    Display,
}

/// Trait for GPU engines
///
/// Each engine has its own command format and scheduling.
pub trait GpuEngine: Send + Sync {
    /// Get the engine type
    fn engine_type(&self) -> EngineType;

    /// Get the engine index (for multi-instance engines)
    fn engine_index(&self) -> u32;

    /// Check if engine is idle
    fn is_idle(&self) -> Result<bool>;

    /// Wait for engine to become idle
    fn wait_idle(&self, timeout_ns: u64) -> Result<()>;

    /// Get engine-specific capabilities
    fn capabilities(&self) -> EngineCapabilities;
}

/// Engine capability flags
#[derive(Debug, Clone, Copy, Default)]
pub struct EngineCapabilities {
    /// Maximum concurrent contexts
    pub max_contexts: u32,
    /// Maximum command buffer size
    pub max_command_size: ByteSize,
    /// Supports compute preemption
    pub supports_preemption: bool,
    /// Supports hardware scheduling
    pub supports_hw_scheduling: bool,
}

// =============================================================================
// MEMORY ALLOCATOR TRAIT
// =============================================================================

/// Memory allocation flags
bitflags::bitflags! {
    /// Flags for memory allocation
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AllocFlags: u32 {
        /// CPU accessible (mappable)
        const CPU_VISIBLE = 1 << 0;
        /// GPU local (VRAM preferred)
        const GPU_LOCAL = 1 << 1;
        /// Coherent (no explicit flush needed)
        const COHERENT = 1 << 2;
        /// Cached on CPU
        const CPU_CACHED = 1 << 3;
        /// Write-combined (good for streaming)
        const WRITE_COMBINE = 1 << 4;
        /// Page-aligned allocation
        const PAGE_ALIGNED = 1 << 5;
        /// Contiguous physical pages
        const CONTIGUOUS = 1 << 6;
        /// Protected/encrypted memory
        const PROTECTED = 1 << 7;
    }
}

/// Memory allocation descriptor
#[derive(Debug, Clone)]
pub struct AllocDesc {
    /// Size in bytes
    pub size: ByteSize,
    /// Alignment requirement
    pub alignment: u64,
    /// Allocation flags
    pub flags: AllocFlags,
    /// Debug name (for tools)
    pub name: Option<&'static str>,
}

impl Default for AllocDesc {
    fn default() -> Self {
        Self {
            size: ByteSize::ZERO,
            alignment: 4096,
            flags: AllocFlags::GPU_LOCAL,
            name: None,
        }
    }
}

/// GPU memory allocation result
#[derive(Debug)]
pub struct Allocation {
    /// GPU virtual address
    pub gpu_addr: GpuAddr,
    /// Size in bytes
    pub size: ByteSize,
    /// Allocation handle (for freeing)
    pub handle: BufferHandle,
    /// CPU mapping (if CPU_VISIBLE)
    pub cpu_ptr: Option<*mut u8>,
}

/// Memory allocator trait
pub trait MemoryAllocator: Send + Sync {
    /// Allocate GPU memory
    fn allocate(&self, desc: &AllocDesc) -> Result<Allocation>;

    /// Free previously allocated memory
    fn free(&self, handle: BufferHandle) -> Result<()>;

    /// Map allocation to CPU address space
    fn map(&self, handle: BufferHandle) -> Result<*mut u8>;

    /// Unmap allocation from CPU
    fn unmap(&self, handle: BufferHandle) -> Result<()>;

    /// Flush CPU writes to GPU
    fn flush(&self, handle: BufferHandle, offset: u64, size: ByteSize) -> Result<()>;

    /// Invalidate CPU caches
    fn invalidate(&self, handle: BufferHandle, offset: u64, size: ByteSize) -> Result<()>;

    /// Get allocation info
    fn info(&self, handle: BufferHandle) -> Result<Allocation>;

    /// Get total memory capacity
    fn total_capacity(&self) -> ByteSize;

    /// Get available memory
    fn available(&self) -> ByteSize;
}

// =============================================================================
// COMMAND SUBMISSION TRAIT
// =============================================================================

/// Command submission priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubmitPriority {
    /// Low priority (background tasks)
    Low      = 0,
    /// Normal priority (default)
    Normal   = 1,
    /// High priority (interactive)
    High     = 2,
    /// Realtime priority (display, VR)
    Realtime = 3,
}

/// Command submission descriptor
#[derive(Debug, Clone)]
pub struct SubmitDesc {
    /// Target engine
    pub engine: EngineType,
    /// Submission priority
    pub priority: SubmitPriority,
    /// Wait on these fences before execution
    pub wait_fences: &'static [FenceHandle],
    /// Signal these fences after completion
    pub signal_fences: &'static [FenceHandle],
}

/// Command submitter trait
pub trait CommandSubmitter: Send + Sync {
    /// Submit commands to GPU
    fn submit(&self, desc: &SubmitDesc, commands: &[u8]) -> Result<FenceHandle>;

    /// Wait for fence to signal
    fn wait_fence(&self, fence: FenceHandle, timeout_ns: u64) -> Result<bool>;

    /// Query fence status
    fn query_fence(&self, fence: FenceHandle) -> Result<bool>;

    /// Signal fence from CPU (for testing)
    fn signal_fence(&self, fence: FenceHandle) -> Result<()>;
}

// =============================================================================
// GSP INTERFACE TRAIT
// =============================================================================

/// GSP firmware state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GspState {
    /// Not initialized
    NotInitialized,
    /// Loading firmware
    Loading,
    /// Handshaking
    Handshaking,
    /// Ready
    Ready,
    /// Error state
    Error,
}

/// GSP RPC message
#[derive(Debug, Clone)]
pub struct GspMessage {
    /// Function ID (RM API function)
    pub function: u32,
    /// Payload data
    pub payload: &'static [u8],
    /// Response buffer
    pub response: &'static mut [u8],
}

/// GSP (GPU System Processor) interface
pub trait GspInterface: Send + Sync {
    /// Get current GSP state
    fn state(&self) -> GspState;

    /// Load and authenticate GSP firmware
    fn load_firmware(&self, firmware: &[u8]) -> Result<()>;

    /// Perform handshake
    fn handshake(&self) -> Result<()>;

    /// Send RPC message to GSP
    fn send_rpc(&self, msg: &GspMessage) -> Result<()>;

    /// Receive RPC response
    fn recv_rpc(&self, timeout_ns: u64) -> Result<()>;

    /// Get GSP version info
    fn version(&self) -> GspVersionInfo;
}

/// GSP version information
#[derive(Debug, Clone, Default)]
pub struct GspVersionInfo {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
    /// Build number
    pub build: u32,
    /// Version string
    pub version_str: &'static str,
}

// =============================================================================
// RESOURCE LIFECYCLE TRAITS
// =============================================================================

/// Trait for resources that can be destroyed
pub trait Destroyable {
    /// Destroy the resource, releasing all GPU memory
    fn destroy(self) -> Result<()>;
}

/// Trait for resources that can be named (for debugging)
pub trait Nameable {
    /// Set debug name
    fn set_name(&mut self, name: &str) -> Result<()>;

    /// Get debug name
    fn name(&self) -> Option<&str>;
}

// =============================================================================
// STATIC ASSERTIONS
// =============================================================================

// Ensure key types are Send + Sync
static_assertions::assert_impl_all!(GpuAddr: Send, Sync, Copy);
static_assertions::assert_impl_all!(PhysAddr: Send, Sync, Copy);
static_assertions::assert_impl_all!(PciAddr: Send, Sync, Copy);
static_assertions::assert_impl_all!(ByteSize: Send, Sync, Copy);
