//! Resource and Asset Management
//!
//! This module provides comprehensive resource management for the
//! Helix UEFI Bootloader, including fonts, icons, themes, and images.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Resource Management                                 │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Resource Types                                 │   │
//! │  │  Fonts │ Icons │ Images │ Themes │ Strings │ Audio │ Data       │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Resource Locations                             │   │
//! │  │  Embedded │ ESP Filesystem │ Network │ UEFI Variables            │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                   Resource Cache                                 │   │
//! │  │  Memory Pool │ LRU Eviction │ Reference Counting                │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use core::fmt;

// =============================================================================
// RESOURCE TYPES
// =============================================================================

/// Resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// Font resource
    Font,
    /// Icon resource
    Icon,
    /// Image/texture resource
    Image,
    /// Theme configuration
    Theme,
    /// String table
    Strings,
    /// Audio clip
    Audio,
    /// Binary data
    Data,
    /// Configuration
    Config,
    /// Shader/effect
    Shader,
}

impl Default for ResourceType {
    fn default() -> Self {
        ResourceType::Data
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Font => write!(f, "Font"),
            ResourceType::Icon => write!(f, "Icon"),
            ResourceType::Image => write!(f, "Image"),
            ResourceType::Theme => write!(f, "Theme"),
            ResourceType::Strings => write!(f, "Strings"),
            ResourceType::Audio => write!(f, "Audio"),
            ResourceType::Data => write!(f, "Data"),
            ResourceType::Config => write!(f, "Config"),
            ResourceType::Shader => write!(f, "Shader"),
        }
    }
}

/// Resource location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLocation {
    /// Embedded in binary
    Embedded,
    /// On ESP filesystem
    Filesystem,
    /// Network/remote
    Network,
    /// UEFI variable
    Variable,
    /// RAM disk
    RamDisk,
    /// Generated at runtime
    Generated,
}

impl Default for ResourceLocation {
    fn default() -> Self {
        ResourceLocation::Embedded
    }
}

/// Resource state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// Not loaded
    Unloaded,
    /// Currently loading
    Loading,
    /// Loaded and ready
    Ready,
    /// Load failed
    Failed,
    /// Evicted from cache
    Evicted,
}

impl Default for ResourceState {
    fn default() -> Self {
        ResourceState::Unloaded
    }
}

// =============================================================================
// RESOURCE ID
// =============================================================================

/// Resource ID type
pub type ResourceId = u32;

/// Resource ID namespace
pub mod resource_ids {
    //! Standard resource IDs

    // Fonts (0x1000 - 0x1FFF)
    pub const FONT_DEFAULT: u32 = 0x1000;
    pub const FONT_MONO: u32 = 0x1001;
    pub const FONT_BOLD: u32 = 0x1002;
    pub const FONT_ITALIC: u32 = 0x1003;
    pub const FONT_SMALL: u32 = 0x1010;
    pub const FONT_LARGE: u32 = 0x1011;
    pub const FONT_HUGE: u32 = 0x1012;
    pub const FONT_CONSOLE: u32 = 0x1020;

    // Icons (0x2000 - 0x2FFF)
    pub const ICON_HELIX: u32 = 0x2000;
    pub const ICON_LINUX: u32 = 0x2001;
    pub const ICON_WINDOWS: u32 = 0x2002;
    pub const ICON_MACOS: u32 = 0x2003;
    pub const ICON_BSD: u32 = 0x2004;
    pub const ICON_EFI: u32 = 0x2005;
    pub const ICON_NETWORK: u32 = 0x2010;
    pub const ICON_DISK: u32 = 0x2011;
    pub const ICON_USB: u32 = 0x2012;
    pub const ICON_CDROM: u32 = 0x2013;
    pub const ICON_SETTINGS: u32 = 0x2020;
    pub const ICON_POWER: u32 = 0x2021;
    pub const ICON_REBOOT: u32 = 0x2022;
    pub const ICON_RECOVERY: u32 = 0x2023;
    pub const ICON_SHELL: u32 = 0x2024;
    pub const ICON_WARNING: u32 = 0x2030;
    pub const ICON_ERROR: u32 = 0x2031;
    pub const ICON_SUCCESS: u32 = 0x2032;
    pub const ICON_INFO: u32 = 0x2033;

    // Images (0x3000 - 0x3FFF)
    pub const IMAGE_SPLASH: u32 = 0x3000;
    pub const IMAGE_BACKGROUND: u32 = 0x3001;
    pub const IMAGE_LOGO: u32 = 0x3002;
    pub const IMAGE_BANNER: u32 = 0x3003;

