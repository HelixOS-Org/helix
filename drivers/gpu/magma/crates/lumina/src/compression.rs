//! Texture Compression Types for Lumina
//!
//! This module provides texture compression infrastructure including
//! BCn compression, ASTC, and runtime texture compression.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Compression Handles
// ============================================================================

/// Compressor handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CompressorHandle(pub u64);

impl CompressorHandle {
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

impl Default for CompressorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Compression task handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CompressionTaskHandle(pub u64);

impl CompressionTaskHandle {
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

impl Default for CompressionTaskHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Block Compression Formats
// ============================================================================

/// Block compression format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlockFormat {
    /// BC1 (DXT1) - RGB, 1-bit alpha
    #[default]
    Bc1 = 0,
    /// BC2 (DXT3) - RGB + explicit alpha
    Bc2 = 1,
    /// BC3 (DXT5) - RGB + interpolated alpha
    Bc3 = 2,
    /// BC4 - Single channel
    Bc4 = 3,
    /// BC5 - Two channels (normals)
    Bc5 = 4,
    /// BC6H - HDR RGB
    Bc6h = 5,
    /// BC6H unsigned
    Bc6hUnsigned = 6,
    /// BC7 - High quality RGBA
    Bc7 = 7,
    /// ETC1
    Etc1 = 8,
    /// ETC2 RGB
    Etc2Rgb = 9,
    /// ETC2 RGBA
    Etc2Rgba = 10,
    /// ASTC 4x4
    Astc4x4 = 11,
    /// ASTC 5x5
    Astc5x5 = 12,
    /// ASTC 6x6
    Astc6x6 = 13,
    /// ASTC 8x8
    Astc8x8 = 14,
    /// ASTC 10x10
    Astc10x10 = 15,
    /// ASTC 12x12
    Astc12x12 = 16,
}

impl BlockFormat {
    /// Block size in pixels
    pub fn block_size(&self) -> (u32, u32) {
        match self {
            Self::Bc1 | Self::Bc2 | Self::Bc3 | Self::Bc4 | Self::Bc5 | Self::Bc6h
            | Self::Bc6hUnsigned | Self::Bc7 | Self::Etc1 | Self::Etc2Rgb | Self::Etc2Rgba => {
                (4, 4)
            }
            Self::Astc4x4 => (4, 4),
            Self::Astc5x5 => (5, 5),
            Self::Astc6x6 => (6, 6),
            Self::Astc8x8 => (8, 8),
            Self::Astc10x10 => (10, 10),
            Self::Astc12x12 => (12, 12),
        }
    }

    /// Bytes per block
    pub fn bytes_per_block(&self) -> u32 {
        match self {
            Self::Bc1 | Self::Bc4 | Self::Etc1 | Self::Etc2Rgb => 8,
            Self::Bc2 | Self::Bc3 | Self::Bc5 | Self::Bc6h | Self::Bc6hUnsigned | Self::Bc7
            | Self::Etc2Rgba | Self::Astc4x4 | Self::Astc5x5 | Self::Astc6x6 | Self::Astc8x8
            | Self::Astc10x10 | Self::Astc12x12 => 16,
        }
    }

    /// Bits per pixel
    pub fn bits_per_pixel(&self) -> f32 {
        let (bw, bh) = self.block_size();
        let pixels = (bw * bh) as f32;
        (self.bytes_per_block() * 8) as f32 / pixels
    }

    /// Has alpha
    pub fn has_alpha(&self) -> bool {
        matches!(
            self,
            Self::Bc2
                | Self::Bc3
                | Self::Bc7
                | Self::Etc2Rgba
                | Self::Astc4x4
                | Self::Astc5x5
                | Self::Astc6x6
                | Self::Astc8x8
                | Self::Astc10x10
                | Self::Astc12x12
        )
    }

    /// Is HDR
    pub fn is_hdr(&self) -> bool {
        matches!(self, Self::Bc6h | Self::Bc6hUnsigned)
    }

