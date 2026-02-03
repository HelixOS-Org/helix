//! Shader Module and Compilation
//!
//! This module provides shader management including:
//! - SPIR-V shader modules
//! - Shader compilation with hot-reload
//! - Include resolution
//! - Shader caching

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

// ============================================================================
// Shader Stage
// ============================================================================

/// Shader stage type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderStage {
    /// Vertex shader.
    Vertex,
    /// Fragment/pixel shader.
    Fragment,
    /// Tessellation control/hull shader.
    TessellationControl,
    /// Tessellation evaluation/domain shader.
    TessellationEvaluation,
    /// Geometry shader.
    Geometry,
    /// Compute shader.
    Compute,
    /// Task shader (mesh shading pipeline).
    Task,
    /// Mesh shader (mesh shading pipeline).
    Mesh,
    /// Ray generation shader.
    RayGeneration,
    /// Intersection shader.
    Intersection,
    /// Any-hit shader.
    AnyHit,
    /// Closest-hit shader.
    ClosestHit,
    /// Miss shader.
    Miss,
    /// Callable shader.
    Callable,
}

impl ShaderStage {
    /// Get the SPIR-V execution model.
    pub fn spirv_execution_model(&self) -> u32 {
        match self {
            Self::Vertex => 0,
            Self::TessellationControl => 1,
            Self::TessellationEvaluation => 2,
            Self::Geometry => 3,
            Self::Fragment => 4,
            Self::Compute => 5,
            Self::Task => 5267, // TaskNV
            Self::Mesh => 5268, // MeshNV
            Self::RayGeneration => 5313,
            Self::Intersection => 5314,
            Self::AnyHit => 5315,
            Self::ClosestHit => 5316,
            Self::Miss => 5317,
            Self::Callable => 5318,
        }
    }

    /// Get file extension for this stage.
    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Vertex => "vert",
            Self::Fragment => "frag",
            Self::TessellationControl => "tesc",
            Self::TessellationEvaluation => "tese",
            Self::Geometry => "geom",
            Self::Compute => "comp",
            Self::Task => "task",
            Self::Mesh => "mesh",
            Self::RayGeneration => "rgen",
            Self::Intersection => "rint",
            Self::AnyHit => "rahit",
            Self::ClosestHit => "rchit",
            Self::Miss => "rmiss",
            Self::Callable => "rcall",
        }
    }

    /// Check if this is a ray tracing stage.
    pub fn is_ray_tracing(&self) -> bool {
        matches!(
            self,
            Self::RayGeneration
                | Self::Intersection
                | Self::AnyHit
                | Self::ClosestHit
                | Self::Miss
                | Self::Callable
        )
    }

    /// Check if this is a mesh shading stage.
    pub fn is_mesh_shading(&self) -> bool {
        matches!(self, Self::Task | Self::Mesh)
    }
}

// ============================================================================
// Shader Source
// ============================================================================

/// Shader source type.
#[derive(Clone)]
pub enum ShaderSource {
    /// SPIR-V binary.
    SpirV(Vec<u32>),
    /// GLSL source code.
    Glsl {
        /// Source code.
        source: String,
        /// Stage.
        stage: ShaderStage,
        /// Include paths.
        include_paths: Vec<String>,
        /// Defines.
        defines: Vec<(String, String)>,
    },
    /// HLSL source code.
    Hlsl {
        /// Source code.
        source: String,
        /// Entry point.
        entry_point: String,
        /// Target profile (e.g., "vs_6_0").
        profile: String,
        /// Include paths.
        include_paths: Vec<String>,
        /// Defines.
        defines: Vec<(String, String)>,
    },
    /// Precompiled shader binary (DXIL, Metal, etc.).
    Binary(Vec<u8>),
}

impl ShaderSource {
    /// Create SPIR-V source.
    pub fn spirv(code: Vec<u32>) -> Self {
        Self::SpirV(code)
    }

    /// Create GLSL source.
    pub fn glsl(source: &str, stage: ShaderStage) -> Self {
        Self::Glsl {
            source: String::from(source),
            stage,
            include_paths: Vec::new(),
            defines: Vec::new(),
        }
    }

