//! # Shader Compilation
//!
//! GLSL to SPIR-V translation using Naga.

use crate::context::{ShaderObject, ShaderType};
use crate::enums::*;
use crate::types::*;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// SHADER STAGE
// =============================================================================

/// Vulkan shader stage flags
pub mod vk_stage {
    pub const VERTEX: u32 = 0x00000001;
    pub const TESSELLATION_CONTROL: u32 = 0x00000002;
    pub const TESSELLATION_EVALUATION: u32 = 0x00000004;
    pub const GEOMETRY: u32 = 0x00000008;
    pub const FRAGMENT: u32 = 0x00000010;
    pub const COMPUTE: u32 = 0x00000020;
}

/// Translate shader type to Vulkan stage
pub fn shader_type_to_vk_stage(shader_type: ShaderType) -> u32 {
    match shader_type {
        ShaderType::Vertex => vk_stage::VERTEX,
        ShaderType::Fragment => vk_stage::FRAGMENT,
        ShaderType::Geometry => vk_stage::GEOMETRY,
        ShaderType::TessControl => vk_stage::TESSELLATION_CONTROL,
        ShaderType::TessEvaluation => vk_stage::TESSELLATION_EVALUATION,
        ShaderType::Compute => vk_stage::COMPUTE,
    }
}

// =============================================================================
// GLSL VERSION DETECTION
// =============================================================================

/// GLSL version info
#[derive(Debug, Clone, Copy, Default)]
pub struct GlslVersion {
    /// Version number (e.g., 330, 450)
    pub version: u32,
    /// Is core profile
    pub core: bool,
    /// Is ES profile
    pub es: bool,
}

impl GlslVersion {
    /// Parse version from GLSL source
    pub fn parse(source: &str) -> Self {
        let mut version = GlslVersion::default();
        
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("#version") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(v) = parts[1].parse::<u32>() {
                        version.version = v;
                    }
                }
                if parts.len() >= 3 {
                    match parts[2] {
                        "core" => version.core = true,
                        "es" => version.es = true,
                        _ => {}
                    }
                }
                break;
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                // Non-comment content before #version
                break;
            }
        }
        
        // Default to 330 core if not specified
        if version.version == 0 {
            version.version = 330;
            version.core = true;
        }
        
        version
    }
}

// =============================================================================
// SHADER COMPILATION
// =============================================================================

/// Shader compilation result
#[derive(Debug)]
pub struct ShaderCompileResult {
    /// Whether compilation succeeded
    pub success: bool,
    /// SPIR-V bytecode (if successful)
    pub spirv: Option<Vec<u32>>,
    /// Compilation log
    pub log: String,
}

impl Default for ShaderCompileResult {
    fn default() -> Self {
        Self {
            success: false,
            spirv: None,
            log: String::new(),
        }
    }
}

