//! Hybrid Rendering Types for Lumina
//!
//! This module provides hybrid rasterization and ray tracing
//! infrastructure for mixed rendering pipelines.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Hybrid Rendering Handles
// ============================================================================

/// Hybrid renderer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HybridRendererHandle(pub u64);

impl HybridRendererHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HybridRendererHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Ray budget handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RayBudgetHandle(pub u64);

impl RayBudgetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for RayBudgetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Visibility buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VisibilityBufferHandle(pub u64);

impl VisibilityBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for VisibilityBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Hybrid Renderer Creation
// ============================================================================

/// Hybrid renderer create info
#[derive(Clone, Debug)]
pub struct HybridRendererCreateInfo {
    /// Name
    pub name: String,
    /// Rendering mode
    pub mode: HybridRenderMode,
    /// Primary renderer
    pub primary_renderer: PrimaryRenderer,
    /// Ray tracing features
    pub rt_features: RayTracingFeatures,
    /// Rasterization features
    pub raster_features: RasterFeatures,
    /// Quality settings
    pub quality: HybridQuality,
    /// Ray budget
    pub ray_budget: RayBudgetConfig,
    /// Fallback mode
    pub fallback: FallbackMode,
}

impl HybridRendererCreateInfo {
    /// Creates new info
    pub fn new(mode: HybridRenderMode) -> Self {
        Self {
            name: String::new(),
            mode,
            primary_renderer: PrimaryRenderer::Rasterization,
            rt_features: RayTracingFeatures::empty(),
            raster_features: RasterFeatures::all(),
            quality: HybridQuality::default(),
            ray_budget: RayBudgetConfig::default(),
            fallback: FallbackMode::Rasterization,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With primary renderer
    pub fn with_primary(mut self, primary: PrimaryRenderer) -> Self {
        self.primary_renderer = primary;
        self
    }

    /// With ray tracing features
    pub fn with_rt_features(mut self, features: RayTracingFeatures) -> Self {
        self.rt_features |= features;
        self
    }

    /// With raster features
    pub fn with_raster_features(mut self, features: RasterFeatures) -> Self {
        self.raster_features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: HybridQuality) -> Self {
        self.quality = quality;
        self
    }

    /// With ray budget
    pub fn with_ray_budget(mut self, budget: RayBudgetConfig) -> Self {
        self.ray_budget = budget;
        self
    }

    /// Rasterization with RT shadows
    pub fn rt_shadows() -> Self {
        Self::new(HybridRenderMode::RtShadows).with_rt_features(RayTracingFeatures::SHADOWS)
    }

    /// Rasterization with RT reflections
    pub fn rt_reflections() -> Self {
        Self::new(HybridRenderMode::RtReflections).with_rt_features(RayTracingFeatures::REFLECTIONS)
    }

    /// Rasterization with RT GI
    pub fn rt_gi() -> Self {
        Self::new(HybridRenderMode::RtGlobalIllumination)
            .with_rt_features(RayTracingFeatures::GLOBAL_ILLUMINATION)
    }

    /// Rasterization with RT AO
    pub fn rt_ao() -> Self {
        Self::new(HybridRenderMode::RtAmbientOcclusion)
            .with_rt_features(RayTracingFeatures::AMBIENT_OCCLUSION)
    }

    /// Full hybrid preset
    pub fn full_hybrid() -> Self {
        Self::new(HybridRenderMode::FullHybrid).with_rt_features(RayTracingFeatures::all())
    }

    /// Performance preset
    pub fn performance() -> Self {
        Self::new(HybridRenderMode::RtShadows)
            .with_quality(HybridQuality::low())
            .with_ray_budget(RayBudgetConfig::limited(0.5))
    }

    /// Quality preset
    pub fn quality() -> Self {
        Self::new(HybridRenderMode::FullHybrid)
            .with_quality(HybridQuality::high())
            .with_ray_budget(RayBudgetConfig::unlimited())
    }
}

impl Default for HybridRendererCreateInfo {
    fn default() -> Self {
        Self::rt_shadows()
    }
}

/// Hybrid render mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HybridRenderMode {
    /// RT shadows only
    #[default]
    RtShadows            = 0,
    /// RT reflections only
    RtReflections        = 1,
    /// RT global illumination
    RtGlobalIllumination = 2,
    /// RT ambient occlusion
    RtAmbientOcclusion   = 3,
    /// RT shadows + reflections
    RtShadowsReflections = 4,
    /// Full hybrid (all RT effects)
    FullHybrid           = 5,
    /// Path traced (primary rays RT)
    PathTraced           = 6,
    /// Visibility buffer + RT
    VisibilityBufferHybrid = 7,
}

impl HybridRenderMode {
    /// Requires ray tracing hardware
    pub const fn requires_rt(&self) -> bool {
        true // All hybrid modes need RT
    }

    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::RtShadows => "RT Shadows",
            Self::RtReflections => "RT Reflections",
            Self::RtGlobalIllumination => "RT GI",
            Self::RtAmbientOcclusion => "RT AO",
            Self::RtShadowsReflections => "RT Shadows + Reflections",
            Self::FullHybrid => "Full Hybrid",
            Self::PathTraced => "Path Traced",
            Self::VisibilityBufferHybrid => "Visibility Buffer Hybrid",
        }
    }
}

