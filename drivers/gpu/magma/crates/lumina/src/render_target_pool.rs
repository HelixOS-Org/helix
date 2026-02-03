//! Render target pool and management
//!
//! This module provides types for render target allocation and pooling.

use core::num::NonZeroU32;

/// Render target handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderTargetHandle(pub NonZeroU32);

impl RenderTargetHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Render target description
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct RenderTargetDesc {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: RenderTargetFormat,
    /// Sample count
    pub samples: SampleCount,
    /// Usage flags
    pub usage: RenderTargetUsageFlags,
    /// Array layers
    pub array_layers: u32,
    /// Mip levels
    pub mip_levels: u32,
}

impl RenderTargetDesc {
    /// Creates a color render target
    pub const fn color(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: RenderTargetFormat::RGBA8,
            samples: SampleCount::S1,
            usage: RenderTargetUsageFlags::COLOR_ATTACHMENT,
            array_layers: 1,
            mip_levels: 1,
        }
    }

    /// Creates a depth render target
    pub const fn depth(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: RenderTargetFormat::D32F,
            samples: SampleCount::S1,
            usage: RenderTargetUsageFlags::DEPTH_ATTACHMENT,
            array_layers: 1,
            mip_levels: 1,
        }
    }

    /// Creates a depth-stencil render target
    pub const fn depth_stencil(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: RenderTargetFormat::D24S8,
            samples: SampleCount::S1,
            usage: RenderTargetUsageFlags::DEPTH_ATTACHMENT,
            array_layers: 1,
            mip_levels: 1,
        }
    }

    /// Creates an HDR render target
    pub const fn hdr(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: RenderTargetFormat::RGBA16F,
            samples: SampleCount::S1,
            usage: RenderTargetUsageFlags::COLOR_ATTACHMENT,
            array_layers: 1,
            mip_levels: 1,
        }
    }

    /// Creates a shadow map
    pub const fn shadow_map(size: u32) -> Self {
        Self {
            width: size,
            height: size,
            format: RenderTargetFormat::D32F,
            samples: SampleCount::S1,
            usage: RenderTargetUsageFlags::SHADOW_MAP,
            array_layers: 1,
            mip_levels: 1,
        }
    }

    /// Creates a shadow cube map
    pub const fn shadow_cube(size: u32) -> Self {
        Self {
            width: size,
            height: size,
            format: RenderTargetFormat::D32F,
            samples: SampleCount::S1,
            usage: RenderTargetUsageFlags::SHADOW_MAP,
            array_layers: 6,
            mip_levels: 1,
        }
    }

    /// With MSAA
    pub const fn with_msaa(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// With array layers
    pub const fn with_layers(mut self, layers: u32) -> Self {
        self.array_layers = layers;
        self
    }

    /// With mip levels
    pub const fn with_mips(mut self, mips: u32) -> Self {
        self.mip_levels = mips;
        self
    }

    /// With sampling support
    pub const fn with_sampling(mut self) -> Self {
        self.usage = self.usage.union(RenderTargetUsageFlags::SAMPLED);
        self
    }

    /// With storage support
    pub const fn with_storage(mut self) -> Self {
        self.usage = self.usage.union(RenderTargetUsageFlags::STORAGE);
        self
    }

    /// Total size in bytes (approximate)
    pub const fn size_bytes(&self) -> u64 {
        let pixel_size = self.format.bytes_per_pixel() as u64;
        let samples = self.samples.count() as u64;
        let layers = self.array_layers as u64;

        let mut total = 0u64;
        let mut w = self.width as u64;
        let mut h = self.height as u64;

        let mut mip = 0;
        while mip < self.mip_levels {
            total += w * h * pixel_size * samples * layers;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            mip += 1;
        }

        total
    }
}

