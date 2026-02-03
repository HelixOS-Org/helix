//! SPIR-V Disassembler
//!
//! Human-readable SPIR-V text output.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

use core::fmt::Write;

use crate::{
    binary::BinaryDecoder,
    instruction::*,
    module::SpirVModule,
    opcode::Opcode,
    SpirVResult,
};

/// Disassembler options
#[derive(Debug, Clone, Default)]
pub struct DisassemblerOptions {
    /// Show instruction byte offsets
    pub show_offsets: bool,
    /// Show instruction result IDs
    pub show_results: bool,
    /// Indent function bodies
    pub indent: bool,
    /// Show friendly names (from OpName)
    pub friendly_names: bool,
    /// Color output (ANSI codes)
    pub color: bool,
    /// Show operand types
    pub show_types: bool,
    /// Comment style (";", "//", or "")
    pub comment_style: &'static str,
}

impl DisassemblerOptions {
    /// Default options
    pub fn default() -> Self {
        Self {
            show_offsets: false,
            show_results: true,
            indent: true,
            friendly_names: true,
            color: false,
            show_types: false,
            comment_style: ";",
        }
    }

    /// Verbose output
    pub fn verbose() -> Self {
        Self {
            show_offsets: true,
            show_results: true,
            indent: true,
            friendly_names: true,
            color: true,
            show_types: true,
            comment_style: ";",
        }
    }

    /// Minimal output
    pub fn minimal() -> Self {
        Self {
            show_offsets: false,
            show_results: false,
            indent: false,
            friendly_names: false,
            color: false,
            show_types: false,
            comment_style: "",
        }
    }
}

/// SPIR-V disassembler
pub struct Disassembler {
    options: DisassemblerOptions,
}

impl Disassembler {
    /// Create a new disassembler
    pub fn new() -> Self {
        Self {
            options: DisassemblerOptions::default(),
        }
    }

    /// Create with options
    pub fn with_options(options: DisassemblerOptions) -> Self {
        Self { options }
    }

    /// Disassemble a module to string
    pub fn disassemble(&self, module: &SpirVModule) -> String {
        let mut output = String::new();
        self.disassemble_to(module, &mut output);
        output
    }

    /// Disassemble to a writer
    pub fn disassemble_to(&self, module: &SpirVModule, output: &mut String) {
        // Header comment
        self.write_header(module, output);

        // Capabilities
        for cap in &module.capabilities {
            writeln!(output, "OpCapability {}", capability_name(*cap)).ok();
        }
        if !module.capabilities.is_empty() {
            writeln!(output).ok();
        }

        // Extensions
        for ext in &module.extensions {
            writeln!(output, "OpExtension \"{}\"", ext).ok();
        }

        // Extended instruction imports
        for (id, name) in &module.ext_inst_imports {
            writeln!(output, "%{} = OpExtInstImport \"{}\"", id, name).ok();
        }
        if !module.ext_inst_imports.is_empty() {
            writeln!(output).ok();
        }

        // Memory model
        writeln!(
            output,
            "OpMemoryModel {} {}",
            addressing_model_name(module.addressing_model),
            memory_model_name(module.memory_model)
        )
        .ok();
        writeln!(output).ok();

        // Entry points
        for ep in &module.entry_points {
            write!(
                output,
                "OpEntryPoint {} %{} \"{}\"",
                execution_model_name(ep.execution_model),
                ep.function,
                ep.name
            )
            .ok();
            for &iface in &ep.interface {
                write!(output, " %{}", iface).ok();
            }
            writeln!(output).ok();
        }

        // Execution modes
        for (&func_id, modes) in &module.execution_modes {
            for mode in modes {
                write!(output, "OpExecutionMode %{} {:?}", func_id, mode.mode).ok();
                for &op in &mode.operands {
                    write!(output, " {}", op).ok();
                }
                writeln!(output).ok();
            }
        }
        writeln!(output).ok();

        // Debug info
        self.write_debug_section(module, output);

        // Decorations
        self.write_decorations(module, output);

        // Types and constants
        self.write_types(module, output);

        // Global variables
        self.write_global_variables(module, output);

        // Functions
        self.write_functions(module, output);
    }

    /// Write header comment
    fn write_header(&self, module: &SpirVModule, output: &mut String) {
        if !self.options.comment_style.is_empty() {
            writeln!(
                output,
                "{} SPIR-V",
                self.options.comment_style
            )
            .ok();
            writeln!(
                output,
                "{} Version: {}.{}",
                self.options.comment_style,
                module.header.major_version(),
                module.header.minor_version()
            )
            .ok();
            writeln!(
                output,
                "{} Generator: 0x{:08x}",
                self.options.comment_style,
                module.header.generator
            )
            .ok();
            writeln!(
                output,
                "{} Bound: {}",
                self.options.comment_style,
                module.header.bound
            )
            .ok();
            writeln!(
                output,
                "{} Schema: {}",
                self.options.comment_style,
                module.header.schema
            )
            .ok();
            writeln!(output).ok();
        }
    }

