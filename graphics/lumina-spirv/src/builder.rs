//! SPIR-V Builder
//!
//! High-level API for constructing SPIR-V modules.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use crate::{
    binary::SectionedBinaryWriter,
    instruction::*,
    opcode::Opcode,
    types::{BuiltIn, ExecutionMode, SpirVType, TypeRegistry},
    SpirVResult, LUMINA_GENERATOR_MAGIC, SPIRV_VERSION_1_5,
};

/// SPIR-V module builder
#[derive(Debug)]
pub struct SpirVBuilder {
    /// Type registry
    types: TypeRegistry,
    /// ID counter
    next_id: Id,
    /// Capability set
    capabilities: Vec<Capability>,
    /// Extensions
    extensions: Vec<String>,
    /// Extended instruction sets
    ext_inst_imports: BTreeMap<String, Id>,
    /// Addressing model
    addressing_model: AddressingModel,
    /// Memory model
    memory_model: MemoryModel,
    /// Entry points
    entry_points: Vec<EntryPointInfo>,
    /// Debug names
    names: BTreeMap<Id, String>,
    /// Member names
    member_names: BTreeMap<(Id, u32), String>,
    /// Decorations
    decorations: Vec<DecorationInfo>,
    /// Member decorations
    member_decorations: Vec<MemberDecorationInfo>,
    /// Constants
    constants: BTreeMap<Id, ConstantInfo>,
    /// Global variables
    globals: Vec<GlobalInfo>,
    /// Functions
    functions: Vec<FunctionBuilder>,
    /// Current function being built
    current_function: Option<usize>,
    /// Current block being built
    current_block: Option<Id>,
}

impl SpirVBuilder {
    /// Create a new SPIR-V builder
    pub fn new() -> Self {
        Self {
            types: TypeRegistry::new(),
            next_id: 1,
            capabilities: Vec::new(),
            extensions: Vec::new(),
            ext_inst_imports: BTreeMap::new(),
            addressing_model: AddressingModel::Logical,
            memory_model: MemoryModel::GLSL450,
            entry_points: Vec::new(),
            names: BTreeMap::new(),
            member_names: BTreeMap::new(),
            decorations: Vec::new(),
            member_decorations: Vec::new(),
            constants: BTreeMap::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            current_function: None,
            current_block: None,
        }
    }

    /// Allocate a new ID
    pub fn alloc_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Get type registry
    pub fn types(&self) -> &TypeRegistry {
        &self.types
    }

    /// Get mutable type registry
    pub fn types_mut(&mut self) -> &mut TypeRegistry {
        &mut self.types
    }

    // Capability and extension management

    /// Add a capability
    pub fn add_capability(&mut self, cap: Capability) {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
    }

    /// Add an extension
    pub fn add_extension(&mut self, ext: String) {
        if !self.extensions.contains(&ext) {
            self.extensions.push(ext);
        }
    }

    /// Import extended instruction set
    pub fn import_ext_inst_set(&mut self, name: &str) -> Id {
        if let Some(&id) = self.ext_inst_imports.get(name) {
            return id;
        }
        let id = self.alloc_id();
        self.ext_inst_imports.insert(name.into(), id);
        id
    }

    /// Import GLSL.std.450
    pub fn import_glsl(&mut self) -> Id {
        self.import_ext_inst_set("GLSL.std.450")
    }

    /// Set memory model
    pub fn set_memory_model(&mut self, addressing: AddressingModel, memory: MemoryModel) {
        self.addressing_model = addressing;
        self.memory_model = memory;
    }

    // Debug information

    /// Set name for ID
    pub fn set_name(&mut self, id: Id, name: &str) {
        self.names.insert(id, name.into());
    }

    /// Set member name
    pub fn set_member_name(&mut self, struct_id: Id, member: u32, name: &str) {
        self.member_names.insert((struct_id, member), name.into());
    }

    // Decorations

    /// Add decoration
    pub fn decorate(&mut self, target: Id, decoration: Decoration) {
        self.decorations.push(DecorationInfo {
            target,
            decoration,
            operands: Vec::new(),
        });
    }

    /// Add decoration with operand
    pub fn decorate_with(&mut self, target: Id, decoration: Decoration, operand: u32) {
        self.decorations.push(DecorationInfo {
            target,
            decoration,
            operands: vec![operand],
        });
    }

    /// Decorate member
    pub fn decorate_member(&mut self, struct_id: Id, member: u32, decoration: Decoration) {
        self.member_decorations.push(MemberDecorationInfo {
            struct_id,
            member,
            decoration,
            operands: Vec::new(),
        });
    }

    /// Decorate member with operand
    pub fn decorate_member_with(
        &mut self,
        struct_id: Id,
        member: u32,
        decoration: Decoration,
        operand: u32,
    ) {
        self.member_decorations.push(MemberDecorationInfo {
            struct_id,
            member,
            decoration,
            operands: vec![operand],
        });
    }

