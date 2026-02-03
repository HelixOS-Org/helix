//! SPIR-V Shader Reflection
//!
//! Extract resource binding information from SPIR-V modules.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec, collections::BTreeMap};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use crate::{
    instruction::*,
    module::{SpirVModule, VariableDecl},
    opcode::Opcode,
    types::{BuiltIn, SpirVType},
    SpirVError, SpirVResult,
};

/// Shader reflection data
#[derive(Debug, Clone, Default)]
pub struct ShaderReflection {
    /// Stage inputs
    pub inputs: Vec<ShaderInput>,
    /// Stage outputs
    pub outputs: Vec<ShaderOutput>,
    /// Uniform buffers
    pub uniform_buffers: Vec<UniformBuffer>,
    /// Storage buffers
    pub storage_buffers: Vec<StorageBuffer>,
    /// Push constants
    pub push_constants: Option<PushConstantBlock>,
    /// Sampled images (combined image samplers)
    pub sampled_images: Vec<SampledImage>,
    /// Storage images
    pub storage_images: Vec<StorageImage>,
    /// Separate samplers
    pub samplers: Vec<SeparateSampler>,
    /// Separate images
    pub separate_images: Vec<SeparateImage>,
    /// Input attachments
    pub input_attachments: Vec<InputAttachment>,
    /// Acceleration structures
    pub acceleration_structures: Vec<AccelerationStructureBinding>,
    /// Specialization constants
    pub spec_constants: Vec<SpecializationConstant>,
    /// Entry point info
    pub entry_point: Option<EntryPointReflection>,
}

impl ShaderReflection {
    /// Reflect a SPIR-V module
    pub fn reflect(module: &SpirVModule) -> SpirVResult<Self> {
        let mut reflection = Self::default();

        // Reflect entry point
        if let Some(ep) = module.entry_points.first() {
            reflection.entry_point = Some(EntryPointReflection {
                name: ep.name.clone(),
                execution_model: ep.execution_model,
                local_size: Self::get_local_size(module, ep.function),
            });
        }

        // Reflect all global variables
        for (&id, var) in &module.global_variables {
            reflection.reflect_variable(module, id, var)?;
        }

        // Reflect specialization constants
        for (&id, constant) in &module.constants {
            if matches!(
                constant.opcode,
                Opcode::OpSpecConstant
                    | Opcode::OpSpecConstantTrue
                    | Opcode::OpSpecConstantFalse
            ) {
                if let Some(spec_id) = module.get_decoration(id, Decoration::SpecId) {
                    reflection.spec_constants.push(SpecializationConstant {
                        id,
                        spec_id,
                        name: module.get_name(id).map(|s| s.to_string()),
                        default_value: Self::get_constant_value(constant),
                    });
                }
            }
        }

        // Sort by location/binding
        reflection.inputs.sort_by_key(|i| i.location);
        reflection.outputs.sort_by_key(|o| o.location);
        reflection.uniform_buffers.sort_by_key(|u| (u.set, u.binding));
        reflection.storage_buffers.sort_by_key(|s| (s.set, s.binding));
        reflection.sampled_images.sort_by_key(|s| (s.set, s.binding));
        reflection.storage_images.sort_by_key(|s| (s.set, s.binding));

        Ok(reflection)
    }

