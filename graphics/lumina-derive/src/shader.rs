//! Shader macro implementations.
//!
//! This module contains the core implementation of shader proc macros.

use crate::analyze::AnalysisContext;
use crate::codegen::CodeGenerator;
use crate::error::{Error, ErrorKind, Result};
use crate::ir_gen::IrGenerator;
use crate::parse::{
    parse_function_inputs, parse_return_type, EntryPoint, Resource, ResourceAttrs, ResourceKind,
    ShaderAttrParser, ShaderModule, ShaderModuleAttrs, ShaderStage, StageAttrs, StructDef,
    StructField,
};
use crate::validate::{ValidationOptions, Validator, VulkanVersion};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{parse2, Attribute, Ident, Item, ItemFn, ItemMod, ItemStatic, ItemStruct, LitInt};

/// Shader module attribute implementation.
pub fn shader_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    // Parse the module
    let module: ItemMod = parse2(item)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    // Parse attributes
    let attrs = ShaderAttrParser::parse_module_attrs(attr)?;

    // Extract shader module contents
    let shader_module = extract_shader_module(&module, attrs)?;

    // Analyze
    let mut ctx = AnalysisContext::new();
    ctx.analyze(&shader_module)?;

    // Validate
    let validation_opts = ValidationOptions {
        target_vulkan: parse_vulkan_version(shader_module.attrs.target.as_deref()),
        strict: true,
        ..Default::default()
    };
    let mut validator = Validator::new(validation_opts);
    validator.validate(&ctx)?;

    // Generate IR
    let mut ir_gen = IrGenerator::new();
    let ir = ir_gen.generate(&ctx)?;

    // Generate code
    let code_gen = CodeGenerator::new(&module.ident.to_string())
        .with_debug(shader_module.attrs.debug);
    let generated = code_gen.generate(&ir, &ctx)?;

    // Combine original module with generated code
    let vis = &module.vis;
    let ident = &module.ident;
    let original_content = if let Some((_, items)) = &module.content {
        quote! { #(#items)* }
    } else {
        quote! {}
    };

    Ok(quote! {
        #vis mod #ident {
            #original_content
            #generated
        }
    })
}

/// Entry point attribute implementation.
pub fn entry_point_impl(
    stage: ShaderStage,
    attr: TokenStream,
    item: TokenStream,
) -> Result<TokenStream> {
    let func: ItemFn = parse2(item)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    // Parse stage attributes
    let stage_attrs = ShaderAttrParser::parse_stage_attrs(stage, attr)?;

    // Validate function signature
    validate_entry_point_signature(&func, stage)?;

    // Parse inputs and outputs
    let inputs = parse_function_inputs(&func)?;
    let output = parse_return_type(&func)?;

    // Generate marker attribute for later processing
    let stage_name = format!("{:?}", stage);
    let func_name = &func.sig.ident;

    // Store entry point info as an attribute for module-level processing
    let local_size_attr = if let Some((x, y, z)) = stage_attrs.local_size {
        quote! {
            #[doc(hidden)]
            #[allow(dead_code)]
            const _LUMINA_LOCAL_SIZE: (u32, u32, u32) = (#x, #y, #z);
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #[doc = concat!("LUMINA ", #stage_name, " shader entry point")]
        #func

        #local_size_attr
    })
}

/// Resource attribute implementation.
pub fn resource_impl(
    kind: ResourceKind,
    attr: TokenStream,
    item: TokenStream,
) -> Result<TokenStream> {
    let struct_item: ItemStruct = parse2(item.clone())
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    // Parse resource attributes
    let res_attrs = ShaderAttrParser::parse_resource_attrs(kind, attr)?;

    let struct_name = &struct_item.ident;
    let binding = res_attrs.binding;
    let set = res_attrs.set;
    let kind_name = format!("{:?}", kind);

    // Generate accessor and marker
    let accessor_name = Ident::new(
        &format!("_lumina_resource_{}", struct_name.to_string().to_lowercase()),
        struct_name.span(),
    );

    Ok(quote! {
        #struct_item

        #[doc(hidden)]
        #[allow(dead_code)]
        mod #accessor_name {
            pub const BINDING: u32 = #binding;
            pub const SET: u32 = #set;
            pub const KIND: &str = #kind_name;
        }
    })
}

/// Location attribute implementation.
pub fn location_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    // Parse location number
    let location: LitInt = parse2(attr)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    // This attribute is handled during function parsing
    // Just pass through the item with documentation
    let loc_value: u32 = location.base10_parse()
        .map_err(|e| Error::syntax(e.to_string(), location.span()))?;

    // Item could be a function parameter or struct field
    Ok(quote! {
        #[doc = concat!("Location ", stringify!(#loc_value))]
        #item
    })
}

