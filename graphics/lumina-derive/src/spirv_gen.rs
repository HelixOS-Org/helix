//! SPIR-V code generation from IR.
//!
//! This module converts LUMINA IR to SPIR-V binary format.

use std::collections::HashMap;

use proc_macro2::Span;

use crate::error::{Error, Result};
use crate::ir_gen::{
    ImageDim, IrBlock, IrDecoration, IrEntryPoint, IrExecutionMode, IrExecutionModel, IrFunction,
    IrGlobal, IrInstruction, IrModule, IrOp, IrOperand, IrType, IrTypeKind,
};
use crate::types::StorageClass;

/// SPIR-V generator.
pub struct SpirVGenerator {
    /// Generated words.
    words: Vec<u32>,
    /// Bound (highest ID + 1).
    bound: u32,
    /// ID mapping from IR to SPIR-V.
    id_map: HashMap<u32, u32>,
    /// Type ID mapping.
    type_ids: HashMap<String, u32>,
    /// Capabilities needed.
    capabilities: Vec<Capability>,
    /// Extensions needed.
    extensions: Vec<String>,
    /// GLSL.std.450 import ID.
    glsl_ext_id: Option<u32>,
}

/// SPIR-V capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    Matrix,
    Shader,
    Geometry,
    Tessellation,
    Float16,
    Float64,
    Int64,
    Int16,
    Int8,
    StorageBuffer16BitAccess,
    UniformAndStorageBuffer16BitAccess,
    StoragePushConstant16,
    StorageInputOutput16,
    DeviceGroup,
    MultiView,
    VariablePointersStorageBuffer,
    VariablePointers,
    AtomicStorageOps,
    SampleMaskPostDepthCoverage,
    StorageBuffer8BitAccess,
    UniformAndStorageBuffer8BitAccess,
    StoragePushConstant8,
    DenormPreserve,
    DenormFlushToZero,
    SignedZeroInfNanPreserve,
    RoundingModeRTE,
    RoundingModeRTZ,
    RayQueryKHR,
    RayTracingKHR,
    Float16ImageAMD,
    ImageGatherBiasLodAMD,
    FragmentMaskAMD,
    StencilExportEXT,
    ImageReadWriteLodAMD,
    ShaderClockKHR,
    SampleMaskOverrideCoverageNV,
    GeometryShaderPassthroughNV,
    ShaderViewportIndexLayerEXT,
    ShaderViewportMaskNV,
    ShaderStereoViewNV,
    PerViewAttributesNV,
    FragmentFullyCoveredEXT,
    MeshShadingNV,
    MeshShadingEXT,
    ImageFootprintNV,
    FragmentBarycentricNV,
    ComputeDerivativeGroupQuadsNV,
    FragmentDensityEXT,
    GroupNonUniform,
    GroupNonUniformVote,
    GroupNonUniformArithmetic,
    GroupNonUniformBallot,
    GroupNonUniformShuffle,
    GroupNonUniformShuffleRelative,
    GroupNonUniformClustered,
    GroupNonUniformQuad,
    SubgroupBallotKHR,
    DrawParameters,
    SubgroupVoteKHR,
    StorageInputOutput8,
    SubgroupShuffleINTEL,
    SubgroupBufferBlockIOINTEL,
    SubgroupImageBlockIOINTEL,
    SubgroupImageMediaBlockIOINTEL,
    IntegerFunctions2INTEL,
    RayTraversalPrimitiveCullingKHR,
}

