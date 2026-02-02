//! # Vulkan Device
//!
//! VkDevice and VkPhysicalDevice implementation for MAGMA.

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void, CStr};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::extensions::{DeviceExtensions, VkExtensionProperties, VkPhysicalDeviceFeatures};
use crate::instance::{PhysicalDeviceInfo, VkPhysicalDeviceType};
use crate::result::VkResult;
use crate::types::{VkDeviceHandle, VkExtent3D, VkPhysicalDeviceHandle, VK_API_VERSION_1_3};

// =============================================================================
// PHYSICAL DEVICE
// =============================================================================

/// Queue family properties
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkQueueFamilyProperties {
    /// Queue flags
    pub queue_flags: VkQueueFlags,
    /// Queue count
    pub queue_count: u32,
    /// Timestamp valid bits
    pub timestamp_valid_bits: u32,
    /// Min image transfer granularity
    pub min_image_transfer_granularity: VkExtent3D,
}

bitflags::bitflags! {
    /// Queue capability flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VkQueueFlags: u32 {
        /// Graphics queue
        const GRAPHICS = 1 << 0;
        /// Compute queue
        const COMPUTE = 1 << 1;
        /// Transfer queue
        const TRANSFER = 1 << 2;
        /// Sparse binding queue
        const SPARSE_BINDING = 1 << 3;
        /// Protected queue
        const PROTECTED = 1 << 4;
        /// Video decode queue
        const VIDEO_DECODE = 1 << 5;
        /// Video encode queue
        const VIDEO_ENCODE = 1 << 6;
    }
}

