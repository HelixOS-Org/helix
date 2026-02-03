//! Shader type system for LUMINA derive macros.
//!
//! This module handles Rust type to SPIR-V type conversion and layout calculations.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, FieldsNamed, GenericArgument, Ident, PathArguments, Type};

use crate::error::{Error, ErrorKind, Result};

/// Shader-compatible types.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderType {
    /// Void type.
    Void,
    /// Boolean (SPIR-V OpTypeBool).
    Bool,
    /// 32-bit signed integer.
    Int32,
    /// 32-bit unsigned integer.
    Uint32,
    /// 32-bit float.
    Float32,
    /// 64-bit float.
    Float64,
    /// Vector of scalars.
    Vector { element: Box<ShaderType>, size: u32 },
    /// Matrix of vectors.
    Matrix {
        element: Box<ShaderType>,
        cols: u32,
        rows: u32,
    },
    /// Fixed-size array.
    Array { element: Box<ShaderType>, size: u32 },
    /// Runtime-sized array (for storage buffers).
    RuntimeArray { element: Box<ShaderType> },
    /// Struct type.
    Struct {
        name: String,
        members: Vec<StructMember>,
    },
    /// Pointer type.
    Pointer {
        pointee: Box<ShaderType>,
        storage: StorageClass,
    },
    /// Sampler type.
    Sampler,
    /// 2D Texture.
    Texture2D { element: Box<ShaderType> },
    /// 3D Texture.
    Texture3D { element: Box<ShaderType> },
    /// Cube Texture.
    TextureCube { element: Box<ShaderType> },
    /// 2D Texture Array.
    Texture2DArray { element: Box<ShaderType> },
    /// Combined image sampler.
    SampledImage { image: Box<ShaderType> },
    /// Storage image.
    StorageImage { format: ImageFormat },
    /// Acceleration structure.
    AccelerationStructure,
    /// Ray query.
    RayQuery,
}

/// Struct member.
#[derive(Debug, Clone, PartialEq)]
pub struct StructMember {
    pub name: String,
    pub ty: ShaderType,
    pub offset: Option<u32>,
}

/// SPIR-V storage classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageClass {
    UniformConstant,
    Input,
    Uniform,
    Output,
    Workgroup,
    CrossWorkgroup,
    Private,
    Function,
    Generic,
    PushConstant,
    AtomicCounter,
    Image,
    StorageBuffer,
    PhysicalStorageBuffer,
    RayPayloadKHR,
    HitAttributeKHR,
    IncomingRayPayloadKHR,
    ShaderRecordBufferKHR,
    CallableDataKHR,
    IncomingCallableDataKHR,
    TaskPayloadWorkgroupEXT,
}

/// Image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Unknown,
    Rgba32f,
    Rgba16f,
    Rgba8,
    Rgba8Snorm,
    Rgba32i,
    Rgba16i,
    Rgba8i,
    Rgba32ui,
    Rgba16ui,
    Rgba8ui,
    R32f,
    R16f,
    R32i,
    R32ui,
    Rg32f,
    Rg16f,
    Rg32i,
    Rg32ui,
    R11fG11fB10f,
}

impl ShaderType {
    /// Get the size in bytes (for layout calculation).
    pub fn size(&self) -> u32 {
        match self {
            ShaderType::Void => 0,
            ShaderType::Bool => 4, // Typically 4 bytes in GPU
            ShaderType::Int32 => 4,
            ShaderType::Uint32 => 4,
            ShaderType::Float32 => 4,
            ShaderType::Float64 => 8,
            ShaderType::Vector { element, size } => element.size() * size,
            ShaderType::Matrix {
                element,
                cols,
                rows,
            } => {
                // Each column is a vector, aligned properly
                element.size() * rows * cols
            },
            ShaderType::Array { element, size } => {
                let stride = element.array_stride();
                stride * size
            },
            ShaderType::RuntimeArray { .. } => 0, // Unknown at compile time
            ShaderType::Struct { members, .. } => {
                if let Some(last) = members.last() {
                    if let Some(offset) = last.offset {
                        offset + last.ty.size()
                    } else {
                        members.iter().map(|m| m.ty.size()).sum()
                    }
                } else {
                    0
                }
            },
            ShaderType::Pointer { .. } => 8,   // 64-bit address
            ShaderType::Sampler => 0,          // Opaque
            ShaderType::Texture2D { .. } => 0, // Opaque
            ShaderType::Texture3D { .. } => 0,
            ShaderType::TextureCube { .. } => 0,
            ShaderType::Texture2DArray { .. } => 0,
            ShaderType::SampledImage { .. } => 0,
            ShaderType::StorageImage { .. } => 0,
            ShaderType::AccelerationStructure => 0,
            ShaderType::RayQuery => 0,
        }
    }

