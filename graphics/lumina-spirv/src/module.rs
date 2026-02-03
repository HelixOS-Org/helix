//! SPIR-V Module Representation
//!
//! Complete module structure for SPIR-V.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use crate::{
    binary::{BinaryDecoder, BinaryEncoder, Header},
    instruction::*,
    opcode::Opcode,
    types::{BuiltIn, ExecutionMode, SpirVType, TypeRegistry},
    SpirVError, SpirVResult, SPIRV_MAGIC,
};

/// SPIR-V module
#[derive(Debug)]
pub struct SpirVModule {
    /// Header
    pub header: Header,
    /// Capabilities
    pub capabilities: Vec<Capability>,
    /// Extensions
    pub extensions: Vec<String>,
    /// Extended instruction set imports
    pub ext_inst_imports: BTreeMap<Id, String>,
    /// Memory model
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    /// Entry points
    pub entry_points: Vec<EntryPoint>,
    /// Execution modes
    pub execution_modes: BTreeMap<Id, Vec<ExecutionModeDecl>>,
    /// Debug info
    pub source: Option<SourceInfo>,
    /// Names
    pub names: BTreeMap<Id, String>,
    /// Member names
    pub member_names: BTreeMap<(Id, u32), String>,
    /// Decorations
    pub decorations: BTreeMap<Id, Vec<DecorationDecl>>,
    /// Member decorations
    pub member_decorations: BTreeMap<(Id, u32), Vec<DecorationDecl>>,
    /// Types (ID -> Type)
    pub types: BTreeMap<Id, TypeDecl>,
    /// Constants
    pub constants: BTreeMap<Id, ConstantDecl>,
    /// Global variables
    pub global_variables: BTreeMap<Id, VariableDecl>,
    /// Functions
    pub functions: Vec<Function>,
}

impl SpirVModule {
    /// Create an empty module
    pub fn new(version: u32, generator: u32) -> Self {
        Self {
            header: Header::new(version, generator, 1),
            capabilities: Vec::new(),
            extensions: Vec::new(),
            ext_inst_imports: BTreeMap::new(),
            addressing_model: AddressingModel::Logical,
            memory_model: MemoryModel::GLSL450,
            entry_points: Vec::new(),
            execution_modes: BTreeMap::new(),
            source: None,
            names: BTreeMap::new(),
            member_names: BTreeMap::new(),
            decorations: BTreeMap::new(),
            member_decorations: BTreeMap::new(),
            types: BTreeMap::new(),
            constants: BTreeMap::new(),
            global_variables: BTreeMap::new(),
            functions: Vec::new(),
        }
    }

