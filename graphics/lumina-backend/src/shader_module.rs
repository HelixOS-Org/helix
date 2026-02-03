//! Shader Module Management
//!
//! Shader compilation, caching, and management.

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicU32, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

// ============================================================================
// Shader Stage
// ============================================================================

bitflags! {
    /// Shader stages.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ShaderStage: u32 {
        /// Vertex shader.
        const VERTEX = 1 << 0;
        /// Fragment/pixel shader.
        const FRAGMENT = 1 << 1;
        /// Compute shader.
        const COMPUTE = 1 << 2;
        /// Geometry shader.
        const GEOMETRY = 1 << 3;
        /// Tessellation control shader.
        const TESSELLATION_CONTROL = 1 << 4;
        /// Tessellation evaluation shader.
        const TESSELLATION_EVALUATION = 1 << 5;
        /// Task shader (mesh shading).
        const TASK = 1 << 6;
        /// Mesh shader.
        const MESH = 1 << 7;
        /// Ray generation shader.
        const RAY_GEN = 1 << 8;
        /// Any hit shader.
        const ANY_HIT = 1 << 9;
        /// Closest hit shader.
        const CLOSEST_HIT = 1 << 10;
        /// Miss shader.
        const MISS = 1 << 11;
        /// Intersection shader.
        const INTERSECTION = 1 << 12;
        /// Callable shader.
        const CALLABLE = 1 << 13;

        /// All graphics stages.
        const ALL_GRAPHICS = Self::VERTEX.bits() | Self::FRAGMENT.bits() |
            Self::GEOMETRY.bits() | Self::TESSELLATION_CONTROL.bits() |
            Self::TESSELLATION_EVALUATION.bits();

        /// All ray tracing stages.
        const ALL_RAY_TRACING = Self::RAY_GEN.bits() | Self::ANY_HIT.bits() |
            Self::CLOSEST_HIT.bits() | Self::MISS.bits() |
            Self::INTERSECTION.bits() | Self::CALLABLE.bits();

        /// All stages.
        const ALL = 0xFFFF;
    }
}

impl ShaderStage {
    /// Get single stage from flags.
    pub fn single_stage(&self) -> Option<Self> {
        if self.bits().count_ones() == 1 {
            Some(*self)
        } else {
            None
        }
    }

    /// Get stage name.
    pub fn name(&self) -> &'static str {
        match *self {
            Self::VERTEX => "vertex",
            Self::FRAGMENT => "fragment",
            Self::COMPUTE => "compute",
            Self::GEOMETRY => "geometry",
            Self::TESSELLATION_CONTROL => "tessellation_control",
            Self::TESSELLATION_EVALUATION => "tessellation_evaluation",
            Self::TASK => "task",
            Self::MESH => "mesh",
            Self::RAY_GEN => "ray_gen",
            Self::ANY_HIT => "any_hit",
            Self::CLOSEST_HIT => "closest_hit",
            Self::MISS => "miss",
            Self::INTERSECTION => "intersection",
            Self::CALLABLE => "callable",
            _ => "unknown",
        }
    }

    /// Get SPIR-V execution model.
    pub fn spirv_execution_model(&self) -> u32 {
        match *self {
            Self::VERTEX => 0,
            Self::TESSELLATION_CONTROL => 1,
            Self::TESSELLATION_EVALUATION => 2,
            Self::GEOMETRY => 3,
            Self::FRAGMENT => 4,
            Self::COMPUTE => 5,
            Self::TASK => 5267,
            Self::MESH => 5268,
            Self::RAY_GEN => 5313,
            Self::INTERSECTION => 5314,
            Self::ANY_HIT => 5315,
            Self::CLOSEST_HIT => 5316,
            Self::MISS => 5317,
            Self::CALLABLE => 5318,
            _ => 0,
        }
    }
}

// ============================================================================
// Shader Source
// ============================================================================

/// Shader source type.
#[derive(Debug, Clone)]
pub enum ShaderSource {
    /// SPIR-V binary.
    SpirV(Vec<u32>),
    /// GLSL source.
    Glsl(String),
    /// HLSL source.
    Hlsl(String),
    /// WGSL source.
    Wgsl(String),
    /// Metal Shading Language.
    Msl(String),
}

impl ShaderSource {
    /// Create from SPIR-V bytes.
    pub fn from_spirv_bytes(bytes: &[u8]) -> Self {
        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        ShaderSource::SpirV(words)
    }

    /// Check if SPIR-V.
    pub fn is_spirv(&self) -> bool {
        matches!(self, ShaderSource::SpirV(_))
    }

    /// Get size in bytes.
    pub fn size(&self) -> usize {
        match self {
            ShaderSource::SpirV(words) => words.len() * 4,
            ShaderSource::Glsl(s) => s.len(),
            ShaderSource::Hlsl(s) => s.len(),
            ShaderSource::Wgsl(s) => s.len(),
            ShaderSource::Msl(s) => s.len(),
        }
    }
}

// ============================================================================
// Shader Module Description
// ============================================================================

