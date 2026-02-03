//! Parsing utilities for LUMINA shader macros.
//!
//! This module provides parsers for shader attributes and expressions.

use crate::error::{Error, ErrorKind, Result};
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Expr, ExprLit, FnArg, Ident, ItemFn, ItemMod, ItemStatic, ItemStruct, Lit, LitInt,
    Meta, MetaList, MetaNameValue, Pat, PatIdent, PatType, Path, Token, Type,
};

/// Parsed shader module.
#[derive(Debug)]
pub struct ShaderModule {
    /// Module name.
    pub name: Ident,
    /// Module attributes.
    pub attrs: ShaderModuleAttrs,
    /// Entry points.
    pub entry_points: Vec<EntryPoint>,
    /// Resource bindings.
    pub resources: Vec<Resource>,
    /// Struct definitions.
    pub structs: Vec<StructDef>,
    /// Shared variables.
    pub shared_vars: Vec<SharedVar>,
    /// Original module for passthrough.
    pub original: ItemMod,
}

/// Shader module attributes.
#[derive(Debug, Default)]
pub struct ShaderModuleAttrs {
    /// Target Vulkan version.
    pub target: Option<String>,
    /// Optimization level.
    pub optimize: Option<String>,
    /// Include debug info.
    pub debug: bool,
    /// Enable validation.
    pub validate: bool,
}

/// Parsed entry point.
#[derive(Debug)]
pub struct EntryPoint {
    /// Function name.
    pub name: Ident,
    /// Shader stage.
    pub stage: ShaderStage,
    /// Stage-specific attributes.
    pub stage_attrs: StageAttrs,
    /// Function parameters (inputs).
    pub inputs: Vec<ShaderInput>,
    /// Return type (outputs).
    pub output: Option<ShaderOutput>,
    /// Original function.
    pub function: ItemFn,
}

/// Shader stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
    Mesh,
    Task,
    RayGeneration,
    ClosestHit,
    AnyHit,
    Miss,
    Intersection,
    Callable,
}

impl ShaderStage {
    /// Get the SPIR-V execution model name.
    pub fn execution_model(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "Vertex",
            ShaderStage::Fragment => "Fragment",
            ShaderStage::Compute => "GLCompute",
            ShaderStage::Geometry => "Geometry",
            ShaderStage::TessellationControl => "TessellationControl",
            ShaderStage::TessellationEvaluation => "TessellationEvaluation",
            ShaderStage::Mesh => "MeshNV",
            ShaderStage::Task => "TaskNV",
            ShaderStage::RayGeneration => "RayGenerationKHR",
            ShaderStage::ClosestHit => "ClosestHitKHR",
            ShaderStage::AnyHit => "AnyHitKHR",
            ShaderStage::Miss => "MissKHR",
            ShaderStage::Intersection => "IntersectionKHR",
            ShaderStage::Callable => "CallableKHR",
        }
    }

    /// Check if this is a ray tracing stage.
    pub fn is_ray_tracing(&self) -> bool {
        matches!(
            self,
            ShaderStage::RayGeneration
                | ShaderStage::ClosestHit
                | ShaderStage::AnyHit
                | ShaderStage::Miss
                | ShaderStage::Intersection
                | ShaderStage::Callable
        )
    }

    /// Check if this is a mesh shading stage.
    pub fn is_mesh_shading(&self) -> bool {
        matches!(self, ShaderStage::Mesh | ShaderStage::Task)
    }

    /// Check if this stage supports shared memory.
    pub fn supports_shared_memory(&self) -> bool {
        matches!(
            self,
            ShaderStage::Compute | ShaderStage::Mesh | ShaderStage::Task
        )
    }
}

/// Stage-specific attributes.
#[derive(Debug, Default)]
pub struct StageAttrs {
    /// Compute/mesh/task local size.
    pub local_size: Option<(u32, u32, u32)>,
    /// Geometry input primitive.
    pub input_primitive: Option<String>,
    /// Geometry output primitive.
    pub output_primitive: Option<String>,
    /// Geometry max vertices.
    pub max_vertices: Option<u32>,
    /// Mesh max primitives.
    pub max_primitives: Option<u32>,
    /// Tessellation output vertices.
    pub output_vertices: Option<u32>,
    /// Tessellation mode.
    pub tess_mode: Option<String>,
    /// Tessellation spacing.
    pub tess_spacing: Option<String>,
    /// Tessellation winding.
    pub tess_winding: Option<String>,
}

