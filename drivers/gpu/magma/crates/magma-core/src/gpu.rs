//! # GPU Device Abstraction
//!
//! High-level GPU device structures and enumeration.

use crate::error::Result;
use crate::traits::*;
use crate::types::*;

// =============================================================================
// GPU INFO
// =============================================================================

/// Comprehensive GPU information structure
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// PCI address
    pub pci_addr: PciAddr,
    /// Device identification
    pub device_id: GpuDeviceId,
    /// GPU generation
    pub generation: GpuGeneration,
    /// Human-readable name
    pub name: &'static str,
    /// VRAM specifications
    pub vram: VramInfo,
    /// Engine availability
    pub engines: EngineInfo,
    /// PCIe link info
    pub pcie: PcieInfo,
}

/// VRAM information
#[derive(Debug, Clone, Default)]
pub struct VramInfo {
    /// Total VRAM size
    pub total: ByteSize,
    /// VRAM type
    pub memory_type: VramType,
    /// Memory bus width in bits
    pub bus_width: u32,
    /// Memory clock speed in MHz
    pub clock_mhz: u32,
    /// Bandwidth in GB/s
    pub bandwidth_gbps: u32,
}

/// VRAM memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VramType {
    /// Unknown type
    #[default]
    Unknown,
    /// GDDR5
    Gddr5,
    /// GDDR5X
    Gddr5x,
    /// GDDR6
    Gddr6,
    /// GDDR6X
    Gddr6x,
    /// HBM2
    Hbm2,
    /// HBM2e
    Hbm2e,
    /// HBM3
    Hbm3,
}

/// Engine availability information
#[derive(Debug, Clone, Default)]
pub struct EngineInfo {
    /// Number of graphics engines
    pub graphics_count: u32,
    /// Number of compute engines
    pub compute_count: u32,
    /// Number of copy engines
    pub copy_count: u32,
    /// Number of NVDEC engines
    pub nvdec_count: u32,
    /// Number of NVENC engines
    pub nvenc_count: u32,
    /// CUDA core count
    pub cuda_cores: u32,
    /// Tensor core count
    pub tensor_cores: u32,
    /// RT core count
    pub rt_cores: u32,
}

/// PCIe link information
#[derive(Debug, Clone, Default)]
pub struct PcieInfo {
    /// Current link width
    pub width: u8,
    /// Maximum link width
    pub max_width: u8,
    /// Current link speed (Gen number)
    pub speed: u8,
    /// Maximum link speed
    pub max_speed: u8,
}

// =============================================================================
// GPU ENUMERATION
// =============================================================================

/// Result of GPU enumeration
#[derive(Debug)]
pub struct GpuEnumeration {
    /// List of detected GPUs
    pub gpus: &'static [GpuInfo],
    /// Number of NVIDIA GPUs found
    pub nvidia_count: usize,
    /// Number of supported GPUs (with GSP)
    pub supported_count: usize,
}