/// Primary renderer type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PrimaryRenderer {
    /// Rasterization
    #[default]
    Rasterization    = 0,
    /// Ray tracing
    RayTracing       = 1,
    /// Visibility buffer
    VisibilityBuffer = 2,
    /// Software rasterization
    SoftwareRaster   = 3,
}

bitflags::bitflags! {
    /// Ray tracing features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct RayTracingFeatures: u32 {
        /// None
        const NONE = 0;
        /// RT shadows
        const SHADOWS = 1 << 0;
        /// RT reflections
        const REFLECTIONS = 1 << 1;
        /// RT global illumination
        const GLOBAL_ILLUMINATION = 1 << 2;
        /// RT ambient occlusion
        const AMBIENT_OCCLUSION = 1 << 3;
        /// RT refractions
        const REFRACTIONS = 1 << 4;
        /// RT caustics
        const CAUSTICS = 1 << 5;
        /// RT subsurface scattering
        const SUBSURFACE = 1 << 6;
        /// RT translucency
        const TRANSLUCENCY = 1 << 7;
    }
}

bitflags::bitflags! {
    /// Rasterization features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct RasterFeatures: u32 {
        /// None
        const NONE = 0;
        /// G-Buffer generation
        const GBUFFER = 1 << 0;
        /// Deferred shading
        const DEFERRED = 1 << 1;
        /// Forward shading
        const FORWARD = 1 << 2;
        /// Screen-space effects
        const SCREEN_SPACE = 1 << 3;
        /// Post-processing
        const POST_PROCESS = 1 << 4;
        /// MSAA
        const MSAA = 1 << 5;
        /// VRS
        const VRS = 1 << 6;
    }
}

/// Fallback mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FallbackMode {
    /// Fall back to rasterization
    #[default]
    Rasterization = 0,
    /// Fall back to screen-space
    ScreenSpace   = 1,
    /// Disable feature
    Disable       = 2,
    /// Reduce quality
    ReduceQuality = 3,
}

// ============================================================================
// Hybrid Quality
// ============================================================================

/// Hybrid quality settings
#[derive(Clone, Debug)]
pub struct HybridQuality {
    /// Shadow ray samples
    pub shadow_samples: u32,
    /// Reflection ray samples
    pub reflection_samples: u32,
    /// GI ray samples
    pub gi_samples: u32,
    /// AO ray samples
    pub ao_samples: u32,
    /// Max ray bounces
    pub max_bounces: u32,
    /// Resolution scale for RT
    pub rt_resolution_scale: f32,
    /// Use temporal accumulation
    pub temporal_accumulation: bool,
    /// Use denoising
    pub denoising: bool,
}

impl HybridQuality {
    /// Creates new quality settings
    pub fn new() -> Self {
        Self {
            shadow_samples: 1,
            reflection_samples: 1,
            gi_samples: 1,
            ao_samples: 1,
            max_bounces: 1,
            rt_resolution_scale: 1.0,
            temporal_accumulation: true,
            denoising: true,
        }
    }

    /// Low quality
    pub fn low() -> Self {
        Self {
            shadow_samples: 1,
            reflection_samples: 1,
            gi_samples: 1,
            ao_samples: 1,
            max_bounces: 1,
            rt_resolution_scale: 0.5,
            temporal_accumulation: true,
            denoising: true,
        }
    }

    /// Medium quality
    pub fn medium() -> Self {
        Self {
            shadow_samples: 1,
            reflection_samples: 1,
            gi_samples: 2,
            ao_samples: 2,
            max_bounces: 2,
            rt_resolution_scale: 0.75,
            temporal_accumulation: true,
            denoising: true,
        }
    }

    /// High quality
    pub fn high() -> Self {
        Self {
            shadow_samples: 2,
            reflection_samples: 2,
            gi_samples: 4,
            ao_samples: 4,
            max_bounces: 3,
            rt_resolution_scale: 1.0,
            temporal_accumulation: true,
            denoising: true,
        }
    }

