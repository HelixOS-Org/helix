//! SPIR-V Validation
//!
//! Validates SPIR-V modules for correctness.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, format, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet};

use crate::instruction::*;
use crate::module::{Block, Function, SpirVModule};
use crate::opcode::Opcode;
use crate::{SpirVError, SpirVResult, SPIRV_MAGIC};

/// Validation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Warning (valid but may cause issues)
    Warning,
    /// Error (invalid SPIR-V)
    Error,
}

/// Validation message
#[derive(Debug, Clone)]
pub struct ValidationMessage {
    /// Severity
    pub severity: Severity,
    /// Message
    pub message: String,
    /// Location (instruction index, if applicable)
    pub location: Option<ValidationLocation>,
}

/// Location of validation error
#[derive(Debug, Clone)]
pub struct ValidationLocation {
    /// Function ID
    pub function: Option<Id>,
    /// Block label
    pub block: Option<Id>,
    /// Instruction index within block
    pub instruction: Option<usize>,
}

/// Validation result
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Validation messages
    pub messages: Vec<ValidationMessage>,
}

impl ValidationResult {
    /// Create empty result
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    /// Add an error
    pub fn error(&mut self, message: impl Into<String>) {
        self.messages.push(ValidationMessage {
            severity: Severity::Error,
            message: message.into(),
            location: None,
        });
    }

    /// Add error with location
    pub fn error_at(
        &mut self,
        message: impl Into<String>,
        function: Option<Id>,
        block: Option<Id>,
        instruction: Option<usize>,
    ) {
        self.messages.push(ValidationMessage {
            severity: Severity::Error,
            message: message.into(),
            location: Some(ValidationLocation {
                function,
                block,
                instruction,
            }),
        });
    }

    /// Add a warning
    pub fn warning(&mut self, message: impl Into<String>) {
        self.messages.push(ValidationMessage {
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        });
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self.messages.iter().any(|m| m.severity == Severity::Error)
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.severity == Severity::Error)
            .count()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| m.severity == Severity::Warning)
            .count()
    }

    /// Merge with another result
    pub fn merge(&mut self, other: ValidationResult) {
        self.messages.extend(other.messages);
    }
}

/// SPIR-V validator
#[derive(Debug, Default)]
pub struct Validator {
    /// Validation options
    options: ValidationOptions,
}

/// Validation options
#[derive(Debug, Clone, Default)]
pub struct ValidationOptions {
    /// Allow relaxed logical pointer (OpPtrAccessChain)
    pub relaxed_logical_pointer: bool,
    /// Skip layout validation
    pub skip_layout_validation: bool,
    /// Allow scalar block layout
    pub scalar_block_layout: bool,
    /// Allow workgroup memory explicit layout
    pub workgroup_memory_explicit_layout: bool,
    /// Vulkan 1.0 rules
    pub vulkan_1_0: bool,
    /// Vulkan 1.1 rules
    pub vulkan_1_1: bool,
    /// Vulkan 1.2 rules
    pub vulkan_1_2: bool,
    /// Vulkan 1.3 rules
    pub vulkan_1_3: bool,
}

impl Validator {
    /// Create a new validator
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with options
    pub fn with_options(options: ValidationOptions) -> Self {
        Self { options }
    }

    /// Validate a module
    pub fn validate(&self, module: &SpirVModule) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Validate header
        self.validate_header(module, &mut result);

        // Validate capabilities
        self.validate_capabilities(module, &mut result);

        // Validate memory model
        self.validate_memory_model(module, &mut result);

        // Validate entry points
        self.validate_entry_points(module, &mut result);

        // Validate types
        self.validate_types(module, &mut result);

        // Validate functions
        for func in &module.functions {
            self.validate_function(module, func, &mut result);
        }

        // Validate ID usage
        self.validate_id_usage(module, &mut result);

        // Validate decorations
        self.validate_decorations(module, &mut result);

