//! # LUMINA Asset Pipeline - Revolutionary GPU Asset Management
//!
//! Industry-leading asset pipeline with:
//! - **Custom Formats**: `.lumesh`, `.lumtex`, `.lummat` optimized for GPU
//! - **Texture Compression**: BC7, ASTC, ETC2 with quality presets
//! - **Mesh Optimization**: Meshlet generation, LOD, quantization
//! - **Streaming**: Virtual textures, progressive loading
//! - **Caching**: Content-addressed storage with deduplication
//! - **Hot Reload**: Live asset updates without restart
//!
//! ## Architecture
//!
//! ```text
//! Source Assets → Importers → Processors → Exporters → Runtime Assets
//!      ↓              ↓            ↓            ↓             ↓
//!   .fbx/.gltf    Parse      Optimize      Pack         GPU-ready
//!   .png/.exr     Validate   Compress     Serialize      Load
//!   .hlsl/.glsl   Convert    Generate     Cache          Bind
//! ```

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

pub mod cache;
pub mod exporter;
pub mod importer;
pub mod material;
pub mod mesh;
pub mod shader;
pub mod streaming;
pub mod texture;

pub use cache::*;
pub use exporter::*;
pub use importer::*;
pub use material::*;
pub use mesh::*;
pub use shader::*;
pub use streaming::*;
pub use texture::*;

/// Result type for asset operations
pub type AssetResult<T> = Result<T, AssetError>;

/// Asset error
#[derive(Debug, Clone)]
pub struct AssetError {
    pub kind: AssetErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetErrorKind {
    NotFound,
    InvalidFormat,
    CompressionError,
    IoError,
    ValidationError,
    ImportError,
    ExportError,
    CacheError,
    StreamingError,
}

impl AssetError {
    pub fn new(kind: AssetErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// Unique asset identifier (content-addressed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId {
    pub hash: [u8; 32],
}

impl AssetId {
    /// Create from content hash
    pub fn from_content(data: &[u8]) -> Self {
        Self {
            hash: compute_hash(data),
        }
    }

    /// Create from string path
    pub fn from_path(path: &str) -> Self {
        Self::from_content(path.as_bytes())
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for byte in &self.hash {
            use core::fmt::Write;
            let _ = write!(s, "{:02x}", byte);
        }
        s
    }
}

fn compute_hash(data: &[u8]) -> [u8; 32] {
    // Simple hash for demonstration (would use SHA-256 in production)
    let mut hash = [0u8; 32];
    for (i, &byte) in data.iter().enumerate() {
        hash[i % 32] ^= byte;
        hash[(i + 1) % 32] = hash[(i + 1) % 32].wrapping_add(byte);
    }
    hash
}

/// Asset metadata
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    pub id: AssetId,
    pub name: String,
    pub asset_type: AssetType,
    pub source_path: Option<String>,
    pub import_time: u64,
    pub size_bytes: u64,
    pub gpu_size_bytes: u64,
    pub dependencies: Vec<AssetId>,
    pub tags: Vec<String>,
    pub custom_data: BTreeMap<String, String>,
}

/// Asset types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Texture,
    Mesh,
    Material,
    Shader,
    Animation,
    Audio,
    Font,
    Scene,
    Prefab,
    Script,
    Custom,
}

/// Asset manager for runtime loading
pub struct AssetManager {
    cache: AssetCache,
    loaders: BTreeMap<AssetType, Box<dyn AssetLoader>>,
    pending_loads: Vec<PendingLoad>,
    loaded_assets: BTreeMap<AssetId, LoadedAsset>,
    hot_reload_enabled: bool,
}

impl AssetManager {
    /// Create a new asset manager
    pub fn new(cache_path: &str) -> AssetResult<Self> {
        Ok(Self {
            cache: AssetCache::new(cache_path)?,
            loaders: BTreeMap::new(),
            pending_loads: Vec::new(),
            loaded_assets: BTreeMap::new(),
            hot_reload_enabled: false,
        })
    }

    /// Register an asset loader
    pub fn register_loader(&mut self, asset_type: AssetType, loader: Box<dyn AssetLoader>) {
        self.loaders.insert(asset_type, loader);
    }

