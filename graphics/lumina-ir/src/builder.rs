//! IR Builder
//!
//! Convenient builder API for constructing IR.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec, vec};

use crate::types::{IrType, ScalarType, VectorSize, AddressSpace};
use crate::instruction::{
    Instruction, BinaryOp, UnaryOp, BlockId, MemoryAccess, ImageOperands,
    LoopControl, SelectionControl, Scope, MemorySemantics, GatherComponent,
};
use crate::value::{ValueId, ConstantValue, ValueTable};
use crate::block::{BasicBlock, BlockMap};
use crate::function::{Function, ExecutionModel, ExecutionMode, FunctionId};
use crate::module::Module;

/// IR Builder for constructing functions
#[derive(Debug)]
pub struct IrBuilder<'a> {
    /// Module being built
    module: &'a mut Module,
    /// Current function
    current_function: Option<FunctionId>,
    /// Current block
    current_block: Option<BlockId>,
    /// Insertion point within block (None = end)
    insertion_point: Option<usize>,
}

impl<'a> IrBuilder<'a> {
    /// Create a new builder
    pub fn new(module: &'a mut Module) -> Self {
        Self {
            module,
            current_function: None,
            current_block: None,
            insertion_point: None,
        }
    }

    /// Get the module
    pub fn module(&self) -> &Module {
        self.module
    }

    /// Get the module mutably
    pub fn module_mut(&mut self) -> &mut Module {
        self.module
    }

    /// Get current function
    pub fn current_function(&self) -> Option<FunctionId> {
        self.current_function
    }

    /// Get current block
    pub fn current_block(&self) -> Option<BlockId> {
        self.current_block
    }

    // ========== Function Management ==========

    /// Begin building an entry point
    pub fn begin_entry_point(
        &mut self,
        name: impl Into<String>,
        execution_model: ExecutionModel,
    ) -> FunctionId {
        let id = self.module.create_entry_point(name, execution_model);
        self.current_function = Some(id);
        
        // Create entry block
        if let Some(func) = self.module.get_function_mut(id) {
            let block_id = func.ensure_entry_block();
            self.current_block = Some(block_id);
        }
        
        id
    }

    /// Begin building a regular function
    pub fn begin_function(
        &mut self,
        name: impl Into<String>,
        return_type: IrType,
    ) -> FunctionId {
        let id = self.module.create_function(name, return_type);
        self.current_function = Some(id);
        
        if let Some(func) = self.module.get_function_mut(id) {
            let block_id = func.ensure_entry_block();
            self.current_block = Some(block_id);
        }
        
        id
    }

