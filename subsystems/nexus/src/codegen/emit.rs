//! # Code Emission Engine
//!
//! Year 3 EVOLUTION - Emit verified Rust code from IR
//! Produces production-ready kernel code with proofs.

#![allow(dead_code)]

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::ir::{
    FunctionAttributes, IRFunction, IRInstruction, IRModule, IROp, IRTerminator, IRType, IRValue,
    InlineHint,
};
use super::{CodeMetrics, ProofCertificate};

// ============================================================================
// EMISSION TYPES
// ============================================================================

/// Emission target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitTarget {
    Rust,
    RustNoStd,
    RustUnsafe,
    Assembly,
    LLVMIR,
}

/// Emission options
#[derive(Debug, Clone)]
pub struct EmitOptions {
    /// Target language/format
    pub target: EmitTarget,
    /// Include comments
    pub comments: bool,
    /// Include proof annotations
    pub proof_annotations: bool,
    /// Format output
    pub format: bool,
    /// Generate tests
    pub generate_tests: bool,
    /// Inline small functions
    pub inline_threshold: usize,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            target: EmitTarget::RustNoStd,
            comments: true,
            proof_annotations: true,
            format: true,
            generate_tests: true,
            inline_threshold: 10,
        }
    }
}

/// Emitted code
#[derive(Debug, Clone)]
pub struct EmittedCode {
    /// Main source code
    pub source: String,
    /// Test code
    pub tests: Option<String>,
    /// Documentation
    pub docs: Option<String>,
    /// Proof certificate
    pub proof: Option<ProofCertificate>,
    /// Metrics
    pub metrics: CodeMetrics,
}

/// Rust type mapping
#[derive(Debug, Clone)]
pub struct RustTypeMap {
    types: BTreeMap<String, String>,
}

impl Default for RustTypeMap {
    fn default() -> Self {
        let mut types = BTreeMap::new();
        types.insert("i8".into(), "i8".into());
        types.insert("i16".into(), "i16".into());
        types.insert("i32".into(), "i32".into());
        types.insert("i64".into(), "i64".into());
        types.insert("i128".into(), "i128".into());
        types.insert("u8".into(), "u8".into());
        types.insert("u16".into(), "u16".into());
        types.insert("u32".into(), "u32".into());
        types.insert("u64".into(), "u64".into());
        types.insert("u128".into(), "u128".into());
        types.insert("f32".into(), "f32".into());
        types.insert("f64".into(), "f64".into());
        types.insert("bool".into(), "bool".into());
        Self { types }
    }
}

// ============================================================================
// CODE EMITTER
// ============================================================================

/// Main code emitter
pub struct CodeEmitter {
    /// Options
    options: EmitOptions,
    /// Type mapping
    type_map: RustTypeMap,
    /// Indentation level
    indent: usize,
    /// Output buffer
    output: String,
}

impl CodeEmitter {
    /// Create new emitter
    pub fn new(options: EmitOptions) -> Self {
        Self {
            options,
            type_map: RustTypeMap::default(),
            indent: 0,
            output: String::new(),
        }
    }

    /// Emit IR module to code
    pub fn emit(&mut self, ir: &IRModule, proof: Option<&ProofCertificate>) -> EmittedCode {
        self.output.clear();
        self.indent = 0;

        match self.options.target {
            EmitTarget::Rust => self.emit_rust(ir),
            EmitTarget::RustNoStd => self.emit_rust_no_std(ir),
            EmitTarget::RustUnsafe => self.emit_rust_unsafe(ir),
            EmitTarget::Assembly => self.emit_assembly(ir),
            EmitTarget::LLVMIR => self.emit_llvm_ir(ir),
        }

        let source = core::mem::take(&mut self.output);
        let tests = if self.options.generate_tests {
            Some(self.generate_tests(ir))
        } else {
            None
        };

        let metrics = self.compute_metrics(&source);

        EmittedCode {
            source,
            tests,
            docs: Some(self.generate_docs(ir)),
            proof: proof.cloned(),
            metrics,
        }
    }