impl Capability {
    fn to_spirv(&self) -> u32 {
        match self {
            Capability::Matrix => 0,
            Capability::Shader => 1,
            Capability::Geometry => 2,
            Capability::Tessellation => 3,
            Capability::Float16 => 9,
            Capability::Float64 => 10,
            Capability::Int64 => 11,
            Capability::Int16 => 22,
            Capability::Int8 => 39,
            Capability::StorageBuffer16BitAccess => 4433,
            Capability::UniformAndStorageBuffer16BitAccess => 4434,
            Capability::StoragePushConstant16 => 4435,
            Capability::StorageInputOutput16 => 4436,
            Capability::DeviceGroup => 4437,
            Capability::MultiView => 4439,
            Capability::VariablePointersStorageBuffer => 4441,
            Capability::VariablePointers => 4442,
            Capability::AtomicStorageOps => 4445,
            Capability::SampleMaskPostDepthCoverage => 4447,
            Capability::StorageBuffer8BitAccess => 4448,
            Capability::UniformAndStorageBuffer8BitAccess => 4449,
            Capability::StoragePushConstant8 => 4450,
            Capability::DenormPreserve => 4464,
            Capability::DenormFlushToZero => 4465,
            Capability::SignedZeroInfNanPreserve => 4466,
            Capability::RoundingModeRTE => 4467,
            Capability::RoundingModeRTZ => 4468,
            Capability::RayQueryKHR => 4472,
            Capability::RayTracingKHR => 4479,
            Capability::Float16ImageAMD => 5008,
            Capability::ImageGatherBiasLodAMD => 5009,
            Capability::FragmentMaskAMD => 5010,
            Capability::StencilExportEXT => 5013,
            Capability::ImageReadWriteLodAMD => 5015,
            Capability::ShaderClockKHR => 5055,
            Capability::SampleMaskOverrideCoverageNV => 5249,
            Capability::GeometryShaderPassthroughNV => 5251,
            Capability::ShaderViewportIndexLayerEXT => 5254,
            Capability::ShaderViewportMaskNV => 5255,
            Capability::ShaderStereoViewNV => 5259,
            Capability::PerViewAttributesNV => 5260,
            Capability::FragmentFullyCoveredEXT => 5265,
            Capability::MeshShadingNV => 5266,
            Capability::MeshShadingEXT => 5283,
            Capability::ImageFootprintNV => 5282,
            Capability::FragmentBarycentricNV => 5284,
            Capability::ComputeDerivativeGroupQuadsNV => 5288,
            Capability::FragmentDensityEXT => 5291,
            Capability::GroupNonUniform => 61,
            Capability::GroupNonUniformVote => 62,
            Capability::GroupNonUniformArithmetic => 63,
            Capability::GroupNonUniformBallot => 64,
            Capability::GroupNonUniformShuffle => 65,
            Capability::GroupNonUniformShuffleRelative => 66,
            Capability::GroupNonUniformClustered => 67,
            Capability::GroupNonUniformQuad => 68,
            Capability::SubgroupBallotKHR => 4423,
            Capability::DrawParameters => 4427,
            Capability::SubgroupVoteKHR => 4431,
            Capability::StorageInputOutput8 => 4448,
            Capability::SubgroupShuffleINTEL => 5568,
            Capability::SubgroupBufferBlockIOINTEL => 5569,
            Capability::SubgroupImageBlockIOINTEL => 5570,
            Capability::SubgroupImageMediaBlockIOINTEL => 5579,
            Capability::IntegerFunctions2INTEL => 5584,
            Capability::RayTraversalPrimitiveCullingKHR => 4478,
        }
    }
}

