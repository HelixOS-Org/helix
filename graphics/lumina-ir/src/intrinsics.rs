//! Shader Intrinsics
//!
//! Built-in shader intrinsic functions.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec, vec::Vec};

use crate::types::{IrType, ScalarType, VectorSize};

/// Intrinsic function category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntrinsicCategory {
    /// Mathematical functions
    Math,
    /// Trigonometric functions
    Trigonometry,
    /// Exponential functions
    Exponential,
    /// Common functions
    Common,
    /// Geometric functions
    Geometric,
    /// Matrix functions
    Matrix,
    /// Vector relational
    Relational,
    /// Integer functions
    Integer,
    /// Floating-point functions
    Float,
    /// Derivative functions
    Derivative,
    /// Interpolation functions
    Interpolation,
    /// Image functions
    Image,
    /// Atomic functions
    Atomic,
    /// Barrier functions
    Barrier,
    /// Group operations
    Group,
    /// Subgroup operations
    Subgroup,
    /// Ray tracing
    RayTracing,
    /// Mesh shader
    MeshShader,
    /// Debug
    Debug,
}

/// Built-in intrinsic function
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intrinsic {
    /// Name of the intrinsic
    pub name: &'static str,
    /// Category
    pub category: IntrinsicCategory,
    /// Parameter types (None = generic)
    pub params: &'static [ParamType],
    /// Return type
    pub return_type: ReturnType,
    /// GLSL function name
    pub glsl_name: Option<&'static str>,
    /// HLSL function name
    pub hlsl_name: Option<&'static str>,
    /// SPIR-V extended instruction set
    pub spirv_set: Option<&'static str>,
    /// SPIR-V opcode or extended instruction
    pub spirv_op: Option<u32>,
}

/// Parameter type specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    /// Same as first parameter
    SameAsFirst,
    /// Same as return type
    SameAsReturn,
    /// Scalar of same type as first
    ScalarOfFirst,
    /// Specific type
    Specific(IrType),
    /// Generic float (f32 or f64)
    GenFloat,
    /// Generic int (i32 or i64)
    GenInt,
    /// Generic uint (u32 or u64)
    GenUInt,
    /// Generic float vector
    GenFloatVec,
    /// Generic int vector
    GenIntVec,
    /// Float scalar
    F32,
    /// Double scalar
    F64,
    /// Int scalar
    I32,
    /// UInt scalar
    U32,
    /// Bool
    Bool,
    /// Pointer
    Pointer,
    /// Sampled image
    SampledImage,
    /// Image
    Image,
    /// Sampler
    Sampler,
}

/// Return type specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnType {
    /// Same as first parameter
    SameAsFirst,
    /// Same as second parameter
    SameAsSecond,
    /// Scalar type of first parameter
    ScalarOfFirst,
    /// Void
    Void,
    /// Specific type
    Specific(IrType),
    /// Bool
    Bool,
    /// Bool vector of same size as first
    BoolVecOfFirst,
    /// Int type
    I32,
    /// UInt type
    U32,
    /// Float type
    F32,
    /// Vec4 of first's scalar
    Vec4OfFirst,
}

impl Intrinsic {
    /// Get all standard intrinsics
    pub fn all() -> &'static [Intrinsic] {
        INTRINSICS
    }

    /// Find intrinsic by name
    pub fn by_name(name: &str) -> Option<&'static Intrinsic> {
        INTRINSICS.iter().find(|i| i.name == name)
    }

    /// Get intrinsics in a category
    pub fn in_category(category: IntrinsicCategory) -> impl Iterator<Item = &'static Intrinsic> {
        INTRINSICS.iter().filter(move |i| i.category == category)
    }
}