    /// Ultra quality
    pub fn ultra() -> Self {
        Self {
            shadow_samples: 4,
            reflection_samples: 4,
            gi_samples: 8,
            ao_samples: 8,
            max_bounces: 4,
            rt_resolution_scale: 1.0,
            temporal_accumulation: true,
            denoising: true,
        }
    }
}

impl Default for HybridQuality {
    fn default() -> Self {
        Self::medium()
    }
}

// ============================================================================
// Ray Budget
// ============================================================================

/// Ray budget configuration
#[derive(Clone, Debug)]
pub struct RayBudgetConfig {
    /// Max rays per frame
    pub max_rays_per_frame: u64,
    /// Budget mode
    pub mode: RayBudgetMode,
    /// Priority for different effects
    pub priorities: RayPriorities,
    /// Target frame time (milliseconds)
    pub target_frame_time_ms: f32,
    /// Allow budget overflow
    pub allow_overflow: bool,
}

impl RayBudgetConfig {
    /// Unlimited rays
    pub fn unlimited() -> Self {
        Self {
            max_rays_per_frame: u64::MAX,
            mode: RayBudgetMode::Unlimited,
            priorities: RayPriorities::default(),
            target_frame_time_ms: 16.67,
            allow_overflow: true,
        }
    }

    /// Limited to ratio of screen pixels
    pub fn limited(ratio: f32) -> Self {
        Self {
            max_rays_per_frame: 0, // Computed from resolution
            mode: RayBudgetMode::ScreenRatio { ratio },
            priorities: RayPriorities::default(),
            target_frame_time_ms: 16.67,
            allow_overflow: false,
        }
    }

    /// Fixed ray count
    pub fn fixed(rays: u64) -> Self {
        Self {
            max_rays_per_frame: rays,
            mode: RayBudgetMode::Fixed,
            priorities: RayPriorities::default(),
            target_frame_time_ms: 16.67,
            allow_overflow: false,
        }
    }

    /// Adaptive based on frame time
    pub fn adaptive(target_ms: f32) -> Self {
        Self {
            max_rays_per_frame: 0,
            mode: RayBudgetMode::Adaptive,
            priorities: RayPriorities::default(),
            target_frame_time_ms: target_ms,
            allow_overflow: false,
        }
    }
}

impl Default for RayBudgetConfig {
    fn default() -> Self {
        Self::limited(1.0)
    }
}

/// Ray budget mode
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum RayBudgetMode {
    /// Unlimited rays
    Unlimited   = 0,
    /// Fixed ray count
    Fixed       = 1,
    /// Ratio of screen pixels
    ScreenRatio { ratio: f32 } = 2,
    /// Adaptive based on performance
    Adaptive    = 3,
}

impl Default for RayBudgetMode {
    fn default() -> Self {
        Self::ScreenRatio { ratio: 1.0 }
    }
}

/// Ray effect priorities
#[derive(Clone, Copy, Debug)]
pub struct RayPriorities {
    /// Shadow priority (0-1)
    pub shadows: f32,
    /// Reflection priority
    pub reflections: f32,
    /// GI priority
    pub gi: f32,
    /// AO priority
    pub ao: f32,
}

impl Default for RayPriorities {
    fn default() -> Self {
        Self {
            shadows: 1.0,
            reflections: 0.8,
            gi: 0.6,
            ao: 0.5,
        }
    }
}

// ============================================================================
// Hybrid Pipeline
// ============================================================================

/// Hybrid render pass
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum HybridRenderPass {
    /// G-Buffer pass (rasterization)
    GBuffer      = 0,
    /// Visibility pass
    Visibility   = 1,
    /// RT shadow pass
    RtShadow     = 2,
    /// RT reflection pass
    RtReflection = 3,
    /// RT GI pass
    RtGi         = 4,
    /// RT AO pass
    RtAo         = 5,
    /// Denoising pass
    Denoise      = 6,
    /// Lighting pass
    Lighting     = 7,
    /// Composition pass
    Composition  = 8,
    /// Post-process pass
    PostProcess  = 9,
}

/// Hybrid pipeline stage
#[derive(Clone, Debug)]
pub struct HybridPipelineStage {
    /// Pass type
    pub pass: HybridRenderPass,
    /// Enabled
    pub enabled: bool,
    /// Resolution scale
    pub resolution_scale: f32,
    /// Dependencies
    pub dependencies: Vec<HybridRenderPass>,
}

impl HybridPipelineStage {
    /// Creates new stage
    pub fn new(pass: HybridRenderPass) -> Self {
        Self {
            pass,
            enabled: true,
            resolution_scale: 1.0,
            dependencies: Vec::new(),
        }
    }