    /// Add a function parameter
    pub fn add_parameter(&mut self, ty: IrType) -> ValueId {
        let value_id = self.module.alloc_value();
        
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                func.add_parameter(value_id, ty);
            }
        }
        
        value_id
    }

    /// Set local workgroup size (for compute shaders)
    pub fn set_local_size(&mut self, x: u32, y: u32, z: u32) {
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                func.set_local_size(x, y, z);
            }
        }
    }

    /// Add an execution mode
    pub fn add_execution_mode(&mut self, mode: ExecutionMode) {
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                func.add_execution_mode(mode);
            }
        }
    }

    // ========== Block Management ==========

    /// Create a new block
    pub fn create_block(&mut self) -> BlockId {
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                return func.blocks.create_block();
            }
        }
        0
    }

    /// Create a new block with a label
    pub fn create_labeled_block(&mut self, label: impl Into<String>) -> BlockId {
        let id = self.create_block();
        if let Some(func_id) = self.current_function {
            if let Some(func) = self.module.get_function_mut(func_id) {
                if let Some(block) = func.blocks.get_mut(id) {
                    block.label = Some(label.into());
                }
            }
        }
        id
    }

    /// Switch to a block
    pub fn position_at_end(&mut self, block: BlockId) {
        self.current_block = Some(block);
        self.insertion_point = None;
    }

    /// Position at the beginning of a block
    pub fn position_at_start(&mut self, block: BlockId) {
        self.current_block = Some(block);
        self.insertion_point = Some(0);
    }

    /// Get the current insertion block
    fn get_current_block_mut(&mut self) -> Option<&mut BasicBlock> {
        let func_id = self.current_function?;
        let block_id = self.current_block?;
        let func = self.module.get_function_mut(func_id)?;
        func.blocks.get_mut(block_id)
    }

    /// Insert an instruction
    fn insert(&mut self, inst: Instruction) {
        if let Some(block) = self.get_current_block_mut() {
            if let Some(pos) = self.insertion_point {
                block.insert(pos, inst);
                self.insertion_point = Some(pos + 1);
            } else {
                block.push(inst);
            }
        }
    }

    // ========== Value Creation ==========

    /// Allocate a new value ID
    pub fn alloc_value(&mut self) -> ValueId {
        self.module.alloc_value()
    }

    /// Create a constant
    pub fn const_bool(&mut self, value: bool) -> ValueId {
        self.module.create_constant(IrType::bool(), ConstantValue::Bool(value))
    }

    pub fn const_i32(&mut self, value: i32) -> ValueId {
        self.module.create_constant(IrType::i32(), ConstantValue::Int32(value))
    }

    pub fn const_u32(&mut self, value: u32) -> ValueId {
        self.module.create_constant(IrType::u32(), ConstantValue::UInt32(value))
    }

    pub fn const_f32(&mut self, value: f32) -> ValueId {
        self.module.create_constant(IrType::f32(), ConstantValue::Float32(value))
    }

    pub fn const_vec2f(&mut self, x: f32, y: f32) -> ValueId {
        self.module.create_constant(
            IrType::vec2f(),
            ConstantValue::Vector(vec![
                ConstantValue::Float32(x),
                ConstantValue::Float32(y),
            ]),
        )
    }

    pub fn const_vec3f(&mut self, x: f32, y: f32, z: f32) -> ValueId {
        self.module.create_constant(
            IrType::vec3f(),
            ConstantValue::Vector(vec![
                ConstantValue::Float32(x),
                ConstantValue::Float32(y),
                ConstantValue::Float32(z),
            ]),
        )
    }

    pub fn const_vec4f(&mut self, x: f32, y: f32, z: f32, w: f32) -> ValueId {
        self.module.create_constant(
            IrType::vec4f(),
            ConstantValue::Vector(vec![
                ConstantValue::Float32(x),
                ConstantValue::Float32(y),
                ConstantValue::Float32(z),
                ConstantValue::Float32(w),
            ]),
        )
    }

    // ========== Variable Instructions ==========

    /// Declare a local variable
    pub fn alloca(&mut self, ty: IrType) -> ValueId {
        let result = self.alloc_value();
        let ptr_ty = IrType::pointer(ty.clone(), AddressSpace::Private);
        
        self.insert(Instruction::Variable {
            result,
            ty: ptr_ty,
            address_space: AddressSpace::Private,
            initializer: None,
        });
        
        result
    }

    /// Declare an initialized local variable
    pub fn alloca_init(&mut self, ty: IrType, init: ValueId) -> ValueId {
        let result = self.alloc_value();
        let ptr_ty = IrType::pointer(ty.clone(), AddressSpace::Private);
        
        self.insert(Instruction::Variable {
            result,
            ty: ptr_ty,
            address_space: AddressSpace::Private,
            initializer: Some(init),
        });
        
        result
    }

    // ========== Memory Instructions ==========

    /// Load from a pointer
    pub fn load(&mut self, ty: IrType, ptr: ValueId) -> ValueId {
        let result = self.alloc_value();
        
        self.insert(Instruction::Load {
            result,
            ty,
            pointer: ptr,
            access: MemoryAccess::default(),
        });
        
        result
    }

    /// Store to a pointer
    pub fn store(&mut self, ptr: ValueId, value: ValueId) {
        self.insert(Instruction::Store {
            pointer: ptr,
            value,
            access: MemoryAccess::default(),
        });
    }

    /// Access chain (struct member or array element access)
    pub fn access_chain(&mut self, result_ty: IrType, base: ValueId, indices: &[ValueId]) -> ValueId {
        let result = self.alloc_value();
        
        self.insert(Instruction::AccessChain {
            result,
            ty: result_ty,
            base,
            indices: indices.to_vec(),
        });
        
        result
    }

    // ========== Arithmetic Instructions ==========

    /// Binary operation helper
    fn binary_op(&mut self, ty: IrType, op: BinaryOp, left: ValueId, right: ValueId) -> ValueId {
        let result = self.alloc_value();
        
        self.insert(Instruction::BinaryOp {
            result,
            ty,
            op,
            left,
            right,
        });
        
        result
    }

    /// Unary operation helper
    fn unary_op(&mut self, ty: IrType, op: UnaryOp, operand: ValueId) -> ValueId {
        let result = self.alloc_value();
        
        self.insert(Instruction::UnaryOp {
            result,
            ty,
            op,
            operand,
        });
        
        result
    }

    // Integer arithmetic
    pub fn iadd(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::IAdd, left, right)
    }

    pub fn isub(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::ISub, left, right)
    }

    pub fn imul(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::IMul, left, right)
    }

    pub fn sdiv(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::SDiv, left, right)
    }

    pub fn udiv(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::UDiv, left, right)
    }

    pub fn srem(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::SRem, left, right)
    }

    pub fn umod(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::UMod, left, right)
    }

    // Float arithmetic
    pub fn fadd(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::FAdd, left, right)
    }

    pub fn fsub(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::FSub, left, right)
    }

    pub fn fmul(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::FMul, left, right)
    }

    pub fn fdiv(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::FDiv, left, right)
    }

    pub fn frem(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::FRem, left, right)
    }

    /// Fused multiply-add
    pub fn fma(&mut self, ty: IrType, a: ValueId, b: ValueId, c: ValueId) -> ValueId {
        let result = self.alloc_value();
        
        self.insert(Instruction::Fma {
            result,
            ty,
            a,
            b,
            c,
        });
        
        result
    }

    // Negation
    pub fn inegate(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Negate, operand)
    }

    pub fn fnegate(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::FNegate, operand)
    }

    // ========== Bitwise Instructions ==========

    pub fn and(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::BitwiseAnd, left, right)
    }

    pub fn or(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::BitwiseOr, left, right)
    }

    pub fn xor(&mut self, ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::BitwiseXor, left, right)
    }

    pub fn not(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::BitwiseNot, operand)
    }

    pub fn shl(&mut self, ty: IrType, value: ValueId, shift: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::ShiftLeft, value, shift)
    }

    pub fn lshr(&mut self, ty: IrType, value: ValueId, shift: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::ShiftRightLogical, value, shift)
    }

    pub fn ashr(&mut self, ty: IrType, value: ValueId, shift: ValueId) -> ValueId {
        self.binary_op(ty, BinaryOp::ShiftRightArithmetic, value, shift)
    }

    // ========== Comparison Instructions ==========

    pub fn icmp_eq(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::Equal, left, right)
    }

    pub fn icmp_ne(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::NotEqual, left, right)
    }

    pub fn icmp_slt(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::Less, left, right)
    }

    pub fn icmp_sle(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::LessEqual, left, right)
    }

    pub fn icmp_sgt(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::Greater, left, right)
    }

    pub fn icmp_sge(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::GreaterEqual, left, right)
    }

    pub fn fcmp_oeq(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::FOrdEqual, left, right)
    }

    pub fn fcmp_one(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::FOrdNotEqual, left, right)
    }

    pub fn fcmp_olt(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::FOrdLess, left, right)
    }

    pub fn fcmp_ole(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::FOrdLessEqual, left, right)
    }

    pub fn fcmp_ogt(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::FOrdGreater, left, right)
    }

    pub fn fcmp_oge(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::FOrdGreaterEqual, left, right)
    }

    // ========== Logical Instructions ==========

    pub fn logical_and(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::LogicalAnd, left, right)
    }

    pub fn logical_or(&mut self, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(IrType::bool(), BinaryOp::LogicalOr, left, right)
    }

    pub fn logical_not(&mut self, operand: ValueId) -> ValueId {
        self.unary_op(IrType::bool(), UnaryOp::LogicalNot, operand)
    }

    // ========== Math Functions ==========

    pub fn abs(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        if ty.is_float_based() {
            self.unary_op(ty, UnaryOp::FAbs, operand)
        } else {
            self.unary_op(ty, UnaryOp::Abs, operand)
        }
    }

    pub fn sign(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        if ty.is_float_based() {
            self.unary_op(ty, UnaryOp::FSign, operand)
        } else {
            self.unary_op(ty, UnaryOp::Sign, operand)
        }
    }

    pub fn floor(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Floor, operand)
    }

    pub fn ceil(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Ceil, operand)
    }

    pub fn round(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Round, operand)
    }

    pub fn trunc(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Trunc, operand)
    }

    pub fn fract(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Fract, operand)
    }

    pub fn sqrt(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Sqrt, operand)
    }

    pub fn inverse_sqrt(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::InverseSqrt, operand)
    }

    pub fn exp(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Exp, operand)
    }

    pub fn exp2(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Exp2, operand)
    }

    pub fn log(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Log, operand)
    }

    pub fn log2(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Log2, operand)
    }

    pub fn sin(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Sin, operand)
    }

    pub fn cos(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Cos, operand)
    }

    pub fn tan(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Tan, operand)
    }

    pub fn asin(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Asin, operand)
    }

    pub fn acos(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Acos, operand)
    }

    pub fn atan(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Atan, operand)
    }

    pub fn pow(&mut self, ty: IrType, base: ValueId, exponent: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Pow { result, ty, base, exponent });
        result
    }

    pub fn atan2(&mut self, ty: IrType, y: ValueId, x: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Atan2 { result, ty, y, x });
        result
    }

    pub fn min(&mut self, ty: IrType, a: ValueId, b: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Min { result, ty, a, b });
        result
    }

    pub fn max(&mut self, ty: IrType, a: ValueId, b: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Max { result, ty, a, b });
        result
    }

    pub fn clamp(&mut self, ty: IrType, value: ValueId, min: ValueId, max: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Clamp { result, ty, value, min, max });
        result
    }

    pub fn mix(&mut self, ty: IrType, a: ValueId, b: ValueId, t: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Mix { result, ty, a, b, t });
        result
    }

    pub fn smoothstep(&mut self, ty: IrType, edge0: ValueId, edge1: ValueId, x: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::SmoothStep { result, ty, edge0, edge1, x });
        result
    }

    pub fn step(&mut self, ty: IrType, edge: ValueId, x: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Step { result, ty, edge, x });
        result
    }

    // ========== Vector Operations ==========

    pub fn dot(&mut self, scalar_ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(scalar_ty, BinaryOp::Dot, left, right)
    }

    pub fn cross(&mut self, vec_ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        self.binary_op(vec_ty, BinaryOp::Cross, left, right)
    }

    pub fn length(&mut self, scalar_ty: IrType, vector: ValueId) -> ValueId {
        self.unary_op(scalar_ty, UnaryOp::Length, vector)
    }

    pub fn normalize(&mut self, vec_ty: IrType, vector: ValueId) -> ValueId {
        self.unary_op(vec_ty, UnaryOp::Normalize, vector)
    }

    pub fn distance(&mut self, scalar_ty: IrType, a: ValueId, b: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Distance { result, ty: scalar_ty, a, b });
        result
    }

    pub fn reflect(&mut self, vec_ty: IrType, incident: ValueId, normal: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Reflect { result, ty: vec_ty, incident, normal });
        result
    }

    pub fn refract(&mut self, vec_ty: IrType, incident: ValueId, normal: ValueId, eta: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Refract { result, ty: vec_ty, incident, normal, eta });
        result
    }

    // ========== Composite Operations ==========

    pub fn composite_construct(&mut self, ty: IrType, components: &[ValueId]) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::CompositeConstruct {
            result,
            ty,
            components: components.to_vec(),
        });
        result
    }

    pub fn composite_extract(&mut self, ty: IrType, composite: ValueId, indices: &[u32]) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::CompositeExtract {
            result,
            ty,
            composite,
            indices: indices.to_vec(),
        });
        result
    }

    pub fn composite_insert(&mut self, ty: IrType, object: ValueId, composite: ValueId, indices: &[u32]) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::CompositeInsert {
            result,
            ty,
            object,
            composite,
            indices: indices.to_vec(),
        });
        result
    }

    pub fn vector_shuffle(&mut self, ty: IrType, v1: ValueId, v2: ValueId, components: &[u32]) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::VectorShuffle {
            result,
            ty,
            vector1: v1,
            vector2: v2,
            components: components.to_vec(),
        });
        result
    }

    // ========== Matrix Operations ==========

    pub fn transpose(&mut self, ty: IrType, matrix: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Transpose, matrix)
    }

    pub fn determinant(&mut self, scalar_ty: IrType, matrix: ValueId) -> ValueId {
        self.unary_op(scalar_ty, UnaryOp::Determinant, matrix)
    }

    pub fn matrix_inverse(&mut self, ty: IrType, matrix: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::MatrixInverse, matrix)
    }

    pub fn matrix_times_vector(&mut self, vec_ty: IrType, matrix: ValueId, vector: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::MatrixTimesVector { result, ty: vec_ty, matrix, vector });
        result
    }

    pub fn vector_times_matrix(&mut self, vec_ty: IrType, vector: ValueId, matrix: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::VectorTimesMatrix { result, ty: vec_ty, vector, matrix });
        result
    }

    pub fn matrix_times_matrix(&mut self, mat_ty: IrType, left: ValueId, right: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::MatrixTimesMatrix { result, ty: mat_ty, left, right });
        result
    }

    pub fn matrix_times_scalar(&mut self, mat_ty: IrType, matrix: ValueId, scalar: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::MatrixTimesScalar { result, ty: mat_ty, matrix, scalar });
        result
    }

    // ========== Type Conversion ==========

    pub fn bitcast(&mut self, ty: IrType, value: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Bitcast { result, ty, value });
        result
    }

    pub fn convert_s_to_f(&mut self, ty: IrType, value: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ConvertSToF { result, ty, value });
        result
    }

    pub fn convert_u_to_f(&mut self, ty: IrType, value: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ConvertUToF { result, ty, value });
        result
    }

    pub fn convert_f_to_s(&mut self, ty: IrType, value: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ConvertFToS { result, ty, value });
        result
    }

    pub fn convert_f_to_u(&mut self, ty: IrType, value: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ConvertFToU { result, ty, value });
        result
    }

    // ========== Control Flow ==========

    pub fn branch(&mut self, target: BlockId) {
        self.insert(Instruction::Branch { target });
    }

    pub fn cond_branch(&mut self, condition: ValueId, true_target: BlockId, false_target: BlockId) {
        self.insert(Instruction::BranchConditional {
            condition,
            true_target,
            false_target,
            true_weight: None,
            false_weight: None,
        });
    }

    pub fn switch(&mut self, selector: ValueId, default_target: BlockId, cases: &[(i64, BlockId)]) {
        self.insert(Instruction::Switch {
            selector,
            default_target,
            cases: cases.to_vec(),
        });
    }

    pub fn ret(&mut self) {
        self.insert(Instruction::Return);
    }

    pub fn ret_value(&mut self, value: ValueId) {
        self.insert(Instruction::ReturnValue { value });
    }

    pub fn kill(&mut self) {
        self.insert(Instruction::Kill);
    }

    pub fn unreachable(&mut self) {
        self.insert(Instruction::Unreachable);
    }

    pub fn select(&mut self, ty: IrType, condition: ValueId, true_value: ValueId, false_value: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Select {
            result,
            ty,
            condition,
            true_value,
            false_value,
        });
        result
    }

    pub fn phi(&mut self, ty: IrType, operands: &[(ValueId, BlockId)]) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::Phi {
            result,
            ty,
            operands: operands.to_vec(),
        });
        result
    }

    /// Call a function
    pub fn call(&mut self, return_ty: IrType, function: FunctionId, args: &[ValueId]) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::FunctionCall {
            result,
            ty: return_ty,
            function,
            arguments: args.to_vec(),
        });
        result
    }

    // ========== Loop & Selection Control ==========

    pub fn loop_merge(&mut self, merge_block: BlockId, continue_target: BlockId) {
        self.insert(Instruction::LoopMerge {
            merge_block,
            continue_target,
            control: LoopControl::default(),
        });
    }

    pub fn selection_merge(&mut self, merge_block: BlockId) {
        self.insert(Instruction::SelectionMerge {
            merge_block,
            control: SelectionControl::default(),
        });
    }

    // ========== Image Operations ==========

    pub fn sample(&mut self, ty: IrType, sampled_image: ValueId, coordinate: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageSampleImplicitLod {
            result,
            ty,
            sampled_image,
            coordinate,
            operands: ImageOperands::default(),
        });
        result
    }

    pub fn sample_lod(&mut self, ty: IrType, sampled_image: ValueId, coordinate: ValueId, lod: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageSampleExplicitLod {
            result,
            ty,
            sampled_image,
            coordinate,
            operands: ImageOperands {
                lod: Some(lod),
                ..Default::default()
            },
        });
        result
    }

    pub fn sample_grad(&mut self, ty: IrType, sampled_image: ValueId, coordinate: ValueId, ddx: ValueId, ddy: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageSampleExplicitLod {
            result,
            ty,
            sampled_image,
            coordinate,
            operands: ImageOperands {
                grad: Some((ddx, ddy)),
                ..Default::default()
            },
        });
        result
    }

    pub fn sample_compare(&mut self, ty: IrType, sampled_image: ValueId, coordinate: ValueId, dref: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageSampleDrefImplicitLod {
            result,
            ty,
            sampled_image,
            coordinate,
            dref,
            operands: ImageOperands::default(),
        });
        result
    }

    pub fn image_fetch(&mut self, ty: IrType, image: ValueId, coordinate: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageFetch {
            result,
            ty,
            image,
            coordinate,
            operands: ImageOperands::default(),
        });
        result
    }

    pub fn image_read(&mut self, ty: IrType, image: ValueId, coordinate: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageRead {
            result,
            ty,
            image,
            coordinate,
            operands: ImageOperands::default(),
        });
        result
    }

    pub fn image_write(&mut self, image: ValueId, coordinate: ValueId, texel: ValueId) {
        self.insert(Instruction::ImageWrite {
            image,
            coordinate,
            texel,
            operands: ImageOperands::default(),
        });
    }

    pub fn image_query_size(&mut self, ty: IrType, image: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::ImageQuerySize { result, ty, image });
        result
    }

    pub fn sampled_image(&mut self, ty: IrType, image: ValueId, sampler: ValueId) -> ValueId {
        let result = self.alloc_value();
        self.insert(Instruction::SampledImage { result, ty, image, sampler });
        result
    }

    // ========== Derivatives ==========

    pub fn dpdx(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::DPdx, operand)
    }

    pub fn dpdy(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::DPdy, operand)
    }

    pub fn fwidth(&mut self, ty: IrType, operand: ValueId) -> ValueId {
        self.unary_op(ty, UnaryOp::Fwidth, operand)
    }

    // ========== Barriers ==========

    pub fn control_barrier(&mut self, execution: Scope, memory: Scope, semantics: MemorySemantics) {
        self.insert(Instruction::ControlBarrier {
            execution_scope: execution,
            memory_scope: memory,
            semantics,
        });
    }

    pub fn memory_barrier(&mut self, scope: Scope, semantics: MemorySemantics) {
        self.insert(Instruction::MemoryBarrier { scope, semantics });
    }

    pub fn workgroup_barrier(&mut self) {
        self.control_barrier(
            Scope::Workgroup,
            Scope::Workgroup,
            MemorySemantics::WorkgroupMemory,
        );
    }

    // ========== Debug ==========

    pub fn debug_printf(&mut self, format: impl Into<String>, values: &[ValueId]) {
        self.insert(Instruction::DebugPrintf {
            format: format.into(),
            values: values.to_vec(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut module = Module::new("test");
        let mut builder = IrBuilder::new(&mut module);
        
        builder.begin_entry_point("main", ExecutionModel::Fragment);
        let a = builder.const_f32(1.0);
        let b = builder.const_f32(2.0);
        let _c = builder.fadd(IrType::f32(), a, b);
        builder.ret();
        
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn test_builder_control_flow() {
        let mut module = Module::new("test");
        let mut builder = IrBuilder::new(&mut module);
        
        builder.begin_entry_point("main", ExecutionModel::Fragment);
        
        let then_block = builder.create_block();
        let else_block = builder.create_block();
        let merge_block = builder.create_block();
        
        let cond = builder.const_bool(true);
        builder.selection_merge(merge_block);
        builder.cond_branch(cond, then_block, else_block);
        
        builder.position_at_end(then_block);
        builder.branch(merge_block);
        
        builder.position_at_end(else_block);
        builder.branch(merge_block);
        
        builder.position_at_end(merge_block);
        builder.ret();
    }

    #[test]
    fn test_builder_compute() {
        let mut module = Module::new("test");
        let mut builder = IrBuilder::new(&mut module);
        
        builder.begin_entry_point("main", ExecutionModel::GLCompute);
        builder.set_local_size(64, 1, 1);
        builder.workgroup_barrier();
        builder.ret();
        
        let func = module.get_function(0).unwrap();
        assert_eq!(func.local_size(), Some((64, 1, 1)));
    }
}