    /// Calculate compressed size
    pub fn compressed_size(&self, width: u32, height: u32) -> u64 {
        let (bw, bh) = self.block_size();
        let blocks_x = (width + bw - 1) / bw;
        let blocks_y = (height + bh - 1) / bh;
        blocks_x as u64 * blocks_y as u64 * self.bytes_per_block() as u64
    }

    /// Compression ratio vs RGBA8
    pub fn compression_ratio(&self) -> f32 {
        32.0 / self.bits_per_pixel()
    }
}

// ============================================================================
// Compression Quality
// ============================================================================

/// Compression quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompressionQuality {
    /// Fastest (lowest quality)
    Fastest = 0,
    /// Fast
    Fast = 1,
    /// Normal
    #[default]
    Normal = 2,
    /// Slow (high quality)
    Slow = 3,
    /// Very slow (best quality)
    VerySlow = 4,
}

impl CompressionQuality {
    /// Iterations for this quality
    pub fn iterations(&self) -> u32 {
        match self {
            Self::Fastest => 1,
            Self::Fast => 4,
            Self::Normal => 16,
            Self::Slow => 64,
            Self::VerySlow => 256,
        }
    }

    /// Partition search depth
    pub fn partition_search(&self) -> u32 {
        match self {
            Self::Fastest => 4,
            Self::Fast => 16,
            Self::Normal => 64,
            Self::Slow => 128,
            Self::VerySlow => 256,
        }
    }
}

// ============================================================================
// Compression Settings
// ============================================================================

/// Compression settings
#[derive(Clone, Debug)]
pub struct CompressionSettings {
    /// Output format
    pub format: BlockFormat,
    /// Quality preset
    pub quality: CompressionQuality,
    /// Use GPU compression
    pub gpu_accelerated: bool,
    /// Perceptual error metric
    pub perceptual: bool,
    /// Alpha weight (for formats with alpha)
    pub alpha_weight: f32,
    /// Generate mipmaps
    pub generate_mipmaps: bool,
    /// Mipmap filter
    pub mipmap_filter: MipmapFilter,
}

impl CompressionSettings {
    /// Creates settings
    pub fn new(format: BlockFormat) -> Self {
        Self {
            format,
            quality: CompressionQuality::Normal,
            gpu_accelerated: true,
            perceptual: true,
            alpha_weight: 1.0,
            generate_mipmaps: true,
            mipmap_filter: MipmapFilter::Kaiser,
        }
    }

    /// BC1 for color
    pub fn bc1_color() -> Self {
        Self::new(BlockFormat::Bc1)
    }

    /// BC3 for color with alpha
    pub fn bc3_rgba() -> Self {
        Self::new(BlockFormat::Bc3)
    }

    /// BC5 for normals
    pub fn bc5_normals() -> Self {
        Self {
            perceptual: false,
            ..Self::new(BlockFormat::Bc5)
        }
    }

    /// BC6H for HDR
    pub fn bc6h_hdr() -> Self {
        Self::new(BlockFormat::Bc6h)
    }

    /// BC7 for high quality
    pub fn bc7_high_quality() -> Self {
        Self {
            quality: CompressionQuality::Slow,
            ..Self::new(BlockFormat::Bc7)
        }
    }

    /// With quality
    pub fn with_quality(mut self, quality: CompressionQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Without mipmaps
    pub fn without_mipmaps(mut self) -> Self {
        self.generate_mipmaps = false;
        self
    }
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self::new(BlockFormat::Bc7)
    }
}

/// Mipmap filter
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MipmapFilter {
    /// Box filter
    Box = 0,
    /// Triangle filter
    Triangle = 1,
    /// Kaiser filter
    #[default]
    Kaiser = 2,
    /// Lanczos filter
    Lanczos = 3,
    /// Mitchell filter
    Mitchell = 4,
}

