//! Vertex Input Layout
//!
//! This module provides vertex input configuration for the graphics pipeline.

use alloc::{string::String, vec::Vec};
use core::hash::{Hash, Hasher};

// ============================================================================
// Vertex Format
// ============================================================================

/// Vertex attribute format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VertexFormat {
    // 8-bit formats
    /// R8 unsigned int.
    Uint8,
    /// R8 signed int.
    Sint8,
    /// R8 unsigned normalized.
    Unorm8,
    /// R8 signed normalized.
    Snorm8,
    /// RG8 unsigned int.
    Uint8x2,
    /// RG8 signed int.
    Sint8x2,
    /// RG8 unsigned normalized.
    Unorm8x2,
    /// RG8 signed normalized.
    Snorm8x2,
    /// RGBA8 unsigned int.
    Uint8x4,
    /// RGBA8 signed int.
    Sint8x4,
    /// RGBA8 unsigned normalized.
    Unorm8x4,
    /// RGBA8 signed normalized.
    Snorm8x4,

    // 16-bit formats
    /// R16 unsigned int.
    Uint16,
    /// R16 signed int.
    Sint16,
    /// R16 unsigned normalized.
    Unorm16,
    /// R16 signed normalized.
    Snorm16,
    /// R16 float.
    Float16,
    /// RG16 unsigned int.
    Uint16x2,
    /// RG16 signed int.
    Sint16x2,
    /// RG16 unsigned normalized.
    Unorm16x2,
    /// RG16 signed normalized.
    Snorm16x2,
    /// RG16 float.
    Float16x2,
    /// RGBA16 unsigned int.
    Uint16x4,
    /// RGBA16 signed int.
    Sint16x4,
    /// RGBA16 unsigned normalized.
    Unorm16x4,
    /// RGBA16 signed normalized.
    Snorm16x4,
    /// RGBA16 float.
    Float16x4,

    // 32-bit formats
    /// R32 unsigned int.
    Uint32,
    /// R32 signed int.
    Sint32,
    /// R32 float.
    Float32,
    /// RG32 unsigned int.
    Uint32x2,
    /// RG32 signed int.
    Sint32x2,
    /// RG32 float.
    Float32x2,
    /// RGB32 unsigned int.
    Uint32x3,
    /// RGB32 signed int.
    Sint32x3,
    /// RGB32 float.
    Float32x3,
    /// RGBA32 unsigned int.
    Uint32x4,
    /// RGBA32 signed int.
    Sint32x4,
    /// RGBA32 float.
    Float32x4,

    // 64-bit formats
    /// R64 float.
    Float64,
    /// RG64 float.
    Float64x2,
    /// RGB64 float.
    Float64x3,
    /// RGBA64 float.
    Float64x4,

    // Packed formats
    /// RGB10A2 unsigned normalized.
    Unorm10_10_10_2,
    /// RGB10A2 unsigned int.
    Uint10_10_10_2,
}

impl VertexFormat {
    /// Get the size of this format in bytes.
    pub fn size(&self) -> u32 {
        match self {
            Self::Uint8 | Self::Sint8 | Self::Unorm8 | Self::Snorm8 => 1,
            Self::Uint8x2
            | Self::Sint8x2
            | Self::Unorm8x2
            | Self::Snorm8x2
            | Self::Uint16
            | Self::Sint16
            | Self::Unorm16
            | Self::Snorm16
            | Self::Float16 => 2,
            Self::Uint8x4 | Self::Sint8x4 | Self::Unorm8x4 | Self::Snorm8x4 => 4,
            Self::Uint16x2
            | Self::Sint16x2
            | Self::Unorm16x2
            | Self::Snorm16x2
            | Self::Float16x2
            | Self::Uint32
            | Self::Sint32
            | Self::Float32
            | Self::Unorm10_10_10_2
            | Self::Uint10_10_10_2 => 4,
            Self::Uint16x4
            | Self::Sint16x4
            | Self::Unorm16x4
            | Self::Snorm16x4
            | Self::Float16x4
            | Self::Uint32x2
            | Self::Sint32x2
            | Self::Float32x2
            | Self::Float64 => 8,
            Self::Uint32x3 | Self::Sint32x3 | Self::Float32x3 => 12,
            Self::Uint32x4 | Self::Sint32x4 | Self::Float32x4 | Self::Float64x2 => 16,
            Self::Float64x3 => 24,
            Self::Float64x4 => 32,
        }
    }

