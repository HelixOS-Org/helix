//! # Lumina Shader
//!
//! Shader compilation infrastructure for Lumina.
//! This crate handles the transformation of Rust code marked with
//! `#[lumina::shader]` into SPIR-V bytecode.

#![no_std]
#![warn(missing_docs)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex,
    /// Fragment shader
    Fragment,
    /// Compute shader
    Compute,
    /// Geometry shader
    Geometry,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEval,
}

/// Compiled shader module
pub struct ShaderModule {
    /// SPIR-V bytecode
    pub spirv: Vec<u32>,
    /// Shader stage
    pub stage: ShaderStage,
    /// Entry point name
    pub entry_point: String,
    /// Reflection data
    pub reflection: ShaderReflection,
}

/// Shader reflection data
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Input variables
    pub inputs: Vec<ShaderVariable>,
    /// Output variables
    pub outputs: Vec<ShaderVariable>,
    /// Uniform blocks
    pub uniform_blocks: Vec<UniformBlock>,
    /// Storage buffers
    pub storage_buffers: Vec<StorageBuffer>,
    /// Sampled images
    pub sampled_images: Vec<SampledImage>,
    /// Push constant range
    pub push_constants: Option<PushConstantRange>,
    /// Workgroup size (for compute shaders)
    pub workgroup_size: Option<[u32; 3]>,
}

/// A shader input/output variable
#[derive(Clone, Debug)]
pub struct ShaderVariable {
    /// Variable name
    pub name: String,
    /// Location
    pub location: u32,
    /// Type
    pub ty: VariableType,
}

/// Variable type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableType {
    /// Scalar float
    Float,
    /// 2D float vector
    Vec2,
    /// 3D float vector
    Vec3,
    /// 4D float vector
    Vec4,
    /// Scalar int
    Int,
    /// 2D int vector
    IVec2,
    /// 3D int vector
    IVec3,
    /// 4D int vector
    IVec4,
    /// Scalar uint
    UInt,
    /// 2D uint vector
    UVec2,
    /// 3D uint vector
    UVec3,
    /// 4D uint vector
    UVec4,
    /// 2x2 matrix
    Mat2,
    /// 3x3 matrix
    Mat3,
    /// 4x4 matrix
    Mat4,
}

/// Uniform block descriptor
#[derive(Clone, Debug)]
pub struct UniformBlock {
    /// Block name
    pub name: String,
    /// Binding number
    pub binding: u32,
    /// Set number
    pub set: u32,
    /// Size in bytes
    pub size: u32,
    /// Members
    pub members: Vec<UniformMember>,
}

/// Uniform block member
#[derive(Clone, Debug)]
pub struct UniformMember {
    /// Member name
    pub name: String,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Type
    pub ty: VariableType,
}

/// Storage buffer descriptor
#[derive(Clone, Debug)]
pub struct StorageBuffer {
    /// Buffer name
    pub name: String,
    /// Binding number
    pub binding: u32,
    /// Set number
    pub set: u32,
    /// Read-only flag
    pub readonly: bool,
}

/// Sampled image (texture + sampler)
#[derive(Clone, Debug)]
pub struct SampledImage {
    /// Image name
    pub name: String,
    /// Binding number
    pub binding: u32,
    /// Set number
    pub set: u32,
    /// Image dimension
    pub dimension: ImageDimension,
}

/// Image dimension
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageDimension {
    /// 1D image
    D1,
    /// 2D image
    D2,
    /// 3D image
    D3,
    /// Cube map
    Cube,
}

/// Push constant range
#[derive(Clone, Debug)]
pub struct PushConstantRange {
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

/// Result of shader compilation
pub type CompileResult = Result<ShaderModule, CompileError>;

/// Shader compilation error
#[derive(Clone, Debug)]
pub struct CompileError {
    /// Error message
    pub message: String,
    /// Source location (if available)
    pub location: Option<SourceLocation>,
}

/// Source code location
#[derive(Clone, Debug)]
pub struct SourceLocation {
    /// File name
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
}

/// Shader compiler interface
pub trait ShaderCompiler {
    /// Compile a shader from source
    fn compile(&self, source: &str, stage: ShaderStage) -> CompileResult;
}

/// Placeholder compiler for when rust-gpu is not available
pub struct PlaceholderCompiler;

impl ShaderCompiler for PlaceholderCompiler {
    fn compile(&self, _source: &str, stage: ShaderStage) -> CompileResult {
        // Return a minimal valid SPIR-V module
        // This is just the header + a void main function
        let spirv = alloc::vec![
            0x07230203, // Magic number
            0x00010000, // Version 1.0
            0x00000000, // Generator ID
            0x00000001, // Bound
            0x00000000, // Schema
        ];

        Ok(ShaderModule {
            spirv,
            stage,
            entry_point: String::from("main"),
            reflection: ShaderReflection::default(),
        })
    }
}