/// All standard intrinsics
static INTRINSICS: &[Intrinsic] = &[
    // ========== Trigonometric Functions ==========
    Intrinsic {
        name: "sin",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("sin"),
        hlsl_name: Some("sin"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(13), // Sin
    },
    Intrinsic {
        name: "cos",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("cos"),
        hlsl_name: Some("cos"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(14), // Cos
    },
    Intrinsic {
        name: "tan",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("tan"),
        hlsl_name: Some("tan"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(15), // Tan
    },
    Intrinsic {
        name: "asin",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("asin"),
        hlsl_name: Some("asin"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(16), // Asin
    },
    Intrinsic {
        name: "acos",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("acos"),
        hlsl_name: Some("acos"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(17), // Acos
    },
    Intrinsic {
        name: "atan",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("atan"),
        hlsl_name: Some("atan"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(18), // Atan
    },
    Intrinsic {
        name: "atan2",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("atan"),
        hlsl_name: Some("atan2"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(25), // Atan2
    },
    Intrinsic {
        name: "sinh",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("sinh"),
        hlsl_name: Some("sinh"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(19), // Sinh
    },
    Intrinsic {
        name: "cosh",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("cosh"),
        hlsl_name: Some("cosh"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(20), // Cosh
    },
    Intrinsic {
        name: "tanh",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("tanh"),
        hlsl_name: Some("tanh"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(21), // Tanh
    },
    Intrinsic {
        name: "asinh",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("asinh"),
        hlsl_name: None,
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(22), // Asinh
    },
    Intrinsic {
        name: "acosh",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("acosh"),
        hlsl_name: None,
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(23), // Acosh
    },
    Intrinsic {
        name: "atanh",
        category: IntrinsicCategory::Trigonometry,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("atanh"),
        hlsl_name: None,
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(24), // Atanh
    },
    // ========== Exponential Functions ==========
    Intrinsic {
        name: "pow",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("pow"),
        hlsl_name: Some("pow"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(26), // Pow
    },
    Intrinsic {
        name: "exp",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("exp"),
        hlsl_name: Some("exp"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(27), // Exp
    },
    Intrinsic {
        name: "exp2",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("exp2"),
        hlsl_name: Some("exp2"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(29), // Exp2
    },
    Intrinsic {
        name: "log",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("log"),
        hlsl_name: Some("log"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(28), // Log
    },
    Intrinsic {
        name: "log2",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("log2"),
        hlsl_name: Some("log2"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(30), // Log2
    },
    Intrinsic {
        name: "sqrt",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("sqrt"),
        hlsl_name: Some("sqrt"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(31), // Sqrt
    },
    Intrinsic {
        name: "inversesqrt",
        category: IntrinsicCategory::Exponential,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("inversesqrt"),
        hlsl_name: Some("rsqrt"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(32), // InverseSqrt
    },
    // ========== Common Functions ==========
    Intrinsic {
        name: "abs",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("abs"),
        hlsl_name: Some("abs"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(4), // FAbs
    },
    Intrinsic {
        name: "sign",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("sign"),
        hlsl_name: Some("sign"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(6), // FSign
    },
    Intrinsic {
        name: "floor",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("floor"),
        hlsl_name: Some("floor"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(8), // Floor
    },
    Intrinsic {
        name: "ceil",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("ceil"),
        hlsl_name: Some("ceil"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(9), // Ceil
    },
    Intrinsic {
        name: "round",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("round"),
        hlsl_name: Some("round"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(1), // Round
    },
    Intrinsic {
        name: "trunc",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("trunc"),
        hlsl_name: Some("trunc"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(3), // Trunc
    },
    Intrinsic {
        name: "fract",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("fract"),
        hlsl_name: Some("frac"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(10), // Fract
    },
    Intrinsic {
        name: "mod",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("mod"),
        hlsl_name: Some("fmod"),
        spirv_set: None,
        spirv_op: Some(141), // FMod
    },
    Intrinsic {
        name: "min",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("min"),
        hlsl_name: Some("min"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(37), // FMin
    },
    Intrinsic {
        name: "max",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("max"),
        hlsl_name: Some("max"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(40), // FMax
    },
    Intrinsic {
        name: "clamp",
        category: IntrinsicCategory::Common,
        params: &[
            ParamType::GenFloat,
            ParamType::SameAsFirst,
            ParamType::SameAsFirst,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("clamp"),
        hlsl_name: Some("clamp"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(43), // FClamp
    },
    Intrinsic {
        name: "saturate",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: None,
        hlsl_name: Some("saturate"),
        spirv_set: None,
        spirv_op: None,
    },
    Intrinsic {
        name: "mix",
        category: IntrinsicCategory::Common,
        params: &[
            ParamType::GenFloat,
            ParamType::SameAsFirst,
            ParamType::SameAsFirst,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("mix"),
        hlsl_name: Some("lerp"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(46), // FMix
    },
    Intrinsic {
        name: "step",
        category: IntrinsicCategory::Common,
        params: &[ParamType::GenFloat, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("step"),
        hlsl_name: Some("step"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(48), // Step
    },
    Intrinsic {
        name: "smoothstep",
        category: IntrinsicCategory::Common,
        params: &[
            ParamType::GenFloat,
            ParamType::SameAsFirst,
            ParamType::SameAsFirst,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("smoothstep"),
        hlsl_name: Some("smoothstep"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(49), // SmoothStep
    },
    Intrinsic {
        name: "fma",
        category: IntrinsicCategory::Common,
        params: &[
            ParamType::GenFloat,
            ParamType::SameAsFirst,
            ParamType::SameAsFirst,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("fma"),
        hlsl_name: Some("mad"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(50), // Fma
    },
    // ========== Geometric Functions ==========
    Intrinsic {
        name: "length",
        category: IntrinsicCategory::Geometric,
        params: &[ParamType::GenFloatVec],
        return_type: ReturnType::ScalarOfFirst,
        glsl_name: Some("length"),
        hlsl_name: Some("length"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(66), // Length
    },
    Intrinsic {
        name: "distance",
        category: IntrinsicCategory::Geometric,
        params: &[ParamType::GenFloatVec, ParamType::SameAsFirst],
        return_type: ReturnType::ScalarOfFirst,
        glsl_name: Some("distance"),
        hlsl_name: Some("distance"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(67), // Distance
    },
    Intrinsic {
        name: "dot",
        category: IntrinsicCategory::Geometric,
        params: &[ParamType::GenFloatVec, ParamType::SameAsFirst],
        return_type: ReturnType::ScalarOfFirst,
        glsl_name: Some("dot"),
        hlsl_name: Some("dot"),
        spirv_set: None,
        spirv_op: Some(148), // OpDot
    },
    Intrinsic {
        name: "cross",
        category: IntrinsicCategory::Geometric,
        params: &[ParamType::GenFloatVec, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("cross"),
        hlsl_name: Some("cross"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(68), // Cross
    },
    Intrinsic {
        name: "normalize",
        category: IntrinsicCategory::Geometric,
        params: &[ParamType::GenFloatVec],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("normalize"),
        hlsl_name: Some("normalize"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(69), // Normalize
    },
    Intrinsic {
        name: "faceforward",
        category: IntrinsicCategory::Geometric,
        params: &[
            ParamType::GenFloatVec,
            ParamType::SameAsFirst,
            ParamType::SameAsFirst,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("faceforward"),
        hlsl_name: Some("faceforward"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(70), // FaceForward
    },
    Intrinsic {
        name: "reflect",
        category: IntrinsicCategory::Geometric,
        params: &[ParamType::GenFloatVec, ParamType::SameAsFirst],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("reflect"),
        hlsl_name: Some("reflect"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(71), // Reflect
    },
    Intrinsic {
        name: "refract",
        category: IntrinsicCategory::Geometric,
        params: &[
            ParamType::GenFloatVec,
            ParamType::SameAsFirst,
            ParamType::ScalarOfFirst,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("refract"),
        hlsl_name: Some("refract"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(72), // Refract
    },
    // ========== Matrix Functions ==========
    Intrinsic {
        name: "transpose",
        category: IntrinsicCategory::Matrix,
        params: &[ParamType::GenFloat],       // Matrix type
        return_type: ReturnType::SameAsFirst, // Transposed
        glsl_name: Some("transpose"),
        hlsl_name: Some("transpose"),
        spirv_set: None,
        spirv_op: Some(84), // OpTranspose
    },
    Intrinsic {
        name: "determinant",
        category: IntrinsicCategory::Matrix,
        params: &[ParamType::GenFloat], // Matrix type
        return_type: ReturnType::ScalarOfFirst,
        glsl_name: Some("determinant"),
        hlsl_name: Some("determinant"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(33), // Determinant
    },
    Intrinsic {
        name: "inverse",
        category: IntrinsicCategory::Matrix,
        params: &[ParamType::GenFloat], // Matrix type
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("inverse"),
        hlsl_name: None,
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(34), // MatrixInverse
    },
    // ========== Derivative Functions ==========
    Intrinsic {
        name: "dFdx",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("dFdx"),
        hlsl_name: Some("ddx"),
        spirv_set: None,
        spirv_op: Some(207), // OpDPdx
    },
    Intrinsic {
        name: "dFdy",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("dFdy"),
        hlsl_name: Some("ddy"),
        spirv_set: None,
        spirv_op: Some(208), // OpDPdy
    },
    Intrinsic {
        name: "fwidth",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("fwidth"),
        hlsl_name: Some("fwidth"),
        spirv_set: None,
        spirv_op: Some(209), // OpFwidth
    },
    Intrinsic {
        name: "dFdxFine",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("dFdxFine"),
        hlsl_name: Some("ddx_fine"),
        spirv_set: None,
        spirv_op: Some(210), // OpDPdxFine
    },
    Intrinsic {
        name: "dFdyFine",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("dFdyFine"),
        hlsl_name: Some("ddy_fine"),
        spirv_set: None,
        spirv_op: Some(211), // OpDPdyFine
    },
    Intrinsic {
        name: "dFdxCoarse",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("dFdxCoarse"),
        hlsl_name: Some("ddx_coarse"),
        spirv_set: None,
        spirv_op: Some(213), // OpDPdxCoarse
    },
    Intrinsic {
        name: "dFdyCoarse",
        category: IntrinsicCategory::Derivative,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("dFdyCoarse"),
        hlsl_name: Some("ddy_coarse"),
        spirv_set: None,
        spirv_op: Some(214), // OpDPdyCoarse
    },
    // ========== Integer Functions ==========
    Intrinsic {
        name: "bitfieldExtract",
        category: IntrinsicCategory::Integer,
        params: &[ParamType::GenInt, ParamType::I32, ParamType::I32],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("bitfieldExtract"),
        hlsl_name: None,
        spirv_set: None,
        spirv_op: Some(201), // OpBitFieldSExtract
    },
    Intrinsic {
        name: "bitfieldInsert",
        category: IntrinsicCategory::Integer,
        params: &[
            ParamType::GenInt,
            ParamType::SameAsFirst,
            ParamType::I32,
            ParamType::I32,
        ],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("bitfieldInsert"),
        hlsl_name: None,
        spirv_set: None,
        spirv_op: Some(202), // OpBitFieldInsert
    },
    Intrinsic {
        name: "bitfieldReverse",
        category: IntrinsicCategory::Integer,
        params: &[ParamType::GenInt],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("bitfieldReverse"),
        hlsl_name: Some("reversebits"),
        spirv_set: None,
        spirv_op: Some(204), // OpBitReverse
    },
    Intrinsic {
        name: "bitCount",
        category: IntrinsicCategory::Integer,
        params: &[ParamType::GenInt],
        return_type: ReturnType::I32,
        glsl_name: Some("bitCount"),
        hlsl_name: Some("countbits"),
        spirv_set: None,
        spirv_op: Some(205), // OpBitCount
    },
    Intrinsic {
        name: "findLSB",
        category: IntrinsicCategory::Integer,
        params: &[ParamType::GenInt],
        return_type: ReturnType::I32,
        glsl_name: Some("findLSB"),
        hlsl_name: Some("firstbitlow"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(73), // FindILsb
    },
    Intrinsic {
        name: "findMSB",
        category: IntrinsicCategory::Integer,
        params: &[ParamType::GenInt],
        return_type: ReturnType::I32,
        glsl_name: Some("findMSB"),
        hlsl_name: Some("firstbithigh"),
        spirv_set: Some("GLSL.std.450"),
        spirv_op: Some(74), // FindSMsb
    },
    // ========== Barrier Functions ==========
    Intrinsic {
        name: "barrier",
        category: IntrinsicCategory::Barrier,
        params: &[],
        return_type: ReturnType::Void,
        glsl_name: Some("barrier"),
        hlsl_name: Some("GroupMemoryBarrierWithGroupSync"),
        spirv_set: None,
        spirv_op: Some(224), // OpControlBarrier
    },
    Intrinsic {
        name: "memoryBarrier",
        category: IntrinsicCategory::Barrier,
        params: &[],
        return_type: ReturnType::Void,
        glsl_name: Some("memoryBarrier"),
        hlsl_name: Some("DeviceMemoryBarrier"),
        spirv_set: None,
        spirv_op: Some(225), // OpMemoryBarrier
    },
    Intrinsic {
        name: "memoryBarrierShared",
        category: IntrinsicCategory::Barrier,
        params: &[],
        return_type: ReturnType::Void,
        glsl_name: Some("memoryBarrierShared"),
        hlsl_name: Some("GroupMemoryBarrier"),
        spirv_set: None,
        spirv_op: Some(225), // OpMemoryBarrier
    },
    Intrinsic {
        name: "groupMemoryBarrier",
        category: IntrinsicCategory::Barrier,
        params: &[],
        return_type: ReturnType::Void,
        glsl_name: Some("groupMemoryBarrier"),
        hlsl_name: Some("AllMemoryBarrier"),
        spirv_set: None,
        spirv_op: Some(225), // OpMemoryBarrier
    },
    // ========== Subgroup Operations ==========
    Intrinsic {
        name: "subgroupElect",
        category: IntrinsicCategory::Subgroup,
        params: &[],
        return_type: ReturnType::Bool,
        glsl_name: Some("subgroupElect"),
        hlsl_name: Some("WaveIsFirstLane"),
        spirv_set: None,
        spirv_op: Some(333), // OpGroupNonUniformElect
    },
    Intrinsic {
        name: "subgroupBroadcast",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat, ParamType::U32],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupBroadcast"),
        hlsl_name: Some("WaveReadLaneAt"),
        spirv_set: None,
        spirv_op: Some(335), // OpGroupNonUniformBroadcast
    },
    Intrinsic {
        name: "subgroupBroadcastFirst",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupBroadcastFirst"),
        hlsl_name: Some("WaveReadLaneFirst"),
        spirv_set: None,
        spirv_op: Some(338), // OpGroupNonUniformBroadcastFirst
    },
    Intrinsic {
        name: "subgroupBallot",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::Bool],
        return_type: ReturnType::Vec4OfFirst,
        glsl_name: Some("subgroupBallot"),
        hlsl_name: Some("WaveActiveBallot"),
        spirv_set: None,
        spirv_op: Some(339), // OpGroupNonUniformBallot
    },
    Intrinsic {
        name: "subgroupAll",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::Bool],
        return_type: ReturnType::Bool,
        glsl_name: Some("subgroupAll"),
        hlsl_name: Some("WaveActiveAllTrue"),
        spirv_set: None,
        spirv_op: Some(334), // OpGroupNonUniformAll
    },
    Intrinsic {
        name: "subgroupAny",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::Bool],
        return_type: ReturnType::Bool,
        glsl_name: Some("subgroupAny"),
        hlsl_name: Some("WaveActiveAnyTrue"),
        spirv_set: None,
        spirv_op: Some(337), // OpGroupNonUniformAny
    },
    Intrinsic {
        name: "subgroupAdd",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupAdd"),
        hlsl_name: Some("WaveActiveSum"),
        spirv_set: None,
        spirv_op: Some(350), // OpGroupNonUniformFAdd
    },
    Intrinsic {
        name: "subgroupMul",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupMul"),
        hlsl_name: Some("WaveActiveProduct"),
        spirv_set: None,
        spirv_op: Some(351), // OpGroupNonUniformFMul
    },
    Intrinsic {
        name: "subgroupMin",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupMin"),
        hlsl_name: Some("WaveActiveMin"),
        spirv_set: None,
        spirv_op: Some(352), // OpGroupNonUniformFMin
    },
    Intrinsic {
        name: "subgroupMax",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupMax"),
        hlsl_name: Some("WaveActiveMax"),
        spirv_set: None,
        spirv_op: Some(353), // OpGroupNonUniformFMax
    },
    Intrinsic {
        name: "subgroupShuffle",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat, ParamType::U32],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupShuffle"),
        hlsl_name: Some("WaveReadLaneAt"),
        spirv_set: None,
        spirv_op: Some(345), // OpGroupNonUniformShuffle
    },
    Intrinsic {
        name: "subgroupShuffleXor",
        category: IntrinsicCategory::Subgroup,
        params: &[ParamType::GenFloat, ParamType::U32],
        return_type: ReturnType::SameAsFirst,
        glsl_name: Some("subgroupShuffleXor"),
        hlsl_name: None,
        spirv_set: None,
        spirv_op: Some(346), // OpGroupNonUniformShuffleXor
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_intrinsic() {
        let sin = Intrinsic::by_name("sin");
        assert!(sin.is_some());
        let sin = sin.unwrap();
        assert_eq!(sin.category, IntrinsicCategory::Trigonometry);
        assert_eq!(sin.glsl_name, Some("sin"));
    }

    #[test]
    fn test_intrinsic_category() {
        let trig: Vec<_> = Intrinsic::in_category(IntrinsicCategory::Trigonometry).collect();
        assert!(!trig.is_empty());
        assert!(trig.iter().any(|i| i.name == "sin"));
        assert!(trig.iter().any(|i| i.name == "cos"));
    }

    #[test]
    fn test_all_intrinsics() {
        let all = Intrinsic::all();
        assert!(all.len() > 50);
    }
}
