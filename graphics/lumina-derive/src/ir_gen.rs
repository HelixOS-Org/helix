//! IR generation from analyzed shader code.
//!
//! This module converts analyzed shader code into LUMINA IR.

use crate::analyze::{
    AnalysisContext, AnalyzedEntryPoint, AnalyzedInput, AnalyzedOutput, AnalyzedResource,
    BuiltinVar, Interpolation,
};
use crate::error::{Error, ErrorKind, Result};
use crate::parse::{ResourceKind, ShaderStage};
use crate::types::{ShaderType, StorageClass};
use proc_macro2::Span;
use std::collections::HashMap;

/// IR generator for shader code.
pub struct IrGenerator {
    /// Generated module.
    module: IrModule,
    /// Current function being built.
    current_function: Option<usize>,
    /// Current block being built.
    current_block: Option<usize>,
    /// Value counter for SSA.
    value_counter: u32,
    /// Type IDs.
    type_ids: HashMap<String, u32>,
    /// Variable IDs.
    variable_ids: HashMap<String, u32>,
}

/// IR module representation.
#[derive(Debug, Default)]
pub struct IrModule {
    /// Module name.
    pub name: String,
    /// Type definitions.
    pub types: Vec<IrType>,
    /// Global variables.
    pub globals: Vec<IrGlobal>,
    /// Functions.
    pub functions: Vec<IrFunction>,
    /// Entry points.
    pub entry_points: Vec<IrEntryPoint>,
}

/// IR type definition.
#[derive(Debug, Clone)]
pub struct IrType {
    /// Type ID.
    pub id: u32,
    /// Type kind.
    pub kind: IrTypeKind,
}

/// IR type kinds.
#[derive(Debug, Clone)]
pub enum IrTypeKind {
    Void,
    Bool,
    Int { width: u32, signed: bool },
    Float { width: u32 },
    Vector { element: u32, size: u32 },
    Matrix { element: u32, cols: u32, rows: u32 },
    Array { element: u32, size: u32 },
    RuntimeArray { element: u32 },
    Struct { name: String, members: Vec<IrStructMember> },
    Pointer { pointee: u32, storage: StorageClass },
    Image { sampled_type: u32, dim: ImageDim, depth: bool, arrayed: bool, ms: bool },
    Sampler,
    SampledImage { image: u32 },
    Function { return_type: u32, params: Vec<u32> },
    AccelerationStructure,
    RayQuery,
}

/// IR struct member.
#[derive(Debug, Clone)]
pub struct IrStructMember {
    pub name: String,
    pub ty: u32,
    pub offset: Option<u32>,
}

/// Image dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDim {
    Dim1D,
    Dim2D,
    Dim3D,
    Cube,
    Rect,
    Buffer,
    SubpassData,
}

/// IR global variable.
#[derive(Debug, Clone)]
pub struct IrGlobal {
    /// Variable ID.
    pub id: u32,
    /// Variable name.
    pub name: String,
    /// Type (pointer type).
    pub ty: u32,
    /// Storage class.
    pub storage: StorageClass,
    /// Decorations.
    pub decorations: Vec<IrDecoration>,
}

/// IR function.
#[derive(Debug)]
pub struct IrFunction {
    /// Function ID.
    pub id: u32,
    /// Function name.
    pub name: String,
    /// Return type.
    pub return_type: u32,
    /// Parameters.
    pub params: Vec<IrParameter>,
    /// Basic blocks.
    pub blocks: Vec<IrBlock>,
}

/// IR parameter.
#[derive(Debug)]
pub struct IrParameter {
    pub id: u32,
    pub name: String,
    pub ty: u32,
}

/// IR basic block.
#[derive(Debug)]
pub struct IrBlock {
    /// Block ID.
    pub id: u32,
    /// Block label.
    pub label: Option<String>,
    /// Instructions.
    pub instructions: Vec<IrInstruction>,
}

/// IR instruction.
#[derive(Debug, Clone)]
pub struct IrInstruction {
    /// Result ID (if any).
    pub result: Option<u32>,
    /// Result type (if any).
    pub result_type: Option<u32>,
    /// Operation.
    pub op: IrOp,
    /// Operands.
    pub operands: Vec<IrOperand>,
}

