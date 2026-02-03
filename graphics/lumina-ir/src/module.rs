//! IR Module
//!
//! This module defines the top-level IR module structure.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::function::{ExecutionModel, Function, FunctionId, FunctionMap, GlobalVariable};
use crate::instruction::Instruction;
use crate::types::{IrType, StructType, TypeRegistry};
use crate::value::{ConstantValue, SpecConstant, SpecConstantMap, ValueId, ValueTable};

/// SPIR-V addressing model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum AddressingModel {
    #[default]
    Logical    = 0,
    Physical32 = 1,
    Physical64 = 2,
    PhysicalStorageBuffer64 = 5348,
}

/// SPIR-V memory model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum MemoryModel {
    Simple  = 0,
    GLSL450 = 1,
    OpenCL  = 2,
    #[default]
    Vulkan  = 3,
}

/// Capability required by the module
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
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
    BindlessTextureNV    = 5390,
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
    FPGABufferLocationINTEL = 5920,
    ArbitraryPrecisionFixedPointINTEL = 5922,
    USMStorageClassesINTEL = 5935,
    IOPipesINTEL         = 5943,
    BlockingPipesINTEL   = 5945,
    FPGARegINTEL         = 5948,
    DotProductInputAll   = 6016,
    DotProductInput4x8Bit = 6017,
    DotProductInput4x8BitPacked = 6018,
    DotProduct           = 6019,
    BitInstructions      = 6025,
    AtomicFloat32AddEXT  = 6033,
    AtomicFloat64AddEXT  = 6034,
    LongConstantCompositeINTEL = 6089,
    OptNoneINTEL         = 6094,
    AtomicFloat16AddEXT  = 6095,
    DebugInfoModuleINTEL = 6114,
    MeshShadingEXT       = 5283,
    RayTracingOpacityMicromapEXT = 5381,
    CooperativeMatrixKHR = 6022,
}

impl Capability {
    /// Get capabilities implied by this capability
    pub fn implies(&self) -> &'static [Capability] {
        match self {
            Self::Shader => &[Self::Matrix],
            Self::Geometry => &[Self::Shader],
            Self::Tessellation => &[Self::Shader],
            Self::Float16 => &[],
            Self::Float64 => &[],
            Self::Int64 => &[],
            Self::Int16 => &[],
            Self::Int8 => &[],
            Self::StorageBuffer16BitAccess => &[],
            Self::StorageBuffer8BitAccess => &[],
            Self::GroupNonUniform => &[],
            Self::GroupNonUniformVote => &[Self::GroupNonUniform],
            Self::GroupNonUniformBallot => &[Self::GroupNonUniform],
            Self::GroupNonUniformShuffle => &[Self::GroupNonUniform],
            Self::GroupNonUniformArithmetic => &[Self::GroupNonUniform],
            Self::RayTracingKHR => &[Self::Shader],
            Self::RayQueryKHR => &[Self::Shader],
            Self::MeshShadingEXT => &[Self::Shader],
            _ => &[],
        }
    }
}

/// Extension required by the module
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Extension(pub String);

