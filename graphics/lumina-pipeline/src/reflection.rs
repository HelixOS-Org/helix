//! Shader Reflection
//!
//! This module provides comprehensive shader reflection utilities for
//! extracting metadata from SPIR-V shaders.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Resource Types
// ============================================================================

/// Resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer.
    StorageBuffer,
    /// Sampled image.
    SampledImage,
    /// Storage image.
    StorageImage,
    /// Combined image sampler.
    CombinedImageSampler,
    /// Sampler.
    Sampler,
    /// Input attachment.
    InputAttachment,
    /// Acceleration structure.
    AccelerationStructure,
    /// Push constant.
    PushConstant,
    /// Subpass input.
    SubpassInput,
    /// Uniform texel buffer.
    UniformTexelBuffer,
    /// Storage texel buffer.
    StorageTexelBuffer,
}

impl ResourceType {
    /// Check if this is a buffer type.
    pub fn is_buffer(&self) -> bool {
        matches!(
            self,
            Self::UniformBuffer | Self::StorageBuffer | Self::PushConstant
        )
    }

    /// Check if this is an image type.
    pub fn is_image(&self) -> bool {
        matches!(
            self,
            Self::SampledImage
                | Self::StorageImage
                | Self::CombinedImageSampler
                | Self::InputAttachment
                | Self::SubpassInput
        )
    }
}

// ============================================================================
// Type Reflection
// ============================================================================

/// Reflected scalar type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarType {
    /// Boolean.
    Bool,
    /// Signed integer.
    Int,
    /// Unsigned integer.
    Uint,
    /// Float.
    Float,
    /// Double.
    Double,
}

/// Reflected type kind.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    /// Scalar type.
    Scalar(ScalarType, u32), // type, bit width
    /// Vector type.
    Vector(ScalarType, u32, u32), // type, bit width, components
    /// Matrix type.
    Matrix(ScalarType, u32, u32, u32), // type, bit width, columns, rows
    /// Array type.
    Array {
        element_type: Box<TypeKind>,
        element_count: u32,
        stride: u32,
    },
    /// Runtime array.
    RuntimeArray {
        element_type: Box<TypeKind>,
        stride: u32,
    },
    /// Struct type.
    Struct(StructType),
    /// Image type.
    Image(ImageType),
    /// Sampler type.
    Sampler,
    /// Combined sampler.
    SampledImage(ImageType),
    /// Acceleration structure.
    AccelerationStructure,
}

impl TypeKind {
    /// Create a scalar type.
    pub fn scalar(ty: ScalarType, bits: u32) -> Self {
        Self::Scalar(ty, bits)
    }

    /// Create a vector type.
    pub fn vector(ty: ScalarType, bits: u32, components: u32) -> Self {
        Self::Vector(ty, bits, components)
    }

    /// Create a matrix type.
    pub fn matrix(ty: ScalarType, bits: u32, cols: u32, rows: u32) -> Self {
        Self::Matrix(ty, bits, cols, rows)
    }

    /// Create float scalar.
    pub fn float32() -> Self {
        Self::scalar(ScalarType::Float, 32)
    }

    /// Create vec2.
    pub fn vec2() -> Self {
        Self::vector(ScalarType::Float, 32, 2)
    }

    /// Create vec3.
    pub fn vec3() -> Self {
        Self::vector(ScalarType::Float, 32, 3)
    }

    /// Create vec4.
    pub fn vec4() -> Self {
        Self::vector(ScalarType::Float, 32, 4)
    }

    /// Create mat4.
    pub fn mat4() -> Self {
        Self::matrix(ScalarType::Float, 32, 4, 4)
    }

    /// Get the size in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Self::Scalar(_, bits) => bits / 8,
            Self::Vector(_, bits, components) => (bits / 8) * components,
            Self::Matrix(_, bits, cols, rows) => (bits / 8) * cols * rows,
            Self::Array {
                element_type,
                element_count,
                stride,
            } => {
                if *stride > 0 {
                    *stride * *element_count
                } else {
                    element_type.size() * *element_count
                }
            },
            Self::RuntimeArray { .. } => 0, // Unknown size
            Self::Struct(s) => s.size,
            Self::Image(_)
            | Self::Sampler
            | Self::SampledImage(_)
            | Self::AccelerationStructure => 0,
        }
    }
}