// ============================================================================
// BC7 Specific
// ============================================================================

/// BC7 mode flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Bc7ModeFlags(pub u32);

impl Bc7ModeFlags {
    /// No modes
    pub const NONE: Self = Self(0);
    /// Mode 0 (3 subsets, 4-bit indices)
    pub const MODE0: Self = Self(1 << 0);
    /// Mode 1 (2 subsets, 3-bit indices)
    pub const MODE1: Self = Self(1 << 1);
    /// Mode 2 (3 subsets, 2-bit indices)
    pub const MODE2: Self = Self(1 << 2);
    /// Mode 3 (2 subsets, 2-bit indices)
    pub const MODE3: Self = Self(1 << 3);
    /// Mode 4 (rotation, separate alpha)
    pub const MODE4: Self = Self(1 << 4);
    /// Mode 5 (rotation, shared alpha)
    pub const MODE5: Self = Self(1 << 5);
    /// Mode 6 (1 subset, 4-bit indices)
    pub const MODE6: Self = Self(1 << 6);
    /// Mode 7 (2 subsets, 2-bit indices, RGBA)
    pub const MODE7: Self = Self(1 << 7);
    /// All modes
    pub const ALL: Self = Self(0xFF);
    /// Opaque modes only
    pub const OPAQUE: Self = Self(0x4F); // Modes 0-3, 6

    /// Has mode
    pub const fn has(&self, mode: Self) -> bool {
        (self.0 & mode.0) != 0
    }
}

impl Default for Bc7ModeFlags {
    fn default() -> Self {
        Self::ALL
    }
}

/// BC7 encoder settings
#[derive(Clone, Debug)]
pub struct Bc7EncoderSettings {
    /// Allowed modes
    pub modes: Bc7ModeFlags,
    /// Max partitions to try
    pub max_partitions: u32,
    /// Refinement passes
    pub refinement_passes: u32,
    /// Error metric
    pub error_metric: ErrorMetric,
}

impl Bc7EncoderSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            modes: Bc7ModeFlags::ALL,
            max_partitions: 64,
            refinement_passes: 2,
            error_metric: ErrorMetric::Perceptual,
        }
    }

    /// Fast preset
    pub fn fast() -> Self {
        Self {
            max_partitions: 16,
            refinement_passes: 1,
            ..Self::new()
        }
    }

    /// Opaque only
    pub fn opaque() -> Self {
        Self {
            modes: Bc7ModeFlags::OPAQUE,
            ..Self::new()
        }
    }
}

impl Default for Bc7EncoderSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Error metric
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ErrorMetric {
    /// Mean squared error
    Mse = 0,
    /// Perceptual (weighted)
    #[default]
    Perceptual = 1,
    /// SSIM-based
    Ssim = 2,
}

// ============================================================================
// ASTC Specific
// ============================================================================

/// ASTC encoder settings
#[derive(Clone, Debug)]
pub struct AstcEncoderSettings {
    /// Block size
    pub block_size: AstcBlockSize,
    /// Quality
    pub quality: CompressionQuality,
    /// Color profile
    pub color_profile: AstcColorProfile,
    /// Block mode
    pub block_mode: AstcBlockMode,
}

impl AstcEncoderSettings {
    /// Creates settings
    pub fn new(block_size: AstcBlockSize) -> Self {
        Self {
            block_size,
            quality: CompressionQuality::Normal,
            color_profile: AstcColorProfile::Linear,
            block_mode: AstcBlockMode::Exhaustive,
        }
    }

    /// 4x4 blocks
    pub fn astc_4x4() -> Self {
        Self::new(AstcBlockSize::Block4x4)
    }

    /// 6x6 blocks
    pub fn astc_6x6() -> Self {
        Self::new(AstcBlockSize::Block6x6)
    }
}

impl Default for AstcEncoderSettings {
    fn default() -> Self {
        Self::astc_4x4()
    }
}