impl Default for VkQueueFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Memory type
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkMemoryType {
    /// Property flags
    pub property_flags: VkMemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

bitflags::bitflags! {
    /// Memory property flags
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct VkMemoryPropertyFlags: u32 {
        /// Device local memory
        const DEVICE_LOCAL = 1 << 0;
        /// Host visible memory
        const HOST_VISIBLE = 1 << 1;
        /// Host coherent memory
        const HOST_COHERENT = 1 << 2;
        /// Host cached memory
        const HOST_CACHED = 1 << 3;
        /// Lazily allocated memory
        const LAZILY_ALLOCATED = 1 << 4;
        /// Protected memory
        const PROTECTED = 1 << 5;
    }
}

/// Memory heap
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkMemoryHeap {
    /// Heap size in bytes
    pub size: u64,
    /// Heap flags
    pub flags: VkMemoryHeapFlags,
}

bitflags::bitflags! {
    /// Memory heap flags
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct VkMemoryHeapFlags: u32 {
        /// Device local heap
        const DEVICE_LOCAL = 1 << 0;
        /// Multi-instance heap
        const MULTI_INSTANCE = 1 << 1;
    }
}

/// Maximum memory types
pub const VK_MAX_MEMORY_TYPES: usize = 32;
/// Maximum memory heaps
pub const VK_MAX_MEMORY_HEAPS: usize = 16;

/// Physical device memory properties
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkPhysicalDeviceMemoryProperties {
    /// Memory type count
    pub memory_type_count: u32,
    /// Memory types
    pub memory_types: [VkMemoryType; VK_MAX_MEMORY_TYPES],
    /// Memory heap count
    pub memory_heap_count: u32,
    /// Memory heaps
    pub memory_heaps: [VkMemoryHeap; VK_MAX_MEMORY_HEAPS],
}

impl Default for VkPhysicalDeviceMemoryProperties {
    fn default() -> Self {
        Self {
            memory_type_count: 0,
            memory_types: [VkMemoryType::default(); VK_MAX_MEMORY_TYPES],
            memory_heap_count: 0,
            memory_heaps: [VkMemoryHeap::default(); VK_MAX_MEMORY_HEAPS],
        }
    }
}

/// Physical device properties
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceProperties {
    /// API version
    pub api_version: u32,
    /// Driver version
    pub driver_version: u32,
    /// Vendor ID
    pub vendor_id: u32,
    /// Device ID
    pub device_id: u32,
    /// Device type
    pub device_type: VkPhysicalDeviceType,
    /// Device name
    pub device_name: [u8; 256],
    /// Pipeline cache UUID
    pub pipeline_cache_uuid: [u8; 16],
    /// Device limits
    pub limits: VkPhysicalDeviceLimits,
    /// Sparse properties
    pub sparse_properties: VkPhysicalDeviceSparseProperties,
}

/// Physical device limits (partial - most important ones)
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkPhysicalDeviceLimits {
    /// Max image dimension 1D
    pub max_image_dimension1_d: u32,
    /// Max image dimension 2D
    pub max_image_dimension2_d: u32,
    /// Max image dimension 3D
    pub max_image_dimension3_d: u32,
    /// Max image dimension cube
    pub max_image_dimension_cube: u32,
    /// Max image array layers
    pub max_image_array_layers: u32,
    /// Max texel buffer elements
    pub max_texel_buffer_elements: u32,
    /// Max uniform buffer range
    pub max_uniform_buffer_range: u32,
    /// Max storage buffer range
    pub max_storage_buffer_range: u32,
    /// Max push constants size
    pub max_push_constants_size: u32,
    /// Max memory allocation count
    pub max_memory_allocation_count: u32,
    /// Max sampler allocation count
    pub max_sampler_allocation_count: u32,
    /// Buffer image granularity
    pub buffer_image_granularity: u64,
    /// Sparse address space size
    pub sparse_address_space_size: u64,
    /// Max bound descriptor sets
    pub max_bound_descriptor_sets: u32,
    /// Max per stage descriptor samplers
    pub max_per_stage_descriptor_samplers: u32,
    /// Max per stage descriptor uniform buffers
    pub max_per_stage_descriptor_uniform_buffers: u32,
    /// Max per stage descriptor storage buffers
    pub max_per_stage_descriptor_storage_buffers: u32,
    /// Max per stage descriptor sampled images
    pub max_per_stage_descriptor_sampled_images: u32,
    /// Max per stage descriptor storage images
    pub max_per_stage_descriptor_storage_images: u32,
    /// Max per stage descriptor input attachments
    pub max_per_stage_descriptor_input_attachments: u32,
    /// Max per stage resources
    pub max_per_stage_resources: u32,
    // ... more limits omitted for brevity
    /// Max compute shared memory size
    pub max_compute_shared_memory_size: u32,
    /// Max compute work group count
    pub max_compute_work_group_count: [u32; 3],
    /// Max compute work group invocations
    pub max_compute_work_group_invocations: u32,
    /// Max compute work group size
    pub max_compute_work_group_size: [u32; 3],
    /// Subpixel precision bits
    pub sub_pixel_precision_bits: u32,
    /// Subpixel interpolation offset bits
    pub sub_texel_precision_bits: u32,
    /// Mipmap precision bits
    pub mipmap_precision_bits: u32,
    /// Max draw indexed index value
    pub max_draw_indexed_index_value: u32,
    /// Max draw indirect count
    pub max_draw_indirect_count: u32,
    /// Max sampler lod bias
    pub max_sampler_lod_bias: f32,
    /// Max sampler anisotropy
    pub max_sampler_anisotropy: f32,
    /// Max viewports
    pub max_viewports: u32,
    /// Max viewport dimensions
    pub max_viewport_dimensions: [u32; 2],
    /// Viewport bounds range
    pub viewport_bounds_range: [f32; 2],
    /// Viewport subpixel bits
    pub viewport_sub_pixel_bits: u32,
    /// Min memory map alignment
    pub min_memory_map_alignment: usize,
    /// Min texel buffer offset alignment
    pub min_texel_buffer_offset_alignment: u64,
    /// Min uniform buffer offset alignment
    pub min_uniform_buffer_offset_alignment: u64,
    /// Min storage buffer offset alignment
    pub min_storage_buffer_offset_alignment: u64,
}

/// Physical device sparse properties
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkPhysicalDeviceSparseProperties {
    /// Residency standard 2D block shape
    pub residency_standard2_d_block_shape: u32,
    /// Residency standard 2D multisample block shape
    pub residency_standard2_d_multisample_block_shape: u32,
    /// Residency standard 3D block shape
    pub residency_standard3_d_block_shape: u32,
    /// Residency aligned mip size
    pub residency_aligned_mip_size: u32,
    /// Residency non-resident strict
    pub residency_non_resident_strict: u32,
}

/// Physical device wrapper for MAGMA
pub struct MagmaPhysicalDevice {
    /// Device info
    info: PhysicalDeviceInfo,
    /// Queue family properties
    queue_families: Vec<VkQueueFamilyProperties>,
    /// Memory properties
    memory_properties: VkPhysicalDeviceMemoryProperties,
    /// Supported features
    features: VkPhysicalDeviceFeatures,
}

impl MagmaPhysicalDevice {
    /// Create a new physical device
    pub fn new(info: PhysicalDeviceInfo) -> Self {
        // NVIDIA typical queue families:
        // 0: Graphics + Compute + Transfer
        // 1: Compute (async)
        // 2: Transfer (DMA engine)
        let queue_families = vec![
            VkQueueFamilyProperties {
                queue_flags: VkQueueFlags::GRAPHICS
                    | VkQueueFlags::COMPUTE
                    | VkQueueFlags::TRANSFER,
                queue_count: 16,
                timestamp_valid_bits: 64,
                min_image_transfer_granularity: VkExtent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
            },
            VkQueueFamilyProperties {
                queue_flags: VkQueueFlags::COMPUTE | VkQueueFlags::TRANSFER,
                queue_count: 8,
                timestamp_valid_bits: 64,
                min_image_transfer_granularity: VkExtent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
            },
            VkQueueFamilyProperties {
                queue_flags: VkQueueFlags::TRANSFER,
                queue_count: 2,
                timestamp_valid_bits: 64,
                min_image_transfer_granularity: VkExtent3D {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
            },
        ];

        // NVIDIA typical memory configuration
        let mut memory_properties = VkPhysicalDeviceMemoryProperties::default();

        // Heap 0: Device local (VRAM)
        memory_properties.memory_heaps[0] = VkMemoryHeap {
            size: 8 * 1024 * 1024 * 1024, // 8GB placeholder
            flags: VkMemoryHeapFlags::DEVICE_LOCAL,
        };

        // Heap 1: Host visible (system RAM)
        memory_properties.memory_heaps[1] = VkMemoryHeap {
            size: 16 * 1024 * 1024 * 1024, // 16GB placeholder
            flags: VkMemoryHeapFlags::empty(),
        };

        memory_properties.memory_heap_count = 2;

        // Memory types
        // Type 0: Device local only
        memory_properties.memory_types[0] = VkMemoryType {
            property_flags: VkMemoryPropertyFlags::DEVICE_LOCAL,
            heap_index: 0,
        };

        // Type 1: Host visible + coherent
        memory_properties.memory_types[1] = VkMemoryType {
            property_flags: VkMemoryPropertyFlags::HOST_VISIBLE
                | VkMemoryPropertyFlags::HOST_COHERENT,
            heap_index: 1,
        };

        // Type 2: Host visible + cached
        memory_properties.memory_types[2] = VkMemoryType {
            property_flags: VkMemoryPropertyFlags::HOST_VISIBLE
                | VkMemoryPropertyFlags::HOST_CACHED,
            heap_index: 1,
        };

        // Type 3: Device local + host visible (BAR)
        memory_properties.memory_types[3] = VkMemoryType {
            property_flags: VkMemoryPropertyFlags::DEVICE_LOCAL
                | VkMemoryPropertyFlags::HOST_VISIBLE
                | VkMemoryPropertyFlags::HOST_COHERENT,
            heap_index: 0,
        };

        memory_properties.memory_type_count = 4;

        // Enable most features by default
        let features = VkPhysicalDeviceFeatures::default();

        Self {
            info,
            queue_families,
            memory_properties,
            features,
        }
    }

    /// Get device properties
    pub fn get_properties(&self) -> VkPhysicalDeviceProperties {
        VkPhysicalDeviceProperties {
            api_version: VK_API_VERSION_1_3,
            driver_version: self.info.driver_version,
            vendor_id: self.info.vendor_id,
            device_id: self.info.device_id,
            device_type: self.info.device_type,
            device_name: self.info.device_name,
            pipeline_cache_uuid: self.info.pipeline_cache_uuid,
            limits: Self::get_nvidia_limits(),
            sparse_properties: VkPhysicalDeviceSparseProperties::default(),
        }
    }

    /// Get queue family properties
    pub fn get_queue_family_properties(&self) -> &[VkQueueFamilyProperties] {
        &self.queue_families
    }

    /// Get memory properties
    pub fn get_memory_properties(&self) -> &VkPhysicalDeviceMemoryProperties {
        &self.memory_properties
    }

    /// Get supported features
    pub fn get_features(&self) -> &VkPhysicalDeviceFeatures {
        &self.features
    }

    /// Get NVIDIA typical device limits
    fn get_nvidia_limits() -> VkPhysicalDeviceLimits {
        VkPhysicalDeviceLimits {
            max_image_dimension1_d: 32768,
            max_image_dimension2_d: 32768,
            max_image_dimension3_d: 16384,
            max_image_dimension_cube: 32768,
            max_image_array_layers: 2048,
            max_texel_buffer_elements: 128 * 1024 * 1024,
            max_uniform_buffer_range: 65536,
            max_storage_buffer_range: u32::MAX,
            max_push_constants_size: 256,
            max_memory_allocation_count: 4096,
            max_sampler_allocation_count: 4000,
            buffer_image_granularity: 1024,
            sparse_address_space_size: 1 << 40,
            max_bound_descriptor_sets: 32,
            max_per_stage_descriptor_samplers: 1024,
            max_per_stage_descriptor_uniform_buffers: 15,
            max_per_stage_descriptor_storage_buffers: 1024,
            max_per_stage_descriptor_sampled_images: 1024,
            max_per_stage_descriptor_storage_images: 1024,
            max_per_stage_descriptor_input_attachments: 1024,
            max_per_stage_resources: 4096,
            max_compute_shared_memory_size: 49152, // 48KB
            max_compute_work_group_count: [2147483647, 65535, 65535],
            max_compute_work_group_invocations: 1024,
            max_compute_work_group_size: [1024, 1024, 64],
            sub_pixel_precision_bits: 8,
            sub_texel_precision_bits: 8,
            mipmap_precision_bits: 8,
            max_draw_indexed_index_value: u32::MAX - 1,
            max_draw_indirect_count: u32::MAX - 1,
            max_sampler_lod_bias: 15.0,
            max_sampler_anisotropy: 16.0,
            max_viewports: 16,
            max_viewport_dimensions: [32768, 32768],
            viewport_bounds_range: [-65536.0, 65536.0],
            viewport_sub_pixel_bits: 8,
            min_memory_map_alignment: 64,
            min_texel_buffer_offset_alignment: 16,
            min_uniform_buffer_offset_alignment: 64,
            min_storage_buffer_offset_alignment: 16,
        }
    }

    /// To handle
    pub fn to_handle(&self) -> VkPhysicalDeviceHandle {
        self as *const _ as VkPhysicalDeviceHandle
    }
}

// =============================================================================
// LOGICAL DEVICE
// =============================================================================

/// Next device ID
static NEXT_DEVICE_ID: AtomicU32 = AtomicU32::new(1);

/// Device create info
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDeviceCreateInfo {
    /// Structure type
    pub s_type: u32,
    /// Next structure
    pub p_next: *const c_void,
    /// Flags
    pub flags: u32,
    /// Queue create info count
    pub queue_create_info_count: u32,
    /// Queue create infos
    pub p_queue_create_infos: *const VkDeviceQueueCreateInfo,
    /// Enabled layer count (deprecated)
    pub enabled_layer_count: u32,
    /// Enabled layer names (deprecated)
    pub pp_enabled_layer_names: *const *const c_char,
    /// Enabled extension count
    pub enabled_extension_count: u32,
    /// Enabled extension names
    pub pp_enabled_extension_names: *const *const c_char,
    /// Enabled features
    pub p_enabled_features: *const VkPhysicalDeviceFeatures,
}

/// Queue create info
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDeviceQueueCreateInfo {
    /// Structure type
    pub s_type: u32,
    /// Next structure
    pub p_next: *const c_void,
    /// Flags
    pub flags: u32,
    /// Queue family index
    pub queue_family_index: u32,
    /// Queue count
    pub queue_count: u32,
    /// Queue priorities
    pub p_queue_priorities: *const f32,
}

/// Logical device
pub struct MagmaDevice {
    /// Device ID
    id: u32,
    /// Physical device handle
    physical_device: VkPhysicalDeviceHandle,
    /// Enabled extensions
    enabled_extensions: DeviceExtensions,
    /// Queue families
    queue_families: Vec<QueueFamily>,
}

/// Queue family in device
struct QueueFamily {
    /// Family index
    family_index: u32,
    /// Queues
    queues: Vec<MagmaQueue>,
}

/// GPU queue
pub struct MagmaQueue {
    /// Queue family index
    family_index: u32,
    /// Queue index
    queue_index: u32,
    /// Priority
    priority: f32,
}

impl MagmaDevice {
    /// Create a new logical device
    ///
    /// # Safety
    /// - All pointers must be valid
    pub unsafe fn create(
        physical_device: VkPhysicalDeviceHandle,
        create_info: *const VkDeviceCreateInfo,
    ) -> Result<Box<Self>, VkResult> {
        if physical_device.is_null() || create_info.is_null() {
            return Err(VkResult::ErrorInitializationFailed);
        }

        let info = unsafe { &*create_info };

        // Parse enabled extensions
        let mut enabled_extensions = DeviceExtensions::empty();
        if info.enabled_extension_count > 0 && !info.pp_enabled_extension_names.is_null() {
            for i in 0..info.enabled_extension_count {
                let ext_name_ptr = unsafe { *info.pp_enabled_extension_names.add(i as usize) };
                if !ext_name_ptr.is_null() {
                    let ext_name = unsafe { CStr::from_ptr(ext_name_ptr) };
                    match ext_name.to_bytes() {
                        b"VK_KHR_swapchain" => {
                            enabled_extensions |= DeviceExtensions::SWAPCHAIN;
                        }
                        b"VK_KHR_dynamic_rendering" => {
                            enabled_extensions |= DeviceExtensions::DYNAMIC_RENDERING;
                        }
                        b"VK_KHR_synchronization2" => {
                            enabled_extensions |= DeviceExtensions::SYNCHRONIZATION_2;
                        }
                        b"VK_KHR_timeline_semaphore" => {
                            enabled_extensions |= DeviceExtensions::TIMELINE_SEMAPHORE;
                        }
                        b"VK_KHR_buffer_device_address" => {
                            enabled_extensions |= DeviceExtensions::BUFFER_DEVICE_ADDRESS;
                        }
                        _ => return Err(VkResult::ErrorExtensionNotPresent),
                    }
                }
            }
        }

        // Create queues
        let mut queue_families = Vec::new();
        if info.queue_create_info_count > 0 && !info.p_queue_create_infos.is_null() {
            for i in 0..info.queue_create_info_count {
                let queue_info = unsafe { &*info.p_queue_create_infos.add(i as usize) };
                let mut queues = Vec::new();

                for q in 0..queue_info.queue_count {
                    let priority = if !queue_info.p_queue_priorities.is_null() {
                        unsafe { *queue_info.p_queue_priorities.add(q as usize) }
                    } else {
                        1.0
                    };

                    queues.push(MagmaQueue {
                        family_index: queue_info.queue_family_index,
                        queue_index: q,
                        priority,
                    });
                }

                queue_families.push(QueueFamily {
                    family_index: queue_info.queue_family_index,
                    queues,
                });
            }
        }

        let device = Box::new(Self {
            id: NEXT_DEVICE_ID.fetch_add(1, Ordering::Relaxed),
            physical_device,
            enabled_extensions,
            queue_families,
        });

        Ok(device)
    }

    /// Get device ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get physical device
    pub fn physical_device(&self) -> VkPhysicalDeviceHandle {
        self.physical_device
    }

    /// Get enabled extensions
    pub fn enabled_extensions(&self) -> DeviceExtensions {
        self.enabled_extensions
    }

    /// To handle
    pub fn to_handle(&self) -> VkDeviceHandle {
        self as *const _ as VkDeviceHandle
    }

    /// From handle
    ///
    /// # Safety
    /// - Handle must be valid
    pub unsafe fn from_handle(handle: VkDeviceHandle) -> Option<&'static Self> {
        if handle.is_null() {
            None
        } else {
            Some(unsafe { &*(handle as *const Self) })
        }
    }
}

impl Drop for MagmaDevice {
    fn drop(&mut self) {
        log::debug!("Destroying MagmaDevice {}", self.id);
    }
}

// =============================================================================
// DEVICE FUNCTIONS
// =============================================================================

/// vkCreateDevice implementation
///
/// # Safety
/// - All pointers must be valid
pub unsafe fn create_device(
    physical_device: VkPhysicalDeviceHandle,
    p_create_info: *const VkDeviceCreateInfo,
    _p_allocator: *const c_void,
    p_device: *mut VkDeviceHandle,
) -> VkResult {
    if p_device.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    match unsafe { MagmaDevice::create(physical_device, p_create_info) } {
        Ok(device) => {
            let handle = device.to_handle();
            core::mem::forget(device);
            unsafe {
                *p_device = handle;
            }
            VkResult::Success
        }
        Err(e) => e,
    }
}

/// vkDestroyDevice implementation
///
/// # Safety
/// - Handle must be valid or null
pub unsafe fn destroy_device(device: VkDeviceHandle, _p_allocator: *const c_void) {
    if !device.is_null() {
        let _ = unsafe { Box::from_raw(device as *mut MagmaDevice) };
    }
}

/// vkEnumerateDeviceExtensionProperties implementation
///
/// # Safety
/// - All pointers must be valid
pub unsafe fn enumerate_device_extension_properties(
    _physical_device: VkPhysicalDeviceHandle,
    p_layer_name: *const c_char,
    p_property_count: *mut u32,
    p_properties: *mut VkExtensionProperties,
) -> VkResult {
    if p_property_count.is_null() {
        return VkResult::ErrorInitializationFailed;
    }

    // If layer is specified, return no extensions
    if !p_layer_name.is_null() {
        let layer = unsafe { CStr::from_ptr(p_layer_name) };
        if !layer.to_bytes().is_empty() {
            unsafe {
                *p_property_count = 0;
            }
            return VkResult::Success;
        }
    }

    let extensions = crate::extensions::get_device_extensions();
    let count = extensions.len() as u32;

    if p_properties.is_null() {
        unsafe {
            *p_property_count = count;
        }
        return VkResult::Success;
    }

    let available = unsafe { *p_property_count };
    let to_copy = available.min(count) as usize;

    unsafe {
        core::ptr::copy_nonoverlapping(extensions.as_ptr(), p_properties, to_copy);
        *p_property_count = to_copy as u32;
    }

    if available < count {
        VkResult::Incomplete
    } else {
        VkResult::Success
    }
}

/// vkDeviceWaitIdle implementation
///
/// # Safety
/// - Handle must be valid
pub unsafe fn device_wait_idle(_device: VkDeviceHandle) -> VkResult {
    // TODO: Implement actual wait
    VkResult::Success
}