/// SPIR-V opcodes.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    Nop                  = 0,
    Undef                = 1,
    Source               = 3,
    SourceExtension      = 4,
    Name                 = 5,
    MemberName           = 6,
    String               = 7,
    Line                 = 8,
    Extension            = 10,
    ExtInstImport        = 11,
    ExtInst              = 12,
    MemoryModel          = 14,
    EntryPoint           = 15,
    ExecutionMode        = 16,
    Capability           = 17,
    TypeVoid             = 19,
    TypeBool             = 20,
    TypeInt              = 21,
    TypeFloat            = 22,
    TypeVector           = 23,
    TypeMatrix           = 24,
    TypeImage            = 25,
    TypeSampler          = 26,
    TypeSampledImage     = 27,
    TypeArray            = 28,
    TypeRuntimeArray     = 29,
    TypeStruct           = 30,
    TypeOpaque           = 31,
    TypePointer          = 32,
    TypeFunction         = 33,
    ConstantTrue         = 41,
    ConstantFalse        = 42,
    Constant             = 43,
    ConstantComposite    = 44,
    ConstantNull         = 46,
    SpecConstantTrue     = 48,
    SpecConstantFalse    = 49,
    SpecConstant         = 50,
    SpecConstantComposite = 51,
    Function             = 54,
    FunctionParameter    = 55,
    FunctionEnd          = 56,
    FunctionCall         = 57,
    Variable             = 59,
    Load                 = 61,
    Store                = 62,
    AccessChain          = 65,
    Decorate             = 71,
    MemberDecorate       = 72,
    VectorShuffle        = 79,
    CompositeConstruct   = 80,
    CompositeExtract     = 81,
    CompositeInsert      = 82,
    Transpose            = 84,
    ConvertFToU          = 109,
    ConvertFToS          = 110,
    ConvertSToF          = 111,
    ConvertUToF          = 112,
    FConvert             = 115,
    SConvert             = 114,
    UConvert             = 113,
    Bitcast              = 124,
    SNegate              = 126,
    FNegate              = 127,
    IAdd                 = 128,
    FAdd                 = 129,
    ISub                 = 130,
    FSub                 = 131,
    IMul                 = 132,
    FMul                 = 133,
    UDiv                 = 134,
    SDiv                 = 135,
    FDiv                 = 136,
    UMod                 = 137,
    SRem                 = 138,
    SMod                 = 139,
    FRem                 = 140,
    FMod                 = 141,
    VectorTimesScalar    = 142,
    MatrixTimesScalar    = 143,
    VectorTimesMatrix    = 144,
    MatrixTimesVector    = 145,
    MatrixTimesMatrix    = 146,
    Dot                  = 148,
    ShiftRightLogical    = 194,
    ShiftRightArithmetic = 195,
    ShiftLeftLogical     = 196,
    BitwiseOr            = 197,
    BitwiseXor           = 198,
    BitwiseAnd           = 199,
    Not                  = 200,
    LogicalEqual         = 164,
    LogicalNotEqual      = 165,
    LogicalOr            = 166,
    LogicalAnd           = 167,
    LogicalNot           = 168,
    Select               = 169,
    IEqual               = 170,
    INotEqual            = 171,
    UGreaterThan         = 172,
    SGreaterThan         = 173,
    UGreaterThanEqual    = 174,
    SGreaterThanEqual    = 175,
    ULessThan            = 176,
    SLessThan            = 177,
    ULessThanEqual       = 178,
    SLessThanEqual       = 179,
    FOrdEqual            = 180,
    FUnordEqual          = 181,
    FOrdNotEqual         = 182,
    FUnordNotEqual       = 183,
    FOrdLessThan         = 184,
    FUnordLessThan       = 185,
    FOrdGreaterThan      = 186,
    FUnordGreaterThan    = 187,
    FOrdLessThanEqual    = 188,
    FUnordLessThanEqual  = 189,
    FOrdGreaterThanEqual = 190,
    FUnordGreaterThanEqual = 191,
    DPdx                 = 207,
    DPdy                 = 208,
    Fwidth               = 209,
    DPdxFine             = 210,
    DPdyFine             = 211,
    FwidthFine           = 212,
    DPdxCoarse           = 213,
    DPdyCoarse           = 214,
    FwidthCoarse         = 215,
    Kill                 = 252,
    Return               = 253,
    ReturnValue          = 254,
    Unreachable          = 255,
    Label                = 248,
    Branch               = 249,
    BranchConditional    = 250,
    Switch               = 251,
    Phi                  = 245,
    LoopMerge            = 246,
    SelectionMerge       = 247,
    ControlBarrier       = 224,
    MemoryBarrier        = 225,
    AtomicLoad           = 227,
    AtomicStore          = 228,
    AtomicExchange       = 229,
    AtomicCompareExchange = 230,
    AtomicIIncrement     = 232,
    AtomicIDecrement     = 233,
    AtomicIAdd           = 234,
    AtomicISub           = 235,
    AtomicSMin           = 236,
    AtomicUMin           = 237,
    AtomicSMax           = 238,
    AtomicUMax           = 239,
    AtomicAnd            = 240,
    AtomicOr             = 241,
    AtomicXor            = 242,
    ImageSampleImplicitLod = 87,
    ImageSampleExplicitLod = 88,
    ImageSampleDrefImplicitLod = 89,
    ImageSampleDrefExplicitLod = 90,
    ImageFetch           = 95,
    ImageGather          = 96,
    ImageDrefGather      = 97,
    ImageRead            = 98,
    ImageWrite           = 99,
    ImageQuerySizeLod    = 103,
    ImageQuerySize       = 104,
    ImageQueryLod        = 105,
    ImageQueryLevels     = 106,
    ImageQuerySamples    = 107,
    TypeAccelerationStructureKHR = 5341,
    TypeRayQueryKHR      = 4472,
    TraceRayKHR          = 4445,
    ReportIntersectionKHR = 5334,
    IgnoreIntersectionKHR = 4448,
    TerminateRayKHR      = 4449,
    ExecuteCallableKHR   = 4446,
    RayQueryInitializeKHR = 4473,
    RayQueryProceedKHR   = 4477,
    SetMeshOutputsEXT    = 5295,
    EmitMeshTasksEXT     = 5294,
}

/// SPIR-V decorations.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum Decoration {
    RelaxedPrecision     = 0,
    SpecId               = 1,
    Block                = 2,
    BufferBlock          = 3,
    RowMajor             = 4,
    ColMajor             = 5,
    ArrayStride          = 6,
    MatrixStride         = 7,
    GLSLShared           = 8,
    GLSLPacked           = 9,
    CPacked              = 10,
    BuiltIn              = 11,
    NoPerspective        = 13,
    Flat                 = 14,
    Patch                = 15,
    Centroid             = 16,
    Sample               = 17,
    Invariant            = 18,
    Restrict             = 19,
    Aliased              = 20,
    Volatile             = 21,
    Constant             = 22,
    Coherent             = 23,
    NonWritable          = 24,
    NonReadable          = 25,
    Uniform              = 26,
    UniformId            = 27,
    SaturatedConversion  = 28,
    Stream               = 29,
    Location             = 30,
    Component            = 31,
    Index                = 32,
    Binding              = 33,
    DescriptorSet        = 34,
    Offset               = 35,
    XfbBuffer            = 36,
    XfbStride            = 37,
    FuncParamAttr        = 38,
    FPRoundingMode       = 39,
    FPFastMathMode       = 40,
    LinkageAttributes    = 41,
    NoContraction        = 42,
    InputAttachmentIndex = 43,
    Alignment            = 44,
    MaxByteOffset        = 45,
}

