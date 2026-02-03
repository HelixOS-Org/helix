//! Value and Constant Representations
//!
//! This module defines values and constants in the Lumina IR.

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

use core::fmt;
use crate::types::{IrType, ScalarType};

/// Value identifier
pub type ValueId = u32;

/// Constant value representation
#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    /// Boolean constant
    Bool(bool),
    /// 8-bit signed integer
    Int8(i8),
    /// 8-bit unsigned integer
    UInt8(u8),
    /// 16-bit signed integer
    Int16(i16),
    /// 16-bit unsigned integer
    UInt16(u16),
    /// 32-bit signed integer
    Int32(i32),
    /// 32-bit unsigned integer
    UInt32(u32),
    /// 64-bit signed integer
    Int64(i64),
    /// 64-bit unsigned integer
    UInt64(u64),
    /// 16-bit floating point
    Float16(u16), // Stored as bits
    /// 32-bit floating point
    Float32(f32),
    /// 64-bit floating point
    Float64(f64),
    /// Vector constant
    Vector(Vec<ConstantValue>),
    /// Matrix constant (column-major)
    Matrix(Vec<ConstantValue>),
    /// Array constant
    Array(Vec<ConstantValue>),
    /// Struct constant
    Struct(Vec<ConstantValue>),
    /// Null/zero constant
    Null,
    /// Composite with all same value
    Splat(Box<ConstantValue>, u32),
    /// Undefined value (for dead code)
    Undef,
}

impl ConstantValue {
    /// Create a boolean constant
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Create an i32 constant
    pub const fn i32(value: i32) -> Self {
        Self::Int32(value)
    }

    /// Create a u32 constant
    pub const fn u32(value: u32) -> Self {
        Self::UInt32(value)
    }

    /// Create an f32 constant
    pub fn f32(value: f32) -> Self {
        Self::Float32(value)
    }

    /// Create an f64 constant
    pub fn f64(value: f64) -> Self {
        Self::Float64(value)
    }

    /// Create a zero constant for the given type
    pub fn zero(ty: &IrType) -> Self {
        match ty {
            IrType::Scalar(ScalarType::Bool) => Self::Bool(false),
            IrType::Scalar(ScalarType::Int8) => Self::Int8(0),
            IrType::Scalar(ScalarType::UInt8) => Self::UInt8(0),
            IrType::Scalar(ScalarType::Int16) => Self::Int16(0),
            IrType::Scalar(ScalarType::UInt16) => Self::UInt16(0),
            IrType::Scalar(ScalarType::Int32) => Self::Int32(0),
            IrType::Scalar(ScalarType::UInt32) => Self::UInt32(0),
            IrType::Scalar(ScalarType::Int64) => Self::Int64(0),
            IrType::Scalar(ScalarType::UInt64) => Self::UInt64(0),
            IrType::Scalar(ScalarType::Float16) => Self::Float16(0),
            IrType::Scalar(ScalarType::Float32) => Self::Float32(0.0),
            IrType::Scalar(ScalarType::Float64) => Self::Float64(0.0),
            IrType::Scalar(ScalarType::Void) => Self::Null,
            IrType::Vector { element, size } => {
                let elem = Self::zero(&IrType::Scalar(*element));
                Self::Vector(vec![elem; size.count() as usize])
            }
            IrType::Matrix { element, size } => {
                let elem = Self::zero(&IrType::Scalar(*element));
                Self::Matrix(vec![elem; size.component_count() as usize])
            }
            IrType::Array(arr) => {
                if let Some(len) = arr.length {
                    let elem = Self::zero(&arr.element);
                    Self::Array(vec![elem; len as usize])
                } else {
                    Self::Array(Vec::new())
                }
            }
            IrType::Struct(s) => {
                let fields = s.fields.iter().map(|f| Self::zero(&f.ty)).collect();
                Self::Struct(fields)
            }
            _ => Self::Null,
        }
    }