    /// Write debug section (names)
    fn write_debug_section(&self, module: &SpirVModule, output: &mut String) {
        // Names
        for (&id, name) in &module.names {
            writeln!(output, "OpName %{} \"{}\"", id, name).ok();
        }

        // Member names
        for (&(struct_id, member), name) in &module.member_names {
            writeln!(output, "OpMemberName %{} {} \"{}\"", struct_id, member, name).ok();
        }

        if !module.names.is_empty() || !module.member_names.is_empty() {
            writeln!(output).ok();
        }
    }

    /// Write decorations
    fn write_decorations(&self, module: &SpirVModule, output: &mut String) {
        for (&target, decs) in &module.decorations {
            for dec in decs {
                write!(output, "OpDecorate %{} {:?}", target, dec.decoration).ok();
                for &op in &dec.operands {
                    write!(output, " {}", op).ok();
                }
                writeln!(output).ok();
            }
        }

        for (&(struct_id, member), decs) in &module.member_decorations {
            for dec in decs {
                write!(
                    output,
                    "OpMemberDecorate %{} {} {:?}",
                    struct_id, member, dec.decoration
                )
                .ok();
                for &op in &dec.operands {
                    write!(output, " {}", op).ok();
                }
                writeln!(output).ok();
            }
        }

        if !module.decorations.is_empty() || !module.member_decorations.is_empty() {
            writeln!(output).ok();
        }
    }

    /// Write types
    fn write_types(&self, module: &SpirVModule, output: &mut String) {
        for (&id, type_decl) in &module.types {
            write!(output, "%{} = {}", id, type_decl.opcode.name()).ok();
            for op in &type_decl.operands {
                write!(output, " {}", self.format_operand(op)).ok();
            }
            // Add friendly name if available
            if self.options.friendly_names {
                if let Some(name) = module.names.get(&id) {
                    write!(output, " {} {}", self.options.comment_style, name).ok();
                }
            }
            writeln!(output).ok();
        }

        // Constants
        for (&id, constant) in &module.constants {
            write!(output, "%{} = {} %{}", id, constant.opcode.name(), constant.result_type).ok();
            for op in &constant.operands {
                write!(output, " {}", self.format_operand(op)).ok();
            }
            if self.options.friendly_names {
                if let Some(name) = module.names.get(&id) {
                    write!(output, " {} {}", self.options.comment_style, name).ok();
                }
            }
            writeln!(output).ok();
        }

        writeln!(output).ok();
    }

    /// Write global variables
    fn write_global_variables(&self, module: &SpirVModule, output: &mut String) {
        for (&id, var) in &module.global_variables {
            write!(
                output,
                "%{} = OpVariable %{} {}",
                id,
                var.result_type,
                storage_class_name(var.storage_class)
            )
            .ok();
            if let Some(init) = var.initializer {
                write!(output, " %{}", init).ok();
            }
            if self.options.friendly_names {
                if let Some(name) = module.names.get(&id) {
                    write!(output, " {} {}", self.options.comment_style, name).ok();
                }
            }
            writeln!(output).ok();
        }

        if !module.global_variables.is_empty() {
            writeln!(output).ok();
        }
    }

    /// Write functions
    fn write_functions(&self, module: &SpirVModule, output: &mut String) {
        for func in &module.functions {
            // Function header
            write!(
                output,
                "%{} = OpFunction %{} None %{}",
                func.id, func.return_type, func.function_type
            )
            .ok();
            if self.options.friendly_names {
                if let Some(name) = module.names.get(&func.id) {
                    write!(output, " {} {}", self.options.comment_style, name).ok();
                }
            }
            writeln!(output).ok();

            // Parameters
            for param in &func.parameters {
                let indent = if self.options.indent { "    " } else { "" };
                writeln!(
                    output,
                    "{}%{} = OpFunctionParameter %{}",
                    indent, param.id, param.param_type
                )
                .ok();
            }

            // Blocks
            for block in &func.blocks {
                let indent = if self.options.indent { "  " } else { "" };
                writeln!(output, "{}%{} = OpLabel", indent, block.label).ok();

                for inst in &block.instructions {
                    self.write_instruction(module, inst, output);
                }
            }

            // Function end
            writeln!(output, "OpFunctionEnd").ok();
            writeln!(output).ok();
        }
    }

