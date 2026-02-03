//! Analysis passes for shader code.
//!
//! This module performs semantic analysis on parsed shader code,
//! including type checking, scope analysis, and dependency tracking.

use std::collections::{HashMap, HashSet};

use proc_macro2::Span;
use syn::spanned::Spanned;

use crate::error::{Diagnostics, Error, ErrorKind, Result};
use crate::parse::{
    AccessMode, EntryPoint, LayoutKind, Resource, ResourceKind, ShaderInput, ShaderModule,
    ShaderOutput, ShaderStage, StructDef,
};
use crate::types::{ShaderType, TypeConverter};

/// Analysis context for shader modules.
pub struct AnalysisContext {
    /// Type environment.
    pub types: TypeEnvironment,
    /// Resource bindings.
    pub bindings: BindingTracker,
    /// Variable scopes.
    pub scopes: ScopeStack,
    /// Diagnostics.
    pub diagnostics: Diagnostics,
    /// Analyzed entry points.
    pub entry_points: Vec<AnalyzedEntryPoint>,
    /// Analyzed resources.
    pub resources: Vec<AnalyzedResource>,
}

/// Analyzed entry point with type information.
pub struct AnalyzedEntryPoint {
    pub name: String,
    pub stage: ShaderStage,
    pub inputs: Vec<AnalyzedInput>,
    pub outputs: Vec<AnalyzedOutput>,
    pub local_size: Option<(u32, u32, u32)>,
    pub used_resources: HashSet<String>,
}

/// Analyzed shader input.
pub struct AnalyzedInput {
    pub name: String,
    pub ty: ShaderType,
    pub location: Option<u32>,
    pub builtin: Option<BuiltinVar>,
    pub interpolation: Interpolation,
}

/// Analyzed shader output.
pub struct AnalyzedOutput {
    pub name: String,
    pub ty: ShaderType,
    pub location: Option<u32>,
    pub builtin: Option<BuiltinVar>,
}

/// Analyzed resource.
pub struct AnalyzedResource {
    pub name: String,
    pub kind: ResourceKind,
    pub ty: ShaderType,
    pub binding: u32,
    pub set: u32,
    pub access: AccessMode,
    pub used_by: HashSet<ShaderStage>,
}

/// Built-in shader variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinVar {
    // Vertex shader
    VertexIndex,
    InstanceIndex,
    DrawIndex,
    BaseVertex,
    BaseInstance,

    // Fragment shader
    FragCoord,
    FrontFacing,
    PointCoord,
    SampleId,
    SamplePosition,
    SampleMask,
    HelperInvocation,

    // Both
    Position,
    PointSize,
    ClipDistance,
    CullDistance,
    Layer,
    ViewportIndex,
    FragDepth,

    // Compute shader
    NumWorkGroups,
    WorkGroupId,
    LocalInvocationId,
    GlobalInvocationId,
    LocalInvocationIndex,
    WorkGroupSize,
    SubgroupSize,
    SubgroupInvocationId,
    SubgroupEqMask,
    SubgroupGeMask,
    SubgroupGtMask,
    SubgroupLeMask,
    SubgroupLtMask,

    // Geometry shader
    PrimitiveId,
    InvocationId,

    // Tessellation
    TessLevelOuter,
    TessLevelInner,
    TessCoord,
    PatchVertices,

    // Mesh shader
    DrawMeshTasksCountNV,
    TaskCountNV,
    PrimitiveCountNV,
    PrimitiveIndicesNV,
    MeshViewCountNV,
    MeshViewIndicesNV,

    // Ray tracing
    LaunchIdKHR,
    LaunchSizeKHR,
    WorldRayOriginKHR,
    WorldRayDirectionKHR,
    ObjectRayOriginKHR,
    ObjectRayDirectionKHR,
    RayTminKHR,
    RayTmaxKHR,
    IncomingRayFlagsKHR,
    HitTKHR,
    HitKindKHR,
    ObjectToWorldKHR,
    WorldToObjectKHR,
    InstanceCustomIndexKHR,
    InstanceId,
    GeometryIndexKHR,
    PrimitiveIdKHR,
    RayGeometryIndexKHR,
}

