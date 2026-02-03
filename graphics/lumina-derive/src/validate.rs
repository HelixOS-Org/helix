//! Validation for shader code.
//!
//! This module performs semantic validation on analyzed shader code.

use crate::analyze::{
    AnalysisContext, AnalyzedEntryPoint, AnalyzedInput, AnalyzedOutput, AnalyzedResource,
    BuiltinVar, Interpolation,
};
use crate::error::{Diagnostics, Error, ErrorKind, Result};
use crate::parse::{ResourceKind, ShaderStage};
use crate::types::ShaderType;
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

/// Shader validator.
pub struct Validator {
    /// Validation options.
    pub options: ValidationOptions,
    /// Diagnostics collector.
    pub diagnostics: Diagnostics,
}

/// Validation options.
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// Target Vulkan version.
    pub target_vulkan: VulkanVersion,
    /// Maximum descriptor sets.
    pub max_descriptor_sets: u32,
    /// Maximum bindings per set.
    pub max_bindings_per_set: u32,
    /// Maximum push constant size.
    pub max_push_constant_size: u32,
    /// Maximum locations.
    pub max_locations: u32,
    /// Allow unused variables.
    pub allow_unused: bool,
    /// Strict mode.
    pub strict: bool,
}

/// Vulkan version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VulkanVersion {
    V1_0,
    V1_1,
    V1_2,
    V1_3,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            target_vulkan: VulkanVersion::V1_2,
            max_descriptor_sets: 8,
            max_bindings_per_set: 32,
            max_push_constant_size: 128,
            max_locations: 32,
            allow_unused: true,
            strict: false,
        }
    }
}

impl Validator {
    /// Create a new validator.
    pub fn new(options: ValidationOptions) -> Self {
        Self {
            options,
            diagnostics: Diagnostics::new(),
        }
    }

    /// Validate an analysis context.
    pub fn validate(&mut self, ctx: &AnalysisContext) -> Result<()> {
        // Validate entry points
        for entry in &ctx.entry_points {
            self.validate_entry_point(entry)?;
        }

        // Validate resources
        for resource in &ctx.resources {
            self.validate_resource(resource)?;
        }

        // Validate bindings
        self.validate_bindings(ctx)?;

        // Check for interface matching between stages
        self.validate_interface_matching(ctx)?;

        self.diagnostics.to_result()
    }

    fn validate_entry_point(&mut self, entry: &AnalyzedEntryPoint) -> Result<()> {
        // Validate stage-specific requirements
        match entry.stage {
            ShaderStage::Compute | ShaderStage::Mesh | ShaderStage::Task => {
                if entry.local_size.is_none() {
                    self.diagnostics.error(Error::missing_attribute(
                        format!("{:?} shader requires local_size", entry.stage),
                        Span::call_site(),
                    ));
                } else if let Some((x, y, z)) = entry.local_size {
                    // Validate workgroup size limits
                    let total = x * y * z;
                    if total > 1024 {
                        self.diagnostics.warning(
                            format!("workgroup size {} exceeds common limit of 1024", total),
                            Span::call_site(),
                        );
                    }
                    if x > 1024 || y > 1024 || z > 64 {
                        self.diagnostics.warning(
                            format!(
                                "individual workgroup dimensions ({}, {}, {}) may exceed hardware limits",
                                x, y, z
                            ),
                            Span::call_site(),
                        );
                    }
                }
            }
            ShaderStage::Vertex => {
                // Vertex shader should have Position output (usually)
                let has_position = entry.outputs.iter().any(|o| {
                    matches!(o.builtin, Some(BuiltinVar::Position))
                });
                if !has_position && self.options.strict {
                    self.diagnostics.warning(
                        "vertex shader has no Position output",
                        Span::call_site(),
                    );
                }
            }
            ShaderStage::Fragment => {
                // Fragment shader should have output
                if entry.outputs.is_empty() && self.options.strict {
                    self.diagnostics.warning(
                        "fragment shader has no output",
                        Span::call_site(),
                    );
                }
            }
            _ => {}
        }

        // Validate inputs
        self.validate_inputs(entry)?;

        // Validate outputs
        self.validate_outputs(entry)?;

        Ok(())
    }