    /// Create HLSL source.
    pub fn hlsl(source: &str, entry_point: &str, profile: &str) -> Self {
        Self::Hlsl {
            source: String::from(source),
            entry_point: String::from(entry_point),
            profile: String::from(profile),
            include_paths: Vec::new(),
            defines: Vec::new(),
        }
    }

    /// Check if this is already compiled.
    pub fn is_compiled(&self) -> bool {
        matches!(self, Self::SpirV(_) | Self::Binary(_))
    }
}

// ============================================================================
// Shader Module
// ============================================================================

/// Compiled shader module.
#[derive(Clone)]
pub struct ShaderModule {
    /// SPIR-V code.
    spirv: Vec<u32>,
    /// Stage.
    stage: ShaderStage,
    /// Entry point name.
    entry_point: String,
    /// Debug name.
    name: String,
    /// Source hash for hot-reload.
    source_hash: u64,
    /// Reflection data.
    reflection: Option<ShaderReflectionData>,
}

impl ShaderModule {
    /// Create a new shader module.
    pub fn new(spirv: Vec<u32>, stage: ShaderStage, entry_point: &str, name: &str) -> Self {
        let source_hash = Self::compute_hash(&spirv);
        Self {
            spirv,
            stage,
            entry_point: String::from(entry_point),
            name: String::from(name),
            source_hash,
            reflection: None,
        }
    }

    /// Create with reflection data.
    pub fn with_reflection(mut self, reflection: ShaderReflectionData) -> Self {
        self.reflection = Some(reflection);
        self
    }

    /// Get SPIR-V code.
    pub fn spirv(&self) -> &[u32] {
        &self.spirv
    }

    /// Get SPIR-V as bytes.
    pub fn spirv_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.spirv.as_ptr() as *const u8, self.spirv.len() * 4)
        }
    }

    /// Get the stage.
    pub fn stage(&self) -> ShaderStage {
        self.stage
    }

    /// Get the entry point.
    pub fn entry_point(&self) -> &str {
        &self.entry_point
    }

    /// Get the debug name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the source hash.
    pub fn source_hash(&self) -> u64 {
        self.source_hash
    }

    /// Get reflection data.
    pub fn reflection(&self) -> Option<&ShaderReflectionData> {
        self.reflection.as_ref()
    }

    /// Compute hash of SPIR-V code.
    fn compute_hash(spirv: &[u32]) -> u64 {
        let mut hasher = FnvHasher::new();
        for word in spirv {
            word.hash(&mut hasher);
        }
        hasher.finish()
    }
}

// ============================================================================
// Shader Reflection Data
// ============================================================================

/// Shader reflection data.
#[derive(Clone, Default)]
pub struct ShaderReflectionData {
    /// Descriptor bindings.
    pub bindings: Vec<ReflectedBinding>,
    /// Push constants.
    pub push_constants: Vec<ReflectedPushConstant>,
    /// Input variables.
    pub inputs: Vec<ReflectedVariable>,
    /// Output variables.
    pub outputs: Vec<ReflectedVariable>,
    /// Workgroup size (for compute shaders).
    pub workgroup_size: Option<[u32; 3]>,
    /// Required extensions.
    pub extensions: Vec<String>,
    /// Capabilities.
    pub capabilities: Vec<u32>,
}

/// Reflected descriptor binding.
#[derive(Clone)]
pub struct ReflectedBinding {
    /// Set index.
    pub set: u32,
    /// Binding index.
    pub binding: u32,
    /// Descriptor type.
    pub descriptor_type: ReflectedDescriptorType,
    /// Array size (0 = runtime array).
    pub count: u32,
    /// Name.
    pub name: String,
    /// Size in bytes (for buffers).
    pub size: u32,
    /// Members (for uniform/storage buffers).
    pub members: Vec<ReflectedMember>,
}