impl BuiltinVar {
    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "VertexIndex" | "gl_VertexIndex" => Some(Self::VertexIndex),
            "InstanceIndex" | "gl_InstanceIndex" => Some(Self::InstanceIndex),
            "DrawIndex" | "gl_DrawIndex" => Some(Self::DrawIndex),
            "BaseVertex" | "gl_BaseVertex" => Some(Self::BaseVertex),
            "BaseInstance" | "gl_BaseInstance" => Some(Self::BaseInstance),
            "FragCoord" | "gl_FragCoord" => Some(Self::FragCoord),
            "FrontFacing" | "gl_FrontFacing" => Some(Self::FrontFacing),
            "PointCoord" | "gl_PointCoord" => Some(Self::PointCoord),
            "SampleId" | "gl_SampleId" => Some(Self::SampleId),
            "SamplePosition" | "gl_SamplePosition" => Some(Self::SamplePosition),
            "Position" | "gl_Position" => Some(Self::Position),
            "PointSize" | "gl_PointSize" => Some(Self::PointSize),
            "FragDepth" | "gl_FragDepth" => Some(Self::FragDepth),
            "NumWorkGroups" | "gl_NumWorkGroups" => Some(Self::NumWorkGroups),
            "WorkGroupId" | "gl_WorkGroupId" => Some(Self::WorkGroupId),
            "LocalInvocationId" | "gl_LocalInvocationId" => Some(Self::LocalInvocationId),
            "GlobalInvocationId" | "gl_GlobalInvocationId" => Some(Self::GlobalInvocationId),
            "LocalInvocationIndex" | "gl_LocalInvocationIndex" => Some(Self::LocalInvocationIndex),
            "PrimitiveId" | "gl_PrimitiveId" => Some(Self::PrimitiveId),
            "InvocationId" | "gl_InvocationId" => Some(Self::InvocationId),
            "TessLevelOuter" | "gl_TessLevelOuter" => Some(Self::TessLevelOuter),
            "TessLevelInner" | "gl_TessLevelInner" => Some(Self::TessLevelInner),
            "TessCoord" | "gl_TessCoord" => Some(Self::TessCoord),
            "LaunchIdKHR" | "gl_LaunchIDKHR" => Some(Self::LaunchIdKHR),
            "LaunchSizeKHR" | "gl_LaunchSizeKHR" => Some(Self::LaunchSizeKHR),
            "WorldRayOriginKHR" => Some(Self::WorldRayOriginKHR),
            "WorldRayDirectionKHR" => Some(Self::WorldRayDirectionKHR),
            "RayTminKHR" => Some(Self::RayTminKHR),
            "RayTmaxKHR" => Some(Self::RayTmaxKHR),
            "HitTKHR" => Some(Self::HitTKHR),
            "HitKindKHR" => Some(Self::HitKindKHR),
            _ => None,
        }
    }

    /// Get the shader type for this builtin.
    pub fn shader_type(&self) -> ShaderType {
        match self {
            // Integer scalars
            Self::VertexIndex
            | Self::InstanceIndex
            | Self::DrawIndex
            | Self::BaseVertex
            | Self::BaseInstance
            | Self::SampleId
            | Self::LocalInvocationIndex
            | Self::PrimitiveId
            | Self::InvocationId
            | Self::PatchVertices
            | Self::HitKindKHR
            | Self::InstanceCustomIndexKHR
            | Self::InstanceId
            | Self::GeometryIndexKHR
            | Self::PrimitiveIdKHR => ShaderType::Int32,

            // Float scalars
            Self::PointSize
            | Self::FragDepth
            | Self::HitTKHR
            | Self::RayTminKHR
            | Self::RayTmaxKHR => ShaderType::Float32,

            // Bool
            Self::FrontFacing | Self::HelperInvocation => ShaderType::Bool,

            // Vec2
            Self::PointCoord | Self::SamplePosition => ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 2,
            },

            // Vec3
            Self::TessCoord
            | Self::WorldRayOriginKHR
            | Self::WorldRayDirectionKHR
            | Self::ObjectRayOriginKHR
            | Self::ObjectRayDirectionKHR => ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 3,
            },

            // Vec4
            Self::FragCoord | Self::Position => ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 4,
            },

            // UVec3
            Self::NumWorkGroups
            | Self::WorkGroupId
            | Self::LocalInvocationId
            | Self::GlobalInvocationId
            | Self::WorkGroupSize
            | Self::LaunchIdKHR
            | Self::LaunchSizeKHR => ShaderType::Vector {
                element: Box::new(ShaderType::Uint32),
                size: 3,
            },

            // UVec4
            Self::SubgroupEqMask
            | Self::SubgroupGeMask
            | Self::SubgroupGtMask
            | Self::SubgroupLeMask
            | Self::SubgroupLtMask => ShaderType::Vector {
                element: Box::new(ShaderType::Uint32),
                size: 4,
            },

            // Float array[4]
            Self::TessLevelOuter => ShaderType::Array {
                element: Box::new(ShaderType::Float32),
                size: 4,
            },

            // Float array[2]
            Self::TessLevelInner => ShaderType::Array {
                element: Box::new(ShaderType::Float32),
                size: 2,
            },

            // Float array (size varies)
            Self::ClipDistance | Self::CullDistance => ShaderType::RuntimeArray {
                element: Box::new(ShaderType::Float32),
            },

            // Mat4x3
            Self::ObjectToWorldKHR | Self::WorldToObjectKHR => ShaderType::Matrix {
                element: Box::new(ShaderType::Float32),
                cols: 4,
                rows: 3,
            },

            // Int
            Self::Layer
            | Self::ViewportIndex
            | Self::SampleMask
            | Self::SubgroupSize
            | Self::SubgroupInvocationId
            | Self::IncomingRayFlagsKHR
            | Self::RayGeometryIndexKHR => ShaderType::Int32,

            // Mesh shader specifics
            Self::DrawMeshTasksCountNV
            | Self::TaskCountNV
            | Self::PrimitiveCountNV
            | Self::MeshViewCountNV => ShaderType::Uint32,

            Self::PrimitiveIndicesNV | Self::MeshViewIndicesNV => ShaderType::Array {
                element: Box::new(ShaderType::Uint32),
                size: 0, // Runtime determined
            },
        }
    }

    /// Check if this builtin is valid for a shader stage.
    pub fn is_valid_for_stage(&self, stage: ShaderStage) -> bool {
        match self {
            Self::VertexIndex
            | Self::InstanceIndex
            | Self::DrawIndex
            | Self::BaseVertex
            | Self::BaseInstance => matches!(stage, ShaderStage::Vertex),

            Self::FragCoord
            | Self::FrontFacing
            | Self::PointCoord
            | Self::SampleId
            | Self::SamplePosition
            | Self::SampleMask
            | Self::HelperInvocation
            | Self::FragDepth => matches!(stage, ShaderStage::Fragment),

            Self::Position | Self::PointSize | Self::ClipDistance | Self::CullDistance => {
                matches!(
                    stage,
                    ShaderStage::Vertex
                        | ShaderStage::Geometry
                        | ShaderStage::TessellationEvaluation
                        | ShaderStage::Mesh
                )
            },

            Self::NumWorkGroups
            | Self::WorkGroupId
            | Self::LocalInvocationId
            | Self::GlobalInvocationId
            | Self::LocalInvocationIndex
            | Self::WorkGroupSize => {
                matches!(
                    stage,
                    ShaderStage::Compute | ShaderStage::Mesh | ShaderStage::Task
                )
            },

            Self::SubgroupSize
            | Self::SubgroupInvocationId
            | Self::SubgroupEqMask
            | Self::SubgroupGeMask
            | Self::SubgroupGtMask
            | Self::SubgroupLeMask
            | Self::SubgroupLtMask => true, // Available in all stages

            Self::PrimitiveId | Self::InvocationId => {
                matches!(
                    stage,
                    ShaderStage::Geometry
                        | ShaderStage::TessellationControl
                        | ShaderStage::Fragment
                )
            },

            Self::TessLevelOuter | Self::TessLevelInner | Self::TessCoord | Self::PatchVertices => {
                matches!(
                    stage,
                    ShaderStage::TessellationControl | ShaderStage::TessellationEvaluation
                )
            },

            Self::Layer | Self::ViewportIndex => {
                matches!(
                    stage,
                    ShaderStage::Vertex
                        | ShaderStage::Geometry
                        | ShaderStage::TessellationEvaluation
                        | ShaderStage::Fragment
                        | ShaderStage::Mesh
                )
            },

            // Mesh shader builtins
            Self::DrawMeshTasksCountNV
            | Self::TaskCountNV
            | Self::PrimitiveCountNV
            | Self::PrimitiveIndicesNV
            | Self::MeshViewCountNV
            | Self::MeshViewIndicesNV => {
                matches!(stage, ShaderStage::Mesh | ShaderStage::Task)
            },

            // Ray tracing builtins
            Self::LaunchIdKHR
            | Self::LaunchSizeKHR
            | Self::WorldRayOriginKHR
            | Self::WorldRayDirectionKHR
            | Self::ObjectRayOriginKHR
            | Self::ObjectRayDirectionKHR
            | Self::RayTminKHR
            | Self::RayTmaxKHR
            | Self::IncomingRayFlagsKHR
            | Self::HitTKHR
            | Self::HitKindKHR
            | Self::ObjectToWorldKHR
            | Self::WorldToObjectKHR
            | Self::InstanceCustomIndexKHR
            | Self::InstanceId
            | Self::GeometryIndexKHR
            | Self::PrimitiveIdKHR
            | Self::RayGeometryIndexKHR => stage.is_ray_tracing(),
        }
    }
}

