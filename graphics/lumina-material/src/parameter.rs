//! Material Parameters
//!
//! This module provides a flexible parameter system for materials.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem;

// ============================================================================
// Parameter Type
// ============================================================================

/// Parameter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterType {
    /// Boolean.
    Bool,
    /// 32-bit signed integer.
    Int,
    /// 32-bit unsigned integer.
    Uint,
    /// 32-bit float.
    Float,
    /// 2D vector.
    Vec2,
    /// 3D vector.
    Vec3,
    /// 4D vector.
    Vec4,
    /// 3x3 matrix.
    Mat3,
    /// 4x4 matrix.
    Mat4,
    /// Color (linear RGBA).
    Color,
    /// Texture reference.
    Texture,
    /// Sampler reference.
    Sampler,
}

impl ParameterType {
    /// Get size in bytes.
    pub fn size(&self) -> usize {
        match self {
            Self::Bool => 4, // Aligned to 4 bytes
            Self::Int | Self::Uint | Self::Float => 4,
            Self::Vec2 => 8,
            Self::Vec3 => 12,
            Self::Vec4 | Self::Color => 16,
            Self::Mat3 => 48, // 3 vec4s for alignment
            Self::Mat4 => 64,
            Self::Texture | Self::Sampler => 4, // Handle index
        }
    }

    /// Get alignment.
    pub fn alignment(&self) -> usize {
        match self {
            Self::Bool | Self::Int | Self::Uint | Self::Float => 4,
            Self::Vec2 => 8,
            Self::Vec3 | Self::Vec4 | Self::Color => 16,
            Self::Mat3 | Self::Mat4 => 16,
            Self::Texture | Self::Sampler => 4,
        }
    }

    /// Get GLSL type name.
    pub fn glsl_name(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Float => "float",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 | Self::Color => "vec4",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
            Self::Texture => "uint",
            Self::Sampler => "uint",
        }
    }
}

// ============================================================================
// Parameter Value
// ============================================================================

/// Parameter value.
#[derive(Debug, Clone)]
pub enum ParameterValue {
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i32),
    /// Unsigned integer.
    Uint(u32),
    /// Float.
    Float(f32),
    /// Vec2.
    Vec2([f32; 2]),
    /// Vec3.
    Vec3([f32; 3]),
    /// Vec4.
    Vec4([f32; 4]),
    /// Color.
    Color([f32; 4]),
    /// Mat3.
    Mat3([[f32; 3]; 3]),
    /// Mat4.
    Mat4([[f32; 4]; 4]),
    /// Texture handle.
    Texture(u32),
    /// Sampler handle.
    Sampler(u32),
}

impl ParameterValue {
    /// Get the type.
    pub fn parameter_type(&self) -> ParameterType {
        match self {
            Self::Bool(_) => ParameterType::Bool,
            Self::Int(_) => ParameterType::Int,
            Self::Uint(_) => ParameterType::Uint,
            Self::Float(_) => ParameterType::Float,
            Self::Vec2(_) => ParameterType::Vec2,
            Self::Vec3(_) => ParameterType::Vec3,
            Self::Vec4(_) => ParameterType::Vec4,
            Self::Color(_) => ParameterType::Color,
            Self::Mat3(_) => ParameterType::Mat3,
            Self::Mat4(_) => ParameterType::Mat4,
            Self::Texture(_) => ParameterType::Texture,
            Self::Sampler(_) => ParameterType::Sampler,
        }
    }