/// Compile GLSL to SPIR-V
/// 
/// This would use the naga crate in a full implementation:
/// ```ignore
/// use naga::front::glsl::{Frontend, Options};
/// use naga::back::spv::{Writer, Options as SpvOptions};
/// ```
pub fn compile_glsl(source: &str, shader_type: ShaderType) -> ShaderCompileResult {
    let mut result = ShaderCompileResult::default();
    
    // Parse GLSL version
    let version = GlslVersion::parse(source);
    
    // Validate version
    if version.version < 330 && !version.es {
        result.log = String::from("Error: Only GLSL 3.30+ core profile is supported");
        return result;
    }
    
    // TODO: Actual compilation using naga
    // For now, return a placeholder success
    // 
    // Real implementation would be:
    // 1. Parse GLSL using naga::front::glsl::Frontend
    // 2. Validate the IR
    // 3. Generate SPIR-V using naga::back::spv::Writer
    
    #[cfg(feature = "naga")]
    {
        // use naga::front::glsl::{Frontend, Options, ShaderStage};
        // use naga::back::spv;
        // use naga::valid::{Validator, ValidationFlags, Capabilities};
        // 
        // let stage = match shader_type {
        //     ShaderType::Vertex => ShaderStage::Vertex,
        //     ShaderType::Fragment => ShaderStage::Fragment,
        //     ShaderType::Geometry => ShaderStage::Geometry,
        //     ShaderType::TessControl => ShaderStage::TessellationControl,
        //     ShaderType::TessEvaluation => ShaderStage::TessellationEvaluation,
        //     ShaderType::Compute => ShaderStage::Compute,
        // };
        // 
        // let mut frontend = Frontend::default();
        // let options = Options::from(stage);
        // 
        // match frontend.parse(&options, source) {
        //     Ok(module) => {
        //         let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        //         match validator.validate(&module) {
        //             Ok(info) => {
        //                 let spv_options = spv::Options::default();
        //                 let mut words = Vec::new();
        //                 let mut writer = spv::Writer::new(&spv_options).unwrap();
        //                 match writer.write(&module, &info, None, &mut words) {
        //                     Ok(_) => {
        //                         result.success = true;
        //                         result.spirv = Some(words);
        //                     }
        //                     Err(e) => {
        //                         result.log = format!("SPIR-V generation error: {:?}", e);
        //                     }
        //                 }
        //             }
        //             Err(e) => {
        //                 result.log = format!("Validation error: {:?}", e);
        //             }
        //         }
        //     }
        //     Err(e) => {
        //         result.log = format!("Parse error: {:?}", e);
        //     }
        // }
    }
    
    // Placeholder: mark as success for testing
    result.success = true;
    result.log = String::from("Compilation successful (placeholder)");
    
    result
}

// =============================================================================
// UNIFORM BLOCK LAYOUT
// =============================================================================

/// Uniform block layout (std140/std430)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UniformLayout {
    /// std140 layout (uniform buffers)
    Std140,
    /// std430 layout (SSBOs)
    Std430,
    /// Shared layout
    Shared,
    /// Packed layout
    Packed,
}

/// Calculate std140 alignment for a type
pub fn std140_alignment(gl_type: GLenum) -> usize {
    match gl_type {
        // Scalars
        GL_FLOAT | GL_INT | GL_UNSIGNED_INT => 4,
        GL_DOUBLE => 8,
        
        // Vectors align to 2N or 4N
        // vec2 -> 8 bytes
        // vec3, vec4 -> 16 bytes
        
        // Matrices align like arrays of vec4
        // mat4 -> 16 bytes per column
        
        _ => 16, // Default to vec4 alignment
    }
}

/// Calculate std140 size for a type
pub fn std140_size(gl_type: GLenum, array_size: usize) -> usize {
    let base_size = match gl_type {
        GL_FLOAT | GL_INT | GL_UNSIGNED_INT => 4,
        GL_DOUBLE => 8,
        // Add more types as needed
        _ => 16,
    };
    
    if array_size > 1 {
        // Array elements are rounded up to vec4 alignment
        let aligned = (base_size + 15) & !15;
        aligned * array_size
    } else {
        base_size
    }
}

// =============================================================================
// REFLECTION DATA
// =============================================================================

/// Uniform variable info from reflection
#[derive(Debug, Clone)]
pub struct ReflectedUniform {
    /// Uniform name
    pub name: String,
    /// GLSL type
    pub gl_type: GLenum,
    /// Array size (1 if not array)
    pub array_size: u32,
    /// Location (for default block)
    pub location: i32,
    /// Offset in block (for block members)
    pub offset: u32,
    /// Block index (-1 if default block)
    pub block_index: i32,
}

/// Uniform block info from reflection
#[derive(Debug, Clone)]
pub struct ReflectedUniformBlock {
    /// Block name
    pub name: String,
    /// Block index
    pub index: u32,
    /// Block size in bytes
    pub size: u32,
    /// Binding point
    pub binding: u32,
    /// Member uniforms
    pub members: Vec<ReflectedUniform>,
}

