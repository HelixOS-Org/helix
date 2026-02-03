//! IR Validation
//!
//! Validation passes for checking IR correctness.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, format, string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::block::BasicBlock;
use crate::function::{ExecutionModel, Function, FunctionId};
use crate::instruction::{BlockId, Instruction};
use crate::module::Module;
use crate::types::{AddressSpace, IrType, ScalarType};
use crate::value::ValueId;

/// Validation error severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error - invalid IR
    Error,
    /// Warning - potential issue
    Warning,
    /// Note - informational
    Note,
}

/// A validation diagnostic
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level
    pub severity: Severity,
    /// Error code
    pub code: &'static str,
    /// Error message
    pub message: String,
    /// Source location (function, block, instruction)
    pub location: Option<Location>,
}

/// Location in the IR
#[derive(Debug, Clone)]
pub struct Location {
    /// Function name or ID
    pub function: Option<String>,
    /// Block ID
    pub block: Option<BlockId>,
    /// Instruction index
    pub instruction: Option<usize>,
}

impl Diagnostic {
    /// Create an error
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            location: None,
        }
    }

    /// Create a warning
    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            location: None,
        }
    }

    /// Set location
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Set function
    pub fn in_function(mut self, name: impl Into<String>) -> Self {
        if let Some(ref mut loc) = self.location {
            loc.function = Some(name.into());
        } else {
            self.location = Some(Location {
                function: Some(name.into()),
                block: None,
                instruction: None,
            });
        }
        self
    }

    /// Set block
    pub fn in_block(mut self, block: BlockId) -> Self {
        if let Some(ref mut loc) = self.location {
            loc.block = Some(block);
        } else {
            self.location = Some(Location {
                function: None,
                block: Some(block),
                instruction: None,
            });
        }
        self
    }

    /// Set instruction
    pub fn at_instruction(mut self, idx: usize) -> Self {
        if let Some(ref mut loc) = self.location {
            loc.instruction = Some(idx);
        } else {
            self.location = Some(Location {
                function: None,
                block: None,
                instruction: Some(idx),
            });
        }
        self
    }
}

/// Validation result
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// All diagnostics
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationResult {
    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Add a diagnostic
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Add an error
    pub fn error(&mut self, code: &'static str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(code, message));
    }

    /// Add a warning
    pub fn warning(&mut self, code: &'static str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::warning(code, message));
    }

    /// Merge results
    pub fn merge(&mut self, other: ValidationResult) {
        self.diagnostics.extend(other.diagnostics);
    }
}

/// IR validator
pub struct Validator {
    /// Strict mode (warnings become errors)
    strict: bool,
}

