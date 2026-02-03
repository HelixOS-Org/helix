//! Pipeline Layout and Push Constants
//!
//! This module provides pipeline layout management including:
//! - Pipeline layout creation and caching
//! - Push constant ranges
//! - Descriptor set binding frequencies
//! - Root signature management

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::hash::{Hash, Hasher};

use crate::descriptor::DescriptorSetLayout;
use crate::shader::ShaderStageFlags;

// ============================================================================
// Binding Frequency
// ============================================================================

/// Descriptor set binding frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BindingFrequency {
    /// Per-frame bindings (set 0).
    PerFrame,
    /// Per-view bindings (set 1).
    PerView,
    /// Per-material bindings (set 2).
    PerMaterial,
    /// Per-draw bindings (set 3).
    #[default]
    PerDraw,
    /// Custom set index.
    Custom(u32),
}

impl BindingFrequency {
    /// Get the set index.
    pub fn set_index(&self) -> u32 {
        match self {
            Self::PerFrame => 0,
            Self::PerView => 1,
            Self::PerMaterial => 2,
            Self::PerDraw => 3,
            Self::Custom(index) => *index,
        }
    }
}

// ============================================================================
// Push Constant Range
// ============================================================================

/// Push constant range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushConstantRange {
    /// Shader stages.
    pub stages: ShaderStageFlags,
    /// Offset in bytes.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
}

impl PushConstantRange {
    /// Create a new push constant range.
    pub fn new(stages: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self {
            stages,
            offset,
            size,
        }
    }

    /// Create for vertex shader.
    pub fn vertex(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::VERTEX, offset, size)
    }

    /// Create for fragment shader.
    pub fn fragment(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::FRAGMENT, offset, size)
    }

    /// Create for compute shader.
    pub fn compute(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::COMPUTE, offset, size)
    }

    /// Create for all graphics stages.
    pub fn all_graphics(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL_GRAPHICS, offset, size)
    }

    /// Create for all stages.
    pub fn all(offset: u32, size: u32) -> Self {
        Self::new(ShaderStageFlags::ALL, offset, size)
    }

    /// Get end offset.
    pub fn end_offset(&self) -> u32 {
        self.offset + self.size
    }

    /// Check if ranges overlap.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.offset < other.end_offset() && other.offset < self.end_offset()
    }

    /// Merge with another range.
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if !self.overlaps(other) && self.end_offset() != other.offset && other.end_offset() != self.offset {
            return None;
        }
        let stages = self.stages.or(other.stages);
        let offset = self.offset.min(other.offset);
        let end = self.end_offset().max(other.end_offset());
        Some(Self::new(stages, offset, end - offset))
    }
}

impl Hash for PushConstantRange {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.offset.hash(state);
        self.size.hash(state);
    }
}

// ============================================================================
// Pipeline Layout
// ============================================================================

/// Pipeline layout.
#[derive(Clone)]
pub struct PipelineLayout {
    /// Descriptor set layouts.
    set_layouts: Vec<Arc<DescriptorSetLayout>>,
    /// Push constant ranges.
    push_constant_ranges: Vec<PushConstantRange>,
    /// Debug name.
    name: String,
    /// Layout hash.
    hash: u64,
}

impl PipelineLayout {
    /// Create a new pipeline layout.
    pub fn new(
        set_layouts: Vec<Arc<DescriptorSetLayout>>,
        push_constant_ranges: Vec<PushConstantRange>,
    ) -> Self {
        let hash = Self::compute_hash(&set_layouts, &push_constant_ranges);
        Self {
            set_layouts,
            push_constant_ranges,
            name: String::new(),
            hash,
        }
    }

    /// Set debug name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Get set layouts.
    pub fn set_layouts(&self) -> &[Arc<DescriptorSetLayout>] {
        &self.set_layouts
    }

    /// Get push constant ranges.
    pub fn push_constant_ranges(&self) -> &[PushConstantRange] {
        &self.push_constant_ranges
    }