/// Interpolation mode for shader inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    #[default]
    Smooth,
    Flat,
    NoPerspective,
    Centroid,
    Sample,
}

/// Type environment for type checking.
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    /// Known types.
    types: HashMap<String, ShaderType>,
    /// Struct definitions.
    structs: HashMap<String, Vec<(String, ShaderType)>>,
}

impl TypeEnvironment {
    /// Create a new type environment.
    pub fn new() -> Self {
        let mut env = Self::default();
        // Register built-in types
        env.register_builtins();
        env
    }

    fn register_builtins(&mut self) {
        // Scalar types
        self.types.insert("bool".to_string(), ShaderType::Bool);
        self.types.insert("i32".to_string(), ShaderType::Int32);
        self.types.insert("u32".to_string(), ShaderType::Uint32);
        self.types.insert("f32".to_string(), ShaderType::Float32);
        self.types.insert("f64".to_string(), ShaderType::Float64);

        // Vector types
        for size in [2, 3, 4] {
            self.types
                .insert(format!("Vec{}", size), ShaderType::Vector {
                    element: Box::new(ShaderType::Float32),
                    size,
                });
            self.types
                .insert(format!("IVec{}", size), ShaderType::Vector {
                    element: Box::new(ShaderType::Int32),
                    size,
                });
            self.types
                .insert(format!("UVec{}", size), ShaderType::Vector {
                    element: Box::new(ShaderType::Uint32),
                    size,
                });
            self.types
                .insert(format!("DVec{}", size), ShaderType::Vector {
                    element: Box::new(ShaderType::Float64),
                    size,
                });
            self.types
                .insert(format!("BVec{}", size), ShaderType::Vector {
                    element: Box::new(ShaderType::Bool),
                    size,
                });
        }

        // Matrix types
        for (name, cols, rows) in [
            ("Mat2", 2, 2),
            ("Mat3", 3, 3),
            ("Mat4", 4, 4),
            ("Mat2x2", 2, 2),
            ("Mat2x3", 2, 3),
            ("Mat2x4", 2, 4),
            ("Mat3x2", 3, 2),
            ("Mat3x3", 3, 3),
            ("Mat3x4", 3, 4),
            ("Mat4x2", 4, 2),
            ("Mat4x3", 4, 3),
            ("Mat4x4", 4, 4),
        ] {
            self.types.insert(name.to_string(), ShaderType::Matrix {
                element: Box::new(ShaderType::Float32),
                cols,
                rows,
            });
        }
    }