/// Reflected struct type.
#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    /// Struct name.
    pub name: String,
    /// Members.
    pub members: Vec<StructMember>,
    /// Total size in bytes.
    pub size: u32,
}

impl StructType {
    /// Create a new struct type.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            members: Vec::new(),
            size: 0,
        }
    }

    /// Add a member.
    pub fn add_member(&mut self, member: StructMember) {
        let end = member.offset + member.ty.size();
        if end > self.size {
            self.size = end;
        }
        self.members.push(member);
    }

    /// Get member by name.
    pub fn member(&self, name: &str) -> Option<&StructMember> {
        self.members.iter().find(|m| m.name == name)
    }

    /// Get member offset.
    pub fn member_offset(&self, name: &str) -> Option<u32> {
        self.member(name).map(|m| m.offset)
    }
}

/// Reflected struct member.
#[derive(Debug, Clone, PartialEq)]
pub struct StructMember {
    /// Member name.
    pub name: String,
    /// Member type.
    pub ty: TypeKind,
    /// Offset in bytes.
    pub offset: u32,
    /// Array stride (if array).
    pub array_stride: Option<u32>,
    /// Matrix stride (if matrix).
    pub matrix_stride: Option<u32>,
}

impl StructMember {
    /// Create a new struct member.
    pub fn new(name: impl Into<String>, ty: TypeKind, offset: u32) -> Self {
        Self {
            name: name.into(),
            ty,
            offset,
            array_stride: None,
            matrix_stride: None,
        }
    }
}

/// Reflected image type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageType {
    /// Image dimension.
    pub dim: ImageDimension,
    /// Depth flag.
    pub depth: ImageDepth,
    /// Arrayed flag.
    pub arrayed: bool,
    /// Multisampled flag.
    pub multisampled: bool,
    /// Sampled flag.
    pub sampled: ImageSampled,
    /// Image format.
    pub format: ImageFormat,
}

/// Image dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDimension {
    D1,
    D2,
    D3,
    Cube,
    Rect,
    Buffer,
    SubpassData,
}

/// Image depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDepth {
    NoDepth,
    Depth,
    Unknown,
}

/// Image sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageSampled {
    Runtime,
    Sampled,
    Storage,
}

/// Image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ImageFormat {
    Unknown,
    Rgba32f,
    Rgba16f,
    R32f,
    Rgba8,
    Rgba8Snorm,
    Rg32f,
    Rg16f,
    R11fG11fB10f,
    R16f,
    Rgba16,
    Rgb10A2,
    Rg16,
    Rg8,
    R16,
    R8,
    Rgba16Snorm,
    Rg16Snorm,
    Rg8Snorm,
    R16Snorm,
    R8Snorm,
    Rgba32i,
    Rgba16i,
    Rgba8i,
    R32i,
    Rg32i,
    Rg16i,
    Rg8i,
    R16i,
    R8i,
    Rgba32ui,
    Rgba16ui,
    Rgba8ui,
    R32ui,
    Rgb10a2ui,
    Rg32ui,
    Rg16ui,
    Rg8ui,
    R16ui,
    R8ui,
    R64ui,
    R64i,
}

// ============================================================================
// Resource Binding
// ============================================================================

/// Reflected resource binding.
#[derive(Debug, Clone)]
pub struct ReflectedBinding {
    /// Resource name.
    pub name: String,
    /// Descriptor set.
    pub set: u32,
    /// Binding index.
    pub binding: u32,
    /// Resource type.
    pub resource_type: ResourceType,
    /// Type information.
    pub type_info: TypeKind,
    /// Array count (0 = runtime).
    pub count: u32,
    /// Access flags.
    pub access: AccessFlags,
}

bitflags::bitflags! {
    /// Resource access flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u32 {
        /// Read access.
        const READ = 0x01;
        /// Write access.
        const WRITE = 0x02;
        /// Read-write access.
        const READ_WRITE = Self::READ.bits() | Self::WRITE.bits();
    }
}

impl Default for AccessFlags {
    fn default() -> Self {
        Self::READ
    }
}