/// ASTC block size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AstcBlockSize {
    /// 4x4
    #[default]
    Block4x4 = 0,
    /// 5x4
    Block5x4 = 1,
    /// 5x5
    Block5x5 = 2,
    /// 6x5
    Block6x5 = 3,
    /// 6x6
    Block6x6 = 4,
    /// 8x5
    Block8x5 = 5,
    /// 8x6
    Block8x6 = 6,
    /// 8x8
    Block8x8 = 7,
    /// 10x5
    Block10x5 = 8,
    /// 10x6
    Block10x6 = 9,
    /// 10x8
    Block10x8 = 10,
    /// 10x10
    Block10x10 = 11,
    /// 12x10
    Block12x10 = 12,
    /// 12x12
    Block12x12 = 13,
}

impl AstcBlockSize {
    /// Dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Block4x4 => (4, 4),
            Self::Block5x4 => (5, 4),
            Self::Block5x5 => (5, 5),
            Self::Block6x5 => (6, 5),
            Self::Block6x6 => (6, 6),
            Self::Block8x5 => (8, 5),
            Self::Block8x6 => (8, 6),
            Self::Block8x8 => (8, 8),
            Self::Block10x5 => (10, 5),
            Self::Block10x6 => (10, 6),
            Self::Block10x8 => (10, 8),
            Self::Block10x10 => (10, 10),
            Self::Block12x10 => (12, 10),
            Self::Block12x12 => (12, 12),
        }
    }

    /// Bits per pixel
    pub fn bits_per_pixel(&self) -> f32 {
        let (w, h) = self.dimensions();
        128.0 / (w * h) as f32
    }
}

/// ASTC color profile
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AstcColorProfile {
    /// Linear
    #[default]
    Linear = 0,
    /// sRGB
    Srgb = 1,
    /// HDR RGB
    HdrRgb = 2,
    /// HDR RGBA
    HdrRgba = 3,
}

/// ASTC block mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AstcBlockMode {
    /// Fast
    Fast = 0,
    /// Medium
    Medium = 1,
    /// Thorough
    Thorough = 2,
    /// Exhaustive
    #[default]
    Exhaustive = 3,
}

// ============================================================================
// Normal Map Compression
// ============================================================================

/// Normal map compression settings
#[derive(Clone, Debug)]
pub struct NormalMapCompressionSettings {
    /// Format
    pub format: NormalMapFormat,
    /// Quality
    pub quality: CompressionQuality,
    /// Normalize output
    pub renormalize: bool,
    /// Swizzle mode
    pub swizzle: NormalMapSwizzle,
}

impl NormalMapCompressionSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            format: NormalMapFormat::Bc5,
            quality: CompressionQuality::Normal,
            renormalize: true,
            swizzle: NormalMapSwizzle::XY,
        }
    }

    /// BC5 format
    pub fn bc5() -> Self {
        Self::new()
    }

    /// DXT5nm format
    pub fn dxt5nm() -> Self {
        Self {
            format: NormalMapFormat::Dxt5nm,
            swizzle: NormalMapSwizzle::AG,
            ..Self::new()
        }
    }
}

impl Default for NormalMapCompressionSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Normal map format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NormalMapFormat {
    /// BC5 (two channel)
    #[default]
    Bc5 = 0,
    /// DXT5nm (AG swizzle)
    Dxt5nm = 1,
    /// RG8
    Rg8 = 2,
    /// RGBA8
    Rgba8 = 3,
}

/// Normal map swizzle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NormalMapSwizzle {
    /// XY in RG
    #[default]
    XY = 0,
    /// XY in AG (DXT5nm style)
    AG = 1,
    /// Full XYZ
    Xyz = 2,
}

// ============================================================================
// GPU Compression
// ============================================================================

/// GPU compressor create info
#[derive(Clone, Debug)]
pub struct GpuCompressorCreateInfo {
    /// Supported formats
    pub formats: Vec<BlockFormat>,
    /// Max texture size
    pub max_texture_size: u32,
    /// Max concurrent
    pub max_concurrent: u32,
}