    /// Write to bytes.
    pub fn write_to(&self, buffer: &mut [u8]) {
        match self {
            Self::Bool(v) => {
                let value = if *v { 1u32 } else { 0u32 };
                buffer[..4].copy_from_slice(&value.to_le_bytes());
            },
            Self::Int(v) => buffer[..4].copy_from_slice(&v.to_le_bytes()),
            Self::Uint(v) => buffer[..4].copy_from_slice(&v.to_le_bytes()),
            Self::Float(v) => buffer[..4].copy_from_slice(&v.to_le_bytes()),
            Self::Vec2(v) => {
                buffer[..4].copy_from_slice(&v[0].to_le_bytes());
                buffer[4..8].copy_from_slice(&v[1].to_le_bytes());
            },
            Self::Vec3(v) => {
                buffer[..4].copy_from_slice(&v[0].to_le_bytes());
                buffer[4..8].copy_from_slice(&v[1].to_le_bytes());
                buffer[8..12].copy_from_slice(&v[2].to_le_bytes());
            },
            Self::Vec4(v) | Self::Color(v) => {
                buffer[..4].copy_from_slice(&v[0].to_le_bytes());
                buffer[4..8].copy_from_slice(&v[1].to_le_bytes());
                buffer[8..12].copy_from_slice(&v[2].to_le_bytes());
                buffer[12..16].copy_from_slice(&v[3].to_le_bytes());
            },
            Self::Mat3(m) => {
                for (i, row) in m.iter().enumerate() {
                    for (j, val) in row.iter().enumerate() {
                        let offset = (i * 16) + (j * 4); // Padded to vec4
                        buffer[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
                    }
                }
            },
            Self::Mat4(m) => {
                for (i, row) in m.iter().enumerate() {
                    for (j, val) in row.iter().enumerate() {
                        let offset = (i * 16) + (j * 4);
                        buffer[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
                    }
                }
            },
            Self::Texture(v) | Self::Sampler(v) => {
                buffer[..4].copy_from_slice(&v.to_le_bytes());
            },
        }
    }

    /// Get as float.
    pub fn as_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f32),
            Self::Uint(v) => Some(*v as f32),
            _ => None,
        }
    }

    /// Get as color.
    pub fn as_color(&self) -> Option<[f32; 4]> {
        match self {
            Self::Color(v) | Self::Vec4(v) => Some(*v),
            Self::Vec3(v) => Some([v[0], v[1], v[2], 1.0]),
            _ => None,
        }
    }

    /// Get as texture handle.
    pub fn as_texture(&self) -> Option<u32> {
        match self {
            Self::Texture(v) => Some(*v),
            _ => None,
        }
    }
}

// ============================================================================
// Parameter Definition
// ============================================================================

/// Parameter definition.
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// Display name.
    pub display_name: String,
    /// Description.
    pub description: String,
    /// Type.
    pub param_type: ParameterType,
    /// Default value.
    pub default_value: ParameterValue,
    /// Minimum value (for numeric types).
    pub min_value: Option<ParameterValue>,
    /// Maximum value (for numeric types).
    pub max_value: Option<ParameterValue>,
    /// UI hints.
    pub ui_hints: ParameterUiHints,
}

/// UI hints for parameter editing.
#[derive(Debug, Clone, Default)]
pub struct ParameterUiHints {
    /// Group name.
    pub group: Option<String>,
    /// Hidden in UI.
    pub hidden: bool,
    /// Read-only.
    pub read_only: bool,
    /// Use slider.
    pub slider: bool,
    /// Use color picker.
    pub color_picker: bool,
    /// Custom widget type.
    pub widget: Option<String>,
}

impl Parameter {
    /// Create a new parameter.
    pub fn new(
        name: impl Into<String>,
        param_type: ParameterType,
        default: ParameterValue,
    ) -> Self {
        let name = name.into();
        Self {
            display_name: name.clone(),
            name,
            description: String::new(),
            param_type,
            default_value: default,
            min_value: None,
            max_value: None,
            ui_hints: ParameterUiHints::default(),
        }
    }

    /// Create a float parameter.
    pub fn float(name: impl Into<String>, default: f32) -> Self {
        Self::new(name, ParameterType::Float, ParameterValue::Float(default))
    }

    /// Create a color parameter.
    pub fn color(name: impl Into<String>, default: [f32; 4]) -> Self {
        let mut param = Self::new(name, ParameterType::Color, ParameterValue::Color(default));
        param.ui_hints.color_picker = true;
        param
    }