/// Vertex attribute info from reflection
#[derive(Debug, Clone)]
pub struct ReflectedAttribute {
    /// Attribute name
    pub name: String,
    /// GLSL type
    pub gl_type: GLenum,
    /// Location
    pub location: u32,
}

/// Fragment output info from reflection
#[derive(Debug, Clone)]
pub struct ReflectedOutput {
    /// Output name
    pub name: String,
    /// GLSL type
    pub gl_type: GLenum,
    /// Location
    pub location: u32,
    /// Color attachment index
    pub index: u32,
}

/// Complete shader reflection data
#[derive(Debug, Clone, Default)]
pub struct ShaderReflection {
    /// Vertex attributes (vertex shader only)
    pub attributes: Vec<ReflectedAttribute>,
    /// Uniforms in default block
    pub uniforms: Vec<ReflectedUniform>,
    /// Uniform blocks
    pub uniform_blocks: Vec<ReflectedUniformBlock>,
    /// Fragment outputs (fragment shader only)
    pub outputs: Vec<ReflectedOutput>,
    /// Shader storage blocks
    pub storage_blocks: Vec<ReflectedUniformBlock>,
}

// =============================================================================
// PROGRAM LINKING
// =============================================================================

/// Program link result
#[derive(Debug)]
pub struct ProgramLinkResult {
    /// Whether linking succeeded
    pub success: bool,
    /// Link log
    pub log: String,
    /// Combined reflection data
    pub reflection: ShaderReflection,
    /// Vulkan pipeline layout info
    pub pipeline_layout: PipelineLayoutInfo,
}

/// Pipeline layout descriptor
#[derive(Debug, Default)]
pub struct PipelineLayoutInfo {
    /// Descriptor set layouts (binding -> type)
    pub descriptor_sets: Vec<DescriptorSetInfo>,
    /// Push constant ranges
    pub push_constants: Vec<PushConstantRange>,
}

/// Descriptor set layout info
#[derive(Debug, Default)]
pub struct DescriptorSetInfo {
    /// Set number
    pub set: u32,
    /// Bindings in this set
    pub bindings: Vec<DescriptorBindingInfo>,
}

/// Descriptor binding info
#[derive(Debug)]
pub struct DescriptorBindingInfo {
    /// Binding number
    pub binding: u32,
    /// Descriptor type (VkDescriptorType)
    pub descriptor_type: u32,
    /// Descriptor count (for arrays)
    pub count: u32,
    /// Shader stages using this binding
    pub stage_flags: u32,
}

/// Push constant range
#[derive(Debug)]
pub struct PushConstantRange {
    /// Shader stages
    pub stage_flags: u32,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

/// Link shaders into a program
pub fn link_program(
    vertex_spirv: Option<&[u32]>,
    fragment_spirv: Option<&[u32]>,
    geometry_spirv: Option<&[u32]>,
    tess_control_spirv: Option<&[u32]>,
    tess_eval_spirv: Option<&[u32]>,
    compute_spirv: Option<&[u32]>,
) -> ProgramLinkResult {
    let mut result = ProgramLinkResult {
        success: false,
        log: String::new(),
        reflection: ShaderReflection::default(),
        pipeline_layout: PipelineLayoutInfo::default(),
    };
    
    // Validate shader combinations
    if compute_spirv.is_some() {
        // Compute pipeline
        if vertex_spirv.is_some() || fragment_spirv.is_some() {
            result.log = String::from("Error: Compute shader cannot be combined with graphics shaders");
            return result;
        }
    } else {
        // Graphics pipeline needs at least vertex and fragment
        if vertex_spirv.is_none() {
            result.log = String::from("Error: Vertex shader required for graphics pipeline");
            return result;
        }
        if fragment_spirv.is_none() {
            result.log = String::from("Error: Fragment shader required for graphics pipeline");
            return result;
        }
    }
    
    // TODO: Use spirv-reflect or naga to extract reflection data
    // TODO: Build descriptor set layouts
    // TODO: Validate interface matching between stages
    
    result.success = true;
    result.log = String::from("Link successful (placeholder)");
    
    result
}