    /// Parse from binary
    pub fn from_binary(words: &[u32]) -> SpirVResult<Self> {
        let mut decoder = BinaryDecoder::new(words);
        let header = decoder.decode_header()?;

        let mut module = Self::new(header.version, header.generator);
        module.header = header;

        // Parse all instructions
        while !decoder.is_empty() {
            let inst = decoder.decode_instruction()?;
            module.process_instruction(inst)?;
        }

        Ok(module)
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> SpirVResult<Self> {
        if bytes.len() % 4 != 0 {
            return Err(SpirVError::Validation("Invalid binary length".into()));
        }

        let words: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        Self::from_binary(&words)
    }

    /// Process a single instruction
    fn process_instruction(&mut self, inst: Instruction) -> SpirVResult<()> {
        match inst.opcode {
            Opcode::OpCapability => {
                if let Some(Operand::Literal(cap)) = inst.operands.first() {
                    // Note: simplified - would need proper capability parsing
                    self.capabilities.push(Capability::Shader);
                }
            }
            Opcode::OpExtension => {
                // Parse extension name from operands
            }
            Opcode::OpExtInstImport => {
                if let Some(result) = inst.result {
                    // Parse name from operands
                    self.ext_inst_imports.insert(result, String::new());
                }
            }
            Opcode::OpMemoryModel => {
                // Already have defaults
            }
            Opcode::OpEntryPoint => {
                // Parse entry point
                if let (Some(Operand::Literal(model)), Some(Operand::Literal(func_id))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let execution_model = match model {
                        0 => ExecutionModel::Vertex,
                        4 => ExecutionModel::Fragment,
                        5 => ExecutionModel::GLCompute,
                        _ => ExecutionModel::Vertex,
                    };
                    self.entry_points.push(EntryPoint {
                        execution_model,
                        function: *func_id,
                        name: String::new(),
                        interface: Vec::new(),
                    });
                }
            }
            Opcode::OpExecutionMode => {
                if let (Some(Operand::Literal(target)), Some(Operand::Literal(mode))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let modes = self.execution_modes.entry(*target).or_default();
                    modes.push(ExecutionModeDecl {
                        mode: ExecutionMode::LocalSize, // Simplified
                        operands: Vec::new(),
                    });
                }
            }
            Opcode::OpName => {
                if let Some(Operand::Literal(target)) = inst.operands.first() {
                    self.names.insert(*target, String::new());
                }
            }
            Opcode::OpMemberName => {
                // Parse member name
            }
            Opcode::OpDecorate => {
                if let Some(Operand::Literal(target)) = inst.operands.first() {
                    let decs = self.decorations.entry(*target).or_default();
                    decs.push(DecorationDecl {
                        decoration: Decoration::Location,
                        operands: Vec::new(),
                    });
                }
            }
            Opcode::OpMemberDecorate => {
                // Parse member decoration
            }
            Opcode::OpTypeVoid
            | Opcode::OpTypeBool
            | Opcode::OpTypeInt
            | Opcode::OpTypeFloat
            | Opcode::OpTypeVector
            | Opcode::OpTypeMatrix
            | Opcode::OpTypeImage
            | Opcode::OpTypeSampler
            | Opcode::OpTypeSampledImage
            | Opcode::OpTypeArray
            | Opcode::OpTypeRuntimeArray
            | Opcode::OpTypeStruct
            | Opcode::OpTypePointer
            | Opcode::OpTypeFunction => {
                if let Some(result) = inst.result {
                    self.types.insert(
                        result,
                        TypeDecl {
                            opcode: inst.opcode,
                            operands: inst.operands.clone(),
                        },
                    );
                }
            }
            Opcode::OpConstant
            | Opcode::OpConstantComposite
            | Opcode::OpConstantTrue
            | Opcode::OpConstantFalse
            | Opcode::OpSpecConstant
            | Opcode::OpSpecConstantTrue
            | Opcode::OpSpecConstantFalse => {
                if let (Some(result_type), Some(result)) = (inst.result_type, inst.result) {
                    self.constants.insert(
                        result,
                        ConstantDecl {
                            result_type,
                            opcode: inst.opcode,
                            operands: inst.operands.clone(),
                        },
                    );
                }
            }
            Opcode::OpVariable => {
                if let (Some(result_type), Some(result)) = (inst.result_type, inst.result) {
                    let storage_class = if let Some(Operand::Literal(sc)) = inst.operands.first() {
                        match sc {
                            0 => StorageClass::UniformConstant,
                            1 => StorageClass::Input,
                            2 => StorageClass::Uniform,
                            3 => StorageClass::Output,
                            4 => StorageClass::Workgroup,
                            6 => StorageClass::Private,
                            7 => StorageClass::Function,
                            9 => StorageClass::PushConstant,
                            12 => StorageClass::StorageBuffer,
                            _ => StorageClass::Private,
                        }
                    } else {
                        StorageClass::Private
                    };
                    self.global_variables.insert(
                        result,
                        VariableDecl {
                            result_type,
                            storage_class,
                            initializer: None,
                        },
                    );
                }
            }
            Opcode::OpFunction => {
                if let (Some(result_type), Some(result)) = (inst.result_type, inst.result) {
                    self.functions.push(Function {
                        id: result,
                        return_type: result_type,
                        function_type: 0, // From operands
                        parameters: Vec::new(),
                        blocks: Vec::new(),
                    });
                }
            }
            Opcode::OpFunctionParameter => {
                if let (Some(result_type), Some(result)) = (inst.result_type, inst.result) {
                    if let Some(func) = self.functions.last_mut() {
                        func.parameters.push(Parameter {
                            id: result,
                            param_type: result_type,
                        });
                    }
                }
            }
            Opcode::OpLabel => {
                if let Some(result) = inst.result {
                    if let Some(func) = self.functions.last_mut() {
                        func.blocks.push(Block {
                            label: result,
                            instructions: Vec::new(),
                        });
                    }
                }
            }
            Opcode::OpFunctionEnd => {
                // Function is complete
            }
            _ => {
                // Add to current block
                if let Some(func) = self.functions.last_mut() {
                    if let Some(block) = func.blocks.last_mut() {
                        block.instructions.push(inst);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get entry point by name
    pub fn get_entry_point(&self, name: &str) -> Option<&EntryPoint> {
        self.entry_points.iter().find(|ep| ep.name == name)
    }

    /// Get entry points by execution model
    pub fn get_entry_points_by_model(&self, model: ExecutionModel) -> Vec<&EntryPoint> {
        self.entry_points
            .iter()
            .filter(|ep| ep.execution_model == model)
            .collect()
    }

    /// Get function by ID
    pub fn get_function(&self, id: Id) -> Option<&Function> {
        self.functions.iter().find(|f| f.id == id)
    }

    /// Get decoration value
    pub fn get_decoration(&self, id: Id, decoration: Decoration) -> Option<u32> {
        self.decorations.get(&id).and_then(|decs| {
            decs.iter()
                .find(|d| d.decoration == decoration)
                .and_then(|d| d.operands.first().copied())
        })
    }

    /// Get location decoration
    pub fn get_location(&self, id: Id) -> Option<u32> {
        self.get_decoration(id, Decoration::Location)
    }

    /// Get binding decoration
    pub fn get_binding(&self, id: Id) -> Option<u32> {
        self.get_decoration(id, Decoration::Binding)
    }

    /// Get descriptor set decoration
    pub fn get_descriptor_set(&self, id: Id) -> Option<u32> {
        self.get_decoration(id, Decoration::DescriptorSet)
    }

    /// Get built-in decoration
    pub fn get_builtin(&self, id: Id) -> Option<BuiltIn> {
        self.get_decoration(id, Decoration::BuiltIn).and_then(|v| {
            // Convert to BuiltIn enum
            Some(BuiltIn::Position) // Simplified
        })
    }

    /// Get name for ID
    pub fn get_name(&self, id: Id) -> Option<&str> {
        self.names.get(&id).map(|s| s.as_str())
    }

    /// Get all input variables
    pub fn get_inputs(&self) -> Vec<(Id, &VariableDecl)> {
        self.global_variables
            .iter()
            .filter(|(_, v)| v.storage_class == StorageClass::Input)
            .map(|(&id, v)| (id, v))
            .collect()
    }

    /// Get all output variables
    pub fn get_outputs(&self) -> Vec<(Id, &VariableDecl)> {
        self.global_variables
            .iter()
            .filter(|(_, v)| v.storage_class == StorageClass::Output)
            .map(|(&id, v)| (id, v))
            .collect()
    }

    /// Get all uniform variables
    pub fn get_uniforms(&self) -> Vec<(Id, &VariableDecl)> {
        self.global_variables
            .iter()
            .filter(|(_, v)| v.storage_class == StorageClass::Uniform)
            .map(|(&id, v)| (id, v))
            .collect()
    }

    /// Get all storage buffers
    pub fn get_storage_buffers(&self) -> Vec<(Id, &VariableDecl)> {
        self.global_variables
            .iter()
            .filter(|(_, v)| v.storage_class == StorageClass::StorageBuffer)
            .map(|(&id, v)| (id, v))
            .collect()
    }

    /// Get all push constants
    pub fn get_push_constants(&self) -> Vec<(Id, &VariableDecl)> {
        self.global_variables
            .iter()
            .filter(|(_, v)| v.storage_class == StorageClass::PushConstant)
            .map(|(&id, v)| (id, v))
            .collect()
    }

    /// Count instructions
    pub fn instruction_count(&self) -> usize {
        let mut count = 0;
        count += self.capabilities.len();
        count += self.extensions.len();
        count += self.ext_inst_imports.len();
        count += 1; // Memory model
        count += self.entry_points.len();
        count += self.execution_modes.values().map(|v| v.len()).sum::<usize>();
        count += self.names.len();
        count += self.member_names.len();
        count += self.decorations.values().map(|v| v.len()).sum::<usize>();
        count += self.member_decorations.values().map(|v| v.len()).sum::<usize>();
        count += self.types.len();
        count += self.constants.len();
        count += self.global_variables.len();
        for func in &self.functions {
            count += 2; // OpFunction + OpFunctionEnd
            count += func.parameters.len();
            for block in &func.blocks {
                count += 1; // OpLabel
                count += block.instructions.len();
            }
        }
        count
    }
}

/// Entry point declaration
#[derive(Debug, Clone)]
pub struct EntryPoint {
    /// Execution model
    pub execution_model: ExecutionModel,
    /// Function ID
    pub function: Id,
    /// Entry point name
    pub name: String,
    /// Interface variables
    pub interface: Vec<Id>,
}

/// Execution mode declaration
#[derive(Debug, Clone)]
pub struct ExecutionModeDecl {
    /// Execution mode
    pub mode: ExecutionMode,
    /// Mode operands
    pub operands: Vec<u32>,
}

/// Source info
#[derive(Debug, Clone)]
pub struct SourceInfo {
    /// Source language
    pub language: SourceLanguage,
    /// Version
    pub version: u32,
    /// File ID
    pub file: Option<Id>,
    /// Source text
    pub source: Option<String>,
}

/// Source language
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SourceLanguage {
    Unknown = 0,
    ESSL = 1,
    GLSL = 2,
    OpenCL_C = 3,
    OpenCL_CPP = 4,
    HLSL = 5,
    CPP_for_OpenCL = 6,
}

/// Decoration declaration
#[derive(Debug, Clone)]
pub struct DecorationDecl {
    /// Decoration type
    pub decoration: Decoration,
    /// Decoration operands
    pub operands: Vec<u32>,
}

/// Type declaration
#[derive(Debug, Clone)]
pub struct TypeDecl {
    /// Type opcode
    pub opcode: Opcode,
    /// Type operands
    pub operands: Vec<Operand>,
}

/// Constant declaration
#[derive(Debug, Clone)]
pub struct ConstantDecl {
    /// Result type
    pub result_type: Id,
    /// Constant opcode
    pub opcode: Opcode,
    /// Constant operands
    pub operands: Vec<Operand>,
}

/// Variable declaration
#[derive(Debug, Clone)]
pub struct VariableDecl {
    /// Pointer type
    pub result_type: Id,
    /// Storage class
    pub storage_class: StorageClass,
    /// Initializer (if any)
    pub initializer: Option<Id>,
}

/// Function
#[derive(Debug, Clone)]
pub struct Function {
    /// Function ID
    pub id: Id,
    /// Return type
    pub return_type: Id,
    /// Function type
    pub function_type: Id,
    /// Parameters
    pub parameters: Vec<Parameter>,
    /// Basic blocks
    pub blocks: Vec<Block>,
}

impl Function {
    /// Get parameter by index
    pub fn get_parameter(&self, index: usize) -> Option<&Parameter> {
        self.parameters.get(index)
    }

    /// Get block by label
    pub fn get_block(&self, label: Id) -> Option<&Block> {
        self.blocks.iter().find(|b| b.label == label)
    }

    /// Get entry block
    pub fn entry_block(&self) -> Option<&Block> {
        self.blocks.first()
    }

    /// Count instructions
    pub fn instruction_count(&self) -> usize {
        self.blocks.iter().map(|b| b.instructions.len()).sum()
    }
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter ID
    pub id: Id,
    /// Parameter type
    pub param_type: Id,
}

/// Basic block
#[derive(Debug, Clone)]
pub struct Block {
    /// Block label
    pub label: Id,
    /// Instructions
    pub instructions: Vec<Instruction>,
}

impl Block {
    /// Get terminator instruction
    pub fn terminator(&self) -> Option<&Instruction> {
        self.instructions.last().filter(|inst| {
            matches!(
                inst.opcode,
                Opcode::OpBranch
                    | Opcode::OpBranchConditional
                    | Opcode::OpSwitch
                    | Opcode::OpReturn
                    | Opcode::OpReturnValue
                    | Opcode::OpKill
                    | Opcode::OpUnreachable
            )
        })
    }

    /// Check if block is terminated
    pub fn is_terminated(&self) -> bool {
        self.terminator().is_some()
    }

    /// Get successor labels
    pub fn successors(&self) -> Vec<Id> {
        match self.terminator() {
            Some(inst) => match inst.opcode {
                Opcode::OpBranch => {
                    if let Some(Operand::Id(target)) = inst.operands.first() {
                        vec![*target]
                    } else {
                        vec![]
                    }
                }
                Opcode::OpBranchConditional => {
                    let mut succs = vec![];
                    if let Some(Operand::Id(true_label)) = inst.operands.get(1) {
                        succs.push(*true_label);
                    }
                    if let Some(Operand::Id(false_label)) = inst.operands.get(2) {
                        succs.push(*false_label);
                    }
                    succs
                }
                Opcode::OpSwitch => {
                    let mut succs = vec![];
                    if let Some(Operand::Id(default)) = inst.operands.get(1) {
                        succs.push(*default);
                    }
                    // Parse case labels...
                    succs
                }
                _ => vec![],
            },
            None => vec![],
        }
    }
}

/// Module statistics
#[derive(Debug, Clone, Default)]
pub struct ModuleStats {
    /// Number of capabilities
    pub capability_count: usize,
    /// Number of extensions
    pub extension_count: usize,
    /// Number of entry points
    pub entry_point_count: usize,
    /// Number of types
    pub type_count: usize,
    /// Number of constants
    pub constant_count: usize,
    /// Number of global variables
    pub global_variable_count: usize,
    /// Number of functions
    pub function_count: usize,
    /// Total instruction count
    pub instruction_count: usize,
}

impl SpirVModule {
    /// Get module statistics
    pub fn stats(&self) -> ModuleStats {
        ModuleStats {
            capability_count: self.capabilities.len(),
            extension_count: self.extensions.len(),
            entry_point_count: self.entry_points.len(),
            type_count: self.types.len(),
            constant_count: self.constants.len(),
            global_variable_count: self.global_variables.len(),
            function_count: self.functions.len(),
            instruction_count: self.instruction_count(),
        }
    }
}

/// Module iterator for walking functions
pub struct FunctionIter<'a> {
    functions: core::slice::Iter<'a, Function>,
}

impl<'a> Iterator for FunctionIter<'a> {
    type Item = &'a Function;

    fn next(&mut self) -> Option<Self::Item> {
        self.functions.next()
    }
}

impl SpirVModule {
    /// Iterate over functions
    pub fn iter_functions(&self) -> FunctionIter<'_> {
        FunctionIter {
            functions: self.functions.iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SPIRV_VERSION_1_5;

    #[test]
    fn test_new_module() {
        let module = SpirVModule::new(SPIRV_VERSION_1_5, 0);
        assert_eq!(module.header.version, SPIRV_VERSION_1_5);
        assert!(module.functions.is_empty());
        assert!(module.entry_points.is_empty());
    }
}