    // Themes (0x4000 - 0x4FFF)
    pub const THEME_DEFAULT: u32 = 0x4000;
    pub const THEME_DARK: u32 = 0x4001;
    pub const THEME_LIGHT: u32 = 0x4002;
    pub const THEME_HIGH_CONTRAST: u32 = 0x4003;
    pub const THEME_HELIX: u32 = 0x4004;

    // Strings (0x5000 - 0x5FFF)
    pub const STRINGS_EN: u32 = 0x5000;
    pub const STRINGS_FR: u32 = 0x5001;
    pub const STRINGS_DE: u32 = 0x5002;
    pub const STRINGS_ES: u32 = 0x5003;
    pub const STRINGS_IT: u32 = 0x5004;
    pub const STRINGS_PT: u32 = 0x5005;
    pub const STRINGS_RU: u32 = 0x5006;
    pub const STRINGS_ZH: u32 = 0x5007;
    pub const STRINGS_JA: u32 = 0x5008;
    pub const STRINGS_KO: u32 = 0x5009;

    // Audio (0x6000 - 0x6FFF)
    pub const AUDIO_STARTUP: u32 = 0x6000;
    pub const AUDIO_SELECT: u32 = 0x6001;
    pub const AUDIO_CONFIRM: u32 = 0x6002;
    pub const AUDIO_ERROR: u32 = 0x6003;
    pub const AUDIO_WARNING: u32 = 0x6004;
}

// =============================================================================
// FONT RESOURCES
// =============================================================================

/// Font format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// PSF (PC Screen Font)
    Psf,
    /// PSF v2
    Psf2,
    /// BDF (Bitmap Distribution Format)
    Bdf,
    /// Custom bitmap font
    Bitmap,
    /// TrueType (if supported)
    TrueType,
    /// OpenType (if supported)
    OpenType,
}

impl Default for FontFormat {
    fn default() -> Self {
        FontFormat::Psf2
    }
}

/// Font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    /// Thin
    Thin,
    /// Light
    Light,
    /// Regular
    Regular,
    /// Medium
    Medium,
    /// SemiBold
    SemiBold,
    /// Bold
    Bold,
    /// ExtraBold
    ExtraBold,
    /// Black
    Black,
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Regular
    }
}

/// Font style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    /// Normal
    Normal,
    /// Italic
    Italic,
    /// Oblique
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Normal
    }
}

/// Font metrics
#[derive(Debug, Clone, Copy, Default)]
pub struct FontMetrics {
    /// Character width (fixed-width)
    pub char_width: u8,
    /// Character height
    pub char_height: u8,
    /// Ascender (above baseline)
    pub ascender: i8,
    /// Descender (below baseline)
    pub descender: i8,
    /// Line height
    pub line_height: u8,
    /// Baseline offset
    pub baseline: u8,
    /// First character code
    pub first_char: u16,
    /// Last character code
    pub last_char: u16,
    /// Number of glyphs
    pub glyph_count: u16,
    /// Has Unicode table
    pub has_unicode: bool,
}

/// Font resource descriptor
#[derive(Debug, Clone, Copy)]
pub struct FontResource {
    /// Resource ID
    pub id: ResourceId,
    /// Format
    pub format: FontFormat,
    /// Weight
    pub weight: FontWeight,
    /// Style
    pub style: FontStyle,
    /// Metrics
    pub metrics: FontMetrics,
    /// Data offset
    pub data_offset: u32,
    /// Data size
    pub data_size: u32,
}

impl Default for FontResource {
    fn default() -> Self {
        Self {
            id: 0,
            format: FontFormat::Psf2,
            weight: FontWeight::Regular,
            style: FontStyle::Normal,
            metrics: FontMetrics::default(),
            data_offset: 0,
            data_size: 0,
        }
    }
}

// =============================================================================
// ICON RESOURCES
// =============================================================================

/// Icon format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconFormat {
    /// Raw BGRA pixels
    Raw,
    /// BMP format
    Bmp,
    /// PNG format
    Png,
    /// ICO format
    Ico,
    /// SVG (if supported)
    Svg,
}

impl Default for IconFormat {
    fn default() -> Self {
        IconFormat::Raw
    }
}

/// Icon size preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    /// 16x16
    Small,
    /// 24x24
    Medium,
    /// 32x32
    Normal,
    /// 48x48
    Large,
    /// 64x64
    XLarge,
    /// 128x128
    Huge,
    /// Custom size
    Custom { width: u16, height: u16 },
}