    /// Get the number of components.
    pub fn components(&self) -> u32 {
        match self {
            Self::Uint8
            | Self::Sint8
            | Self::Unorm8
            | Self::Snorm8
            | Self::Uint16
            | Self::Sint16
            | Self::Unorm16
            | Self::Snorm16
            | Self::Float16
            | Self::Uint32
            | Self::Sint32
            | Self::Float32
            | Self::Float64 => 1,
            Self::Uint8x2
            | Self::Sint8x2
            | Self::Unorm8x2
            | Self::Snorm8x2
            | Self::Uint16x2
            | Self::Sint16x2
            | Self::Unorm16x2
            | Self::Snorm16x2
            | Self::Float16x2
            | Self::Uint32x2
            | Self::Sint32x2
            | Self::Float32x2
            | Self::Float64x2 => 2,
            Self::Uint32x3 | Self::Sint32x3 | Self::Float32x3 | Self::Float64x3 => 3,
            Self::Uint8x4
            | Self::Sint8x4
            | Self::Unorm8x4
            | Self::Snorm8x4
            | Self::Uint16x4
            | Self::Sint16x4
            | Self::Unorm16x4
            | Self::Snorm16x4
            | Self::Float16x4
            | Self::Uint32x4
            | Self::Sint32x4
            | Self::Float32x4
            | Self::Float64x4
            | Self::Unorm10_10_10_2
            | Self::Uint10_10_10_2 => 4,
        }
    }

    /// Check if this is a float format.
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            Self::Float16
                | Self::Float16x2
                | Self::Float16x4
                | Self::Float32
                | Self::Float32x2
                | Self::Float32x3
                | Self::Float32x4
                | Self::Float64
                | Self::Float64x2
                | Self::Float64x3
                | Self::Float64x4
        )
    }

    /// Check if this is a normalized format.
    pub fn is_normalized(&self) -> bool {
        matches!(
            self,
            Self::Unorm8
                | Self::Snorm8
                | Self::Unorm8x2
                | Self::Snorm8x2
                | Self::Unorm8x4
                | Self::Snorm8x4
                | Self::Unorm16
                | Self::Snorm16
                | Self::Unorm16x2
                | Self::Snorm16x2
                | Self::Unorm16x4
                | Self::Snorm16x4
                | Self::Unorm10_10_10_2
        )
    }
}

// ============================================================================
// Vertex Input Rate
// ============================================================================

/// Vertex input rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VertexInputRate {
    /// Per-vertex data.
    #[default]
    Vertex,
    /// Per-instance data.
    Instance,
}

// ============================================================================
// Vertex Attribute
// ============================================================================

/// Vertex attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexAttribute {
    /// Shader location.
    pub location: u32,
    /// Binding index.
    pub binding: u32,
    /// Attribute format.
    pub format: VertexFormat,
    /// Offset in bytes from binding start.
    pub offset: u32,
    /// Semantic name (optional, for debugging).
    pub semantic: Option<String>,
}

impl VertexAttribute {
    /// Create a new vertex attribute.
    pub fn new(location: u32, binding: u32, format: VertexFormat, offset: u32) -> Self {
        Self {
            location,
            binding,
            format,
            offset,
            semantic: None,
        }
    }

    /// Create with semantic name.
    pub fn with_semantic(mut self, semantic: &str) -> Self {
        self.semantic = Some(String::from(semantic));
        self
    }

    /// Create position attribute.
    pub fn position(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float32x3, offset)
            .with_semantic("POSITION")
    }

    /// Create normal attribute.
    pub fn normal(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float32x3, offset)
            .with_semantic("NORMAL")
    }

    /// Create tangent attribute.
    pub fn tangent(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float32x4, offset)
            .with_semantic("TANGENT")
    }

    /// Create texcoord attribute.
    pub fn texcoord(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float32x2, offset)
            .with_semantic("TEXCOORD")
    }

    /// Create color attribute.
    pub fn color(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Unorm8x4, offset)
            .with_semantic("COLOR")
    }

    /// Create joint indices attribute.
    pub fn joints(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Uint16x4, offset)
            .with_semantic("JOINTS")
    }

    /// Create joint weights attribute.
    pub fn weights(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float32x4, offset)
            .with_semantic("WEIGHTS")
    }
}

// ============================================================================
// Vertex Binding
// ============================================================================