    /// Get debug name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Get set layout at index.
    pub fn get_set_layout(&self, index: u32) -> Option<&Arc<DescriptorSetLayout>> {
        self.set_layouts.get(index as usize)
    }

    /// Get total push constant size.
    pub fn push_constant_size(&self) -> u32 {
        self.push_constant_ranges
            .iter()
            .map(|r| r.end_offset())
            .max()
            .unwrap_or(0)
    }

    /// Check if compatible with another layout (same set 0).
    pub fn is_compatible_with(&self, other: &PipelineLayout, set_index: u32) -> bool {
        let set_idx = set_index as usize;
        if set_idx >= self.set_layouts.len() || set_idx >= other.set_layouts.len() {
            return false;
        }
        self.set_layouts[set_idx].hash() == other.set_layouts[set_idx].hash()
    }

    /// Compute layout hash.
    fn compute_hash(
        set_layouts: &[Arc<DescriptorSetLayout>],
        push_constant_ranges: &[PushConstantRange],
    ) -> u64 {
        let mut hasher = FnvHasher::new();
        for layout in set_layouts {
            layout.hash().hash(&mut hasher);
        }
        for range in push_constant_ranges {
            range.hash(&mut hasher);
        }
        hasher.finish()
    }
}

// ============================================================================
// Pipeline Layout Builder
// ============================================================================

/// Builder for pipeline layouts.
pub struct PipelineLayoutBuilder {
    set_layouts: Vec<Arc<DescriptorSetLayout>>,
    push_constant_ranges: Vec<PushConstantRange>,
    name: String,
}

impl PipelineLayoutBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            set_layouts: Vec::new(),
            push_constant_ranges: Vec::new(),
            name: String::new(),
        }
    }

    /// Add a descriptor set layout.
    pub fn set_layout(mut self, layout: Arc<DescriptorSetLayout>) -> Self {
        self.set_layouts.push(layout);
        self
    }

    /// Add a descriptor set layout at a specific index.
    pub fn set_layout_at(mut self, index: u32, layout: Arc<DescriptorSetLayout>) -> Self {
        let idx = index as usize;
        while self.set_layouts.len() <= idx {
            // Add empty layouts
            self.set_layouts.push(Arc::new(DescriptorSetLayout::new(Vec::new())));
        }
        self.set_layouts[idx] = layout;
        self
    }

    /// Add a push constant range.
    pub fn push_constant(mut self, range: PushConstantRange) -> Self {
        self.push_constant_ranges.push(range);
        self
    }

    /// Add push constants for vertex stage.
    pub fn vertex_push_constant<T>(self) -> Self {
        self.push_constant(PushConstantRange::vertex(0, core::mem::size_of::<T>() as u32))
    }

    /// Add push constants for all graphics stages.
    pub fn graphics_push_constant<T>(self) -> Self {
        self.push_constant(PushConstantRange::all_graphics(0, core::mem::size_of::<T>() as u32))
    }

    /// Set debug name.
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Build the pipeline layout.
    pub fn build(self) -> PipelineLayout {
        PipelineLayout::new(self.set_layouts, self.push_constant_ranges)
            .with_name(&self.name)
    }
}

impl Default for PipelineLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Pipeline Layout Cache
// ============================================================================

/// Cache for pipeline layouts.
pub struct PipelineLayoutCache {
    /// Cached layouts.
    layouts: Vec<(u64, Arc<PipelineLayout>)>,
    /// Maximum cache size.
    max_size: usize,
}

impl PipelineLayoutCache {
    /// Create a new cache.
    pub fn new(max_size: usize) -> Self {
        Self {
            layouts: Vec::new(),
            max_size,
        }
    }

    /// Get or create a layout.
    pub fn get_or_create<F>(&mut self, hash: u64, create: F) -> Arc<PipelineLayout>
    where
        F: FnOnce() -> PipelineLayout,
    {
        if let Some((_, layout)) = self.layouts.iter().find(|(h, _)| *h == hash) {
            return layout.clone();
        }

        // Evict if at capacity
        if self.layouts.len() >= self.max_size {
            self.layouts.remove(0);
        }

        let layout = Arc::new(create());
        self.layouts.push((hash, layout.clone()));
        layout
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.layouts.clear();
    }

