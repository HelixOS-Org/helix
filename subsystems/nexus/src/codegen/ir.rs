//! # Intermediate Representation
//!
//! Year 3 EVOLUTION - Code Generation IR
//! A rich intermediate representation for code synthesis and optimization.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// IR TYPES
// ============================================================================

/// IR Node ID
pub type NodeId = u64;

/// IR Block ID
pub type BlockId = u64;

/// IR Function ID
pub type FuncId = u64;

/// IR Module - top level container
#[derive(Debug, Clone)]
pub struct IRModule {
    /// Module name
    pub name: String,
    /// Functions
    pub functions: BTreeMap<FuncId, IRFunction>,
    /// Global variables
    pub globals: BTreeMap<String, IRGlobal>,
    /// Type definitions
    pub types: BTreeMap<String, IRTypeDef>,
    /// Constants
    pub constants: BTreeMap<String, IRConstant>,
}

/// IR Function
#[derive(Debug, Clone)]
pub struct IRFunction {
    /// Function ID
    pub id: FuncId,
    /// Function name
    pub name: String,
    /// Parameters
    pub params: Vec<IRParam>,
    /// Return type
    pub return_type: IRType,
    /// Basic blocks
    pub blocks: BTreeMap<BlockId, IRBlock>,
    /// Entry block
    pub entry: BlockId,
    /// Attributes
    pub attributes: FunctionAttributes,
    /// Local variables
    pub locals: BTreeMap<String, IRLocal>,
}

/// IR Parameter
#[derive(Debug, Clone)]
pub struct IRParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub typ: IRType,
    /// Attributes
    pub attributes: ParamAttributes,
}

/// IR Local variable
#[derive(Debug, Clone)]
pub struct IRLocal {
    /// Variable name
    pub name: String,
    /// Variable type
    pub typ: IRType,
    /// Mutable
    pub mutable: bool,
    /// Stack slot (if allocated)
    pub stack_slot: Option<u32>,
}

/// IR Basic Block
#[derive(Debug, Clone)]
pub struct IRBlock {
    /// Block ID
    pub id: BlockId,
    /// Label
    pub label: String,
    /// Instructions
    pub instructions: Vec<IRInstruction>,
    /// Terminator
    pub terminator: IRTerminator,
    /// Predecessors
    pub predecessors: Vec<BlockId>,
    /// Successors
    pub successors: Vec<BlockId>,
}

/// IR Instruction
#[derive(Debug, Clone)]
pub struct IRInstruction {
    /// Instruction ID
    pub id: NodeId,
    /// Destination (if any)
    pub dest: Option<String>,
    /// Operation
    pub op: IROp,
    /// Source location
    pub loc: Option<SourceLoc>,
}

/// IR Operations
#[derive(Debug, Clone)]
pub enum IROp {
    // Arithmetic
    Add(IRValue, IRValue),
    Sub(IRValue, IRValue),
    Mul(IRValue, IRValue),
    Div(IRValue, IRValue),
    Rem(IRValue, IRValue),
    Neg(IRValue),

    // Bitwise
    And(IRValue, IRValue),
    Or(IRValue, IRValue),
    Xor(IRValue, IRValue),
    Not(IRValue),
    Shl(IRValue, IRValue),
    Shr(IRValue, IRValue),
    Rotl(IRValue, IRValue),
    Rotr(IRValue, IRValue),

    // Comparison
    Eq(IRValue, IRValue),
    Ne(IRValue, IRValue),
    Lt(IRValue, IRValue),
    Le(IRValue, IRValue),
    Gt(IRValue, IRValue),
    Ge(IRValue, IRValue),

    // Memory
    Load(IRValue),
    Store(IRValue, IRValue),
    Alloca(IRType),
    GetElementPtr(IRValue, Vec<IRValue>),

    // Conversions
    Cast(IRValue, IRType),
    Bitcast(IRValue, IRType),
    ZeroExtend(IRValue, IRType),
    SignExtend(IRValue, IRType),
    Truncate(IRValue, IRType),

    // Control
    Call(String, Vec<IRValue>),
    IndirectCall(IRValue, Vec<IRValue>),

    // Phi nodes
    Phi(Vec<(BlockId, IRValue)>),

    // Select
    Select(IRValue, IRValue, IRValue),

    // Atomic
    AtomicLoad(IRValue, MemoryOrdering),
    AtomicStore(IRValue, IRValue, MemoryOrdering),
    AtomicRmw(AtomicOp, IRValue, IRValue, MemoryOrdering),
    CompareExchange(IRValue, IRValue, IRValue, MemoryOrdering, MemoryOrdering),
    Fence(MemoryOrdering),

