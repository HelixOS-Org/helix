//! # Resource Inspector
//!
//! Deep inspection of GPU resources with visualization and analysis.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Buffer,
    Image,
    ImageView,
    Sampler,
    AccelerationStructure,
    Pipeline,
    PipelineLayout,
    DescriptorSet,
    DescriptorSetLayout,
    RenderPass,
    Framebuffer,
    CommandBuffer,
    Fence,
    Semaphore,
    Event,
    QueryPool,
    ShaderModule,
}

/// Resource tracker
pub struct ResourceTracker {
    resources: BTreeMap<u64, TrackedResource>,
    next_id: u64,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            next_id: 1,
        }
    }

    /// Track a new resource
    pub fn track(&mut self, info: ResourceInfo) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.resources.insert(id, TrackedResource {
            id,
            info,
            access_history: Vec::new(),
            creation_frame: 0,
            last_access_frame: 0,
            access_count: 0,
        });

        id
    }

    /// Untrack a resource
    pub fn untrack(&mut self, id: u64) {
        self.resources.remove(&id);
    }

    /// Get resource info
    pub fn get(&self, id: u64) -> Option<ResourceInfo> {
        self.resources.get(&id).map(|r| r.info.clone())
    }

    /// Record resource access
    pub fn record_access(&mut self, id: u64, access: ResourceAccess) {
        if let Some(resource) = self.resources.get_mut(&id) {
            resource.access_history.push(access);
            resource.access_count += 1;
            resource.last_access_frame = access.frame;
        }
    }

    /// Get snapshot of all resources
    pub fn snapshot(&self) -> ResourceSnapshot {
        ResourceSnapshot {
            resources: self
                .resources
                .values()
                .map(|r| (r.id, r.info.clone()))
                .collect(),
            total_count: self.resources.len(),
            by_type: count_by_type(&self.resources),
        }
    }

    /// Find unused resources
    pub fn find_unused(&self, current_frame: u64, threshold: u64) -> Vec<u64> {
        self.resources
            .values()
            .filter(|r| current_frame - r.last_access_frame > threshold)
            .map(|r| r.id)
            .collect()
    }

    /// Get resources by type
    pub fn get_by_type(&self, resource_type: ResourceType) -> Vec<u64> {
        self.resources
            .values()
            .filter(|r| r.info.resource_type == resource_type)
            .map(|r| r.id)
            .collect()
    }
}

impl Default for ResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}

fn count_by_type(resources: &BTreeMap<u64, TrackedResource>) -> BTreeMap<ResourceType, usize> {
    let mut counts = BTreeMap::new();
    for resource in resources.values() {
        *counts.entry(resource.info.resource_type).or_insert(0) += 1;
    }
    counts
}

/// Tracked resource
struct TrackedResource {
    id: u64,
    info: ResourceInfo,
    access_history: Vec<ResourceAccess>,
    creation_frame: u64,
    last_access_frame: u64,
    access_count: u64,
}

/// Resource information
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub resource_type: ResourceType,
    pub name: Option<String>,
    pub size: u64,
    pub details: ResourceDetails,
}

/// Resource-specific details
#[derive(Debug, Clone)]
pub enum ResourceDetails {
    Buffer(BufferDetails),
    Image(ImageDetails),
    ImageView(ImageViewDetails),
    Sampler(SamplerDetails),
    Pipeline(PipelineDetails),
    DescriptorSet(DescriptorSetDetails),
    Other,
}

/// Buffer details
#[derive(Debug, Clone)]
pub struct BufferDetails {
    pub size: u64,
    pub usage: BufferUsage,
    pub memory_type: MemoryType,
    pub mapped: bool,
}

/// Buffer usage flags
#[derive(Debug, Clone, Copy)]
pub struct BufferUsage {
    pub vertex: bool,
    pub index: bool,
    pub uniform: bool,
    pub storage: bool,
    pub indirect: bool,
    pub transfer_src: bool,
    pub transfer_dst: bool,
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    DeviceLocal,
    HostVisible,
    HostCached,
    LazilyAllocated,
}