    /// Create a one constant for the given type
    pub fn one(ty: &IrType) -> Option<Self> {
        match ty {
            IrType::Scalar(ScalarType::Bool) => Some(Self::Bool(true)),
            IrType::Scalar(ScalarType::Int8) => Some(Self::Int8(1)),
            IrType::Scalar(ScalarType::UInt8) => Some(Self::UInt8(1)),
            IrType::Scalar(ScalarType::Int16) => Some(Self::Int16(1)),
            IrType::Scalar(ScalarType::UInt16) => Some(Self::UInt16(1)),
            IrType::Scalar(ScalarType::Int32) => Some(Self::Int32(1)),
            IrType::Scalar(ScalarType::UInt32) => Some(Self::UInt32(1)),
            IrType::Scalar(ScalarType::Int64) => Some(Self::Int64(1)),
            IrType::Scalar(ScalarType::UInt64) => Some(Self::UInt64(1)),
            IrType::Scalar(ScalarType::Float16) => Some(Self::Float16(0x3C00)), // 1.0 in f16
            IrType::Scalar(ScalarType::Float32) => Some(Self::Float32(1.0)),
            IrType::Scalar(ScalarType::Float64) => Some(Self::Float64(1.0)),
            _ => None,
        }
    }

    /// Check if this is a zero constant
    pub fn is_zero(&self) -> bool {
        match self {
            Self::Bool(false) => true,
            Self::Int8(0) | Self::UInt8(0) => true,
            Self::Int16(0) | Self::UInt16(0) => true,
            Self::Int32(0) | Self::UInt32(0) => true,
            Self::Int64(0) | Self::UInt64(0) => true,
            Self::Float16(0) => true,
            Self::Float32(v) => *v == 0.0,
            Self::Float64(v) => *v == 0.0,
            Self::Null => true,
            Self::Vector(v) | Self::Matrix(v) | Self::Array(v) | Self::Struct(v) => {
                v.iter().all(|c| c.is_zero())
            }
            Self::Splat(v, _) => v.is_zero(),
            _ => false,
        }
    }

    /// Check if this is a one constant
    pub fn is_one(&self) -> bool {
        match self {
            Self::Bool(true) => true,
            Self::Int8(1) | Self::UInt8(1) => true,
            Self::Int16(1) | Self::UInt16(1) => true,
            Self::Int32(1) | Self::UInt32(1) => true,
            Self::Int64(1) | Self::UInt64(1) => true,
            Self::Float16(0x3C00) => true,
            Self::Float32(v) => *v == 1.0,
            Self::Float64(v) => *v == 1.0,
            _ => false,
        }
    }

    /// Check if this is a negative one constant
    pub fn is_neg_one(&self) -> bool {
        match self {
            Self::Int8(-1) => true,
            Self::Int16(-1) => true,
            Self::Int32(-1) => true,
            Self::Int64(-1) => true,
            Self::Float32(v) => *v == -1.0,
            Self::Float64(v) => *v == -1.0,
            _ => false,
        }
    }