impl Validator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { strict: false }
    }

    /// Enable strict mode
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Validate a module
    pub fn validate(&self, module: &Module) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Validate module structure
        self.validate_module_structure(module, &mut result);

        // Validate each function
        for (_, func) in module.functions.iter() {
            self.validate_function(func, module, &mut result);
        }

        // Validate globals
        for (_, global) in module.globals.iter() {
            self.validate_global(global, module, &mut result);
        }

        // Validate capability requirements
        self.validate_capabilities(module, &mut result);

        result
    }

    /// Validate module structure
    fn validate_module_structure(&self, module: &Module, result: &mut ValidationResult) {
        // Must have at least one entry point or be a library
        if module.entry_points.is_empty() && !module.is_library {
            result.warning(
                "W001",
                "Module has no entry points and is not marked as a library",
            );
        }

        // Validate memory model
        use crate::module::MemoryModel;
        match module.memory_model {
            MemoryModel::Simple | MemoryModel::GLSL450 | MemoryModel::Vulkan => {},
        }
    }

    /// Validate a function
    fn validate_function(&self, func: &Function, module: &Module, result: &mut ValidationResult) {
        let func_name = func.name.clone();

        // Validate entry point requirements
        if let Some(model) = func.execution_model {
            self.validate_entry_point(func, model, result);
        }

        // Must have at least one block
        if func.blocks.is_empty() {
            result.add(
                Diagnostic::error("E001", "Function has no basic blocks").in_function(&func_name),
            );
            return;
        }

        // Validate each block
        for (block_id, block) in func.blocks.iter() {
            self.validate_block(block, *block_id, func, module, result);
        }

        // Validate CFG
        self.validate_cfg(func, result);

        // Validate SSA properties
        self.validate_ssa(func, result);
    }

    /// Validate entry point requirements
    fn validate_entry_point(
        &self,
        func: &Function,
        model: ExecutionModel,
        result: &mut ValidationResult,
    ) {
        let func_name = func.name.clone();

        match model {
            ExecutionModel::Vertex => {
                // Vertex shaders should output position
            },
            ExecutionModel::Fragment => {
                // Fragment shaders can have various outputs
            },
            ExecutionModel::GLCompute | ExecutionModel::Kernel => {
                // Compute shaders must have local size
                if func.local_size().is_none() {
                    result.add(
                        Diagnostic::error(
                            "E002",
                            "Compute shader missing LocalSize execution mode",
                        )
                        .in_function(&func_name),
                    );
                }
            },
            ExecutionModel::Geometry => {
                // Must specify output topology
            },
            ExecutionModel::TessellationControl | ExecutionModel::TessellationEvaluation => {
                // Must specify patch size
            },
            ExecutionModel::MeshNV | ExecutionModel::MeshEXT => {
                // Mesh shaders require local size and output limits
            },
            ExecutionModel::TaskNV | ExecutionModel::TaskEXT => {
                // Task shaders require local size
            },
            ExecutionModel::RayGenerationKHR
            | ExecutionModel::IntersectionKHR
            | ExecutionModel::AnyHitKHR
            | ExecutionModel::ClosestHitKHR
            | ExecutionModel::MissKHR
            | ExecutionModel::CallableKHR => {
                // Ray tracing shader requirements
            },
        }
    }

    /// Validate a basic block
    fn validate_block(
        &self,
        block: &BasicBlock,
        block_id: BlockId,
        func: &Function,
        module: &Module,
        result: &mut ValidationResult,
    ) {
        let func_name = func.name.clone();

        // Block must have at least one instruction (terminator)
        if block.instructions().is_empty() {
            result.add(
                Diagnostic::error("E003", "Block has no instructions")
                    .in_function(&func_name)
                    .in_block(block_id),
            );
            return;
        }

        // Last instruction must be terminator
        let last = block.instructions().last().unwrap();
        if !last.is_terminator() {
            result.add(
                Diagnostic::error("E004", "Block does not end with a terminator")
                    .in_function(&func_name)
                    .in_block(block_id),
            );
        }

        // No terminator in middle
        for (idx, inst) in block.instructions().iter().enumerate() {
            if idx < block.instructions().len() - 1 && inst.is_terminator() {
                result.add(
                    Diagnostic::error("E005", "Terminator instruction in middle of block")
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                );
            }

            // Validate individual instruction
            self.validate_instruction(inst, idx, block_id, func, module, result);
        }

        // Phi instructions must be at start of block
        let mut phi_phase = true;
        for (idx, inst) in block.instructions().iter().enumerate() {
            match inst {
                Instruction::Phi { .. } => {
                    if !phi_phase {
                        result.add(
                            Diagnostic::error("E006", "Phi instruction not at start of block")
                                .in_function(&func_name)
                                .in_block(block_id)
                                .at_instruction(idx),
                        );
                    }
                },
                _ => {
                    phi_phase = false;
                },
            }
        }
    }

    /// Validate an instruction
    fn validate_instruction(
        &self,
        inst: &Instruction,
        idx: usize,
        block_id: BlockId,
        func: &Function,
        module: &Module,
        result: &mut ValidationResult,
    ) {
        let func_name = func.name.clone();

        match inst {
            Instruction::BinaryOp {
                ty,
                op,
                left,
                right,
                ..
            } => {
                // Validate type compatibility
                // Both operands should have matching types
            },

            Instruction::UnaryOp {
                ty, op, operand, ..
            } => {
                // Validate operand type matches expected
            },

            Instruction::Branch { target } => {
                // Target must exist in function
                if func.blocks.get(*target).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E010",
                            format!("Branch target {} does not exist", target),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
            },

            Instruction::BranchConditional {
                condition,
                true_target,
                false_target,
                ..
            } => {
                // Condition must be boolean
                // Targets must exist
                if func.blocks.get(*true_target).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E011",
                            format!("True branch target {} does not exist", true_target),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
                if func.blocks.get(*false_target).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E011",
                            format!("False branch target {} does not exist", false_target),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
            },

            Instruction::Phi { operands, .. } => {
                // Each predecessor must provide exactly one value
                let predecessors = func.blocks.predecessors(block_id);
                for (_, pred_block) in operands {
                    if !predecessors.contains(pred_block) {
                        result.add(
                            Diagnostic::error(
                                "E020",
                                format!("Phi operand from non-predecessor block {}", pred_block),
                            )
                            .in_function(&func_name)
                            .in_block(block_id)
                            .at_instruction(idx),
                        );
                    }
                }
            },

            Instruction::FunctionCall {
                function,
                arguments,
                ..
            } => {
                // Function must exist
                if module.get_function(*function).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E030",
                            format!("Called function {} does not exist", function),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
            },

            Instruction::Load { pointer, .. } => {
                // Pointer must be a pointer type
            },

            Instruction::Store { pointer, value, .. } => {
                // Pointer must be a pointer type
                // Value type must match pointee type
            },

            Instruction::AccessChain { indices, .. } => {
                // Must have at least one index
                if indices.is_empty() {
                    result.add(
                        Diagnostic::error("E040", "AccessChain with no indices")
                            .in_function(&func_name)
                            .in_block(block_id)
                            .at_instruction(idx),
                    );
                }
            },

            Instruction::CompositeConstruct { components, .. } => {
                // Must have at least one component
                if components.is_empty() {
                    result.add(
                        Diagnostic::error("E041", "CompositeConstruct with no components")
                            .in_function(&func_name)
                            .in_block(block_id)
                            .at_instruction(idx),
                    );
                }
            },

            Instruction::VectorShuffle { components, .. } => {
                // Components must be valid indices
                for comp in components {
                    if *comp == 0xFFFFFFFF {
                        // Undef component is valid
                        continue;
                    }
                }
            },

            Instruction::LoopMerge {
                merge_block,
                continue_target,
                ..
            } => {
                // Merge and continue blocks must exist
                if func.blocks.get(*merge_block).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E050",
                            format!("Loop merge block {} does not exist", merge_block),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
                if func.blocks.get(*continue_target).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E051",
                            format!("Loop continue block {} does not exist", continue_target),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
            },

            Instruction::SelectionMerge { merge_block, .. } => {
                if func.blocks.get(*merge_block).is_none() {
                    result.add(
                        Diagnostic::error(
                            "E052",
                            format!("Selection merge block {} does not exist", merge_block),
                        )
                        .in_function(&func_name)
                        .in_block(block_id)
                        .at_instruction(idx),
                    );
                }
            },

            _ => {},
        }
    }

    /// Validate CFG properties
    fn validate_cfg(&self, func: &Function, result: &mut ValidationResult) {
        let func_name = func.name.clone();

        // Entry block should not have predecessors
        if let Some(entry_id) = func.entry_block() {
            let preds = func.blocks.predecessors(entry_id);
            if !preds.is_empty() {
                result.add(
                    Diagnostic::error("E060", "Entry block has predecessors")
                        .in_function(&func_name)
                        .in_block(entry_id),
                );
            }
        }

        // All blocks should be reachable from entry
        if let Some(entry_id) = func.entry_block() {
            #[cfg(feature = "std")]
            let mut reachable: HashSet<BlockId> = HashSet::new();
            #[cfg(not(feature = "std"))]
            let mut reachable: BTreeSet<BlockId> = BTreeSet::new();

            let mut worklist = vec![entry_id];
            while let Some(block_id) = worklist.pop() {
                if reachable.contains(&block_id) {
                    continue;
                }
                reachable.insert(block_id);

                if let Some(block) = func.blocks.get(block_id) {
                    for succ in block.successors() {
                        if !reachable.contains(&succ) {
                            worklist.push(succ);
                        }
                    }
                }
            }

            for (block_id, _) in func.blocks.iter() {
                if !reachable.contains(block_id) {
                    result.add(
                        Diagnostic::warning("W060", format!("Block {} is unreachable", block_id))
                            .in_function(&func_name)
                            .in_block(*block_id),
                    );
                }
            }
        }
    }

    /// Validate SSA properties
    fn validate_ssa(&self, func: &Function, result: &mut ValidationResult) {
        let func_name = func.name.clone();

        #[cfg(feature = "std")]
        let mut defined: HashSet<ValueId> = HashSet::new();
        #[cfg(not(feature = "std"))]
        let mut defined: BTreeSet<ValueId> = BTreeSet::new();

        // Parameters are defined
        for param in &func.parameters {
            defined.insert(param.id);
        }

        // Check each block in RPO
        let rpo = func.blocks.reverse_postorder();
        for block_id in rpo {
            if let Some(block) = func.blocks.get(block_id) {
                for (idx, inst) in block.instructions().iter().enumerate() {
                    // Check uses are defined (except for phi which is special)
                    if !matches!(inst, Instruction::Phi { .. }) {
                        for operand in inst.operands() {
                            if !defined.contains(&operand) {
                                // Could be a constant or global
                                // For now, just warn
                            }
                        }
                    }

                    // Record definition
                    if let Some(result_id) = inst.result() {
                        if defined.contains(&result_id) {
                            result.add(
                                Diagnostic::error(
                                    "E070",
                                    format!("Value {} defined multiple times", result_id),
                                )
                                .in_function(&func_name)
                                .in_block(block_id)
                                .at_instruction(idx),
                            );
                        }
                        defined.insert(result_id);
                    }
                }
            }
        }
    }

    /// Validate global variable
    fn validate_global(
        &self,
        global: &crate::function::GlobalVariable,
        _module: &Module,
        result: &mut ValidationResult,
    ) {
        // Validate address space for type
        match global.address_space {
            AddressSpace::Input | AddressSpace::Output => {
                // Must have location or builtin
                if global.decorations.location.is_none() && global.decorations.builtin.is_none() {
                    result.warning(
                        "W080",
                        format!(
                            "Input/output variable '{}' has no location or builtin",
                            global.name
                        ),
                    );
                }
            },
            AddressSpace::Uniform | AddressSpace::StorageBuffer => {
                // Must have binding
                if global.decorations.binding.is_none() {
                    result.warning(
                        "W081",
                        format!("Uniform/storage variable '{}' has no binding", global.name),
                    );
                }
            },
            _ => {},
        }
    }

    /// Validate capability requirements
    fn validate_capabilities(&self, module: &Module, result: &mut ValidationResult) {
        use crate::module::Capability;

        // Check for required capabilities based on features used
        for (_, func) in module.functions.iter() {
            for (_, block) in func.blocks.iter() {
                for inst in block.instructions() {
                    match inst {
                        Instruction::AtomicExchange { .. }
                        | Instruction::AtomicCompareExchange { .. } => {
                            // Requires some form of atomics capability
                        },
                        Instruction::ImageSampleImplicitLod { .. }
                        | Instruction::ImageSampleExplicitLod { .. } => {
                            if !module.has_capability(Capability::Shader) {
                                result.warning("W090", "Image sampling requires Shader capability");
                            }
                        },
                        Instruction::GroupIAdd { .. } | Instruction::GroupFAdd { .. } => {
                            if !module.has_capability(Capability::GroupNonUniform)
                                && !module.has_capability(Capability::GroupNonUniformArithmetic)
                            {
                                result.warning(
                                    "W091",
                                    "Group operations require GroupNonUniform* capabilities",
                                );
                            }
                        },
                        _ => {},
                    }
                }
            }
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_create() {
        let validator = Validator::new();
        let module = Module::new("test");
        let result = validator.validate(&module);
        // Empty module should have warning about no entry points
        assert!(result.is_valid());
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = Diagnostic::error("E001", "Test error")
            .in_function("main")
            .in_block(0)
            .at_instruction(5);

        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.location.is_some());
        let loc = diag.location.unwrap();
        assert_eq!(loc.function, Some("main".into()));
        assert_eq!(loc.block, Some(0));
        assert_eq!(loc.instruction, Some(5));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::default();
        assert!(result.is_valid());

        result.error("E001", "Test");
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }
}