/// Image details
#[derive(Debug, Clone)]
pub struct ImageDetails {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: u32,
    pub usage: ImageUsage,
    pub layout: ImageLayout,
}

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageFormat {
    pub format: u32,
    pub channels: u8,
    pub bits_per_channel: u8,
    pub is_compressed: bool,
    pub is_depth: bool,
    pub is_stencil: bool,
}

/// Image usage flags
#[derive(Debug, Clone, Copy)]
pub struct ImageUsage {
    pub sampled: bool,
    pub storage: bool,
    pub color_attachment: bool,
    pub depth_attachment: bool,
    pub input_attachment: bool,
    pub transfer_src: bool,
    pub transfer_dst: bool,
}

/// Image layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageLayout {
    Undefined,
    General,
    ColorAttachmentOptimal,
    DepthStencilAttachmentOptimal,
    DepthStencilReadOnlyOptimal,
    ShaderReadOnlyOptimal,
    TransferSrcOptimal,
    TransferDstOptimal,
    PresentSrc,
}

/// Image view details
#[derive(Debug, Clone)]
pub struct ImageViewDetails {
    pub image_id: u64,
    pub view_type: ImageViewType,
    pub format: ImageFormat,
    pub base_mip_level: u32,
    pub level_count: u32,
    pub base_array_layer: u32,
    pub layer_count: u32,
}

/// Image view type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageViewType {
    View1D,
    View2D,
    View3D,
    Cube,
    Array1D,
    Array2D,
    CubeArray,
}

/// Sampler details
#[derive(Debug, Clone)]
pub struct SamplerDetails {
    pub mag_filter: Filter,
    pub min_filter: Filter,
    pub mipmap_mode: MipmapMode,
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub anisotropy: Option<f32>,
    pub compare_op: Option<CompareOp>,
    pub min_lod: f32,
    pub max_lod: f32,
}

/// Filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Linear,
}

/// Mipmap mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipmapMode {
    Nearest,
    Linear,
}

/// Address mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    Repeat,
    MirroredRepeat,
    ClampToEdge,
    ClampToBorder,
    MirrorClampToEdge,
}

/// Compare operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

/// Pipeline details
#[derive(Debug, Clone)]
pub struct PipelineDetails {
    pub pipeline_type: PipelineType,
    pub stages: Vec<ShaderStageInfo>,
    pub layout_id: u64,
}

/// Pipeline type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineType {
    Graphics,
    Compute,
    RayTracing,
}

/// Shader stage info
#[derive(Debug, Clone)]
pub struct ShaderStageInfo {
    pub stage: ShaderStage,
    pub module_id: u64,
    pub entry_point: String,
}

/// Shader stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    Compute,
    Task,
    Mesh,
    RayGeneration,
    AnyHit,
    ClosestHit,
    Miss,
    Intersection,
    Callable,
}

/// Descriptor set details
#[derive(Debug, Clone)]
pub struct DescriptorSetDetails {
    pub layout_id: u64,
    pub bindings: Vec<DescriptorBinding>,
}

/// Descriptor binding
#[derive(Debug, Clone)]
pub struct DescriptorBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub count: u32,
    pub resources: Vec<u64>,
}

/// Descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    InputAttachment,
    AccelerationStructure,
}

/// Resource access record
#[derive(Debug, Clone)]
pub struct ResourceAccess {
    pub frame: u64,
    pub command_buffer: u64,
    pub access_type: AccessType,
    pub stage: u64,
    pub timestamp: u64,
}

/// Access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    ReadWrite,
}

/// Resource snapshot
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub resources: BTreeMap<u64, ResourceInfo>,
    pub total_count: usize,
    pub by_type: BTreeMap<ResourceType, usize>,
}