/// Render target format
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum RenderTargetFormat {
    /// R8 normalized
    R8         = 9,
    /// RG8 normalized
    RG8        = 16,
    /// RGBA8 normalized
    #[default]
    RGBA8      = 37,
    /// RGBA8 sRGB
    RGBA8_SRGB = 43,
    /// BGRA8 normalized
    BGRA8      = 44,
    /// BGRA8 sRGB
    BGRA8_SRGB = 50,
    /// RGB10A2 normalized
    RGB10A2    = 64,
    /// R16F
    R16F       = 76,
    /// RG16F
    RG16F      = 83,
    /// RGBA16F
    RGBA16F    = 97,
    /// R32F
    R32F       = 100,
    /// RG32F
    RG32F      = 103,
    /// RGBA32F
    RGBA32F    = 109,
    /// R32UI
    R32UI      = 98,
    /// RG32UI
    RG32UI     = 101,
    /// RGBA32UI
    RGBA32UI   = 107,
    /// D16
    D16        = 124,
    /// D32F
    D32F       = 126,
    /// D24S8
    D24S8      = 129,
    /// D32FS8
    D32FS8     = 130,
}

impl RenderTargetFormat {
    /// Is this a depth format
    pub const fn is_depth(self) -> bool {
        matches!(self, Self::D16 | Self::D32F | Self::D24S8 | Self::D32FS8)
    }

    /// Is this a depth-stencil format
    pub const fn is_depth_stencil(self) -> bool {
        matches!(self, Self::D24S8 | Self::D32FS8)
    }

    /// Is this a color format
    pub const fn is_color(self) -> bool {
        !self.is_depth()
    }

    /// Is this an sRGB format
    pub const fn is_srgb(self) -> bool {
        matches!(self, Self::RGBA8_SRGB | Self::BGRA8_SRGB)
    }

    /// Is this an HDR format
    pub const fn is_hdr(self) -> bool {
        matches!(
            self,
            Self::R16F
                | Self::RG16F
                | Self::RGBA16F
                | Self::R32F
                | Self::RG32F
                | Self::RGBA32F
                | Self::RGB10A2
        )
    }

    /// Bytes per pixel
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::R8 => 1,
            Self::RG8 | Self::R16F | Self::D16 => 2,
            Self::RGBA8
            | Self::RGBA8_SRGB
            | Self::BGRA8
            | Self::BGRA8_SRGB
            | Self::RGB10A2
            | Self::RG16F
            | Self::R32F
            | Self::R32UI
            | Self::D32F
            | Self::D24S8 => 4,
            Self::RGBA16F | Self::RG32F | Self::RG32UI | Self::D32FS8 => 8,
            Self::RGBA32F | Self::RGBA32UI => 16,
        }
    }

    /// Get sRGB version
    pub const fn to_srgb(self) -> Self {
        match self {
            Self::RGBA8 => Self::RGBA8_SRGB,
            Self::BGRA8 => Self::BGRA8_SRGB,
            _ => self,
        }
    }

    /// Get linear version
    pub const fn to_linear(self) -> Self {
        match self {
            Self::RGBA8_SRGB => Self::RGBA8,
            Self::BGRA8_SRGB => Self::BGRA8,
            _ => self,
        }
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    S1  = 1,
    /// 2 samples
    S2  = 2,
    /// 4 samples
    S4  = 4,
    /// 8 samples
    S8  = 8,
    /// 16 samples
    S16 = 16,
    /// 32 samples
    S32 = 32,
    /// 64 samples
    S64 = 64,
}

impl SampleCount {
    /// Sample count as u32
    pub const fn count(self) -> u32 {
        self as u32
    }

    /// Is MSAA enabled
    pub const fn is_msaa(self) -> bool {
        self.count() > 1
    }

    /// From count
    pub const fn from_count(count: u32) -> Option<Self> {
        match count {
            1 => Some(Self::S1),
            2 => Some(Self::S2),
            4 => Some(Self::S4),
            8 => Some(Self::S8),
            16 => Some(Self::S16),
            32 => Some(Self::S32),
            64 => Some(Self::S64),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// Render target usage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RenderTargetUsageFlags: u32 {
        /// Color attachment
        const COLOR_ATTACHMENT = 1 << 0;
        /// Depth attachment
        const DEPTH_ATTACHMENT = 1 << 1;
        /// Can be sampled
        const SAMPLED = 1 << 2;
        /// Storage image
        const STORAGE = 1 << 3;
        /// Transfer source
        const TRANSFER_SRC = 1 << 4;
        /// Transfer destination
        const TRANSFER_DST = 1 << 5;
        /// Input attachment
        const INPUT_ATTACHMENT = 1 << 6;
        /// Transient attachment
        const TRANSIENT = 1 << 7;
    }
}

impl RenderTargetUsageFlags {
    /// Shadow map usage
    pub const SHADOW_MAP: Self =
        Self::from_bits_truncate(Self::DEPTH_ATTACHMENT.bits() | Self::SAMPLED.bits());