    /// Get the alignment in bytes.
    pub fn alignment(&self) -> u32 {
        match self {
            ShaderType::Void => 1,
            ShaderType::Bool => 4,
            ShaderType::Int32 => 4,
            ShaderType::Uint32 => 4,
            ShaderType::Float32 => 4,
            ShaderType::Float64 => 8,
            ShaderType::Vector { element, size } => {
                let base = element.alignment();
                match size {
                    2 => base * 2,
                    3 | 4 => base * 4,
                    _ => base,
                }
            },
            ShaderType::Matrix { element, rows, .. } => {
                // Column vector alignment
                let vec_ty = ShaderType::Vector {
                    element: element.clone(),
                    size: *rows,
                };
                vec_ty.alignment()
            },
            ShaderType::Array { element, .. } => element.alignment().max(16), // std140 requires 16-byte alignment
            ShaderType::RuntimeArray { element } => element.alignment().max(16),
            ShaderType::Struct { members, .. } => {
                members.iter().map(|m| m.ty.alignment()).max().unwrap_or(1)
            },
            _ => 1,
        }
    }

    /// Get the array stride (for arrays).
    pub fn array_stride(&self) -> u32 {
        let size = self.size();
        let align = self.alignment();
        (size + align - 1) / align * align
    }

    /// Check if this is a scalar type.
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            ShaderType::Bool
                | ShaderType::Int32
                | ShaderType::Uint32
                | ShaderType::Float32
                | ShaderType::Float64
        )
    }

    /// Check if this is a vector type.
    pub fn is_vector(&self) -> bool {
        matches!(self, ShaderType::Vector { .. })
    }

    /// Check if this is a matrix type.
    pub fn is_matrix(&self) -> bool {
        matches!(self, ShaderType::Matrix { .. })
    }

    /// Check if this is an opaque type.
    pub fn is_opaque(&self) -> bool {
        matches!(
            self,
            ShaderType::Sampler
                | ShaderType::Texture2D { .. }
                | ShaderType::Texture3D { .. }
                | ShaderType::TextureCube { .. }
                | ShaderType::Texture2DArray { .. }
                | ShaderType::SampledImage { .. }
                | ShaderType::StorageImage { .. }
                | ShaderType::AccelerationStructure
                | ShaderType::RayQuery
        )
    }

    /// Get vector size if this is a vector.
    pub fn vector_size(&self) -> Option<u32> {
        match self {
            ShaderType::Vector { size, .. } => Some(*size),
            _ => None,
        }
    }

    /// Get element type if this is a vector/matrix/array.
    pub fn element_type(&self) -> Option<&ShaderType> {
        match self {
            ShaderType::Vector { element, .. } => Some(element),
            ShaderType::Matrix { element, .. } => Some(element),
            ShaderType::Array { element, .. } => Some(element),
            ShaderType::RuntimeArray { element } => Some(element),
            _ => None,
        }
    }
}

/// Type converter from Rust types to shader types.
pub struct TypeConverter;