        result
    }

    /// Validate header
    fn validate_header(&self, module: &SpirVModule, result: &mut ValidationResult) {
        // Check version
        let major = module.header.major_version();
        let minor = module.header.minor_version();

        if major < 1 {
            result.error(format!("Invalid SPIR-V version: {}.{}", major, minor));
        }

        // Check bound
        if module.header.bound == 0 {
            result.error("Invalid bound: 0");
        }

        // Check schema
        if module.header.schema != 0 {
            result.error(format!("Invalid schema: {}", module.header.schema));
        }
    }

    /// Validate capabilities
    fn validate_capabilities(&self, module: &SpirVModule, result: &mut ValidationResult) {
        if module.capabilities.is_empty() {
            result.error("Module has no capabilities");
        }

        // Check for required capabilities based on Vulkan version
        if self.options.vulkan_1_0 || self.options.vulkan_1_1 {
            // Shader capability is required
            if !module.capabilities.contains(&Capability::Shader) {
                result.warning("Shader capability not declared (required for Vulkan)");
            }
        }
    }

    /// Validate memory model
    fn validate_memory_model(&self, module: &SpirVModule, result: &mut ValidationResult) {
        // Check for valid memory model
        match module.memory_model {
            MemoryModel::Simple => {
                result.warning("Simple memory model may not be supported by all implementations");
            },
            MemoryModel::GLSL450 => {
                // Common for graphics shaders
            },
            MemoryModel::OpenCL => {
                if self.options.vulkan_1_0 || self.options.vulkan_1_1 {
                    result.error("OpenCL memory model not valid for Vulkan");
                }
            },
            MemoryModel::Vulkan => {
                // Requires VulkanMemoryModel capability
                if !module.capabilities.contains(&Capability::VulkanMemoryModel) {
                    result.error("Vulkan memory model requires VulkanMemoryModel capability");
                }
            },
        }
    }

    /// Validate entry points
    fn validate_entry_points(&self, module: &SpirVModule, result: &mut ValidationResult) {
        if module.entry_points.is_empty() {
            result.warning("Module has no entry points");
        }

        for ep in &module.entry_points {
            // Check function exists
            if module.get_function(ep.function).is_none() {
                result.error(format!(
                    "Entry point '{}' references undefined function {}",
                    ep.name, ep.function
                ));
            }

            // Check interface variables exist
            for &iface in &ep.interface {
                if !module.global_variables.contains_key(&iface) {
                    result.error(format!(
                        "Entry point '{}' references undefined interface variable {}",
                        ep.name, iface
                    ));
                }
            }

            // Validate execution model specific requirements
            self.validate_execution_model(module, ep, result);
        }
    }

    /// Validate execution model requirements
    fn validate_execution_model(
        &self,
        module: &SpirVModule,
        ep: &crate::module::EntryPoint,
        result: &mut ValidationResult,
    ) {
        match ep.execution_model {
            ExecutionModel::Vertex => {
                // Vertex shaders need Position output
                let has_position = ep
                    .interface
                    .iter()
                    .any(|&id| module.get_builtin(id) == Some(crate::types::BuiltIn::Position));
                if !has_position {
                    result.warning(format!(
                        "Vertex shader '{}' may need Position output",
                        ep.name
                    ));
                }
            },
            ExecutionModel::Fragment => {
                // Check for OriginUpperLeft or OriginLowerLeft
                let has_origin = module
                    .execution_modes
                    .get(&ep.function)
                    .map(|modes| {
                        modes.iter().any(|m| {
                            matches!(
                                m.mode,
                                crate::types::ExecutionMode::OriginUpperLeft
                                    | crate::types::ExecutionMode::OriginLowerLeft
                            )
                        })
                    })
                    .unwrap_or(false);
                if !has_origin {
                    result.error(format!(
                        "Fragment shader '{}' must have OriginUpperLeft or OriginLowerLeft",
                        ep.name
                    ));
                }
            },
            ExecutionModel::GLCompute => {
                // Check for LocalSize
                let has_local_size = module
                    .execution_modes
                    .get(&ep.function)
                    .map(|modes| {
                        modes
                            .iter()
                            .any(|m| matches!(m.mode, crate::types::ExecutionMode::LocalSize))
                    })
                    .unwrap_or(false);
                if !has_local_size {
                    result.error(format!(
                        "Compute shader '{}' must have LocalSize execution mode",
                        ep.name
                    ));
                }
            },
            _ => {},
        }
    }

    /// Validate types
    fn validate_types(&self, module: &SpirVModule, result: &mut ValidationResult) {
        for (&id, type_decl) in &module.types {
            match type_decl.opcode {
                Opcode::OpTypeVector => {
                    // Vector component count must be 2, 3, or 4
                    if let Some(Operand::Literal(count)) = type_decl.operands.get(1) {
                        if *count < 2 || *count > 4 {
                            result.error(format!(
                                "Type {} has invalid vector component count: {}",
                                id, count
                            ));
                        }
                    }
                },
                Opcode::OpTypeMatrix => {
                    // Matrix column count must be 2, 3, or 4
                    if let Some(Operand::Literal(count)) = type_decl.operands.get(1) {
                        if *count < 2 || *count > 4 {
                            result.error(format!(
                                "Type {} has invalid matrix column count: {}",
                                id, count
                            ));
                        }
                    }
                    // Column type must be a vector
                    if let Some(Operand::Id(col_type)) = type_decl.operands.first() {
                        if let Some(col_decl) = module.types.get(col_type) {
                            if col_decl.opcode != Opcode::OpTypeVector {
                                result.error(format!(
                                    "Matrix type {} column type must be a vector",
                                    id
                                ));
                            }
                        }
                    }
                },
                Opcode::OpTypeInt => {
                    // Width must be 8, 16, 32, or 64
                    if let Some(Operand::Literal(width)) = type_decl.operands.first() {
                        if !matches!(width, 8 | 16 | 32 | 64) {
                            result
                                .error(format!("Type {} has invalid integer width: {}", id, width));
                        }
                    }
                },
                Opcode::OpTypeFloat => {
                    // Width must be 16, 32, or 64
                    if let Some(Operand::Literal(width)) = type_decl.operands.first() {
                        if !matches!(width, 16 | 32 | 64) {
                            result.error(format!("Type {} has invalid float width: {}", id, width));
                        }
                    }
                },
                _ => {},
            }
        }
    }

    /// Validate a function
    fn validate_function(
        &self,
        module: &SpirVModule,
        func: &Function,
        result: &mut ValidationResult,
    ) {
        // Check function type exists
        if !module.types.contains_key(&func.function_type) {
            result.error_at(
                format!(
                    "Function {} references undefined type {}",
                    func.id, func.function_type
                ),
                Some(func.id),
                None,
                None,
            );
        }

        // Check return type exists
        if !module.types.contains_key(&func.return_type) {
            result.error_at(
                format!(
                    "Function {} references undefined return type {}",
                    func.id, func.return_type
                ),
                Some(func.id),
                None,
                None,
            );
        }

        // Validate blocks
        if func.blocks.is_empty() {
            result.warning(format!("Function {} has no blocks", func.id));
        } else {
            // First block is entry block
            let entry = &func.blocks[0];

            // Check all blocks are terminated
            for block in &func.blocks {
                self.validate_block(module, func, block, result);
            }

            // Check for unreachable blocks
            self.validate_block_reachability(module, func, result);
        }
    }

    /// Validate a basic block
    fn validate_block(
        &self,
        module: &SpirVModule,
        func: &Function,
        block: &Block,
        result: &mut ValidationResult,
    ) {
        // Block must be terminated
        if !block.is_terminated() {
            result.error_at(
                format!("Block {} is not terminated", block.label),
                Some(func.id),
                Some(block.label),
                None,
            );
        }

        // Validate each instruction
        for (idx, inst) in block.instructions.iter().enumerate() {
            self.validate_instruction(module, func, block, inst, idx, result);
        }

        // Check no instructions after terminator
        if let Some(term_idx) = block.instructions.iter().position(|inst| {
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
        }) {
            if term_idx < block.instructions.len() - 1 {
                result.error_at(
                    format!("Block {} has instructions after terminator", block.label),
                    Some(func.id),
                    Some(block.label),
                    Some(term_idx + 1),
                );
            }
        }
    }

    /// Validate a single instruction
    fn validate_instruction(
        &self,
        module: &SpirVModule,
        func: &Function,
        block: &Block,
        inst: &Instruction,
        idx: usize,
        result: &mut ValidationResult,
    ) {
        // Check result type exists
        if let Some(rt) = inst.result_type {
            if !module.types.contains_key(&rt) {
                result.error_at(
                    format!("Instruction uses undefined result type {}", rt),
                    Some(func.id),
                    Some(block.label),
                    Some(idx),
                );
            }
        }

        // Instruction-specific validation
        match inst.opcode {
            Opcode::OpBranch => {
                if let Some(Operand::Id(target)) = inst.operands.first() {
                    if !func.blocks.iter().any(|b| b.label == *target) {
                        result.error_at(
                            format!("Branch target {} not found in function", target),
                            Some(func.id),
                            Some(block.label),
                            Some(idx),
                        );
                    }
                }
            },
            Opcode::OpBranchConditional => {
                // Check condition is bool
                // Check both targets exist
                for target_idx in [1, 2] {
                    if let Some(Operand::Id(target)) = inst.operands.get(target_idx) {
                        if !func.blocks.iter().any(|b| b.label == *target) {
                            result.error_at(
                                format!("Branch target {} not found in function", target),
                                Some(func.id),
                                Some(block.label),
                                Some(idx),
                            );
                        }
                    }
                }
            },
            Opcode::OpReturnValue => {
                // Check return value type matches function return type
                // Would need type tracking to verify
            },
            Opcode::OpPhi => {
                // Phi must be at start of block (after OpLabel)
                if idx > 0 && !matches!(block.instructions[idx - 1].opcode, Opcode::OpPhi) {
                    // This is okay if previous was another phi
                }
                // Check all incoming block labels are valid
                let mut i = 0;
                while i + 1 < inst.operands.len() {
                    if let Some(Operand::Id(block_id)) = inst.operands.get(i + 1) {
                        if !func.blocks.iter().any(|b| b.label == *block_id) {
                            result.error_at(
                                format!("Phi references undefined block {}", block_id),
                                Some(func.id),
                                Some(block.label),
                                Some(idx),
                            );
                        }
                    }
                    i += 2;
                }
            },
            Opcode::OpSelectionMerge => {
                // Must be immediately followed by branching instruction
                if idx + 1 >= block.instructions.len() {
                    result.error_at(
                        "OpSelectionMerge must be followed by branch",
                        Some(func.id),
                        Some(block.label),
                        Some(idx),
                    );
                }
            },
            Opcode::OpLoopMerge => {
                // Must be immediately followed by branching instruction
                if idx + 1 >= block.instructions.len() {
                    result.error_at(
                        "OpLoopMerge must be followed by branch",
                        Some(func.id),
                        Some(block.label),
                        Some(idx),
                    );
                }
            },
            _ => {},
        }
    }

    /// Validate block reachability
    fn validate_block_reachability(
        &self,
        _module: &SpirVModule,
        func: &Function,
        result: &mut ValidationResult,
    ) {
        if func.blocks.is_empty() {
            return;
        }

        // BFS from entry block
        let mut reachable: BTreeSet<Id> = BTreeSet::new();
        let mut worklist: Vec<Id> = vec![func.blocks[0].label];

        while let Some(label) = worklist.pop() {
            if reachable.contains(&label) {
                continue;
            }
            reachable.insert(label);

            if let Some(block) = func.blocks.iter().find(|b| b.label == label) {
                for succ in block.successors() {
                    if !reachable.contains(&succ) {
                        worklist.push(succ);
                    }
                }
            }
        }

        // Check for unreachable blocks
        for block in &func.blocks {
            if !reachable.contains(&block.label) {
                result.warning(format!(
                    "Block {} in function {} is unreachable",
                    block.label, func.id
                ));
            }
        }
    }

    /// Validate ID usage
    fn validate_id_usage(&self, module: &SpirVModule, result: &mut ValidationResult) {
        let mut defined: BTreeSet<Id> = BTreeSet::new();

        // Collect all defined IDs
        for &id in module.types.keys() {
            defined.insert(id);
        }
        for &id in module.constants.keys() {
            defined.insert(id);
        }
        for &id in module.global_variables.keys() {
            defined.insert(id);
        }
        for (&id, _) in &module.ext_inst_imports {
            defined.insert(id);
        }
        for func in &module.functions {
            defined.insert(func.id);
            for param in &func.parameters {
                defined.insert(param.id);
            }
            for block in &func.blocks {
                defined.insert(block.label);
                for inst in &block.instructions {
                    if let Some(r) = inst.result {
                        defined.insert(r);
                    }
                }
            }
        }

        // Check all used IDs are defined
        // This would require tracking uses and comparing to definitions
    }

    /// Validate decorations
    fn validate_decorations(&self, module: &SpirVModule, result: &mut ValidationResult) {
        for (&target, decs) in &module.decorations {
            // Check target exists
            let target_exists = module.types.contains_key(&target)
                || module.constants.contains_key(&target)
                || module.global_variables.contains_key(&target)
                || module.functions.iter().any(|f| f.id == target);

            if !target_exists {
                result.warning(format!("Decoration targets undefined ID {}", target));
            }

            // Check for conflicting decorations
            for dec in decs {
                match dec.decoration {
                    Decoration::RowMajor | Decoration::ColMajor => {
                        // Can't have both
                        let has_row = decs.iter().any(|d| d.decoration == Decoration::RowMajor);
                        let has_col = decs.iter().any(|d| d.decoration == Decoration::ColMajor);
                        if has_row && has_col {
                            result.error(format!(
                                "ID {} has both RowMajor and ColMajor decorations",
                                target
                            ));
                        }
                    },
                    _ => {},
                }
            }
        }
    }
}

/// Validate binary directly
pub fn validate_binary(words: &[u32]) -> SpirVResult<ValidationResult> {
    // Check magic number
    if words.is_empty() || words[0] != SPIRV_MAGIC {
        return Err(SpirVError::Validation("Invalid SPIR-V magic number".into()));
    }

    // Parse and validate
    let module = SpirVModule::from_binary(words)?;
    let validator = Validator::new();
    Ok(validator.validate(&module))
}

/// Validate bytes directly
pub fn validate_bytes(bytes: &[u8]) -> SpirVResult<ValidationResult> {
    if bytes.len() < 20 {
        return Err(SpirVError::Validation("SPIR-V too short for header".into()));
    }

    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    validate_binary(&words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);

        result.warning("test warning");
        assert!(result.is_valid());
        assert_eq!(result.warning_count(), 1);

        result.error("test error");
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_options() {
        let options = ValidationOptions {
            vulkan_1_2: true,
            ..Default::default()
        };
        let validator = Validator::with_options(options);
        // Validator created successfully
    }
}