    /// Look up a type by name.
    pub fn lookup(&self, name: &str) -> Option<&ShaderType> {
        self.types.get(name)
    }

    /// Register a struct type.
    pub fn register_struct(&mut self, name: &str, members: Vec<(String, ShaderType)>) {
        let ty = ShaderType::Struct {
            name: name.to_string(),
            members: members
                .iter()
                .map(|(n, t)| crate::types::StructMember {
                    name: n.clone(),
                    ty: t.clone(),
                    offset: None,
                })
                .collect(),
        };
        self.types.insert(name.to_string(), ty);
        self.structs.insert(name.to_string(), members);
    }

    /// Get struct members.
    pub fn get_struct_members(&self, name: &str) -> Option<&[(String, ShaderType)]> {
        self.structs.get(name).map(|v| v.as_slice())
    }
}

/// Binding tracker for resources.
#[derive(Debug, Default)]
pub struct BindingTracker {
    /// Bindings by set.
    bindings: HashMap<u32, HashMap<u32, ResourceBinding>>,
}

#[derive(Debug, Clone)]
struct ResourceBinding {
    name: String,
    kind: ResourceKind,
    span: Span,
}

impl BindingTracker {
    /// Create a new binding tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a binding.
    pub fn register(
        &mut self,
        name: String,
        set: u32,
        binding: u32,
        kind: ResourceKind,
        span: Span,
    ) -> Result<()> {
        let set_bindings = self.bindings.entry(set).or_default();

        if let Some(existing) = set_bindings.get(&binding) {
            return Err(Error::duplicate(
                format!(
                    "binding {} in set {} is already used by '{}'",
                    binding, set, existing.name
                ),
                span,
            ));
        }

        set_bindings.insert(binding, ResourceBinding { name, kind, span });

        Ok(())
    }