    /// Add location decoration
    pub fn decorate_location(&mut self, target: Id, location: u32) {
        self.decorate_with(target, Decoration::Location, location);
    }

    /// Add binding decoration
    pub fn decorate_binding(&mut self, target: Id, binding: u32) {
        self.decorate_with(target, Decoration::Binding, binding);
    }

    /// Add descriptor set decoration
    pub fn decorate_descriptor_set(&mut self, target: Id, set: u32) {
        self.decorate_with(target, Decoration::DescriptorSet, set);
    }

    /// Add built-in decoration
    pub fn decorate_builtin(&mut self, target: Id, builtin: BuiltIn) {
        self.decorate_with(target, Decoration::BuiltIn, builtin as u32);
    }

    // Type helpers (delegate to registry)

    /// Get void type
    pub fn type_void(&mut self) -> Id {
        self.types.void()
    }

    /// Get bool type
    pub fn type_bool(&mut self) -> Id {
        self.types.bool()
    }

    /// Get i32 type
    pub fn type_i32(&mut self) -> Id {
        self.types.i32()
    }

    /// Get u32 type
    pub fn type_u32(&mut self) -> Id {
        self.types.u32()
    }

    /// Get f32 type
    pub fn type_f32(&mut self) -> Id {
        self.types.f32()
    }

    /// Get vec2 type
    pub fn type_vec2(&mut self) -> Id {
        self.types.vec2()
    }

    /// Get vec3 type
    pub fn type_vec3(&mut self) -> Id {
        self.types.vec3()
    }

    /// Get vec4 type
    pub fn type_vec4(&mut self) -> Id {
        self.types.vec4()
    }

    /// Get mat4 type
    pub fn type_mat4(&mut self) -> Id {
        self.types.mat4()
    }

    /// Get pointer type
    pub fn type_pointer(&mut self, storage_class: StorageClass, pointee: Id) -> Id {
        self.types.pointer(storage_class, pointee)
    }

    /// Get function type
    pub fn type_function(&mut self, return_type: Id, params: Vec<Id>) -> Id {
        self.types.function(return_type, params)
    }

    /// Get array type
    pub fn type_array(&mut self, element: Id, length: Id) -> Id {
        self.types.array(element, length)
    }

    /// Get struct type
    pub fn type_struct(&mut self, members: Vec<Id>) -> Id {
        self.types.struct_type(members)
    }

    /// Get sampler type
    pub fn type_sampler(&mut self) -> Id {
        self.types.sampler()
    }

    /// Get 2D image type
    pub fn type_image_2d(&mut self, sampled_type: Id, format: ImageFormat) -> Id {
        self.types.image_2d(sampled_type, format)
    }

    /// Get sampled image type
    pub fn type_sampled_image(&mut self, image: Id) -> Id {
        self.types.sampled_image(image)
    }

    // Constants

    /// Define a constant
    pub fn constant_bool(&mut self, value: bool) -> Id {
        let type_id = self.type_bool();
        self.define_constant(type_id, ConstantValue::Bool(value))
    }

    /// Define i32 constant
    pub fn constant_i32(&mut self, value: i32) -> Id {
        let type_id = self.type_i32();
        self.define_constant(type_id, ConstantValue::Int32(value))
    }

    /// Define u32 constant
    pub fn constant_u32(&mut self, value: u32) -> Id {
        let type_id = self.type_u32();
        self.define_constant(type_id, ConstantValue::Uint32(value))
    }

    /// Define f32 constant
    pub fn constant_f32(&mut self, value: f32) -> Id {
        let type_id = self.type_f32();
        self.define_constant(type_id, ConstantValue::Float32(value))
    }

    /// Define composite constant
    pub fn constant_composite(&mut self, type_id: Id, constituents: Vec<Id>) -> Id {
        self.define_constant(type_id, ConstantValue::Composite(constituents))
    }

    /// Define constant
    fn define_constant(&mut self, type_id: Id, value: ConstantValue) -> Id {
        // Check if already exists
        for (&id, info) in &self.constants {
            if info.type_id == type_id && info.value == value {
                return id;
            }
        }

        let id = self.alloc_id();
        self.constants.insert(
            id,
            ConstantInfo {
                type_id,
                value,
                spec: false,
            },
        );
        id
    }

    // Global variables

    /// Define a global variable
    pub fn global_variable(
        &mut self,
        pointer_type: Id,
        storage_class: StorageClass,
        initializer: Option<Id>,
    ) -> Id {
        let id = self.alloc_id();
        self.globals.push(GlobalInfo {
            id,
            pointer_type,
            storage_class,
            initializer,
        });
        id
    }