impl SpirVGenerator {
    /// Create a new SPIR-V generator.
    pub fn new() -> Self {
        Self {
            words: Vec::new(),
            bound: 1,
            id_map: HashMap::new(),
            type_ids: HashMap::new(),
            capabilities: Vec::new(),
            extensions: Vec::new(),
            glsl_ext_id: None,
        }
    }

    /// Generate SPIR-V from IR module.
    pub fn generate(&mut self, module: &IrModule) -> Result<Vec<u32>> {
        // Determine required capabilities
        self.analyze_capabilities(module);

        // Allocate IDs
        self.allocate_ids(module);

        // Generate header (will be filled in at the end)
        let header_start = self.words.len();
        self.words.extend([0u32; 5]);

        // Capabilities
        for cap in &self.capabilities.clone() {
            self.emit_capability(*cap);
        }

        // Extensions
        for ext in &self.extensions.clone() {
            self.emit_extension(ext);
        }

        // Import GLSL.std.450
        let glsl_id = self.bound;
        self.bound += 1;
        self.glsl_ext_id = Some(glsl_id);
        self.emit_ext_inst_import(glsl_id, "GLSL.std.450");

        // Memory model
        self.emit_instruction(Opcode::MemoryModel, &[0, 1]); // Logical, GLSL450

        // Entry points
        for entry in &module.entry_points {
            self.emit_entry_point(entry);
        }

        // Execution modes
        for entry in &module.entry_points {
            self.emit_execution_modes(entry);
        }

        // Debug names
        for ty in &module.types {
            if let IrTypeKind::Struct { name, .. } = &ty.kind {
                self.emit_name(ty.id, name);
            }
        }
        for global in &module.globals {
            self.emit_name(global.id, &global.name);
        }
        for func in &module.functions {
            self.emit_name(func.id, &func.name);
        }

        // Decorations
        for global in &module.globals {
            self.emit_decorations(global.id, &global.decorations);
        }

        // Types
        for ty in &module.types {
            self.emit_type(ty);
        }

        // Global variables
        for global in &module.globals {
            self.emit_global(global);
        }

        // Functions
        for func in &module.functions {
            self.emit_function(func, module)?;
        }

        // Fill in header
        let word_count = self.words.len() as u32;
        self.words[header_start] = 0x07230203; // SPIR-V magic
        self.words[header_start + 1] = 0x00010500; // Version 1.5
        self.words[header_start + 2] = 0x4C554D49; // "LUMI" generator magic
        self.words[header_start + 3] = self.bound;
        self.words[header_start + 4] = 0; // Schema

        Ok(std::mem::take(&mut self.words))
    }

    fn analyze_capabilities(&mut self, module: &IrModule) {
        // Always need Shader capability
        self.capabilities.push(Capability::Shader);

        // Check execution models
        for entry in &module.entry_points {
            match entry.execution_model {
                IrExecutionModel::Geometry => {
                    self.capabilities.push(Capability::Geometry);
                },
                IrExecutionModel::TessellationControl
                | IrExecutionModel::TessellationEvaluation => {
                    self.capabilities.push(Capability::Tessellation);
                },
                IrExecutionModel::MeshNV | IrExecutionModel::TaskNV => {
                    self.capabilities.push(Capability::MeshShadingNV);
                },
                IrExecutionModel::MeshEXT | IrExecutionModel::TaskEXT => {
                    self.capabilities.push(Capability::MeshShadingEXT);
                    self.extensions.push("SPV_EXT_mesh_shader".to_string());
                },
                IrExecutionModel::RayGenerationKHR
                | IrExecutionModel::ClosestHitKHR
                | IrExecutionModel::AnyHitKHR
                | IrExecutionModel::MissKHR
                | IrExecutionModel::IntersectionKHR
                | IrExecutionModel::CallableKHR => {
                    self.capabilities.push(Capability::RayTracingKHR);
                    self.extensions.push("SPV_KHR_ray_tracing".to_string());
                },
                _ => {},
            }
        }

        // Check types for Float64, Int64, etc.
        for ty in &module.types {
            match &ty.kind {
                IrTypeKind::Float { width: 64 } => {
                    self.capabilities.push(Capability::Float64);
                },
                IrTypeKind::Int { width: 64, .. } => {
                    self.capabilities.push(Capability::Int64);
                },
                IrTypeKind::Int { width: 16, .. } => {
                    self.capabilities.push(Capability::Int16);
                },
                IrTypeKind::Int { width: 8, .. } => {
                    self.capabilities.push(Capability::Int8);
                },
                IrTypeKind::AccelerationStructure => {
                    if !self.capabilities.contains(&Capability::RayTracingKHR) {
                        self.capabilities.push(Capability::RayQueryKHR);
                    }
                },
                IrTypeKind::RayQuery => {
                    self.capabilities.push(Capability::RayQueryKHR);
                    self.extensions.push("SPV_KHR_ray_query".to_string());
                },
                _ => {},
            }
        }

        // Deduplicate
        self.capabilities.sort_by_key(|c| c.to_spirv());
        self.capabilities.dedup();
        self.extensions.sort();
        self.extensions.dedup();
    }