    /// Write a single instruction
    fn write_instruction(&self, module: &SpirVModule, inst: &Instruction, output: &mut String) {
        let indent = if self.options.indent { "    " } else { "" };

        // Result assignment
        if let Some(result) = inst.result {
            if let Some(result_type) = inst.result_type {
                write!(output, "{}%{} = {} %{}", indent, result, inst.opcode.name(), result_type)
                    .ok();
            } else {
                write!(output, "{}%{} = {}", indent, result, inst.opcode.name()).ok();
            }
        } else {
            write!(output, "{}{}", indent, inst.opcode.name()).ok();
        }

        // Operands
        for op in &inst.operands {
            write!(output, " {}", self.format_operand(op)).ok();
        }

        writeln!(output).ok();
    }

    /// Format an operand for display
    fn format_operand(&self, op: &Operand) -> String {
        match op {
            Operand::Id(id) => format!("%{}", id),
            Operand::Literal(v) => format!("{}", v),
            Operand::Literal64(v) => format!("{}", v),
            Operand::String(s) => format!("\"{}\"", s),
            Operand::ExecutionModel(m) => execution_model_name(*m).to_string(),
            Operand::AddressingModel(m) => addressing_model_name(*m).to_string(),
            Operand::MemoryModel(m) => memory_model_name(*m).to_string(),
            Operand::StorageClass(c) => storage_class_name(*c).to_string(),
            Operand::Decoration(d) => format!("{:?}", d),
            Operand::Capability(c) => capability_name(*c).to_string(),
            Operand::Dim(d) => format!("{:?}", d),
            Operand::ImageFormat(f) => format!("{:?}", f),
            Operand::FunctionControl(f) => format!("{}", f.bits()),
            Operand::MemoryAccess(m) => format!("{}", m.bits()),
            Operand::SelectionControl(s) => format!("{}", s.bits()),
            Operand::LoopControl(l) => format!("{}", l.bits()),
            Operand::Scope(s) => format!("{:?}", s),
            Operand::MemorySemantics(m) => format!("{}", m.bits()),
            Operand::GroupOperation(g) => format!("{:?}", g),
        }
    }
}

impl Default for Disassembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Disassemble binary to string
pub fn disassemble(words: &[u32]) -> SpirVResult<String> {
    let module = SpirVModule::from_binary(words)?;
    let disasm = Disassembler::new();
    Ok(disasm.disassemble(&module))
}

/// Disassemble bytes to string
pub fn disassemble_bytes(bytes: &[u8]) -> SpirVResult<String> {
    let module = SpirVModule::from_bytes(bytes)?;
    let disasm = Disassembler::new();
    Ok(disasm.disassemble(&module))
}

// Helper functions for name formatting

fn capability_name(cap: Capability) -> &'static str {
    match cap {
        Capability::Matrix => "Matrix",
        Capability::Shader => "Shader",
        Capability::Geometry => "Geometry",
        Capability::Tessellation => "Tessellation",
        Capability::Addresses => "Addresses",
        Capability::Linkage => "Linkage",
        Capability::Kernel => "Kernel",
        Capability::Float16 => "Float16",
        Capability::Float64 => "Float64",
        Capability::Int64 => "Int64",
        Capability::Int16 => "Int16",
        Capability::Int8 => "Int8",
        Capability::ImageBasic => "ImageBasic",
        Capability::ImageReadWrite => "ImageReadWrite",
        Capability::Sampled1D => "Sampled1D",
        Capability::SampledBuffer => "SampledBuffer",
        Capability::SampledCubeArray => "SampledCubeArray",
        Capability::ImageCubeArray => "ImageCubeArray",
        Capability::SampledRect => "SampledRect",
        Capability::InputAttachment => "InputAttachment",
        Capability::SparseResidency => "SparseResidency",
        Capability::MinLod => "MinLod",
        Capability::ImageQuery => "ImageQuery",
        Capability::DerivativeControl => "DerivativeControl",
        Capability::InterpolationFunction => "InterpolationFunction",
        Capability::TransformFeedback => "TransformFeedback",
        Capability::StorageImageExtendedFormats => "StorageImageExtendedFormats",
        Capability::StorageImageReadWithoutFormat => "StorageImageReadWithoutFormat",
        Capability::StorageImageWriteWithoutFormat => "StorageImageWriteWithoutFormat",
        Capability::MultiViewport => "MultiViewport",
        Capability::DrawParameters => "DrawParameters",
        Capability::MultiView => "MultiView",
        Capability::DeviceGroup => "DeviceGroup",
        Capability::VariablePointers => "VariablePointers",
        Capability::VariablePointersStorageBuffer => "VariablePointersStorageBuffer",
        Capability::ShaderClockKHR => "ShaderClockKHR",
        Capability::FragmentShadingRateKHR => "FragmentShadingRateKHR",
        Capability::RayTracingKHR => "RayTracingKHR",
        Capability::RayQueryKHR => "RayQueryKHR",
        Capability::MeshShadingEXT => "MeshShadingEXT",
        Capability::MeshShadingNV => "MeshShadingNV",
        Capability::VulkanMemoryModel => "VulkanMemoryModel",
        _ => "Unknown",
    }
}