impl Extension {
    pub const SPV_KHR_VULKAN_MEMORY_MODEL: &'static str = "SPV_KHR_vulkan_memory_model";
    pub const SPV_KHR_PHYSICAL_STORAGE_BUFFER: &'static str = "SPV_KHR_physical_storage_buffer";
    pub const SPV_KHR_16BIT_STORAGE: &'static str = "SPV_KHR_16bit_storage";
    pub const SPV_KHR_8BIT_STORAGE: &'static str = "SPV_KHR_8bit_storage";
    pub const SPV_KHR_VARIABLE_POINTERS: &'static str = "SPV_KHR_variable_pointers";
    pub const SPV_KHR_SHADER_DRAW_PARAMETERS: &'static str = "SPV_KHR_shader_draw_parameters";
    pub const SPV_KHR_STORAGE_BUFFER_STORAGE_CLASS: &'static str =
        "SPV_KHR_storage_buffer_storage_class";
    pub const SPV_KHR_RAY_TRACING: &'static str = "SPV_KHR_ray_tracing";
    pub const SPV_KHR_RAY_QUERY: &'static str = "SPV_KHR_ray_query";
    pub const SPV_EXT_MESH_SHADER: &'static str = "SPV_EXT_mesh_shader";
    pub const SPV_EXT_DESCRIPTOR_INDEXING: &'static str = "SPV_EXT_descriptor_indexing";
    pub const SPV_EXT_FRAGMENT_FULLY_COVERED: &'static str = "SPV_EXT_fragment_fully_covered";
    pub const SPV_EXT_SHADER_STENCIL_EXPORT: &'static str = "SPV_EXT_shader_stencil_export";
    pub const SPV_EXT_DEMOTE_TO_HELPER_INVOCATION: &'static str =
        "SPV_EXT_demote_to_helper_invocation";
    pub const SPV_KHR_SHADER_CLOCK: &'static str = "SPV_KHR_shader_clock";
    pub const SPV_KHR_FRAGMENT_SHADING_RATE: &'static str = "SPV_KHR_fragment_shading_rate";
    pub const SPV_KHR_WORKGROUP_MEMORY_EXPLICIT_LAYOUT: &'static str =
        "SPV_KHR_workgroup_memory_explicit_layout";
    pub const SPV_EXT_SHADER_ATOMIC_FLOAT_ADD: &'static str = "SPV_EXT_shader_atomic_float_add";
    pub const SPV_EXT_SHADER_ATOMIC_FLOAT_MIN_MAX: &'static str =
        "SPV_EXT_shader_atomic_float_min_max";
    pub const SPV_KHR_EXPECT_ASSUME: &'static str = "SPV_KHR_expect_assume";
    pub const SPV_KHR_COOPERATIVE_MATRIX: &'static str = "SPV_KHR_cooperative_matrix";
}

/// Imported instruction set
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtInstImport {
    pub id: u32,
    pub name: String,
}

impl ExtInstImport {
    pub const GLSL_STD_450: &'static str = "GLSL.std.450";
    pub const OPENCL_STD: &'static str = "OpenCL.std";
    pub const SPV_AMD_SHADER_TRINARY_MINMAX: &'static str = "SPV_AMD_shader_trinary_minmax";
    pub const NON_SEMANTIC_DEBUG_PRINTF: &'static str = "NonSemantic.DebugPrintf";
    pub const NON_SEMANTIC_SHADER_DEBUG_INFO: &'static str = "NonSemantic.Shader.DebugInfo.100";
}

/// Source language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SourceLanguage {
    #[default]
    Unknown        = 0,
    ESSL           = 1,
    GLSL           = 2,
    OpenCL_C       = 3,
    OpenCL_CPP     = 4,
    HLSL           = 5,
    CPP_for_OpenCL = 6,
    SYCL           = 7,
}

/// Debug source info
#[derive(Debug, Clone, Default)]
pub struct SourceInfo {
    pub language: SourceLanguage,
    pub version: u32,
    pub file: Option<String>,
    pub source: Option<String>,
}

/// The top-level IR module
#[derive(Debug)]
pub struct Module {
    /// Module name
    pub name: String,
    /// SPIR-V version (major, minor)
    pub version: (u8, u8),
    /// Addressing model
    pub addressing_model: AddressingModel,
    /// Memory model
    pub memory_model: MemoryModel,
    /// Required capabilities
    pub capabilities: Vec<Capability>,
    /// Required extensions
    pub extensions: Vec<Extension>,
    /// Extended instruction sets
    pub ext_inst_imports: Vec<ExtInstImport>,
    /// Source info
    pub source: SourceInfo,
    /// Type registry
    pub types: TypeRegistry,
    /// Value table
    pub values: ValueTable,
    /// Specialization constants
    pub spec_constants: SpecConstantMap,
    /// Global variables
    pub globals: Vec<GlobalVariable>,
    /// Functions
    pub functions: FunctionMap,
    /// Debug names
    pub debug_names: Vec<(ValueId, String)>,
    /// Debug member names
    pub debug_member_names: Vec<(ValueId, u32, String)>,
}

impl Default for Module {
    fn default() -> Self {
        Self::new("main")
    }
}