    /// Define input variable
    pub fn input(&mut self, type_id: Id) -> Id {
        let ptr_type = self.type_pointer(StorageClass::Input, type_id);
        self.global_variable(ptr_type, StorageClass::Input, None)
    }

    /// Define output variable
    pub fn output(&mut self, type_id: Id) -> Id {
        let ptr_type = self.type_pointer(StorageClass::Output, type_id);
        self.global_variable(ptr_type, StorageClass::Output, None)
    }

    /// Define uniform variable
    pub fn uniform(&mut self, type_id: Id) -> Id {
        let ptr_type = self.type_pointer(StorageClass::Uniform, type_id);
        self.global_variable(ptr_type, StorageClass::Uniform, None)
    }

    /// Define storage buffer variable
    pub fn storage_buffer(&mut self, type_id: Id) -> Id {
        let ptr_type = self.type_pointer(StorageClass::StorageBuffer, type_id);
        self.global_variable(ptr_type, StorageClass::StorageBuffer, None)
    }

    /// Define push constant variable
    pub fn push_constant(&mut self, type_id: Id) -> Id {
        let ptr_type = self.type_pointer(StorageClass::PushConstant, type_id);
        self.global_variable(ptr_type, StorageClass::PushConstant, None)
    }

    // Entry points

    /// Add entry point
    pub fn entry_point(
        &mut self,
        execution_model: ExecutionModel,
        function: Id,
        name: &str,
        interface: Vec<Id>,
    ) {
        self.entry_points.push(EntryPointInfo {
            execution_model,
            function,
            name: name.into(),
            interface,
            execution_modes: Vec::new(),
        });
    }

    /// Add execution mode to last entry point
    pub fn execution_mode(&mut self, mode: ExecutionMode, operands: Vec<u32>) {
        if let Some(ep) = self.entry_points.last_mut() {
            ep.execution_modes.push(ExecutionModeInfo { mode, operands });
        }
    }

    /// Set local size for compute shader
    pub fn local_size(&mut self, x: u32, y: u32, z: u32) {
        self.execution_mode(ExecutionMode::LocalSize, vec![x, y, z]);
    }

    /// Set origin upper left for fragment shader
    pub fn origin_upper_left(&mut self) {
        self.execution_mode(ExecutionMode::OriginUpperLeft, vec![]);
    }

    // Function building

    /// Begin a function
    pub fn begin_function(
        &mut self,
        return_type: Id,
        function_type: Id,
        control: FunctionControl,
    ) -> Id {
        let id = self.alloc_id();
        let func = FunctionBuilder {
            id,
            return_type,
            function_type,
            control,
            parameters: Vec::new(),
            blocks: Vec::new(),
        };
        self.functions.push(func);
        self.current_function = Some(self.functions.len() - 1);
        id
    }

    /// Add function parameter
    pub fn function_parameter(&mut self, param_type: Id) -> Id {
        let id = self.alloc_id();
        if let Some(idx) = self.current_function {
            self.functions[idx].parameters.push(ParameterInfo {
                id,
                param_type,
            });
        }
        id
    }

    /// Begin a block
    pub fn begin_block(&mut self) -> Id {
        let id = self.alloc_id();
        if let Some(idx) = self.current_function {
            self.functions[idx].blocks.push(BlockBuilder {
                label: id,
                instructions: Vec::new(),
            });
        }
        self.current_block = Some(id);
        id
    }

    /// Get current block
    pub fn current_block(&self) -> Option<Id> {
        self.current_block
    }

    /// End function
    pub fn end_function(&mut self) {
        self.current_function = None;
        self.current_block = None;
    }

    // Instructions

    /// Emit instruction in current block
    fn emit(&mut self, inst: Instruction) {
        if let Some(func_idx) = self.current_function {
            if let Some(block) = self.functions[func_idx].blocks.last_mut() {
                block.instructions.push(inst);
            }
        }
    }

    /// Emit instruction with result
    fn emit_result(&mut self, result_type: Id, mut inst: Instruction) -> Id {
        let id = self.alloc_id();
        inst.result_type = Some(result_type);
        inst.result = Some(id);
        self.emit(inst);
        id
    }

    // Memory operations

