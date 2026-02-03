//! SPIR-V Instruction Types
//!
//! Typed instruction representation for SPIR-V.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::opcode::Opcode;

/// SPIR-V ID
pub type Id = u32;

/// SPIR-V instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Opcode
    pub opcode: Opcode,
    /// Result type ID (if any)
    pub result_type: Option<Id>,
    /// Result ID (if any)
    pub result: Option<Id>,
    /// Operands
    pub operands: Vec<Operand>,
}

impl Instruction {
    /// Create a new instruction
    pub fn new(opcode: Opcode) -> Self {
        Self {
            opcode,
            result_type: None,
            result: None,
            operands: Vec::new(),
        }
    }

    /// With result type
    pub fn with_result_type(mut self, id: Id) -> Self {
        self.result_type = Some(id);
        self
    }

    /// With result
    pub fn with_result(mut self, id: Id) -> Self {
        self.result = Some(id);
        self
    }

    /// Add operand
    pub fn with_operand(mut self, op: Operand) -> Self {
        self.operands.push(op);
        self
    }

    /// Add ID operand
    pub fn with_id(mut self, id: Id) -> Self {
        self.operands.push(Operand::Id(id));
        self
    }

    /// Add literal operand
    pub fn with_literal(mut self, value: u32) -> Self {
        self.operands.push(Operand::Literal(value));
        self
    }

    /// Add string operand
    pub fn with_string(mut self, s: String) -> Self {
        self.operands.push(Operand::String(s));
        self
    }

    /// Calculate word count
    pub fn word_count(&self) -> u16 {
        let mut count = 1u16; // opcode word
        if self.result_type.is_some() {
            count += 1;
        }
        if self.result.is_some() {
            count += 1;
        }
        for op in &self.operands {
            count += op.word_count() as u16;
        }
        count
    }

    /// Encode to words
    pub fn encode(&self) -> Vec<u32> {
        let mut words = Vec::new();

        // First word: word count << 16 | opcode
        let first = ((self.word_count() as u32) << 16) | (self.opcode as u32);
        words.push(first);

        // Result type
        if let Some(rt) = self.result_type {
            words.push(rt);
        }

        // Result
        if let Some(r) = self.result {
            words.push(r);
        }

        // Operands
        for op in &self.operands {
            op.encode(&mut words);
        }

        words
    }
}

/// SPIR-V operand
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// ID reference
    Id(Id),
    /// Literal number
    Literal(u32),
    /// Literal 64-bit number
    Literal64(u64),
    /// Literal string
    String(String),
    /// Execution model
    ExecutionModel(ExecutionModel),
    /// Addressing model
    AddressingModel(AddressingModel),
    /// Memory model
    MemoryModel(MemoryModel),
    /// Storage class
    StorageClass(StorageClass),
    /// Decoration
    Decoration(Decoration),
    /// Capability
    Capability(Capability),
    /// Dimension
    Dim(Dim),
    /// Image format
    ImageFormat(ImageFormat),
    /// Function control
    FunctionControl(FunctionControl),
    /// Memory access
    MemoryAccess(MemoryAccessFlags),
    /// Selection control
    SelectionControl(SelectionControlFlags),
    /// Loop control
    LoopControl(LoopControlFlags),
    /// Scope
    Scope(Scope),
    /// Memory semantics
    MemorySemantics(MemorySemanticsFlags),
    /// Group operation
    GroupOperation(GroupOperation),
}

impl Operand {
    /// Word count for this operand
    pub fn word_count(&self) -> usize {
        match self {
            Operand::Id(_) => 1,
            Operand::Literal(_) => 1,
            Operand::Literal64(_) => 2,
            Operand::String(s) => {
                // String is null-terminated, 4 bytes per word
                (s.len() + 1 + 3) / 4
            },
            Operand::ExecutionModel(_) => 1,
            Operand::AddressingModel(_) => 1,
            Operand::MemoryModel(_) => 1,
            Operand::StorageClass(_) => 1,
            Operand::Decoration(_) => 1,
            Operand::Capability(_) => 1,
            Operand::Dim(_) => 1,
            Operand::ImageFormat(_) => 1,
            Operand::FunctionControl(_) => 1,
            Operand::MemoryAccess(_) => 1,
            Operand::SelectionControl(_) => 1,
            Operand::LoopControl(_) => 1,
            Operand::Scope(_) => 1,
            Operand::MemorySemantics(_) => 1,
            Operand::GroupOperation(_) => 1,
        }
    }