impl Module {
    /// Create a new module
    pub fn new(name: impl Into<String>) -> Self {
        let mut module = Self {
            name: name.into(),
            version: (1, 6),
            addressing_model: AddressingModel::Logical,
            memory_model: MemoryModel::Vulkan,
            capabilities: Vec::new(),
            extensions: Vec::new(),
            ext_inst_imports: Vec::new(),
            source: SourceInfo::default(),
            types: TypeRegistry::new(),
            values: ValueTable::new(),
            spec_constants: SpecConstantMap::new(),
            globals: Vec::new(),
            functions: FunctionMap::new(),
            debug_names: Vec::new(),
            debug_member_names: Vec::new(),
        };

        // Add default capabilities
        module.add_capability(Capability::Shader);

        module
    }

    /// Create a module for Vulkan shaders
    pub fn vulkan_shader(name: impl Into<String>) -> Self {
        let mut module = Self::new(name);
        module.memory_model = MemoryModel::Vulkan;
        module.add_capability(Capability::VulkanMemoryModel);
        module
    }

    /// Create a compute module
    pub fn compute(name: impl Into<String>) -> Self {
        let mut module = Self::vulkan_shader(name);
        module.add_capability(Capability::Shader);
        module
    }

    /// Set SPIR-V version
    pub fn set_version(&mut self, major: u8, minor: u8) {
        self.version = (major, minor);
    }

    /// Add a capability
    pub fn add_capability(&mut self, cap: Capability) {
        if !self.capabilities.contains(&cap) {
            // Add implied capabilities first
            for implied in cap.implies() {
                self.add_capability(*implied);
            }
            self.capabilities.push(cap);
        }
    }

    /// Check if a capability is enabled
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Add an extension
    pub fn add_extension(&mut self, ext: impl Into<String>) {
        let ext = Extension(ext.into());
        if !self.extensions.contains(&ext) {
            self.extensions.push(ext);
        }
    }

    /// Add GLSL.std.450 import
    pub fn add_glsl_std_450(&mut self) -> u32 {
        if let Some(import) = self
            .ext_inst_imports
            .iter()
            .find(|i| i.name == ExtInstImport::GLSL_STD_450)
        {
            return import.id;
        }

        let id = self.ext_inst_imports.len() as u32 + 1;
        self.ext_inst_imports.push(ExtInstImport {
            id,
            name: ExtInstImport::GLSL_STD_450.into(),
        });
        id
    }

    /// Add a global variable
    pub fn add_global(&mut self, var: GlobalVariable) {
        self.globals.push(var);
    }

    /// Get a global variable by value ID
    pub fn get_global(&self, id: ValueId) -> Option<&GlobalVariable> {
        self.globals.iter().find(|g| g.value_id == id)
    }

    /// Get a mutable global variable by value ID
    pub fn get_global_mut(&mut self, id: ValueId) -> Option<&mut GlobalVariable> {
        self.globals.iter_mut().find(|g| g.value_id == id)
    }

    /// Add a debug name
    pub fn add_debug_name(&mut self, id: ValueId, name: impl Into<String>) {
        self.debug_names.push((id, name.into()));
    }

    /// Add a debug member name
    pub fn add_debug_member_name(&mut self, id: ValueId, member: u32, name: impl Into<String>) {
        self.debug_member_names.push((id, member, name.into()));
    }

    /// Get debug name for a value
    pub fn get_debug_name(&self, id: ValueId) -> Option<&str> {
        self.debug_names
            .iter()
            .find(|(v, _)| *v == id)
            .map(|(_, n)| n.as_str())
    }

    /// Create a new function
    pub fn create_function(&mut self, name: impl Into<String>, return_type: IrType) -> FunctionId {
        self.functions.create_function(name, return_type)
    }

    /// Create a new entry point
    pub fn create_entry_point(
        &mut self,
        name: impl Into<String>,
        execution_model: ExecutionModel,
    ) -> FunctionId {
        // Add appropriate capabilities
        match execution_model {
            ExecutionModel::Geometry => self.add_capability(Capability::Geometry),
            ExecutionModel::TessellationControl | ExecutionModel::TessellationEvaluation => {
                self.add_capability(Capability::Tessellation);
            },
            ExecutionModel::TaskEXT | ExecutionModel::MeshEXT => {
                self.add_capability(Capability::MeshShadingEXT);
            },
            ExecutionModel::RayGenerationKHR
            | ExecutionModel::IntersectionKHR
            | ExecutionModel::AnyHitKHR
            | ExecutionModel::ClosestHitKHR
            | ExecutionModel::MissKHR
            | ExecutionModel::CallableKHR => {
                self.add_capability(Capability::RayTracingKHR);
            },
            _ => {},
        }

        self.functions.create_entry_point(name, execution_model)
    }