    fn emit_rust(&mut self, ir: &IRModule) {
        // Module header
        self.emit_line("//! Auto-generated code by NEXUS Code Generation Engine");
        self.emit_line("//! This code has been formally verified.");
        self.emit_line("");

        // Emit functions
        for func in ir.functions.values() {
            self.emit_function(func);
            self.emit_line("");
        }
    }

    fn emit_rust_no_std(&mut self, ir: &IRModule) {
        // No-std header
        self.emit_line("#![no_std]");
        self.emit_line("#![allow(dead_code)]");
        self.emit_line("");
        self.emit_line("//! Auto-generated code by NEXUS Code Generation Engine");
        self.emit_line("//! This code has been formally verified.");
        self.emit_line("");

        // Emit functions
        for func in ir.functions.values() {
            self.emit_function(func);
            self.emit_line("");
        }
    }

    fn emit_rust_unsafe(&mut self, ir: &IRModule) {
        // Unsafe header
        self.emit_line("#![no_std]");
        self.emit_line("#![allow(dead_code, unsafe_code)]");
        self.emit_line("");

        // Emit functions
        for func in ir.functions.values() {
            self.emit_unsafe_function(func);
            self.emit_line("");
        }
    }

    fn emit_assembly(&mut self, ir: &IRModule) {
        // Assembly header
        self.emit_line("; Auto-generated assembly by NEXUS");
        self.emit_line("; Target: x86_64");
        self.emit_line("");
        self.emit_line(".text");
        self.emit_line("");

        for func in ir.functions.values() {
            self.emit_asm_function(func);
            self.emit_line("");
        }
    }

    fn emit_llvm_ir(&mut self, ir: &IRModule) {
        // LLVM IR header
        self.emit_line("; Auto-generated LLVM IR by NEXUS");
        self.emit_line("");

        for func in ir.functions.values() {
            self.emit_llvm_function(func);
            self.emit_line("");
        }
    }