    /// Encode to words
    pub fn encode(&self, words: &mut Vec<u32>) {
        match self {
            Operand::Id(id) => words.push(*id),
            Operand::Literal(v) => words.push(*v),
            Operand::Literal64(v) => {
                words.push(*v as u32);
                words.push((*v >> 32) as u32);
            },
            Operand::String(s) => {
                let bytes = s.as_bytes();
                let mut word = 0u32;
                for (i, &b) in bytes.iter().enumerate() {
                    word |= (b as u32) << ((i % 4) * 8);
                    if i % 4 == 3 {
                        words.push(word);
                        word = 0;
                    }
                }
                // Push remaining (with null terminator)
                words.push(word);
            },
            Operand::ExecutionModel(m) => words.push(*m as u32),
            Operand::AddressingModel(m) => words.push(*m as u32),
            Operand::MemoryModel(m) => words.push(*m as u32),
            Operand::StorageClass(c) => words.push(*c as u32),
            Operand::Decoration(d) => words.push(*d as u32),
            Operand::Capability(c) => words.push(*c as u32),
            Operand::Dim(d) => words.push(*d as u32),
            Operand::ImageFormat(f) => words.push(*f as u32),
            Operand::FunctionControl(f) => words.push(f.bits()),
            Operand::MemoryAccess(m) => words.push(m.bits()),
            Operand::SelectionControl(s) => words.push(s.bits()),
            Operand::LoopControl(l) => words.push(l.bits()),
            Operand::Scope(s) => words.push(*s as u32),
            Operand::MemorySemantics(m) => words.push(m.bits()),
            Operand::GroupOperation(g) => words.push(*g as u32),
        }
    }
}

/// Execution model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExecutionModel {
    Vertex              = 0,
    TessellationControl = 1,
    TessellationEvaluation = 2,
    Geometry            = 3,
    Fragment            = 4,
    GLCompute           = 5,
    Kernel              = 6,
    TaskNV              = 5267,
    MeshNV              = 5268,
    RayGenerationKHR    = 5313,
    IntersectionKHR     = 5314,
    AnyHitKHR           = 5315,
    ClosestHitKHR       = 5316,
    MissKHR             = 5317,
    CallableKHR         = 5318,
    TaskEXT             = 5364,
    MeshEXT             = 5365,
}

/// Addressing model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AddressingModel {
    Logical    = 0,
    Physical32 = 1,
    Physical64 = 2,
    PhysicalStorageBuffer64 = 5348,
}

/// Memory model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum MemoryModel {
    Simple  = 0,
    GLSL450 = 1,
    OpenCL  = 2,
    Vulkan  = 3,
}

/// Storage class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum StorageClass {
    UniformConstant = 0,
    Input           = 1,
    Uniform         = 2,
    Output          = 3,
    Workgroup       = 4,
    CrossWorkgroup  = 5,
    Private         = 6,
    Function        = 7,
    Generic         = 8,
    PushConstant    = 9,
    AtomicCounter   = 10,
    Image           = 11,
    StorageBuffer   = 12,
    CallableDataKHR = 5328,
    IncomingCallableDataKHR = 5329,
    RayPayloadKHR   = 5338,
    HitAttributeKHR = 5339,
    IncomingRayPayloadKHR = 5342,
    ShaderRecordBufferKHR = 5343,
    PhysicalStorageBuffer = 5349,
    TaskPayloadWorkgroupEXT = 5402,
}

/// Decoration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
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
    AlignmentId          = 46,
    MaxByteOffsetId      = 47,
    NoSignedWrap         = 4469,
    NoUnsignedWrap       = 4470,
}

/// Capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Capability {
    Matrix               = 0,
    Shader               = 1,
    Geometry             = 2,
    Tessellation         = 3,
    Addresses            = 4,
    Linkage              = 5,
    Kernel               = 6,
    Vector16             = 7,
    Float16Buffer        = 8,
    Float16              = 9,
    Float64              = 10,
    Int64                = 11,
    Int64Atomics         = 12,
    ImageBasic           = 13,
    ImageReadWrite       = 14,
    ImageMipmap          = 15,
    Pipes                = 17,
    Groups               = 18,
    DeviceEnqueue        = 19,
    LiteralSampler       = 20,
    AtomicStorage        = 21,
    Int16                = 22,
    TessellationPointSize = 23,
    GeometryPointSize    = 24,
    ImageGatherExtended  = 25,
    StorageImageMultisample = 27,
    UniformBufferArrayDynamicIndexing = 28,
    SampledImageArrayDynamicIndexing = 29,
    StorageBufferArrayDynamicIndexing = 30,
    StorageImageArrayDynamicIndexing = 31,
    ClipDistance         = 32,
    CullDistance         = 33,
    ImageCubeArray       = 34,
    SampleRateShading    = 35,
    ImageRect            = 36,
    SampledRect          = 37,
    GenericPointer       = 38,
    Int8                 = 39,
    InputAttachment      = 40,
    SparseResidency      = 41,
    MinLod               = 42,
    Sampled1D            = 43,
    Image1D              = 44,
    SampledCubeArray     = 45,
    SampledBuffer        = 46,
    ImageBuffer          = 47,
    ImageMSArray         = 48,
    StorageImageExtendedFormats = 49,
    ImageQuery           = 50,
    DerivativeControl    = 51,
    InterpolationFunction = 52,
    TransformFeedback    = 53,
    GeometryStreams      = 54,
    StorageImageReadWithoutFormat = 55,
    StorageImageWriteWithoutFormat = 56,
    MultiViewport        = 57,
    SubgroupDispatch     = 58,
    NamedBarrier         = 59,
    PipeStorage          = 60,
    GroupNonUniform      = 61,
    GroupNonUniformVote  = 62,
    GroupNonUniformArithmetic = 63,
    GroupNonUniformBallot = 64,
    GroupNonUniformShuffle = 65,
    GroupNonUniformShuffleRelative = 66,
    GroupNonUniformClustered = 67,
    GroupNonUniformQuad  = 68,
    ShaderLayer          = 69,
    ShaderViewportIndex  = 70,
    FragmentShadingRateKHR = 4422,
    SubgroupBallotKHR    = 4423,
    DrawParameters       = 4427,
    WorkgroupMemoryExplicitLayoutKHR = 4428,
    WorkgroupMemoryExplicitLayout8BitAccessKHR = 4429,
    WorkgroupMemoryExplicitLayout16BitAccessKHR = 4430,
    SubgroupVoteKHR      = 4431,
    StorageBuffer16BitAccess = 4433,
    UniformAndStorageBuffer16BitAccess = 4434,
    StoragePushConstant16 = 4435,
    StorageInputOutput16 = 4436,
    DeviceGroup          = 4437,
    MultiView            = 4439,
    VariablePointersStorageBuffer = 4441,
    VariablePointers     = 4442,
    AtomicStorageOps     = 4445,
    SampleMaskPostDepthCoverage = 4447,
    StorageBuffer8BitAccess = 4448,
    UniformAndStorageBuffer8BitAccess = 4449,
    StoragePushConstant8 = 4450,
    DenormPreserve       = 4464,
    DenormFlushToZero    = 4465,
    SignedZeroInfNanPreserve = 4466,
    RoundingModeRTE      = 4467,
    RoundingModeRTZ      = 4468,
    RayQueryProvisionalKHR = 4471,
    RayQueryKHR          = 4472,
    RayTraversalPrimitiveCullingKHR = 4478,
    RayTracingKHR        = 4479,
    Float16ImageAMD      = 5008,
    ImageGatherBiasLodAMD = 5009,
    FragmentMaskAMD      = 5010,
    StencilExportEXT     = 5013,
    ImageReadWriteLodAMD = 5015,
    Int64ImageEXT        = 5016,
    ShaderClockKHR       = 5055,
    SampleMaskOverrideCoverageNV = 5249,
    GeometryShaderPassthroughNV = 5251,
    ShaderViewportIndexLayerEXT = 5254,
    ShaderViewportMaskNV = 5255,
    ShaderStereoViewNV   = 5259,
    PerViewAttributesNV  = 5260,
    FragmentFullyCoveredEXT = 5265,
    MeshShadingNV        = 5266,
    ImageFootprintNV     = 5282,
    MeshShadingEXT       = 5283,
    FragmentBarycentricKHR = 5284,
    ComputeDerivativeGroupQuadsNV = 5288,
    FragmentDensityEXT   = 5291,
    GroupNonUniformPartitionedNV = 5297,
    ShaderNonUniform     = 5301,
    RuntimeDescriptorArray = 5302,
    InputAttachmentArrayDynamicIndexing = 5303,
    UniformTexelBufferArrayDynamicIndexing = 5304,
    StorageTexelBufferArrayDynamicIndexing = 5305,
    UniformBufferArrayNonUniformIndexing = 5306,
    SampledImageArrayNonUniformIndexing = 5307,
    StorageBufferArrayNonUniformIndexing = 5308,
    StorageImageArrayNonUniformIndexing = 5309,
    InputAttachmentArrayNonUniformIndexing = 5310,
    UniformTexelBufferArrayNonUniformIndexing = 5311,
    StorageTexelBufferArrayNonUniformIndexing = 5312,
    RayTracingPositionFetchKHR = 5336,
    RayTracingNV         = 5340,
    RayTracingMotionBlurNV = 5341,
    VulkanMemoryModel    = 5345,
    VulkanMemoryModelDeviceScope = 5346,
    PhysicalStorageBufferAddresses = 5347,
    ComputeDerivativeGroupLinearNV = 5350,
    RayTracingProvisionalKHR = 5353,
    CooperativeMatrixNV  = 5357,
    FragmentShaderSampleInterlockEXT = 5363,
    FragmentShaderShadingRateInterlockEXT = 5372,
    ShaderSMBuiltinsNV   = 5373,
    FragmentShaderPixelInterlockEXT = 5378,
    DemoteToHelperInvocation = 5379,
    RayTracingOpacityMicromapEXT = 5381,
    ShaderInvocationReorderNV = 5383,
    BindlessTextureNV    = 5390,
    RayQueryPositionFetchKHR = 5391,
    AtomicFloat16VectorNV = 5404,
    SubgroupShuffleINTEL = 5568,
    SubgroupBufferBlockIOINTEL = 5569,
    SubgroupImageBlockIOINTEL = 5570,
    SubgroupImageMediaBlockIOINTEL = 5579,
    RoundToInfinityINTEL = 5582,
    FloatingPointModeINTEL = 5583,
    IntegerFunctions2INTEL = 5584,
    FunctionPointersINTEL = 5603,
    IndirectReferencesINTEL = 5604,
    AsmINTEL             = 5606,
    AtomicFloat32MinMaxEXT = 5612,
    AtomicFloat64MinMaxEXT = 5613,
    AtomicFloat16MinMaxEXT = 5616,
    VectorComputeINTEL   = 5617,
    VectorAnyINTEL       = 5619,
    ExpectAssumeKHR      = 5629,
    SubgroupAvcMotionEstimationINTEL = 5696,
    SubgroupAvcMotionEstimationIntraINTEL = 5697,
    SubgroupAvcMotionEstimationChromaINTEL = 5698,
    VariableLengthArrayINTEL = 5817,
    FunctionFloatControlINTEL = 5821,
    FPGAMemoryAttributesINTEL = 5824,
    FPFastMathModeINTEL  = 5837,
    ArbitraryPrecisionIntegersINTEL = 5844,
    ArbitraryPrecisionFloatingPointINTEL = 5845,
    UnstructuredLoopControlsINTEL = 5886,
    FPGALoopControlsINTEL = 5888,
    KernelAttributesINTEL = 5892,
    FPGAKernelAttributesINTEL = 5897,
    FPGAMemoryAccessesINTEL = 5898,
    FPGAClusterAttributesINTEL = 5904,
    LoopFuseINTEL        = 5906,
    MemoryAccessAliasingINTEL = 5910,
    FPGABufferLocationINTEL = 5920,
    ArbitraryPrecisionFixedPointINTEL = 5922,
    USMStorageClassesINTEL = 5935,
    RuntimeAlignedAttributeINTEL = 5939,
    IOPipesINTEL         = 5943,
    BlockingPipesINTEL   = 5945,
    FPGARegINTEL         = 5948,
    DotProductInputAll   = 6016,
    DotProductInput4x8Bit = 6017,
    DotProductInput4x8BitPacked = 6018,
    DotProduct           = 6019,
    RayCullMaskKHR       = 6020,
    CooperativeMatrixKHR = 6022,
    BitInstructions      = 6025,
    GroupNonUniformRotateKHR = 6026,
    AtomicFloat32AddEXT  = 6033,
    AtomicFloat64AddEXT  = 6034,
    LongCompositesINTEL  = 6089,
    OptNoneINTEL         = 6094,
    AtomicFloat16AddEXT  = 6095,
    DebugInfoModuleINTEL = 6114,
    BFloat16ConversionINTEL = 6115,
    SplitBarrierINTEL    = 6141,
    FPGAKernelAttributesv2INTEL = 6161,
    FPGALatencyControlINTEL = 6171,
    FPGAArgumentInterfacesINTEL = 6174,
    GroupUniformArithmeticKHR = 6400,
}

/// Dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Dim {
    Dim1D       = 0,
    Dim2D       = 1,
    Dim3D       = 2,
    Cube        = 3,
    Rect        = 4,
    Buffer      = 5,
    SubpassData = 6,
}

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ImageFormat {
    Unknown      = 0,
    Rgba32f      = 1,
    Rgba16f      = 2,
    R32f         = 3,
    Rgba8        = 4,
    Rgba8Snorm   = 5,
    Rg32f        = 6,
    Rg16f        = 7,
    R11fG11fB10f = 8,
    R16f         = 9,
    Rgba16       = 10,
    Rgb10A2      = 11,
    Rg16         = 12,
    Rg8          = 13,
    R16          = 14,
    R8           = 15,
    Rgba16Snorm  = 16,
    Rg16Snorm    = 17,
    Rg8Snorm     = 18,
    R16Snorm     = 19,
    R8Snorm      = 20,
    Rgba32i      = 21,
    Rgba16i      = 22,
    Rgba8i       = 23,
    R32i         = 24,
    Rg32i        = 25,
    Rg16i        = 26,
    Rg8i         = 27,
    R16i         = 28,
    R8i          = 29,
    Rgba32ui     = 30,
    Rgba16ui     = 31,
    Rgba8ui      = 32,
    R32ui        = 33,
    Rgb10a2ui    = 34,
    Rg32ui       = 35,
    Rg16ui       = 36,
    Rg8ui        = 37,
    R16ui        = 38,
    R8ui         = 39,
    R64ui        = 40,
    R64i         = 41,
}