    fn allocate_ids(&mut self, module: &IrModule) {
        for ty in &module.types {
            self.id_map.insert(ty.id, ty.id);
            self.bound = self.bound.max(ty.id + 1);
        }
        for global in &module.globals {
            self.id_map.insert(global.id, global.id);
            self.bound = self.bound.max(global.id + 1);
        }
        for func in &module.functions {
            self.id_map.insert(func.id, func.id);
            self.bound = self.bound.max(func.id + 1);
            for param in &func.params {
                self.id_map.insert(param.id, param.id);
                self.bound = self.bound.max(param.id + 1);
            }
            for block in &func.blocks {
                self.id_map.insert(block.id, block.id);
                self.bound = self.bound.max(block.id + 1);
            }
        }
    }

    fn emit_instruction(&mut self, opcode: Opcode, operands: &[u32]) {
        let word_count = (1 + operands.len()) as u32;
        self.words.push((word_count << 16) | (opcode as u32));
        self.words.extend_from_slice(operands);
    }

    fn emit_instruction_with_result(
        &mut self,
        opcode: Opcode,
        result_type: u32,
        result: u32,
        operands: &[u32],
    ) {
        let word_count = (3 + operands.len()) as u32;
        self.words.push((word_count << 16) | (opcode as u32));
        self.words.push(result_type);
        self.words.push(result);
        self.words.extend_from_slice(operands);
    }

    fn emit_capability(&mut self, cap: Capability) {
        self.emit_instruction(Opcode::Capability, &[cap.to_spirv()]);
    }

    fn emit_extension(&mut self, ext: &str) {
        let mut operands = Vec::new();
        operands.extend(Self::encode_string(ext));
        let word_count = (1 + operands.len()) as u32;
        self.words
            .push((word_count << 16) | (Opcode::Extension as u32));
        self.words.extend(operands);
    }

    fn emit_ext_inst_import(&mut self, id: u32, name: &str) {
        let mut operands = vec![id];
        operands.extend(Self::encode_string(name));
        let word_count = (1 + operands.len()) as u32;
        self.words
            .push((word_count << 16) | (Opcode::ExtInstImport as u32));
        self.words.extend(operands);
    }

    fn emit_entry_point(&mut self, entry: &IrEntryPoint) {
        let exec_model = match entry.execution_model {
            IrExecutionModel::Vertex => 0,
            IrExecutionModel::TessellationControl => 1,
            IrExecutionModel::TessellationEvaluation => 2,
            IrExecutionModel::Geometry => 3,
            IrExecutionModel::Fragment => 4,
            IrExecutionModel::GLCompute => 5,
            IrExecutionModel::Kernel => 6,
            IrExecutionModel::TaskNV => 5267,
            IrExecutionModel::MeshNV => 5268,
            IrExecutionModel::TaskEXT => 5364,
            IrExecutionModel::MeshEXT => 5365,
            IrExecutionModel::RayGenerationKHR => 5313,
            IrExecutionModel::IntersectionKHR => 5314,
            IrExecutionModel::AnyHitKHR => 5315,
            IrExecutionModel::ClosestHitKHR => 5316,
            IrExecutionModel::MissKHR => 5317,
            IrExecutionModel::CallableKHR => 5318,
        };

        let mut operands = vec![exec_model, entry.function];
        operands.extend(Self::encode_string(&entry.name));
        operands.extend(&entry.interface);

        let word_count = (1 + operands.len()) as u32;
        self.words
            .push((word_count << 16) | (Opcode::EntryPoint as u32));
        self.words.extend(operands);
    }

    fn emit_execution_modes(&mut self, entry: &IrEntryPoint) {
        for mode in &entry.execution_modes {
            self.emit_execution_mode(entry.function, mode);
        }
    }

