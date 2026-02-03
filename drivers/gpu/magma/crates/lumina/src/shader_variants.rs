//! Shader Variants Types for Lumina
//!
//! This module provides shader variant management infrastructure
//! for handling shader permutations and specialization constants.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Shader Variant Handles
// ============================================================================

/// Shader variant handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderVariantHandle(pub u64);

impl ShaderVariantHandle {
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

impl Default for ShaderVariantHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader variant set handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderVariantSetHandle(pub u64);

impl ShaderVariantSetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ShaderVariantSetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader permutation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderPermutationHandle(pub u64);

impl ShaderPermutationHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ShaderPermutationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader feature set handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderFeatureSetHandle(pub u64);

impl ShaderFeatureSetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ShaderFeatureSetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Shader Variant Definition
// ============================================================================

/// Shader variant create info
#[derive(Clone, Debug)]
pub struct ShaderVariantCreateInfo {
    /// Name
    pub name: String,
    /// Base shader
    pub base_shader: u64,
    /// Defines
    pub defines: Vec<ShaderDefine>,
    /// Specialization constants
    pub specializations: Vec<SpecializationConstant>,
    /// Feature flags
    pub features: ShaderFeatureFlags,
    /// Quality level
    pub quality: ShaderQualityLevel,
}

impl ShaderVariantCreateInfo {
    /// Creates new info
    pub fn new(base_shader: u64) -> Self {
        Self {
            name: String::new(),
            base_shader,
            defines: Vec::new(),
            specializations: Vec::new(),
            features: ShaderFeatureFlags::empty(),
            quality: ShaderQualityLevel::Medium,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With define
    pub fn with_define(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.defines.push(ShaderDefine {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// With bool define
    pub fn with_bool_define(mut self, name: impl Into<String>, value: bool) -> Self {
        self.defines.push(ShaderDefine {
            name: name.into(),
            value: if value { "1".into() } else { "0".into() },
        });
        self
    }

    /// With int define
    pub fn with_int_define(mut self, name: impl Into<String>, value: i32) -> Self {
        let mut buf = [0u8; 16];
        let s = format_i32(value, &mut buf);
        self.defines.push(ShaderDefine {
            name: name.into(),
            value: String::from(s),
        });
        self
    }

    /// With specialization constant
    pub fn with_specialization(mut self, constant: SpecializationConstant) -> Self {
        self.specializations.push(constant);
        self
    }

    /// With features
    pub fn with_features(mut self, features: ShaderFeatureFlags) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: ShaderQualityLevel) -> Self {
        self.quality = quality;
        self
    }
}

impl Default for ShaderVariantCreateInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Simple i32 to string helper
fn format_i32(value: i32, buf: &mut [u8; 16]) -> &str {
    use core::fmt::Write;
    struct SliceWriter<'a> {
        buf: &'a mut [u8],
        pos: usize,
    }
    impl<'a> Write for SliceWriter<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let bytes = s.as_bytes();
            if self.pos + bytes.len() <= self.buf.len() {
                self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
                self.pos += bytes.len();
                Ok(())
            } else {
                Err(core::fmt::Error)
            }
        }
    }
    let mut writer = SliceWriter { buf, pos: 0 };
    let _ = write!(writer, "{}", value);
    core::str::from_utf8(&buf[..writer.pos]).unwrap_or("0")
}

/// Shader define
#[derive(Clone, Debug)]
pub struct ShaderDefine {
    /// Define name
    pub name: String,
    /// Define value
    pub value: String,
}

impl ShaderDefine {
    /// Creates new define
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Bool define
    pub fn boolean(name: impl Into<String>, value: bool) -> Self {
        Self::new(name, if value { "1" } else { "0" })
    }

    /// Integer define
    pub fn integer(name: impl Into<String>, value: i32) -> Self {
        let mut buf = [0u8; 16];
        let s = format_i32(value, &mut buf);
        Self::new(name, s)
    }