/// IR operations.
#[derive(Debug, Clone)]
pub enum IrOp {
    // Memory
    Load,
    Store,
    AccessChain,
    Variable,
    CopyMemory,

    // Constants
    Constant,
    ConstantComposite,
    ConstantNull,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,

    // Floating-point
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    FNeg,

    // Integer
    IAdd,
    ISub,
    IMul,
    IDiv,
    SMod,
    UMod,
    SNeg,

    // Logical
    LogicalNot,
    LogicalAnd,
    LogicalOr,
    LogicalEqual,
    LogicalNotEqual,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,
    FLessThan,
    FLessOrEqual,
    FGreaterThan,
    FGreaterOrEqual,
    SLessThan,
    SLessOrEqual,
    SGreaterThan,
    SGreaterOrEqual,
    ULessThan,
    ULessOrEqual,
    UGreaterThan,
    UGreaterOrEqual,

    // Bitwise
    BitwiseNot,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRightLogical,
    ShiftRightArithmetic,

    // Vector/Matrix
    VectorShuffle,
    CompositeConstruct,
    CompositeExtract,
    CompositeInsert,
    VectorExtractDynamic,
    VectorInsertDynamic,
    MatrixTimesVector,
    VectorTimesMatrix,
    MatrixTimesMatrix,
    Transpose,

    // Conversion
    ConvertFToS,
    ConvertFToU,
    ConvertSToF,
    ConvertUToF,
    FConvert,
    SConvert,
    UConvert,
    Bitcast,

    // Control flow
    Label,
    Branch,
    BranchConditional,
    Switch,
    Return,
    ReturnValue,
    Kill,
    Unreachable,

    // Function
    FunctionCall,
    FunctionParameter,

    // Selection
    Select,
    Phi,

    // Derivative
    DPdx,
    DPdy,
    Fwidth,
    DPdxFine,
    DPdyFine,
    FwidthFine,
    DPdxCoarse,
    DPdyCoarse,
    FwidthCoarse,

    // Image
    ImageSampleImplicitLod,
    ImageSampleExplicitLod,
    ImageSampleDrefImplicitLod,
    ImageSampleDrefExplicitLod,
    ImageFetch,
    ImageGather,
    ImageRead,
    ImageWrite,
    ImageQuerySize,
    ImageQueryLod,
    ImageQueryLevels,
    ImageQuerySamples,

    // Atomic
    AtomicLoad,
    AtomicStore,
    AtomicExchange,
    AtomicCompareExchange,
    AtomicIAdd,
    AtomicISub,
    AtomicSMin,
    AtomicUMin,
    AtomicSMax,
    AtomicUMax,
    AtomicAnd,
    AtomicOr,
    AtomicXor,

    // Barrier
    ControlBarrier,
    MemoryBarrier,

    // Ray tracing
    TraceRayKHR,
    ReportIntersectionKHR,
    IgnoreIntersectionKHR,
    TerminateRayKHR,
    ExecuteCallableKHR,
    RayQueryInitializeKHR,
    RayQueryProceedKHR,
    RayQueryGetIntersectionTypeKHR,

    // Mesh shader
    SetMeshOutputsEXT,
    EmitMeshTasksEXT,

    // Subgroup
    GroupNonUniformElect,
    GroupNonUniformAll,
    GroupNonUniformAny,
    GroupNonUniformBallot,
    GroupNonUniformBroadcast,
    GroupNonUniformShuffle,
    GroupNonUniformIAdd,
    GroupNonUniformFAdd,
    GroupNonUniformIMul,
    GroupNonUniformFMul,
    GroupNonUniformSMin,
    GroupNonUniformUMin,
    GroupNonUniformFMin,
    GroupNonUniformSMax,
    GroupNonUniformUMax,
    GroupNonUniformFMax,

    // Extended instructions (GLSL.std.450)
    ExtInst,

    // Special
    Nop,
    Undef,
}