impl Default for ResourceSnapshot {
    fn default() -> Self {
        Self {
            resources: BTreeMap::new(),
            total_count: 0,
            by_type: BTreeMap::new(),
        }
    }
}

/// Resource visualization helpers
pub struct ResourceVisualizer;

impl ResourceVisualizer {
    /// Generate texture preview data (RGBA8)
    pub fn preview_image(
        _image_id: u64,
        _mip_level: u32,
        _array_layer: u32,
    ) -> Option<ImagePreview> {
        // Would require GPU readback
        None
    }

    /// Format buffer contents as text
    pub fn format_buffer(
        data: &[u8],
        format: BufferFormat,
        offset: usize,
        count: usize,
    ) -> Vec<FormattedValue> {
        let mut values = Vec::new();
        let stride = format.stride();

        for i in 0..count {
            let start = offset + i * stride;
            if start + stride > data.len() {
                break;
            }

            let value = format_value(&data[start..start + stride], &format);
            values.push(value);
        }

        values
    }
}

/// Image preview data
#[derive(Debug, Clone)]
pub struct ImagePreview {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGBA8
}

/// Buffer format for visualization
#[derive(Debug, Clone)]
pub struct BufferFormat {
    pub elements: Vec<BufferElement>,
}

impl BufferFormat {
    pub fn stride(&self) -> usize {
        self.elements.iter().map(|e| e.size()).sum()
    }
}

/// Buffer element
#[derive(Debug, Clone)]
pub struct BufferElement {
    pub name: String,
    pub element_type: ElementType,
}

impl BufferElement {
    pub fn size(&self) -> usize {
        match self.element_type {
            ElementType::Float => 4,
            ElementType::Float2 => 8,
            ElementType::Float3 => 12,
            ElementType::Float4 => 16,
            ElementType::Int => 4,
            ElementType::Int2 => 8,
            ElementType::Int3 => 12,
            ElementType::Int4 => 16,
            ElementType::Uint => 4,
            ElementType::Uint2 => 8,
            ElementType::Uint3 => 12,
            ElementType::Uint4 => 16,
            ElementType::Mat3 => 36,
            ElementType::Mat4 => 64,
        }
    }
}

/// Element type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Float,
    Float2,
    Float3,
    Float4,
    Int,
    Int2,
    Int3,
    Int4,
    Uint,
    Uint2,
    Uint3,
    Uint4,
    Mat3,
    Mat4,
}

/// Formatted value for display
#[derive(Debug, Clone)]
pub struct FormattedValue {
    pub index: usize,
    pub fields: Vec<(String, String)>,
}

fn format_value(data: &[u8], format: &BufferFormat) -> FormattedValue {
    let mut fields = Vec::new();
    let mut offset = 0;

    for element in &format.elements {
        let value_str = match element.element_type {
            ElementType::Float => {
                let v = f32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                alloc::format!("{:.6}", v)
            },
            ElementType::Float2 => {
                let x = f32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                let y = f32::from_le_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);
                alloc::format!("({:.4}, {:.4})", x, y)
            },
            ElementType::Float3 => {
                let x = f32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                let y = f32::from_le_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);
                let z = f32::from_le_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]);
                alloc::format!("({:.4}, {:.4}, {:.4})", x, y, z)
            },
            ElementType::Float4 => {
                let x = f32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                let y = f32::from_le_bytes([
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ]);
                let z = f32::from_le_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]);
                let w = f32::from_le_bytes([
                    data[offset + 12],
                    data[offset + 13],
                    data[offset + 14],
                    data[offset + 15],
                ]);
                alloc::format!("({:.4}, {:.4}, {:.4}, {:.4})", x, y, z, w)
            },
            ElementType::Int | ElementType::Uint => {
                let v = i32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                alloc::format!("{}", v)
            },
            _ => String::from("..."),
        };

        fields.push((element.name.clone(), value_str));
        offset += element.size();
    }

    FormattedValue { index: 0, fields }
}