impl Default for IconSize {
    fn default() -> Self {
        IconSize::Normal
    }
}

impl IconSize {
    /// Get width
    pub const fn width(&self) -> u16 {
        match self {
            IconSize::Small => 16,
            IconSize::Medium => 24,
            IconSize::Normal => 32,
            IconSize::Large => 48,
            IconSize::XLarge => 64,
            IconSize::Huge => 128,
            IconSize::Custom { width, .. } => *width,
        }
    }

    /// Get height
    pub const fn height(&self) -> u16 {
        match self {
            IconSize::Small => 16,
            IconSize::Medium => 24,
            IconSize::Normal => 32,
            IconSize::Large => 48,
            IconSize::XLarge => 64,
            IconSize::Huge => 128,
            IconSize::Custom { height, .. } => *height,
        }
    }
}

/// Icon resource descriptor
#[derive(Debug, Clone, Copy)]
pub struct IconResource {
    /// Resource ID
    pub id: ResourceId,
    /// Format
    pub format: IconFormat,
    /// Size
    pub size: IconSize,
    /// Color depth (bits per pixel)
    pub depth: u8,
    /// Has alpha channel
    pub has_alpha: bool,
    /// Data offset
    pub data_offset: u32,
    /// Data size
    pub data_size: u32,
}

impl Default for IconResource {
    fn default() -> Self {
        Self {
            id: 0,
            format: IconFormat::Raw,
            size: IconSize::Normal,
            depth: 32,
            has_alpha: true,
            data_offset: 0,
            data_size: 0,
        }
    }
}

// =============================================================================
// IMAGE RESOURCES
// =============================================================================

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Raw pixels
    Raw,
    /// BMP format
    Bmp,
    /// PNG format
    Png,
    /// JPEG format
    Jpeg,
    /// GIF (static)
    Gif,
    /// TGA format
    Tga,
    /// PCX format
    Pcx,
}

impl Default for ImageFormat {
    fn default() -> Self {
        ImageFormat::Raw
    }
}

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 8-bit grayscale
    Gray8,
    /// 16-bit grayscale
    Gray16,
    /// 24-bit RGB
    Rgb24,
    /// 32-bit RGBA
    Rgba32,
    /// 32-bit BGRA (common for UEFI)
    Bgra32,
    /// 32-bit ARGB
    Argb32,
    /// 16-bit RGB565
    Rgb565,
    /// 8-bit indexed
    Indexed8,
}

impl Default for PixelFormat {
    fn default() -> Self {
        PixelFormat::Bgra32
    }
}

impl PixelFormat {
    /// Get bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u8 {
        match self {
            PixelFormat::Gray8 | PixelFormat::Indexed8 => 1,
            PixelFormat::Gray16 | PixelFormat::Rgb565 => 2,
            PixelFormat::Rgb24 => 3,
            PixelFormat::Rgba32 | PixelFormat::Bgra32 | PixelFormat::Argb32 => 4,
        }
    }

    /// Check if has alpha
    pub const fn has_alpha(&self) -> bool {
        matches!(
            self,
            PixelFormat::Rgba32 | PixelFormat::Bgra32 | PixelFormat::Argb32
        )
    }
}

/// Image resource descriptor
#[derive(Debug, Clone, Copy)]
pub struct ImageResource {
    /// Resource ID
    pub id: ResourceId,
    /// Format
    pub format: ImageFormat,
    /// Pixel format
    pub pixel_format: PixelFormat,
    /// Width
    pub width: u16,
    /// Height
    pub height: u16,
    /// Row stride (bytes)
    pub stride: u32,
    /// Data offset
    pub data_offset: u32,
    /// Data size (compressed)
    pub data_size: u32,
    /// Uncompressed size
    pub uncompressed_size: u32,
    /// Is compressed
    pub compressed: bool,
}

impl Default for ImageResource {
    fn default() -> Self {
        Self {
            id: 0,
            format: ImageFormat::Raw,
            pixel_format: PixelFormat::Bgra32,
            width: 0,
            height: 0,
            stride: 0,
            data_offset: 0,
            data_size: 0,
            uncompressed_size: 0,
            compressed: false,
        }
    }
}

// =============================================================================
// THEME RESOURCES
// =============================================================================

/// Color value (RGBA)
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create opaque color
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create color with alpha
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Black
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Transparent
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    // Standard colors
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    pub const GRAY: Self = Self::rgb(128, 128, 128);
}

