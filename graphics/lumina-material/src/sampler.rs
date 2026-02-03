//! Sampler Management
//!
//! This module provides sampler creation and caching.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

// ============================================================================
// Sampler Handle
// ============================================================================

/// Handle to a sampler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerHandle {
    index: u32,
}

impl SamplerHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self { index: u32::MAX };

    /// Create a new handle.
    pub fn new(index: u32) -> Self {
        Self { index }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Filter Mode
// ============================================================================

/// Texture filtering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FilterMode {
    /// Nearest neighbor (point) filtering.
    Nearest,
    /// Linear (bilinear) filtering.
    #[default]
    Linear,
}

/// Mipmap filter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MipmapMode {
    /// Nearest mip level.
    Nearest,
    /// Linear interpolation between mip levels.
    #[default]
    Linear,
}

// ============================================================================
// Address Mode
// ============================================================================

/// Texture addressing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AddressMode {
    /// Repeat the texture.
    #[default]
    Repeat,
    /// Mirror the texture.
    MirrorRepeat,
    /// Clamp to edge.
    ClampToEdge,
    /// Clamp to border color.
    ClampToBorder,
    /// Mirror then clamp.
    MirrorClampToEdge,
}

// ============================================================================
// Border Color
// ============================================================================

/// Border color for ClampToBorder addressing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BorderColor {
    /// Transparent black.
    #[default]
    TransparentBlack,
    /// Opaque black.
    OpaqueBlack,
    /// Opaque white.
    OpaqueWhite,
}

// ============================================================================
// Compare Operation
// ============================================================================

/// Compare operation for depth samplers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompareOp {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

// ============================================================================
// Sampler Descriptor
// ============================================================================

/// Sampler descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerDesc {
    /// Minification filter.
    pub min_filter: FilterMode,
    /// Magnification filter.
    pub mag_filter: FilterMode,
    /// Mipmap filter.
    pub mipmap_mode: MipmapMode,
    /// Address mode U.
    pub address_mode_u: AddressMode,
    /// Address mode V.
    pub address_mode_v: AddressMode,
    /// Address mode W.
    pub address_mode_w: AddressMode,
    /// Mip LOD bias.
    pub mip_lod_bias: i16, // Fixed point 8.8
    /// Enable anisotropic filtering.
    pub anisotropy_enable: bool,
    /// Max anisotropy.
    pub max_anisotropy: u8,
    /// Enable compare.
    pub compare_enable: bool,
    /// Compare operation.
    pub compare_op: Option<CompareOp>,
    /// Min LOD.
    pub min_lod: u16, // Fixed point 8.8
    /// Max LOD.
    pub max_lod: u16, // Fixed point 8.8
    /// Border color.
    pub border_color: BorderColor,
    /// Unnormalized coordinates.
    pub unnormalized_coordinates: bool,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mip_lod_bias: 0,
            anisotropy_enable: false,
            max_anisotropy: 1,
            compare_enable: false,
            compare_op: None,
            min_lod: 0,
            max_lod: 1000 << 8, // 1000.0
            border_color: BorderColor::TransparentBlack,
            unnormalized_coordinates: false,
        }
    }
}

impl SamplerDesc {
    /// Create a new sampler descriptor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a point (nearest) sampler.
    pub fn point() -> Self {
        Self {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_mode: MipmapMode::Nearest,
            ..Default::default()
        }
    }