    /// Load an asset synchronously
    pub fn load_sync(&mut self, id: AssetId) -> AssetResult<&LoadedAsset> {
        if self.loaded_assets.contains_key(&id) {
            return self
                .loaded_assets
                .get(&id)
                .ok_or_else(|| AssetError::new(AssetErrorKind::NotFound, "Asset not found"));
        }

        let metadata = self.cache.get_metadata(id)?;
        let data = self.cache.load_data(id)?;

        let loader = self.loaders.get(&metadata.asset_type).ok_or_else(|| {
            AssetError::new(AssetErrorKind::InvalidFormat, "No loader for asset type")
        })?;

        let asset = loader.load(&data, &metadata)?;
        self.loaded_assets.insert(id, asset);

        self.loaded_assets
            .get(&id)
            .ok_or_else(|| AssetError::new(AssetErrorKind::NotFound, "Failed to get loaded asset"))
    }

    /// Queue async asset load
    pub fn load_async(&mut self, id: AssetId, priority: LoadPriority) {
        self.pending_loads.push(PendingLoad {
            id,
            priority,
            state: LoadState::Queued,
        });

        self.pending_loads
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Process pending loads
    pub fn update(&mut self) {
        // Process one pending load per update
        if let Some(pending) = self.pending_loads.first_mut() {
            if pending.state == LoadState::Queued {
                pending.state = LoadState::Loading;
                // Would start async load here
            }
        }
    }

    /// Unload an asset
    pub fn unload(&mut self, id: AssetId) {
        self.loaded_assets.remove(&id);
    }

    /// Enable hot reload
    pub fn enable_hot_reload(&mut self, enabled: bool) {
        self.hot_reload_enabled = enabled;
    }

    /// Check for hot reload updates
    pub fn check_hot_reload(&mut self) -> Vec<AssetId> {
        if !self.hot_reload_enabled {
            return Vec::new();
        }

        // Would check file modification times
        Vec::new()
    }
}

/// Asset loader trait
pub trait AssetLoader: Send + Sync {
    fn load(&self, data: &[u8], metadata: &AssetMetadata) -> AssetResult<LoadedAsset>;
    fn unload(&self, asset: &LoadedAsset);
}

/// Loaded asset
#[derive(Debug)]
pub struct LoadedAsset {
    pub id: AssetId,
    pub asset_type: AssetType,
    pub gpu_resources: Vec<GpuResource>,
    pub cpu_data: Option<Vec<u8>>,
    pub ref_count: u32,
}

/// GPU resource handle
#[derive(Debug, Clone)]
pub struct GpuResource {
    pub resource_type: GpuResourceType,
    pub handle: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuResourceType {
    Buffer,
    Texture,
    Sampler,
    AccelerationStructure,
}

/// Pending load request
struct PendingLoad {
    id: AssetId,
    priority: LoadPriority,
    state: LoadState,
}

/// Load priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadPriority {
    Low      = 0,
    Normal   = 1,
    High     = 2,
    Critical = 3,
}

/// Load state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadState {
    Queued,
    Loading,
    Processing,
    Complete,
    Failed,
}

/// Asset build pipeline
pub struct AssetPipeline {
    importers: BTreeMap<String, Box<dyn Importer>>,
    processors: Vec<Box<dyn Processor>>,
    exporters: BTreeMap<AssetType, Box<dyn Exporter>>,
    cache: AssetCache,
}

impl AssetPipeline {
    /// Create a new asset pipeline
    pub fn new(cache_path: &str) -> AssetResult<Self> {
        Ok(Self {
            importers: BTreeMap::new(),
            processors: Vec::new(),
            exporters: BTreeMap::new(),
            cache: AssetCache::new(cache_path)?,
        })
    }

    /// Register an importer for file extension
    pub fn register_importer(&mut self, extension: &str, importer: Box<dyn Importer>) {
        self.importers.insert(extension.to_lowercase(), importer);
    }

    /// Add a processor
    pub fn add_processor(&mut self, processor: Box<dyn Processor>) {
        self.processors.push(processor);
    }

    /// Register an exporter
    pub fn register_exporter(&mut self, asset_type: AssetType, exporter: Box<dyn Exporter>) {
        self.exporters.insert(asset_type, exporter);
    }

    /// Process an asset
    pub fn process(
        &mut self,
        source_path: &str,
        settings: &ImportSettings,
    ) -> AssetResult<AssetId> {
        // Get file extension
        let ext = source_path
            .rsplit('.')
            .next()
            .ok_or_else(|| AssetError::new(AssetErrorKind::InvalidFormat, "No file extension"))?
            .to_lowercase();

        // Find importer
        let importer = self.importers.get(&ext).ok_or_else(|| {
            AssetError::new(
                AssetErrorKind::ImportError,
                alloc::format!("No importer for .{}", ext),
            )
        })?;

        // Import
        let mut imported = importer.import(source_path, settings)?;

        // Process
        for processor in &self.processors {
            imported = processor.process(imported)?;
        }

        // Export
        let exporter = self.exporters.get(&imported.asset_type).ok_or_else(|| {
            AssetError::new(AssetErrorKind::ExportError, "No exporter for asset type")
        })?;

        let exported = exporter.export(&imported)?;

        // Cache
        let id = AssetId::from_content(&exported.data);
        self.cache.store(id, &exported.data, &imported.metadata)?;

        Ok(id)
    }
}