    /// Get all sets used.
    pub fn sets(&self) -> Vec<u32> {
        let mut sets: Vec<_> = self.bindings.keys().copied().collect();
        sets.sort();
        sets
    }

    /// Get bindings for a set.
    pub fn bindings_for_set(&self, set: u32) -> Vec<(u32, &str, ResourceKind)> {
        self.bindings
            .get(&set)
            .map(|bindings| {
                let mut result: Vec<_> = bindings
                    .iter()
                    .map(|(b, r)| (*b, r.name.as_str(), r.kind))
                    .collect();
                result.sort_by_key(|(b, _, _)| *b);
                result
            })
            .unwrap_or_default()
    }
}

/// Scope stack for variable tracking.
#[derive(Debug, Default)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

#[derive(Debug, Default)]
struct Scope {
    variables: HashMap<String, VariableInfo>,
}

#[derive(Debug, Clone)]
struct VariableInfo {
    ty: ShaderType,
    mutable: bool,
    used: bool,
}

impl ScopeStack {
    /// Create a new scope stack.
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::default()],
        }
    }

    /// Push a new scope.
    pub fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    /// Pop a scope.
    pub fn pop(&mut self) -> Option<HashMap<String, ShaderType>> {
        self.scopes
            .pop()
            .map(|s| s.variables.into_iter().map(|(n, v)| (n, v.ty)).collect())
    }

    /// Define a variable.
    pub fn define(&mut self, name: &str, ty: ShaderType, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.variables.insert(name.to_string(), VariableInfo {
                ty,
                mutable,
                used: false,
            });
        }
    }

    /// Look up a variable.
    pub fn lookup(&self, name: &str) -> Option<&ShaderType> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.variables.get(name) {
                return Some(&var.ty);
            }
        }
        None
    }

    /// Mark a variable as used.
    pub fn mark_used(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.variables.get_mut(name) {
                var.used = true;
                return;
            }
        }
    }

    /// Check if a variable is mutable.
    pub fn is_mutable(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.variables.get(name) {
                return var.mutable;
            }
        }
        false
    }
}

impl AnalysisContext {
    /// Create a new analysis context.
    pub fn new() -> Self {
        Self {
            types: TypeEnvironment::new(),
            bindings: BindingTracker::new(),
            scopes: ScopeStack::new(),
            diagnostics: Diagnostics::new(),
            entry_points: Vec::new(),
            resources: Vec::new(),
        }
    }

    /// Analyze a shader module.
    pub fn analyze(&mut self, module: &ShaderModule) -> Result<()> {
        // Register struct types
        for struct_def in &module.structs {
            self.analyze_struct(struct_def)?;
        }

        // Analyze resources
        for resource in &module.resources {
            self.analyze_resource(resource)?;
        }

        // Analyze entry points
        for entry_point in &module.entry_points {
            self.analyze_entry_point(entry_point)?;
        }

        // Check for errors
        self.diagnostics.to_result()
    }

    fn analyze_struct(&mut self, struct_def: &StructDef) -> Result<()> {
        let mut members = Vec::new();

        for field in &struct_def.fields {
            let ty = TypeConverter::convert(&field.ty)?;
            members.push((field.name.to_string(), ty));
        }

        self.types
            .register_struct(&struct_def.name.to_string(), members);
        Ok(())
    }

