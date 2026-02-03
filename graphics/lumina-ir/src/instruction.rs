//! IR Instructions
//!
//! This module defines all instructions in the Lumina IR.
//! Instructions are grouped by category and designed to map efficiently
//! to SPIR-V, DXIL, and Metal backends.

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

use crate::types::{IrType, ScalarType, VectorSize, AddressSpace, BuiltinKind, ImageDimension};
use crate::value::{ValueId, ConstantValue};

/// Block identifier
pub type BlockId = u32;

/// Function identifier
pub type FunctionId = u32;

/// Instruction identifier
pub type InstructionId = u32;

/// Memory semantics for atomic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum MemorySemantics {
    None = 0,
    Acquire = 1 << 1,
    Release = 1 << 2,
    AcquireRelease = (1 << 1) | (1 << 2),
    SequentiallyConsistent = 1 << 4,
    UniformMemory = 1 << 6,
    WorkgroupMemory = 1 << 8,
    ImageMemory = 1 << 11,
    OutputMemory = 1 << 12,
}

/// Memory scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Scope {
    CrossDevice = 0,
    Device = 1,
    Workgroup = 2,
    Subgroup = 3,
    Invocation = 4,
    QueueFamily = 5,
    ShaderCall = 6,
}

/// Binary operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Mod,
    
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRightLogical,
    ShiftRightArithmetic,
    
    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    
    // Logical
    LogicalAnd,
    LogicalOr,
    LogicalEqual,
    LogicalNotEqual,
    
    // Vector
    Dot,
    Cross,
    
    // Float-specific
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    FMod,
    
    // Ordered comparison (for floats)
    FOrdEqual,
    FOrdNotEqual,
    FOrdLess,
    FOrdLessEqual,
    FOrdGreater,
    FOrdGreaterEqual,
    
    // Unordered comparison (for floats)
    FUnordEqual,
    FUnordNotEqual,
    FUnordLess,
    FUnordLessEqual,
    FUnordGreater,
    FUnordGreaterEqual,
    
    // Integer-specific
    IAdd,
    ISub,
    IMul,
    SDiv,
    UDiv,
    SRem,
    UMod,
    SMod,
}

impl BinaryOp {
    /// Check if this is a comparison operation
    pub const fn is_comparison(&self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::Less
                | Self::LessEqual
                | Self::Greater
                | Self::GreaterEqual
                | Self::FOrdEqual
                | Self::FOrdNotEqual
                | Self::FOrdLess
                | Self::FOrdLessEqual
                | Self::FOrdGreater
                | Self::FOrdGreaterEqual
                | Self::FUnordEqual
                | Self::FUnordNotEqual
                | Self::FUnordLess
                | Self::FUnordLessEqual
                | Self::FUnordGreater
                | Self::FUnordGreaterEqual
        )
    }

    /// Check if this is a logical operation
    pub const fn is_logical(&self) -> bool {
        matches!(
            self,
            Self::LogicalAnd | Self::LogicalOr | Self::LogicalEqual | Self::LogicalNotEqual
        )
    }

    /// Check if this is a bitwise operation
    pub const fn is_bitwise(&self) -> bool {
        matches!(
            self,
            Self::BitwiseAnd
                | Self::BitwiseOr
                | Self::BitwiseXor
                | Self::ShiftLeft
                | Self::ShiftRightLogical
                | Self::ShiftRightArithmetic
        )
    }

    /// Check if this is an arithmetic operation
    pub const fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            Self::Add
                | Self::Sub
                | Self::Mul
                | Self::Div
                | Self::Rem
                | Self::Mod
                | Self::FAdd
                | Self::FSub
                | Self::FMul
                | Self::FDiv
                | Self::FRem
                | Self::FMod
                | Self::IAdd
                | Self::ISub
                | Self::IMul
                | Self::SDiv
                | Self::UDiv
                | Self::SRem
                | Self::UMod
                | Self::SMod
        )
    }
}

/// Unary operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum UnaryOp {
    // Arithmetic
    Negate,
    FNegate,
    
    // Bitwise
    BitwiseNot,
    
    // Logical
    LogicalNot,
    
    // Math functions
    Abs,
    FAbs,
    Sign,
    FSign,
    Floor,
    Ceil,
    Round,
    RoundEven,
    Trunc,
    Fract,
    Sqrt,
    InverseSqrt,
    Exp,
    Exp2,
    Log,
    Log2,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    
    // Conversion
    BitcastToFloat,
    BitcastToInt,
    BitcastToUint,
    ConvertSToF,
    ConvertUToF,
    ConvertFToS,
    ConvertFToU,
    
    // Vector
    Length,
    Normalize,
    
    // Derivative
    DPdx,
    DPdy,
    DPdxFine,
    DPdyFine,
    DPdxCoarse,
    DPdyCoarse,
    Fwidth,
    FwidthFine,
    FwidthCoarse,
    
    // Bit counting
    BitCount,
    BitReverse,
    FindLSB,
    FindMSB,
    FindSMSB,
    
    // Type info
    IsNan,
    IsInf,
    IsFinite,
    IsNormal,
    
    // Special
    All,
    Any,
    Transpose,
    Determinant,
    MatrixInverse,
}

/// Atomic operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AtomicOp {
    Load,
    Store,
    Exchange,
    CompareExchange,
    CompareExchangeWeak,
    Add,
    Sub,
    And,
    Or,
    Xor,
    Min,
    Max,
    SMin,
    SMax,
    UMin,
    UMax,
    FlagTestAndSet,
    FlagClear,
}

/// Image operand flags
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ImageOperands {
    pub bias: Option<ValueId>,
    pub lod: Option<ValueId>,
    pub grad: Option<(ValueId, ValueId)>,
    pub const_offset: Option<ValueId>,
    pub offset: Option<ValueId>,
    pub const_offsets: Option<ValueId>,
    pub sample: Option<ValueId>,
    pub min_lod: Option<ValueId>,
    pub make_texel_available: bool,
    pub make_texel_visible: bool,
    pub non_private_texel: bool,
    pub volatile_texel: bool,
    pub signed_result: bool,
    pub nontemporal: bool,
}