    /// Flag define (no value)
    pub fn flag(name: impl Into<String>) -> Self {
        Self::new(name, "")
    }
}

// ============================================================================
// Specialization Constants
// ============================================================================

/// Specialization constant
#[derive(Clone, Debug)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: u32,
    /// Constant name
    pub name: String,
    /// Value
    pub value: SpecializationValue,
}

impl SpecializationConstant {
    /// Creates bool constant
    pub fn bool(id: u32, name: impl Into<String>, value: bool) -> Self {
        Self {
            id,
            name: name.into(),
            value: SpecializationValue::Bool(value),
        }
    }

    /// Creates i32 constant
    pub fn i32(id: u32, name: impl Into<String>, value: i32) -> Self {
        Self {
            id,
            name: name.into(),
            value: SpecializationValue::I32(value),
        }
    }

    /// Creates u32 constant
    pub fn u32(id: u32, name: impl Into<String>, value: u32) -> Self {
        Self {
            id,
            name: name.into(),
            value: SpecializationValue::U32(value),
        }
    }

    /// Creates f32 constant
    pub fn f32(id: u32, name: impl Into<String>, value: f32) -> Self {
        Self {
            id,
            name: name.into(),
            value: SpecializationValue::F32(value),
        }
    }
}

/// Specialization value
#[derive(Clone, Copy, Debug)]
pub enum SpecializationValue {
    /// Boolean
    Bool(bool),
    /// 32-bit signed integer
    I32(i32),
    /// 32-bit unsigned integer
    U32(u32),
    /// 32-bit float
    F32(f32),
    /// 64-bit signed integer
    I64(i64),
    /// 64-bit unsigned integer
    U64(u64),
    /// 64-bit float
    F64(f64),
}

impl SpecializationValue {
    /// Size in bytes
    pub const fn size(&self) -> usize {
        match self {
            Self::Bool(_) => 4, // SPIR-V bool is 4 bytes
            Self::I32(_) | Self::U32(_) | Self::F32(_) => 4,
            Self::I64(_) | Self::U64(_) | Self::F64(_) => 8,
        }
    }

    /// As u32 (for 32-bit types)
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Bool(v) => {
                if *v {
                    1
                } else {
                    0
                }
            },
            Self::I32(v) => *v as u32,
            Self::U32(v) => *v,
            Self::F32(v) => v.to_bits(),
            _ => 0,
        }
    }
}

impl Default for SpecializationValue {
    fn default() -> Self {
        Self::U32(0)
    }
}

// ============================================================================
// Shader Feature Flags
// ============================================================================

bitflags::bitflags! {
    /// Shader feature flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ShaderFeatureFlags: u64 {
        /// None
        const NONE = 0;

        // Lighting features
        /// Shadows
        const SHADOWS = 1 << 0;
        /// Soft shadows
        const SOFT_SHADOWS = 1 << 1;
        /// Ambient occlusion
        const AMBIENT_OCCLUSION = 1 << 2;
        /// Global illumination
        const GLOBAL_ILLUMINATION = 1 << 3;
        /// Reflections
        const REFLECTIONS = 1 << 4;
        /// Refractions
        const REFRACTIONS = 1 << 5;

        // Material features
        /// Normal mapping
        const NORMAL_MAPPING = 1 << 8;
        /// Parallax mapping
        const PARALLAX_MAPPING = 1 << 9;
        /// Subsurface scattering
        const SUBSURFACE_SCATTERING = 1 << 10;
        /// Anisotropic
        const ANISOTROPIC = 1 << 11;
        /// Clear coat
        const CLEAR_COAT = 1 << 12;
        /// Sheen
        const SHEEN = 1 << 13;

        // Effect features
        /// Bloom
        const BLOOM = 1 << 16;
        /// Depth of field
        const DEPTH_OF_FIELD = 1 << 17;
        /// Motion blur
        const MOTION_BLUR = 1 << 18;
        /// Fog
        const FOG = 1 << 19;
        /// Volumetrics
        const VOLUMETRICS = 1 << 20;

        // Technical features
        /// Instancing
        const INSTANCING = 1 << 24;
        /// Skinning
        const SKINNING = 1 << 25;
        /// Morphing
        const MORPHING = 1 << 26;
        /// Tessellation
        const TESSELLATION = 1 << 27;
        /// Geometry shader
        const GEOMETRY_SHADER = 1 << 28;

        // Debug features
        /// Debug output
        const DEBUG_OUTPUT = 1 << 32;
        /// Wireframe
        const WIREFRAME = 1 << 33;
        /// Normals visualization
        const VISUALIZE_NORMALS = 1 << 34;
    }
}