/// Reflected descriptor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectedDescriptorType {
    /// Sampler.
    Sampler,
    /// Combined image sampler.
    CombinedImageSampler,
    /// Sampled image.
    SampledImage,
    /// Storage image.
    StorageImage,
    /// Uniform texel buffer.
    UniformTexelBuffer,
    /// Storage texel buffer.
    StorageTexelBuffer,
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer.
    StorageBuffer,
    /// Uniform buffer dynamic.
    UniformBufferDynamic,
    /// Storage buffer dynamic.
    StorageBufferDynamic,
    /// Input attachment.
    InputAttachment,
    /// Acceleration structure.
    AccelerationStructure,
}

/// Reflected push constant.
#[derive(Clone)]
pub struct ReflectedPushConstant {
    /// Offset in bytes.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
    /// Shader stages.
    pub stages: ShaderStageFlags,
    /// Members.
    pub members: Vec<ReflectedMember>,
}

/// Reflected struct member.
#[derive(Clone)]
pub struct ReflectedMember {
    /// Name.
    pub name: String,
    /// Offset in bytes.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
    /// Type.
    pub member_type: ReflectedType,
    /// Array dimensions.
    pub array_dims: Vec<u32>,
    /// Matrix stride (for matrices).
    pub matrix_stride: u32,
}

/// Reflected type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectedType {
    /// Boolean.
    Bool,
    /// 32-bit signed integer.
    Int,
    /// 32-bit unsigned integer.
    Uint,
    /// 32-bit float.
    Float,
    /// 64-bit float.
    Double,
    /// Vector.
    Vector { component_count: u32 },
    /// Matrix.
    Matrix { columns: u32, rows: u32 },
    /// Struct.
    Struct,
    /// Unknown type.
    Unknown,
}

/// Reflected shader variable.
#[derive(Clone)]
pub struct ReflectedVariable {
    /// Location.
    pub location: u32,
    /// Name.
    pub name: String,
    /// Type.
    pub variable_type: ReflectedType,
    /// Format.
    pub format: ReflectedFormat,
    /// Component count.
    pub components: u32,
}

/// Reflected format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectedFormat {
    /// Unknown format.
    Unknown,
    /// R32 float.
    R32Float,
    /// RG32 float.
    Rg32Float,
    /// RGB32 float.
    Rgb32Float,
    /// RGBA32 float.
    Rgba32Float,
    /// R32 signed int.
    R32Sint,
    /// RG32 signed int.
    Rg32Sint,
    /// RGB32 signed int.
    Rgb32Sint,
    /// RGBA32 signed int.
    Rgba32Sint,
    /// R32 unsigned int.
    R32Uint,
    /// RG32 unsigned int.
    Rg32Uint,
    /// RGB32 unsigned int.
    Rgb32Uint,
    /// RGBA32 unsigned int.
    Rgba32Uint,
}

/// Shader stage flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShaderStageFlags(u32);

impl ShaderStageFlags {
    /// No stages.
    pub const NONE: Self = Self(0);
    /// Vertex stage.
    pub const VERTEX: Self = Self(1 << 0);
    /// Fragment stage.
    pub const FRAGMENT: Self = Self(1 << 1);
    /// Tessellation control stage.
    pub const TESSELLATION_CONTROL: Self = Self(1 << 2);
    /// Tessellation evaluation stage.
    pub const TESSELLATION_EVALUATION: Self = Self(1 << 3);
    /// Geometry stage.
    pub const GEOMETRY: Self = Self(1 << 4);
    /// Compute stage.
    pub const COMPUTE: Self = Self(1 << 5);
    /// Task stage.
    pub const TASK: Self = Self(1 << 6);
    /// Mesh stage.
    pub const MESH: Self = Self(1 << 7);
    /// Ray generation stage.
    pub const RAY_GENERATION: Self = Self(1 << 8);
    /// Intersection stage.
    pub const INTERSECTION: Self = Self(1 << 9);
    /// Any-hit stage.
    pub const ANY_HIT: Self = Self(1 << 10);
    /// Closest-hit stage.
    pub const CLOSEST_HIT: Self = Self(1 << 11);
    /// Miss stage.
    pub const MISS: Self = Self(1 << 12);
    /// Callable stage.
    pub const CALLABLE: Self = Self(1 << 13);
    /// All graphics stages.
    pub const ALL_GRAPHICS: Self = Self(0x1F);
    /// All stages.
    pub const ALL: Self = Self(0x3FFF);