/// Texture gather component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GatherComponent {
    X = 0,
    Y = 1,
    Z = 2,
    W = 3,
}

/// Memory access flags
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MemoryAccess {
    pub volatile: bool,
    pub aligned: Option<u32>,
    pub nontemporal: bool,
    pub make_pointer_available: bool,
    pub make_pointer_visible: bool,
    pub non_private_pointer: bool,
}

/// Loop control hints
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoopControl {
    pub unroll: bool,
    pub dont_unroll: bool,
    pub dependency_infinite: bool,
    pub dependency_length: Option<u32>,
    pub min_iterations: Option<u32>,
    pub max_iterations: Option<u32>,
    pub iteration_multiple: Option<u32>,
    pub peel_count: Option<u32>,
    pub partial_count: Option<u32>,
}

/// Selection control hints
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectionControl {
    pub flatten: bool,
    pub dont_flatten: bool,
}

/// Function control hints
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FunctionControl {
    pub inline: bool,
    pub dont_inline: bool,
    pub pure: bool,
    pub const_: bool,
    pub opt_none: bool,
}

/// Ray flags for ray tracing
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RayFlags {
    pub opaque: bool,
    pub no_opaque: bool,
    pub terminate_on_first_hit: bool,
    pub skip_closest_hit_shader: bool,
    pub cull_back_facing_triangles: bool,
    pub cull_front_facing_triangles: bool,
    pub cull_opaque: bool,
    pub cull_no_opaque: bool,
    pub skip_triangles: bool,
    pub skip_aabbs: bool,
}

/// The main instruction enum
#[derive(Debug, Clone)]
pub enum Instruction {
    // ========== Constant & Variable ==========
    /// Define a constant value
    Constant {
        result: ValueId,
        ty: IrType,
        value: ConstantValue,
    },
    
    /// Declare a variable
    Variable {
        result: ValueId,
        ty: IrType,
        address_space: AddressSpace,
        initializer: Option<ValueId>,
    },
    
    /// Declare a function parameter
    FunctionParameter {
        result: ValueId,
        ty: IrType,
    },

    // ========== Memory Operations ==========
    /// Load from a pointer
    Load {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        access: MemoryAccess,
    },
    
    /// Store to a pointer
    Store {
        pointer: ValueId,
        value: ValueId,
        access: MemoryAccess,
    },
    
    /// Get pointer to struct member
    AccessChain {
        result: ValueId,
        ty: IrType,
        base: ValueId,
        indices: Vec<ValueId>,
    },
    
    /// Get pointer to array element
    PtrAccessChain {
        result: ValueId,
        ty: IrType,
        base: ValueId,
        element: ValueId,
        indices: Vec<ValueId>,
    },
    
    /// Copy memory
    CopyMemory {
        target: ValueId,
        source: ValueId,
        access: MemoryAccess,
    },
    
    /// Copy memory with size
    CopyMemorySized {
        target: ValueId,
        source: ValueId,
        size: ValueId,
        access: MemoryAccess,
    },

    // ========== Arithmetic Operations ==========
    /// Binary operation
    BinaryOp {
        result: ValueId,
        ty: IrType,
        op: BinaryOp,
        left: ValueId,
        right: ValueId,
    },
    
    /// Unary operation
    UnaryOp {
        result: ValueId,
        ty: IrType,
        op: UnaryOp,
        operand: ValueId,
    },
    
    /// Fused multiply-add: a * b + c
    Fma {
        result: ValueId,
        ty: IrType,
        a: ValueId,
        b: ValueId,
        c: ValueId,
    },
    
    /// Clamp value between min and max
    Clamp {
        result: ValueId,
        ty: IrType,
        value: ValueId,
        min: ValueId,
        max: ValueId,
    },
    
    /// Mix/lerp: a + (b - a) * t
    Mix {
        result: ValueId,
        ty: IrType,
        a: ValueId,
        b: ValueId,
        t: ValueId,
    },
    
    /// Smoothstep
    SmoothStep {
        result: ValueId,
        ty: IrType,
        edge0: ValueId,
        edge1: ValueId,
        x: ValueId,
    },
    
    /// Step function
    Step {
        result: ValueId,
        ty: IrType,
        edge: ValueId,
        x: ValueId,
    },
    
    /// Min of two values
    Min {
        result: ValueId,
        ty: IrType,
        a: ValueId,
        b: ValueId,
    },
    
    /// Max of two values
    Max {
        result: ValueId,
        ty: IrType,
        a: ValueId,
        b: ValueId,
    },
    
    /// Power function
    Pow {
        result: ValueId,
        ty: IrType,
        base: ValueId,
        exponent: ValueId,
    },
    
    /// Atan2
    Atan2 {
        result: ValueId,
        ty: IrType,
        y: ValueId,
        x: ValueId,
    },
    
    /// Reflect vector
    Reflect {
        result: ValueId,
        ty: IrType,
        incident: ValueId,
        normal: ValueId,
    },
    
    /// Refract vector
    Refract {
        result: ValueId,
        ty: IrType,
        incident: ValueId,
        normal: ValueId,
        eta: ValueId,
    },
    
    /// Face forward
    FaceForward {
        result: ValueId,
        ty: IrType,
        n: ValueId,
        i: ValueId,
        nref: ValueId,
    },
    
    /// Distance between points
    Distance {
        result: ValueId,
        ty: IrType,
        a: ValueId,
        b: ValueId,
    },
    
    /// Outer product of vectors
    OuterProduct {
        result: ValueId,
        ty: IrType,
        a: ValueId,
        b: ValueId,
    },
    
    /// Matrix times vector
    MatrixTimesVector {
        result: ValueId,
        ty: IrType,
        matrix: ValueId,
        vector: ValueId,
    },
    
    /// Vector times matrix
    VectorTimesMatrix {
        result: ValueId,
        ty: IrType,
        vector: ValueId,
        matrix: ValueId,
    },
    
    /// Matrix times matrix
    MatrixTimesMatrix {
        result: ValueId,
        ty: IrType,
        left: ValueId,
        right: ValueId,
    },
    