/// Vertex binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VertexBinding {
    /// Binding index.
    pub binding: u32,
    /// Stride in bytes.
    pub stride: u32,
    /// Input rate.
    pub input_rate: VertexInputRate,
    /// Divisor for instanced rendering.
    pub divisor: u32,
}

impl VertexBinding {
    /// Create a new vertex binding.
    pub fn new(binding: u32, stride: u32, input_rate: VertexInputRate) -> Self {
        Self {
            binding,
            stride,
            input_rate,
            divisor: 1,
        }
    }

    /// Create per-vertex binding.
    pub fn per_vertex(binding: u32, stride: u32) -> Self {
        Self::new(binding, stride, VertexInputRate::Vertex)
    }

    /// Create per-instance binding.
    pub fn per_instance(binding: u32, stride: u32) -> Self {
        Self::new(binding, stride, VertexInputRate::Instance)
    }

    /// Set divisor.
    pub fn with_divisor(mut self, divisor: u32) -> Self {
        self.divisor = divisor;
        self
    }
}

// ============================================================================
// Vertex Layout
// ============================================================================

/// Complete vertex layout.
#[derive(Clone, Default)]
pub struct VertexLayout {
    /// Bindings.
    pub bindings: Vec<VertexBinding>,
    /// Attributes.
    pub attributes: Vec<VertexAttribute>,
    /// Layout hash.
    hash: u64,
}