    /// Create from stage.
    pub fn from_stage(stage: ShaderStage) -> Self {
        match stage {
            ShaderStage::Vertex => Self::VERTEX,
            ShaderStage::Fragment => Self::FRAGMENT,
            ShaderStage::TessellationControl => Self::TESSELLATION_CONTROL,
            ShaderStage::TessellationEvaluation => Self::TESSELLATION_EVALUATION,
            ShaderStage::Geometry => Self::GEOMETRY,
            ShaderStage::Compute => Self::COMPUTE,
            ShaderStage::Task => Self::TASK,
            ShaderStage::Mesh => Self::MESH,
            ShaderStage::RayGeneration => Self::RAY_GENERATION,
            ShaderStage::Intersection => Self::INTERSECTION,
            ShaderStage::AnyHit => Self::ANY_HIT,
            ShaderStage::ClosestHit => Self::CLOSEST_HIT,
            ShaderStage::Miss => Self::MISS,
            ShaderStage::Callable => Self::CALLABLE,
        }
    }

    /// Check if contains stage.
    pub fn contains(&self, stage: ShaderStage) -> bool {
        let flag = Self::from_stage(stage);
        (self.0 & flag.0) != 0
    }

    /// Combine flags.
    pub fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Shader Compiler
// ============================================================================

/// Shader compilation options.
#[derive(Clone, Default)]
pub struct CompileOptions {
    /// Optimization level (0-3).
    pub optimization_level: u32,
    /// Generate debug info.
    pub debug_info: bool,
    /// Defines.
    pub defines: Vec<(String, String)>,
    /// Include paths.
    pub include_paths: Vec<String>,
    /// Target Vulkan version (e.g., 0x00010002 for 1.2).
    pub vulkan_version: u32,
    /// Target SPIR-V version.
    pub spirv_version: u32,
}

/// Shader compilation error.
#[derive(Debug, Clone)]
pub enum ShaderError {
    /// Compilation failed.
    Compilation(String),
    /// Link failed.
    Link(String),
    /// Invalid SPIR-V.
    InvalidSpirV(String),
    /// File not found.
    FileNotFound(String),
    /// Include error.
    IncludeError(String),
    /// Unsupported feature.
    UnsupportedFeature(String),
    /// Reflection error.
    ReflectionError(String),
}

/// Shader compiler.
pub struct ShaderCompiler {
    /// Compilation options.
    options: CompileOptions,
    /// Shader cache.
    cache: ShaderCache,
    /// Include resolver.
    include_resolver: Option<Box<dyn IncludeResolver>>,
}

impl ShaderCompiler {
    /// Create a new shader compiler.
    pub fn new() -> Self {
        Self {
            options: CompileOptions::default(),
            cache: ShaderCache::new(),
            include_resolver: None,
        }
    }

    /// Create with options.
    pub fn with_options(options: CompileOptions) -> Self {
        Self {
            options,
            cache: ShaderCache::new(),
            include_resolver: None,
        }
    }

    /// Set include resolver.
    pub fn set_include_resolver<R: IncludeResolver + 'static>(&mut self, resolver: R) {
        self.include_resolver = Some(Box::new(resolver));
    }