    // SIMD
    VectorInsert(IRValue, IRValue, u32),
    VectorExtract(IRValue, u32),
    VectorShuffle(IRValue, IRValue, Vec<u32>),

    // Intrinsics
    Intrinsic(String, Vec<IRValue>),

    // No-op
    Nop,
}

/// IR Terminator
#[derive(Debug, Clone)]
pub enum IRTerminator {
    /// Return from function
    Return(Option<IRValue>),
    /// Unconditional branch
    Branch(BlockId),
    /// Conditional branch
    CondBranch(IRValue, BlockId, BlockId),
    /// Switch
    Switch(IRValue, BlockId, Vec<(i128, BlockId)>),
    /// Unreachable
    Unreachable,
    /// Invoke (call that may throw)
    Invoke(String, Vec<IRValue>, BlockId, BlockId),
}

/// IR Value
#[derive(Debug, Clone)]
pub enum IRValue {
    /// Variable reference
    Var(String),
    /// Parameter reference
    Param(u32),
    /// Constant integer
    ConstInt(i128, IRType),
    /// Constant float
    ConstFloat(f64, IRType),
    /// Constant boolean
    ConstBool(bool),
    /// Null pointer
    Null(IRType),
    /// Undefined value
    Undef(IRType),
    /// Zero initializer
    Zero(IRType),
    /// Constant struct
    ConstStruct(Vec<IRValue>),
    /// Constant array
    ConstArray(Vec<IRValue>),
    /// Global reference
    GlobalRef(String),
    /// Function reference
    FuncRef(String),
    /// Block address
    BlockAddr(BlockId),
}

/// IR Type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IRType {
    Void,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Ptr(Box<IRType>),
    Array(Box<IRType>, usize),
    Vector(Box<IRType>, usize),
    Struct(Vec<IRType>),
    Function(Vec<IRType>, Box<IRType>),
    Named(String),
}

/// IR Type Definition
#[derive(Debug, Clone)]
pub struct IRTypeDef {
    /// Type name
    pub name: String,
    /// Underlying type
    pub typ: IRType,
    /// Size in bytes
    pub size: usize,
    /// Alignment
    pub align: usize,
}

/// IR Global variable
#[derive(Debug, Clone)]
pub struct IRGlobal {
    /// Global name
    pub name: String,
    /// Type
    pub typ: IRType,
    /// Initializer
    pub init: Option<IRValue>,
    /// Mutable
    pub mutable: bool,
    /// Linkage
    pub linkage: Linkage,
}

/// IR Constant
#[derive(Debug, Clone)]
pub struct IRConstant {
    /// Constant name
    pub name: String,
    /// Value
    pub value: IRValue,
}

/// Memory ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOrdering {
    Relaxed,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

/// Atomic operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomicOp {
    Xchg,
    Add,
    Sub,
    And,
    Or,
    Xor,
    Min,
    Max,
}

/// Linkage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    Private,
    Internal,
    External,
    Weak,
    LinkOnce,
}

/// Function attributes
#[derive(Debug, Clone, Default)]
pub struct FunctionAttributes {
    /// Always inline
    pub inline: InlineHint,
    /// No return
    pub noreturn: bool,
    /// Pure function
    pub pure_fn: bool,
    /// Const function
    pub const_fn: bool,
    /// No unwind
    pub nounwind: bool,
    /// Naked function
    pub naked: bool,
    /// Cold function
    pub cold: bool,
    /// Hot function
    pub hot: bool,
}

/// Inline hint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InlineHint {
    #[default]
    None,
    Inline,
    AlwaysInline,
    NoInline,
}

/// Parameter attributes
#[derive(Debug, Clone, Default)]
pub struct ParamAttributes {
    /// Non-null
    pub nonnull: bool,
    /// Read-only
    pub readonly: bool,
    /// No capture
    pub nocapture: bool,
    /// Alignment
    pub align: Option<usize>,
}

/// Source location
#[derive(Debug, Clone)]
pub struct SourceLoc {
    /// File
    pub file: String,
    /// Line
    pub line: u32,
    /// Column
    pub column: u32,
}

// ============================================================================
// IR BUILDER
// ============================================================================

/// IR Builder for constructing IR programmatically
pub struct IRBuilder {
    /// Current module
    module: IRModule,
    /// Current function
    current_func: Option<FuncId>,
    /// Current block
    current_block: Option<BlockId>,
    /// Next node ID
    next_node: AtomicU64,
    /// Next block ID
    next_block: AtomicU64,
    /// Next function ID
    next_func: AtomicU64,
}

impl IRBuilder {
    /// Create new builder
    pub fn new(module_name: &str) -> Self {
        Self {
            module: IRModule {
                name: module_name.into(),
                functions: BTreeMap::new(),
                globals: BTreeMap::new(),
                types: BTreeMap::new(),
                constants: BTreeMap::new(),
            },
            current_func: None,
            current_block: None,
            next_node: AtomicU64::new(1),
            next_block: AtomicU64::new(1),
            next_func: AtomicU64::new(1),
        }
    }

