//! Sampler Management
//!
//! Texture sampling state configuration.

use alloc::{string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use lumina_core::Handle;

// ============================================================================
// Filter Mode
// ============================================================================

/// Texture filter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterMode {
    /// Nearest-neighbor filtering.
    Nearest,
    /// Linear (bilinear/trilinear) filtering.
    Linear,
}

impl Default for FilterMode {
    fn default() -> Self {
        FilterMode::Linear
    }
}

// ============================================================================
// Address Mode
// ============================================================================

/// Texture address mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressMode {
    /// Repeat the texture.
    Repeat,
    /// Mirror and repeat.
    MirrorRepeat,
    /// Clamp to edge.
    ClampToEdge,
    /// Clamp to border color.
    ClampToBorder,
    /// Mirror once then clamp.
    MirrorClampToEdge,
}

impl Default for AddressMode {
    fn default() -> Self {
        AddressMode::Repeat
    }
}

// ============================================================================
// Compare Operation
// ============================================================================

/// Comparison operation for depth sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompareOp {
    /// Never pass.
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

impl Default for CompareOp {
    fn default() -> Self {
        CompareOp::Never
    }
}

// ============================================================================
// Border Color
// ============================================================================

/// Border color for clamped sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorderColor {
    /// Transparent black (0, 0, 0, 0).
    TransparentBlack,
    /// Opaque black (0, 0, 0, 1).
    OpaqueBlack,
    /// Opaque white (1, 1, 1, 1).
    OpaqueWhite,
}

impl Default for BorderColor {
    fn default() -> Self {
        BorderColor::TransparentBlack
    }
}

// ============================================================================
// Sampler Description
// ============================================================================

/// Description for sampler creation.
#[derive(Debug, Clone)]
pub struct SamplerDesc {
    /// Minification filter.
    pub min_filter: FilterMode,
    /// Magnification filter.
    pub mag_filter: FilterMode,
    /// Mipmap filter.
    pub mipmap_filter: FilterMode,
    /// Address mode for U coordinate.
    pub address_mode_u: AddressMode,
    /// Address mode for V coordinate.
    pub address_mode_v: AddressMode,
    /// Address mode for W coordinate.
    pub address_mode_w: AddressMode,
    /// LOD bias.
    pub lod_bias: f32,
    /// Minimum LOD.
    pub lod_min: f32,
    /// Maximum LOD.
    pub lod_max: f32,
    /// Enable anisotropic filtering.
    pub anisotropy_enable: bool,
    /// Maximum anisotropy.
    pub max_anisotropy: f32,
    /// Enable comparison.
    pub compare_enable: bool,
    /// Comparison operation.
    pub compare_op: CompareOp,
    /// Border color.
    pub border_color: BorderColor,
    /// Use unnormalized coordinates.
    pub unnormalized_coordinates: bool,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for SamplerDesc {
    fn default() -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            lod_bias: 0.0,
            lod_min: 0.0,
            lod_max: 1000.0,
            anisotropy_enable: false,
            max_anisotropy: 1.0,
            compare_enable: false,
            compare_op: CompareOp::Never,
            border_color: BorderColor::TransparentBlack,
            unnormalized_coordinates: false,
            label: None,
        }
    }
}

impl SamplerDesc {
    /// Create a linear sampler.
    pub fn linear() -> Self {
        Self::default()
    }

    /// Create a nearest sampler.
    pub fn nearest() -> Self {
        Self {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        }
    }

    /// Create an anisotropic sampler.
    pub fn anisotropic(max_anisotropy: f32) -> Self {
        Self {
            anisotropy_enable: true,
            max_anisotropy,
            ..Default::default()
        }
    }

    /// Create a shadow sampler.
    pub fn shadow(compare_op: CompareOp) -> Self {
        Self {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            compare_enable: true,
            compare_op,
            ..Default::default()
        }
    }

    /// Set address mode for all axes.
    pub fn with_address_mode(mut self, mode: AddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
        self
    }

    /// Set label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

// ============================================================================
// Sampler Handle
// ============================================================================

/// Handle to a sampler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerHandle(Handle<Sampler>);

impl SamplerHandle {
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
// Sampler
// ============================================================================

/// A texture sampler.
pub struct Sampler {
    /// Handle.
    pub handle: SamplerHandle,
    /// Minification filter.
    pub min_filter: FilterMode,
    /// Magnification filter.
    pub mag_filter: FilterMode,
    /// Mipmap filter.
    pub mipmap_filter: FilterMode,
    /// Address mode U.
    pub address_mode_u: AddressMode,
    /// Address mode V.
    pub address_mode_v: AddressMode,
    /// Address mode W.
    pub address_mode_w: AddressMode,
    /// LOD bias.
    pub lod_bias: f32,
    /// Maximum anisotropy.
    pub max_anisotropy: f32,
    /// Compare operation.
    pub compare_op: Option<CompareOp>,
    /// Debug label.
    pub label: Option<String>,
}

impl Sampler {
    /// Create a new sampler.
    pub fn new(handle: SamplerHandle, desc: &SamplerDesc) -> Self {
        Self {
            handle,
            min_filter: desc.min_filter,
            mag_filter: desc.mag_filter,
            mipmap_filter: desc.mipmap_filter,
            address_mode_u: desc.address_mode_u,
            address_mode_v: desc.address_mode_v,
            address_mode_w: desc.address_mode_w,
            lod_bias: desc.lod_bias,
            max_anisotropy: if desc.anisotropy_enable {
                desc.max_anisotropy
            } else {
                1.0
            },
            compare_op: if desc.compare_enable {
                Some(desc.compare_op)
            } else {
                None
            },
            label: desc.label.clone(),
        }
    }

    /// Check if sampler uses comparison.
    pub fn is_comparison(&self) -> bool {
        self.compare_op.is_some()
    }

    /// Check if sampler uses anisotropy.
    pub fn uses_anisotropy(&self) -> bool {
        self.max_anisotropy > 1.0
    }
}

// ============================================================================
// Sampler Manager
// ============================================================================

/// Manages sampler resources.
pub struct SamplerManager {
    /// Samplers.
    samplers: Vec<Option<Sampler>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Sampler count.
    sampler_count: AtomicU32,
}

impl SamplerManager {
    /// Create a new sampler manager.
    pub fn new() -> Self {
        Self {
            samplers: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            sampler_count: AtomicU32::new(0),
        }
    }

    /// Create a sampler.
    pub fn create(&mut self, desc: &SamplerDesc) -> SamplerHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.samplers.len() as u32;
            self.samplers.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = SamplerHandle::new(index, generation);
        let sampler = Sampler::new(handle, desc);

        self.samplers[index as usize] = Some(sampler);
        self.sampler_count.fetch_add(1, Ordering::Relaxed);

        handle
    }

    /// Get a sampler.
    pub fn get(&self, handle: SamplerHandle) -> Option<&Sampler> {
        let index = handle.index() as usize;
        if index >= self.samplers.len() {
            return None;
        }
        self.samplers[index].as_ref()
    }

    /// Destroy a sampler.
    pub fn destroy(&mut self, handle: SamplerHandle) {
        let index = handle.index() as usize;
        if index >= self.samplers.len() {
            return;
        }

        if self.samplers[index].take().is_some() {
            self.sampler_count.fetch_sub(1, Ordering::Relaxed);
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);
    }

    /// Get sampler count.
    pub fn count(&self) -> u32 {
        self.sampler_count.load(Ordering::Relaxed)
    }
}

impl Default for SamplerManager {
    fn default() -> Self {
        Self::new()
    }
}