    /// Compile shader source.
    pub fn compile(
        &mut self,
        source: &ShaderSource,
        name: &str,
    ) -> Result<Arc<ShaderModule>, ShaderError> {
        match source {
            ShaderSource::SpirV(spirv) => {
                // Validate and wrap existing SPIR-V
                let stage = self.detect_stage_from_spirv(spirv)?;
                let module = ShaderModule::new(spirv.clone(), stage, "main", name);
                Ok(Arc::new(module))
            },
            ShaderSource::Glsl {
                source,
                stage,
                defines,
                include_paths,
            } => {
                // Check cache first
                let cache_key = self.compute_source_hash(source, defines);
                if let Some(module) = self.cache.get(cache_key) {
                    return Ok(module);
                }

                // Compile GLSL to SPIR-V
                let spirv = self.compile_glsl(source, *stage, defines, include_paths)?;
                let module = Arc::new(ShaderModule::new(spirv, *stage, "main", name));
                self.cache.insert(cache_key, module.clone());
                Ok(module)
            },
            ShaderSource::Hlsl {
                source,
                entry_point,
                profile,
                defines,
                include_paths,
            } => {
                // Check cache first
                let cache_key = self.compute_source_hash(source, defines);
                if let Some(module) = self.cache.get(cache_key) {
                    return Ok(module);
                }

                // Compile HLSL to SPIR-V
                let (spirv, stage) =
                    self.compile_hlsl(source, entry_point, profile, defines, include_paths)?;
                let module = Arc::new(ShaderModule::new(spirv, stage, entry_point, name));
                self.cache.insert(cache_key, module.clone());
                Ok(module)
            },
            ShaderSource::Binary(data) => {
                // Assume it's SPIR-V binary
                if data.len() % 4 != 0 {
                    return Err(ShaderError::InvalidSpirV(
                        "Binary length not aligned to 4 bytes".into(),
                    ));
                }
                let spirv: Vec<u32> = data
                    .chunks_exact(4)
                    .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                let stage = self.detect_stage_from_spirv(&spirv)?;
                let module = ShaderModule::new(spirv, stage, "main", name);
                Ok(Arc::new(module))
            },
        }
    }

    /// Compile GLSL to SPIR-V.
    fn compile_glsl(
        &self,
        _source: &str,
        stage: ShaderStage,
        _defines: &[(String, String)],
        _include_paths: &[String],
    ) -> Result<Vec<u32>, ShaderError> {
        // In a real implementation, this would use shaderc or glslang
        // For now, return a placeholder
        Ok(self.create_placeholder_spirv(stage))
    }

    /// Compile HLSL to SPIR-V.
    fn compile_hlsl(
        &self,
        _source: &str,
        _entry_point: &str,
        profile: &str,
        _defines: &[(String, String)],
        _include_paths: &[String],
    ) -> Result<(Vec<u32>, ShaderStage), ShaderError> {
        // Parse stage from profile
        let stage = if profile.starts_with("vs") {
            ShaderStage::Vertex
        } else if profile.starts_with("ps") || profile.starts_with("fs") {
            ShaderStage::Fragment
        } else if profile.starts_with("cs") {
            ShaderStage::Compute
        } else if profile.starts_with("gs") {
            ShaderStage::Geometry
        } else if profile.starts_with("hs") {
            ShaderStage::TessellationControl
        } else if profile.starts_with("ds") {
            ShaderStage::TessellationEvaluation
        } else if profile.starts_with("ms") {
            ShaderStage::Mesh
        } else if profile.starts_with("as") {
            ShaderStage::Task
        } else {
            return Err(ShaderError::UnsupportedFeature(alloc::format!(
                "Unknown profile: {}",
                profile
            )));
        };

        // In a real implementation, this would use DXC
        Ok((self.create_placeholder_spirv(stage), stage))
    }

    /// Create placeholder SPIR-V.
    fn create_placeholder_spirv(&self, _stage: ShaderStage) -> Vec<u32> {
        // Minimal valid SPIR-V header
        vec![
            0x07230203, // Magic number
            0x00010000, // Version 1.0
            0x00000000, // Generator
            0x00000001, // Bound
            0x00000000, // Schema
        ]
    }