    /// Try to get as i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Bool(v) => Some(*v as i64),
            Self::Int8(v) => Some(*v as i64),
            Self::UInt8(v) => Some(*v as i64),
            Self::Int16(v) => Some(*v as i64),
            Self::UInt16(v) => Some(*v as i64),
            Self::Int32(v) => Some(*v as i64),
            Self::UInt32(v) => Some(*v as i64),
            Self::Int64(v) => Some(*v),
            Self::UInt64(v) => Some(*v as i64),
            _ => None,
        }
    }

    /// Try to get as u64
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Bool(v) => Some(*v as u64),
            Self::Int8(v) => Some(*v as u64),
            Self::UInt8(v) => Some(*v as u64),
            Self::Int16(v) => Some(*v as u64),
            Self::UInt16(v) => Some(*v as u64),
            Self::Int32(v) => Some(*v as u64),
            Self::UInt32(v) => Some(*v as u64),
            Self::Int64(v) => Some(*v as u64),
            Self::UInt64(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get as f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float32(v) => Some(*v as f64),
            Self::Float64(v) => Some(*v),
            Self::Int32(v) => Some(*v as f64),
            Self::UInt32(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            Self::Int32(v) => Some(*v != 0),
            Self::UInt32(v) => Some(*v != 0),
            _ => None,
        }
    }

    /// Negate this constant
    pub fn negate(&self) -> Option<Self> {
        match self {
            Self::Int8(v) => Some(Self::Int8(-*v)),
            Self::Int16(v) => Some(Self::Int16(-*v)),
            Self::Int32(v) => Some(Self::Int32(-*v)),
            Self::Int64(v) => Some(Self::Int64(-*v)),
            Self::Float32(v) => Some(Self::Float32(-*v)),
            Self::Float64(v) => Some(Self::Float64(-*v)),
            _ => None,
        }
    }

    /// Add two constants
    pub fn add(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a.wrapping_add(*b))),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a.wrapping_add(*b))),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a.wrapping_add(*b))),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a.wrapping_add(*b))),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a.wrapping_add(*b))),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a.wrapping_add(*b))),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a.wrapping_add(*b))),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a.wrapping_add(*b))),
            (Self::Float32(a), Self::Float32(b)) => Some(Self::Float32(a + b)),
            (Self::Float64(a), Self::Float64(b)) => Some(Self::Float64(a + b)),
            _ => None,
        }
    }

    /// Subtract two constants
    pub fn sub(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a.wrapping_sub(*b))),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a.wrapping_sub(*b))),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a.wrapping_sub(*b))),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a.wrapping_sub(*b))),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a.wrapping_sub(*b))),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a.wrapping_sub(*b))),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a.wrapping_sub(*b))),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a.wrapping_sub(*b))),
            (Self::Float32(a), Self::Float32(b)) => Some(Self::Float32(a - b)),
            (Self::Float64(a), Self::Float64(b)) => Some(Self::Float64(a - b)),
            _ => None,
        }
    }

    /// Multiply two constants
    pub fn mul(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a.wrapping_mul(*b))),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a.wrapping_mul(*b))),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a.wrapping_mul(*b))),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a.wrapping_mul(*b))),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a.wrapping_mul(*b))),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a.wrapping_mul(*b))),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a.wrapping_mul(*b))),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a.wrapping_mul(*b))),
            (Self::Float32(a), Self::Float32(b)) => Some(Self::Float32(a * b)),
            (Self::Float64(a), Self::Float64(b)) => Some(Self::Float64(a * b)),
            _ => None,
        }
    }

    /// Divide two constants
    pub fn div(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) if *b != 0 => Some(Self::Int8(a / b)),
            (Self::UInt8(a), Self::UInt8(b)) if *b != 0 => Some(Self::UInt8(a / b)),
            (Self::Int16(a), Self::Int16(b)) if *b != 0 => Some(Self::Int16(a / b)),
            (Self::UInt16(a), Self::UInt16(b)) if *b != 0 => Some(Self::UInt16(a / b)),
            (Self::Int32(a), Self::Int32(b)) if *b != 0 => Some(Self::Int32(a / b)),
            (Self::UInt32(a), Self::UInt32(b)) if *b != 0 => Some(Self::UInt32(a / b)),
            (Self::Int64(a), Self::Int64(b)) if *b != 0 => Some(Self::Int64(a / b)),
            (Self::UInt64(a), Self::UInt64(b)) if *b != 0 => Some(Self::UInt64(a / b)),
            (Self::Float32(a), Self::Float32(b)) => Some(Self::Float32(a / b)),
            (Self::Float64(a), Self::Float64(b)) => Some(Self::Float64(a / b)),
            _ => None,
        }
    }

    /// Bitwise AND
    pub fn bitwise_and(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a & b)),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a & b)),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a & b)),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a & b)),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a & b)),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a & b)),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a & b)),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a & b)),
            (Self::Bool(a), Self::Bool(b)) => Some(Self::Bool(*a && *b)),
            _ => None,
        }
    }

    /// Bitwise OR
    pub fn bitwise_or(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a | b)),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a | b)),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a | b)),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a | b)),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a | b)),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a | b)),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a | b)),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a | b)),
            (Self::Bool(a), Self::Bool(b)) => Some(Self::Bool(*a || *b)),
            _ => None,
        }
    }

    /// Bitwise XOR
    pub fn bitwise_xor(&self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(Self::Int8(a ^ b)),
            (Self::UInt8(a), Self::UInt8(b)) => Some(Self::UInt8(a ^ b)),
            (Self::Int16(a), Self::Int16(b)) => Some(Self::Int16(a ^ b)),
            (Self::UInt16(a), Self::UInt16(b)) => Some(Self::UInt16(a ^ b)),
            (Self::Int32(a), Self::Int32(b)) => Some(Self::Int32(a ^ b)),
            (Self::UInt32(a), Self::UInt32(b)) => Some(Self::UInt32(a ^ b)),
            (Self::Int64(a), Self::Int64(b)) => Some(Self::Int64(a ^ b)),
            (Self::UInt64(a), Self::UInt64(b)) => Some(Self::UInt64(a ^ b)),
            (Self::Bool(a), Self::Bool(b)) => Some(Self::Bool(*a ^ *b)),
            _ => None,
        }
    }

    /// Comparison: equal
    pub fn eq(&self, other: &Self) -> Option<bool> {
        match (self, other) {
            (Self::Bool(a), Self::Bool(b)) => Some(a == b),
            (Self::Int8(a), Self::Int8(b)) => Some(a == b),
            (Self::UInt8(a), Self::UInt8(b)) => Some(a == b),
            (Self::Int16(a), Self::Int16(b)) => Some(a == b),
            (Self::UInt16(a), Self::UInt16(b)) => Some(a == b),
            (Self::Int32(a), Self::Int32(b)) => Some(a == b),
            (Self::UInt32(a), Self::UInt32(b)) => Some(a == b),
            (Self::Int64(a), Self::Int64(b)) => Some(a == b),
            (Self::UInt64(a), Self::UInt64(b)) => Some(a == b),
            (Self::Float32(a), Self::Float32(b)) => Some(a == b),
            (Self::Float64(a), Self::Float64(b)) => Some(a == b),
            _ => None,
        }
    }

    /// Comparison: less than
    pub fn lt(&self, other: &Self) -> Option<bool> {
        match (self, other) {
            (Self::Int8(a), Self::Int8(b)) => Some(a < b),
            (Self::UInt8(a), Self::UInt8(b)) => Some(a < b),
            (Self::Int16(a), Self::Int16(b)) => Some(a < b),
            (Self::UInt16(a), Self::UInt16(b)) => Some(a < b),
            (Self::Int32(a), Self::Int32(b)) => Some(a < b),
            (Self::UInt32(a), Self::UInt32(b)) => Some(a < b),
            (Self::Int64(a), Self::Int64(b)) => Some(a < b),
            (Self::UInt64(a), Self::UInt64(b)) => Some(a < b),
            (Self::Float32(a), Self::Float32(b)) => Some(a < b),
            (Self::Float64(a), Self::Float64(b)) => Some(a < b),
            _ => None,
        }
    }

    /// Get element from vector/array
    pub fn get_element(&self, index: usize) -> Option<&ConstantValue> {
        match self {
            Self::Vector(v) | Self::Array(v) | Self::Matrix(v) | Self::Struct(v) => v.get(index),
            Self::Splat(v, count) if index < *count as usize => Some(v.as_ref()),
            _ => None,
        }
    }

    /// Get the number of elements
    pub fn element_count(&self) -> usize {
        match self {
            Self::Vector(v) | Self::Array(v) | Self::Matrix(v) | Self::Struct(v) => v.len(),
            Self::Splat(_, count) => *count as usize,
            _ => 1,
        }
    }
}