impl ReflectedBinding {
    /// Check if this is a read-only binding.
    pub fn is_read_only(&self) -> bool {
        self.access == AccessFlags::READ
    }

    /// Check if this binding is writable.
    pub fn is_writable(&self) -> bool {
        self.access.contains(AccessFlags::WRITE)
    }

    /// Check if this is a buffer binding.
    pub fn is_buffer(&self) -> bool {
        self.resource_type.is_buffer()
    }

    /// Check if this is an image binding.
    pub fn is_image(&self) -> bool {
        self.resource_type.is_image()
    }
}

// ============================================================================
// Push Constants
// ============================================================================

/// Reflected push constant range.
#[derive(Debug, Clone)]
pub struct ReflectedPushConstant {
    /// Block name.
    pub name: String,
    /// Stage flags.
    pub stages: ShaderStageFlags,
    /// Offset in bytes.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
    /// Type information.
    pub type_info: StructType,
}

bitflags::bitflags! {
    /// Shader stage flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ShaderStageFlags: u32 {
        /// Vertex stage.
        const VERTEX = 0x0001;
        /// Fragment stage.
        const FRAGMENT = 0x0010;
        /// Compute stage.
        const COMPUTE = 0x0020;
        /// Geometry stage.
        const GEOMETRY = 0x0008;
        /// Tessellation control.
        const TESS_CONTROL = 0x0002;
        /// Tessellation evaluation.
        const TESS_EVAL = 0x0004;
        /// Ray generation.
        const RAY_GEN = 0x0100;
        /// Any hit.
        const ANY_HIT = 0x0200;
        /// Closest hit.
        const CLOSEST_HIT = 0x0400;
        /// Miss.
        const MISS = 0x0800;
        /// Intersection.
        const INTERSECTION = 0x1000;
        /// Callable.
        const CALLABLE = 0x2000;
        /// Task (mesh shading).
        const TASK = 0x0040;
        /// Mesh (mesh shading).
        const MESH = 0x0080;
        /// All graphics stages.
        const ALL_GRAPHICS = Self::VERTEX.bits() | Self::FRAGMENT.bits()
            | Self::GEOMETRY.bits() | Self::TESS_CONTROL.bits() | Self::TESS_EVAL.bits();
        /// All stages.
        const ALL = 0xFFFFFFFF;
    }
}

// ============================================================================
// Vertex Input
// ============================================================================

/// Reflected vertex input.
#[derive(Debug, Clone)]
pub struct ReflectedVertexInput {
    /// Input name.
    pub name: String,
    /// Location.
    pub location: u32,
    /// Type.
    pub ty: TypeKind,
    /// Semantic (if available).
    pub semantic: Option<String>,
}

impl ReflectedVertexInput {
    /// Get the format.
    pub fn format(&self) -> VertexInputFormat {
        match &self.ty {
            TypeKind::Scalar(ScalarType::Float, 32) => VertexInputFormat::Float,
            TypeKind::Vector(ScalarType::Float, 32, 2) => VertexInputFormat::Float2,
            TypeKind::Vector(ScalarType::Float, 32, 3) => VertexInputFormat::Float3,
            TypeKind::Vector(ScalarType::Float, 32, 4) => VertexInputFormat::Float4,
            TypeKind::Scalar(ScalarType::Int, 32) => VertexInputFormat::Int,
            TypeKind::Vector(ScalarType::Int, 32, 2) => VertexInputFormat::Int2,
            TypeKind::Vector(ScalarType::Int, 32, 3) => VertexInputFormat::Int3,
            TypeKind::Vector(ScalarType::Int, 32, 4) => VertexInputFormat::Int4,
            TypeKind::Scalar(ScalarType::Uint, 32) => VertexInputFormat::Uint,
            TypeKind::Vector(ScalarType::Uint, 32, 2) => VertexInputFormat::Uint2,
            TypeKind::Vector(ScalarType::Uint, 32, 3) => VertexInputFormat::Uint3,
            TypeKind::Vector(ScalarType::Uint, 32, 4) => VertexInputFormat::Uint4,
            _ => VertexInputFormat::Unknown,
        }
    }
}

/// Vertex input format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexInputFormat {
    Unknown,
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
}