fn execution_model_name(model: ExecutionModel) -> &'static str {
    match model {
        ExecutionModel::Vertex => "Vertex",
        ExecutionModel::TessellationControl => "TessellationControl",
        ExecutionModel::TessellationEvaluation => "TessellationEvaluation",
        ExecutionModel::Geometry => "Geometry",
        ExecutionModel::Fragment => "Fragment",
        ExecutionModel::GLCompute => "GLCompute",
        ExecutionModel::Kernel => "Kernel",
        ExecutionModel::TaskNV => "TaskNV",
        ExecutionModel::MeshNV => "MeshNV",
        ExecutionModel::RayGenerationKHR => "RayGenerationKHR",
        ExecutionModel::IntersectionKHR => "IntersectionKHR",
        ExecutionModel::AnyHitKHR => "AnyHitKHR",
        ExecutionModel::ClosestHitKHR => "ClosestHitKHR",
        ExecutionModel::MissKHR => "MissKHR",
        ExecutionModel::CallableKHR => "CallableKHR",
        ExecutionModel::TaskEXT => "TaskEXT",
        ExecutionModel::MeshEXT => "MeshEXT",
    }
}

fn addressing_model_name(model: AddressingModel) -> &'static str {
    match model {
        AddressingModel::Logical => "Logical",
        AddressingModel::Physical32 => "Physical32",
        AddressingModel::Physical64 => "Physical64",
        AddressingModel::PhysicalStorageBuffer64 => "PhysicalStorageBuffer64",
    }
}

fn memory_model_name(model: MemoryModel) -> &'static str {
    match model {
        MemoryModel::Simple => "Simple",
        MemoryModel::GLSL450 => "GLSL450",
        MemoryModel::OpenCL => "OpenCL",
        MemoryModel::Vulkan => "Vulkan",
    }
}

fn storage_class_name(class: StorageClass) -> &'static str {
    match class {
        StorageClass::UniformConstant => "UniformConstant",
        StorageClass::Input => "Input",
        StorageClass::Uniform => "Uniform",
        StorageClass::Output => "Output",
        StorageClass::Workgroup => "Workgroup",
        StorageClass::CrossWorkgroup => "CrossWorkgroup",
        StorageClass::Private => "Private",
        StorageClass::Function => "Function",
        StorageClass::Generic => "Generic",
        StorageClass::PushConstant => "PushConstant",
        StorageClass::AtomicCounter => "AtomicCounter",
        StorageClass::Image => "Image",
        StorageClass::StorageBuffer => "StorageBuffer",
        StorageClass::CallableDataKHR => "CallableDataKHR",
        StorageClass::IncomingCallableDataKHR => "IncomingCallableDataKHR",
        StorageClass::RayPayloadKHR => "RayPayloadKHR",
        StorageClass::HitAttributeKHR => "HitAttributeKHR",
        StorageClass::IncomingRayPayloadKHR => "IncomingRayPayloadKHR",
        StorageClass::ShaderRecordBufferKHR => "ShaderRecordBufferKHR",
        StorageClass::PhysicalStorageBuffer => "PhysicalStorageBuffer",
        StorageClass::TaskPayloadWorkgroupEXT => "TaskPayloadWorkgroupEXT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disassembler_options() {
        let default = DisassemblerOptions::default();
        assert!(default.indent);
        assert!(default.friendly_names);

        let minimal = DisassemblerOptions::minimal();
        assert!(!minimal.indent);
        assert!(!minimal.friendly_names);

        let verbose = DisassemblerOptions::verbose();
        assert!(verbose.show_offsets);
        assert!(verbose.color);
    }

    #[test]
    fn test_capability_names() {
        assert_eq!(capability_name(Capability::Shader), "Shader");
        assert_eq!(capability_name(Capability::Float64), "Float64");
    }

    #[test]
    fn test_execution_model_names() {
        assert_eq!(execution_model_name(ExecutionModel::Vertex), "Vertex");
        assert_eq!(execution_model_name(ExecutionModel::Fragment), "Fragment");
        assert_eq!(execution_model_name(ExecutionModel::GLCompute), "GLCompute");
    }
}