/// Builtin attribute implementation.
pub fn builtin_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let builtin_name: Ident = parse2(attr)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    // Validate builtin name
    let builtin_str = builtin_name.to_string();
    if crate::analyze::BuiltinVar::from_str(&builtin_str).is_none() {
        return Err(Error::invalid_attribute(
            format!("unknown builtin: {}", builtin_str),
            builtin_name.span(),
        ));
    }

    Ok(quote! {
        #[doc = concat!("Built-in: ", #builtin_str)]
        #item
    })
}

/// Interface struct implementation.
pub fn interface_impl(
    kind: InterfaceKind,
    _attr: TokenStream,
    item: TokenStream,
) -> Result<TokenStream> {
    let struct_item: ItemStruct = parse2(item)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    let kind_name = match kind {
        InterfaceKind::Input => "input",
        InterfaceKind::Output => "output",
    };

    Ok(quote! {
        #[doc = concat!("Shader ", #kind_name, " interface")]
        #[repr(C)]
        #struct_item
    })
}

/// Shared memory implementation.
pub fn shared_impl(_attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let static_item: ItemStatic = parse2(item)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    let name = &static_item.ident;

    Ok(quote! {
        #[doc = "Workgroup shared memory"]
        #static_item

        #[doc(hidden)]
        const _LUMINA_SHARED: bool = true;
    })
}

/// SPIR-V assembly inline implementation.
pub fn spirv_asm_impl(input: TokenStream) -> Result<TokenStream> {
    // Parse assembly strings
    // For now, just return a placeholder
    Err(Error::unsupported(
        "inline SPIR-V assembly is not yet supported",
        Span::call_site(),
    ))
}