/// IR operand.
#[derive(Debug, Clone)]
pub enum IrOperand {
    /// ID reference.
    Id(u32),
    /// Literal integer.
    LitInt(u32),
    /// Literal float.
    LitFloat(f32),
    /// Literal string.
    LitString(String),
    /// Memory access mask.
    MemoryAccess(u32),
    /// Scope.
    Scope(u32),
    /// Semantics.
    Semantics(u32),
}

/// IR decoration.
#[derive(Debug, Clone)]
pub enum IrDecoration {
    Location(u32),
    Binding(u32),
    DescriptorSet(u32),
    BuiltIn(u32),
    Flat,
    NoPerspective,
    Centroid,
    Sample,
    Block,
    BufferBlock,
    RowMajor,
    ColMajor,
    ArrayStride(u32),
    MatrixStride(u32),
    Offset(u32),
    NonWritable,
    NonReadable,
    Restrict,
    Aliased,
    Volatile,
    Coherent,
    SpecId(u32),
}

/// IR entry point.
#[derive(Debug)]
pub struct IrEntryPoint {
    /// Entry point name.
    pub name: String,
    /// Execution model.
    pub execution_model: IrExecutionModel,
    /// Function ID.
    pub function: u32,
    /// Interface variables.
    pub interface: Vec<u32>,
    /// Execution modes.
    pub execution_modes: Vec<IrExecutionMode>,
}

/// IR execution model.
#[derive(Debug, Clone, Copy)]
pub enum IrExecutionModel {
    Vertex,
    TessellationControl,
    TessellationEvaluation,
    Geometry,
    Fragment,
    GLCompute,
    Kernel,
    TaskNV,
    MeshNV,
    TaskEXT,
    MeshEXT,
    RayGenerationKHR,
    IntersectionKHR,
    AnyHitKHR,
    ClosestHitKHR,
    MissKHR,
    CallableKHR,
}

impl From<ShaderStage> for IrExecutionModel {
    fn from(stage: ShaderStage) -> Self {
        match stage {
            ShaderStage::Vertex => IrExecutionModel::Vertex,
            ShaderStage::Fragment => IrExecutionModel::Fragment,
            ShaderStage::Compute => IrExecutionModel::GLCompute,
            ShaderStage::Geometry => IrExecutionModel::Geometry,
            ShaderStage::TessellationControl => IrExecutionModel::TessellationControl,
            ShaderStage::TessellationEvaluation => IrExecutionModel::TessellationEvaluation,
            ShaderStage::Mesh => IrExecutionModel::MeshEXT,
            ShaderStage::Task => IrExecutionModel::TaskEXT,
            ShaderStage::RayGeneration => IrExecutionModel::RayGenerationKHR,
            ShaderStage::ClosestHit => IrExecutionModel::ClosestHitKHR,
            ShaderStage::AnyHit => IrExecutionModel::AnyHitKHR,
            ShaderStage::Miss => IrExecutionModel::MissKHR,
            ShaderStage::Intersection => IrExecutionModel::IntersectionKHR,
            ShaderStage::Callable => IrExecutionModel::CallableKHR,
        }
    }
}

/// IR execution mode.
#[derive(Debug, Clone)]
pub enum IrExecutionMode {
    LocalSize(u32, u32, u32),
    LocalSizeId(u32, u32, u32),
    OriginUpperLeft,
    OriginLowerLeft,
    EarlyFragmentTests,
    DepthReplacing,
    DepthGreater,
    DepthLess,
    DepthUnchanged,
    Invocations(u32),
    InputPoints,
    InputLines,
    InputLinesAdjacency,
    InputTriangles,
    InputTrianglesAdjacency,
    Quads,
    Isolines,
    OutputVertices(u32),
    OutputPoints,
    OutputLineStrip,
    OutputTriangleStrip,
    SpacingEqual,
    SpacingFractionalEven,
    SpacingFractionalOdd,
    VertexOrderCw,
    VertexOrderCcw,
    PointMode,
    OutputPrimitivesNV(u32),
}

impl IrGenerator {
    /// Create a new IR generator.
    pub fn new() -> Self {
        Self {
            module: IrModule::default(),
            current_function: None,
            current_block: None,
            value_counter: 1,
            type_ids: HashMap::new(),
            variable_ids: HashMap::new(),
        }
    }