    /// Create a texture parameter.
    pub fn texture(name: impl Into<String>) -> Self {
        Self::new(
            name,
            ParameterType::Texture,
            ParameterValue::Texture(u32::MAX),
        )
    }

    /// Set display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    /// Set description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set range for numeric types.
    pub fn range(mut self, min: ParameterValue, max: ParameterValue) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self.ui_hints.slider = true;
        self
    }

    /// Set group.
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.ui_hints.group = Some(group.into());
        self
    }

    /// Set hidden.
    pub fn hidden(mut self) -> Self {
        self.ui_hints.hidden = true;
        self
    }
}

// ============================================================================
// Parameter Binding
// ============================================================================

/// Parameter binding for GPU access.
#[derive(Debug, Clone)]
pub struct ParameterBinding {
    /// Binding index.
    pub binding: u32,
    /// Offset in buffer.
    pub offset: u32,
    /// Size in bytes.
    pub size: u32,
}

// ============================================================================
// Parameter Block
// ============================================================================

/// Parameter block for efficient GPU upload.
pub struct ParameterBlock {
    /// Block name.
    name: String,
    /// Parameters.
    parameters: Vec<Parameter>,
    /// Values.
    values: BTreeMap<String, ParameterValue>,
    /// Data buffer.
    data: Vec<u8>,
    /// Parameter offsets.
    offsets: BTreeMap<String, usize>,
    /// Total size.
    size: usize,
    /// Dirty flag.
    dirty: bool,
}