    /// Get cache size.
    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}

impl Default for PipelineLayoutCache {
    fn default() -> Self {
        Self::new(256)
    }
}

// ============================================================================
// Common Pipeline Layouts
// ============================================================================

/// Standard pipeline layout templates.
pub struct StandardLayouts;

impl StandardLayouts {
    /// Create a simple layout with one set and push constants.
    pub fn simple(set_layout: Arc<DescriptorSetLayout>, push_constant_size: u32) -> PipelineLayout {
        let ranges = if push_constant_size > 0 {
            alloc::vec![PushConstantRange::all_graphics(0, push_constant_size)]
        } else {
            Vec::new()
        };
        PipelineLayout::new(alloc::vec![set_layout], ranges)
    }

    /// Create a layout for PBR rendering.
    pub fn pbr(
        global_layout: Arc<DescriptorSetLayout>,
        material_layout: Arc<DescriptorSetLayout>,
        push_constant_size: u32,
    ) -> PipelineLayout {
        let ranges = if push_constant_size > 0 {
            alloc::vec![PushConstantRange::all_graphics(0, push_constant_size)]
        } else {
            Vec::new()
        };
        PipelineLayout::new(alloc::vec![global_layout, material_layout], ranges)
    }

    /// Create a compute layout.
    pub fn compute(set_layout: Arc<DescriptorSetLayout>, push_constant_size: u32) -> PipelineLayout {
        let ranges = if push_constant_size > 0 {
            alloc::vec![PushConstantRange::compute(0, push_constant_size)]
        } else {
            Vec::new()
        };
        PipelineLayout::new(alloc::vec![set_layout], ranges)
    }
}

// ============================================================================
// Root Signature (D3D12 style)
// ============================================================================

/// Root parameter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootParameterType {
    /// Descriptor table.
    DescriptorTable,
    /// 32-bit constants.
    Constants,
    /// Constant buffer view.
    Cbv,
    /// Shader resource view.
    Srv,
    /// Unordered access view.
    Uav,
}

/// Descriptor range.
#[derive(Debug, Clone)]
pub struct DescriptorRange {
    /// Range type.
    pub range_type: DescriptorRangeType,
    /// Number of descriptors.
    pub num_descriptors: u32,
    /// Base shader register.
    pub base_shader_register: u32,
    /// Register space.
    pub register_space: u32,
    /// Offset in descriptors from table start.
    pub offset_in_descriptors_from_table_start: u32,
}

/// Descriptor range type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorRangeType {
    /// Shader resource view.
    Srv,
    /// Unordered access view.
    Uav,
    /// Constant buffer view.
    Cbv,
    /// Sampler.
    Sampler,
}

/// Root parameter.
#[derive(Clone)]
pub struct RootParameter {
    /// Parameter type.
    pub parameter_type: RootParameterType,
    /// Shader visibility.
    pub shader_visibility: ShaderVisibility,
    /// Descriptor table ranges (for DescriptorTable type).
    pub descriptor_table: Option<Vec<DescriptorRange>>,
    /// Number of 32-bit values (for Constants type).
    pub num_32bit_values: u32,
    /// Shader register.
    pub shader_register: u32,
    /// Register space.
    pub register_space: u32,
}

impl RootParameter {
    /// Create a descriptor table parameter.
    pub fn descriptor_table(ranges: Vec<DescriptorRange>, visibility: ShaderVisibility) -> Self {
        Self {
            parameter_type: RootParameterType::DescriptorTable,
            shader_visibility: visibility,
            descriptor_table: Some(ranges),
            num_32bit_values: 0,
            shader_register: 0,
            register_space: 0,
        }
    }

    /// Create a 32-bit constants parameter.
    pub fn constants(
        shader_register: u32,
        register_space: u32,
        num_values: u32,
        visibility: ShaderVisibility,
    ) -> Self {
        Self {
            parameter_type: RootParameterType::Constants,
            shader_visibility: visibility,
            descriptor_table: None,
            num_32bit_values: num_values,
            shader_register,
            register_space,
        }
    }