    /// Create a new function
    pub fn create_function(
        &mut self,
        name: &str,
        params: Vec<IRParam>,
        return_type: IRType,
    ) -> FuncId {
        let id = self.next_func.fetch_add(1, Ordering::Relaxed);
        let entry_block = self.next_block.fetch_add(1, Ordering::Relaxed);

        let mut blocks = BTreeMap::new();
        blocks.insert(entry_block, IRBlock {
            id: entry_block,
            label: "entry".into(),
            instructions: Vec::new(),
            terminator: IRTerminator::Unreachable,
            predecessors: Vec::new(),
            successors: Vec::new(),
        });

        let func = IRFunction {
            id,
            name: name.into(),
            params,
            return_type,
            blocks,
            entry: entry_block,
            attributes: FunctionAttributes::default(),
            locals: BTreeMap::new(),
        };

        self.module.functions.insert(id, func);
        self.current_func = Some(id);
        self.current_block = Some(entry_block);

        id
    }

    /// Create a new basic block
    pub fn create_block(&mut self, label: &str) -> BlockId {
        let id = self.next_block.fetch_add(1, Ordering::Relaxed);

        let block = IRBlock {
            id,
            label: label.into(),
            instructions: Vec::new(),
            terminator: IRTerminator::Unreachable,
            predecessors: Vec::new(),
            successors: Vec::new(),
        };

        if let Some(func_id) = self.current_func {
            if let Some(func) = self.module.functions.get_mut(&func_id) {
                func.blocks.insert(id, block);
            }
        }

        id
    }

    /// Set current block
    pub fn set_block(&mut self, block: BlockId) {
        self.current_block = Some(block);
    }

    /// Add instruction to current block
    pub fn emit(&mut self, dest: Option<&str>, op: IROp) -> NodeId {
        let id = self.next_node.fetch_add(1, Ordering::Relaxed);

        let instr = IRInstruction {
            id,
            dest: dest.map(|s| s.into()),
            op,
            loc: None,
        };

        if let (Some(func_id), Some(block_id)) = (self.current_func, self.current_block) {
            if let Some(func) = self.module.functions.get_mut(&func_id) {
                if let Some(block) = func.blocks.get_mut(&block_id) {
                    block.instructions.push(instr);
                }
            }
        }

        id
    }

    /// Set block terminator
    pub fn terminate(&mut self, term: IRTerminator) {
        if let (Some(func_id), Some(block_id)) = (self.current_func, self.current_block) {
            if let Some(func) = self.module.functions.get_mut(&func_id) {
                if let Some(block) = func.blocks.get_mut(&block_id) {
                    block.terminator = term;
                }
            }
        }
    }

    /// Add local variable
    pub fn add_local(&mut self, name: &str, typ: IRType, mutable: bool) {
        let local = IRLocal {
            name: name.into(),
            typ,
            mutable,
            stack_slot: None,
        };

        if let Some(func_id) = self.current_func {
            if let Some(func) = self.module.functions.get_mut(&func_id) {
                func.locals.insert(name.into(), local);
            }
        }
    }

    /// Add global variable
    pub fn add_global(&mut self, name: &str, typ: IRType, init: Option<IRValue>, mutable: bool) {
        let global = IRGlobal {
            name: name.into(),
            typ,
            init,
            mutable,
            linkage: Linkage::Internal,
        };

        self.module.globals.insert(name.into(), global);
    }

    /// Emit add instruction
    pub fn build_add(&mut self, dest: &str, lhs: IRValue, rhs: IRValue) -> NodeId {
        self.emit(Some(dest), IROp::Add(lhs, rhs))
    }

    /// Emit sub instruction
    pub fn build_sub(&mut self, dest: &str, lhs: IRValue, rhs: IRValue) -> NodeId {
        self.emit(Some(dest), IROp::Sub(lhs, rhs))
    }

    /// Emit mul instruction
    pub fn build_mul(&mut self, dest: &str, lhs: IRValue, rhs: IRValue) -> NodeId {
        self.emit(Some(dest), IROp::Mul(lhs, rhs))
    }

    /// Emit load instruction
    pub fn build_load(&mut self, dest: &str, ptr: IRValue) -> NodeId {
        self.emit(Some(dest), IROp::Load(ptr))
    }

    /// Emit store instruction
    pub fn build_store(&mut self, ptr: IRValue, val: IRValue) -> NodeId {
        self.emit(None, IROp::Store(ptr, val))
    }