impl ShaderFeatureFlags {
    /// Standard PBR features
    pub const STANDARD_PBR: Self = Self::from_bits_truncate(
        Self::SHADOWS.bits() | Self::NORMAL_MAPPING.bits() | Self::AMBIENT_OCCLUSION.bits(),
    );

    /// High quality features
    pub const HIGH_QUALITY: Self = Self::from_bits_truncate(
        Self::SHADOWS.bits()
            | Self::SOFT_SHADOWS.bits()
            | Self::NORMAL_MAPPING.bits()
            | Self::AMBIENT_OCCLUSION.bits()
            | Self::REFLECTIONS.bits()
            | Self::GLOBAL_ILLUMINATION.bits(),
    );
}

// ============================================================================
// Shader Quality Levels
// ============================================================================

/// Shader quality level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderQualityLevel {
    /// Lowest quality
    VeryLow = 0,
    /// Low quality
    Low     = 1,
    /// Medium quality
    #[default]
    Medium  = 2,
    /// High quality
    High    = 3,
    /// Ultra quality
    Ultra   = 4,
}

impl ShaderQualityLevel {
    /// Quality value (0-100)
    pub const fn quality_value(&self) -> u32 {
        match self {
            Self::VeryLow => 10,
            Self::Low => 30,
            Self::Medium => 50,
            Self::High => 75,
            Self::Ultra => 100,
        }
    }

    /// Shadow sample count
    pub const fn shadow_samples(&self) -> u32 {
        match self {
            Self::VeryLow => 1,
            Self::Low => 4,
            Self::Medium => 8,
            Self::High => 16,
            Self::Ultra => 32,
        }
    }

    /// AO sample count
    pub const fn ao_samples(&self) -> u32 {
        match self {
            Self::VeryLow => 4,
            Self::Low => 8,
            Self::Medium => 16,
            Self::High => 32,
            Self::Ultra => 64,
        }
    }

    /// Reflection quality (0-4)
    pub const fn reflection_quality(&self) -> u32 {
        match self {
            Self::VeryLow => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Ultra => 4,
        }
    }
}

// ============================================================================
// Shader Variant Set
// ============================================================================

/// Shader variant set create info
#[derive(Clone, Debug)]
pub struct ShaderVariantSetCreateInfo {
    /// Name
    pub name: String,
    /// Base shader
    pub base_shader: u64,
    /// Variant dimensions
    pub dimensions: Vec<VariantDimension>,
    /// Max variants to cache
    pub max_cached_variants: u32,
    /// Compile on demand
    pub on_demand_compilation: bool,
}