impl fmt::Display for ConstantValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{}", v),
            Self::Int8(v) => write!(f, "{}i8", v),
            Self::UInt8(v) => write!(f, "{}u8", v),
            Self::Int16(v) => write!(f, "{}i16", v),
            Self::UInt16(v) => write!(f, "{}u16", v),
            Self::Int32(v) => write!(f, "{}", v),
            Self::UInt32(v) => write!(f, "{}u", v),
            Self::Int64(v) => write!(f, "{}i64", v),
            Self::UInt64(v) => write!(f, "{}u64", v),
            Self::Float16(v) => write!(f, "{}h", v),
            Self::Float32(v) => write!(f, "{}f", v),
            Self::Float64(v) => write!(f, "{}lf", v),
            Self::Vector(v) => {
                write!(f, "vec{}(", v.len())?;
                for (i, c) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", c)?;
                }
                write!(f, ")")
            }
            Self::Matrix(v) => write!(f, "mat({})", v.len()),
            Self::Array(v) => write!(f, "[{}; {}]", if v.is_empty() { "..." } else { "..." }, v.len()),
            Self::Struct(v) => write!(f, "struct {{ {} fields }}", v.len()),
            Self::Null => write!(f, "null"),
            Self::Splat(v, n) => write!(f, "splat({}, {})", v, n),
            Self::Undef => write!(f, "undef"),
        }
    }
}