/// Import settings
#[derive(Debug, Clone)]
pub struct ImportSettings {
    pub texture_settings: TextureImportSettings,
    pub mesh_settings: MeshImportSettings,
    pub shader_settings: ShaderImportSettings,
    pub generate_mipmaps: bool,
    pub compress: bool,
    pub optimize: bool,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            texture_settings: TextureImportSettings::default(),
            mesh_settings: MeshImportSettings::default(),
            shader_settings: ShaderImportSettings::default(),
            generate_mipmaps: true,
            compress: true,
            optimize: true,
        }
    }
}

/// Importer trait
pub trait Importer: Send + Sync {
    fn import(&self, path: &str, settings: &ImportSettings) -> AssetResult<ImportedAsset>;
    fn supported_extensions(&self) -> &[&str];
}

/// Imported asset before processing
#[derive(Debug, Clone)]
pub struct ImportedAsset {
    pub asset_type: AssetType,
    pub metadata: AssetMetadata,
    pub data: ImportedData,
}

/// Imported data variants
#[derive(Debug, Clone)]
pub enum ImportedData {
    Texture(ImportedTexture),
    Mesh(ImportedMesh),
    Material(ImportedMaterial),
    Shader(ImportedShader),
    Raw(Vec<u8>),
}

/// Processor trait
pub trait Processor: Send + Sync {
    fn process(&self, asset: ImportedAsset) -> AssetResult<ImportedAsset>;
    fn supported_types(&self) -> &[AssetType];
}

/// Exporter trait
pub trait Exporter: Send + Sync {
    fn export(&self, asset: &ImportedAsset) -> AssetResult<ExportedAsset>;
}

/// Exported asset ready for storage
#[derive(Debug, Clone)]
pub struct ExportedAsset {
    pub data: Vec<u8>,
    pub format_version: u32,
}

/// Imported texture data
#[derive(Debug, Clone)]
pub struct ImportedTexture {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub format: TextureFormat,
    pub mip_levels: Vec<Vec<u8>>,
    pub array_layers: u32,
    pub is_cubemap: bool,
}

/// Imported mesh data
#[derive(Debug, Clone)]
pub struct ImportedMesh {
    pub vertices: Vec<ImportedVertex>,
    pub indices: Vec<u32>,
    pub submeshes: Vec<Submesh>,
    pub bounds: MeshBounds,
    pub skeleton: Option<ImportedSkeleton>,
}

/// Imported vertex
#[derive(Debug, Clone)]
pub struct ImportedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub uv0: [f32; 2],
    pub uv1: Option<[f32; 2]>,
    pub color: Option<[f32; 4]>,
    pub bone_indices: Option<[u8; 4]>,
    pub bone_weights: Option<[f32; 4]>,
}

/// Submesh
#[derive(Debug, Clone)]
pub struct Submesh {
    pub index_offset: u32,
    pub index_count: u32,
    pub material_index: u32,
}

/// Mesh bounds
#[derive(Debug, Clone)]
pub struct MeshBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub center: [f32; 3],
    pub radius: f32,
}

/// Imported skeleton
#[derive(Debug, Clone)]
pub struct ImportedSkeleton {
    pub bones: Vec<ImportedBone>,
    pub root_bone: u32,
}

/// Imported bone
#[derive(Debug, Clone)]
pub struct ImportedBone {
    pub name: String,
    pub parent: Option<u32>,
    pub local_transform: [[f32; 4]; 4],
    pub inverse_bind_matrix: [[f32; 4]; 4],
}

/// Imported material
#[derive(Debug, Clone)]
pub struct ImportedMaterial {
    pub name: String,
    pub shader: String,
    pub properties: BTreeMap<String, MaterialProperty>,
    pub textures: BTreeMap<String, String>,
}

/// Material property value
#[derive(Debug, Clone)]
pub enum MaterialProperty {
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float4([f32; 4]),
    Int(i32),
    Bool(bool),
    Texture(String),
}

/// Imported shader
#[derive(Debug, Clone)]
pub struct ImportedShader {
    pub name: String,
    pub stages: Vec<ShaderStageSource>,
    pub defines: BTreeMap<String, String>,
    pub includes: Vec<String>,
}