    /// Generate IR from analysis context.
    pub fn generate(&mut self, ctx: &AnalysisContext) -> Result<IrModule> {
        self.module.name = "shader".to_string();

        // Generate types
        self.generate_builtin_types();

        // Generate resources
        for resource in &ctx.resources {
            self.generate_resource(resource)?;
        }

        // Generate entry points
        for entry in &ctx.entry_points {
            self.generate_entry_point(entry, ctx)?;
        }

        Ok(std::mem::take(&mut self.module))
    }

    fn next_id(&mut self) -> u32 {
        let id = self.value_counter;
        self.value_counter += 1;
        id
    }

    fn generate_builtin_types(&mut self) {
        // Void
        let void_id = self.next_id();
        self.module.types.push(IrType {
            id: void_id,
            kind: IrTypeKind::Void,
        });
        self.type_ids.insert("void".to_string(), void_id);

        // Bool
        let bool_id = self.next_id();
        self.module.types.push(IrType {
            id: bool_id,
            kind: IrTypeKind::Bool,
        });
        self.type_ids.insert("bool".to_string(), bool_id);

        // Int32
        let i32_id = self.next_id();
        self.module.types.push(IrType {
            id: i32_id,
            kind: IrTypeKind::Int { width: 32, signed: true },
        });
        self.type_ids.insert("i32".to_string(), i32_id);

        // Uint32
        let u32_id = self.next_id();
        self.module.types.push(IrType {
            id: u32_id,
            kind: IrTypeKind::Int { width: 32, signed: false },
        });
        self.type_ids.insert("u32".to_string(), u32_id);

        // Float32
        let f32_id = self.next_id();
        self.module.types.push(IrType {
            id: f32_id,
            kind: IrTypeKind::Float { width: 32 },
        });
        self.type_ids.insert("f32".to_string(), f32_id);

        // Float64
        let f64_id = self.next_id();
        self.module.types.push(IrType {
            id: f64_id,
            kind: IrTypeKind::Float { width: 64 },
        });
        self.type_ids.insert("f64".to_string(), f64_id);

        // Common vector types
        for size in [2, 3, 4] {
            // Vec<n>
            let vec_id = self.next_id();
            self.module.types.push(IrType {
                id: vec_id,
                kind: IrTypeKind::Vector { element: f32_id, size },
            });
            self.type_ids.insert(format!("Vec{}", size), vec_id);

            // IVec<n>
            let ivec_id = self.next_id();
            self.module.types.push(IrType {
                id: ivec_id,
                kind: IrTypeKind::Vector { element: i32_id, size },
            });
            self.type_ids.insert(format!("IVec{}", size), ivec_id);

            // UVec<n>
            let uvec_id = self.next_id();
            self.module.types.push(IrType {
                id: uvec_id,
                kind: IrTypeKind::Vector { element: u32_id, size },
            });
            self.type_ids.insert(format!("UVec{}", size), uvec_id);

            // BVec<n>
            let bvec_id = self.next_id();
            self.module.types.push(IrType {
                id: bvec_id,
                kind: IrTypeKind::Vector { element: bool_id, size },
            });
            self.type_ids.insert(format!("BVec{}", size), bvec_id);
        }

        // Common matrix types
        let vec4_id = *self.type_ids.get("Vec4").unwrap();
        let vec3_id = *self.type_ids.get("Vec3").unwrap();
        let vec2_id = *self.type_ids.get("Vec2").unwrap();

        for (name, element, cols, rows) in [
            ("Mat2", f32_id, 2, 2),
            ("Mat3", f32_id, 3, 3),
            ("Mat4", f32_id, 4, 4),
            ("Mat2x2", f32_id, 2, 2),
            ("Mat2x3", f32_id, 2, 3),
            ("Mat2x4", f32_id, 2, 4),
            ("Mat3x2", f32_id, 3, 2),
            ("Mat3x3", f32_id, 3, 3),
            ("Mat3x4", f32_id, 3, 4),
            ("Mat4x2", f32_id, 4, 2),
            ("Mat4x3", f32_id, 4, 3),
            ("Mat4x4", f32_id, 4, 4),
        ] {
            let mat_id = self.next_id();
            self.module.types.push(IrType {
                id: mat_id,
                kind: IrTypeKind::Matrix { element, cols, rows },
            });
            self.type_ids.insert(name.to_string(), mat_id);
        }

        // Sampler
        let sampler_id = self.next_id();
        self.module.types.push(IrType {
            id: sampler_id,
            kind: IrTypeKind::Sampler,
        });
        self.type_ids.insert("Sampler".to_string(), sampler_id);
    }