impl ShaderVariantSetCreateInfo {
    /// Creates new info
    pub fn new(base_shader: u64) -> Self {
        Self {
            name: String::new(),
            base_shader,
            dimensions: Vec::new(),
            max_cached_variants: 256,
            on_demand_compilation: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With dimension
    pub fn with_dimension(mut self, dimension: VariantDimension) -> Self {
        self.dimensions.push(dimension);
        self
    }

    /// With max cached
    pub fn with_max_cached(mut self, max: u32) -> Self {
        self.max_cached_variants = max;
        self
    }

    /// Disable on-demand compilation
    pub fn precompile_all(mut self) -> Self {
        self.on_demand_compilation = false;
        self
    }

    /// Total variant count
    pub fn total_variants(&self) -> usize {
        self.dimensions.iter().map(|d| d.value_count()).product()
    }
}

impl Default for ShaderVariantSetCreateInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Variant dimension
#[derive(Clone, Debug)]
pub struct VariantDimension {
    /// Name
    pub name: String,
    /// Dimension type
    pub dimension_type: VariantDimensionType,
    /// Possible values
    pub values: Vec<VariantValue>,
}

impl VariantDimension {
    /// Boolean dimension
    pub fn boolean(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dimension_type: VariantDimensionType::Boolean,
            values: alloc::vec![VariantValue::Bool(false), VariantValue::Bool(true),],
        }
    }

    /// Integer dimension
    pub fn integer(name: impl Into<String>, values: impl IntoIterator<Item = i32>) -> Self {
        Self {
            name: name.into(),
            dimension_type: VariantDimensionType::Integer,
            values: values.into_iter().map(VariantValue::Int).collect(),
        }
    }

    /// Enum dimension
    pub fn enumeration(
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            dimension_type: VariantDimensionType::Enum,
            values: values
                .into_iter()
                .map(|v| VariantValue::Enum(v.into()))
                .collect(),
        }
    }

    /// Value count
    pub fn value_count(&self) -> usize {
        self.values.len().max(1)
    }
}

/// Variant dimension type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VariantDimensionType {
    /// Boolean (true/false)
    #[default]
    Boolean = 0,
    /// Integer values
    Integer = 1,
    /// Enum values
    Enum    = 2,
    /// Quality levels
    Quality = 3,
}

/// Variant value
#[derive(Clone, Debug)]
pub enum VariantValue {
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i32),
    /// Enum string
    Enum(String),
}

// ============================================================================
// Shader Permutation
// ============================================================================

/// Shader permutation key
#[derive(Clone, Debug, Default)]
pub struct ShaderPermutationKey {
    /// Dimension values
    pub values: Vec<(String, VariantValue)>,
    /// Feature flags
    pub features: ShaderFeatureFlags,
    /// Quality level
    pub quality: ShaderQualityLevel,
}

impl ShaderPermutationKey {
    /// Creates new key
    pub fn new() -> Self {
        Self::default()
    }

    /// With boolean value
    pub fn with_bool(mut self, name: impl Into<String>, value: bool) -> Self {
        self.values.push((name.into(), VariantValue::Bool(value)));
        self
    }

    /// With integer value
    pub fn with_int(mut self, name: impl Into<String>, value: i32) -> Self {
        self.values.push((name.into(), VariantValue::Int(value)));
        self
    }

    /// With enum value
    pub fn with_enum(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.values
            .push((name.into(), VariantValue::Enum(value.into())));
        self
    }

    /// With features
    pub fn with_features(mut self, features: ShaderFeatureFlags) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: ShaderQualityLevel) -> Self {
        self.quality = quality;
        self
    }

    /// Compute hash
    pub fn compute_hash(&self) -> u64 {
        // Simple FNV-1a hash
        let mut hash: u64 = 0xcbf29ce484222325;
        for (name, value) in &self.values {
            for b in name.bytes() {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            match value {
                VariantValue::Bool(v) => {
                    hash ^= *v as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                },
                VariantValue::Int(v) => {
                    hash ^= *v as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                },
                VariantValue::Enum(s) => {
                    for b in s.bytes() {
                        hash ^= b as u64;
                        hash = hash.wrapping_mul(0x100000001b3);
                    }
                },
            }
        }
        hash ^= self.features.bits();
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.quality as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }
}

// ============================================================================
// Shader Feature Set
// ============================================================================

/// Shader feature set create info
#[derive(Clone, Debug)]
pub struct ShaderFeatureSetCreateInfo {
    /// Name
    pub name: String,
    /// Required features
    pub required: ShaderFeatureFlags,
    /// Optional features
    pub optional: ShaderFeatureFlags,
    /// Excluded features (mutually exclusive)
    pub excluded: ShaderFeatureFlags,
}