    fn emit_execution_mode(&mut self, func: u32, mode: &IrExecutionMode) {
        let operands: Vec<u32> = match mode {
            IrExecutionMode::LocalSize(x, y, z) => vec![func, 17, *x, *y, *z],
            IrExecutionMode::OriginUpperLeft => vec![func, 7],
            IrExecutionMode::OriginLowerLeft => vec![func, 8],
            IrExecutionMode::EarlyFragmentTests => vec![func, 9],
            IrExecutionMode::DepthReplacing => vec![func, 12],
            IrExecutionMode::DepthGreater => vec![func, 14],
            IrExecutionMode::DepthLess => vec![func, 15],
            IrExecutionMode::DepthUnchanged => vec![func, 16],
            IrExecutionMode::Invocations(n) => vec![func, 0, *n],
            IrExecutionMode::InputPoints => vec![func, 19],
            IrExecutionMode::InputLines => vec![func, 20],
            IrExecutionMode::InputLinesAdjacency => vec![func, 21],
            IrExecutionMode::InputTriangles => vec![func, 22],
            IrExecutionMode::InputTrianglesAdjacency => vec![func, 23],
            IrExecutionMode::Quads => vec![func, 24],
            IrExecutionMode::Isolines => vec![func, 25],
            IrExecutionMode::OutputVertices(n) => vec![func, 26, *n],
            IrExecutionMode::OutputPoints => vec![func, 27],
            IrExecutionMode::OutputLineStrip => vec![func, 28],
            IrExecutionMode::OutputTriangleStrip => vec![func, 29],
            IrExecutionMode::SpacingEqual => vec![func, 1],
            IrExecutionMode::SpacingFractionalEven => vec![func, 2],
            IrExecutionMode::SpacingFractionalOdd => vec![func, 3],
            IrExecutionMode::VertexOrderCw => vec![func, 4],
            IrExecutionMode::VertexOrderCcw => vec![func, 5],
            IrExecutionMode::PointMode => vec![func, 10],
            IrExecutionMode::OutputPrimitivesNV(n) => vec![func, 5270, *n],
            _ => return,
        };

        let word_count = (1 + operands.len()) as u32;
        self.words
            .push((word_count << 16) | (Opcode::ExecutionMode as u32));
        self.words.extend(operands);
    }

    fn emit_name(&mut self, id: u32, name: &str) {
        let mut operands = vec![id];
        operands.extend(Self::encode_string(name));
        let word_count = (1 + operands.len()) as u32;
        self.words.push((word_count << 16) | (Opcode::Name as u32));
        self.words.extend(operands);
    }

    fn emit_decorations(&mut self, id: u32, decorations: &[IrDecoration]) {
        for dec in decorations {
            self.emit_decoration(id, dec);
        }
    }

    fn emit_decoration(&mut self, id: u32, decoration: &IrDecoration) {
        let operands: Vec<u32> = match decoration {
            IrDecoration::Location(loc) => vec![id, Decoration::Location as u32, *loc],
            IrDecoration::Binding(binding) => vec![id, Decoration::Binding as u32, *binding],
            IrDecoration::DescriptorSet(set) => vec![id, Decoration::DescriptorSet as u32, *set],
            IrDecoration::BuiltIn(builtin) => vec![id, Decoration::BuiltIn as u32, *builtin],
            IrDecoration::Flat => vec![id, Decoration::Flat as u32],
            IrDecoration::NoPerspective => vec![id, Decoration::NoPerspective as u32],
            IrDecoration::Centroid => vec![id, Decoration::Centroid as u32],
            IrDecoration::Sample => vec![id, Decoration::Sample as u32],
            IrDecoration::Block => vec![id, Decoration::Block as u32],
            IrDecoration::BufferBlock => vec![id, Decoration::BufferBlock as u32],
            IrDecoration::RowMajor => vec![id, Decoration::RowMajor as u32],
            IrDecoration::ColMajor => vec![id, Decoration::ColMajor as u32],
            IrDecoration::ArrayStride(stride) => {
                vec![id, Decoration::ArrayStride as u32, *stride]
            },
            IrDecoration::MatrixStride(stride) => {
                vec![id, Decoration::MatrixStride as u32, *stride]
            },
            IrDecoration::Offset(offset) => vec![id, Decoration::Offset as u32, *offset],
            IrDecoration::NonWritable => vec![id, Decoration::NonWritable as u32],
            IrDecoration::NonReadable => vec![id, Decoration::NonReadable as u32],
            IrDecoration::Restrict => vec![id, Decoration::Restrict as u32],
            IrDecoration::Aliased => vec![id, Decoration::Aliased as u32],
            IrDecoration::Volatile => vec![id, Decoration::Volatile as u32],
            IrDecoration::Coherent => vec![id, Decoration::Coherent as u32],
            IrDecoration::SpecId(spec_id) => vec![id, Decoration::SpecId as u32, *spec_id],
        };

        let word_count = (1 + operands.len()) as u32;
        self.words
            .push((word_count << 16) | (Opcode::Decorate as u32));
        self.words.extend(operands);
    }