impl VertexInputFormat {
    /// Get the size in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Self::Unknown => 0,
            Self::Float | Self::Int | Self::Uint => 4,
            Self::Float2 | Self::Int2 | Self::Uint2 => 8,
            Self::Float3 | Self::Int3 | Self::Uint3 => 12,
            Self::Float4 | Self::Int4 | Self::Uint4 => 16,
        }
    }
}

// ============================================================================
// Fragment Output
// ============================================================================

/// Reflected fragment output.
#[derive(Debug, Clone)]
pub struct ReflectedFragmentOutput {
    /// Output name.
    pub name: String,
    /// Location.
    pub location: u32,
    /// Index (for dual-source blending).
    pub index: u32,
    /// Type.
    pub ty: TypeKind,
}

// ============================================================================
// Compute Workgroup
// ============================================================================

/// Reflected compute workgroup size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkgroupSize {
    /// X dimension.
    pub x: u32,
    /// Y dimension.
    pub y: u32,
    /// Z dimension.
    pub z: u32,
    /// Whether dimensions use specialization constants.
    pub spec_constant_x: Option<u32>,
    pub spec_constant_y: Option<u32>,
    pub spec_constant_z: Option<u32>,
}

impl WorkgroupSize {
    /// Create a new workgroup size.
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            x,
            y,
            z,
            spec_constant_x: None,
            spec_constant_y: None,
            spec_constant_z: None,
        }
    }

    /// Get total thread count.
    pub fn total(&self) -> u32 {
        self.x * self.y * self.z
    }
}

impl Default for WorkgroupSize {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

// ============================================================================
// Shader Reflection Data
// ============================================================================

/// Complete shader reflection data.
#[derive(Debug, Clone, Default)]
pub struct ShaderReflection {
    /// Entry point name.
    pub entry_point: String,
    /// Shader stage.
    pub stage: ShaderStageFlags,
    /// Resource bindings.
    pub bindings: Vec<ReflectedBinding>,
    /// Push constants.
    pub push_constants: Vec<ReflectedPushConstant>,
    /// Vertex inputs (vertex shader only).
    pub vertex_inputs: Vec<ReflectedVertexInput>,
    /// Fragment outputs (fragment shader only).
    pub fragment_outputs: Vec<ReflectedFragmentOutput>,
    /// Workgroup size (compute shader only).
    pub workgroup_size: Option<WorkgroupSize>,
    /// Specialization constants used.
    pub specialization_constants: Vec<SpecializationInfo>,
}

/// Specialization constant info.
#[derive(Debug, Clone)]
pub struct SpecializationInfo {
    /// Constant ID.
    pub id: u32,
    /// Name.
    pub name: String,
    /// Type.
    pub ty: TypeKind,
    /// Default value (as bytes).
    pub default_value: Vec<u8>,
}

impl ShaderReflection {
    /// Create new empty reflection.
    pub fn new(entry_point: impl Into<String>, stage: ShaderStageFlags) -> Self {
        Self {
            entry_point: entry_point.into(),
            stage,
            ..Default::default()
        }
    }

    /// Get bindings for a specific set.
    pub fn bindings_for_set(&self, set: u32) -> impl Iterator<Item = &ReflectedBinding> {
        self.bindings.iter().filter(move |b| b.set == set)
    }

    /// Get binding by name.
    pub fn binding_by_name(&self, name: &str) -> Option<&ReflectedBinding> {
        self.bindings.iter().find(|b| b.name == name)
    }

    /// Get the number of descriptor sets used.
    pub fn descriptor_set_count(&self) -> u32 {
        self.bindings
            .iter()
            .map(|b| b.set)
            .max()
            .map(|s| s + 1)
            .unwrap_or(0)
    }

    /// Get total push constant size.
    pub fn push_constant_size(&self) -> u32 {
        self.push_constants
            .iter()
            .map(|pc| pc.offset + pc.size)
            .max()
            .unwrap_or(0)
    }