    /// Get entry points
    pub fn entry_points(&self) -> &[FunctionId] {
        self.functions.entry_points()
    }

    /// Get a function by ID
    pub fn get_function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id)
    }

    /// Get a mutable function by ID
    pub fn get_function_mut(&mut self, id: FunctionId) -> Option<&mut Function> {
        self.functions.get_mut(id)
    }

    /// Add a specialization constant
    pub fn add_spec_constant(&mut self, constant: SpecConstant) {
        self.spec_constants.add(constant);
    }

    /// Register a struct type
    pub fn register_struct(&mut self, struct_type: StructType) -> u32 {
        self.types.register_struct(struct_type)
    }

    /// Allocate a new value ID
    pub fn alloc_value(&mut self) -> ValueId {
        self.values.alloc_id()
    }

    /// Create a constant value
    pub fn create_constant(&mut self, ty: IrType, value: ConstantValue) -> ValueId {
        self.values.create_constant(ty, value)
    }

    /// Validate the module
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check entry points exist
        if self.functions.entry_points().is_empty() {
            errors.push(ValidationError::NoEntryPoints);
        }

        // Check each entry point has execution model
        for &ep_id in self.functions.entry_points() {
            if let Some(func) = self.functions.get(ep_id) {
                if func.execution_model.is_none() {
                    errors.push(ValidationError::MissingExecutionModel(ep_id));
                }
            }
        }

        // Check all functions have terminators
        for func in self.functions.iter() {
            for block in func.blocks() {
                if !block.has_terminator() && !block.is_empty() {
                    errors.push(ValidationError::MissingTerminator {
                        function: func.id,
                        block: block.id,
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get statistics about the module
    pub fn stats(&self) -> ModuleStats {
        let mut instruction_count = 0;
        let mut block_count = 0;

        for func in self.functions.iter() {
            block_count += func.block_count();
            instruction_count += func.instruction_count();
        }

        ModuleStats {
            function_count: self.functions.len(),
            entry_point_count: self.functions.entry_points().len(),
            global_count: self.globals.len(),
            block_count,
            instruction_count,
            capability_count: self.capabilities.len(),
            extension_count: self.extensions.len(),
        }
    }
}

/// Module validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    NoEntryPoints,
    MissingExecutionModel(FunctionId),
    MissingTerminator {
        function: FunctionId,
        block: u32,
    },
    InvalidType {
        location: String,
        message: String,
    },
    InvalidInstruction {
        function: FunctionId,
        block: u32,
        instruction: usize,
        message: String,
    },
    UndefinedValue(ValueId),
    TypeMismatch {
        expected: String,
        found: String,
    },
    InvalidCapability(Capability),
}

/// Module statistics
#[derive(Debug, Clone, Default)]
pub struct ModuleStats {
    pub function_count: usize,
    pub entry_point_count: usize,
    pub global_count: usize,
    pub block_count: usize,
    pub instruction_count: usize,
    pub capability_count: usize,
    pub extension_count: usize,
}

/// Module builder for convenient construction
#[derive(Debug)]
pub struct ModuleBuilder {
    module: Module,
    current_function: Option<FunctionId>,
    current_block: Option<u32>,
}

impl ModuleBuilder {
    /// Create a new module builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            module: Module::new(name),
            current_function: None,
            current_block: None,
        }
    }

    /// Create for Vulkan shaders
    pub fn vulkan(name: impl Into<String>) -> Self {
        Self {
            module: Module::vulkan_shader(name),
            current_function: None,
            current_block: None,
        }
    }

    /// Add capability
    pub fn capability(&mut self, cap: Capability) -> &mut Self {
        self.module.add_capability(cap);
        self
    }

    /// Add extension
    pub fn extension(&mut self, ext: impl Into<String>) -> &mut Self {
        self.module.add_extension(ext);
        self
    }

    /// Add global variable
    pub fn global(&mut self, var: GlobalVariable) -> &mut Self {
        self.module.add_global(var);
        self
    }

    /// Begin entry point function
    pub fn entry_point(
        &mut self,
        name: impl Into<String>,
        execution_model: ExecutionModel,
    ) -> &mut Self {
        let id = self.module.create_entry_point(name, execution_model);
        self.current_function = Some(id);

        // Create entry block
        if let Some(func) = self.module.get_function_mut(id) {
            let block_id = func.blocks.create_entry_block();
            self.current_block = Some(block_id);
        }

        self
    }

    /// Begin regular function
    pub fn function(&mut self, name: impl Into<String>, return_type: IrType) -> &mut Self {
        let id = self.module.create_function(name, return_type);
        self.current_function = Some(id);

        if let Some(func) = self.module.get_function_mut(id) {
            let block_id = func.blocks.create_entry_block();
            self.current_block = Some(block_id);
        }

        self
    }

    /// Add instruction to current block
    pub fn instruction(&mut self, inst: Instruction) -> &mut Self {
        if let (Some(func_id), Some(block_id)) = (self.current_function, self.current_block) {
            if let Some(func) = self.module.get_function_mut(func_id) {
                if let Some(block) = func.blocks.get_mut(block_id) {
                    block.push(inst);
                }
            }
        }
        self
    }

    /// Create a new block in current function
    pub fn block(&mut self) -> u32 {
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                let id = func.blocks.create_block();
                self.current_block = Some(id);
                return id;
            }
        }
        0
    }

    /// Switch to a block
    pub fn switch_block(&mut self, block_id: u32) -> &mut Self {
        self.current_block = Some(block_id);
        self
    }

    /// Build the module
    pub fn build(self) -> Module {
        self.module
    }

    /// Get the module reference
    pub fn module(&self) -> &Module {
        &self.module
    }

    /// Get the module mutable reference
    pub fn module_mut(&mut self) -> &mut Module {
        &mut self.module
    }

    /// Allocate a value ID
    pub fn alloc_value(&mut self) -> ValueId {
        self.module.alloc_value()
    }

    /// Create a constant
    pub fn constant(&mut self, ty: IrType, value: ConstantValue) -> ValueId {
        self.module.create_constant(ty, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creation() {
        let module = Module::new("test");
        assert_eq!(module.name, "test");
        assert!(module.has_capability(Capability::Shader));
    }

    #[test]
    fn test_vulkan_module() {
        let module = Module::vulkan_shader("test");
        assert!(module.has_capability(Capability::VulkanMemoryModel));
        assert_eq!(module.memory_model, MemoryModel::Vulkan);
    }

    #[test]
    fn test_capability_implies() {
        let mut module = Module::new("test");
        module.add_capability(Capability::Geometry);

        // Geometry implies Shader which implies Matrix
        assert!(module.has_capability(Capability::Geometry));
        assert!(module.has_capability(Capability::Shader));
        assert!(module.has_capability(Capability::Matrix));
    }

    #[test]
    fn test_module_builder() {
        let module = ModuleBuilder::vulkan("test")
            .capability(Capability::Float16)
            .entry_point("main", ExecutionModel::Fragment)
            .instruction(Instruction::Return)
            .build();

        assert!(module.has_capability(Capability::Float16));
        assert_eq!(module.entry_points().len(), 1);
    }

    #[test]
    fn test_glsl_import() {
        let mut module = Module::new("test");
        let id1 = module.add_glsl_std_450();
        let id2 = module.add_glsl_std_450();
        assert_eq!(id1, id2); // Should return same ID
    }

    #[test]
    fn test_module_stats() {
        let module = ModuleBuilder::vulkan("test")
            .entry_point("main", ExecutionModel::Vertex)
            .instruction(Instruction::Return)
            .build();

        let stats = module.stats();
        assert_eq!(stats.function_count, 1);
        assert_eq!(stats.entry_point_count, 1);
        assert_eq!(stats.instruction_count, 1);
    }
}