/// Shader input parameter.
#[derive(Debug)]
pub struct ShaderInput {
    /// Parameter name.
    pub name: Ident,
    /// Parameter type.
    pub ty: Type,
    /// Location attribute.
    pub location: Option<u32>,
    /// Built-in attribute.
    pub builtin: Option<String>,
    /// Flat interpolation.
    pub flat: bool,
    /// No perspective interpolation.
    pub no_perspective: bool,
}

/// Shader output.
#[derive(Debug)]
pub struct ShaderOutput {
    /// Output type.
    pub ty: Type,
    /// Location (for simple outputs).
    pub location: Option<u32>,
    /// Built-in (for position, etc.).
    pub builtin: Option<String>,
}

/// Resource binding.
#[derive(Debug)]
pub struct Resource {
    /// Resource name.
    pub name: Ident,
    /// Resource kind.
    pub kind: ResourceKind,
    /// Binding number.
    pub binding: u32,
    /// Descriptor set.
    pub set: u32,
    /// Resource type.
    pub ty: Type,
    /// Access mode (for storage).
    pub access: Option<AccessMode>,
    /// Layout (for buffers).
    pub layout: Option<LayoutKind>,
    /// Original item.
    pub span: Span,
}

/// Resource kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    UniformBuffer,
    StorageBuffer,
    SampledImage,
    StorageImage,
    Sampler,
    CombinedImageSampler,
    InputAttachment,
    AccelerationStructure,
    PushConstant,
}

/// Access modes for storage resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

/// Layout kinds for buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKind {
    Std140,
    Std430,
    Scalar,
}

/// Struct definition.
#[derive(Debug)]
pub struct StructDef {
    /// Struct name.
    pub name: Ident,
    /// Struct fields.
    pub fields: Vec<StructField>,
    /// Is this an input struct.
    pub is_input: bool,
    /// Is this an output struct.
    pub is_output: bool,
    /// Original struct.
    pub original: ItemStruct,
}

/// Struct field.
#[derive(Debug)]
pub struct StructField {
    /// Field name.
    pub name: Ident,
    /// Field type.
    pub ty: Type,
    /// Location attribute.
    pub location: Option<u32>,
    /// Built-in attribute.
    pub builtin: Option<String>,
    /// Flat interpolation.
    pub flat: bool,
}

/// Shared variable.
#[derive(Debug)]
pub struct SharedVar {
    /// Variable name.
    pub name: Ident,
    /// Variable type.
    pub ty: Type,
    /// Original item.
    pub original: ItemStatic,
}

/// Parser for shader module attributes.
pub struct ShaderAttrParser;