    fn get_or_create_type(&mut self, shader_type: &ShaderType) -> u32 {
        let type_name = self.type_to_name(shader_type);

        if let Some(&id) = self.type_ids.get(&type_name) {
            return id;
        }

        let id = self.next_id();
        let kind = self.shader_type_to_ir(shader_type);

        self.module.types.push(IrType { id, kind });
        self.type_ids.insert(type_name, id);
        id
    }

    fn type_to_name(&self, shader_type: &ShaderType) -> String {
        match shader_type {
            ShaderType::Void => "void".to_string(),
            ShaderType::Bool => "bool".to_string(),
            ShaderType::Int32 => "i32".to_string(),
            ShaderType::Uint32 => "u32".to_string(),
            ShaderType::Float32 => "f32".to_string(),
            ShaderType::Float64 => "f64".to_string(),
            ShaderType::Vector { element, size } => {
                let elem_name = match element.as_ref() {
                    ShaderType::Float32 => "Vec",
                    ShaderType::Int32 => "IVec",
                    ShaderType::Uint32 => "UVec",
                    ShaderType::Bool => "BVec",
                    ShaderType::Float64 => "DVec",
                    _ => "Vec",
                };
                format!("{}{}", elem_name, size)
            }
            ShaderType::Matrix { cols, rows, .. } => {
                if cols == rows {
                    format!("Mat{}", cols)
                } else {
                    format!("Mat{}x{}", cols, rows)
                }
            }
            ShaderType::Array { element, size } => {
                format!("Array_{}_{}", self.type_to_name(element), size)
            }
            ShaderType::RuntimeArray { element } => {
                format!("RuntimeArray_{}", self.type_to_name(element))
            }
            ShaderType::Struct { name, .. } => name.clone(),
            ShaderType::Pointer { pointee, storage } => {
                format!("Ptr_{:?}_{}", storage, self.type_to_name(pointee))
            }
            ShaderType::Sampler => "Sampler".to_string(),
            ShaderType::Texture2D { .. } => "Texture2D".to_string(),
            ShaderType::Texture3D { .. } => "Texture3D".to_string(),
            ShaderType::TextureCube { .. } => "TextureCube".to_string(),
            ShaderType::Texture2DArray { .. } => "Texture2DArray".to_string(),
            ShaderType::SampledImage { .. } => "SampledImage".to_string(),
            ShaderType::StorageImage { .. } => "StorageImage".to_string(),
            ShaderType::AccelerationStructure => "AccelerationStructure".to_string(),
            ShaderType::RayQuery => "RayQuery".to_string(),
        }
    }