    /// Create a bilinear sampler.
    pub fn bilinear() -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Nearest,
            ..Default::default()
        }
    }

    /// Create a trilinear sampler.
    pub fn trilinear() -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Linear,
            ..Default::default()
        }
    }

    /// Create an anisotropic sampler.
    pub fn anisotropic(max_anisotropy: u8) -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Linear,
            anisotropy_enable: true,
            max_anisotropy,
            ..Default::default()
        }
    }

    /// Create a shadow sampler.
    pub fn shadow() -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_mode: MipmapMode::Nearest,
            address_mode_u: AddressMode::ClampToBorder,
            address_mode_v: AddressMode::ClampToBorder,
            address_mode_w: AddressMode::ClampToBorder,
            compare_enable: true,
            compare_op: Some(CompareOp::LessEqual),
            border_color: BorderColor::OpaqueWhite,
            ..Default::default()
        }
    }

    /// Create a clamp sampler.
    pub fn clamp() -> Self {
        Self {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..Default::default()
        }
    }

    /// Set min filter.
    pub fn min_filter(mut self, filter: FilterMode) -> Self {
        self.min_filter = filter;
        self
    }

    /// Set mag filter.
    pub fn mag_filter(mut self, filter: FilterMode) -> Self {
        self.mag_filter = filter;
        self
    }

    /// Set both filters.
    pub fn filter(mut self, filter: FilterMode) -> Self {
        self.min_filter = filter;
        self.mag_filter = filter;
        self
    }

    /// Set mipmap mode.
    pub fn mipmap(mut self, mode: MipmapMode) -> Self {
        self.mipmap_mode = mode;
        self
    }

    /// Set address mode for all axes.
    pub fn address_mode(mut self, mode: AddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
        self
    }

    /// Set address mode U.
    pub fn address_mode_u(mut self, mode: AddressMode) -> Self {
        self.address_mode_u = mode;
        self
    }

    /// Set address mode V.
    pub fn address_mode_v(mut self, mode: AddressMode) -> Self {
        self.address_mode_v = mode;
        self
    }

    /// Set address mode W.
    pub fn address_mode_w(mut self, mode: AddressMode) -> Self {
        self.address_mode_w = mode;
        self
    }

    /// Enable anisotropic filtering.
    pub fn anisotropy(mut self, max: u8) -> Self {
        self.anisotropy_enable = max > 1;
        self.max_anisotropy = max;
        self
    }

    /// Set LOD bias.
    pub fn lod_bias(mut self, bias: f32) -> Self {
        self.mip_lod_bias = (bias * 256.0) as i16;
        self
    }

    /// Set LOD clamp.
    pub fn lod_clamp(mut self, min: f32, max: f32) -> Self {
        self.min_lod = (min * 256.0) as u16;
        self.max_lod = (max * 256.0) as u16;
        self
    }

    /// Set border color.
    pub fn border_color(mut self, color: BorderColor) -> Self {
        self.border_color = color;
        self
    }

    /// Enable compare.
    pub fn compare(mut self, op: CompareOp) -> Self {
        self.compare_enable = true;
        self.compare_op = Some(op);
        self
    }

    /// Compute hash.
    pub fn hash(&self) -> u64 {
        let mut hasher = FnvHasher::new();
        self.min_filter.hash(&mut hasher);
        self.mag_filter.hash(&mut hasher);
        self.mipmap_mode.hash(&mut hasher);
        self.address_mode_u.hash(&mut hasher);
        self.address_mode_v.hash(&mut hasher);
        self.address_mode_w.hash(&mut hasher);
        self.mip_lod_bias.hash(&mut hasher);
        self.anisotropy_enable.hash(&mut hasher);
        self.max_anisotropy.hash(&mut hasher);
        self.compare_enable.hash(&mut hasher);
        self.compare_op.hash(&mut hasher);
        self.min_lod.hash(&mut hasher);
        self.max_lod.hash(&mut hasher);
        self.border_color.hash(&mut hasher);
        hasher.finish()
    }
}

// ============================================================================
// Sampler
// ============================================================================

/// Sampler resource.
pub struct Sampler {
    /// Handle.
    handle: SamplerHandle,
    /// Descriptor.
    desc: SamplerDesc,
    /// Bindless index.
    bindless_index: Option<u32>,
}

impl Sampler {
    /// Create a new sampler.
    pub fn new(handle: SamplerHandle, desc: SamplerDesc) -> Self {
        Self {
            handle,
            desc,
            bindless_index: None,
        }
    }

    /// Get handle.
    pub fn handle(&self) -> SamplerHandle {
        self.handle
    }

    /// Get descriptor.
    pub fn desc(&self) -> &SamplerDesc {
        &self.desc
    }

    /// Get bindless index.
    pub fn bindless_index(&self) -> Option<u32> {
        self.bindless_index
    }

    /// Set bindless index.
    pub fn set_bindless_index(&mut self, index: u32) {
        self.bindless_index = Some(index);
    }
}

// ============================================================================
// Sampler Cache
// ============================================================================

/// Sampler cache for deduplication.
pub struct SamplerCache {
    /// Samplers by descriptor hash.
    cache: BTreeMap<u64, SamplerHandle>,
    /// All samplers.
    samplers: Vec<Sampler>,
    /// Common samplers.
    common: CommonSamplers,
}

/// Common sampler handles.
#[derive(Debug, Clone, Copy)]
pub struct CommonSamplers {
    /// Point sampler (repeat).
    pub point_repeat: SamplerHandle,
    /// Point sampler (clamp).
    pub point_clamp: SamplerHandle,
    /// Linear sampler (repeat).
    pub linear_repeat: SamplerHandle,
    /// Linear sampler (clamp).
    pub linear_clamp: SamplerHandle,
    /// Trilinear sampler (repeat).
    pub trilinear_repeat: SamplerHandle,
    /// Trilinear sampler (clamp).
    pub trilinear_clamp: SamplerHandle,
    /// Anisotropic sampler (4x).
    pub aniso4x: SamplerHandle,
    /// Anisotropic sampler (8x).
    pub aniso8x: SamplerHandle,
    /// Anisotropic sampler (16x).
    pub aniso16x: SamplerHandle,
    /// Shadow sampler.
    pub shadow: SamplerHandle,
}