    /// Emit call instruction
    pub fn build_call(&mut self, dest: Option<&str>, func: &str, args: Vec<IRValue>) -> NodeId {
        self.emit(dest, IROp::Call(func.into(), args))
    }

    /// Emit return
    pub fn build_return(&mut self, value: Option<IRValue>) {
        self.terminate(IRTerminator::Return(value));
    }

    /// Emit branch
    pub fn build_branch(&mut self, target: BlockId) {
        self.terminate(IRTerminator::Branch(target));
    }

    /// Emit conditional branch
    pub fn build_cond_branch(&mut self, cond: IRValue, then_block: BlockId, else_block: BlockId) {
        self.terminate(IRTerminator::CondBranch(cond, then_block, else_block));
    }

    /// Finalize and return module
    pub fn finalize(self) -> IRModule {
        self.module
    }

    /// Get module reference
    pub fn module(&self) -> &IRModule {
        &self.module
    }
}

// ============================================================================
// IR UTILITIES
// ============================================================================

impl IRType {
    /// Get size in bytes
    pub fn size(&self) -> usize {
        match self {
            IRType::Void => 0,
            IRType::Bool => 1,
            IRType::I8 | IRType::U8 => 1,
            IRType::I16 | IRType::U16 => 2,
            IRType::I32 | IRType::U32 => 4,
            IRType::I64 | IRType::U64 => 8,
            IRType::I128 | IRType::U128 => 16,
            IRType::F32 => 4,
            IRType::F64 => 8,
            IRType::Ptr(_) => 8,
            IRType::Array(elem, count) => elem.size() * count,
            IRType::Vector(elem, count) => elem.size() * count,
            IRType::Struct(fields) => fields.iter().map(|f| f.size()).sum(),
            IRType::Function(_, _) => 8,
            IRType::Named(_) => 0,
        }
    }

    /// Get alignment
    pub fn align(&self) -> usize {
        match self {
            IRType::Void => 1,
            IRType::Bool => 1,
            IRType::I8 | IRType::U8 => 1,
            IRType::I16 | IRType::U16 => 2,
            IRType::I32 | IRType::U32 => 4,
            IRType::I64 | IRType::U64 => 8,
            IRType::I128 | IRType::U128 => 16,
            IRType::F32 => 4,
            IRType::F64 => 8,
            IRType::Ptr(_) => 8,
            IRType::Array(elem, _) => elem.align(),
            IRType::Vector(elem, _) => elem.align(),
            IRType::Struct(fields) => fields.iter().map(|f| f.align()).max().unwrap_or(1),
            IRType::Function(_, _) => 8,
            IRType::Named(_) => 1,
        }
    }

    /// Check if integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            IRType::I8
                | IRType::I16
                | IRType::I32
                | IRType::I64
                | IRType::I128
                | IRType::U8
                | IRType::U16
                | IRType::U32
                | IRType::U64
                | IRType::U128
        )
    }

    /// Check if floating point
    pub fn is_float(&self) -> bool {
        matches!(self, IRType::F32 | IRType::F64)
    }

    /// Check if pointer
    pub fn is_pointer(&self) -> bool {
        matches!(self, IRType::Ptr(_))
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_size() {
        assert_eq!(IRType::I32.size(), 4);
        assert_eq!(IRType::I64.size(), 8);
        assert_eq!(IRType::Array(Box::new(IRType::I32), 10).size(), 40);
    }

    #[test]
    fn test_builder_function() {
        let mut builder = IRBuilder::new("test_module");

        let func = builder.create_function(
            "add",
            vec![
                IRParam {
                    name: "a".into(),
                    typ: IRType::I32,
                    attributes: ParamAttributes::default(),
                },
                IRParam {
                    name: "b".into(),
                    typ: IRType::I32,
                    attributes: ParamAttributes::default(),
                },
            ],
            IRType::I32,
        );

        builder.build_add("sum", IRValue::Param(0), IRValue::Param(1));
        builder.build_return(Some(IRValue::Var("sum".into())));

        let module = builder.finalize();
        assert!(module.functions.contains_key(&func));
    }

    #[test]
    fn test_builder_blocks() {
        let mut builder = IRBuilder::new("test");

        builder.create_function("test_fn", vec![], IRType::Void);

        let then_block = builder.create_block("then");
        let else_block = builder.create_block("else");
        let merge_block = builder.create_block("merge");

        builder.build_cond_branch(IRValue::ConstBool(true), then_block, else_block);

        builder.set_block(then_block);
        builder.build_branch(merge_block);

        builder.set_block(else_block);
        builder.build_branch(merge_block);

        builder.set_block(merge_block);
        builder.build_return(None);

        let module = builder.finalize();
        let func = module.functions.values().next().unwrap();

        assert_eq!(func.blocks.len(), 4); // entry + 3 created
    }
}