impl TypeConverter {
    /// Convert a Rust type to a shader type.
    pub fn convert(ty: &Type) -> Result<ShaderType> {
        match ty {
            Type::Path(type_path) => {
                let path = &type_path.path;

                // Handle built-in types
                if let Some(ident) = path.get_ident() {
                    return Self::convert_simple_type(ident);
                }

                // Handle generic types like Vec2<f32>, Array<T, N>
                if let Some(segment) = path.segments.last() {
                    let name = segment.ident.to_string();

                    match name.as_str() {
                        "Vec2" | "Vec3" | "Vec4" | "IVec2" | "IVec3" | "IVec4" | "UVec2"
                        | "UVec3" | "UVec4" | "DVec2" | "DVec3" | "DVec4" | "BVec2" | "BVec3"
                        | "BVec4" => {
                            return Self::convert_vector_type(&name, &segment.arguments);
                        },
                        "Mat2" | "Mat3" | "Mat4" | "Mat2x2" | "Mat2x3" | "Mat2x4" | "Mat3x2"
                        | "Mat3x3" | "Mat3x4" | "Mat4x2" | "Mat4x3" | "Mat4x4" | "DMat2"
                        | "DMat3" | "DMat4" => {
                            return Self::convert_matrix_type(&name);
                        },
                        "Texture2D" | "Texture3D" | "TextureCube" | "Texture2DArray" => {
                            return Self::convert_texture_type(&name, &segment.arguments);
                        },
                        "Sampler" => {
                            return Ok(ShaderType::Sampler);
                        },
                        "AccelerationStructure" => {
                            return Ok(ShaderType::AccelerationStructure);
                        },
                        "RayQuery" => {
                            return Ok(ShaderType::RayQuery);
                        },
                        _ => {},
                    }
                }

                // Assume it's a user-defined struct
                let name = path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                Ok(ShaderType::Struct {
                    name,
                    members: Vec::new(),
                })
            },
            Type::Array(array) => {
                let element = Self::convert(&array.elem)?;

                // Try to get array size
                if let syn::Expr::Lit(expr_lit) = &array.len {
                    if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                        let size: u32 = lit_int.base10_parse().map_err(|e| {
                            Error::type_error(format!("invalid array size: {}", e), array.span())
                        })?;
                        return Ok(ShaderType::Array {
                            element: Box::new(element),
                            size,
                        });
                    }
                }

                Err(Error::type_error(
                    "array size must be a literal",
                    array.len.span(),
                ))
            },
            Type::Slice(slice) => {
                // Runtime array
                let element = Self::convert(&slice.elem)?;
                Ok(ShaderType::RuntimeArray {
                    element: Box::new(element),
                })
            },
            Type::Reference(reference) => {
                // References become pointers
                let pointee = Self::convert(&reference.elem)?;
                Ok(ShaderType::Pointer {
                    pointee: Box::new(pointee),
                    storage: StorageClass::Function,
                })
            },
            Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(ShaderType::Void),
            _ => Err(Error::type_error(
                format!("unsupported type: {}", quote!(#ty)),
                ty.span(),
            )),
        }
    }

    fn convert_simple_type(ident: &Ident) -> Result<ShaderType> {
        let name = ident.to_string();
        match name.as_str() {
            "bool" => Ok(ShaderType::Bool),
            "i32" => Ok(ShaderType::Int32),
            "u32" => Ok(ShaderType::Uint32),
            "f32" => Ok(ShaderType::Float32),
            "f64" => Ok(ShaderType::Float64),
            // Shorthand types
            "Vec2" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 2,
            }),
            "Vec3" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 3,
            }),
            "Vec4" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 4,
            }),
            "IVec2" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Int32),
                size: 2,
            }),
            "IVec3" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Int32),
                size: 3,
            }),
            "IVec4" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Int32),
                size: 4,
            }),
            "UVec2" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Uint32),
                size: 2,
            }),
            "UVec3" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Uint32),
                size: 3,
            }),
            "UVec4" => Ok(ShaderType::Vector {
                element: Box::new(ShaderType::Uint32),
                size: 4,
            }),
            "Mat2" | "Mat2x2" => Ok(ShaderType::Matrix {
                element: Box::new(ShaderType::Float32),
                cols: 2,
                rows: 2,
            }),
            "Mat3" | "Mat3x3" => Ok(ShaderType::Matrix {
                element: Box::new(ShaderType::Float32),
                cols: 3,
                rows: 3,
            }),
            "Mat4" | "Mat4x4" => Ok(ShaderType::Matrix {
                element: Box::new(ShaderType::Float32),
                cols: 4,
                rows: 4,
            }),
            _ => {
                // Assume user-defined struct
                Ok(ShaderType::Struct {
                    name,
                    members: Vec::new(),
                })
            },
        }
    }

    fn convert_vector_type(name: &str, args: &PathArguments) -> Result<ShaderType> {
        let (base_element, size) = match name {
            "Vec2" => (ShaderType::Float32, 2),
            "Vec3" => (ShaderType::Float32, 3),
            "Vec4" => (ShaderType::Float32, 4),
            "IVec2" => (ShaderType::Int32, 2),
            "IVec3" => (ShaderType::Int32, 3),
            "IVec4" => (ShaderType::Int32, 4),
            "UVec2" => (ShaderType::Uint32, 2),
            "UVec3" => (ShaderType::Uint32, 3),
            "UVec4" => (ShaderType::Uint32, 4),
            "DVec2" => (ShaderType::Float64, 2),
            "DVec3" => (ShaderType::Float64, 3),
            "DVec4" => (ShaderType::Float64, 4),
            "BVec2" => (ShaderType::Bool, 2),
            "BVec3" => (ShaderType::Bool, 3),
            "BVec4" => (ShaderType::Bool, 4),
            _ => unreachable!(),
        };

        // Check for generic arguments override
        let element = if let PathArguments::AngleBracketed(args) = args {
            if let Some(GenericArgument::Type(ty)) = args.args.first() {
                Self::convert(ty)?
            } else {
                base_element
            }
        } else {
            base_element
        };

        Ok(ShaderType::Vector {
            element: Box::new(element),
            size,
        })
    }

    fn convert_matrix_type(name: &str) -> Result<ShaderType> {
        let (element, cols, rows) = match name {
            "Mat2" | "Mat2x2" => (ShaderType::Float32, 2, 2),
            "Mat2x3" => (ShaderType::Float32, 2, 3),
            "Mat2x4" => (ShaderType::Float32, 2, 4),
            "Mat3x2" => (ShaderType::Float32, 3, 2),
            "Mat3" | "Mat3x3" => (ShaderType::Float32, 3, 3),
            "Mat3x4" => (ShaderType::Float32, 3, 4),
            "Mat4x2" => (ShaderType::Float32, 4, 2),
            "Mat4x3" => (ShaderType::Float32, 4, 3),
            "Mat4" | "Mat4x4" => (ShaderType::Float32, 4, 4),
            "DMat2" => (ShaderType::Float64, 2, 2),
            "DMat3" => (ShaderType::Float64, 3, 3),
            "DMat4" => (ShaderType::Float64, 4, 4),
            _ => unreachable!(),
        };

        Ok(ShaderType::Matrix {
            element: Box::new(element),
            cols,
            rows,
        })
    }

    fn convert_texture_type(name: &str, args: &PathArguments) -> Result<ShaderType> {
        let element = if let PathArguments::AngleBracketed(args) = args {
            if let Some(GenericArgument::Type(ty)) = args.args.first() {
                Box::new(Self::convert(ty)?)
            } else {
                Box::new(ShaderType::Vector {
                    element: Box::new(ShaderType::Float32),
                    size: 4,
                })
            }
        } else {
            Box::new(ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 4,
            })
        };

        match name {
            "Texture2D" => Ok(ShaderType::Texture2D { element }),
            "Texture3D" => Ok(ShaderType::Texture3D { element }),
            "TextureCube" => Ok(ShaderType::TextureCube { element }),
            "Texture2DArray" => Ok(ShaderType::Texture2DArray { element }),
            _ => unreachable!(),
        }
    }
}