/// Include SPIR-V implementation.
pub fn include_spirv_impl(input: TokenStream) -> Result<TokenStream> {
    // Parse file path
    let path: syn::LitStr = parse2(input)
        .map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    let path_str = path.value();

    Ok(quote! {
        {
            static SPIRV: &[u8] = include_bytes!(#path_str);
            unsafe {
                core::slice::from_raw_parts(
                    SPIRV.as_ptr() as *const u32,
                    SPIRV.len() / 4,
                )
            }
        }
    })
}

/// Interface kind.
#[derive(Debug, Clone, Copy)]
pub enum InterfaceKind {
    Input,
    Output,
}

/// Extract shader module from parsed syn module.
fn extract_shader_module(module: &ItemMod, attrs: ShaderModuleAttrs) -> Result<ShaderModule> {
    let items = module.content.as_ref()
        .map(|(_, items)| items.as_slice())
        .unwrap_or(&[]);

    let mut entry_points = Vec::new();
    let mut resources = Vec::new();
    let mut structs = Vec::new();
    let mut shared_vars = Vec::new();

    for item in items {
        match item {
            Item::Fn(func) => {
                if let Some(entry) = try_parse_entry_point(func)? {
                    entry_points.push(entry);
                }
            }
            Item::Struct(s) => {
                if let Some(resource) = try_parse_resource(s)? {
                    resources.push(resource);
                }
                if let Some(struct_def) = try_parse_struct(s)? {
                    structs.push(struct_def);
                }
            }
            Item::Static(s) => {
                if is_shared_memory(s) {
                    shared_vars.push(crate::parse::SharedVar {
                        name: s.ident.clone(),
                        ty: (*s.ty).clone(),
                        original: s.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(ShaderModule {
        name: module.ident.clone(),
        attrs,
        entry_points,
        resources,
        structs,
        shared_vars,
        original: module.clone(),
    })
}

fn try_parse_entry_point(func: &ItemFn) -> Result<Option<EntryPoint>> {
    for attr in &func.attrs {
        let stage = if attr.path().is_ident("vertex") {
            Some(ShaderStage::Vertex)
        } else if attr.path().is_ident("fragment") {
            Some(ShaderStage::Fragment)
        } else if attr.path().is_ident("compute") {
            Some(ShaderStage::Compute)
        } else if attr.path().is_ident("geometry") {
            Some(ShaderStage::Geometry)
        } else if attr.path().is_ident("tessellation_control") {
            Some(ShaderStage::TessellationControl)
        } else if attr.path().is_ident("tessellation_evaluation") {
            Some(ShaderStage::TessellationEvaluation)
        } else if attr.path().is_ident("mesh") {
            Some(ShaderStage::Mesh)
        } else if attr.path().is_ident("task") {
            Some(ShaderStage::Task)
        } else if attr.path().is_ident("ray_generation") {
            Some(ShaderStage::RayGeneration)
        } else if attr.path().is_ident("closest_hit") {
            Some(ShaderStage::ClosestHit)
        } else if attr.path().is_ident("any_hit") {
            Some(ShaderStage::AnyHit)
        } else if attr.path().is_ident("miss") {
            Some(ShaderStage::Miss)
        } else if attr.path().is_ident("intersection") {
            Some(ShaderStage::Intersection)
        } else if attr.path().is_ident("callable") {
            Some(ShaderStage::Callable)
        } else {
            None
        };

        if let Some(stage) = stage {
            let stage_attrs = if let Ok(tokens) = attr.meta.require_list() {
                ShaderAttrParser::parse_stage_attrs(stage, tokens.tokens.clone())?
            } else {
                StageAttrs::default()
            };

            let inputs = parse_function_inputs(func)?;
            let output = parse_return_type(func)?;

            return Ok(Some(EntryPoint {
                name: func.sig.ident.clone(),
                stage,
                stage_attrs,
                inputs,
                output,
                function: func.clone(),
            }));
        }
    }

    Ok(None)
}

fn try_parse_resource(s: &ItemStruct) -> Result<Option<Resource>> {
    for attr in &s.attrs {
        let kind = if attr.path().is_ident("uniform") {
            Some(ResourceKind::UniformBuffer)
        } else if attr.path().is_ident("storage") {
            Some(ResourceKind::StorageBuffer)
        } else if attr.path().is_ident("texture") {
            Some(ResourceKind::SampledImage)
        } else if attr.path().is_ident("sampler") {
            Some(ResourceKind::Sampler)
        } else if attr.path().is_ident("push_constant") {
            Some(ResourceKind::PushConstant)
        } else {
            None
        };

        if let Some(kind) = kind {
            let res_attrs = if let Ok(tokens) = attr.meta.require_list() {
                ShaderAttrParser::parse_resource_attrs(kind, tokens.tokens.clone())?
            } else {
                ResourceAttrs {
                    binding: 0,
                    set: 0,
                    access: None,
                    layout: None,
                    dim: None,
                    format: None,
                }
            };

            return Ok(Some(Resource {
                name: s.ident.clone(),
                kind,
                binding: res_attrs.binding,
                set: res_attrs.set,
                ty: syn::Type::Path(syn::TypePath {
                    qself: None,
                    path: syn::Path::from(s.ident.clone()),
                }),
                access: res_attrs.access,
                layout: res_attrs.layout,
                span: s.span(),
            }));
        }
    }

    Ok(None)
}

fn try_parse_struct(s: &ItemStruct) -> Result<Option<StructDef>> {
    let is_input = s.attrs.iter().any(|a| a.path().is_ident("input"));
    let is_output = s.attrs.iter().any(|a| a.path().is_ident("output"));

    if !is_input && !is_output {
        return Ok(None);
    }

    let fields = match &s.fields {
        syn::Fields::Named(named) => {
            named.named.iter().map(|f| {
                let mut location = None;
                let mut builtin = None;
                let mut flat = false;

                for attr in &f.attrs {
                    if attr.path().is_ident("location") {
                        if let Ok(lit) = attr.parse_args::<LitInt>() {
                            location = lit.base10_parse().ok();
                        }
                    } else if attr.path().is_ident("builtin") {
                        if let Ok(ident) = attr.parse_args::<Ident>() {
                            builtin = Some(ident.to_string());
                        }
                    } else if attr.path().is_ident("flat") {
                        flat = true;
                    }
                }

                StructField {
                    name: f.ident.clone().unwrap(),
                    ty: f.ty.clone(),
                    location,
                    builtin,
                    flat,
                }
            }).collect()
        }
        _ => Vec::new(),
    };

    Ok(Some(StructDef {
        name: s.ident.clone(),
        fields,
        is_input,
        is_output,
        original: s.clone(),
    }))
}

fn is_shared_memory(s: &ItemStatic) -> bool {
    s.attrs.iter().any(|a| a.path().is_ident("shared"))
}

fn validate_entry_point_signature(func: &ItemFn, stage: ShaderStage) -> Result<()> {
    // Check for async
    if func.sig.asyncness.is_some() {
        return Err(Error::unsupported(
            "shader functions cannot be async",
            func.sig.asyncness.span(),
        ));
    }

    // Check for generics
    if !func.sig.generics.params.is_empty() {
        return Err(Error::unsupported(
            "shader functions cannot have generic parameters",
            func.sig.generics.span(),
        ));
    }

    // Check for unsafe
    if func.sig.unsafety.is_some() {
        return Err(Error::unsupported(
            "shader functions cannot be unsafe",
            func.sig.unsafety.span(),
        ));
    }

    Ok(())
}

fn parse_vulkan_version(target: Option<&str>) -> VulkanVersion {
    match target {
        Some("vulkan1.0") => VulkanVersion::V1_0,
        Some("vulkan1.1") => VulkanVersion::V1_1,
        Some("vulkan1.2") => VulkanVersion::V1_2,
        Some("vulkan1.3") => VulkanVersion::V1_3,
        _ => VulkanVersion::V1_2, // Default
    }
}

/// Re-export shader stage for lib.rs.
pub use crate::parse::ShaderStage;

/// Re-export resource kind for lib.rs.
pub use crate::parse::ResourceKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulkan_version_parsing() {
        assert_eq!(parse_vulkan_version(Some("vulkan1.0")), VulkanVersion::V1_0);
        assert_eq!(parse_vulkan_version(Some("vulkan1.3")), VulkanVersion::V1_3);
        assert_eq!(parse_vulkan_version(None), VulkanVersion::V1_2);
    }
}