    /// Load from pointer
    pub fn load(&mut self, result_type: Id, pointer: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpLoad).with_id(pointer),
        )
    }

    /// Store to pointer
    pub fn store(&mut self, pointer: Id, value: Id) {
        self.emit(
            Instruction::new(Opcode::OpStore)
                .with_id(pointer)
                .with_id(value),
        );
    }

    /// Access chain
    pub fn access_chain(&mut self, result_type: Id, base: Id, indices: &[Id]) -> Id {
        let mut inst = Instruction::new(Opcode::OpAccessChain).with_id(base);
        for &idx in indices {
            inst = inst.with_id(idx);
        }
        self.emit_result(result_type, inst)
    }

    /// Variable
    pub fn variable(&mut self, result_type: Id, storage_class: StorageClass) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpVariable)
                .with_operand(Operand::StorageClass(storage_class)),
        )
    }

    // Arithmetic operations

    /// Floating-point add
    pub fn f_add(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFAdd).with_id(a).with_id(b),
        )
    }

    /// Floating-point subtract
    pub fn f_sub(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFSub).with_id(a).with_id(b),
        )
    }

    /// Floating-point multiply
    pub fn f_mul(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFMul).with_id(a).with_id(b),
        )
    }

    /// Floating-point divide
    pub fn f_div(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFDiv).with_id(a).with_id(b),
        )
    }

    /// Floating-point negate
    pub fn f_negate(&mut self, result_type: Id, a: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFNegate).with_id(a),
        )
    }

    /// Integer add
    pub fn i_add(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpIAdd).with_id(a).with_id(b),
        )
    }

    /// Integer subtract
    pub fn i_sub(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpISub).with_id(a).with_id(b),
        )
    }

    /// Integer multiply
    pub fn i_mul(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpIMul).with_id(a).with_id(b),
        )
    }

    /// Signed integer divide
    pub fn s_div(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpSDiv).with_id(a).with_id(b),
        )
    }

    /// Unsigned integer divide
    pub fn u_div(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpUDiv).with_id(a).with_id(b),
        )
    }

    /// Dot product
    pub fn dot(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpDot).with_id(a).with_id(b),
        )
    }

    /// Matrix times vector
    pub fn matrix_times_vector(&mut self, result_type: Id, matrix: Id, vector: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpMatrixTimesVector)
                .with_id(matrix)
                .with_id(vector),
        )
    }

    /// Vector times matrix
    pub fn vector_times_matrix(&mut self, result_type: Id, vector: Id, matrix: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpVectorTimesMatrix)
                .with_id(vector)
                .with_id(matrix),
        )
    }

    /// Matrix times matrix
    pub fn matrix_times_matrix(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpMatrixTimesMatrix)
                .with_id(a)
                .with_id(b),
        )
    }

    // Comparison operations

    /// Floating-point ordered equal
    pub fn f_ord_equal(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFOrdEqual).with_id(a).with_id(b),
        )
    }

    /// Floating-point ordered less than
    pub fn f_ord_less_than(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFOrdLessThan).with_id(a).with_id(b),
        )
    }

    /// Floating-point ordered greater than
    pub fn f_ord_greater_than(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpFOrdGreaterThan).with_id(a).with_id(b),
        )
    }

    /// Integer equal
    pub fn i_equal(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpIEqual).with_id(a).with_id(b),
        )
    }

    /// Signed integer less than
    pub fn s_less_than(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpSLessThan).with_id(a).with_id(b),
        )
    }

    // Logic operations

    /// Logical and
    pub fn logical_and(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpLogicalAnd).with_id(a).with_id(b),
        )
    }

    /// Logical or
    pub fn logical_or(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpLogicalOr).with_id(a).with_id(b),
        )
    }

    /// Logical not
    pub fn logical_not(&mut self, result_type: Id, a: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpLogicalNot).with_id(a),
        )
    }

    /// Select
    pub fn select(&mut self, result_type: Id, cond: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpSelect)
                .with_id(cond)
                .with_id(a)
                .with_id(b),
        )
    }

    // Bitwise operations

    /// Bitwise and
    pub fn bitwise_and(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpBitwiseAnd).with_id(a).with_id(b),
        )
    }

    /// Bitwise or
    pub fn bitwise_or(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpBitwiseOr).with_id(a).with_id(b),
        )
    }

    /// Bitwise xor
    pub fn bitwise_xor(&mut self, result_type: Id, a: Id, b: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpBitwiseXor).with_id(a).with_id(b),
        )
    }

    /// Bitwise not
    pub fn not(&mut self, result_type: Id, a: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpNot).with_id(a),
        )
    }

    /// Shift left logical
    pub fn shift_left_logical(&mut self, result_type: Id, base: Id, shift: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpShiftLeftLogical)
                .with_id(base)
                .with_id(shift),
        )
    }

    /// Shift right logical
    pub fn shift_right_logical(&mut self, result_type: Id, base: Id, shift: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpShiftRightLogical)
                .with_id(base)
                .with_id(shift),
        )
    }

    // Composite operations

    /// Composite construct
    pub fn composite_construct(&mut self, result_type: Id, constituents: &[Id]) -> Id {
        let mut inst = Instruction::new(Opcode::OpCompositeConstruct);
        for &c in constituents {
            inst = inst.with_id(c);
        }
        self.emit_result(result_type, inst)
    }

    /// Composite extract
    pub fn composite_extract(&mut self, result_type: Id, composite: Id, indices: &[u32]) -> Id {
        let mut inst = Instruction::new(Opcode::OpCompositeExtract).with_id(composite);
        for &idx in indices {
            inst = inst.with_literal(idx);
        }
        self.emit_result(result_type, inst)
    }

    /// Composite insert
    pub fn composite_insert(
        &mut self,
        result_type: Id,
        object: Id,
        composite: Id,
        indices: &[u32],
    ) -> Id {
        let mut inst = Instruction::new(Opcode::OpCompositeInsert)
            .with_id(object)
            .with_id(composite);
        for &idx in indices {
            inst = inst.with_literal(idx);
        }
        self.emit_result(result_type, inst)
    }

    /// Vector shuffle
    pub fn vector_shuffle(
        &mut self,
        result_type: Id,
        vec1: Id,
        vec2: Id,
        components: &[u32],
    ) -> Id {
        let mut inst = Instruction::new(Opcode::OpVectorShuffle)
            .with_id(vec1)
            .with_id(vec2);
        for &c in components {
            inst = inst.with_literal(c);
        }
        self.emit_result(result_type, inst)
    }

    // Conversion operations

    /// Convert float to signed int
    pub fn convert_f_to_s(&mut self, result_type: Id, value: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpConvertFToS).with_id(value),
        )
    }

    /// Convert float to unsigned int
    pub fn convert_f_to_u(&mut self, result_type: Id, value: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpConvertFToU).with_id(value),
        )
    }

    /// Convert signed int to float
    pub fn convert_s_to_f(&mut self, result_type: Id, value: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpConvertSToF).with_id(value),
        )
    }

    /// Convert unsigned int to float
    pub fn convert_u_to_f(&mut self, result_type: Id, value: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpConvertUToF).with_id(value),
        )
    }

    /// Bitcast
    pub fn bitcast(&mut self, result_type: Id, value: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpBitcast).with_id(value),
        )
    }

    // Control flow

    /// Branch
    pub fn branch(&mut self, target: Id) {
        self.emit(Instruction::new(Opcode::OpBranch).with_id(target));
    }

    /// Conditional branch
    pub fn branch_conditional(&mut self, condition: Id, true_label: Id, false_label: Id) {
        self.emit(
            Instruction::new(Opcode::OpBranchConditional)
                .with_id(condition)
                .with_id(true_label)
                .with_id(false_label),
        );
    }

    /// Switch
    pub fn switch(&mut self, selector: Id, default: Id, cases: &[(u32, Id)]) {
        let mut inst = Instruction::new(Opcode::OpSwitch)
            .with_id(selector)
            .with_id(default);
        for &(literal, label) in cases {
            inst = inst.with_literal(literal).with_id(label);
        }
        self.emit(inst);
    }

    /// Return
    pub fn ret(&mut self) {
        self.emit(Instruction::new(Opcode::OpReturn));
    }

    /// Return value
    pub fn ret_value(&mut self, value: Id) {
        self.emit(Instruction::new(Opcode::OpReturnValue).with_id(value));
    }

    /// Kill (discard)
    pub fn kill(&mut self) {
        self.emit(Instruction::new(Opcode::OpKill));
    }

    /// Phi node
    pub fn phi(&mut self, result_type: Id, incoming: &[(Id, Id)]) -> Id {
        let mut inst = Instruction::new(Opcode::OpPhi);
        for &(value, block) in incoming {
            inst = inst.with_id(value).with_id(block);
        }
        self.emit_result(result_type, inst)
    }

    /// Selection merge
    pub fn selection_merge(&mut self, merge_block: Id, control: SelectionControlFlags) {
        self.emit(
            Instruction::new(Opcode::OpSelectionMerge)
                .with_id(merge_block)
                .with_operand(Operand::SelectionControl(control)),
        );
    }

    /// Loop merge
    pub fn loop_merge(&mut self, merge_block: Id, continue_target: Id, control: LoopControlFlags) {
        self.emit(
            Instruction::new(Opcode::OpLoopMerge)
                .with_id(merge_block)
                .with_id(continue_target)
                .with_operand(Operand::LoopControl(control)),
        );
    }

    // Function calls

    /// Function call
    pub fn function_call(&mut self, result_type: Id, function: Id, args: &[Id]) -> Id {
        let mut inst = Instruction::new(Opcode::OpFunctionCall).with_id(function);
        for &arg in args {
            inst = inst.with_id(arg);
        }
        self.emit_result(result_type, inst)
    }

    /// Extended instruction
    pub fn ext_inst(&mut self, result_type: Id, set: Id, instruction: u32, operands: &[Id]) -> Id {
        let mut inst = Instruction::new(Opcode::OpExtInst)
            .with_id(set)
            .with_literal(instruction);
        for &op in operands {
            inst = inst.with_id(op);
        }
        self.emit_result(result_type, inst)
    }

    // Image operations

    /// Sample image (implicit lod)
    pub fn image_sample_implicit_lod(&mut self, result_type: Id, sampled_image: Id, coord: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpImageSampleImplicitLod)
                .with_id(sampled_image)
                .with_id(coord),
        )
    }

    /// Sample image (explicit lod)
    pub fn image_sample_explicit_lod(
        &mut self,
        result_type: Id,
        sampled_image: Id,
        coord: Id,
        lod: Id,
    ) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpImageSampleExplicitLod)
                .with_id(sampled_image)
                .with_id(coord)
                .with_literal(2) // Lod image operand
                .with_id(lod),
        )
    }

    /// Image fetch
    pub fn image_fetch(&mut self, result_type: Id, image: Id, coord: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpImageFetch)
                .with_id(image)
                .with_id(coord),
        )
    }

    /// Image read
    pub fn image_read(&mut self, result_type: Id, image: Id, coord: Id) -> Id {
        self.emit_result(
            result_type,
            Instruction::new(Opcode::OpImageRead)
                .with_id(image)
                .with_id(coord),
        )
    }

    /// Image write
    pub fn image_write(&mut self, image: Id, coord: Id, texel: Id) {
        self.emit(
            Instruction::new(Opcode::OpImageWrite)
                .with_id(image)
                .with_id(coord)
                .with_id(texel),
        );
    }

    // Barriers

    /// Control barrier
    pub fn control_barrier(
        &mut self,
        execution: Scope,
        memory: Scope,
        semantics: MemorySemanticsFlags,
    ) {
        let exec_id = self.constant_u32(execution as u32);
        let mem_id = self.constant_u32(memory as u32);
        let sem_id = self.constant_u32(semantics.bits());
        self.emit(
            Instruction::new(Opcode::OpControlBarrier)
                .with_id(exec_id)
                .with_id(mem_id)
                .with_id(sem_id),
        );
    }

    /// Memory barrier
    pub fn memory_barrier(&mut self, memory: Scope, semantics: MemorySemanticsFlags) {
        let mem_id = self.constant_u32(memory as u32);
        let sem_id = self.constant_u32(semantics.bits());
        self.emit(
            Instruction::new(Opcode::OpMemoryBarrier)
                .with_id(mem_id)
                .with_id(sem_id),
        );
    }

    // Build the module

    /// Build the SPIR-V binary
    pub fn build(&self) -> SpirVResult<Vec<u32>> {
        let mut writer = SectionedBinaryWriter::new();

        // Capabilities
        for cap in &self.capabilities {
            writer.capabilities.push(
                Instruction::new(Opcode::OpCapability)
                    .with_operand(Operand::Capability(*cap)),
            );
        }

        // Add required capabilities from types
        for cap in self.types.capabilities() {
            if !self.capabilities.contains(cap) {
                writer.capabilities.push(
                    Instruction::new(Opcode::OpCapability)
                        .with_operand(Operand::Capability(*cap)),
                );
            }
        }

        // Extensions
        for ext in &self.extensions {
            writer.extensions.push(
                Instruction::new(Opcode::OpExtension)
                    .with_string(ext.clone()),
            );
        }

        // Extended instruction imports
        for (name, &id) in &self.ext_inst_imports {
            writer.ext_inst_imports.push(
                Instruction::new(Opcode::OpExtInstImport)
                    .with_result(id)
                    .with_string(name.clone()),
            );
        }

        // Memory model
        writer.memory_model.push(
            Instruction::new(Opcode::OpMemoryModel)
                .with_operand(Operand::AddressingModel(self.addressing_model))
                .with_operand(Operand::MemoryModel(self.memory_model)),
        );

        // Entry points
        for ep in &self.entry_points {
            let mut inst = Instruction::new(Opcode::OpEntryPoint)
                .with_operand(Operand::ExecutionModel(ep.execution_model))
                .with_id(ep.function)
                .with_string(ep.name.clone());
            for &iface in &ep.interface {
                inst = inst.with_id(iface);
            }
            writer.entry_points.push(inst);

            // Execution modes
            for mode in &ep.execution_modes {
                let mut mode_inst = Instruction::new(Opcode::OpExecutionMode)
                    .with_id(ep.function)
                    .with_literal(mode.mode as u32);
                for &op in &mode.operands {
                    mode_inst = mode_inst.with_literal(op);
                }
                writer.execution_modes.push(mode_inst);
            }
        }

        // Debug names
        for (&id, name) in &self.names {
            writer.debug.push(
                Instruction::new(Opcode::OpName)
                    .with_id(id)
                    .with_string(name.clone()),
            );
        }

        // Member names
        for (&(struct_id, member), name) in &self.member_names {
            writer.debug.push(
                Instruction::new(Opcode::OpMemberName)
                    .with_id(struct_id)
                    .with_literal(member)
                    .with_string(name.clone()),
            );
        }

        // Decorations
        for dec in &self.decorations {
            let mut inst = Instruction::new(Opcode::OpDecorate)
                .with_id(dec.target)
                .with_operand(Operand::Decoration(dec.decoration));
            for &op in &dec.operands {
                inst = inst.with_literal(op);
            }
            writer.annotations.push(inst);
        }

        // Member decorations
        for dec in &self.member_decorations {
            let mut inst = Instruction::new(Opcode::OpMemberDecorate)
                .with_id(dec.struct_id)
                .with_literal(dec.member)
                .with_operand(Operand::Decoration(dec.decoration));
            for &op in &dec.operands {
                inst = inst.with_literal(op);
            }
            writer.annotations.push(inst);
        }

        // Types
        self.emit_types(&mut writer);

        // Constants
        for (&id, info) in &self.constants {
            let inst = self.constant_instruction(id, info);
            writer.types_constants.push(inst);
        }

        // Global variables
        for global in &self.globals {
            let mut inst = Instruction::new(Opcode::OpVariable)
                .with_result_type(global.pointer_type)
                .with_result(global.id)
                .with_operand(Operand::StorageClass(global.storage_class));
            if let Some(init) = global.initializer {
                inst = inst.with_id(init);
            }
            writer.global_variables.push(inst);
        }

        // Functions
        for func in &self.functions {
            // Function header
            writer.functions.push(
                Instruction::new(Opcode::OpFunction)
                    .with_result_type(func.return_type)
                    .with_result(func.id)
                    .with_operand(Operand::FunctionControl(func.control))
                    .with_id(func.function_type),
            );

            // Parameters
            for param in &func.parameters {
                writer.functions.push(
                    Instruction::new(Opcode::OpFunctionParameter)
                        .with_result_type(param.param_type)
                        .with_result(param.id),
                );
            }

            // Blocks
            for block in &func.blocks {
                writer.functions.push(
                    Instruction::new(Opcode::OpLabel).with_result(block.label),
                );
                for inst in &block.instructions {
                    writer.functions.push(inst.clone());
                }
            }

            // Function end
            writer.functions.push(Instruction::new(Opcode::OpFunctionEnd));
        }

        Ok(writer.encode(SPIRV_VERSION_1_5, LUMINA_GENERATOR_MAGIC, self.next_id))
    }

    /// Emit type declarations
    fn emit_types(&self, writer: &mut SectionedBinaryWriter) {
        for (id, ty) in self.types.types() {
            let inst = match ty {
                SpirVType::Void => Instruction::new(Opcode::OpTypeVoid).with_result(id),
                SpirVType::Bool => Instruction::new(Opcode::OpTypeBool).with_result(id),
                SpirVType::Int { width, signed } => Instruction::new(Opcode::OpTypeInt)
                    .with_result(id)
                    .with_literal(*width)
                    .with_literal(if *signed { 1 } else { 0 }),
                SpirVType::Float { width } => Instruction::new(Opcode::OpTypeFloat)
                    .with_result(id)
                    .with_literal(*width),
                SpirVType::Vector { component, count } => Instruction::new(Opcode::OpTypeVector)
                    .with_result(id)
                    .with_id(*component)
                    .with_literal(*count),
                SpirVType::Matrix { column, columns } => Instruction::new(Opcode::OpTypeMatrix)
                    .with_result(id)
                    .with_id(*column)
                    .with_literal(*columns),
                SpirVType::Array { element, length } => Instruction::new(Opcode::OpTypeArray)
                    .with_result(id)
                    .with_id(*element)
                    .with_id(*length),
                SpirVType::RuntimeArray { element } => Instruction::new(Opcode::OpTypeRuntimeArray)
                    .with_result(id)
                    .with_id(*element),
                SpirVType::Struct { members, .. } => {
                    let mut inst = Instruction::new(Opcode::OpTypeStruct).with_result(id);
                    for &m in members {
                        inst = inst.with_id(m);
                    }
                    inst
                }
                SpirVType::Pointer {
                    storage_class,
                    pointee,
                } => Instruction::new(Opcode::OpTypePointer)
                    .with_result(id)
                    .with_operand(Operand::StorageClass(*storage_class))
                    .with_id(*pointee),
                SpirVType::Function {
                    return_type,
                    parameters,
                } => {
                    let mut inst = Instruction::new(Opcode::OpTypeFunction)
                        .with_result(id)
                        .with_id(*return_type);
                    for &p in parameters {
                        inst = inst.with_id(p);
                    }
                    inst
                }
                SpirVType::Image {
                    sampled_type,
                    dim,
                    depth,
                    arrayed,
                    multisampled,
                    sampled,
                    format,
                } => Instruction::new(Opcode::OpTypeImage)
                    .with_result(id)
                    .with_id(*sampled_type)
                    .with_operand(Operand::Dim(*dim))
                    .with_literal(*depth)
                    .with_literal(if *arrayed { 1 } else { 0 })
                    .with_literal(if *multisampled { 1 } else { 0 })
                    .with_literal(*sampled)
                    .with_operand(Operand::ImageFormat(*format)),
                SpirVType::Sampler => Instruction::new(Opcode::OpTypeSampler).with_result(id),
                SpirVType::SampledImage { image } => Instruction::new(Opcode::OpTypeSampledImage)
                    .with_result(id)
                    .with_id(*image),
                SpirVType::AccelerationStructure => {
                    Instruction::new(Opcode::OpTypeAccelerationStructureKHR).with_result(id)
                }
                SpirVType::RayQuery => {
                    Instruction::new(Opcode::OpTypeRayQueryKHR).with_result(id)
                }
            };
            writer.types_constants.push(inst);
        }
    }

    /// Generate constant instruction
    fn constant_instruction(&self, id: Id, info: &ConstantInfo) -> Instruction {
        match &info.value {
            ConstantValue::Bool(true) => {
                if info.spec {
                    Instruction::new(Opcode::OpSpecConstantTrue)
                } else {
                    Instruction::new(Opcode::OpConstantTrue)
                }
                .with_result_type(info.type_id)
                .with_result(id)
            }
            ConstantValue::Bool(false) => {
                if info.spec {
                    Instruction::new(Opcode::OpSpecConstantFalse)
                } else {
                    Instruction::new(Opcode::OpConstantFalse)
                }
                .with_result_type(info.type_id)
                .with_result(id)
            }
            ConstantValue::Int32(v) => {
                if info.spec {
                    Instruction::new(Opcode::OpSpecConstant)
                } else {
                    Instruction::new(Opcode::OpConstant)
                }
                .with_result_type(info.type_id)
                .with_result(id)
                .with_literal(*v as u32)
            }
            ConstantValue::Uint32(v) => {
                if info.spec {
                    Instruction::new(Opcode::OpSpecConstant)
                } else {
                    Instruction::new(Opcode::OpConstant)
                }
                .with_result_type(info.type_id)
                .with_result(id)
                .with_literal(*v)
            }
            ConstantValue::Float32(v) => {
                if info.spec {
                    Instruction::new(Opcode::OpSpecConstant)
                } else {
                    Instruction::new(Opcode::OpConstant)
                }
                .with_result_type(info.type_id)
                .with_result(id)
                .with_literal(v.to_bits())
            }
            ConstantValue::Composite(constituents) => {
                let mut inst = if info.spec {
                    Instruction::new(Opcode::OpSpecConstantComposite)
                } else {
                    Instruction::new(Opcode::OpConstantComposite)
                }
                .with_result_type(info.type_id)
                .with_result(id);
                for &c in constituents {
                    inst = inst.with_id(c);
                }
                inst
            }
        }
    }
}