    /// Detect shader stage from SPIR-V.
    fn detect_stage_from_spirv(&self, spirv: &[u32]) -> Result<ShaderStage, ShaderError> {
        if spirv.len() < 5 {
            return Err(ShaderError::InvalidSpirV("SPIR-V too short".into()));
        }
        if spirv[0] != 0x07230203 {
            return Err(ShaderError::InvalidSpirV("Invalid magic number".into()));
        }

        // Scan for OpEntryPoint
        let mut i = 5;
        while i < spirv.len() {
            let word_count = (spirv[i] >> 16) as usize;
            let opcode = spirv[i] & 0xFFFF;

            if opcode == 15 && word_count >= 3 {
                // OpEntryPoint
                let execution_model = spirv[i + 1];
                return match execution_model {
                    0 => Ok(ShaderStage::Vertex),
                    1 => Ok(ShaderStage::TessellationControl),
                    2 => Ok(ShaderStage::TessellationEvaluation),
                    3 => Ok(ShaderStage::Geometry),
                    4 => Ok(ShaderStage::Fragment),
                    5 => Ok(ShaderStage::Compute),
                    5267 => Ok(ShaderStage::Task),
                    5268 => Ok(ShaderStage::Mesh),
                    5313 => Ok(ShaderStage::RayGeneration),
                    5314 => Ok(ShaderStage::Intersection),
                    5315 => Ok(ShaderStage::AnyHit),
                    5316 => Ok(ShaderStage::ClosestHit),
                    5317 => Ok(ShaderStage::Miss),
                    5318 => Ok(ShaderStage::Callable),
                    _ => Err(ShaderError::InvalidSpirV(alloc::format!(
                        "Unknown execution model: {}",
                        execution_model
                    ))),
                };
            }

            i += word_count.max(1);
        }

        Err(ShaderError::InvalidSpirV("No OpEntryPoint found".into()))
    }

    /// Compute source hash.
    fn compute_source_hash(&self, source: &str, defines: &[(String, String)]) -> u64 {
        let mut hasher = FnvHasher::new();
        source.hash(&mut hasher);
        for (name, value) in defines {
            name.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        self.options.optimization_level.hash(&mut hasher);
        self.options.debug_info.hash(&mut hasher);
        hasher.finish()
    }

    /// Reflect shader module.
    pub fn reflect(&self, module: &ShaderModule) -> Result<ShaderReflectionData, ShaderError> {
        // In a real implementation, this would use spirv-reflect or similar
        let mut data = ShaderReflectionData::default();

        // Parse SPIR-V for reflection data
        let spirv = module.spirv();
        let mut i = 5;

        while i < spirv.len() {
            let word_count = (spirv[i] >> 16) as usize;
            let _opcode = spirv[i] & 0xFFFF;

            // In real implementation, parse OpDecorate, OpVariable, etc.
            i += word_count.max(1);
        }

        // Detect workgroup size for compute shaders
        if module.stage() == ShaderStage::Compute {
            data.workgroup_size = Some([1, 1, 1]); // Default
        }

        Ok(data)
    }
}

impl Default for ShaderCompiler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Include Resolver
// ============================================================================

/// Include resolver trait.
pub trait IncludeResolver: Send + Sync {
    /// Resolve an include.
    fn resolve(&self, path: &str, parent: Option<&str>) -> Result<String, ShaderError>;
}

/// Default include resolver (no-op).
pub struct DefaultIncludeResolver;

impl IncludeResolver for DefaultIncludeResolver {
    fn resolve(&self, path: &str, _parent: Option<&str>) -> Result<String, ShaderError> {
        Err(ShaderError::FileNotFound(String::from(path)))
    }
}

// ============================================================================
// Shader Cache
// ============================================================================

/// Shader module cache.
pub struct ShaderCache {
    /// Cached modules.
    modules: Vec<(u64, Arc<ShaderModule>)>,
    /// Maximum cache size.
    max_size: usize,
}

impl ShaderCache {
    /// Create a new shader cache.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            max_size: 256,
        }
    }

    /// Create with max size.
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            modules: Vec::new(),
            max_size,
        }
    }

    /// Get a cached module.
    pub fn get(&self, hash: u64) -> Option<Arc<ShaderModule>> {
        self.modules
            .iter()
            .find(|(h, _)| *h == hash)
            .map(|(_, m)| m.clone())
    }

    /// Insert a module.
    pub fn insert(&mut self, hash: u64, module: Arc<ShaderModule>) {
        // Evict if at capacity
        if self.modules.len() >= self.max_size {
            self.modules.remove(0);
        }
        self.modules.push((hash, module));
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.modules.clear();
    }

    /// Get cache size.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Default for ShaderCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hot Reload Support
// ============================================================================