    /// Post-processing target
    pub const POST_PROCESS: Self =
        Self::from_bits_truncate(Self::COLOR_ATTACHMENT.bits() | Self::SAMPLED.bits());

    /// G-buffer target
    pub const GBUFFER: Self = Self::from_bits_truncate(
        Self::COLOR_ATTACHMENT.bits() | Self::SAMPLED.bits() | Self::INPUT_ATTACHMENT.bits(),
    );
}

/// Render target pool configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderTargetPoolConfig {
    /// Maximum memory budget (bytes)
    pub max_memory: u64,
    /// Maximum targets
    pub max_targets: u32,
    /// Target retention frames (how long to keep unused targets)
    pub retention_frames: u32,
    /// Enable size rounding
    pub round_sizes: bool,
    /// Size rounding granularity
    pub size_granularity: u32,
}

impl RenderTargetPoolConfig {
    /// Default configuration (256 MB budget)
    pub const fn default() -> Self {
        Self {
            max_memory: 256 * 1024 * 1024,
            max_targets: 64,
            retention_frames: 3,
            round_sizes: true,
            size_granularity: 64,
        }
    }

    /// Low memory configuration (64 MB budget)
    pub const fn low_memory() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024,
            max_targets: 16,
            retention_frames: 1,
            round_sizes: true,
            size_granularity: 32,
        }
    }

    /// High quality configuration (512 MB budget)
    pub const fn high_quality() -> Self {
        Self {
            max_memory: 512 * 1024 * 1024,
            max_targets: 128,
            retention_frames: 5,
            round_sizes: true,
            size_granularity: 64,
        }
    }

    /// With custom memory budget
    pub const fn with_memory(mut self, bytes: u64) -> Self {
        self.max_memory = bytes;
        self
    }
}

impl Default for RenderTargetPoolConfig {
    fn default() -> Self {
        Self::default()
    }
}

/// Render target allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderTargetAlloc {
    /// Description
    pub desc: RenderTargetDesc,
    /// Clear value (optional)
    pub clear_value: ClearValue,
    /// Name hint (for debugging)
    pub name_hint: u32,
}

impl RenderTargetAlloc {
    /// Creates allocation request
    pub const fn new(desc: RenderTargetDesc) -> Self {
        Self {
            desc,
            clear_value: ClearValue::COLOR_BLACK,
            name_hint: 0,
        }
    }

    /// With clear value
    pub const fn with_clear(mut self, value: ClearValue) -> Self {
        self.clear_value = value;
        self
    }

    /// With name hint
    pub const fn with_name(mut self, name: u32) -> Self {
        self.name_hint = name;
        self
    }
}

/// Clear value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union ClearValue {
    /// Color clear value
    pub color: ClearColorValue,
    /// Depth-stencil clear value
    pub depth_stencil: ClearDepthStencilValue,
}

impl ClearValue {
    /// Black color
    pub const COLOR_BLACK: Self = Self {
        color: ClearColorValue::ZERO,
    };

    /// White color
    pub const COLOR_WHITE: Self = Self {
        color: ClearColorValue::ONE,
    };

    /// Default depth clear
    pub const DEPTH_ONE: Self = Self {
        depth_stencil: ClearDepthStencilValue::DEFAULT,
    };

    /// Zero depth clear
    pub const DEPTH_ZERO: Self = Self {
        depth_stencil: ClearDepthStencilValue::ZERO,
    };

    /// Creates a color clear value
    pub const fn color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: ClearColorValue { r, g, b, a },
        }
    }

    /// Creates a depth clear value
    pub const fn depth(depth: f32, stencil: u32) -> Self {
        Self {
            depth_stencil: ClearDepthStencilValue { depth, stencil },
        }
    }
}

/// Clear color value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearColorValue {
    /// Red
    pub r: f32,
    /// Green
    pub g: f32,
    /// Blue
    pub b: f32,
    /// Alpha
    pub a: f32,
}

impl ClearColorValue {
    /// Black
    pub const ZERO: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// White
    pub const ONE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Red
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    /// Green
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };

    /// Blue
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
}

/// Clear depth-stencil value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearDepthStencilValue {
    /// Depth value
    pub depth: f32,
    /// Stencil value
    pub stencil: u32,
}