    /// With resolution scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.resolution_scale = scale;
        self
    }

    /// With dependencies
    pub fn with_dependencies(mut self, deps: Vec<HybridRenderPass>) -> Self {
        self.dependencies = deps;
        self
    }

    /// Disabled
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU hybrid render params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuHybridParams {
    /// Resolution
    pub resolution: [u32; 2],
    /// RT resolution
    pub rt_resolution: [u32; 2],
    /// Frame index
    pub frame_index: u32,
    /// Mode flags
    pub mode_flags: u32,
    /// Max bounces
    pub max_bounces: u32,
    /// Ray budget
    pub ray_budget: u32,
    /// Shadow samples
    pub shadow_samples: u32,
    /// Reflection samples
    pub reflection_samples: u32,
    /// GI samples
    pub gi_samples: u32,
    /// AO samples
    pub ao_samples: u32,
    /// Jitter
    pub jitter: [f32; 2],
    /// Padding
    pub _padding: [f32; 2],
}

impl GpuHybridParams {
    /// From create info
    pub fn from_create_info(
        info: &HybridRendererCreateInfo,
        width: u32,
        height: u32,
        frame: u32,
    ) -> Self {
        let rt_width = (width as f32 * info.quality.rt_resolution_scale) as u32;
        let rt_height = (height as f32 * info.quality.rt_resolution_scale) as u32;

        Self {
            resolution: [width, height],
            rt_resolution: [rt_width, rt_height],
            frame_index: frame,
            mode_flags: info.mode as u32,
            max_bounces: info.quality.max_bounces,
            ray_budget: info.ray_budget.max_rays_per_frame as u32,
            shadow_samples: info.quality.shadow_samples,
            reflection_samples: info.quality.reflection_samples,
            gi_samples: info.quality.gi_samples,
            ao_samples: info.quality.ao_samples,
            jitter: [0.0, 0.0],
            _padding: [0.0, 0.0],
        }
    }
}

// ============================================================================
// Visibility Buffer
// ============================================================================

/// Visibility buffer create info
#[derive(Clone, Debug)]
pub struct VisibilityBufferCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: VisibilityFormat,
    /// Features
    pub features: VisibilityFeatures,
}

impl VisibilityBufferCreateInfo {
    /// Creates new info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            format: VisibilityFormat::TriangleId32,
            features: VisibilityFeatures::empty(),
        }
    }

    /// With format
    pub fn with_format(mut self, format: VisibilityFormat) -> Self {
        self.format = format;
        self
    }

    /// With features
    pub fn with_features(mut self, features: VisibilityFeatures) -> Self {
        self.features |= features;
        self
    }
}

impl Default for VisibilityBufferCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// Visibility buffer format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VisibilityFormat {
    /// 32-bit triangle ID
    #[default]
    TriangleId32 = 0,
    /// 64-bit triangle ID + instance
    TriangleId64 = 1,
    /// Triangle ID + barycentrics
    TriangleIdBarycentrics = 2,
}

bitflags::bitflags! {
    /// Visibility buffer features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct VisibilityFeatures: u32 {
        /// None
        const NONE = 0;
        /// Store barycentrics
        const BARYCENTRICS = 1 << 0;
        /// Store derivatives
        const DERIVATIVES = 1 << 1;
        /// Per-pixel velocity
        const VELOCITY = 1 << 2;
        /// Material ID
        const MATERIAL_ID = 1 << 3;
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Hybrid rendering statistics
#[derive(Clone, Debug, Default)]
pub struct HybridRenderStats {
    /// Total frame time (microseconds)
    pub frame_time_us: u64,
    /// Rasterization time
    pub raster_time_us: u64,
    /// Ray tracing time
    pub rt_time_us: u64,
    /// Denoising time
    pub denoise_time_us: u64,
    /// Rays traced
    pub rays_traced: u64,
    /// Ray budget used ratio
    pub budget_used: f32,
    /// Shadow rays
    pub shadow_rays: u64,
    /// Reflection rays
    pub reflection_rays: u64,
    /// GI rays
    pub gi_rays: u64,
    /// AO rays
    pub ao_rays: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl HybridRenderStats {
    /// RT to raster time ratio
    pub fn rt_raster_ratio(&self) -> f32 {
        if self.raster_time_us == 0 {
            return 0.0;
        }
        self.rt_time_us as f32 / self.raster_time_us as f32
    }

    /// Total rays
    pub fn total_rays(&self) -> u64 {
        self.shadow_rays + self.reflection_rays + self.gi_rays + self.ao_rays
    }
}