/// Layout calculator for struct types.
pub struct LayoutCalculator;

impl LayoutCalculator {
    /// Calculate std140 layout for a struct.
    pub fn std140_layout(members: &[StructMember]) -> Vec<StructMember> {
        let mut result = Vec::new();
        let mut offset = 0u32;

        for member in members {
            let alignment = Self::std140_alignment(&member.ty);
            offset = Self::align_to(offset, alignment);

            result.push(StructMember {
                name: member.name.clone(),
                ty: member.ty.clone(),
                offset: Some(offset),
            });

            offset += member.ty.size();
        }

        result
    }

    /// Calculate std430 layout for a struct.
    pub fn std430_layout(members: &[StructMember]) -> Vec<StructMember> {
        let mut result = Vec::new();
        let mut offset = 0u32;

        for member in members {
            let alignment = Self::std430_alignment(&member.ty);
            offset = Self::align_to(offset, alignment);

            result.push(StructMember {
                name: member.name.clone(),
                ty: member.ty.clone(),
                offset: Some(offset),
            });

            offset += member.ty.size();
        }

        result
    }

    /// Calculate scalar layout for a struct.
    pub fn scalar_layout(members: &[StructMember]) -> Vec<StructMember> {
        let mut result = Vec::new();
        let mut offset = 0u32;

        for member in members {
            let alignment = Self::scalar_alignment(&member.ty);
            offset = Self::align_to(offset, alignment);

            result.push(StructMember {
                name: member.name.clone(),
                ty: member.ty.clone(),
                offset: Some(offset),
            });

            offset += member.ty.size();
        }

        result
    }