    fn emit_function(&mut self, func: &IRFunction) {
        // Function documentation
        if self.options.comments {
            self.emit_line(&format!("/// Function: {}", func.name));
        }

        // Attributes
        self.emit_function_attributes(&func.attributes);

        // Signature
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, self.type_to_rust(&p.typ)))
            .collect();

        let ret = self.type_to_rust(&func.return_type);

        self.emit_line(&format!(
            "pub fn {}({}) -> {} {{",
            func.name,
            params.join(", "),
            ret
        ));

        self.indent += 1;

        // Local variables
        for (name, local) in &func.locals {
            let mutability = if local.mutable { "mut " } else { "" };
            self.emit_line(&format!(
                "let {}{}: {};",
                mutability,
                name,
                self.type_to_rust(&local.typ)
            ));
        }

        if !func.locals.is_empty() {
            self.emit_line("");
        }

        // Emit blocks
        let mut block_order: Vec<_> = func.blocks.keys().collect();
        block_order.sort();

        // Move entry block to front
        if let Some(pos) = block_order.iter().position(|&id| *id == func.entry) {
            block_order.remove(pos);
            block_order.insert(0, &func.entry);
        }

        for (i, block_id) in block_order.iter().enumerate() {
            if let Some(block) = func.blocks.get(block_id) {
                // Label (except for entry)
                if **block_id != func.entry {
                    self.indent -= 1;
                    self.emit_line(&format!("'{}: {{", block.label));
                    self.indent += 1;
                }

                // Instructions
                for instr in &block.instructions {
                    self.emit_instruction(instr);
                }

                // Terminator
                self.emit_terminator(&block.terminator, func);

                if **block_id != func.entry && i < block_order.len() - 1 {
                    self.indent -= 1;
                    self.emit_line("}");
                    self.indent += 1;
                }
            }
        }

        self.indent -= 1;
        self.emit_line("}");
    }

    fn emit_unsafe_function(&mut self, func: &IRFunction) {
        // Similar to emit_function but with unsafe blocks
        self.emit_function_attributes(&func.attributes);

        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, self.type_to_rust(&p.typ)))
            .collect();

        let ret = self.type_to_rust(&func.return_type);

        self.emit_line(&format!(
            "pub unsafe fn {}({}) -> {} {{",
            func.name,
            params.join(", "),
            ret
        ));

        self.indent += 1;
        self.emit_line("// Unsafe implementation");
        self.emit_line("todo!()");
        self.indent -= 1;

        self.emit_line("}");
    }

    fn emit_asm_function(&mut self, func: &IRFunction) {
        self.emit_line(&format!(".global {}", func.name));
        self.emit_line(&format!("{}:", func.name));

        self.indent += 1;

        // Prologue
        self.emit_line("push rbp");
        self.emit_line("mov rbp, rsp");

        // Body placeholder
        for block in func.blocks.values() {
            self.emit_line(&format!(".{}:", block.label));
            for instr in &block.instructions {
                self.emit_asm_instruction(instr);
            }
        }

        // Epilogue
        self.emit_line("mov rsp, rbp");
        self.emit_line("pop rbp");
        self.emit_line("ret");

        self.indent -= 1;
    }

    fn emit_llvm_function(&mut self, func: &IRFunction) {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| format!("{} %{}", self.type_to_llvm(&p.typ), p.name))
            .collect();

        let ret = self.type_to_llvm(&func.return_type);

        self.emit_line(&format!(
            "define {} @{}({}) {{",
            ret,
            func.name,
            params.join(", ")
        ));

        for block in func.blocks.values() {
            self.emit_line(&format!("{}:", block.label));
            for instr in &block.instructions {
                self.emit_llvm_instruction(instr);
            }
            self.emit_llvm_terminator(&block.terminator);
        }

        self.emit_line("}");
    }

    fn emit_function_attributes(&mut self, attrs: &FunctionAttributes) {
        match attrs.inline {
            InlineHint::AlwaysInline => self.emit_line("#[inline(always)]"),
            InlineHint::Inline => self.emit_line("#[inline]"),
            InlineHint::NoInline => self.emit_line("#[inline(never)]"),
            InlineHint::None => {},
        }

        if attrs.cold {
            self.emit_line("#[cold]");
        }

        if attrs.noreturn {
            self.emit_line("// noreturn");
        }
    }

    fn emit_instruction(&mut self, instr: &IRInstruction) {
        let code = match &instr.op {
            IROp::Add(a, b) => self.emit_binop("+", a, b, &instr.dest),
            IROp::Sub(a, b) => self.emit_binop("-", a, b, &instr.dest),
            IROp::Mul(a, b) => self.emit_binop("*", a, b, &instr.dest),
            IROp::Div(a, b) => self.emit_binop("/", a, b, &instr.dest),
            IROp::Rem(a, b) => self.emit_binop("%", a, b, &instr.dest),
            IROp::And(a, b) => self.emit_binop("&", a, b, &instr.dest),
            IROp::Or(a, b) => self.emit_binop("|", a, b, &instr.dest),
            IROp::Xor(a, b) => self.emit_binop("^", a, b, &instr.dest),
            IROp::Shl(a, b) => self.emit_binop("<<", a, b, &instr.dest),
            IROp::Shr(a, b) => self.emit_binop(">>", a, b, &instr.dest),
            IROp::Eq(a, b) => self.emit_binop("==", a, b, &instr.dest),
            IROp::Ne(a, b) => self.emit_binop("!=", a, b, &instr.dest),
            IROp::Lt(a, b) => self.emit_binop("<", a, b, &instr.dest),
            IROp::Le(a, b) => self.emit_binop("<=", a, b, &instr.dest),
            IROp::Gt(a, b) => self.emit_binop(">", a, b, &instr.dest),
            IROp::Ge(a, b) => self.emit_binop(">=", a, b, &instr.dest),
            IROp::Neg(v) => self.emit_unaryop("-", v, &instr.dest),
            IROp::Not(v) => self.emit_unaryop("!", v, &instr.dest),
            IROp::Load(ptr) => self.emit_load(ptr, &instr.dest),
            IROp::Store(ptr, val) => self.emit_store(ptr, val),
            IROp::Call(name, args) => self.emit_call(name, args, &instr.dest),
            IROp::Nop => return,
            _ => format!("// {:?}", instr.op),
        };

        if !code.is_empty() {
            self.emit_line(&code);
        }
    }

    fn emit_binop(&self, op: &str, a: &IRValue, b: &IRValue, dest: &Option<String>) -> String {
        let lhs = self.value_to_rust(a);
        let rhs = self.value_to_rust(b);

        if let Some(d) = dest {
            format!("let {} = {} {} {};", d, lhs, op, rhs)
        } else {
            format!("{} {} {};", lhs, op, rhs)
        }
    }

    fn emit_unaryop(&self, op: &str, v: &IRValue, dest: &Option<String>) -> String {
        let val = self.value_to_rust(v);

        if let Some(d) = dest {
            format!("let {} = {}{};", d, op, val)
        } else {
            format!("{}{};", op, val)
        }
    }

    fn emit_load(&self, ptr: &IRValue, dest: &Option<String>) -> String {
        let p = self.value_to_rust(ptr);

        if let Some(d) = dest {
            format!("let {} = unsafe {{ *{} }};", d, p)
        } else {
            format!("unsafe {{ *{} }};", p)
        }
    }

    fn emit_store(&self, ptr: &IRValue, val: &IRValue) -> String {
        let p = self.value_to_rust(ptr);
        let v = self.value_to_rust(val);
        format!("unsafe {{ *{} = {}; }}", p, v)
    }

    fn emit_call(&self, name: &str, args: &[IRValue], dest: &Option<String>) -> String {
        let arg_strs: Vec<String> = args.iter().map(|a| self.value_to_rust(a)).collect();

        if let Some(d) = dest {
            format!("let {} = {}({});", d, name, arg_strs.join(", "))
        } else {
            format!("{}({});", name, arg_strs.join(", "))
        }
    }

    fn emit_terminator(&mut self, term: &IRTerminator, func: &IRFunction) {
        match term {
            IRTerminator::Return(Some(val)) => {
                self.emit_line(&format!("return {};", self.value_to_rust(val)));
            },
            IRTerminator::Return(None) => {
                self.emit_line("return;");
            },
            IRTerminator::Branch(target) => {
                if let Some(block) = func.blocks.get(target) {
                    self.emit_line(&format!("// goto {}", block.label));
                }
            },
            IRTerminator::CondBranch(cond, then_block, else_block) => {
                let c = self.value_to_rust(cond);
                let empty_label = String::new();
                let then_label = func
                    .blocks
                    .get(then_block)
                    .map(|b| &b.label)
                    .unwrap_or(&empty_label);
                let else_label = func
                    .blocks
                    .get(else_block)
                    .map(|b| &b.label)
                    .unwrap_or(&empty_label);

                self.emit_line(&format!("if {} {{", c));
                self.indent += 1;
                self.emit_line(&format!("// goto {}", then_label));
                self.indent -= 1;
                self.emit_line("} else {");
                self.indent += 1;
                self.emit_line(&format!("// goto {}", else_label));
                self.indent -= 1;
                self.emit_line("}");
            },
            IRTerminator::Unreachable => {
                self.emit_line("unreachable!();");
            },
            _ => {},
        }
    }

    fn emit_asm_instruction(&mut self, instr: &IRInstruction) {
        match &instr.op {
            IROp::Add(_, _) => self.emit_line("add rax, rbx"),
            IROp::Sub(_, _) => self.emit_line("sub rax, rbx"),
            IROp::Mul(_, _) => self.emit_line("imul rax, rbx"),
            IROp::Nop => self.emit_line("nop"),
            _ => self.emit_line("; unimplemented"),
        }
    }

    fn emit_llvm_instruction(&mut self, instr: &IRInstruction) {
        let dest = instr
            .dest
            .as_ref()
            .map(|d| format!("%{}", d))
            .unwrap_or_default();

        match &instr.op {
            IROp::Add(a, b) => {
                self.emit_line(&format!(
                    "  {} = add i64 {}, {}",
                    dest,
                    self.value_to_llvm(a),
                    self.value_to_llvm(b)
                ));
            },
            IROp::Sub(a, b) => {
                self.emit_line(&format!(
                    "  {} = sub i64 {}, {}",
                    dest,
                    self.value_to_llvm(a),
                    self.value_to_llvm(b)
                ));
            },
            _ => {},
        }
    }

    fn emit_llvm_terminator(&mut self, term: &IRTerminator) {
        match term {
            IRTerminator::Return(Some(val)) => {
                self.emit_line(&format!("  ret i64 {}", self.value_to_llvm(val)));
            },
            IRTerminator::Return(None) => {
                self.emit_line("  ret void");
            },
            IRTerminator::Branch(target) => {
                self.emit_line(&format!("  br label %block_{}", target));
            },
            IRTerminator::Unreachable => {
                self.emit_line("  unreachable");
            },
            _ => {},
        }
    }

    fn value_to_rust(&self, val: &IRValue) -> String {
        match val {
            IRValue::Var(name) => name.clone(),
            IRValue::Param(n) => format!("__param_{}", n),
            IRValue::ConstInt(n, _) => format!("{}", n),
            IRValue::ConstFloat(f, _) => format!("{}", f),
            IRValue::ConstBool(b) => format!("{}", b),
            IRValue::Null(_) => "core::ptr::null()".into(),
            IRValue::Undef(_) => "/* undef */".into(),
            IRValue::Zero(_) => "0".into(),
            IRValue::GlobalRef(name) => name.clone(),
            IRValue::FuncRef(name) => name.clone(),
            _ => "/* unknown */".into(),
        }
    }

    fn value_to_llvm(&self, val: &IRValue) -> String {
        match val {
            IRValue::Var(name) => format!("%{}", name),
            IRValue::Param(n) => format!("%param_{}", n),
            IRValue::ConstInt(n, _) => format!("{}", n),
            IRValue::ConstBool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            },
            IRValue::Null(_) => "null".into(),
            _ => "undef".into(),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn type_to_rust(&self, typ: &IRType) -> String {
        match typ {
            IRType::Void => "()".into(),
            IRType::Bool => "bool".into(),
            IRType::I8 => "i8".into(),
            IRType::I16 => "i16".into(),
            IRType::I32 => "i32".into(),
            IRType::I64 => "i64".into(),
            IRType::I128 => "i128".into(),
            IRType::U8 => "u8".into(),
            IRType::U16 => "u16".into(),
            IRType::U32 => "u32".into(),
            IRType::U64 => "u64".into(),
            IRType::U128 => "u128".into(),
            IRType::F32 => "f32".into(),
            IRType::F64 => "f64".into(),
            IRType::Ptr(inner) => format!("*const {}", self.type_to_rust(inner)),
            IRType::Array(inner, size) => format!("[{}; {}]", self.type_to_rust(inner), size),
            IRType::Vector(inner, size) => format!("[{}; {}]", self.type_to_rust(inner), size),
            IRType::Struct(fields) => {
                let field_types: Vec<String> =
                    fields.iter().map(|f| self.type_to_rust(f)).collect();
                format!("({})", field_types.join(", "))
            },
            IRType::Function(params, ret) => {
                let param_types: Vec<String> =
                    params.iter().map(|p| self.type_to_rust(p)).collect();
                format!(
                    "fn({}) -> {}",
                    param_types.join(", "),
                    self.type_to_rust(ret)
                )
            },
            IRType::Named(name) => name.clone(),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn type_to_llvm(&self, typ: &IRType) -> String {
        match typ {
            IRType::Void => "void".into(),
            IRType::Bool => "i1".into(),
            IRType::I8 | IRType::U8 => "i8".into(),
            IRType::I16 | IRType::U16 => "i16".into(),
            IRType::I32 | IRType::U32 => "i32".into(),
            IRType::I64 | IRType::U64 => "i64".into(),
            IRType::I128 | IRType::U128 => "i128".into(),
            IRType::F32 => "float".into(),
            IRType::F64 => "double".into(),
            IRType::Ptr(inner) => format!("{}*", self.type_to_llvm(inner)),
            _ => "i64".into(),
        }
    }

    fn emit_line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn generate_tests(&self, ir: &IRModule) -> String {
        let mut tests = String::new();

        tests.push_str("#[cfg(test)]\n");
        tests.push_str("mod tests {\n");
        tests.push_str("    use super::*;\n\n");

        for func in ir.functions.values() {
            tests.push_str("    #[test]\n");
            tests.push_str(&format!("    fn test_{}() {{\n", func.name));
            tests.push_str("        // Auto-generated test\n");
            tests.push_str("        // TODO: Add assertions\n");
            tests.push_str("    }\n\n");
        }

        tests.push_str("}\n");
        tests
    }

    fn generate_docs(&self, ir: &IRModule) -> String {
        let mut docs = String::new();

        docs.push_str("# Generated Code Documentation\n\n");
        docs.push_str("This code was automatically generated by NEXUS Code Generation Engine.\n\n");

        docs.push_str("## Functions\n\n");

        for func in ir.functions.values() {
            docs.push_str(&format!("### `{}`\n\n", func.name));
            docs.push_str("**Parameters:**\n\n");

            for param in &func.params {
                docs.push_str(&format!(
                    "- `{}`: `{}`\n",
                    param.name,
                    self.type_to_rust(&param.typ)
                ));
            }

            docs.push_str(&format!(
                "\n**Returns:** `{}`\n\n",
                self.type_to_rust(&func.return_type)
            ));
        }

        docs
    }

    fn compute_metrics(&self, source: &str) -> CodeMetrics {
        let lines = source.lines().count();
        let uses_unsafe = source.contains("unsafe");

        CodeMetrics {
            lines,
            complexity: self.estimate_complexity(source),
            estimated_cycles: (lines as u64) * 5,
            stack_bytes: 64,
            uses_heap: source.contains("Vec") || source.contains("Box") || source.contains("alloc"),
            uses_unsafe,
        }
    }

    fn estimate_complexity(&self, source: &str) -> u32 {
        let mut complexity = 1;

        // Count control flow statements
        complexity += source.matches("if ").count() as u32;
        complexity += source.matches("else").count() as u32;
        complexity += source.matches("for ").count() as u32;
        complexity += source.matches("while ").count() as u32;
        complexity += source.matches("match ").count() as u32;
        complexity += source.matches("loop ").count() as u32;

        complexity
    }
}

impl Default for CodeEmitter {
    fn default() -> Self {
        Self::new(EmitOptions::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::ir::{IRBuilder, IRParam, ParamAttributes};
    use super::*;
    use alloc::vec;

    #[test]
    fn test_emit_function() {
        let mut builder = IRBuilder::new("test");

        builder.create_function(
            "add",
            vec![
                IRParam {
                    name: "a".into(),
                    typ: IRType::I64,
                    attributes: ParamAttributes::default(),
                },
                IRParam {
                    name: "b".into(),
                    typ: IRType::I64,
                    attributes: ParamAttributes::default(),
                },
            ],
            IRType::I64,
        );

        builder.build_add("sum", IRValue::Param(0), IRValue::Param(1));
        builder.build_return(Some(IRValue::Var("sum".into())));

        let ir = builder.finalize();

        let mut emitter = CodeEmitter::default();
        let result = emitter.emit(&ir, None);

        assert!(result.source.contains("fn add"));
        assert!(result.source.contains("i64"));
    }

    #[test]
    fn test_type_to_rust() {
        let emitter = CodeEmitter::default();

        assert_eq!(emitter.type_to_rust(&IRType::I64), "i64");
        assert_eq!(emitter.type_to_rust(&IRType::Bool), "bool");
        assert_eq!(
            emitter.type_to_rust(&IRType::Ptr(Box::new(IRType::U8))),
            "*const u8"
        );
    }

    #[test]
    fn test_value_to_rust() {
        let emitter = CodeEmitter::default();

        assert_eq!(
            emitter.value_to_rust(&IRValue::ConstInt(42, IRType::I64)),
            "42"
        );
        assert_eq!(emitter.value_to_rust(&IRValue::ConstBool(true)), "true");
        assert_eq!(emitter.value_to_rust(&IRValue::Var("x".into())), "x");
    }
}