impl GpuCompressorCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            formats: Vec::new(),
            max_texture_size: 16384,
            max_concurrent: 4,
        }
    }

    /// Add format
    pub fn with_format(mut self, format: BlockFormat) -> Self {
        self.formats.push(format);
        self
    }

    /// BCn formats
    pub fn bcn_formats() -> Self {
        Self {
            formats: alloc::vec![
                BlockFormat::Bc1,
                BlockFormat::Bc3,
                BlockFormat::Bc4,
                BlockFormat::Bc5,
                BlockFormat::Bc6h,
                BlockFormat::Bc7
            ],
            ..Self::new()
        }
    }
}

impl Default for GpuCompressorCreateInfo {
    fn default() -> Self {
        Self::bcn_formats()
    }
}

/// GPU compression pass
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCompressionParams {
    /// Source size
    pub src_size: [u32; 2],
    /// Block count
    pub block_count: [u32; 2],
    /// Format ID
    pub format: u32,
    /// Quality level
    pub quality: u32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding: u32,
}

// ============================================================================
// Compression Task
// ============================================================================

/// Compression task info
#[derive(Clone, Debug)]
pub struct CompressionTaskInfo {
    /// Source width
    pub src_width: u32,
    /// Source height
    pub src_height: u32,
    /// Source format
    pub src_format: SourceFormat,
    /// Target settings
    pub settings: CompressionSettings,
    /// Priority
    pub priority: u32,
}

impl CompressionTaskInfo {
    /// Creates info
    pub fn new(width: u32, height: u32, settings: CompressionSettings) -> Self {
        Self {
            src_width: width,
            src_height: height,
            src_format: SourceFormat::Rgba8,
            settings,
            priority: 0,
        }
    }

    /// Output size
    pub fn output_size(&self) -> u64 {
        self.settings.format.compressed_size(self.src_width, self.src_height)
    }

    /// Block count
    pub fn block_count(&self) -> (u32, u32) {
        let (bw, bh) = self.settings.format.block_size();
        (
            (self.src_width + bw - 1) / bw,
            (self.src_height + bh - 1) / bh,
        )
    }
}

impl Default for CompressionTaskInfo {
    fn default() -> Self {
        Self::new(256, 256, CompressionSettings::default())
    }
}

/// Source format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SourceFormat {
    /// RGBA8 unorm
    #[default]
    Rgba8 = 0,
    /// RGBA16F
    Rgba16f = 1,
    /// RGBA32F
    Rgba32f = 2,
    /// BGRA8 unorm
    Bgra8 = 3,
}

/// Compression task state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompressionTaskState {
    /// Pending
    #[default]
    Pending = 0,
    /// Compressing
    Compressing = 1,
    /// Complete
    Complete = 2,
    /// Failed
    Failed = 3,
}

// ============================================================================
// Statistics
// ============================================================================

/// Compression statistics
#[derive(Clone, Debug, Default)]
pub struct CompressionStats {
    /// Textures compressed
    pub textures_compressed: u32,
    /// Bytes input
    pub bytes_input: u64,
    /// Bytes output
    pub bytes_output: u64,
    /// Compression time (microseconds)
    pub compression_time_us: u64,
    /// PSNR (if calculated)
    pub psnr: f32,
}

impl CompressionStats {
    /// Compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.bytes_output == 0 {
            0.0
        } else {
            self.bytes_input as f32 / self.bytes_output as f32
        }
    }

    /// Throughput (MB/s)
    pub fn throughput_mbps(&self) -> f32 {
        if self.compression_time_us == 0 {
            0.0
        } else {
            let mb = self.bytes_input as f64 / (1024.0 * 1024.0);
            let seconds = self.compression_time_us as f64 / 1_000_000.0;
            (mb / seconds) as f32
        }
    }
}