impl VertexLayout {
    /// Create an empty vertex layout.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            attributes: Vec::new(),
            hash: 0,
        }
    }

    /// Create from bindings and attributes.
    pub fn from_parts(bindings: Vec<VertexBinding>, attributes: Vec<VertexAttribute>) -> Self {
        let mut layout = Self {
            bindings,
            attributes,
            hash: 0,
        };
        layout.update_hash();
        layout
    }

    /// Add a binding.
    pub fn binding(mut self, binding: VertexBinding) -> Self {
        self.bindings.push(binding);
        self.update_hash();
        self
    }

    /// Add an attribute.
    pub fn attribute(mut self, attribute: VertexAttribute) -> Self {
        self.attributes.push(attribute);
        self.update_hash();
        self
    }

    /// Get layout hash.
    pub fn hash(&self) -> u64 {
        self.hash
    }

    /// Get total vertex size for a binding.
    pub fn vertex_size(&self, binding: u32) -> u32 {
        self.bindings
            .iter()
            .find(|b| b.binding == binding)
            .map(|b| b.stride)
            .unwrap_or(0)
    }

    /// Validate the layout.
    pub fn validate(&self) -> Result<(), LayoutError> {
        // Check for duplicate locations
        let mut locations: Vec<u32> = self.attributes.iter().map(|a| a.location).collect();
        locations.sort();
        for i in 1..locations.len() {
            if locations[i] == locations[i - 1] {
                return Err(LayoutError::DuplicateLocation(locations[i]));
            }
        }

        // Check that all attributes reference valid bindings
        for attr in &self.attributes {
            if !self.bindings.iter().any(|b| b.binding == attr.binding) {
                return Err(LayoutError::InvalidBinding(attr.binding));
            }
        }

        // Check for overlapping attributes within same binding
        for binding in &self.bindings {
            let mut attrs: Vec<_> = self
                .attributes
                .iter()
                .filter(|a| a.binding == binding.binding)
                .collect();
            attrs.sort_by_key(|a| a.offset);

            for i in 1..attrs.len() {
                let prev_end = attrs[i - 1].offset + attrs[i - 1].format.size();
                if attrs[i].offset < prev_end {
                    return Err(LayoutError::OverlappingAttributes(
                        attrs[i - 1].location,
                        attrs[i].location,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Update the hash.
    fn update_hash(&mut self) {
        let mut hasher = FnvHasher::new();
        for binding in &self.bindings {
            binding.binding.hash(&mut hasher);
            binding.stride.hash(&mut hasher);
            (binding.input_rate as u32).hash(&mut hasher);
        }
        for attr in &self.attributes {
            attr.location.hash(&mut hasher);
            attr.binding.hash(&mut hasher);
            attr.offset.hash(&mut hasher);
        }
        self.hash = hasher.finish();
    }
}

/// Layout validation error.
#[derive(Debug, Clone)]
pub enum LayoutError {
    /// Duplicate attribute location.
    DuplicateLocation(u32),
    /// Attribute references invalid binding.
    InvalidBinding(u32),
    /// Overlapping attributes.
    OverlappingAttributes(u32, u32),
}

// ============================================================================
// Vertex Layout Builder
// ============================================================================

/// Builder for vertex layouts.
pub struct VertexLayoutBuilder {
    bindings: Vec<VertexBinding>,
    attributes: Vec<VertexAttribute>,
    current_binding: u32,
    current_offset: u32,
    current_location: u32,
}

impl VertexLayoutBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            attributes: Vec::new(),
            current_binding: 0,
            current_offset: 0,
            current_location: 0,
        }
    }

    /// Begin a new binding.
    pub fn begin_binding(mut self, input_rate: VertexInputRate) -> Self {
        self.current_binding = self.bindings.len() as u32;
        self.current_offset = 0;
        // Binding will be added when finalized or when next binding begins
        self.bindings.push(VertexBinding::new(
            self.current_binding,
            0,
            input_rate,
        ));
        self
    }

    /// Add attribute to current binding.
    pub fn attribute(mut self, format: VertexFormat) -> Self {
        let attr = VertexAttribute::new(
            self.current_location,
            self.current_binding,
            format,
            self.current_offset,
        );
        self.current_offset += format.size();
        self.current_location += 1;
        self.attributes.push(attr);

        // Update binding stride
        if let Some(binding) = self.bindings.get_mut(self.current_binding as usize) {
            binding.stride = self.current_offset;
        }

        self
    }

    /// Add attribute with semantic.
    pub fn attribute_semantic(mut self, format: VertexFormat, semantic: &str) -> Self {
        let attr = VertexAttribute::new(
            self.current_location,
            self.current_binding,
            format,
            self.current_offset,
        )
        .with_semantic(semantic);
        self.current_offset += format.size();
        self.current_location += 1;
        self.attributes.push(attr);

        // Update binding stride
        if let Some(binding) = self.bindings.get_mut(self.current_binding as usize) {
            binding.stride = self.current_offset;
        }

        self
    }

    /// Add position attribute.
    pub fn position(self) -> Self {
        self.attribute_semantic(VertexFormat::Float32x3, "POSITION")
    }

    /// Add normal attribute.
    pub fn normal(self) -> Self {
        self.attribute_semantic(VertexFormat::Float32x3, "NORMAL")
    }

    /// Add tangent attribute.
    pub fn tangent(self) -> Self {
        self.attribute_semantic(VertexFormat::Float32x4, "TANGENT")
    }

    /// Add texcoord attribute.
    pub fn texcoord(self) -> Self {
        self.attribute_semantic(VertexFormat::Float32x2, "TEXCOORD")
    }

    /// Add color attribute.
    pub fn color(self) -> Self {
        self.attribute_semantic(VertexFormat::Unorm8x4, "COLOR")
    }

    /// Build the vertex layout.
    pub fn build(self) -> VertexLayout {
        VertexLayout::from_parts(self.bindings, self.attributes)
    }
}

impl Default for VertexLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Standard Vertex Layouts
// ============================================================================

/// Standard vertex layouts.
pub struct StandardLayouts;

impl StandardLayouts {
    /// Position only layout.
    pub fn position_only() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Vertex)
            .position()
            .build()
    }

    /// Position and texcoord layout.
    pub fn position_texcoord() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Vertex)
            .position()
            .texcoord()
            .build()
    }

    /// Position, normal, texcoord layout.
    pub fn position_normal_texcoord() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Vertex)
            .position()
            .normal()
            .texcoord()
            .build()
    }

    /// Full PBR layout.
    pub fn pbr() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Vertex)
            .position()
            .normal()
            .tangent()
            .texcoord()
            .build()
    }

    /// Skinned mesh layout.
    pub fn skinned() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Vertex)
            .position()
            .normal()
            .tangent()
            .texcoord()
            .attribute_semantic(VertexFormat::Uint16x4, "JOINTS")
            .attribute_semantic(VertexFormat::Float32x4, "WEIGHTS")
            .build()
    }

    /// UI/2D layout.
    pub fn ui() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Vertex)
            .attribute_semantic(VertexFormat::Float32x2, "POSITION")
            .texcoord()
            .color()
            .build()
    }

    /// Particle layout.
    pub fn particle() -> VertexLayout {
        VertexLayoutBuilder::new()
            .begin_binding(VertexInputRate::Instance)
            .attribute_semantic(VertexFormat::Float32x3, "POSITION")
            .attribute_semantic(VertexFormat::Float32, "SIZE")
            .attribute_semantic(VertexFormat::Float32, "ROTATION")
            .color()
            .build()
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