    fn emit_type(&mut self, ty: &IrType) {
        match &ty.kind {
            IrTypeKind::Void => {
                self.emit_instruction(Opcode::TypeVoid, &[ty.id]);
            },
            IrTypeKind::Bool => {
                self.emit_instruction(Opcode::TypeBool, &[ty.id]);
            },
            IrTypeKind::Int { width, signed } => {
                let signedness = if *signed { 1 } else { 0 };
                self.emit_instruction(Opcode::TypeInt, &[ty.id, *width, signedness]);
            },
            IrTypeKind::Float { width } => {
                self.emit_instruction(Opcode::TypeFloat, &[ty.id, *width]);
            },
            IrTypeKind::Vector { element, size } => {
                self.emit_instruction(Opcode::TypeVector, &[ty.id, *element, *size]);
            },
            IrTypeKind::Matrix {
                element,
                cols,
                rows,
            } => {
                // Matrix is array of column vectors
                // First create vector type if needed, then matrix
                self.emit_instruction(Opcode::TypeMatrix, &[ty.id, *element, *cols]);
            },
            IrTypeKind::Array { element, size } => {
                self.emit_instruction(Opcode::TypeArray, &[ty.id, *element, *size]);
            },
            IrTypeKind::RuntimeArray { element } => {
                self.emit_instruction(Opcode::TypeRuntimeArray, &[ty.id, *element]);
            },
            IrTypeKind::Struct { members, .. } => {
                let mut operands = vec![ty.id];
                operands.extend(members.iter().map(|m| m.ty));
                let word_count = (1 + operands.len()) as u32;
                self.words
                    .push((word_count << 16) | (Opcode::TypeStruct as u32));
                self.words.extend(operands);
            },
            IrTypeKind::Pointer { pointee, storage } => {
                let storage_class = storage_to_spirv(*storage);
                self.emit_instruction(Opcode::TypePointer, &[ty.id, storage_class, *pointee]);
            },
            IrTypeKind::Function {
                return_type,
                params,
            } => {
                let mut operands = vec![ty.id, *return_type];
                operands.extend(params);
                let word_count = (1 + operands.len()) as u32;
                self.words
                    .push((word_count << 16) | (Opcode::TypeFunction as u32));
                self.words.extend(operands);
            },
            IrTypeKind::Image {
                sampled_type,
                dim,
                depth,
                arrayed,
                ms,
            } => {
                let dim_val = match dim {
                    ImageDim::Dim1D => 0,
                    ImageDim::Dim2D => 1,
                    ImageDim::Dim3D => 2,
                    ImageDim::Cube => 3,
                    ImageDim::Rect => 4,
                    ImageDim::Buffer => 5,
                    ImageDim::SubpassData => 6,
                };
                let depth_val = if *depth { 1 } else { 0 };
                let arrayed_val = if *arrayed { 1 } else { 0 };
                let ms_val = if *ms { 1 } else { 0 };
                self.emit_instruction(Opcode::TypeImage, &[
                    ty.id,
                    *sampled_type,
                    dim_val,
                    depth_val,
                    arrayed_val,
                    ms_val,
                    1,
                    0,
                ]);
            },
            IrTypeKind::Sampler => {
                self.emit_instruction(Opcode::TypeSampler, &[ty.id]);
            },
            IrTypeKind::SampledImage { image } => {
                self.emit_instruction(Opcode::TypeSampledImage, &[ty.id, *image]);
            },
            IrTypeKind::AccelerationStructure => {
                self.emit_instruction(Opcode::TypeAccelerationStructureKHR, &[ty.id]);
            },
            IrTypeKind::RayQuery => {
                self.emit_instruction(Opcode::TypeRayQueryKHR, &[ty.id]);
            },
        }
    }

    fn emit_global(&mut self, global: &IrGlobal) {
        let storage = storage_to_spirv(global.storage);
        self.emit_instruction(Opcode::Variable, &[global.ty, global.id, storage]);
    }

    fn emit_function(&mut self, func: &IrFunction, module: &IrModule) -> Result<()> {
        // Create function type
        let func_type_id = self.bound;
        self.bound += 1;

        let param_types: Vec<u32> = func.params.iter().map(|p| p.ty).collect();

        // Emit function type
        {
            let mut operands = vec![func_type_id, func.return_type];
            operands.extend(&param_types);
            let word_count = (1 + operands.len()) as u32;
            self.words
                .push((word_count << 16) | (Opcode::TypeFunction as u32));
            self.words.extend(operands);
        }

        // Function declaration
        // OpFunction <result type> <result id> <function control> <function type>
        let word_count = 5u32;
        self.words
            .push((word_count << 16) | (Opcode::Function as u32));
        self.words.push(func.return_type);
        self.words.push(func.id);
        self.words.push(0); // Function control: None
        self.words.push(func_type_id);

        // Parameters
        for param in &func.params {
            self.emit_instruction(Opcode::FunctionParameter, &[param.ty, param.id]);
        }

        // Blocks
        for block in &func.blocks {
            self.emit_block(block)?;
        }

        // Function end
        self.emit_instruction(Opcode::FunctionEnd, &[]);

        Ok(())
    }