impl ClearDepthStencilValue {
    /// Default (depth 1.0, stencil 0)
    pub const DEFAULT: Self = Self {
        depth: 1.0,
        stencil: 0,
    };

    /// Zero
    pub const ZERO: Self = Self {
        depth: 0.0,
        stencil: 0,
    };
}

/// Render target pool statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RenderTargetPoolStats {
    /// Total allocated memory
    pub allocated_memory: u64,
    /// Total targets allocated
    pub total_targets: u32,
    /// Targets in use
    pub targets_in_use: u32,
    /// Cache hits this frame
    pub cache_hits: u32,
    /// Cache misses this frame
    pub cache_misses: u32,
    /// Targets recycled this frame
    pub targets_recycled: u32,
    /// Targets evicted this frame
    pub targets_evicted: u32,
}

impl RenderTargetPoolStats {
    /// Cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }

    /// Memory utilization
    pub fn memory_utilization(&self, budget: u64) -> f32 {
        if budget == 0 {
            0.0
        } else {
            self.allocated_memory as f32 / budget as f32
        }
    }
}

/// Transient render target
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TransientRenderTarget {
    /// Handle
    pub handle: RenderTargetHandle,
    /// Description
    pub desc: RenderTargetDesc,
    /// First use pass index
    pub first_use: u32,
    /// Last use pass index
    pub last_use: u32,
}

impl TransientRenderTarget {
    /// Creates a transient target
    pub const fn new(handle: RenderTargetHandle, desc: RenderTargetDesc) -> Self {
        Self {
            handle,
            desc,
            first_use: 0,
            last_use: 0,
        }
    }

    /// With usage range
    pub const fn with_usage(mut self, first: u32, last: u32) -> Self {
        self.first_use = first;
        self.last_use = last;
        self
    }

    /// Lifetime in passes
    pub const fn lifetime(&self) -> u32 {
        self.last_use - self.first_use + 1
    }
}

/// Render target aliasing info
#[derive(Clone, Debug)]
pub struct RenderTargetAliasInfo {
    /// Targets that can share memory
    pub aliases: alloc::vec::Vec<(RenderTargetHandle, RenderTargetHandle)>,
    /// Memory saved by aliasing
    pub memory_saved: u64,
}

use alloc::vec::Vec;

impl RenderTargetAliasInfo {
    /// Creates empty aliasing info
    pub fn new() -> Self {
        Self {
            aliases: Vec::new(),
            memory_saved: 0,
        }
    }

    /// Adds an alias
    pub fn add_alias(&mut self, a: RenderTargetHandle, b: RenderTargetHandle, saved: u64) {
        self.aliases.push((a, b));
        self.memory_saved += saved;
    }
}

impl Default for RenderTargetAliasInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Image view handle for render targets
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderTargetViewHandle(pub NonZeroU32);

impl RenderTargetViewHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Render target view description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderTargetViewDesc {
    /// Target
    pub target: RenderTargetHandle,
    /// First mip level
    pub base_mip_level: u32,
    /// Mip level count
    pub mip_level_count: u32,
    /// First array layer
    pub base_array_layer: u32,
    /// Array layer count
    pub array_layer_count: u32,
    /// Format override (0 = use target format)
    pub format_override: u32,
}

impl RenderTargetViewDesc {
    /// Whole target view
    pub const fn whole(target: RenderTargetHandle) -> Self {
        Self {
            target,
            base_mip_level: 0,
            mip_level_count: 1,
            base_array_layer: 0,
            array_layer_count: 1,
            format_override: 0,
        }
    }

    /// Single mip level
    pub const fn mip(target: RenderTargetHandle, mip: u32) -> Self {
        Self {
            target,
            base_mip_level: mip,
            mip_level_count: 1,
            base_array_layer: 0,
            array_layer_count: 1,
            format_override: 0,
        }
    }

    /// Single array layer
    pub const fn layer(target: RenderTargetHandle, layer: u32) -> Self {
        Self {
            target,
            base_mip_level: 0,
            mip_level_count: 1,
            base_array_layer: layer,
            array_layer_count: 1,
            format_override: 0,
        }
    }

    /// Cube face
    pub const fn cube_face(target: RenderTargetHandle, face: u32) -> Self {
        Self::layer(target, face)
    }
}