    /// Validate compatibility with another shader.
    pub fn validate_compatibility(&self, other: &ShaderReflection) -> Vec<ReflectionError> {
        let mut errors = Vec::new();

        // Check push constant compatibility
        for pc in &self.push_constants {
            if let Some(other_pc) = other.push_constants.iter().find(|p| p.name == pc.name) {
                if pc.size != other_pc.size {
                    errors.push(ReflectionError::PushConstantMismatch {
                        name: pc.name.clone(),
                        expected: pc.size,
                        found: other_pc.size,
                    });
                }
            }
        }

        // Check binding compatibility
        for binding in &self.bindings {
            if let Some(other_binding) = other
                .bindings
                .iter()
                .find(|b| b.set == binding.set && b.binding == binding.binding)
            {
                if binding.resource_type != other_binding.resource_type {
                    errors.push(ReflectionError::BindingTypeMismatch {
                        set: binding.set,
                        binding: binding.binding,
                        expected: binding.resource_type,
                        found: other_binding.resource_type,
                    });
                }
            }
        }

        errors
    }
}

/// Reflection error.
#[derive(Debug, Clone)]
pub enum ReflectionError {
    /// Push constant size mismatch.
    PushConstantMismatch {
        name: String,
        expected: u32,
        found: u32,
    },
    /// Binding type mismatch.
    BindingTypeMismatch {
        set: u32,
        binding: u32,
        expected: ResourceType,
        found: ResourceType,
    },
    /// Missing required binding.
    MissingBinding { set: u32, binding: u32 },
}

// ============================================================================
// Program Reflection
// ============================================================================

/// Combined reflection for a complete program.
#[derive(Debug, Clone, Default)]
pub struct ProgramReflection {
    /// Stages.
    pub stages: BTreeMap<ShaderStageFlags, ShaderReflection>,
    /// Merged bindings.
    pub bindings: Vec<ReflectedBinding>,
    /// Merged push constants.
    pub push_constants: Vec<ReflectedPushConstant>,
    /// Vertex inputs.
    pub vertex_inputs: Vec<ReflectedVertexInput>,
    /// Fragment outputs.
    pub fragment_outputs: Vec<ReflectedFragmentOutput>,
}

impl ProgramReflection {
    /// Create new program reflection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a shader stage.
    pub fn add_stage(&mut self, reflection: ShaderReflection) {
        let stage = reflection.stage;

        // Merge vertex inputs
        if stage == ShaderStageFlags::VERTEX {
            self.vertex_inputs = reflection.vertex_inputs.clone();
        }

        // Merge fragment outputs
        if stage == ShaderStageFlags::FRAGMENT {
            self.fragment_outputs = reflection.fragment_outputs.clone();
        }

        // Merge bindings
        for binding in &reflection.bindings {
            if !self
                .bindings
                .iter()
                .any(|b| b.set == binding.set && b.binding == binding.binding)
            {
                self.bindings.push(binding.clone());
            }
        }

        // Merge push constants
        for pc in &reflection.push_constants {
            if !self.push_constants.iter().any(|p| p.name == pc.name) {
                self.push_constants.push(pc.clone());
            } else if let Some(existing) =
                self.push_constants.iter_mut().find(|p| p.name == pc.name)
            {
                // Merge stages
                existing.stages |= pc.stages;
            }
        }

        self.stages.insert(stage, reflection);
    }

    /// Get the total number of descriptor sets.
    pub fn descriptor_set_count(&self) -> u32 {
        self.bindings
            .iter()
            .map(|b| b.set)
            .max()
            .map(|s| s + 1)
            .unwrap_or(0)
    }

    /// Validate the program.
    pub fn validate(&self) -> Vec<ReflectionError> {
        let mut errors = Vec::new();

        // Check vertex-fragment interface
        if let (Some(vs), Some(fs)) = (
            self.stages.get(&ShaderStageFlags::VERTEX),
            self.stages.get(&ShaderStageFlags::FRAGMENT),
        ) {
            errors.extend(vs.validate_compatibility(fs));
        }

        errors
    }
}

// ============================================================================
// SPIR-V Parser (Minimal)
// ============================================================================

/// Minimal SPIR-V magic number check.
pub fn is_spirv(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    magic == 0x07230203
}

/// Get SPIR-V version.
pub fn spirv_version(data: &[u8]) -> Option<(u8, u8)> {
    if data.len() < 8 || !is_spirv(data) {
        return None;
    }

    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let major = ((version >> 16) & 0xFF) as u8;
    let minor = ((version >> 8) & 0xFF) as u8;
    Some((major, minor))
}