impl ShaderAttrParser {
    /// Parse shader module attributes.
    pub fn parse_module_attrs(attr: proc_macro2::TokenStream) -> Result<ShaderModuleAttrs> {
        if attr.is_empty() {
            return Ok(ShaderModuleAttrs::default());
        }

        let args: AttributeArgs = syn::parse2(attr)
            .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

        let mut attrs = ShaderModuleAttrs::default();
        attrs.validate = true; // Default to true

        for arg in args.0 {
            match arg {
                NestedMeta::NameValue { name, value } => {
                    let name_str = name.to_string();
                    match name_str.as_str() {
                        "target" => {
                            attrs.target = Some(Self::expect_string(&value)?);
                        }
                        "optimize" => {
                            attrs.optimize = Some(Self::expect_string(&value)?);
                        }
                        "debug" => {
                            attrs.debug = Self::expect_bool(&value)?;
                        }
                        "validate" => {
                            attrs.validate = Self::expect_bool(&value)?;
                        }
                        _ => {
                            return Err(Error::invalid_attribute(
                                format!("unknown attribute: {}", name_str),
                                name.span(),
                            ));
                        }
                    }
                }
                NestedMeta::Path(path) => {
                    return Err(Error::invalid_attribute(
                        format!("expected key = value, got: {}", quote::quote!(#path)),
                        path.span(),
                    ));
                }
            }
        }

        Ok(attrs)
    }

    /// Parse stage attributes.
    pub fn parse_stage_attrs(
        stage: ShaderStage,
        attr: proc_macro2::TokenStream,
    ) -> Result<StageAttrs> {
        if attr.is_empty() {
            return Ok(StageAttrs::default());
        }

        let args: AttributeArgs = syn::parse2(attr)
            .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

        let mut attrs = StageAttrs::default();

        for arg in args.0 {
            match arg {
                NestedMeta::NameValue { name, value } => {
                    let name_str = name.to_string();
                    match name_str.as_str() {
                        "local_size" => {
                            attrs.local_size = Some(Self::parse_local_size(&value)?);
                        }
                        "input" => {
                            attrs.input_primitive = Some(Self::expect_ident(&value)?);
                        }
                        "output" => {
                            attrs.output_primitive = Some(Self::expect_ident(&value)?);
                        }
                        "max_vertices" => {
                            attrs.max_vertices = Some(Self::expect_u32(&value)?);
                        }
                        "max_primitives" => {
                            attrs.max_primitives = Some(Self::expect_u32(&value)?);
                        }
                        "output_vertices" => {
                            attrs.output_vertices = Some(Self::expect_u32(&value)?);
                        }
                        "mode" => {
                            attrs.tess_mode = Some(Self::expect_ident(&value)?);
                        }
                        "spacing" => {
                            attrs.tess_spacing = Some(Self::expect_ident(&value)?);
                        }
                        "winding" => {
                            attrs.tess_winding = Some(Self::expect_ident(&value)?);
                        }
                        _ => {
                            return Err(Error::invalid_attribute(
                                format!("unknown attribute for {:?}: {}", stage, name_str),
                                name.span(),
                            ));
                        }
                    }
                }
                NestedMeta::Path(path) => {
                    return Err(Error::invalid_attribute(
                        format!("expected key = value"),
                        path.span(),
                    ));
                }
            }
        }

        // Validate required attributes
        Self::validate_stage_attrs(stage, &attrs)?;

        Ok(attrs)
    }

    /// Parse resource attributes.
    pub fn parse_resource_attrs(
        kind: ResourceKind,
        attr: proc_macro2::TokenStream,
    ) -> Result<ResourceAttrs> {
        let args: AttributeArgs = syn::parse2(attr)
            .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

        let mut binding = None;
        let mut set = Some(0u32);
        let mut access = None;
        let mut layout = None;
        let mut dim = None;
        let mut format = None;

        for arg in args.0 {
            match arg {
                NestedMeta::NameValue { name, value } => {
                    let name_str = name.to_string();
                    match name_str.as_str() {
                        "binding" => {
                            binding = Some(Self::expect_u32(&value)?);
                        }
                        "set" => {
                            set = Some(Self::expect_u32(&value)?);
                        }
                        "access" => {
                            let access_str = Self::expect_ident(&value)?;
                            access = Some(match access_str.as_str() {
                                "read" => AccessMode::Read,
                                "write" => AccessMode::Write,
                                "read_write" => AccessMode::ReadWrite,
                                _ => {
                                    return Err(Error::invalid_attribute(
                                        format!("unknown access mode: {}", access_str),
                                        name.span(),
                                    ));
                                }
                            });
                        }
                        "layout" => {
                            let layout_str = Self::expect_ident(&value)?;
                            layout = Some(match layout_str.as_str() {
                                "std140" => LayoutKind::Std140,
                                "std430" => LayoutKind::Std430,
                                "scalar" => LayoutKind::Scalar,
                                _ => {
                                    return Err(Error::invalid_attribute(
                                        format!("unknown layout: {}", layout_str),
                                        name.span(),
                                    ));
                                }
                            });
                        }
                        "dim" => {
                            dim = Some(Self::expect_ident(&value)?);
                        }
                        "format" => {
                            format = Some(Self::expect_ident(&value)?);
                        }
                        _ => {
                            return Err(Error::invalid_attribute(
                                format!("unknown attribute: {}", name_str),
                                name.span(),
                            ));
                        }
                    }
                }
                NestedMeta::Path(path) => {
                    return Err(Error::invalid_attribute(
                        format!("expected key = value"),
                        path.span(),
                    ));
                }
            }
        }

        // Push constants don't need binding
        if kind != ResourceKind::PushConstant {
            if binding.is_none() {
                return Err(Error::missing_attribute(
                    "binding attribute is required",
                    Span::call_site(),
                ));
            }
        }

        Ok(ResourceAttrs {
            binding: binding.unwrap_or(0),
            set: set.unwrap_or(0),
            access,
            layout,
            dim,
            format,
        })
    }

    fn parse_local_size(value: &Expr) -> Result<(u32, u32, u32)> {
        // Parse (x, y, z) tuple
        match value {
            Expr::Tuple(tuple) => {
                if tuple.elems.len() != 3 {
                    return Err(Error::invalid_attribute(
                        "local_size requires (x, y, z)",
                        value.span(),
                    ));
                }
                let x = Self::expr_to_u32(&tuple.elems[0])?;
                let y = Self::expr_to_u32(&tuple.elems[1])?;
                let z = Self::expr_to_u32(&tuple.elems[2])?;
                Ok((x, y, z))
            }
            _ => Err(Error::invalid_attribute(
                "local_size must be a tuple (x, y, z)",
                value.span(),
            )),
        }
    }

    fn expr_to_u32(expr: &Expr) -> Result<u32> {
        match expr {
            Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) => {
                lit.base10_parse().map_err(|e| {
                    Error::invalid_attribute(format!("invalid integer: {}", e), lit.span())
                })
            }
            _ => Err(Error::invalid_attribute(
                "expected integer literal",
                expr.span(),
            )),
        }
    }

    fn expect_string(value: &Expr) -> Result<String> {
        match value {
            Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Ok(s.value()),
            _ => Err(Error::invalid_attribute(
                "expected string literal",
                value.span(),
            )),
        }
    }

    fn expect_bool(value: &Expr) -> Result<bool> {
        match value {
            Expr::Lit(ExprLit { lit: Lit::Bool(b), .. }) => Ok(b.value),
            Expr::Path(path) => {
                let ident = path.path.get_ident()
                    .ok_or_else(|| Error::invalid_attribute("expected bool", value.span()))?;
                match ident.to_string().as_str() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    _ => Err(Error::invalid_attribute("expected bool", value.span())),
                }
            }
            _ => Err(Error::invalid_attribute(
                "expected boolean literal",
                value.span(),
            )),
        }
    }