/// Function control flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionControl(u32);

impl FunctionControl {
    pub const NONE: Self = Self(0);
    pub const INLINE: Self = Self(1);
    pub const DONT_INLINE: Self = Self(2);
    pub const PURE: Self = Self(4);
    pub const CONST: Self = Self(8);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

/// Memory access flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MemoryAccessFlags(u32);

impl MemoryAccessFlags {
    pub const NONE: Self = Self(0);
    pub const VOLATILE: Self = Self(1);
    pub const ALIGNED: Self = Self(2);
    pub const NONTEMPORAL: Self = Self(4);
    pub const MAKE_POINTER_AVAILABLE: Self = Self(8);
    pub const MAKE_POINTER_VISIBLE: Self = Self(16);
    pub const NON_PRIVATE_POINTER: Self = Self(32);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

/// Selection control flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SelectionControlFlags(u32);

impl SelectionControlFlags {
    pub const NONE: Self = Self(0);
    pub const FLATTEN: Self = Self(1);
    pub const DONT_FLATTEN: Self = Self(2);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

/// Loop control flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LoopControlFlags(u32);

impl LoopControlFlags {
    pub const NONE: Self = Self(0);
    pub const UNROLL: Self = Self(1);
    pub const DONT_UNROLL: Self = Self(2);
    pub const DEPENDENCY_INFINITE: Self = Self(4);
    pub const DEPENDENCY_LENGTH: Self = Self(8);
    pub const MIN_ITERATIONS: Self = Self(16);
    pub const MAX_ITERATIONS: Self = Self(32);
    pub const ITERATION_MULTIPLE: Self = Self(64);
    pub const PEEL_COUNT: Self = Self(128);
    pub const PARTIAL_COUNT: Self = Self(256);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

/// Scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Scope {
    CrossDevice   = 0,
    Device        = 1,
    Workgroup     = 2,
    Subgroup      = 3,
    Invocation    = 4,
    QueueFamily   = 5,
    ShaderCallKHR = 6,
}

/// Memory semantics flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MemorySemanticsFlags(u32);

impl MemorySemanticsFlags {
    pub const NONE: Self = Self(0);
    pub const ACQUIRE: Self = Self(2);
    pub const RELEASE: Self = Self(4);
    pub const ACQUIRE_RELEASE: Self = Self(8);
    pub const SEQUENTIALLY_CONSISTENT: Self = Self(16);
    pub const UNIFORM_MEMORY: Self = Self(64);
    pub const SUBGROUP_MEMORY: Self = Self(128);
    pub const WORKGROUP_MEMORY: Self = Self(256);
    pub const CROSS_WORKGROUP_MEMORY: Self = Self(512);
    pub const ATOMIC_COUNTER_MEMORY: Self = Self(1024);
    pub const IMAGE_MEMORY: Self = Self(2048);
    pub const OUTPUT_MEMORY: Self = Self(4096);
    pub const MAKE_AVAILABLE: Self = Self(8192);
    pub const MAKE_VISIBLE: Self = Self(16384);
    pub const VOLATILE: Self = Self(32768);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

/// Group operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum GroupOperation {
    Reduce              = 0,
    InclusiveScan       = 1,
    ExclusiveScan       = 2,
    ClusteredReduce     = 3,
    PartitionedReduceNV = 6,
    PartitionedInclusiveScanNV = 7,
    PartitionedExclusiveScanNV = 8,
}