/// Theme color scheme
#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    /// Background color
    pub background: Color,
    /// Foreground/text color
    pub foreground: Color,
    /// Primary accent color
    pub primary: Color,
    /// Secondary accent color
    pub secondary: Color,
    /// Success color
    pub success: Color,
    /// Warning color
    pub warning: Color,
    /// Error color
    pub error: Color,
    /// Info color
    pub info: Color,
    /// Border color
    pub border: Color,
    /// Selection background
    pub selection_bg: Color,
    /// Selection foreground
    pub selection_fg: Color,
    /// Disabled text
    pub disabled: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            background: Color::rgb(0, 0, 0),
            foreground: Color::rgb(255, 255, 255),
            primary: Color::rgb(0, 122, 204),
            secondary: Color::rgb(108, 117, 125),
            success: Color::rgb(40, 167, 69),
            warning: Color::rgb(255, 193, 7),
            error: Color::rgb(220, 53, 69),
            info: Color::rgb(23, 162, 184),
            border: Color::rgb(64, 64, 64),
            selection_bg: Color::rgb(0, 122, 204),
            selection_fg: Color::rgb(255, 255, 255),
            disabled: Color::rgb(128, 128, 128),
        }
    }
}

/// Theme resource
#[derive(Debug, Clone, Copy)]
pub struct ThemeResource {
    /// Resource ID
    pub id: ResourceId,
    /// Theme name
    pub name: [u8; 32],
    /// Name length
    pub name_len: usize,
    /// Color scheme
    pub colors: ColorScheme,
    /// Font ID for normal text
    pub font_normal: ResourceId,
    /// Font ID for bold text
    pub font_bold: ResourceId,
    /// Font ID for monospace
    pub font_mono: ResourceId,
    /// Background image ID (0 = none)
    pub background_image: ResourceId,
    /// Logo icon ID
    pub logo_icon: ResourceId,
    /// Use gradients
    pub use_gradients: bool,
    /// Use shadows
    pub use_shadows: bool,
    /// Border radius
    pub border_radius: u8,
    /// Animation enabled
    pub animation: bool,
}

impl Default for ThemeResource {
    fn default() -> Self {
        Self {
            id: 0,
            name: [0u8; 32],
            name_len: 0,
            colors: ColorScheme::default(),
            font_normal: resource_ids::FONT_DEFAULT,
            font_bold: resource_ids::FONT_BOLD,
            font_mono: resource_ids::FONT_MONO,
            background_image: 0,
            logo_icon: resource_ids::ICON_HELIX,
            use_gradients: true,
            use_shadows: true,
            border_radius: 4,
            animation: true,
        }
    }
}

// =============================================================================
// STRING RESOURCES
// =============================================================================

/// String table entry
#[derive(Debug, Clone, Copy)]
pub struct StringEntry {
    /// String ID
    pub id: u16,
    /// Offset in string data
    pub offset: u32,
    /// String length
    pub length: u16,
}

/// String table resource
#[derive(Debug, Clone, Copy)]
pub struct StringTableResource {
    /// Resource ID
    pub id: ResourceId,
    /// Language code (e.g., "en", "fr")
    pub language: [u8; 8],
    /// Language length
    pub language_len: usize,
    /// Number of entries
    pub entry_count: u16,
    /// String data offset
    pub data_offset: u32,
    /// String data size
    pub data_size: u32,
}

impl Default for StringTableResource {
    fn default() -> Self {
        Self {
            id: 0,
            language: [0u8; 8],
            language_len: 0,
            entry_count: 0,
            data_offset: 0,
            data_size: 0,
        }
    }
}

// =============================================================================
// RESOURCE BUNDLE
// =============================================================================

/// Resource bundle header
#[derive(Debug, Clone, Copy)]
pub struct ResourceBundleHeader {
    /// Magic number "HXRB"
    pub magic: [u8; 4],
    /// Version
    pub version: u16,
    /// Flags
    pub flags: u16,
    /// Total resources
    pub resource_count: u32,
    /// Total data size
    pub data_size: u32,
    /// Creation timestamp
    pub created: u64,
    /// Checksum
    pub checksum: u32,
}

impl ResourceBundleHeader {
    /// Magic bytes
    pub const MAGIC: [u8; 4] = *b"HXRB";
    /// Current version
    pub const VERSION: u16 = 1;

    /// Validate header
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.version == Self::VERSION
    }
}

impl Default for ResourceBundleHeader {
    fn default() -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            flags: 0,
            resource_count: 0,
            data_size: 0,
            created: 0,
            checksum: 0,
        }
    }
}