    fn std140_alignment(ty: &ShaderType) -> u32 {
        match ty {
            ShaderType::Bool | ShaderType::Int32 | ShaderType::Uint32 | ShaderType::Float32 => 4,
            ShaderType::Float64 => 8,
            ShaderType::Vector { element, size } => {
                let base = Self::std140_alignment(element);
                match size {
                    2 => base * 2,
                    3 | 4 => base * 4,
                    _ => base,
                }
            },
            ShaderType::Matrix { element, rows, .. } => {
                // Column alignment (treated as array of vectors)
                let vec_align = Self::std140_alignment(&ShaderType::Vector {
                    element: element.clone(),
                    size: *rows,
                });
                vec_align.max(16) // Round up to 16 for std140
            },
            ShaderType::Array { element, .. } => Self::std140_alignment(element).max(16),
            ShaderType::Struct { members, .. } => {
                let max_align = members
                    .iter()
                    .map(|m| Self::std140_alignment(&m.ty))
                    .max()
                    .unwrap_or(4);
                max_align.max(16) // Struct alignment rounded up to 16
            },
            _ => 4,
        }
    }

    fn std430_alignment(ty: &ShaderType) -> u32 {
        match ty {
            ShaderType::Bool | ShaderType::Int32 | ShaderType::Uint32 | ShaderType::Float32 => 4,
            ShaderType::Float64 => 8,
            ShaderType::Vector { element, size } => {
                let base = Self::std430_alignment(element);
                match size {
                    2 => base * 2,
                    3 | 4 => base * 4,
                    _ => base,
                }
            },
            ShaderType::Matrix { element, rows, .. } => {
                Self::std430_alignment(&ShaderType::Vector {
                    element: element.clone(),
                    size: *rows,
                })
            },
            ShaderType::Array { element, .. } => {
                Self::std430_alignment(element) // No rounding for std430
            },
            ShaderType::Struct { members, .. } => members
                .iter()
                .map(|m| Self::std430_alignment(&m.ty))
                .max()
                .unwrap_or(4),
            _ => 4,
        }
    }

    fn scalar_alignment(ty: &ShaderType) -> u32 {
        match ty {
            ShaderType::Bool | ShaderType::Int32 | ShaderType::Uint32 | ShaderType::Float32 => 4,
            ShaderType::Float64 => 8,
            ShaderType::Vector { element, .. } => Self::scalar_alignment(element),
            ShaderType::Matrix { element, .. } => Self::scalar_alignment(element),
            ShaderType::Array { element, .. } => Self::scalar_alignment(element),
            ShaderType::Struct { members, .. } => members
                .iter()
                .map(|m| Self::scalar_alignment(&m.ty))
                .max()
                .unwrap_or(4),
            _ => 4,
        }
    }

    fn align_to(offset: u32, alignment: u32) -> u32 {
        (offset + alignment - 1) / alignment * alignment
    }
}

/// Derive ShaderType implementation.
pub fn derive_shader_type_impl(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput =
        syn::parse2(input).map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Ensure it's a struct
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::unsupported(
                    "ShaderType can only be derived for structs with named fields",
                    input.ident.span(),
                ));
            },
        },
        _ => {
            return Err(Error::unsupported(
                "ShaderType can only be derived for structs",
                input.ident.span(),
            ));
        },
    };

    let field_names: Vec<_> = fields.iter().filter_map(|f| f.ident.as_ref()).collect();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let output = quote! {
        impl #impl_generics lumina_core::ShaderType for #name #ty_generics #where_clause {
            const SIZE: usize = {
                let mut size = 0usize;
                #(
                    // Add alignment padding
                    let align = <#field_types as lumina_core::ShaderType>::ALIGNMENT;
                    size = (size + align - 1) / align * align;
                    // Add field size
                    size += <#field_types as lumina_core::ShaderType>::SIZE;
                )*
                // Final alignment
                let struct_align = Self::ALIGNMENT;
                (size + struct_align - 1) / struct_align * struct_align
            };

            const ALIGNMENT: usize = {
                let mut max_align = 1usize;
                #(
                    let align = <#field_types as lumina_core::ShaderType>::ALIGNMENT;
                    if align > max_align { max_align = align; }
                )*
                max_align
            };

            fn write_bytes(&self, buffer: &mut [u8]) {
                let mut offset = 0usize;
                #(
                    // Align
                    let align = <#field_types as lumina_core::ShaderType>::ALIGNMENT;
                    offset = (offset + align - 1) / align * align;
                    // Write
                    let size = <#field_types as lumina_core::ShaderType>::SIZE;
                    self.#field_names.write_bytes(&mut buffer[offset..offset + size]);
                    offset += size;
                )*
            }
        }
    };

    Ok(output)
}