    fn validate_inputs(&mut self, entry: &AnalyzedEntryPoint) -> Result<()> {
        let mut used_locations = HashSet::new();

        for input in &entry.inputs {
            // Check location conflicts
            if let Some(loc) = input.location {
                if !used_locations.insert(loc) {
                    self.diagnostics.error(Error::duplicate(
                        format!("location {} is used multiple times", loc),
                        Span::call_site(),
                    ));
                }

                if loc >= self.options.max_locations {
                    self.diagnostics.error(Error::invalid_binding(
                        format!(
                            "location {} exceeds maximum {}",
                            loc, self.options.max_locations
                        ),
                        Span::call_site(),
                    ));
                }
            }

            // Validate type is appropriate for input
            if input.ty.is_opaque() {
                self.diagnostics.error(Error::type_error(
                    format!("opaque types cannot be used as shader inputs"),
                    Span::call_site(),
                ));
            }

            // Validate interpolation
            if let Some(builtin) = &input.builtin {
                if input.interpolation != Interpolation::Smooth {
                    // Built-ins don't use interpolation attributes
                }
            } else if !matches!(entry.stage, ShaderStage::Fragment) {
                if input.interpolation == Interpolation::Flat
                    || input.interpolation == Interpolation::NoPerspective
                {
                    // Only fragment inputs can have interpolation modifiers
                    // (technically tess eval too but that's rare)
                }
            }
        }

        Ok(())
    }

    fn validate_outputs(&mut self, entry: &AnalyzedEntryPoint) -> Result<()> {
        let mut used_locations = HashSet::new();

        for output in &entry.outputs {
            if let Some(loc) = output.location {
                if !used_locations.insert(loc) {
                    self.diagnostics.error(Error::duplicate(
                        format!("output location {} is used multiple times", loc),
                        Span::call_site(),
                    ));
                }
            }

            // Validate output type
            if output.ty.is_opaque() && output.builtin.is_none() {
                self.diagnostics.error(Error::type_error(
                    "opaque types cannot be used as shader outputs",
                    Span::call_site(),
                ));
            }
        }

        Ok(())
    }