    fn analyze_resource(&mut self, resource: &Resource) -> Result<()> {
        let ty = TypeConverter::convert(&resource.ty)?;

        // Register binding
        self.bindings.register(
            resource.name.to_string(),
            resource.set,
            resource.binding,
            resource.kind,
            resource.span,
        )?;

        self.resources.push(AnalyzedResource {
            name: resource.name.to_string(),
            kind: resource.kind,
            ty,
            binding: resource.binding,
            set: resource.set,
            access: resource.access.unwrap_or(AccessMode::Read),
            used_by: HashSet::new(),
        });

        Ok(())
    }

    fn analyze_entry_point(&mut self, entry: &EntryPoint) -> Result<()> {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        // Analyze inputs
        for input in &entry.inputs {
            let ty = TypeConverter::convert(&input.ty)?;

            // Validate builtin
            if let Some(ref builtin_name) = input.builtin {
                if let Some(builtin) = BuiltinVar::from_str(builtin_name) {
                    if !builtin.is_valid_for_stage(entry.stage) {
                        self.diagnostics.error(Error::invalid_stage(
                            format!(
                                "builtin {} is not valid for {:?} shader",
                                builtin_name, entry.stage
                            ),
                            entry.function.sig.ident.span(),
                        ));
                    }
                } else {
                    self.diagnostics.error(Error::invalid_attribute(
                        format!("unknown builtin: {}", builtin_name),
                        entry.function.sig.ident.span(),
                    ));
                }
            }

            inputs.push(AnalyzedInput {
                name: input.name.to_string(),
                ty,
                location: input.location,
                builtin: input.builtin.as_ref().and_then(|s| BuiltinVar::from_str(s)),
                interpolation: if input.flat {
                    Interpolation::Flat
                } else if input.no_perspective {
                    Interpolation::NoPerspective
                } else {
                    Interpolation::Smooth
                },
            });
        }

        // Analyze output
        if let Some(ref output) = entry.output {
            let ty = TypeConverter::convert(&output.ty)?;

            outputs.push(AnalyzedOutput {
                name: "output".to_string(),
                ty,
                location: output.location,
                builtin: output
                    .builtin
                    .as_ref()
                    .and_then(|s| BuiltinVar::from_str(s)),
            });
        }

        self.entry_points.push(AnalyzedEntryPoint {
            name: entry.name.to_string(),
            stage: entry.stage,
            inputs,
            outputs,
            local_size: entry.stage_attrs.local_size,
            used_resources: HashSet::new(),
        });

        Ok(())
    }
}

impl Default for AnalysisContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_environment() {
        let env = TypeEnvironment::new();

        assert!(env.lookup("Vec3").is_some());
        assert!(env.lookup("Mat4").is_some());
        assert!(env.lookup("f32").is_some());
    }

    #[test]
    fn test_binding_tracker() {
        let mut tracker = BindingTracker::new();

        tracker
            .register(
                "buffer".to_string(),
                0,
                0,
                ResourceKind::UniformBuffer,
                Span::call_site(),
            )
            .unwrap();

        // Duplicate should fail
        let result = tracker.register(
            "buffer2".to_string(),
            0,
            0,
            ResourceKind::StorageBuffer,
            Span::call_site(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_scope_stack() {
        let mut scopes = ScopeStack::new();

        scopes.define("x", ShaderType::Float32, false);
        assert!(scopes.lookup("x").is_some());

        scopes.push();
        scopes.define("y", ShaderType::Int32, true);
        assert!(scopes.lookup("y").is_some());
        assert!(scopes.lookup("x").is_some()); // Still visible

        scopes.pop();
        assert!(scopes.lookup("y").is_none()); // Out of scope
        assert!(scopes.lookup("x").is_some());
    }

    #[test]
    fn test_builtin_validation() {
        assert!(BuiltinVar::VertexIndex.is_valid_for_stage(ShaderStage::Vertex));
        assert!(!BuiltinVar::VertexIndex.is_valid_for_stage(ShaderStage::Fragment));
        assert!(BuiltinVar::FragCoord.is_valid_for_stage(ShaderStage::Fragment));
        assert!(BuiltinVar::GlobalInvocationId.is_valid_for_stage(ShaderStage::Compute));
    }
}
