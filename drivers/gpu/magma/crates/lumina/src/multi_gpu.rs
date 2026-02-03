//! Multi-GPU and device group types
//!
//! This module provides types for multi-GPU rendering and device groups.

extern crate alloc;
use alloc::vec::Vec;

use crate::buffer::BufferHandle;
use crate::memory::DeviceMemory;
use crate::sync::{FenceHandle, SemaphoreHandle};
use crate::texture::TextureHandle;

/// Device group handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeviceGroupHandle(pub u64);

impl DeviceGroupHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Physical device group properties
#[derive(Clone, Debug)]
pub struct PhysicalDeviceGroupProperties {
    /// Physical devices in the group
    pub physical_devices: Vec<PhysicalDeviceHandle>,
    /// Whether all devices support subset allocation
    pub subset_allocation: bool,
}

/// Physical device handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PhysicalDeviceHandle(pub u64);

impl PhysicalDeviceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Device group device create info
#[derive(Clone, Debug)]
pub struct DeviceGroupDeviceCreateInfo {
    /// Physical devices to use
    pub physical_devices: Vec<PhysicalDeviceHandle>,
}

impl DeviceGroupDeviceCreateInfo {
    /// Creates new device group info
    pub fn new() -> Self {
        Self {
            physical_devices: Vec::new(),
        }
    }

    /// Adds a physical device
    pub fn add_device(mut self, device: PhysicalDeviceHandle) -> Self {
        self.physical_devices.push(device);
        self
    }
}

impl Default for DeviceGroupDeviceCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Device mask for multi-GPU
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DeviceMask(pub u32);

impl DeviceMask {
    /// All devices
    pub const ALL: Self = Self(0xFFFFFFFF);
    /// Device 0 only
    pub const DEVICE_0: Self = Self(1 << 0);
    /// Device 1 only
    pub const DEVICE_1: Self = Self(1 << 1);
    /// Device 2 only
    pub const DEVICE_2: Self = Self(1 << 2);
    /// Device 3 only
    pub const DEVICE_3: Self = Self(1 << 3);

    /// Creates mask for single device
    pub const fn single(index: u32) -> Self {
        Self(1 << index)
    }

    /// Creates mask from device indices
    pub fn from_indices(indices: &[u32]) -> Self {
        let mut mask = 0u32;
        for &index in indices {
            mask |= 1 << index;
        }
        Self(mask)
    }

    /// Checks if device is included
    pub const fn includes(&self, device_index: u32) -> bool {
        (self.0 & (1 << device_index)) != 0
    }

    /// Returns the number of devices in the mask
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }
}

impl core::ops::BitOr for DeviceMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for DeviceMask {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Memory allocate flags for device groups
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryAllocateDeviceFlags(pub u32);

impl MemoryAllocateDeviceFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device mask allocation
    pub const DEVICE_MASK: Self = Self(1 << 0);
    /// Device address
    pub const DEVICE_ADDRESS: Self = Self(1 << 1);
    /// Device address capture replay
    pub const DEVICE_ADDRESS_CAPTURE_REPLAY: Self = Self(1 << 2);
}

/// Memory allocate info for device groups
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryAllocateFlagsInfo {
    /// Flags
    pub flags: MemoryAllocateDeviceFlags,
    /// Device mask
    pub device_mask: DeviceMask,
}

impl MemoryAllocateFlagsInfo {
    /// Creates new flags info
    pub const fn new() -> Self {
        Self {
            flags: MemoryAllocateDeviceFlags::NONE,
            device_mask: DeviceMask::ALL,
        }
    }

    /// Sets device mask
    pub const fn with_device_mask(mut self, mask: DeviceMask) -> Self {
        self.flags = MemoryAllocateDeviceFlags::DEVICE_MASK;
        self.device_mask = mask;
        self
    }
}

/// Bind buffer memory device group info
#[derive(Clone, Debug)]
pub struct BindBufferMemoryDeviceGroupInfo {
    /// Device indices for each device in the group
    pub device_indices: Vec<u32>,
}

impl BindBufferMemoryDeviceGroupInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            device_indices: Vec::new(),
        }
    }

    /// Adds a device index
    pub fn add_device(mut self, index: u32) -> Self {
        self.device_indices.push(index);
        self
    }
}

impl Default for BindBufferMemoryDeviceGroupInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Bind image memory device group info
#[derive(Clone, Debug)]
pub struct BindImageMemoryDeviceGroupInfo {
    /// Device indices for each device in the group
    pub device_indices: Vec<u32>,
    /// Split instance bind regions
    pub split_instance_bind_regions: Vec<DeviceRect>,
}

impl BindImageMemoryDeviceGroupInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            device_indices: Vec::new(),
            split_instance_bind_regions: Vec::new(),
        }
    }
}

impl Default for BindImageMemoryDeviceGroupInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Device-specific rectangle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DeviceRect {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl DeviceRect {
    /// Creates new device rect
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// Device group render pass begin info
#[derive(Clone, Debug)]
pub struct DeviceGroupRenderPassBeginInfo {
    /// Device mask
    pub device_mask: DeviceMask,
    /// Device render areas
    pub device_render_areas: Vec<DeviceRect>,
}

impl DeviceGroupRenderPassBeginInfo {
    /// Creates new info
    pub fn new(device_mask: DeviceMask) -> Self {
        Self {
            device_mask,
            device_render_areas: Vec::new(),
        }
    }

    /// Adds a render area
    pub fn add_render_area(mut self, rect: DeviceRect) -> Self {
        self.device_render_areas.push(rect);
        self
    }
}

/// Device group command buffer begin info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DeviceGroupCommandBufferBeginInfo {
    /// Device mask
    pub device_mask: DeviceMask,
}

impl DeviceGroupCommandBufferBeginInfo {
    /// Creates new info
    pub const fn new(device_mask: DeviceMask) -> Self {
        Self { device_mask }
    }
}

/// Device group submit info
#[derive(Clone, Debug)]
pub struct DeviceGroupSubmitInfo {
    /// Wait semaphore device indices
    pub wait_semaphore_device_indices: Vec<u32>,
    /// Command buffer device masks
    pub command_buffer_device_masks: Vec<DeviceMask>,
    /// Signal semaphore device indices
    pub signal_semaphore_device_indices: Vec<u32>,
}

impl DeviceGroupSubmitInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            wait_semaphore_device_indices: Vec::new(),
            command_buffer_device_masks: Vec::new(),
            signal_semaphore_device_indices: Vec::new(),
        }
    }
}

impl Default for DeviceGroupSubmitInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Device group bind sparse info
#[derive(Clone, Debug)]
pub struct DeviceGroupBindSparseInfo {
    /// Resource device index
    pub resource_device_index: u32,
    /// Memory device index
    pub memory_device_index: u32,
}

impl DeviceGroupBindSparseInfo {
    /// Creates new info
    pub const fn new(resource_device_index: u32, memory_device_index: u32) -> Self {
        Self {
            resource_device_index,
            memory_device_index,
        }
    }
}

/// Device group present capabilities
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DeviceGroupPresentCapabilities {
    /// Present modes for each device
    pub present_mask: [DeviceMask; 32],
    /// Supported present modes
    pub modes: DeviceGroupPresentMode,
}

/// Device group present modes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DeviceGroupPresentMode(pub u32);

impl DeviceGroupPresentMode {
    /// Local presentation (same device)
    pub const LOCAL: Self = Self(1 << 0);
    /// Remote presentation (different device)
    pub const REMOTE: Self = Self(1 << 1);
    /// Sum presentation (combine outputs)
    pub const SUM: Self = Self(1 << 2);
    /// Local multi-device presentation
    pub const LOCAL_MULTI_DEVICE: Self = Self(1 << 3);

    /// Checks if mode is supported
    pub const fn supports(&self, mode: Self) -> bool {
        (self.0 & mode.0) != 0
    }
}

impl core::ops::BitOr for DeviceGroupPresentMode {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Device group present info
#[derive(Clone, Debug)]
pub struct DeviceGroupPresentInfo {
    /// Device masks for each swapchain
    pub device_masks: Vec<DeviceMask>,
    /// Present mode
    pub mode: DeviceGroupPresentMode,
}

impl DeviceGroupPresentInfo {
    /// Creates new info
    pub fn new(mode: DeviceGroupPresentMode) -> Self {
        Self {
            device_masks: Vec::new(),
            mode,
        }
    }

    /// Adds device mask for swapchain
    pub fn add_device_mask(mut self, mask: DeviceMask) -> Self {
        self.device_masks.push(mask);
        self
    }
}

/// Image swapchain create info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ImageSwapchainCreateInfo {
    /// Swapchain handle
    pub swapchain: crate::queue::SwapchainHandle,
}

/// Device group swapchain create info
#[derive(Clone, Debug)]
pub struct DeviceGroupSwapchainCreateInfo {
    /// Present modes
    pub modes: DeviceGroupPresentMode,
}

impl DeviceGroupSwapchainCreateInfo {
    /// Creates new info
    pub const fn new(modes: DeviceGroupPresentMode) -> Self {
        Self { modes }
    }
}

/// Peer memory features
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PeerMemoryFeatures(pub u32);

impl PeerMemoryFeatures {
    /// Copy source
    pub const COPY_SRC: Self = Self(1 << 0);
    /// Copy destination
    pub const COPY_DST: Self = Self(1 << 1);
    /// Generic source
    pub const GENERIC_SRC: Self = Self(1 << 2);
    /// Generic destination
    pub const GENERIC_DST: Self = Self(1 << 3);