    fn emit_block(&mut self, block: &IrBlock) -> Result<()> {
        // Label
        self.emit_instruction(Opcode::Label, &[block.id]);

        // Instructions
        for inst in &block.instructions {
            self.emit_ir_instruction(inst)?;
        }

        Ok(())
    }

    fn emit_ir_instruction(&mut self, inst: &IrInstruction) -> Result<()> {
        let opcode = match inst.op {
            IrOp::Load => Opcode::Load,
            IrOp::Store => Opcode::Store,
            IrOp::AccessChain => Opcode::AccessChain,
            IrOp::FAdd => Opcode::FAdd,
            IrOp::FSub => Opcode::FSub,
            IrOp::FMul => Opcode::FMul,
            IrOp::FDiv => Opcode::FDiv,
            IrOp::IAdd => Opcode::IAdd,
            IrOp::ISub => Opcode::ISub,
            IrOp::IMul => Opcode::IMul,
            IrOp::Return => Opcode::Return,
            IrOp::ReturnValue => Opcode::ReturnValue,
            IrOp::Branch => Opcode::Branch,
            IrOp::BranchConditional => Opcode::BranchConditional,
            IrOp::Kill => Opcode::Kill,
            // Add more as needed
            _ => return Ok(()), // Skip unhandled instructions
        };

        let operand_words: Vec<u32> = inst
            .operands
            .iter()
            .map(|op| {
                match op {
                    IrOperand::Id(id) => *id,
                    IrOperand::LitInt(v) => *v,
                    IrOperand::LitFloat(v) => v.to_bits(),
                    IrOperand::MemoryAccess(v) => *v,
                    IrOperand::Scope(v) => *v,
                    IrOperand::Semantics(v) => *v,
                    IrOperand::LitString(_) => 0, // Should use encode_string
                }
            })
            .collect();

        if let (Some(result), Some(result_type)) = (inst.result, inst.result_type) {
            self.emit_instruction_with_result(opcode, result_type, result, &operand_words);
        } else {
            self.emit_instruction(opcode, &operand_words);
        }

        Ok(())
    }

    fn encode_string(s: &str) -> Vec<u32> {
        let bytes = s.as_bytes();
        let word_count = (bytes.len() + 4) / 4;
        let mut words = vec![0u32; word_count];

        for (i, &byte) in bytes.iter().enumerate() {
            let word_idx = i / 4;
            let byte_idx = i % 4;
            words[word_idx] |= (byte as u32) << (byte_idx * 8);
        }

        words
    }
}

fn storage_to_spirv(storage: StorageClass) -> u32 {
    match storage {
        StorageClass::UniformConstant => 0,
        StorageClass::Input => 1,
        StorageClass::Uniform => 2,
        StorageClass::Output => 3,
        StorageClass::Workgroup => 4,
        StorageClass::CrossWorkgroup => 5,
        StorageClass::Private => 6,
        StorageClass::Function => 7,
        StorageClass::Generic => 8,
        StorageClass::PushConstant => 9,
        StorageClass::AtomicCounter => 10,
        StorageClass::Image => 11,
        StorageClass::StorageBuffer => 12,
        StorageClass::PhysicalStorageBuffer => 5349,
        StorageClass::RayPayloadKHR => 5338,
        StorageClass::HitAttributeKHR => 5339,
        StorageClass::IncomingRayPayloadKHR => 5342,
        StorageClass::ShaderRecordBufferKHR => 5343,
        StorageClass::CallableDataKHR => 5328,
        StorageClass::IncomingCallableDataKHR => 5329,
        StorageClass::TaskPayloadWorkgroupEXT => 5402,
    }
}

impl Default for SpirVGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_encoding() {
        let words = SpirVGenerator::encode_string("main");
        assert!(!words.is_empty());
        // "main" = 'm', 'a', 'i', 'n', '\0' = 5 bytes = 2 words
        assert_eq!(words.len(), 2);
    }

    #[test]
    fn test_capability_values() {
        assert_eq!(Capability::Shader.to_spirv(), 1);
        assert_eq!(Capability::Geometry.to_spirv(), 2);
        assert_eq!(Capability::Float64.to_spirv(), 10);
    }

    #[test]
    fn test_generator_new() {
        let gen = SpirVGenerator::new();
        assert_eq!(gen.bound, 1);
        assert!(gen.words.is_empty());
    }
}