impl ParameterBlock {
    /// Create a new parameter block.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameters: Vec::new(),
            values: BTreeMap::new(),
            data: Vec::new(),
            offsets: BTreeMap::new(),
            size: 0,
            dirty: true,
        }
    }

    /// Add a parameter.
    pub fn add_parameter(&mut self, param: Parameter) {
        self.values
            .insert(param.name.clone(), param.default_value.clone());
        self.parameters.push(param);
        self.recalculate_layout();
    }

    /// Recalculate layout.
    fn recalculate_layout(&mut self) {
        self.offsets.clear();
        let mut offset = 0usize;

        for param in &self.parameters {
            let alignment = param.param_type.alignment();
            let size = param.param_type.size();

            // Align offset
            offset = (offset + alignment - 1) & !(alignment - 1);

            self.offsets.insert(param.name.clone(), offset);
            offset += size;
        }

        // Align to 16 bytes for buffer binding
        self.size = (offset + 15) & !15;
        self.data.resize(self.size, 0);
        self.dirty = true;
    }

    /// Set parameter value.
    pub fn set(&mut self, name: &str, value: ParameterValue) -> bool {
        if self.values.contains_key(name) {
            self.values.insert(name.into(), value);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Get parameter value.
    pub fn get(&self, name: &str) -> Option<&ParameterValue> {
        self.values.get(name)
    }

    /// Update data buffer.
    pub fn update_buffer(&mut self) {
        if !self.dirty {
            return;
        }

        for param in &self.parameters {
            if let Some(value) = self.values.get(&param.name) {
                if let Some(&offset) = self.offsets.get(&param.name) {
                    let size = param.param_type.size();
                    value.write_to(&mut self.data[offset..offset + size]);
                }
            }
        }

        self.dirty = false;
    }

    /// Get data buffer.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if dirty.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Iterate over parameters.
    pub fn parameters(&self) -> impl Iterator<Item = &Parameter> {
        self.parameters.iter()
    }

    /// Get parameter offset.
    pub fn offset(&self, name: &str) -> Option<usize> {
        self.offsets.get(name).copied()
    }
}

// ============================================================================
// Standard Parameter Blocks
// ============================================================================

/// Standard PBR parameter block.
pub fn create_pbr_block() -> ParameterBlock {
    let mut block = ParameterBlock::new("PBRMaterial");

    block.add_parameter(
        Parameter::color("base_color", [1.0, 1.0, 1.0, 1.0])
            .display_name("Base Color")
            .group("Base"),
    );

    block.add_parameter(
        Parameter::float("metallic", 0.0)
            .display_name("Metallic")
            .range(ParameterValue::Float(0.0), ParameterValue::Float(1.0))
            .group("PBR"),
    );

    block.add_parameter(
        Parameter::float("roughness", 0.5)
            .display_name("Roughness")
            .range(ParameterValue::Float(0.0), ParameterValue::Float(1.0))
            .group("PBR"),
    );

    block.add_parameter(
        Parameter::float("normal_scale", 1.0)
            .display_name("Normal Scale")
            .range(ParameterValue::Float(0.0), ParameterValue::Float(2.0))
            .group("Normal"),
    );

    block.add_parameter(
        Parameter::float("ao_strength", 1.0)
            .display_name("AO Strength")
            .range(ParameterValue::Float(0.0), ParameterValue::Float(1.0))
            .group("Occlusion"),
    );

    block.add_parameter(
        Parameter::new(
            "emissive",
            ParameterType::Vec3,
            ParameterValue::Vec3([0.0, 0.0, 0.0]),
        )
        .display_name("Emissive")
        .group("Emission"),
    );

    block.add_parameter(
        Parameter::float("emissive_strength", 1.0)
            .display_name("Emissive Strength")
            .range(ParameterValue::Float(0.0), ParameterValue::Float(10.0))
            .group("Emission"),
    );

    block.add_parameter(
        Parameter::float("alpha_cutoff", 0.5)
            .display_name("Alpha Cutoff")
            .range(ParameterValue::Float(0.0), ParameterValue::Float(1.0))
            .group("Alpha"),
    );

    block.add_parameter(
        Parameter::texture("albedo_texture")
            .display_name("Albedo Map")
            .group("Textures"),
    );

    block.add_parameter(
        Parameter::texture("normal_texture")
            .display_name("Normal Map")
            .group("Textures"),
    );

    block.add_parameter(
        Parameter::texture("metallic_roughness_texture")
            .display_name("Metallic-Roughness Map")
            .group("Textures"),
    );

    block.add_parameter(
        Parameter::texture("occlusion_texture")
            .display_name("Occlusion Map")
            .group("Textures"),
    );

    block.add_parameter(
        Parameter::texture("emissive_texture")
            .display_name("Emissive Map")
            .group("Textures"),
    );

    block
}

/// Standard view parameter block.
pub fn create_view_block() -> ParameterBlock {
    let mut block = ParameterBlock::new("ViewData");

    let identity = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    block.add_parameter(
        Parameter::new(
            "view_matrix",
            ParameterType::Mat4,
            ParameterValue::Mat4(identity),
        )
        .hidden(),
    );

    block.add_parameter(
        Parameter::new(
            "projection_matrix",
            ParameterType::Mat4,
            ParameterValue::Mat4(identity),
        )
        .hidden(),
    );

    block.add_parameter(
        Parameter::new(
            "view_projection_matrix",
            ParameterType::Mat4,
            ParameterValue::Mat4(identity),
        )
        .hidden(),
    );

    block.add_parameter(
        Parameter::new(
            "inverse_view_matrix",
            ParameterType::Mat4,
            ParameterValue::Mat4(identity),
        )
        .hidden(),
    );

    block.add_parameter(
        Parameter::new(
            "camera_position",
            ParameterType::Vec3,
            ParameterValue::Vec3([0.0, 0.0, 0.0]),
        )
        .hidden(),
    );

    block.add_parameter(
        Parameter::new(
            "viewport_size",
            ParameterType::Vec2,
            ParameterValue::Vec2([1920.0, 1080.0]),
        )
        .hidden(),
    );

    block.add_parameter(
        Parameter::new("time", ParameterType::Float, ParameterValue::Float(0.0)).hidden(),
    );

    block.add_parameter(
        Parameter::new(
            "delta_time",
            ParameterType::Float,
            ParameterValue::Float(0.016),
        )
        .hidden(),
    );

    block
}