    /// Create a CBV parameter.
    pub fn cbv(shader_register: u32, register_space: u32, visibility: ShaderVisibility) -> Self {
        Self {
            parameter_type: RootParameterType::Cbv,
            shader_visibility: visibility,
            descriptor_table: None,
            num_32bit_values: 0,
            shader_register,
            register_space,
        }
    }

    /// Create a SRV parameter.
    pub fn srv(shader_register: u32, register_space: u32, visibility: ShaderVisibility) -> Self {
        Self {
            parameter_type: RootParameterType::Srv,
            shader_visibility: visibility,
            descriptor_table: None,
            num_32bit_values: 0,
            shader_register,
            register_space,
        }
    }

    /// Create a UAV parameter.
    pub fn uav(shader_register: u32, register_space: u32, visibility: ShaderVisibility) -> Self {
        Self {
            parameter_type: RootParameterType::Uav,
            shader_visibility: visibility,
            descriptor_table: None,
            num_32bit_values: 0,
            shader_register,
            register_space,
        }
    }
}

/// Shader visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShaderVisibility {
    /// All stages.
    #[default]
    All,
    /// Vertex stage.
    Vertex,
    /// Hull stage.
    Hull,
    /// Domain stage.
    Domain,
    /// Geometry stage.
    Geometry,
    /// Pixel stage.
    Pixel,
    /// Amplification stage.
    Amplification,
    /// Mesh stage.
    Mesh,
}

/// Static sampler.
#[derive(Clone)]
pub struct StaticSampler {
    /// Filter.
    pub filter: SamplerFilter,
    /// Address mode U.
    pub address_u: SamplerAddressMode,
    /// Address mode V.
    pub address_v: SamplerAddressMode,
    /// Address mode W.
    pub address_w: SamplerAddressMode,
    /// Mip LOD bias.
    pub mip_lod_bias: f32,
    /// Max anisotropy.
    pub max_anisotropy: u32,
    /// Comparison function.
    pub comparison_func: ComparisonFunc,
    /// Border color.
    pub border_color: BorderColor,
    /// Min LOD.
    pub min_lod: f32,
    /// Max LOD.
    pub max_lod: f32,
    /// Shader register.
    pub shader_register: u32,
    /// Register space.
    pub register_space: u32,
    /// Shader visibility.
    pub shader_visibility: ShaderVisibility,
}

impl Default for StaticSampler {
    fn default() -> Self {
        Self {
            filter: SamplerFilter::MinMagMipLinear,
            address_u: SamplerAddressMode::Wrap,
            address_v: SamplerAddressMode::Wrap,
            address_w: SamplerAddressMode::Wrap,
            mip_lod_bias: 0.0,
            max_anisotropy: 1,
            comparison_func: ComparisonFunc::Never,
            border_color: BorderColor::TransparentBlack,
            min_lod: 0.0,
            max_lod: f32::MAX,
            shader_register: 0,
            register_space: 0,
            shader_visibility: ShaderVisibility::All,
        }
    }
}

/// Sampler filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamplerFilter {
    /// Point filtering.
    MinMagMipPoint,
    /// Linear min/mag, point mip.
    MinMagLinearMipPoint,
    /// Linear filtering.
    #[default]
    MinMagMipLinear,
    /// Anisotropic filtering.
    Anisotropic,
    /// Comparison point.
    ComparisonMinMagMipPoint,
    /// Comparison linear.
    ComparisonMinMagMipLinear,
    /// Comparison anisotropic.
    ComparisonAnisotropic,
}

/// Sampler address mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamplerAddressMode {
    /// Wrap.
    #[default]
    Wrap,
    /// Mirror.
    Mirror,
    /// Clamp.
    Clamp,
    /// Border.
    Border,
    /// Mirror once.
    MirrorOnce,
}