/// Description for shader module creation.
#[derive(Debug, Clone)]
pub struct ShaderModuleDesc {
    /// Shader source.
    pub source: ShaderSource,
    /// Entry point name.
    pub entry_point: String,
    /// Shader stage.
    pub stage: ShaderStage,
    /// Debug label.
    pub label: Option<String>,
}

impl ShaderModuleDesc {
    /// Create a new shader module description.
    pub fn new(source: ShaderSource, entry_point: impl Into<String>, stage: ShaderStage) -> Self {
        Self {
            source,
            entry_point: entry_point.into(),
            stage,
            label: None,
        }
    }

    /// Create vertex shader.
    pub fn vertex(source: ShaderSource) -> Self {
        Self::new(source, "main", ShaderStage::VERTEX)
    }

    /// Create fragment shader.
    pub fn fragment(source: ShaderSource) -> Self {
        Self::new(source, "main", ShaderStage::FRAGMENT)
    }

    /// Create compute shader.
    pub fn compute(source: ShaderSource) -> Self {
        Self::new(source, "main", ShaderStage::COMPUTE)
    }

    /// Set entry point.
    pub fn with_entry_point(mut self, entry_point: impl Into<String>) -> Self {
        self.entry_point = entry_point.into();
        self
    }

    /// Set label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

// ============================================================================
// Shader Module Handle
// ============================================================================

/// Handle to a shader module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderModuleHandle(Handle<ShaderModule>);

impl ShaderModuleHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Shader Module
// ============================================================================

/// A compiled shader module.
pub struct ShaderModule {
    /// Handle.
    pub handle: ShaderModuleHandle,
    /// Stage.
    pub stage: ShaderStage,
    /// Entry point.
    pub entry_point: String,
    /// Source hash (for caching).
    pub source_hash: u64,
    /// Size in bytes.
    pub size: usize,
    /// Debug label.
    pub label: Option<String>,
}

impl ShaderModule {
    /// Create a new shader module.
    pub fn new(handle: ShaderModuleHandle, desc: &ShaderModuleDesc) -> Self {
        Self {
            handle,
            stage: desc.stage,
            entry_point: desc.entry_point.clone(),
            source_hash: Self::hash_source(&desc.source),
            size: desc.source.size(),
            label: desc.label.clone(),
        }
    }

    /// Hash shader source.
    fn hash_source(source: &ShaderSource) -> u64 {
        // Simple FNV-1a hash
        let mut hash = 0xcbf29ce484222325u64;
        let bytes: &[u8] = match source {
            ShaderSource::SpirV(words) => {
                // SAFETY: Vec<u32> can be safely viewed as bytes
                unsafe {
                    core::slice::from_raw_parts(
                        words.as_ptr() as *const u8,
                        words.len() * 4,
                    )
                }
            }
            ShaderSource::Glsl(s) => s.as_bytes(),
            ShaderSource::Hlsl(s) => s.as_bytes(),
            ShaderSource::Wgsl(s) => s.as_bytes(),
            ShaderSource::Msl(s) => s.as_bytes(),
        };

        for byte in bytes {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        hash
    }
}

// ============================================================================
// Shader Manager
// ============================================================================

/// Manages shader modules.
pub struct ShaderManager {
    /// Shader modules.
    modules: Vec<Option<ShaderModule>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Hash to handle cache.
    cache: BTreeMap<u64, ShaderModuleHandle>,
    /// Module count.
    module_count: AtomicU32,
}

impl ShaderManager {
    /// Create a new shader manager.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            cache: BTreeMap::new(),
            module_count: AtomicU32::new(0),
        }
    }

    /// Create a shader module.
    pub fn create(&mut self, desc: &ShaderModuleDesc) -> ShaderModuleHandle {
        // Check cache
        let hash = ShaderModule::hash_source(&desc.source);
        if let Some(&handle) = self.cache.get(&hash) {
            if self.get(handle).is_some() {
                return handle;
            }
        }

        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.modules.len() as u32;
            self.modules.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = ShaderModuleHandle::new(index, generation);
        let module = ShaderModule::new(handle, desc);

        self.cache.insert(hash, handle);
        self.modules[index as usize] = Some(module);
        self.module_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Get a shader module.
    pub fn get(&self, handle: ShaderModuleHandle) -> Option<&ShaderModule> {
        let index = handle.index() as usize;
        if index >= self.modules.len() {
            return None;
        }
        self.modules[index].as_ref()
    }

    /// Destroy a shader module.
    pub fn destroy(&mut self, handle: ShaderModuleHandle) {
        let index = handle.index() as usize;
        if index >= self.modules.len() {
            return;
        }

        if let Some(module) = self.modules[index].take() {
            self.cache.remove(&module.source_hash);
            self.module_count.fetch_sub(1, Ordering::Relaxed);
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);
    }

    /// Get module count.
    pub fn count(&self) -> u32 {
        self.module_count.load(Ordering::Relaxed)
    }

    /// Clear cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for ShaderManager {
    fn default() -> Self {
        Self::new()
    }
}