/// Value definition in the IR
#[derive(Debug, Clone)]
pub struct Value {
    /// Unique identifier
    pub id: ValueId,
    /// Type of the value
    pub ty: IrType,
    /// Value kind
    pub kind: ValueKind,
    /// Debug name
    pub name: Option<String>,
}

/// Kind of value
#[derive(Debug, Clone)]
pub enum ValueKind {
    /// Constant value
    Constant(ConstantValue),
    /// Instruction result
    Instruction,
    /// Function parameter
    Parameter(u32),
    /// Global variable
    Global,
    /// Undefined value
    Undef,
}

impl Value {
    /// Create a new constant value
    pub fn constant(id: ValueId, ty: IrType, value: ConstantValue) -> Self {
        Self {
            id,
            ty,
            kind: ValueKind::Constant(value),
            name: None,
        }
    }

    /// Create a new instruction result
    pub fn instruction(id: ValueId, ty: IrType) -> Self {
        Self {
            id,
            ty,
            kind: ValueKind::Instruction,
            name: None,
        }
    }

    /// Create a new parameter
    pub fn parameter(id: ValueId, ty: IrType, index: u32) -> Self {
        Self {
            id,
            ty,
            kind: ValueKind::Parameter(index),
            name: None,
        }
    }

    /// Create a new global value
    pub fn global(id: ValueId, ty: IrType) -> Self {
        Self {
            id,
            ty,
            kind: ValueKind::Global,
            name: None,
        }
    }

    /// Create an undefined value
    pub fn undef(id: ValueId, ty: IrType) -> Self {
        Self {
            id,
            ty,
            kind: ValueKind::Undef,
            name: None,
        }
    }

    /// Set the debug name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Check if this is a constant
    pub fn is_constant(&self) -> bool {
        matches!(self.kind, ValueKind::Constant(_))
    }

    /// Try to get the constant value
    pub fn as_constant(&self) -> Option<&ConstantValue> {
        match &self.kind {
            ValueKind::Constant(c) => Some(c),
            _ => None,
        }
    }

    /// Check if this is a parameter
    pub fn is_parameter(&self) -> bool {
        matches!(self.kind, ValueKind::Parameter(_))
    }

    /// Get parameter index if this is a parameter
    pub fn parameter_index(&self) -> Option<u32> {
        match self.kind {
            ValueKind::Parameter(i) => Some(i),
            _ => None,
        }
    }
}

/// Value numbering table for SSA construction
#[derive(Debug, Default)]
pub struct ValueTable {
    /// Values by ID
    values: Vec<Option<Value>>,
    /// Next value ID
    next_id: ValueId,
}