    fn shader_type_to_ir(&mut self, shader_type: &ShaderType) -> IrTypeKind {
        match shader_type {
            ShaderType::Void => IrTypeKind::Void,
            ShaderType::Bool => IrTypeKind::Bool,
            ShaderType::Int32 => IrTypeKind::Int { width: 32, signed: true },
            ShaderType::Uint32 => IrTypeKind::Int { width: 32, signed: false },
            ShaderType::Float32 => IrTypeKind::Float { width: 32 },
            ShaderType::Float64 => IrTypeKind::Float { width: 64 },
            ShaderType::Vector { element, size } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Vector { element: elem_id, size: *size }
            }
            ShaderType::Matrix { element, cols, rows } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Matrix { element: elem_id, cols: *cols, rows: *rows }
            }
            ShaderType::Array { element, size } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Array { element: elem_id, size: *size }
            }
            ShaderType::RuntimeArray { element } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::RuntimeArray { element: elem_id }
            }
            ShaderType::Struct { name, members } => {
                let ir_members: Vec<_> = members
                    .iter()
                    .map(|m| {
                        let ty_id = self.get_or_create_type(&m.ty);
                        IrStructMember {
                            name: m.name.clone(),
                            ty: ty_id,
                            offset: m.offset,
                        }
                    })
                    .collect();
                IrTypeKind::Struct { name: name.clone(), members: ir_members }
            }
            ShaderType::Pointer { pointee, storage } => {
                let pointee_id = self.get_or_create_type(pointee);
                IrTypeKind::Pointer { pointee: pointee_id, storage: *storage }
            }
            ShaderType::Sampler => IrTypeKind::Sampler,
            ShaderType::Texture2D { element } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Image {
                    sampled_type: elem_id,
                    dim: ImageDim::Dim2D,
                    depth: false,
                    arrayed: false,
                    ms: false,
                }
            }
            ShaderType::Texture3D { element } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Image {
                    sampled_type: elem_id,
                    dim: ImageDim::Dim3D,
                    depth: false,
                    arrayed: false,
                    ms: false,
                }
            }
            ShaderType::TextureCube { element } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Image {
                    sampled_type: elem_id,
                    dim: ImageDim::Cube,
                    depth: false,
                    arrayed: false,
                    ms: false,
                }
            }
            ShaderType::Texture2DArray { element } => {
                let elem_id = self.get_or_create_type(element);
                IrTypeKind::Image {
                    sampled_type: elem_id,
                    dim: ImageDim::Dim2D,
                    depth: false,
                    arrayed: true,
                    ms: false,
                }
            }
            ShaderType::SampledImage { image } => {
                let image_id = self.get_or_create_type(image);
                IrTypeKind::SampledImage { image: image_id }
            }
            ShaderType::StorageImage { .. } => {
                let vec4_id = *self.type_ids.get("Vec4").unwrap();
                IrTypeKind::Image {
                    sampled_type: vec4_id,
                    dim: ImageDim::Dim2D,
                    depth: false,
                    arrayed: false,
                    ms: false,
                }
            }
            ShaderType::AccelerationStructure => IrTypeKind::AccelerationStructure,
            ShaderType::RayQuery => IrTypeKind::RayQuery,
        }
    }

    fn generate_resource(&mut self, resource: &AnalyzedResource) -> Result<()> {
        let ty_id = self.get_or_create_type(&resource.ty);

        // Create pointer type
        let storage = match resource.kind {
            ResourceKind::UniformBuffer => StorageClass::Uniform,
            ResourceKind::StorageBuffer => StorageClass::StorageBuffer,
            ResourceKind::SampledImage | ResourceKind::StorageImage | ResourceKind::Sampler => {
                StorageClass::UniformConstant
            }
            ResourceKind::PushConstant => StorageClass::PushConstant,
            ResourceKind::AccelerationStructure => StorageClass::UniformConstant,
            _ => StorageClass::UniformConstant,
        };

        let ptr_ty = ShaderType::Pointer {
            pointee: Box::new(resource.ty.clone()),
            storage,
        };
        let ptr_ty_id = self.get_or_create_type(&ptr_ty);

        let var_id = self.next_id();
        let mut decorations = Vec::new();

        // Add binding decorations
        if resource.kind != ResourceKind::PushConstant {
            decorations.push(IrDecoration::DescriptorSet(resource.set));
            decorations.push(IrDecoration::Binding(resource.binding));
        }

        // Add block decoration for buffers
        if matches!(resource.kind, ResourceKind::UniformBuffer | ResourceKind::StorageBuffer) {
            decorations.push(IrDecoration::Block);
        }

        self.module.globals.push(IrGlobal {
            id: var_id,
            name: resource.name.clone(),
            ty: ptr_ty_id,
            storage,
            decorations,
        });

        self.variable_ids.insert(resource.name.clone(), var_id);

        Ok(())
    }

    fn generate_entry_point(
        &mut self,
        entry: &AnalyzedEntryPoint,
        ctx: &AnalysisContext,
    ) -> Result<()> {
        let mut interface_vars = Vec::new();

        // Generate input variables
        for input in &entry.inputs {
            let var_id = self.generate_input_variable(input, entry.stage)?;
            interface_vars.push(var_id);
        }

        // Generate output variables
        for output in &entry.outputs {
            let var_id = self.generate_output_variable(output, entry.stage)?;
            interface_vars.push(var_id);
        }

        // Create function
        let void_id = *self.type_ids.get("void").unwrap();
        let func_id = self.next_id();

        let function = IrFunction {
            id: func_id,
            name: entry.name.clone(),
            return_type: void_id,
            params: Vec::new(),
            blocks: vec![IrBlock {
                id: self.next_id(),
                label: Some("entry".to_string()),
                instructions: vec![IrInstruction {
                    result: None,
                    result_type: None,
                    op: IrOp::Return,
                    operands: Vec::new(),
                }],
            }],
        };

        self.module.functions.push(function);

        // Create entry point
        let mut execution_modes = Vec::new();

        // Add execution modes
        match entry.stage {
            ShaderStage::Compute | ShaderStage::Mesh | ShaderStage::Task => {
                if let Some((x, y, z)) = entry.local_size {
                    execution_modes.push(IrExecutionMode::LocalSize(x, y, z));
                }
            }
            ShaderStage::Fragment => {
                execution_modes.push(IrExecutionMode::OriginUpperLeft);
            }
            _ => {}
        }

        self.module.entry_points.push(IrEntryPoint {
            name: entry.name.clone(),
            execution_model: entry.stage.into(),
            function: func_id,
            interface: interface_vars,
            execution_modes,
        });

        Ok(())
    }

    fn generate_input_variable(
        &mut self,
        input: &AnalyzedInput,
        stage: ShaderStage,
    ) -> Result<u32> {
        let ty_id = self.get_or_create_type(&input.ty);

        let ptr_ty = ShaderType::Pointer {
            pointee: Box::new(input.ty.clone()),
            storage: StorageClass::Input,
        };
        let ptr_ty_id = self.get_or_create_type(&ptr_ty);

        let var_id = self.next_id();
        let mut decorations = Vec::new();

        if let Some(loc) = input.location {
            decorations.push(IrDecoration::Location(loc));
        }

        if let Some(builtin) = &input.builtin {
            decorations.push(IrDecoration::BuiltIn(builtin_to_spirv(*builtin)));
        }

        match input.interpolation {
            Interpolation::Flat => decorations.push(IrDecoration::Flat),
            Interpolation::NoPerspective => decorations.push(IrDecoration::NoPerspective),
            Interpolation::Centroid => decorations.push(IrDecoration::Centroid),
            Interpolation::Sample => decorations.push(IrDecoration::Sample),
            Interpolation::Smooth => {}
        }

        self.module.globals.push(IrGlobal {
            id: var_id,
            name: input.name.clone(),
            ty: ptr_ty_id,
            storage: StorageClass::Input,
            decorations,
        });

        self.variable_ids.insert(input.name.clone(), var_id);

        Ok(var_id)
    }

    fn generate_output_variable(
        &mut self,
        output: &AnalyzedOutput,
        stage: ShaderStage,
    ) -> Result<u32> {
        let ty_id = self.get_or_create_type(&output.ty);

        let ptr_ty = ShaderType::Pointer {
            pointee: Box::new(output.ty.clone()),
            storage: StorageClass::Output,
        };
        let ptr_ty_id = self.get_or_create_type(&ptr_ty);

        let var_id = self.next_id();
        let mut decorations = Vec::new();

        if let Some(loc) = output.location {
            decorations.push(IrDecoration::Location(loc));
        }

        if let Some(builtin) = &output.builtin {
            decorations.push(IrDecoration::BuiltIn(builtin_to_spirv(*builtin)));
        }

        self.module.globals.push(IrGlobal {
            id: var_id,
            name: output.name.clone(),
            ty: ptr_ty_id,
            storage: StorageClass::Output,
            decorations,
        });

        Ok(var_id)
    }
}