    /// Reflect a single variable
    fn reflect_variable(
        &mut self,
        module: &SpirVModule,
        id: Id,
        var: &VariableDecl,
    ) -> SpirVResult<()> {
        let name = module.get_name(id).map(|s| s.to_string());

        match var.storage_class {
            StorageClass::Input => {
                if let Some(location) = module.get_location(id) {
                    self.inputs.push(ShaderInput {
                        id,
                        location,
                        name,
                        ty: Self::get_type_info(module, var.result_type),
                        builtin: module.get_builtin(id),
                    });
                } else if let Some(builtin) = module.get_builtin(id) {
                    self.inputs.push(ShaderInput {
                        id,
                        location: 0,
                        name,
                        ty: Self::get_type_info(module, var.result_type),
                        builtin: Some(builtin),
                    });
                }
            }
            StorageClass::Output => {
                if let Some(location) = module.get_location(id) {
                    self.outputs.push(ShaderOutput {
                        id,
                        location,
                        name,
                        ty: Self::get_type_info(module, var.result_type),
                        builtin: module.get_builtin(id),
                    });
                } else if let Some(builtin) = module.get_builtin(id) {
                    self.outputs.push(ShaderOutput {
                        id,
                        location: 0,
                        name,
                        ty: Self::get_type_info(module, var.result_type),
                        builtin: Some(builtin),
                    });
                }
            }
            StorageClass::Uniform => {
                let set = module.get_descriptor_set(id).unwrap_or(0);
                let binding = module.get_binding(id).unwrap_or(0);
                let size = Self::get_buffer_size(module, var.result_type);

                self.uniform_buffers.push(UniformBuffer {
                    id,
                    set,
                    binding,
                    name,
                    size,
                    members: Self::get_struct_members(module, var.result_type),
                });
            }
            StorageClass::StorageBuffer => {
                let set = module.get_descriptor_set(id).unwrap_or(0);
                let binding = module.get_binding(id).unwrap_or(0);
                let size = Self::get_buffer_size(module, var.result_type);
                let non_writable = module
                    .decorations
                    .get(&id)
                    .map(|decs| decs.iter().any(|d| d.decoration == Decoration::NonWritable))
                    .unwrap_or(false);

                self.storage_buffers.push(StorageBuffer {
                    id,
                    set,
                    binding,
                    name,
                    size,
                    read_only: non_writable,
                    members: Self::get_struct_members(module, var.result_type),
                });
            }
            StorageClass::PushConstant => {
                let size = Self::get_buffer_size(module, var.result_type);
                self.push_constants = Some(PushConstantBlock {
                    id,
                    name,
                    size,
                    members: Self::get_struct_members(module, var.result_type),
                });
            }
            StorageClass::UniformConstant => {
                // Could be sampler, image, or combined image sampler
                let set = module.get_descriptor_set(id).unwrap_or(0);
                let binding = module.get_binding(id).unwrap_or(0);

                // Check type to determine what kind of resource
                if let Some(type_decl) = module.types.get(&var.result_type) {
                    match type_decl.opcode {
                        Opcode::OpTypeSampledImage => {
                            self.sampled_images.push(SampledImage {
                                id,
                                set,
                                binding,
                                name,
                                dim: Dim::Dim2D,
                                arrayed: false,
                                multisampled: false,
                                array_size: 1,
                            });
                        }
                        Opcode::OpTypeImage => {
                            // Check if storage or sampled
                            self.storage_images.push(StorageImage {
                                id,
                                set,
                                binding,
                                name,
                                dim: Dim::Dim2D,
                                format: ImageFormat::Unknown,
                                arrayed: false,
                                array_size: 1,
                            });
                        }
                        Opcode::OpTypeSampler => {
                            self.samplers.push(SeparateSampler {
                                id,
                                set,
                                binding,
                                name,
                            });
                        }
                        Opcode::OpTypeAccelerationStructureKHR => {
                            self.acceleration_structures.push(AccelerationStructureBinding {
                                id,
                                set,
                                binding,
                                name,
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Get local size for compute shaders
    fn get_local_size(module: &SpirVModule, func_id: Id) -> Option<[u32; 3]> {
        module
            .execution_modes
            .get(&func_id)
            .and_then(|modes| {
                modes.iter().find_map(|m| {
                    if matches!(m.mode, ExecutionMode::LocalSize) && m.operands.len() >= 3 {
                        Some([m.operands[0], m.operands[1], m.operands[2]])
                    } else {
                        None
                    }
                })
            })
    }

    /// Get type information
    fn get_type_info(module: &SpirVModule, type_id: Id) -> TypeInfo {
        // Dereference pointer type
        let inner_type = if let Some(type_decl) = module.types.get(&type_id) {
            if type_decl.opcode == Opcode::OpTypePointer {
                if let Some(Operand::Id(pointee)) = type_decl.operands.get(1) {
                    *pointee
                } else {
                    type_id
                }
            } else {
                type_id
            }
        } else {
            type_id
        };

        if let Some(type_decl) = module.types.get(&inner_type) {
            match type_decl.opcode {
                Opcode::OpTypeFloat => {
                    let width = type_decl
                        .operands
                        .first()
                        .and_then(|op| {
                            if let Operand::Literal(w) = op {
                                Some(*w)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(32);
                    TypeInfo::Scalar(ScalarInfo::Float(width))
                }
                Opcode::OpTypeInt => {
                    let width = type_decl
                        .operands
                        .first()
                        .and_then(|op| {
                            if let Operand::Literal(w) = op {
                                Some(*w)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(32);
                    let signed = type_decl
                        .operands
                        .get(1)
                        .and_then(|op| {
                            if let Operand::Literal(s) = op {
                                Some(*s != 0)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(true);
                    TypeInfo::Scalar(ScalarInfo::Int { width, signed })
                }
                Opcode::OpTypeBool => TypeInfo::Scalar(ScalarInfo::Bool),
                Opcode::OpTypeVector => {
                    let count = type_decl
                        .operands
                        .get(1)
                        .and_then(|op| {
                            if let Operand::Literal(c) = op {
                                Some(*c)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(4);
                    let component = type_decl
                        .operands
                        .first()
                        .and_then(|op| {
                            if let Operand::Id(id) = op {
                                Some(*id)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                    let scalar = if let TypeInfo::Scalar(s) = Self::get_type_info(module, component)
                    {
                        s
                    } else {
                        ScalarInfo::Float(32)
                    };
                    TypeInfo::Vector {
                        scalar,
                        count: count as u8,
                    }
                }
                Opcode::OpTypeMatrix => {
                    let columns = type_decl
                        .operands
                        .get(1)
                        .and_then(|op| {
                            if let Operand::Literal(c) = op {
                                Some(*c)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(4);
                    let column_type = type_decl
                        .operands
                        .first()
                        .and_then(|op| {
                            if let Operand::Id(id) = op {
                                Some(*id)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                    let rows = if let Some(col_decl) = module.types.get(&column_type) {
                        col_decl
                            .operands
                            .get(1)
                            .and_then(|op| {
                                if let Operand::Literal(r) = op {
                                    Some(*r)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(4)
                    } else {
                        4
                    };
                    TypeInfo::Matrix {
                        columns: columns as u8,
                        rows: rows as u8,
                    }
                }
                Opcode::OpTypeArray => TypeInfo::Array {
                    element: Box::new(TypeInfo::Unknown),
                    size: None,
                },
                Opcode::OpTypeStruct => TypeInfo::Struct,
                _ => TypeInfo::Unknown,
            }
        } else {
            TypeInfo::Unknown
        }
    }

    /// Get buffer size
    fn get_buffer_size(_module: &SpirVModule, _type_id: Id) -> usize {
        // Would need to calculate from type info
        0
    }

    /// Get struct members
    fn get_struct_members(_module: &SpirVModule, _type_id: Id) -> Vec<StructMember> {
        // Would need to extract member info from decorations
        Vec::new()
    }

    /// Get constant value
    fn get_constant_value(constant: &crate::module::ConstantDecl) -> Option<ConstantValue> {
        match constant.opcode {
            Opcode::OpConstantTrue | Opcode::OpSpecConstantTrue => {
                Some(ConstantValue::Bool(true))
            }
            Opcode::OpConstantFalse | Opcode::OpSpecConstantFalse => {
                Some(ConstantValue::Bool(false))
            }
            Opcode::OpConstant | Opcode::OpSpecConstant => constant
                .operands
                .first()
                .and_then(|op| {
                    if let Operand::Literal(v) = op {
                        Some(ConstantValue::Uint(*v))
                    } else {
                        None
                    }
                }),
            _ => None,
        }
    }

    /// Get total descriptor count
    pub fn descriptor_count(&self) -> usize {
        self.uniform_buffers.len()
            + self.storage_buffers.len()
            + self.sampled_images.len()
            + self.storage_images.len()
            + self.samplers.len()
            + self.separate_images.len()
            + self.input_attachments.len()
            + self.acceleration_structures.len()
    }

    /// Get all descriptor bindings
    pub fn get_bindings(&self) -> Vec<DescriptorBinding> {
        let mut bindings = Vec::new();

        for ub in &self.uniform_buffers {
            bindings.push(DescriptorBinding {
                set: ub.set,
                binding: ub.binding,
                descriptor_type: DescriptorType::UniformBuffer,
                count: 1,
                name: ub.name.clone(),
            });
        }

        for sb in &self.storage_buffers {
            bindings.push(DescriptorBinding {
                set: sb.set,
                binding: sb.binding,
                descriptor_type: DescriptorType::StorageBuffer,
                count: 1,
                name: sb.name.clone(),
            });
        }

        for si in &self.sampled_images {
            bindings.push(DescriptorBinding {
                set: si.set,
                binding: si.binding,
                descriptor_type: DescriptorType::CombinedImageSampler,
                count: si.array_size,
                name: si.name.clone(),
            });
        }

        for si in &self.storage_images {
            bindings.push(DescriptorBinding {
                set: si.set,
                binding: si.binding,
                descriptor_type: DescriptorType::StorageImage,
                count: si.array_size,
                name: si.name.clone(),
            });
        }

        for s in &self.samplers {
            bindings.push(DescriptorBinding {
                set: s.set,
                binding: s.binding,
                descriptor_type: DescriptorType::Sampler,
                count: 1,
                name: s.name.clone(),
            });
        }

        for ia in &self.input_attachments {
            bindings.push(DescriptorBinding {
                set: ia.set,
                binding: ia.binding,
                descriptor_type: DescriptorType::InputAttachment,
                count: 1,
                name: ia.name.clone(),
            });
        }

        for acc in &self.acceleration_structures {
            bindings.push(DescriptorBinding {
                set: acc.set,
                binding: acc.binding,
                descriptor_type: DescriptorType::AccelerationStructure,
                count: 1,
                name: acc.name.clone(),
            });
        }

        bindings.sort_by_key(|b| (b.set, b.binding));
        bindings
    }

    /// Get descriptor set layouts
    pub fn get_set_layouts(&self) -> BTreeMap<u32, Vec<DescriptorBinding>> {
        let mut layouts: BTreeMap<u32, Vec<DescriptorBinding>> = BTreeMap::new();

        for binding in self.get_bindings() {
            layouts.entry(binding.set).or_default().push(binding);
        }

        layouts
    }
}

/// Entry point reflection info
#[derive(Debug, Clone)]
pub struct EntryPointReflection {
    /// Entry point name
    pub name: String,
    /// Execution model
    pub execution_model: ExecutionModel,
    /// Local size (for compute shaders)
    pub local_size: Option<[u32; 3]>,
}

/// Shader input variable
#[derive(Debug, Clone)]
pub struct ShaderInput {
    /// Variable ID
    pub id: Id,
    /// Location
    pub location: u32,
    /// Name
    pub name: Option<String>,
    /// Type info
    pub ty: TypeInfo,
    /// Built-in (if any)
    pub builtin: Option<BuiltIn>,
}

/// Shader output variable
#[derive(Debug, Clone)]
pub struct ShaderOutput {
    /// Variable ID
    pub id: Id,
    /// Location
    pub location: u32,
    /// Name
    pub name: Option<String>,
    /// Type info
    pub ty: TypeInfo,
    /// Built-in (if any)
    pub builtin: Option<BuiltIn>,
}

/// Uniform buffer binding
#[derive(Debug, Clone)]
pub struct UniformBuffer {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
    /// Size in bytes
    pub size: usize,
    /// Struct members
    pub members: Vec<StructMember>,
}

/// Storage buffer binding
#[derive(Debug, Clone)]
pub struct StorageBuffer {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
    /// Size in bytes (0 for runtime arrays)
    pub size: usize,
    /// Read-only (NonWritable decoration)
    pub read_only: bool,
    /// Struct members
    pub members: Vec<StructMember>,
}

/// Push constant block
#[derive(Debug, Clone)]
pub struct PushConstantBlock {
    /// Variable ID
    pub id: Id,
    /// Name
    pub name: Option<String>,
    /// Size in bytes
    pub size: usize,
    /// Struct members
    pub members: Vec<StructMember>,
}

/// Struct member info
#[derive(Debug, Clone)]
pub struct StructMember {
    /// Member name
    pub name: Option<String>,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Type info
    pub ty: TypeInfo,
}

/// Sampled image (combined image sampler)
#[derive(Debug, Clone)]
pub struct SampledImage {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
    /// Dimension
    pub dim: Dim,
    /// Arrayed
    pub arrayed: bool,
    /// Multisampled
    pub multisampled: bool,
    /// Array size (1 for non-arrays)
    pub array_size: u32,
}

/// Storage image
#[derive(Debug, Clone)]
pub struct StorageImage {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
    /// Dimension
    pub dim: Dim,
    /// Image format
    pub format: ImageFormat,
    /// Arrayed
    pub arrayed: bool,
    /// Array size (1 for non-arrays)
    pub array_size: u32,
}

/// Separate sampler
#[derive(Debug, Clone)]
pub struct SeparateSampler {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
}

/// Separate image
#[derive(Debug, Clone)]
pub struct SeparateImage {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
    /// Dimension
    pub dim: Dim,
    /// Arrayed
    pub arrayed: bool,
    /// Multisampled
    pub multisampled: bool,
    /// Array size
    pub array_size: u32,
}

/// Input attachment
#[derive(Debug, Clone)]
pub struct InputAttachment {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
    /// Input attachment index
    pub index: u32,
}

/// Acceleration structure binding
#[derive(Debug, Clone)]
pub struct AccelerationStructureBinding {
    /// Variable ID
    pub id: Id,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Name
    pub name: Option<String>,
}

/// Specialization constant
#[derive(Debug, Clone)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: Id,
    /// Specialization ID
    pub spec_id: u32,
    /// Name
    pub name: Option<String>,
    /// Default value
    pub default_value: Option<ConstantValue>,
}

/// Constant value
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Bool(bool),
    Int(i32),
    Uint(u32),
    Float(f32),
}

/// Type info for reflection
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Unknown,
    Scalar(ScalarInfo),
    Vector { scalar: ScalarInfo, count: u8 },
    Matrix { columns: u8, rows: u8 },
    Array { element: Box<TypeInfo>, size: Option<u32> },
    Struct,
}

/// Scalar type info
#[derive(Debug, Clone)]
pub enum ScalarInfo {
    Bool,
    Int { width: u32, signed: bool },
    Float(u32),
}

/// Descriptor binding info
#[derive(Debug, Clone)]
pub struct DescriptorBinding {
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor count (for arrays)
    pub count: u32,
    /// Name
    pub name: Option<String>,
}

/// Descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    Sampler,
    CombinedImageSampler,
    SampledImage,
    StorageImage,
    UniformTexelBuffer,
    StorageTexelBuffer,
    UniformBuffer,
    StorageBuffer,
    UniformBufferDynamic,
    StorageBufferDynamic,
    InputAttachment,
    AccelerationStructure,
}

impl DescriptorType {
    /// Get Vulkan VkDescriptorType value
    pub fn to_vulkan(&self) -> u32 {
        match self {
            DescriptorType::Sampler => 0,
            DescriptorType::CombinedImageSampler => 1,
            DescriptorType::SampledImage => 2,
            DescriptorType::StorageImage => 3,
            DescriptorType::UniformTexelBuffer => 4,
            DescriptorType::StorageTexelBuffer => 5,
            DescriptorType::UniformBuffer => 6,
            DescriptorType::StorageBuffer => 7,
            DescriptorType::UniformBufferDynamic => 8,
            DescriptorType::StorageBufferDynamic => 9,
            DescriptorType::InputAttachment => 10,
            DescriptorType::AccelerationStructure => 1000150000,
        }
    }
}

/// Cross-stage interface validation
pub fn validate_interface(
    outputs: &[ShaderOutput],
    inputs: &[ShaderInput],
) -> Result<(), InterfaceMismatch> {
    for input in inputs {
        // Skip built-ins
        if input.builtin.is_some() {
            continue;
        }

        let matching_output = outputs.iter().find(|o| o.location == input.location);

        match matching_output {
            Some(output) => {
                // Check type compatibility
                if !types_compatible(&output.ty, &input.ty) {
                    return Err(InterfaceMismatch::TypeMismatch {
                        location: input.location,
                        expected: format!("{:?}", input.ty),
                        found: format!("{:?}", output.ty),
                    });
                }
            }
            None => {
                return Err(InterfaceMismatch::MissingOutput {
                    location: input.location,
                    name: input.name.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Check if types are compatible
fn types_compatible(a: &TypeInfo, b: &TypeInfo) -> bool {
    match (a, b) {
        (TypeInfo::Scalar(sa), TypeInfo::Scalar(sb)) => scalars_compatible(sa, sb),
        (
            TypeInfo::Vector {
                scalar: sa,
                count: ca,
            },
            TypeInfo::Vector {
                scalar: sb,
                count: cb,
            },
        ) => ca == cb && scalars_compatible(sa, sb),
        (
            TypeInfo::Matrix {
                columns: ca,
                rows: ra,
            },
            TypeInfo::Matrix {
                columns: cb,
                rows: rb,
            },
        ) => ca == cb && ra == rb,
        _ => false,
    }
}

/// Check if scalar types are compatible
fn scalars_compatible(a: &ScalarInfo, b: &ScalarInfo) -> bool {
    match (a, b) {
        (ScalarInfo::Bool, ScalarInfo::Bool) => true,
        (
            ScalarInfo::Int {
                width: wa,
                signed: sa,
            },
            ScalarInfo::Int {
                width: wb,
                signed: sb,
            },
        ) => wa == wb && sa == sb,
        (ScalarInfo::Float(wa), ScalarInfo::Float(wb)) => wa == wb,
        _ => false,
    }
}

/// Interface mismatch error
#[derive(Debug, Clone)]
pub enum InterfaceMismatch {
    MissingOutput {
        location: u32,
        name: Option<String>,
    },
    TypeMismatch {
        location: u32,
        expected: String,
        found: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_type_vulkan() {
        assert_eq!(DescriptorType::UniformBuffer.to_vulkan(), 6);
        assert_eq!(DescriptorType::StorageBuffer.to_vulkan(), 7);
        assert_eq!(DescriptorType::CombinedImageSampler.to_vulkan(), 1);
    }

    #[test]
    fn test_type_compatibility() {
        let float32 = TypeInfo::Scalar(ScalarInfo::Float(32));
        let float32_b = TypeInfo::Scalar(ScalarInfo::Float(32));
        let float64 = TypeInfo::Scalar(ScalarInfo::Float(64));

        assert!(types_compatible(&float32, &float32_b));
        assert!(!types_compatible(&float32, &float64));
    }
}