/// Shader hot reload manager.
#[cfg(feature = "shader-hot-reload")]
pub struct ShaderHotReloader {
    /// Watched shaders.
    watched: Vec<WatchedShader>,
    /// Compiler.
    compiler: ShaderCompiler,
    /// Reload callbacks.
    callbacks: Vec<Box<dyn Fn(&Arc<ShaderModule>) + Send + Sync>>,
}

#[cfg(feature = "shader-hot-reload")]
struct WatchedShader {
    path: String,
    source: ShaderSource,
    last_hash: u64,
    module: Arc<ShaderModule>,
}

#[cfg(feature = "shader-hot-reload")]
impl ShaderHotReloader {
    /// Create a new hot reloader.
    pub fn new() -> Self {
        Self {
            watched: Vec::new(),
            compiler: ShaderCompiler::new(),
            callbacks: Vec::new(),
        }
    }

    /// Watch a shader file.
    pub fn watch(
        &mut self,
        path: &str,
        source: ShaderSource,
    ) -> Result<Arc<ShaderModule>, ShaderError> {
        let module = self.compiler.compile(&source, path)?;
        let hash = module.source_hash();

        self.watched.push(WatchedShader {
            path: String::from(path),
            source,
            last_hash: hash,
            module: module.clone(),
        });

        Ok(module)
    }

    /// Add reload callback.
    pub fn on_reload<F: Fn(&Arc<ShaderModule>) + Send + Sync + 'static>(&mut self, callback: F) {
        self.callbacks.push(Box::new(callback));
    }

    /// Check for changes and reload.
    pub fn check_and_reload(&mut self) -> Vec<Arc<ShaderModule>> {
        let mut reloaded = Vec::new();

        for watched in &mut self.watched {
            // In real implementation, check file modification time
            // and recompile if changed
            let _ = watched;
        }

        // Notify callbacks
        for module in &reloaded {
            for callback in &self.callbacks {
                callback(module);
            }
        }

        reloaded
    }

    /// Force reload a shader.
    pub fn force_reload(&mut self, path: &str) -> Option<Arc<ShaderModule>> {
        for watched in &mut self.watched {
            if watched.path == path {
                if let Ok(module) = self.compiler.compile(&watched.source, path) {
                    watched.module = module.clone();
                    watched.last_hash = module.source_hash();
                    return Some(module);
                }
            }
        }
        None
    }
}

#[cfg(feature = "shader-hot-reload")]
impl Default for ShaderHotReloader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Shader Library
// ============================================================================

/// Shader library for managing shader modules.
pub struct ShaderLibrary {
    /// Shaders by name.
    shaders: Vec<(String, Arc<ShaderModule>)>,
    /// Compiler.
    compiler: ShaderCompiler,
}

impl ShaderLibrary {
    /// Create a new shader library.
    pub fn new() -> Self {
        Self {
            shaders: Vec::new(),
            compiler: ShaderCompiler::new(),
        }
    }

    /// Create with compiler.
    pub fn with_compiler(compiler: ShaderCompiler) -> Self {
        Self {
            shaders: Vec::new(),
            compiler,
        }
    }

    /// Add a shader.
    pub fn add(
        &mut self,
        name: &str,
        source: ShaderSource,
    ) -> Result<Arc<ShaderModule>, ShaderError> {
        let module = self.compiler.compile(&source, name)?;
        self.shaders.push((String::from(name), module.clone()));
        Ok(module)
    }

    /// Get a shader by name.
    pub fn get(&self, name: &str) -> Option<Arc<ShaderModule>> {
        self.shaders
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, m)| m.clone())
    }

    /// Remove a shader.
    pub fn remove(&mut self, name: &str) -> Option<Arc<ShaderModule>> {
        if let Some(pos) = self.shaders.iter().position(|(n, _)| n == name) {
            Some(self.shaders.remove(pos).1)
        } else {
            None
        }
    }

    /// Get all shader names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.shaders.iter().map(|(n, _)| n.as_str())
    }

    /// Clear the library.
    pub fn clear(&mut self) {
        self.shaders.clear();
    }
}

impl Default for ShaderLibrary {
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