/// Resource entry in bundle
#[derive(Debug, Clone, Copy)]
pub struct ResourceEntry {
    /// Resource ID
    pub id: ResourceId,
    /// Resource type
    pub resource_type: ResourceType,
    /// Location
    pub location: ResourceLocation,
    /// Data offset in bundle
    pub offset: u32,
    /// Data size
    pub size: u32,
    /// Uncompressed size
    pub uncompressed_size: u32,
    /// Compression method (0 = none)
    pub compression: u8,
    /// Flags
    pub flags: u8,
    /// Name (for lookup)
    pub name: [u8; 32],
    /// Name length
    pub name_len: u8,
}

impl Default for ResourceEntry {
    fn default() -> Self {
        Self {
            id: 0,
            resource_type: ResourceType::Data,
            location: ResourceLocation::Embedded,
            offset: 0,
            size: 0,
            uncompressed_size: 0,
            compression: 0,
            flags: 0,
            name: [0u8; 32],
            name_len: 0,
        }
    }
}

// =============================================================================
// RESOURCE CACHE
// =============================================================================

/// Maximum cache entries
pub const MAX_CACHE_ENTRIES: usize = 32;

/// Cache entry
#[derive(Debug, Clone, Copy)]
pub struct CacheEntry {
    /// Resource ID
    pub id: ResourceId,
    /// State
    pub state: ResourceState,
    /// Memory address
    pub address: u64,
    /// Size in memory
    pub size: u32,
    /// Reference count
    pub ref_count: u16,
    /// Last access timestamp
    pub last_access: u64,
    /// Access count
    pub access_count: u32,
}

impl Default for CacheEntry {
    fn default() -> Self {
        Self {
            id: 0,
            state: ResourceState::Unloaded,
            address: 0,
            size: 0,
            ref_count: 0,
            last_access: 0,
            access_count: 0,
        }
    }
}

/// Resource cache
#[derive(Debug)]
pub struct ResourceCache {
    /// Cache entries
    entries: [CacheEntry; MAX_CACHE_ENTRIES],
    /// Entry count
    count: usize,
    /// Total memory used
    memory_used: u64,
    /// Maximum memory
    max_memory: u64,
    /// Cache hits
    hits: u32,
    /// Cache misses
    misses: u32,
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new(4 * 1024 * 1024) // 4MB default
    }
}

impl ResourceCache {
    /// Create new cache with size limit
    pub const fn new(max_memory: u64) -> Self {
        Self {
            entries: [CacheEntry {
                id: 0,
                state: ResourceState::Unloaded,
                address: 0,
                size: 0,
                ref_count: 0,
                last_access: 0,
                access_count: 0,
            }; MAX_CACHE_ENTRIES],
            count: 0,
            memory_used: 0,
            max_memory,
            hits: 0,
            misses: 0,
        }
    }

    /// Find entry by ID
    pub fn find(&mut self, id: ResourceId) -> Option<&CacheEntry> {
        for entry in &mut self.entries[..self.count] {
            if entry.id == id && entry.state == ResourceState::Ready {
                entry.access_count += 1;
                self.hits += 1;
                return Some(entry);
            }
        }
        self.misses += 1;
        None
    }

    /// Get cache hit rate (percentage)
    pub fn hit_rate(&self) -> u8 {
        let total = self.hits + self.misses;
        if total == 0 {
            return 0;
        }
        ((self.hits as u64 * 100) / total as u64) as u8
    }

    /// Get memory usage (percentage)
    pub fn memory_usage(&self) -> u8 {
        if self.max_memory == 0 {
            return 0;
        }
        ((self.memory_used * 100) / self.max_memory) as u8
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_color() {
        let c = Color::rgb(255, 128, 64);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_icon_size() {
        assert_eq!(IconSize::Normal.width(), 32);
        assert_eq!(IconSize::Normal.height(), 32);

        let custom = IconSize::Custom {
            width: 100,
            height: 50,
        };
        assert_eq!(custom.width(), 100);
        assert_eq!(custom.height(), 50);
    }

    #[test]
    fn test_pixel_format() {
        assert_eq!(PixelFormat::Bgra32.bytes_per_pixel(), 4);
        assert!(PixelFormat::Rgba32.has_alpha());
        assert!(!PixelFormat::Rgb24.has_alpha());
    }

    #[test]
    fn test_bundle_header() {
        let header = ResourceBundleHeader::default();
        assert!(header.is_valid());
    }

    #[test]
    fn test_resource_cache() {
        let cache = ResourceCache::new(1024 * 1024);
        assert_eq!(cache.hit_rate(), 0);
        assert_eq!(cache.memory_usage(), 0);
    }
}