/// Shader stage source
#[derive(Debug, Clone)]
pub struct ShaderStageSource {
    pub stage: ShaderStageType,
    pub source: String,
    pub entry_point: String,
}

/// Shader stage type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStageType {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessControl,
    TessEval,
    Task,
    Mesh,
    RayGen,
    RayAnyHit,
    RayClosestHit,
    RayMiss,
    RayIntersection,
}

/// Texture format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    R8,
    Rg8,
    Rgba8,
    Rgba8Srgb,
    Bgra8,
    Bgra8Srgb,
    R16,
    Rg16,
    Rgba16,
    R16f,
    Rg16f,
    Rgba16f,
    R32f,
    Rg32f,
    Rgba32f,
    Depth16,
    Depth24,
    Depth32f,
    Depth24Stencil8,
    Bc1,
    Bc3,
    Bc4,
    Bc5,
    Bc6h,
    Bc7,
    Etc2Rgb,
    Etc2Rgba,
    Astc4x4,
    Astc5x5,
    Astc6x6,
    Astc8x8,
}

impl TextureFormat {
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1
                | Self::Bc3
                | Self::Bc4
                | Self::Bc5
                | Self::Bc6h
                | Self::Bc7
                | Self::Etc2Rgb
                | Self::Etc2Rgba
                | Self::Astc4x4
                | Self::Astc5x5
                | Self::Astc6x6
                | Self::Astc8x8
        )
    }

    pub fn bytes_per_pixel(&self) -> Option<u32> {
        match self {
            Self::R8 => Some(1),
            Self::Rg8 => Some(2),
            Self::Rgba8 | Self::Rgba8Srgb | Self::Bgra8 | Self::Bgra8Srgb => Some(4),
            Self::R16 | Self::R16f => Some(2),
            Self::Rg16 | Self::Rg16f => Some(4),
            Self::Rgba16 | Self::Rgba16f => Some(8),
            Self::R32f => Some(4),
            Self::Rg32f => Some(8),
            Self::Rgba32f => Some(16),
            _ => None, // Compressed formats have block size instead
        }
    }
}

/// Texture import settings
#[derive(Debug, Clone)]
pub struct TextureImportSettings {
    pub srgb: bool,
    pub generate_mipmaps: bool,
    pub compression: TextureCompression,
    pub max_size: Option<u32>,
    pub power_of_two: bool,
    pub filter: MipmapFilter,
}

impl Default for TextureImportSettings {
    fn default() -> Self {
        Self {
            srgb: true,
            generate_mipmaps: true,
            compression: TextureCompression::Auto,
            max_size: None,
            power_of_two: false,
            filter: MipmapFilter::Kaiser,
        }
    }
}

/// Texture compression mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureCompression {
    None,
    Auto,
    Bc7,
    Bc5,
    Bc4,
    Bc1,
    Astc,
    Etc2,
}

/// Mipmap filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipmapFilter {
    Box,
    Triangle,
    Lanczos,
    Kaiser,
    Mitchell,
}

/// Mesh import settings
#[derive(Debug, Clone)]
pub struct MeshImportSettings {
    pub generate_normals: bool,
    pub generate_tangents: bool,
    pub optimize_vertex_cache: bool,
    pub optimize_overdraw: bool,
    pub generate_lods: bool,
    pub lod_count: u32,
    pub generate_meshlets: bool,
    pub meshlet_max_vertices: u32,
    pub meshlet_max_triangles: u32,
    pub quantize_positions: bool,
    pub quantize_normals: bool,
}

impl Default for MeshImportSettings {
    fn default() -> Self {
        Self {
            generate_normals: true,
            generate_tangents: true,
            optimize_vertex_cache: true,
            optimize_overdraw: true,
            generate_lods: true,
            lod_count: 4,
            generate_meshlets: true,
            meshlet_max_vertices: 64,
            meshlet_max_triangles: 124,
            quantize_positions: false,
            quantize_normals: true,
        }
    }
}

/// Shader import settings
#[derive(Debug, Clone)]
pub struct ShaderImportSettings {
    pub target: ShaderTarget,
    pub optimization_level: OptimizationLevel,
    pub debug_info: bool,
    pub validate: bool,
}

impl Default for ShaderImportSettings {
    fn default() -> Self {
        Self {
            target: ShaderTarget::SpirV,
            optimization_level: OptimizationLevel::Performance,
            debug_info: false,
            validate: true,
        }
    }
}

/// Shader target format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderTarget {
    SpirV,
    Dxil,
    MetalSl,
    Glsl,
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,
    Size,
    Performance,
    Aggressive,
}