    fn expect_u32(value: &Expr) -> Result<u32> {
        Self::expr_to_u32(value)
    }

    fn expect_ident(value: &Expr) -> Result<String> {
        match value {
            Expr::Path(path) => {
                path.path.get_ident()
                    .map(|i| i.to_string())
                    .ok_or_else(|| Error::invalid_attribute("expected identifier", value.span()))
            }
            _ => Err(Error::invalid_attribute(
                "expected identifier",
                value.span(),
            )),
        }
    }

    fn validate_stage_attrs(stage: ShaderStage, attrs: &StageAttrs) -> Result<()> {
        match stage {
            ShaderStage::Compute | ShaderStage::Mesh | ShaderStage::Task => {
                if attrs.local_size.is_none() {
                    return Err(Error::missing_attribute(
                        format!("{:?} shader requires local_size attribute", stage),
                        Span::call_site(),
                    ));
                }
            }
            ShaderStage::Geometry => {
                if attrs.input_primitive.is_none() {
                    return Err(Error::missing_attribute(
                        "geometry shader requires input primitive type",
                        Span::call_site(),
                    ));
                }
                if attrs.output_primitive.is_none() {
                    return Err(Error::missing_attribute(
                        "geometry shader requires output primitive type",
                        Span::call_site(),
                    ));
                }
                if attrs.max_vertices.is_none() {
                    return Err(Error::missing_attribute(
                        "geometry shader requires max_vertices",
                        Span::call_site(),
                    ));
                }
            }
            ShaderStage::TessellationControl => {
                if attrs.output_vertices.is_none() {
                    return Err(Error::missing_attribute(
                        "tessellation control shader requires output_vertices",
                        Span::call_site(),
                    ));
                }
            }
            ShaderStage::TessellationEvaluation => {
                if attrs.tess_mode.is_none() {
                    return Err(Error::missing_attribute(
                        "tessellation evaluation shader requires mode",
                        Span::call_site(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Parsed resource attributes.
#[derive(Debug)]
pub struct ResourceAttrs {
    pub binding: u32,
    pub set: u32,
    pub access: Option<AccessMode>,
    pub layout: Option<LayoutKind>,
    pub dim: Option<String>,
    pub format: Option<String>,
}

/// Attribute arguments (key = value pairs).
struct AttributeArgs(Vec<NestedMeta>);

enum NestedMeta {
    Path(Path),
    NameValue { name: Ident, value: Expr },
}

impl Parse for AttributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = Vec::new();

        while !input.is_empty() {
            if input.peek(Ident) && input.peek2(Token![=]) {
                let name: Ident = input.parse()?;
                let _: Token![=] = input.parse()?;
                let value: Expr = input.parse()?;
                args.push(NestedMeta::NameValue { name, value });
            } else {
                let path: Path = input.parse()?;
                args.push(NestedMeta::Path(path));
            }

            if !input.is_empty() {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(AttributeArgs(args))
    }
}

/// Parse function inputs to shader inputs.
pub fn parse_function_inputs(func: &ItemFn) -> Result<Vec<ShaderInput>> {
    let mut inputs = Vec::new();

    for arg in &func.sig.inputs {
        match arg {
            FnArg::Receiver(_) => {
                return Err(Error::syntax(
                    "shader functions cannot have self parameter",
                    arg.span(),
                ));
            }
            FnArg::Typed(PatType { pat, ty, attrs, .. }) => {
                let name = match pat.as_ref() {
                    Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                    _ => {
                        return Err(Error::syntax(
                            "expected identifier pattern",
                            pat.span(),
                        ));
                    }
                };

                let (location, builtin, flat, no_perspective) = parse_input_attrs(attrs)?;

                inputs.push(ShaderInput {
                    name,
                    ty: ty.as_ref().clone(),
                    location,
                    builtin,
                    flat,
                    no_perspective,
                });
            }
        }
    }

    Ok(inputs)
}

fn parse_input_attrs(attrs: &[Attribute]) -> Result<(Option<u32>, Option<String>, bool, bool)> {
    let mut location = None;
    let mut builtin = None;
    let mut flat = false;
    let mut no_perspective = false;

    for attr in attrs {
        if attr.path().is_ident("location") {
            let loc: LitInt = attr.parse_args()
                .map_err(|e| Error::syntax(e.to_string(), attr.span()))?;
            location = Some(loc.base10_parse()
                .map_err(|e| Error::syntax(e.to_string(), loc.span()))?);
        } else if attr.path().is_ident("builtin") {
            let ident: Ident = attr.parse_args()
                .map_err(|e| Error::syntax(e.to_string(), attr.span()))?;
            builtin = Some(ident.to_string());
        } else if attr.path().is_ident("flat") {
            flat = true;
        } else if attr.path().is_ident("no_perspective") {
            no_perspective = true;
        }
    }

    Ok((location, builtin, flat, no_perspective))
}

/// Parse return type to shader output.
pub fn parse_return_type(func: &ItemFn) -> Result<Option<ShaderOutput>> {
    match &func.sig.output {
        syn::ReturnType::Default => Ok(None),
        syn::ReturnType::Type(_, ty) => {
            // Check for builtin attribute on return type
            let mut builtin = None;
            for attr in &func.attrs {
                if attr.path().is_ident("returns_builtin") {
                    let ident: Ident = attr.parse_args()
                        .map_err(|e| Error::syntax(e.to_string(), attr.span()))?;
                    builtin = Some(ident.to_string());
                }
            }

            Ok(Some(ShaderOutput {
                ty: ty.as_ref().clone(),
                location: None, // Determined by type
                builtin,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_attrs() {
        let attrs = ShaderAttrParser::parse_module_attrs(quote::quote!()).unwrap();
        assert!(attrs.target.is_none());
        assert!(attrs.validate);
    }

    #[test]
    fn test_shader_stage_execution_model() {
        assert_eq!(ShaderStage::Vertex.execution_model(), "Vertex");
        assert_eq!(ShaderStage::Fragment.execution_model(), "Fragment");
        assert_eq!(ShaderStage::Compute.execution_model(), "GLCompute");
    }

    #[test]
    fn test_shader_stage_categories() {
        assert!(ShaderStage::RayGeneration.is_ray_tracing());
        assert!(!ShaderStage::Vertex.is_ray_tracing());
        assert!(ShaderStage::Mesh.is_mesh_shading());
        assert!(ShaderStage::Compute.supports_shared_memory());
    }
}