impl Default for CommonSamplers {
    fn default() -> Self {
        Self {
            point_repeat: SamplerHandle::INVALID,
            point_clamp: SamplerHandle::INVALID,
            linear_repeat: SamplerHandle::INVALID,
            linear_clamp: SamplerHandle::INVALID,
            trilinear_repeat: SamplerHandle::INVALID,
            trilinear_clamp: SamplerHandle::INVALID,
            aniso4x: SamplerHandle::INVALID,
            aniso8x: SamplerHandle::INVALID,
            aniso16x: SamplerHandle::INVALID,
            shadow: SamplerHandle::INVALID,
        }
    }
}

impl SamplerCache {
    /// Create a new cache.
    pub fn new() -> Self {
        let mut cache = Self {
            cache: BTreeMap::new(),
            samplers: Vec::new(),
            common: CommonSamplers::default(),
        };

        // Create common samplers
        cache.create_common_samplers();
        cache
    }

    /// Create common samplers.
    fn create_common_samplers(&mut self) {
        self.common.point_repeat = self.get_or_create(SamplerDesc::point());
        self.common.point_clamp =
            self.get_or_create(SamplerDesc::point().address_mode(AddressMode::ClampToEdge));
        self.common.linear_repeat = self.get_or_create(SamplerDesc::bilinear());
        self.common.linear_clamp =
            self.get_or_create(SamplerDesc::bilinear().address_mode(AddressMode::ClampToEdge));
        self.common.trilinear_repeat = self.get_or_create(SamplerDesc::trilinear());
        self.common.trilinear_clamp =
            self.get_or_create(SamplerDesc::trilinear().address_mode(AddressMode::ClampToEdge));
        self.common.aniso4x = self.get_or_create(SamplerDesc::anisotropic(4));
        self.common.aniso8x = self.get_or_create(SamplerDesc::anisotropic(8));
        self.common.aniso16x = self.get_or_create(SamplerDesc::anisotropic(16));
        self.common.shadow = self.get_or_create(SamplerDesc::shadow());
    }

    /// Get or create a sampler.
    pub fn get_or_create(&mut self, desc: SamplerDesc) -> SamplerHandle {
        let hash = desc.hash();

        if let Some(&handle) = self.cache.get(&hash) {
            return handle;
        }

        let handle = SamplerHandle::new(self.samplers.len() as u32);
        let sampler = Sampler::new(handle, desc);
        self.samplers.push(sampler);
        self.cache.insert(hash, handle);

        handle
    }

    /// Get sampler.
    pub fn get(&self, handle: SamplerHandle) -> Option<&Sampler> {
        self.samplers.get(handle.index as usize)
    }

    /// Get mutable sampler.
    pub fn get_mut(&mut self, handle: SamplerHandle) -> Option<&mut Sampler> {
        self.samplers.get_mut(handle.index as usize)
    }

    /// Get common samplers.
    pub fn common(&self) -> &CommonSamplers {
        &self.common
    }

    /// Get sampler count.
    pub fn count(&self) -> usize {
        self.samplers.len()
    }

    /// Iterate over all samplers.
    pub fn iter(&self) -> impl Iterator<Item = &Sampler> {
        self.samplers.iter()
    }
}

impl Default for SamplerCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Static Sampler
// ============================================================================

/// Static sampler descriptor for pipeline layouts.
#[derive(Debug, Clone)]
pub struct StaticSampler {
    /// Register/binding.
    pub register: u32,
    /// Register space/set.
    pub space: u32,
    /// Shader visibility.
    pub visibility: ShaderVisibility,
    /// Sampler descriptor.
    pub desc: SamplerDesc,
}

/// Shader visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShaderVisibility {
    /// All shader stages.
    #[default]
    All,
    /// Vertex shader only.
    Vertex,
    /// Fragment shader only.
    Fragment,
    /// Compute shader only.
    Compute,
    /// Geometry shader only.
    Geometry,
    /// Tessellation control only.
    TessControl,
    /// Tessellation evaluation only.
    TessEval,
}

impl StaticSampler {
    /// Create a new static sampler.
    pub fn new(register: u32, space: u32, desc: SamplerDesc) -> Self {
        Self {
            register,
            space,
            visibility: ShaderVisibility::All,
            desc,
        }
    }

    /// Set visibility.
    pub fn visibility(mut self, visibility: ShaderVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Create common static samplers.
    pub fn common_set() -> Vec<StaticSampler> {
        vec![
            Self::new(0, 0, SamplerDesc::point()),
            Self::new(1, 0, SamplerDesc::bilinear()),
            Self::new(2, 0, SamplerDesc::trilinear()),
            Self::new(3, 0, SamplerDesc::anisotropic(8)),
            Self::new(4, 0, SamplerDesc::shadow()),
            Self::new(
                5,
                0,
                SamplerDesc::trilinear().address_mode(AddressMode::ClampToEdge),
            ),
        ]
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