/// Derive VertexInput implementation.
pub fn derive_vertex_input_impl(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput =
        syn::parse2(input).map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::unsupported(
                    "VertexInput can only be derived for structs with named fields",
                    input.ident.span(),
                ));
            },
        },
        _ => {
            return Err(Error::unsupported(
                "VertexInput can only be derived for structs",
                input.ident.span(),
            ));
        },
    };

    let mut attribute_descs = Vec::new();
    let mut offset = 0u32;

    for (idx, field) in fields.iter().enumerate() {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;

        // Find location attribute
        let location = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident("location"))
            .map(|a| {
                let lit: syn::LitInt = a
                    .parse_args()
                    .map_err(|e| Error::syntax(e.to_string(), a.span()))?;
                lit.base10_parse::<u32>()
                    .map_err(|e| Error::syntax(e.to_string(), lit.span()))
            })
            .transpose()?
            .unwrap_or(idx as u32);

        attribute_descs.push(quote! {
            lumina_core::VertexAttributeDescriptor {
                location: #location,
                offset: #offset,
                format: <#field_ty as lumina_core::VertexAttribute>::FORMAT,
            }
        });

        // Increase offset (simplified, should use actual size)
        offset += 16; // Placeholder, should be field size
    }

    let output = quote! {
        impl #impl_generics lumina_core::VertexInput for #name #ty_generics #where_clause {
            const STRIDE: u32 = core::mem::size_of::<Self>() as u32;

            fn attributes() -> &'static [lumina_core::VertexAttributeDescriptor] {
                static ATTRIBUTES: &[lumina_core::VertexAttributeDescriptor] = &[
                    #(#attribute_descs),*
                ];
                ATTRIBUTES
            }
        }
    };

    Ok(output)
}

/// Derive PushConstant implementation.
pub fn derive_push_constant_impl(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput =
        syn::parse2(input).map_err(|e| Error::syntax(e.to_string(), Span::call_site()))?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let output = quote! {
        impl #impl_generics lumina_core::PushConstant for #name #ty_generics #where_clause {
            const SIZE: u32 = core::mem::size_of::<Self>() as u32;

            fn as_bytes(&self) -> &[u8] {
                unsafe {
                    core::slice::from_raw_parts(
                        self as *const Self as *const u8,
                        core::mem::size_of::<Self>(),
                    )
                }
            }
        }

        // Compile-time size check
        const _: () = {
            if core::mem::size_of::<#name>() > 128 {
                panic!("Push constant size exceeds typical limit of 128 bytes");
            }
        };
    };

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_type_size() {
        assert_eq!(ShaderType::Float32.size(), 4);
        assert_eq!(
            ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 3,
            }
            .size(),
            12
        );
        assert_eq!(
            ShaderType::Matrix {
                element: Box::new(ShaderType::Float32),
                cols: 4,
                rows: 4,
            }
            .size(),
            64
        );
    }

    #[test]
    fn test_shader_type_alignment() {
        assert_eq!(ShaderType::Float32.alignment(), 4);
        assert_eq!(
            ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 2,
            }
            .alignment(),
            8
        );
        assert_eq!(
            ShaderType::Vector {
                element: Box::new(ShaderType::Float32),
                size: 4,
            }
            .alignment(),
            16
        );
    }

    #[test]
    fn test_type_classification() {
        assert!(ShaderType::Float32.is_scalar());
        assert!(ShaderType::Vector {
            element: Box::new(ShaderType::Float32),
            size: 3,
        }
        .is_vector());
        assert!(ShaderType::Sampler.is_opaque());
    }
}