    fn validate_resource(&mut self, resource: &AnalyzedResource) -> Result<()> {
        // Validate set/binding limits
        if resource.set >= self.options.max_descriptor_sets {
            self.diagnostics.error(Error::invalid_binding(
                format!(
                    "descriptor set {} exceeds maximum {}",
                    resource.set,
                    self.options.max_descriptor_sets - 1
                ),
                Span::call_site(),
            ));
        }

        if resource.binding >= self.options.max_bindings_per_set {
            self.diagnostics.error(Error::invalid_binding(
                format!(
                    "binding {} exceeds maximum {} per set",
                    resource.binding,
                    self.options.max_bindings_per_set - 1
                ),
                Span::call_site(),
            ));
        }

        // Validate type matches resource kind
        match resource.kind {
            ResourceKind::UniformBuffer | ResourceKind::StorageBuffer => {
                if resource.ty.is_opaque() {
                    self.diagnostics.error(Error::type_error(
                        "buffer resources cannot have opaque types",
                        Span::call_site(),
                    ));
                }
            }
            ResourceKind::SampledImage | ResourceKind::StorageImage => {
                if !matches!(
                    resource.ty,
                    ShaderType::Texture2D { .. }
                        | ShaderType::Texture3D { .. }
                        | ShaderType::TextureCube { .. }
                        | ShaderType::Texture2DArray { .. }
                        | ShaderType::SampledImage { .. }
                        | ShaderType::StorageImage { .. }
                ) {
                    self.diagnostics.error(Error::type_error(
                        "image resources must have image types",
                        Span::call_site(),
                    ));
                }
            }
            ResourceKind::Sampler => {
                if !matches!(resource.ty, ShaderType::Sampler) {
                    self.diagnostics.error(Error::type_error(
                        "sampler resource must have Sampler type",
                        Span::call_site(),
                    ));
                }
            }
            ResourceKind::PushConstant => {
                // Validate push constant size
                let size = resource.ty.size();
                if size > self.options.max_push_constant_size {
                    self.diagnostics.error(Error::layout(
                        format!(
                            "push constant size {} exceeds maximum {}",
                            size, self.options.max_push_constant_size
                        ),
                        Span::call_site(),
                    ));
                }
            }
            ResourceKind::AccelerationStructure => {
                if !matches!(resource.ty, ShaderType::AccelerationStructure) {
                    self.diagnostics.error(Error::type_error(
                        "acceleration structure binding must have AccelerationStructure type",
                        Span::call_site(),
                    ));
                }

                // Check Vulkan version
                if self.options.target_vulkan < VulkanVersion::V1_2 {
                    self.diagnostics.error(Error::unsupported(
                        "ray tracing requires Vulkan 1.2 or later",
                        Span::call_site(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn validate_bindings(&mut self, ctx: &AnalysisContext) -> Result<()> {
        // Check for push constant conflicts
        let push_constants: Vec<_> = ctx
            .resources
            .iter()
            .filter(|r| r.kind == ResourceKind::PushConstant)
            .collect();

        // Group by stage
        let mut push_constants_by_stage: HashMap<ShaderStage, Vec<&AnalyzedResource>> =
            HashMap::new();
        for pc in &push_constants {
            for stage in &pc.used_by {
                push_constants_by_stage
                    .entry(*stage)
                    .or_default()
                    .push(pc);
            }
        }

        // Warn if multiple push constants per stage
        for (stage, pcs) in &push_constants_by_stage {
            if pcs.len() > 1 {
                self.diagnostics.warning(
                    format!(
                        "{:?} stage has {} push constant blocks, only one is typically supported",
                        stage,
                        pcs.len()
                    ),
                    Span::call_site(),
                );
            }
        }

        Ok(())
    }

    fn validate_interface_matching(&mut self, ctx: &AnalysisContext) -> Result<()> {
        // Find vertex and fragment shaders
        let vertex = ctx.entry_points.iter().find(|e| e.stage == ShaderStage::Vertex);
        let fragment = ctx.entry_points.iter().find(|e| e.stage == ShaderStage::Fragment);

        if let (Some(vs), Some(fs)) = (vertex, fragment) {
            // Check that vertex outputs match fragment inputs
            for fs_input in &fs.inputs {
                // Skip builtins
                if fs_input.builtin.is_some() {
                    continue;
                }

                if let Some(loc) = fs_input.location {
                    let matching_output = vs.outputs.iter().find(|o| o.location == Some(loc));

                    if let Some(vs_output) = matching_output {
                        // Type matching
                        if vs_output.ty != fs_input.ty {
                            self.diagnostics.warning(
                                format!(
                                    "interface mismatch at location {}: vertex outputs {:?}, fragment expects {:?}",
                                    loc, vs_output.ty, fs_input.ty
                                ),
                                Span::call_site(),
                            );
                        }
                    } else if self.options.strict {
                        self.diagnostics.warning(
                            format!(
                                "fragment input at location {} has no matching vertex output",
                                loc
                            ),
                            Span::call_site(),
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

/// Validate shader type compatibility.
pub fn types_compatible(a: &ShaderType, b: &ShaderType) -> bool {
    match (a, b) {
        (ShaderType::Bool, ShaderType::Bool) => true,
        (ShaderType::Int32, ShaderType::Int32) => true,
        (ShaderType::Uint32, ShaderType::Uint32) => true,
        (ShaderType::Float32, ShaderType::Float32) => true,
        (ShaderType::Float64, ShaderType::Float64) => true,
        (
            ShaderType::Vector { element: e1, size: s1 },
            ShaderType::Vector { element: e2, size: s2 },
        ) => s1 == s2 && types_compatible(e1, e2),
        (
            ShaderType::Matrix { element: e1, cols: c1, rows: r1 },
            ShaderType::Matrix { element: e2, cols: c2, rows: r2 },
        ) => c1 == c2 && r1 == r2 && types_compatible(e1, e2),
        (
            ShaderType::Array { element: e1, size: s1 },
            ShaderType::Array { element: e2, size: s2 },
        ) => s1 == s2 && types_compatible(e1, e2),
        (ShaderType::Struct { name: n1, .. }, ShaderType::Struct { name: n2, .. }) => n1 == n2,
        _ => false,
    }
}

/// Check if a type can be implicitly converted to another.
pub fn can_convert(from: &ShaderType, to: &ShaderType) -> bool {
    if types_compatible(from, to) {
        return true;
    }

    match (from, to) {
        // int -> float
        (ShaderType::Int32, ShaderType::Float32) => true,
        (ShaderType::Uint32, ShaderType::Float32) => true,
        // float -> double
        (ShaderType::Float32, ShaderType::Float64) => true,
        // Vector element conversions
        (
            ShaderType::Vector { element: e1, size: s1 },
            ShaderType::Vector { element: e2, size: s2 },
        ) if s1 == s2 => can_convert(e1, e2),
        // Matrix element conversions
        (
            ShaderType::Matrix { element: e1, cols: c1, rows: r1 },
            ShaderType::Matrix { element: e2, cols: c2, rows: r2 },
        ) if c1 == c2 && r1 == r2 => can_convert(e1, e2),
        _ => false,
    }
}

/// Check if binary operation is valid for types.
pub fn check_binary_op(op: &str, left: &ShaderType, right: &ShaderType) -> Option<ShaderType> {
    match op {
        "+" | "-" | "*" | "/" => {
            // Arithmetic operations
            match (left, right) {
                // Scalar operations
                (ShaderType::Int32, ShaderType::Int32) => Some(ShaderType::Int32),
                (ShaderType::Uint32, ShaderType::Uint32) => Some(ShaderType::Uint32),
                (ShaderType::Float32, ShaderType::Float32) => Some(ShaderType::Float32),
                (ShaderType::Float64, ShaderType::Float64) => Some(ShaderType::Float64),
                // Vector operations
                (
                    ShaderType::Vector { element: e1, size: s1 },
                    ShaderType::Vector { element: e2, size: s2 },
                ) if s1 == s2 && types_compatible(e1, e2) => Some(left.clone()),
                // Vector-scalar
                (ShaderType::Vector { element, size }, scalar)
                    if types_compatible(element, scalar) =>
                {
                    Some(left.clone())
                }
                (scalar, ShaderType::Vector { element, size })
                    if types_compatible(scalar, element) =>
                {
                    Some(right.clone())
                }
                // Matrix multiplication
                (
                    ShaderType::Matrix { element: e1, cols: c1, rows: r1 },
                    ShaderType::Matrix { element: e2, cols: c2, rows: r2 },
                ) if op == "*" && c1 == r2 && types_compatible(e1, e2) => {
                    Some(ShaderType::Matrix {
                        element: e1.clone(),
                        cols: *c2,
                        rows: *r1,
                    })
                }
                // Matrix-vector multiplication
                (
                    ShaderType::Matrix { element, cols, rows },
                    ShaderType::Vector { element: ve, size },
                ) if op == "*" && cols == size && types_compatible(element, ve) => {
                    Some(ShaderType::Vector {
                        element: element.clone(),
                        size: *rows,
                    })
                }
                _ => None,
            }
        }
        "%" => {
            // Modulo - integer only
            match (left, right) {
                (ShaderType::Int32, ShaderType::Int32) => Some(ShaderType::Int32),
                (ShaderType::Uint32, ShaderType::Uint32) => Some(ShaderType::Uint32),
                _ => None,
            }
        }
        "&" | "|" | "^" => {
            // Bitwise operations - integer only
            match (left, right) {
                (ShaderType::Int32, ShaderType::Int32) => Some(ShaderType::Int32),
                (ShaderType::Uint32, ShaderType::Uint32) => Some(ShaderType::Uint32),
                (
                    ShaderType::Vector { element: e1, size: s1 },
                    ShaderType::Vector { element: e2, size: s2 },
                ) if s1 == s2
                    && matches!(e1.as_ref(), ShaderType::Int32 | ShaderType::Uint32)
                    && types_compatible(e1, e2) =>
                {
                    Some(left.clone())
                }
                _ => None,
            }
        }
        "<<" | ">>" => {
            // Shift operations
            match (left, right) {
                (ShaderType::Int32, ShaderType::Int32 | ShaderType::Uint32) => {
                    Some(ShaderType::Int32)
                }
                (ShaderType::Uint32, ShaderType::Int32 | ShaderType::Uint32) => {
                    Some(ShaderType::Uint32)
                }
                _ => None,
            }
        }
        "==" | "!=" => {
            // Equality - returns bool
            if types_compatible(left, right) {
                Some(ShaderType::Bool)
            } else {
                None
            }
        }
        "<" | "<=" | ">" | ">=" => {
            // Comparison - numeric only
            match (left, right) {
                (ShaderType::Int32, ShaderType::Int32)
                | (ShaderType::Uint32, ShaderType::Uint32)
                | (ShaderType::Float32, ShaderType::Float32)
                | (ShaderType::Float64, ShaderType::Float64) => Some(ShaderType::Bool),
                _ => None,
            }
        }
        "&&" | "||" => {
            // Logical operations - bool only
            match (left, right) {
                (ShaderType::Bool, ShaderType::Bool) => Some(ShaderType::Bool),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_compatible() {
        assert!(types_compatible(&ShaderType::Float32, &ShaderType::Float32));
        assert!(!types_compatible(&ShaderType::Float32, &ShaderType::Int32));

        let vec3a = ShaderType::Vector {
            element: Box::new(ShaderType::Float32),
            size: 3,
        };
        let vec3b = ShaderType::Vector {
            element: Box::new(ShaderType::Float32),
            size: 3,
        };
        let vec4 = ShaderType::Vector {
            element: Box::new(ShaderType::Float32),
            size: 4,
        };

        assert!(types_compatible(&vec3a, &vec3b));
        assert!(!types_compatible(&vec3a, &vec4));
    }

    #[test]
    fn test_can_convert() {
        assert!(can_convert(&ShaderType::Int32, &ShaderType::Float32));
        assert!(can_convert(&ShaderType::Float32, &ShaderType::Float64));
        assert!(!can_convert(&ShaderType::Float32, &ShaderType::Int32));
    }

    #[test]
    fn test_binary_op() {
        let result = check_binary_op("+", &ShaderType::Float32, &ShaderType::Float32);
        assert_eq!(result, Some(ShaderType::Float32));

        let result = check_binary_op("+", &ShaderType::Float32, &ShaderType::Int32);
        assert_eq!(result, None);

        let result = check_binary_op("==", &ShaderType::Float32, &ShaderType::Float32);
        assert_eq!(result, Some(ShaderType::Bool));
    }

    #[test]
    fn test_validation_options() {
        let opts = ValidationOptions::default();
        assert_eq!(opts.max_descriptor_sets, 8);
        assert_eq!(opts.max_push_constant_size, 128);
    }
}