    /// Checks if feature is supported
    pub const fn supports(&self, feature: Self) -> bool {
        (self.0 & feature.0) != 0
    }
}

impl core::ops::BitOr for PeerMemoryFeatures {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// SLI/CrossFire mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MultiGpuMode {
    /// Single GPU
    Single,
    /// Alternate frame rendering
    AFR,
    /// Split frame rendering
    SFR,
    /// Linked device group
    LinkedDeviceGroup,
}

impl Default for MultiGpuMode {
    fn default() -> Self {
        Self::Single
    }
}

/// Multi-GPU configuration
#[derive(Clone, Debug, Default)]
pub struct MultiGpuConfig {
    /// Mode to use
    pub mode: MultiGpuMode,
    /// Devices to use
    pub device_mask: DeviceMask,
    /// Enable peer-to-peer transfers
    pub enable_p2p: bool,
}

impl MultiGpuConfig {
    /// Single GPU configuration
    pub const fn single() -> Self {
        Self {
            mode: MultiGpuMode::Single,
            device_mask: DeviceMask::DEVICE_0,
            enable_p2p: false,
        }
    }

    /// Alternate frame rendering
    pub fn afr(device_mask: DeviceMask) -> Self {
        Self {
            mode: MultiGpuMode::AFR,
            device_mask,
            enable_p2p: true,
        }
    }

    /// Split frame rendering
    pub fn sfr(device_mask: DeviceMask) -> Self {
        Self {
            mode: MultiGpuMode::SFR,
            device_mask,
            enable_p2p: true,
        }
    }
}

/// Frame pacing info for AFR
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AfrFrameInfo {
    /// Current frame index
    pub frame_index: u64,
    /// Device rendering this frame
    pub device_index: u32,
    /// Frame latency in frames
    pub latency_frames: u32,
}

impl AfrFrameInfo {
    /// Gets device for frame
    pub const fn device_for_frame(frame: u64, device_count: u32) -> u32 {
        (frame % device_count as u64) as u32
    }
}

/// Split frame rendering region
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SfrRegion {
    /// Device index
    pub device_index: u32,
    /// Region rectangle
    pub rect: DeviceRect,
}

impl SfrRegion {
    /// Creates new SFR region
    pub const fn new(device_index: u32, rect: DeviceRect) -> Self {
        Self { device_index, rect }
    }
}

/// Split frame rendering configuration
#[derive(Clone, Debug)]
pub struct SfrConfig {
    /// Regions for each device
    pub regions: Vec<SfrRegion>,
    /// Overlap region size (for seams)
    pub overlap: u32,
}

impl SfrConfig {
    /// Creates new SFR config
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            overlap: 0,
        }
    }

    /// Horizontal split between two devices
    pub fn horizontal_split(width: u32, height: u32) -> Self {
        let half_width = width / 2;
        Self {
            regions: alloc::vec![
                SfrRegion::new(0, DeviceRect::new(0, 0, half_width, height)),
                SfrRegion::new(1, DeviceRect::new(half_width as i32, 0, width - half_width, height)),
            ],
            overlap: 16,
        }
    }

    /// Vertical split between two devices
    pub fn vertical_split(width: u32, height: u32) -> Self {
        let half_height = height / 2;
        Self {
            regions: alloc::vec![
                SfrRegion::new(0, DeviceRect::new(0, 0, width, half_height)),
                SfrRegion::new(1, DeviceRect::new(0, half_height as i32, width, height - half_height)),
            ],
            overlap: 16,
        }
    }
}

impl Default for SfrConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Peer-to-peer copy info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct P2pCopyInfo {
    /// Source device index
    pub src_device: u32,
    /// Destination device index
    pub dst_device: u32,
    /// Source buffer/image
    pub src_handle: u64,
    /// Destination buffer/image
    pub dst_handle: u64,
    /// Size in bytes
    pub size: u64,
}

impl P2pCopyInfo {
    /// Creates new P2P copy info
    pub const fn new(src_device: u32, dst_device: u32, src: u64, dst: u64, size: u64) -> Self {
        Self {
            src_device,
            dst_device,
            src_handle: src,
            dst_handle: dst,
            size,
        }
    }
}

/// Device link type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DeviceLinkType {
    /// Unknown link
    Unknown,
    /// PCIe bus
    Pcie,
    /// NVLink
    NvLink,
    /// AMD Infinity Fabric
    InfinityFabric,
    /// CPU shared memory
    SharedMemory,
}

/// Device link info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DeviceLinkInfo {
    /// Source device
    pub src_device: u32,
    /// Destination device
    pub dst_device: u32,
    /// Link type
    pub link_type: u8,
    /// Link bandwidth in GB/s
    pub bandwidth_gbps: f32,
    /// Latency in nanoseconds
    pub latency_ns: u32,
}