impl Default for IrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn builtin_to_spirv(builtin: BuiltinVar) -> u32 {
    match builtin {
        BuiltinVar::Position => 0,
        BuiltinVar::PointSize => 1,
        BuiltinVar::ClipDistance => 3,
        BuiltinVar::CullDistance => 4,
        BuiltinVar::VertexIndex => 42,
        BuiltinVar::InstanceIndex => 43,
        BuiltinVar::PrimitiveId => 7,
        BuiltinVar::InvocationId => 8,
        BuiltinVar::Layer => 9,
        BuiltinVar::ViewportIndex => 10,
        BuiltinVar::FragCoord => 15,
        BuiltinVar::PointCoord => 16,
        BuiltinVar::FrontFacing => 17,
        BuiltinVar::SampleId => 18,
        BuiltinVar::SamplePosition => 19,
        BuiltinVar::SampleMask => 20,
        BuiltinVar::FragDepth => 22,
        BuiltinVar::HelperInvocation => 23,
        BuiltinVar::NumWorkGroups => 24,
        BuiltinVar::WorkGroupSize => 25,
        BuiltinVar::WorkGroupId => 26,
        BuiltinVar::LocalInvocationId => 27,
        BuiltinVar::GlobalInvocationId => 28,
        BuiltinVar::LocalInvocationIndex => 29,
        BuiltinVar::SubgroupSize => 36,
        BuiltinVar::SubgroupInvocationId => 41,
        BuiltinVar::SubgroupEqMask => 4416,
        BuiltinVar::SubgroupGeMask => 4417,
        BuiltinVar::SubgroupGtMask => 4418,
        BuiltinVar::SubgroupLeMask => 4419,
        BuiltinVar::SubgroupLtMask => 4420,
        BuiltinVar::DrawIndex => 4426,
        BuiltinVar::BaseVertex => 4424,
        BuiltinVar::BaseInstance => 4425,
        BuiltinVar::TessLevelOuter => 11,
        BuiltinVar::TessLevelInner => 12,
        BuiltinVar::TessCoord => 13,
        BuiltinVar::PatchVertices => 14,
        BuiltinVar::LaunchIdKHR => 5319,
        BuiltinVar::LaunchSizeKHR => 5320,
        BuiltinVar::WorldRayOriginKHR => 5321,
        BuiltinVar::WorldRayDirectionKHR => 5322,
        BuiltinVar::ObjectRayOriginKHR => 5323,
        BuiltinVar::ObjectRayDirectionKHR => 5324,
        BuiltinVar::RayTminKHR => 5325,
        BuiltinVar::RayTmaxKHR => 5326,
        BuiltinVar::InstanceCustomIndexKHR => 5327,
        BuiltinVar::ObjectToWorldKHR => 5330,
        BuiltinVar::WorldToObjectKHR => 5331,
        BuiltinVar::HitTKHR => 5332,
        BuiltinVar::HitKindKHR => 5333,
        BuiltinVar::IncomingRayFlagsKHR => 5351,
        BuiltinVar::InstanceId => 5327,
        BuiltinVar::RayGeometryIndexKHR => 5352,
        BuiltinVar::GeometryIndexKHR => 5352,
        BuiltinVar::PrimitiveIdKHR => 7,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_generator_new() {
        let gen = IrGenerator::new();
        assert_eq!(gen.value_counter, 1);
    }

    #[test]
    fn test_builtin_types() {
        let mut gen = IrGenerator::new();
        gen.generate_builtin_types();

        assert!(gen.type_ids.contains_key("void"));
        assert!(gen.type_ids.contains_key("f32"));
        assert!(gen.type_ids.contains_key("Vec3"));
        assert!(gen.type_ids.contains_key("Mat4"));
    }

    #[test]
    fn test_execution_model_conversion() {
        assert!(matches!(
            IrExecutionModel::from(ShaderStage::Vertex),
            IrExecutionModel::Vertex
        ));
        assert!(matches!(
            IrExecutionModel::from(ShaderStage::Fragment),
            IrExecutionModel::Fragment
        ));
        assert!(matches!(
            IrExecutionModel::from(ShaderStage::Compute),
            IrExecutionModel::GLCompute
        ));
    }
}