    /// Matrix times scalar
    MatrixTimesScalar {
        result: ValueId,
        ty: IrType,
        matrix: ValueId,
        scalar: ValueId,
    },
    
    /// Vector times scalar
    VectorTimesScalar {
        result: ValueId,
        ty: IrType,
        vector: ValueId,
        scalar: ValueId,
    },

    // ========== Type Conversion ==========
    /// Bitcast between types
    Bitcast {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert signed int to float
    ConvertSToF {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert unsigned int to float
    ConvertUToF {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert float to signed int
    ConvertFToS {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert float to unsigned int
    ConvertFToU {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert between signed int sizes
    SConvert {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert between unsigned int sizes
    UConvert {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Convert between float sizes
    FConvert {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },
    
    /// Quantize float to f16 precision
    QuantizeToF16 {
        result: ValueId,
        ty: IrType,
        value: ValueId,
    },

    // ========== Composite Operations ==========
    /// Construct a composite (vector, matrix, struct, array)
    CompositeConstruct {
        result: ValueId,
        ty: IrType,
        components: Vec<ValueId>,
    },
    
    /// Extract a component from a composite
    CompositeExtract {
        result: ValueId,
        ty: IrType,
        composite: ValueId,
        indices: Vec<u32>,
    },
    
    /// Insert a component into a composite
    CompositeInsert {
        result: ValueId,
        ty: IrType,
        object: ValueId,
        composite: ValueId,
        indices: Vec<u32>,
    },
    
    /// Shuffle vector components
    VectorShuffle {
        result: ValueId,
        ty: IrType,
        vector1: ValueId,
        vector2: ValueId,
        components: Vec<u32>,
    },
    
    /// Extract a scalar from a vector
    VectorExtractDynamic {
        result: ValueId,
        ty: IrType,
        vector: ValueId,
        index: ValueId,
    },
    
    /// Insert a scalar into a vector
    VectorInsertDynamic {
        result: ValueId,
        ty: IrType,
        vector: ValueId,
        component: ValueId,
        index: ValueId,
    },
    
    /// Copy object
    CopyObject {
        result: ValueId,
        ty: IrType,
        operand: ValueId,
    },

    // ========== Control Flow ==========
    /// Unconditional branch
    Branch {
        target: BlockId,
    },
    
    /// Conditional branch
    BranchConditional {
        condition: ValueId,
        true_target: BlockId,
        false_target: BlockId,
        true_weight: Option<u32>,
        false_weight: Option<u32>,
    },
    
    /// Switch statement
    Switch {
        selector: ValueId,
        default_target: BlockId,
        cases: Vec<(i64, BlockId)>,
    },
    
    /// Return from function
    Return,
    
    /// Return with value
    ReturnValue {
        value: ValueId,
    },
    
    /// Function call
    FunctionCall {
        result: ValueId,
        ty: IrType,
        function: FunctionId,
        arguments: Vec<ValueId>,
    },
    
    /// Kill fragment (discard)
    Kill,
    
    /// Terminate invocation
    TerminateInvocation,
    
    /// Unreachable code
    Unreachable,
    
    /// Demote fragment to helper invocation
    DemoteToHelperInvocation,
    
    /// Check if current invocation is helper
    IsHelperInvocation {
        result: ValueId,
    },
    
    /// Phi node for SSA
    Phi {
        result: ValueId,
        ty: IrType,
        operands: Vec<(ValueId, BlockId)>,
    },
    
    /// Select between values
    Select {
        result: ValueId,
        ty: IrType,
        condition: ValueId,
        true_value: ValueId,
        false_value: ValueId,
    },
    
    /// Loop merge point
    LoopMerge {
        merge_block: BlockId,
        continue_target: BlockId,
        control: LoopControl,
    },
    
    /// Selection merge point
    SelectionMerge {
        merge_block: BlockId,
        control: SelectionControl,
    },
    
    /// Label (block start)
    Label {
        result: BlockId,
    },

    // ========== Image Operations ==========
    /// Sample texture
    ImageSampleImplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Sample texture with explicit LOD
    ImageSampleExplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Sample depth texture with comparison
    ImageSampleDrefImplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        dref: ValueId,
        operands: ImageOperands,
    },
    
    /// Sample depth texture with explicit LOD
    ImageSampleDrefExplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        dref: ValueId,
        operands: ImageOperands,
    },
    
    /// Sample texture with projection
    ImageSampleProjImplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Sample texture with projection and explicit LOD
    ImageSampleProjExplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Fetch texel
    ImageFetch {
        result: ValueId,
        ty: IrType,
        image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Read from storage image
    ImageRead {
        result: ValueId,
        ty: IrType,
        image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Write to storage image
    ImageWrite {
        image: ValueId,
        coordinate: ValueId,
        texel: ValueId,
        operands: ImageOperands,
    },
    
    /// Gather texture
    ImageGather {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        component: GatherComponent,
        operands: ImageOperands,
    },
    
    /// Gather depth texture
    ImageDrefGather {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        dref: ValueId,
        operands: ImageOperands,
    },
    
    /// Get image dimensions
    ImageQuerySize {
        result: ValueId,
        ty: IrType,
        image: ValueId,
    },
    
    /// Get image dimensions with LOD
    ImageQuerySizeLod {
        result: ValueId,
        ty: IrType,
        image: ValueId,
        lod: ValueId,
    },
    
    /// Get number of LOD levels
    ImageQueryLevels {
        result: ValueId,
        ty: IrType,
        image: ValueId,
    },
    
    /// Get number of samples
    ImageQuerySamples {
        result: ValueId,
        ty: IrType,
        image: ValueId,
    },
    
    /// Get LOD for sampling
    ImageQueryLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
    },
    
    /// Get image from sampled image
    Image {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
    },
    
    /// Create sampled image
    SampledImage {
        result: ValueId,
        ty: IrType,
        image: ValueId,
        sampler: ValueId,
    },
    
    /// Sparse texture sample
    ImageSparseSampleImplicitLod {
        result: ValueId,
        ty: IrType,
        sampled_image: ValueId,
        coordinate: ValueId,
        operands: ImageOperands,
    },
    
    /// Get sparse texture residency
    ImageSparseTexelsResident {
        result: ValueId,
        resident_code: ValueId,
    },

    // ========== Atomic Operations ==========
    /// Atomic load
    AtomicLoad {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
    },
    
    /// Atomic store
    AtomicStore {
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic exchange
    AtomicExchange {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic compare-exchange
    AtomicCompareExchange {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        equal_semantics: MemorySemantics,
        unequal_semantics: MemorySemantics,
        value: ValueId,
        comparator: ValueId,
    },
    
    /// Atomic increment and wrap
    AtomicIIncrement {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
    },
    
    /// Atomic decrement and wrap
    AtomicIDecrement {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
    },
    
    /// Atomic add
    AtomicIAdd {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic sub
    AtomicISub {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic signed min
    AtomicSMin {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic unsigned min
    AtomicUMin {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic signed max
    AtomicSMax {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic unsigned max
    AtomicUMax {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic and
    AtomicAnd {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic or
    AtomicOr {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic xor
    AtomicXor {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic float add
    AtomicFAdd {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic float min
    AtomicFMin {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },
    
    /// Atomic float max
    AtomicFMax {
        result: ValueId,
        ty: IrType,
        pointer: ValueId,
        scope: Scope,
        semantics: MemorySemantics,
        value: ValueId,
    },

    // ========== Barrier & Control ==========
    /// Memory barrier
    MemoryBarrier {
        scope: Scope,
        semantics: MemorySemantics,
    },
    
    /// Control barrier
    ControlBarrier {
        execution_scope: Scope,
        memory_scope: Scope,
        semantics: MemorySemantics,
    },

    // ========== Subgroup Operations ==========
    /// Broadcast subgroup value
    GroupBroadcast {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        local_id: ValueId,
    },
    
    /// Subgroup any
    GroupNonUniformAny {
        result: ValueId,
        scope: Scope,
        predicate: ValueId,
    },
    
    /// Subgroup all
    GroupNonUniformAll {
        result: ValueId,
        scope: Scope,
        predicate: ValueId,
    },
    
    /// Subgroup all equal
    GroupNonUniformAllEqual {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
    },
    
    /// Subgroup broadcast
    GroupNonUniformBroadcast {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        id: ValueId,
    },
    
    /// Subgroup broadcast first
    GroupNonUniformBroadcastFirst {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
    },
    
    /// Subgroup ballot
    GroupNonUniformBallot {
        result: ValueId,
        scope: Scope,
        predicate: ValueId,
    },
    
    /// Inverse subgroup ballot
    GroupNonUniformInverseBallot {
        result: ValueId,
        scope: Scope,
        value: ValueId,
    },
    
    /// Ballot bit extract
    GroupNonUniformBallotBitExtract {
        result: ValueId,
        scope: Scope,
        value: ValueId,
        index: ValueId,
    },
    
    /// Ballot bit count
    GroupNonUniformBallotBitCount {
        result: ValueId,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
    },
    
    /// Ballot find LSB
    GroupNonUniformBallotFindLSB {
        result: ValueId,
        scope: Scope,
        value: ValueId,
    },
    
    /// Ballot find MSB
    GroupNonUniformBallotFindMSB {
        result: ValueId,
        scope: Scope,
        value: ValueId,
    },
    
    /// Subgroup shuffle
    GroupNonUniformShuffle {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        id: ValueId,
    },
    
    /// Subgroup shuffle XOR
    GroupNonUniformShuffleXor {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        mask: ValueId,
    },
    
    /// Subgroup shuffle up
    GroupNonUniformShuffleUp {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        delta: ValueId,
    },
    
    /// Subgroup shuffle down
    GroupNonUniformShuffleDown {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        delta: ValueId,
    },
    
    /// Subgroup reduction
    GroupNonUniformIAdd {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (float)
    GroupNonUniformFAdd {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (multiply)
    GroupNonUniformIMul {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (float multiply)
    GroupNonUniformFMul {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (min)
    GroupNonUniformSMin {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (max)
    GroupNonUniformSMax {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (bitwise AND)
    GroupNonUniformBitwiseAnd {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (bitwise OR)
    GroupNonUniformBitwiseOr {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (bitwise XOR)
    GroupNonUniformBitwiseXor {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (logical AND)
    GroupNonUniformLogicalAnd {
        result: ValueId,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (logical OR)
    GroupNonUniformLogicalOr {
        result: ValueId,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Subgroup reduction (logical XOR)
    GroupNonUniformLogicalXor {
        result: ValueId,
        scope: Scope,
        operation: GroupOperation,
        value: ValueId,
        cluster_size: Option<ValueId>,
    },
    
    /// Quad broadcast
    GroupNonUniformQuadBroadcast {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        index: ValueId,
    },
    
    /// Quad swap
    GroupNonUniformQuadSwap {
        result: ValueId,
        ty: IrType,
        scope: Scope,
        value: ValueId,
        direction: QuadDirection,
    },

    // ========== Ray Tracing ==========
    /// Trace ray
    TraceRay {
        accel: ValueId,
        ray_flags: RayFlags,
        cull_mask: ValueId,
        sbt_offset: ValueId,
        sbt_stride: ValueId,
        miss_index: ValueId,
        ray_origin: ValueId,
        ray_tmin: ValueId,
        ray_direction: ValueId,
        ray_tmax: ValueId,
        payload: ValueId,
    },
    
    /// Report ray-triangle intersection
    ReportIntersection {
        result: ValueId,
        hit_t: ValueId,
        hit_kind: ValueId,
    },
    
    /// Ignore intersection
    IgnoreIntersection,
    
    /// Terminate ray
    TerminateRay,
    
    /// Execute callable shader
    ExecuteCallable {
        sbt_index: ValueId,
        callable_data: ValueId,
    },
    
    /// Initialize ray query
    RayQueryInitialize {
        ray_query: ValueId,
        accel: ValueId,
        ray_flags: RayFlags,
        cull_mask: ValueId,
        ray_origin: ValueId,
        ray_tmin: ValueId,
        ray_direction: ValueId,
        ray_tmax: ValueId,
    },
    
    /// Proceed with ray query
    RayQueryProceed {
        result: ValueId,
        ray_query: ValueId,
    },
    
    /// Terminate ray query
    RayQueryTerminate {
        ray_query: ValueId,
    },
    
    /// Confirm ray query intersection
    RayQueryConfirmIntersection {
        ray_query: ValueId,
    },
    
    /// Generate candidate intersection
    RayQueryGenerateIntersection {
        ray_query: ValueId,
        hit_t: ValueId,
    },
    
    /// Get ray query intersection type
    RayQueryGetIntersectionType {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection T
    RayQueryGetIntersectionT {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection instance custom index
    RayQueryGetIntersectionInstanceCustomIndex {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection instance id
    RayQueryGetIntersectionInstanceId {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection instance shader binding table offset
    RayQueryGetIntersectionInstanceSBTOffset {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection geometry index
    RayQueryGetIntersectionGeometryIndex {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection primitive index
    RayQueryGetIntersectionPrimitiveIndex {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection barycentrics
    RayQueryGetIntersectionBarycentrics {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection front face
    RayQueryGetIntersectionFrontFace {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection candidate AABB opaque
    RayQueryGetIntersectionCandidateAABBOpaque {
        result: ValueId,
        ray_query: ValueId,
    },
    
    /// Get ray query intersection object ray direction
    RayQueryGetIntersectionObjectRayDirection {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection object ray origin
    RayQueryGetIntersectionObjectRayOrigin {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query world ray direction
    RayQueryGetWorldRayDirection {
        result: ValueId,
        ray_query: ValueId,
    },
    
    /// Get ray query world ray origin
    RayQueryGetWorldRayOrigin {
        result: ValueId,
        ray_query: ValueId,
    },
    
    /// Get ray query intersection object to world matrix
    RayQueryGetIntersectionObjectToWorld {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query intersection world to object matrix
    RayQueryGetIntersectionWorldToObject {
        result: ValueId,
        ray_query: ValueId,
        committed: bool,
    },
    
    /// Get ray query ray Tmin
    RayQueryGetRayTmin {
        result: ValueId,
        ray_query: ValueId,
    },
    
    /// Get ray query ray flags
    RayQueryGetRayFlags {
        result: ValueId,
        ray_query: ValueId,
    },

    // ========== Mesh Shader ==========
    /// Set mesh outputs
    SetMeshOutputs {
        vertex_count: ValueId,
        primitive_count: ValueId,
    },
    
    /// Emit mesh vertex
    EmitMeshVertex {
        vertex_index: ValueId,
    },
    
    /// Emit mesh primitive
    EmitMeshPrimitive {
        primitive_index: ValueId,
        vertex_indices: Vec<ValueId>,
    },
    
    /// Write packed primitive indices
    WritePrimitiveIndices {
        index_offset: ValueId,
        packed_indices: ValueId,
    },

    // ========== Misc ==========
    /// Debug printf
    DebugPrintf {
        format: String,
        values: Vec<ValueId>,
    },
    
    /// No operation
    Nop,
    
    /// Undefined value
    Undef {
        result: ValueId,
        ty: IrType,
    },
    
    /// Extension instruction
    ExtInst {
        result: ValueId,
        ty: IrType,
        set: String,
        instruction: u32,
        operands: Vec<ValueId>,
    },
    
    /// Assume condition is true (for optimizer)
    Assume {
        condition: ValueId,
    },
    
    /// Expect value (for branch prediction)
    Expect {
        result: ValueId,
        ty: IrType,
        value: ValueId,
        expected: ValueId,
    },
}

/// Group operation for subgroup reductions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GroupOperation {
    Reduce = 0,
    InclusiveScan = 1,
    ExclusiveScan = 2,
    ClusteredReduce = 3,
    PartitionedReduceNV = 6,
    PartitionedInclusiveScanNV = 7,
    PartitionedExclusiveScanNV = 8,
}

/// Quad swap direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum QuadDirection {
    Horizontal = 0,
    Vertical = 1,
    Diagonal = 2,
}

impl Instruction {
    /// Get the result value ID if this instruction produces one
    pub fn result(&self) -> Option<ValueId> {
        match self {
            Self::Constant { result, .. }
            | Self::Variable { result, .. }
            | Self::FunctionParameter { result, .. }
            | Self::Load { result, .. }
            | Self::AccessChain { result, .. }
            | Self::PtrAccessChain { result, .. }
            | Self::BinaryOp { result, .. }
            | Self::UnaryOp { result, .. }
            | Self::Fma { result, .. }
            | Self::Clamp { result, .. }
            | Self::Mix { result, .. }
            | Self::SmoothStep { result, .. }
            | Self::Step { result, .. }
            | Self::Min { result, .. }
            | Self::Max { result, .. }
            | Self::Pow { result, .. }
            | Self::Atan2 { result, .. }
            | Self::Reflect { result, .. }
            | Self::Refract { result, .. }
            | Self::FaceForward { result, .. }
            | Self::Distance { result, .. }
            | Self::OuterProduct { result, .. }
            | Self::MatrixTimesVector { result, .. }
            | Self::VectorTimesMatrix { result, .. }
            | Self::MatrixTimesMatrix { result, .. }
            | Self::MatrixTimesScalar { result, .. }
            | Self::VectorTimesScalar { result, .. }
            | Self::Bitcast { result, .. }
            | Self::ConvertSToF { result, .. }
            | Self::ConvertUToF { result, .. }
            | Self::ConvertFToS { result, .. }
            | Self::ConvertFToU { result, .. }
            | Self::SConvert { result, .. }
            | Self::UConvert { result, .. }
            | Self::FConvert { result, .. }
            | Self::QuantizeToF16 { result, .. }
            | Self::CompositeConstruct { result, .. }
            | Self::CompositeExtract { result, .. }
            | Self::CompositeInsert { result, .. }
            | Self::VectorShuffle { result, .. }
            | Self::VectorExtractDynamic { result, .. }
            | Self::VectorInsertDynamic { result, .. }
            | Self::CopyObject { result, .. }
            | Self::FunctionCall { result, .. }
            | Self::IsHelperInvocation { result }
            | Self::Phi { result, .. }
            | Self::Select { result, .. }
            | Self::ImageSampleImplicitLod { result, .. }
            | Self::ImageSampleExplicitLod { result, .. }
            | Self::ImageSampleDrefImplicitLod { result, .. }
            | Self::ImageSampleDrefExplicitLod { result, .. }
            | Self::ImageSampleProjImplicitLod { result, .. }
            | Self::ImageSampleProjExplicitLod { result, .. }
            | Self::ImageFetch { result, .. }
            | Self::ImageRead { result, .. }
            | Self::ImageGather { result, .. }
            | Self::ImageDrefGather { result, .. }
            | Self::ImageQuerySize { result, .. }
            | Self::ImageQuerySizeLod { result, .. }
            | Self::ImageQueryLevels { result, .. }
            | Self::ImageQuerySamples { result, .. }
            | Self::ImageQueryLod { result, .. }
            | Self::Image { result, .. }
            | Self::SampledImage { result, .. }
            | Self::ImageSparseSampleImplicitLod { result, .. }
            | Self::ImageSparseTexelsResident { result, .. }
            | Self::AtomicLoad { result, .. }
            | Self::AtomicExchange { result, .. }
            | Self::AtomicCompareExchange { result, .. }
            | Self::AtomicIIncrement { result, .. }
            | Self::AtomicIDecrement { result, .. }
            | Self::AtomicIAdd { result, .. }
            | Self::AtomicISub { result, .. }
            | Self::AtomicSMin { result, .. }
            | Self::AtomicUMin { result, .. }
            | Self::AtomicSMax { result, .. }
            | Self::AtomicUMax { result, .. }
            | Self::AtomicAnd { result, .. }
            | Self::AtomicOr { result, .. }
            | Self::AtomicXor { result, .. }
            | Self::AtomicFAdd { result, .. }
            | Self::AtomicFMin { result, .. }
            | Self::AtomicFMax { result, .. }
            | Self::GroupBroadcast { result, .. }
            | Self::GroupNonUniformAny { result, .. }
            | Self::GroupNonUniformAll { result, .. }
            | Self::GroupNonUniformAllEqual { result, .. }
            | Self::GroupNonUniformBroadcast { result, .. }
            | Self::GroupNonUniformBroadcastFirst { result, .. }
            | Self::GroupNonUniformBallot { result, .. }
            | Self::GroupNonUniformInverseBallot { result, .. }
            | Self::GroupNonUniformBallotBitExtract { result, .. }
            | Self::GroupNonUniformBallotBitCount { result, .. }
            | Self::GroupNonUniformBallotFindLSB { result, .. }
            | Self::GroupNonUniformBallotFindMSB { result, .. }
            | Self::GroupNonUniformShuffle { result, .. }
            | Self::GroupNonUniformShuffleXor { result, .. }
            | Self::GroupNonUniformShuffleUp { result, .. }
            | Self::GroupNonUniformShuffleDown { result, .. }
            | Self::GroupNonUniformIAdd { result, .. }
            | Self::GroupNonUniformFAdd { result, .. }
            | Self::GroupNonUniformIMul { result, .. }
            | Self::GroupNonUniformFMul { result, .. }
            | Self::GroupNonUniformSMin { result, .. }
            | Self::GroupNonUniformSMax { result, .. }
            | Self::GroupNonUniformBitwiseAnd { result, .. }
            | Self::GroupNonUniformBitwiseOr { result, .. }
            | Self::GroupNonUniformBitwiseXor { result, .. }
            | Self::GroupNonUniformLogicalAnd { result, .. }
            | Self::GroupNonUniformLogicalOr { result, .. }
            | Self::GroupNonUniformLogicalXor { result, .. }
            | Self::GroupNonUniformQuadBroadcast { result, .. }
            | Self::GroupNonUniformQuadSwap { result, .. }
            | Self::ReportIntersection { result, .. }
            | Self::RayQueryProceed { result, .. }
            | Self::RayQueryGetIntersectionType { result, .. }
            | Self::RayQueryGetIntersectionT { result, .. }
            | Self::RayQueryGetIntersectionInstanceCustomIndex { result, .. }
            | Self::RayQueryGetIntersectionInstanceId { result, .. }
            | Self::RayQueryGetIntersectionInstanceSBTOffset { result, .. }
            | Self::RayQueryGetIntersectionGeometryIndex { result, .. }
            | Self::RayQueryGetIntersectionPrimitiveIndex { result, .. }
            | Self::RayQueryGetIntersectionBarycentrics { result, .. }
            | Self::RayQueryGetIntersectionFrontFace { result, .. }
            | Self::RayQueryGetIntersectionCandidateAABBOpaque { result, .. }
            | Self::RayQueryGetIntersectionObjectRayDirection { result, .. }
            | Self::RayQueryGetIntersectionObjectRayOrigin { result, .. }
            | Self::RayQueryGetWorldRayDirection { result, .. }
            | Self::RayQueryGetWorldRayOrigin { result, .. }
            | Self::RayQueryGetIntersectionObjectToWorld { result, .. }
            | Self::RayQueryGetIntersectionWorldToObject { result, .. }
            | Self::RayQueryGetRayTmin { result, .. }
            | Self::RayQueryGetRayFlags { result, .. }
            | Self::Undef { result, .. }
            | Self::ExtInst { result, .. }
            | Self::Expect { result, .. } => Some(*result),
            
            _ => None,
        }
    }

    /// Get the result type if this instruction produces a value
    pub fn result_type(&self) -> Option<&IrType> {
        match self {
            Self::Constant { ty, .. }
            | Self::Variable { ty, .. }
            | Self::FunctionParameter { ty, .. }
            | Self::Load { ty, .. }
            | Self::AccessChain { ty, .. }
            | Self::PtrAccessChain { ty, .. }
            | Self::BinaryOp { ty, .. }
            | Self::UnaryOp { ty, .. }
            | Self::Fma { ty, .. }
            | Self::Clamp { ty, .. }
            | Self::Mix { ty, .. }
            | Self::SmoothStep { ty, .. }
            | Self::Step { ty, .. }
            | Self::Min { ty, .. }
            | Self::Max { ty, .. }
            | Self::Pow { ty, .. }
            | Self::Atan2 { ty, .. }
            | Self::Reflect { ty, .. }
            | Self::Refract { ty, .. }
            | Self::FaceForward { ty, .. }
            | Self::Distance { ty, .. }
            | Self::OuterProduct { ty, .. }
            | Self::MatrixTimesVector { ty, .. }
            | Self::VectorTimesMatrix { ty, .. }
            | Self::MatrixTimesMatrix { ty, .. }
            | Self::MatrixTimesScalar { ty, .. }
            | Self::VectorTimesScalar { ty, .. }
            | Self::Bitcast { ty, .. }
            | Self::ConvertSToF { ty, .. }
            | Self::ConvertUToF { ty, .. }
            | Self::ConvertFToS { ty, .. }
            | Self::ConvertFToU { ty, .. }
            | Self::SConvert { ty, .. }
            | Self::UConvert { ty, .. }
            | Self::FConvert { ty, .. }
            | Self::QuantizeToF16 { ty, .. }
            | Self::CompositeConstruct { ty, .. }
            | Self::CompositeExtract { ty, .. }
            | Self::CompositeInsert { ty, .. }
            | Self::VectorShuffle { ty, .. }
            | Self::VectorExtractDynamic { ty, .. }
            | Self::VectorInsertDynamic { ty, .. }
            | Self::CopyObject { ty, .. }
            | Self::FunctionCall { ty, .. }
            | Self::Phi { ty, .. }
            | Self::Select { ty, .. }
            | Self::ImageSampleImplicitLod { ty, .. }
            | Self::ImageSampleExplicitLod { ty, .. }
            | Self::ImageSampleDrefImplicitLod { ty, .. }
            | Self::ImageSampleDrefExplicitLod { ty, .. }
            | Self::ImageSampleProjImplicitLod { ty, .. }
            | Self::ImageSampleProjExplicitLod { ty, .. }
            | Self::ImageFetch { ty, .. }
            | Self::ImageRead { ty, .. }
            | Self::ImageGather { ty, .. }
            | Self::ImageDrefGather { ty, .. }
            | Self::ImageQuerySize { ty, .. }
            | Self::ImageQuerySizeLod { ty, .. }
            | Self::ImageQueryLevels { ty, .. }
            | Self::ImageQuerySamples { ty, .. }
            | Self::ImageQueryLod { ty, .. }
            | Self::Image { ty, .. }
            | Self::SampledImage { ty, .. }
            | Self::ImageSparseSampleImplicitLod { ty, .. }
            | Self::AtomicLoad { ty, .. }
            | Self::AtomicExchange { ty, .. }
            | Self::AtomicCompareExchange { ty, .. }
            | Self::AtomicIIncrement { ty, .. }
            | Self::AtomicIDecrement { ty, .. }
            | Self::AtomicIAdd { ty, .. }
            | Self::AtomicISub { ty, .. }
            | Self::AtomicSMin { ty, .. }
            | Self::AtomicUMin { ty, .. }
            | Self::AtomicSMax { ty, .. }
            | Self::AtomicUMax { ty, .. }
            | Self::AtomicAnd { ty, .. }
            | Self::AtomicOr { ty, .. }
            | Self::AtomicXor { ty, .. }
            | Self::AtomicFAdd { ty, .. }
            | Self::AtomicFMin { ty, .. }
            | Self::AtomicFMax { ty, .. }
            | Self::GroupBroadcast { ty, .. }
            | Self::GroupNonUniformAllEqual { ty, .. }
            | Self::GroupNonUniformBroadcast { ty, .. }
            | Self::GroupNonUniformBroadcastFirst { ty, .. }
            | Self::GroupNonUniformShuffle { ty, .. }
            | Self::GroupNonUniformShuffleXor { ty, .. }
            | Self::GroupNonUniformShuffleUp { ty, .. }
            | Self::GroupNonUniformShuffleDown { ty, .. }
            | Self::GroupNonUniformIAdd { ty, .. }
            | Self::GroupNonUniformFAdd { ty, .. }
            | Self::GroupNonUniformIMul { ty, .. }
            | Self::GroupNonUniformFMul { ty, .. }
            | Self::GroupNonUniformSMin { ty, .. }
            | Self::GroupNonUniformSMax { ty, .. }
            | Self::GroupNonUniformBitwiseAnd { ty, .. }
            | Self::GroupNonUniformBitwiseOr { ty, .. }
            | Self::GroupNonUniformBitwiseXor { ty, .. }
            | Self::GroupNonUniformQuadBroadcast { ty, .. }
            | Self::GroupNonUniformQuadSwap { ty, .. }
            | Self::Undef { ty, .. }
            | Self::ExtInst { ty, .. }
            | Self::Expect { ty, .. } => Some(ty),
            
            _ => None,
        }
    }

    /// Check if this is a terminator instruction
    pub fn is_terminator(&self) -> bool {
        matches!(
            self,
            Self::Branch { .. }
                | Self::BranchConditional { .. }
                | Self::Switch { .. }
                | Self::Return
                | Self::ReturnValue { .. }
                | Self::Kill
                | Self::TerminateInvocation
                | Self::Unreachable
                | Self::TerminateRay
                | Self::IgnoreIntersection
        )
    }

    /// Check if this instruction has side effects
    pub fn has_side_effects(&self) -> bool {
        matches!(
            self,
            Self::Store { .. }
                | Self::CopyMemory { .. }
                | Self::CopyMemorySized { .. }
                | Self::ImageWrite { .. }
                | Self::AtomicStore { .. }
                | Self::AtomicExchange { .. }
                | Self::AtomicCompareExchange { .. }
                | Self::AtomicIIncrement { .. }
                | Self::AtomicIDecrement { .. }
                | Self::AtomicIAdd { .. }
                | Self::AtomicISub { .. }
                | Self::AtomicSMin { .. }
                | Self::AtomicUMin { .. }
                | Self::AtomicSMax { .. }
                | Self::AtomicUMax { .. }
                | Self::AtomicAnd { .. }
                | Self::AtomicOr { .. }
                | Self::AtomicXor { .. }
                | Self::AtomicFAdd { .. }
                | Self::AtomicFMin { .. }
                | Self::AtomicFMax { .. }
                | Self::MemoryBarrier { .. }
                | Self::ControlBarrier { .. }
                | Self::TraceRay { .. }
                | Self::ExecuteCallable { .. }
                | Self::FunctionCall { .. }
                | Self::Kill
                | Self::TerminateInvocation
                | Self::DemoteToHelperInvocation
                | Self::DebugPrintf { .. }
                | Self::SetMeshOutputs { .. }
                | Self::EmitMeshVertex { .. }
                | Self::EmitMeshPrimitive { .. }
                | Self::WritePrimitiveIndices { .. }
        )
    }

    /// Check if this instruction is a memory operation
    pub fn is_memory_op(&self) -> bool {
        matches!(
            self,
            Self::Load { .. }
                | Self::Store { .. }
                | Self::AccessChain { .. }
                | Self::PtrAccessChain { .. }
                | Self::CopyMemory { .. }
                | Self::CopyMemorySized { .. }
                | Self::AtomicLoad { .. }
                | Self::AtomicStore { .. }
                | Self::AtomicExchange { .. }
                | Self::AtomicCompareExchange { .. }
                | Self::AtomicIIncrement { .. }
                | Self::AtomicIDecrement { .. }
                | Self::AtomicIAdd { .. }
                | Self::AtomicISub { .. }
                | Self::AtomicSMin { .. }
                | Self::AtomicUMin { .. }
                | Self::AtomicSMax { .. }
                | Self::AtomicUMax { .. }
                | Self::AtomicAnd { .. }
                | Self::AtomicOr { .. }
                | Self::AtomicXor { .. }
                | Self::AtomicFAdd { .. }
                | Self::AtomicFMin { .. }
                | Self::AtomicFMax { .. }
        )
    }

    /// Check if this instruction is an image operation
    pub fn is_image_op(&self) -> bool {
        matches!(
            self,
            Self::ImageSampleImplicitLod { .. }
                | Self::ImageSampleExplicitLod { .. }
                | Self::ImageSampleDrefImplicitLod { .. }
                | Self::ImageSampleDrefExplicitLod { .. }
                | Self::ImageSampleProjImplicitLod { .. }
                | Self::ImageSampleProjExplicitLod { .. }
                | Self::ImageFetch { .. }
                | Self::ImageRead { .. }
                | Self::ImageWrite { .. }
                | Self::ImageGather { .. }
                | Self::ImageDrefGather { .. }
                | Self::ImageQuerySize { .. }
                | Self::ImageQuerySizeLod { .. }
                | Self::ImageQueryLevels { .. }
                | Self::ImageQuerySamples { .. }
                | Self::ImageQueryLod { .. }
                | Self::Image { .. }
                | Self::SampledImage { .. }
                | Self::ImageSparseSampleImplicitLod { .. }
                | Self::ImageSparseTexelsResident { .. }
        )
    }

    /// Check if this instruction is a control flow instruction
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Self::Branch { .. }
                | Self::BranchConditional { .. }
                | Self::Switch { .. }
                | Self::Return
                | Self::ReturnValue { .. }
                | Self::FunctionCall { .. }
                | Self::Kill
                | Self::TerminateInvocation
                | Self::Unreachable
                | Self::LoopMerge { .. }
                | Self::SelectionMerge { .. }
                | Self::Label { .. }
                | Self::Phi { .. }
        )
    }

    /// Get operand values used by this instruction
    pub fn operands(&self) -> Vec<ValueId> {
        let mut ops = Vec::new();
        
        match self {
            Self::Load { pointer, .. } => ops.push(*pointer),
            Self::Store { pointer, value, .. } => {
                ops.push(*pointer);
                ops.push(*value);
            }
            Self::AccessChain { base, indices, .. } => {
                ops.push(*base);
                ops.extend(indices.iter().copied());
            }
            Self::BinaryOp { left, right, .. } => {
                ops.push(*left);
                ops.push(*right);
            }
            Self::UnaryOp { operand, .. } => ops.push(*operand),
            Self::Phi { operands, .. } => {
                for (val, _) in operands {
                    ops.push(*val);
                }
            }
            Self::Select { condition, true_value, false_value, .. } => {
                ops.push(*condition);
                ops.push(*true_value);
                ops.push(*false_value);
            }
            Self::FunctionCall { arguments, .. } => ops.extend(arguments.iter().copied()),
            Self::CompositeConstruct { components, .. } => ops.extend(components.iter().copied()),
            Self::CompositeExtract { composite, .. } => ops.push(*composite),
            Self::BranchConditional { condition, .. } => ops.push(*condition),
            Self::ReturnValue { value } => ops.push(*value),
            // Add more cases as needed
            _ => {}
        }
        
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_op_categories() {
        assert!(BinaryOp::Add.is_arithmetic());
        assert!(BinaryOp::Equal.is_comparison());
        assert!(BinaryOp::LogicalAnd.is_logical());
        assert!(BinaryOp::BitwiseAnd.is_bitwise());
    }

    #[test]
    fn test_instruction_terminator() {
        assert!(Instruction::Return.is_terminator());
        assert!(Instruction::Branch { target: 0 }.is_terminator());
        assert!(!Instruction::Nop.is_terminator());
    }

    #[test]
    fn test_instruction_side_effects() {
        let store = Instruction::Store {
            pointer: 0,
            value: 1,
            access: MemoryAccess::default(),
        };
        assert!(store.has_side_effects());
        assert!(!Instruction::Nop.has_side_effects());
    }
}