impl ShaderFeatureSetCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            required: ShaderFeatureFlags::empty(),
            optional: ShaderFeatureFlags::empty(),
            excluded: ShaderFeatureFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With required features
    pub fn with_required(mut self, features: ShaderFeatureFlags) -> Self {
        self.required |= features;
        self
    }

    /// With optional features
    pub fn with_optional(mut self, features: ShaderFeatureFlags) -> Self {
        self.optional |= features;
        self
    }

    /// With excluded features
    pub fn with_excluded(mut self, features: ShaderFeatureFlags) -> Self {
        self.excluded |= features;
        self
    }

    /// Standard PBR feature set
    pub fn standard_pbr() -> Self {
        Self::new()
            .with_name("Standard PBR")
            .with_required(ShaderFeatureFlags::NORMAL_MAPPING)
            .with_optional(
                ShaderFeatureFlags::SHADOWS
                    | ShaderFeatureFlags::AMBIENT_OCCLUSION
                    | ShaderFeatureFlags::REFLECTIONS,
            )
    }

    /// Unlit feature set
    pub fn unlit() -> Self {
        Self::new().with_name("Unlit").with_excluded(
            ShaderFeatureFlags::SHADOWS
                | ShaderFeatureFlags::AMBIENT_OCCLUSION
                | ShaderFeatureFlags::GLOBAL_ILLUMINATION,
        )
    }
}

impl Default for ShaderFeatureSetCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Variant Compilation
// ============================================================================

/// Variant compilation request
#[derive(Clone, Debug)]
pub struct VariantCompilationRequest {
    /// Base shader
    pub base_shader: u64,
    /// Permutation key
    pub key: ShaderPermutationKey,
    /// Priority
    pub priority: CompilationPriority,
    /// Callback user data
    pub user_data: u64,
}

impl VariantCompilationRequest {
    /// Creates new request
    pub fn new(base_shader: u64, key: ShaderPermutationKey) -> Self {
        Self {
            base_shader,
            key,
            priority: CompilationPriority::Normal,
            user_data: 0,
        }
    }

    /// With priority
    pub fn with_priority(mut self, priority: CompilationPriority) -> Self {
        self.priority = priority;
        self
    }

    /// With user data
    pub fn with_user_data(mut self, data: u64) -> Self {
        self.user_data = data;
        self
    }
}

/// Compilation priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompilationPriority {
    /// Low priority (background)
    Low       = 0,
    /// Normal priority
    #[default]
    Normal    = 1,
    /// High priority
    High      = 2,
    /// Immediate (blocking)
    Immediate = 3,
}

/// Variant compilation result
#[derive(Clone, Debug)]
pub struct VariantCompilationResult {
    /// Success
    pub success: bool,
    /// Compiled shader handle
    pub shader: ShaderVariantHandle,
    /// Compilation time in microseconds
    pub compilation_time_us: u64,
    /// Binary size
    pub binary_size: u64,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl Default for VariantCompilationResult {
    fn default() -> Self {
        Self {
            success: false,
            shader: ShaderVariantHandle::NULL,
            compilation_time_us: 0,
            binary_size: 0,
            error: None,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Shader variant statistics
#[derive(Clone, Debug, Default)]
pub struct ShaderVariantStats {
    /// Total variants
    pub total_variants: u64,
    /// Compiled variants
    pub compiled_variants: u64,
    /// Cached variants
    pub cached_variants: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Total compilation time (microseconds)
    pub total_compilation_time_us: u64,
    /// Total binary size
    pub total_binary_size: u64,
    /// Average compilation time (microseconds)
    pub avg_compilation_time_us: u64,
    /// Pending compilations
    pub pending_compilations: u32,
}

impl ShaderVariantStats {
    /// Cache hit rate (0.0 - 1.0)
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 1.0;
        }
        self.cache_hits as f32 / total as f32
    }
}