impl ValueTable {
    /// Create a new value table
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            next_id: 0,
        }
    }

    /// Allocate a new value ID
    pub fn alloc_id(&mut self) -> ValueId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Insert a value
    pub fn insert(&mut self, value: Value) {
        let id = value.id as usize;
        if id >= self.values.len() {
            self.values.resize(id + 1, None);
        }
        self.values[id] = Some(value);
    }

    /// Get a value by ID
    pub fn get(&self, id: ValueId) -> Option<&Value> {
        self.values.get(id as usize).and_then(|v| v.as_ref())
    }

    /// Get a mutable value by ID
    pub fn get_mut(&mut self, id: ValueId) -> Option<&mut Value> {
        self.values.get_mut(id as usize).and_then(|v| v.as_mut())
    }

    /// Check if a value exists
    pub fn contains(&self, id: ValueId) -> bool {
        self.values.get(id as usize).map(|v| v.is_some()).unwrap_or(false)
    }

    /// Get the type of a value
    pub fn get_type(&self, id: ValueId) -> Option<&IrType> {
        self.get(id).map(|v| &v.ty)
    }

    /// Iterate over all values
    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.values.iter().filter_map(|v| v.as_ref())
    }

    /// Get the count of values
    pub fn len(&self) -> usize {
        self.values.iter().filter(|v| v.is_some()).count()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create a constant value and insert it
    pub fn create_constant(&mut self, ty: IrType, value: ConstantValue) -> ValueId {
        let id = self.alloc_id();
        let val = Value::constant(id, ty, value);
        self.insert(val);
        id
    }

    /// Create an instruction result and insert it
    pub fn create_instruction(&mut self, ty: IrType) -> ValueId {
        let id = self.alloc_id();
        let val = Value::instruction(id, ty);
        self.insert(val);
        id
    }

    /// Create a parameter and insert it
    pub fn create_parameter(&mut self, ty: IrType, index: u32) -> ValueId {
        let id = self.alloc_id();
        let val = Value::parameter(id, ty, index);
        self.insert(val);
        id
    }
}

/// Specialization constant
#[derive(Debug, Clone)]
pub struct SpecConstant {
    /// Constant ID (for SPIR-V)
    pub constant_id: u32,
    /// Type
    pub ty: IrType,
    /// Default value
    pub default: ConstantValue,
    /// Name
    pub name: Option<String>,
}

impl SpecConstant {
    /// Create a new specialization constant
    pub fn new(constant_id: u32, ty: IrType, default: ConstantValue) -> Self {
        Self {
            constant_id,
            ty,
            default,
            name: None,
        }
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Collection of specialization constants
#[derive(Debug, Default)]
pub struct SpecConstantMap {
    constants: Vec<SpecConstant>,
}

impl SpecConstantMap {
    /// Create a new specialization constant map
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
        }
    }

    /// Add a specialization constant
    pub fn add(&mut self, constant: SpecConstant) {
        self.constants.push(constant);
    }

    /// Get a constant by ID
    pub fn get(&self, constant_id: u32) -> Option<&SpecConstant> {
        self.constants.iter().find(|c| c.constant_id == constant_id)
    }

    /// Get a constant by name
    pub fn get_by_name(&self, name: &str) -> Option<&SpecConstant> {
        self.constants.iter().find(|c| c.name.as_deref() == Some(name))
    }

    /// Iterate over all constants
    pub fn iter(&self) -> impl Iterator<Item = &SpecConstant> {
        self.constants.iter()
    }

    /// Get the count
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_zero() {
        assert!(ConstantValue::i32(0).is_zero());
        assert!(ConstantValue::f32(0.0).is_zero());
        assert!(!ConstantValue::i32(1).is_zero());
    }

    #[test]
    fn test_constant_one() {
        assert!(ConstantValue::i32(1).is_one());
        assert!(ConstantValue::f32(1.0).is_one());
        assert!(!ConstantValue::i32(0).is_one());
    }

    #[test]
    fn test_constant_arithmetic() {
        let a = ConstantValue::i32(10);
        let b = ConstantValue::i32(3);
        
        assert_eq!(a.add(&b), Some(ConstantValue::i32(13)));
        assert_eq!(a.sub(&b), Some(ConstantValue::i32(7)));
        assert_eq!(a.mul(&b), Some(ConstantValue::i32(30)));
        assert_eq!(a.div(&b), Some(ConstantValue::i32(3)));
    }

    #[test]
    fn test_value_table() {
        let mut table = ValueTable::new();
        
        let id1 = table.create_constant(IrType::i32(), ConstantValue::i32(42));
        let id2 = table.create_instruction(IrType::f32());
        
        assert!(table.contains(id1));
        assert!(table.contains(id2));
        assert_eq!(table.len(), 2);
        
        let val1 = table.get(id1).unwrap();
        assert!(val1.is_constant());
    }

    #[test]
    fn test_spec_constant() {
        let sc = SpecConstant::new(0, IrType::i32(), ConstantValue::i32(16))
            .with_name("BLOCK_SIZE");
        
        assert_eq!(sc.constant_id, 0);
        assert_eq!(sc.name, Some("BLOCK_SIZE".into()));
    }
}