impl Default for SpirVBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Helper types

#[derive(Debug, Clone)]
struct EntryPointInfo {
    execution_model: ExecutionModel,
    function: Id,
    name: String,
    interface: Vec<Id>,
    execution_modes: Vec<ExecutionModeInfo>,
}

#[derive(Debug, Clone)]
struct ExecutionModeInfo {
    mode: ExecutionMode,
    operands: Vec<u32>,
}

#[derive(Debug, Clone)]
struct DecorationInfo {
    target: Id,
    decoration: Decoration,
    operands: Vec<u32>,
}

#[derive(Debug, Clone)]
struct MemberDecorationInfo {
    struct_id: Id,
    member: u32,
    decoration: Decoration,
    operands: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
enum ConstantValue {
    Bool(bool),
    Int32(i32),
    Uint32(u32),
    Float32(f32),
    Composite(Vec<Id>),
}

#[derive(Debug, Clone)]
struct ConstantInfo {
    type_id: Id,
    value: ConstantValue,
    spec: bool,
}

#[derive(Debug, Clone)]
struct GlobalInfo {
    id: Id,
    pointer_type: Id,
    storage_class: StorageClass,
    initializer: Option<Id>,
}

#[derive(Debug, Clone)]
struct FunctionBuilder {
    id: Id,
    return_type: Id,
    function_type: Id,
    control: FunctionControl,
    parameters: Vec<ParameterInfo>,
    blocks: Vec<BlockBuilder>,
}

#[derive(Debug, Clone)]
struct ParameterInfo {
    id: Id,
    param_type: Id,
}

#[derive(Debug, Clone)]
struct BlockBuilder {
    label: Id,
    instructions: Vec<Instruction>,
}