/// Comparison function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ComparisonFunc {
    /// Never pass.
    #[default]
    Never,
    /// Pass if less.
    Less,
    /// Pass if equal.
    Equal,
    /// Pass if less or equal.
    LessEqual,
    /// Pass if greater.
    Greater,
    /// Pass if not equal.
    NotEqual,
    /// Pass if greater or equal.
    GreaterEqual,
    /// Always pass.
    Always,
}

/// Border color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderColor {
    /// Transparent black.
    #[default]
    TransparentBlack,
    /// Opaque black.
    OpaqueBlack,
    /// Opaque white.
    OpaqueWhite,
}

/// Root signature flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RootSignatureFlags(u32);

impl RootSignatureFlags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Allow input assembler input layout.
    pub const ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT: Self = Self(1 << 0);
    /// Deny vertex shader root access.
    pub const DENY_VERTEX_SHADER_ROOT_ACCESS: Self = Self(1 << 1);
    /// Deny hull shader root access.
    pub const DENY_HULL_SHADER_ROOT_ACCESS: Self = Self(1 << 2);
    /// Deny domain shader root access.
    pub const DENY_DOMAIN_SHADER_ROOT_ACCESS: Self = Self(1 << 3);
    /// Deny geometry shader root access.
    pub const DENY_GEOMETRY_SHADER_ROOT_ACCESS: Self = Self(1 << 4);
    /// Deny pixel shader root access.
    pub const DENY_PIXEL_SHADER_ROOT_ACCESS: Self = Self(1 << 5);
    /// Allow stream output.
    pub const ALLOW_STREAM_OUTPUT: Self = Self(1 << 6);
    /// Local root signature.
    pub const LOCAL_ROOT_SIGNATURE: Self = Self(1 << 7);
    /// Deny amplification shader root access.
    pub const DENY_AMPLIFICATION_SHADER_ROOT_ACCESS: Self = Self(1 << 8);
    /// Deny mesh shader root access.
    pub const DENY_MESH_SHADER_ROOT_ACCESS: Self = Self(1 << 9);
    /// CBV/SRV/UAV heap directly indexed.
    pub const CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED: Self = Self(1 << 10);
    /// Sampler heap directly indexed.
    pub const SAMPLER_HEAP_DIRECTLY_INDEXED: Self = Self(1 << 11);

    /// Combine flags.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Root signature.
#[derive(Clone)]
pub struct RootSignature {
    /// Parameters.
    pub parameters: Vec<RootParameter>,
    /// Static samplers.
    pub static_samplers: Vec<StaticSampler>,
    /// Flags.
    pub flags: RootSignatureFlags,
}

impl RootSignature {
    /// Create a new root signature.
    pub fn new(
        parameters: Vec<RootParameter>,
        static_samplers: Vec<StaticSampler>,
        flags: RootSignatureFlags,
    ) -> Self {
        Self {
            parameters,
            static_samplers,
            flags,
        }
    }

    /// Convert to Vulkan pipeline layout.
    pub fn to_pipeline_layout(&self) -> PipelineLayout {
        // Convert root parameters to descriptor set layouts
        // This is a simplified conversion
        let set_layouts = Vec::new();
        let push_constant_ranges = Vec::new();
        
        PipelineLayout::new(set_layouts, push_constant_ranges)
    }
}

/// Builder for root signatures.
pub struct RootSignatureBuilder {
    parameters: Vec<RootParameter>,
    static_samplers: Vec<StaticSampler>,
    flags: RootSignatureFlags,
}

impl RootSignatureBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            static_samplers: Vec::new(),
            flags: RootSignatureFlags::NONE,
        }
    }

    /// Add a parameter.
    pub fn parameter(mut self, param: RootParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add a static sampler.
    pub fn static_sampler(mut self, sampler: StaticSampler) -> Self {
        self.static_samplers.push(sampler);
        self
    }

    /// Set flags.
    pub fn flags(mut self, flags: RootSignatureFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Build the root signature.
    pub fn build(self) -> RootSignature {
        RootSignature::new(self.parameters, self.static_samplers, self.flags)
    }
}

impl Default for RootSignatureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// FNV Hasher
// ============================================================================

/// FNV-1a hasher.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}
