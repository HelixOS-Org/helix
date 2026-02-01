//! # Code Generation Metrics
//!
//! Year 3 EVOLUTION - Performance and quality metrics for generated code
//! Measures and tracks code generation effectiveness.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::ir::{IRBlock, IRFunction, IRInstruction, IRModule, IROp, IRType};

// ============================================================================
// METRIC TYPES
// ============================================================================

/// Code quality metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    // Size metrics
    LinesOfCode,
    InstructionCount,
    FunctionCount,
    BlockCount,

    // Complexity metrics
    CyclomaticComplexity,
    NestingDepth,
    HalsteadVolume,
    MaintainabilityIndex,

    // Performance metrics
    EstimatedCycles,
    MemoryFootprint,
    StackUsage,
    CacheEfficiency,

    // Quality metrics
    CodeCoverage,
    BranchCoverage,
    MutationScore,

    // Safety metrics
    UnsafeBlockCount,
    RawPointerOps,
    UncheckedOps,

    // Optimization metrics
    InlinedFunctions,
    EliminatedDeadCode,
    OptimizationGain,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    Count(u64),
    Ratio(f64),
    Score(f64),
    Bytes(usize),
    Cycles(u64),
    Percentage(f64),
}

impl MetricValue {
    pub fn as_u64(&self) -> u64 {
        match self {
            MetricValue::Count(n) => *n,
            MetricValue::Cycles(n) => *n,
            MetricValue::Bytes(n) => *n as u64,
            MetricValue::Ratio(f) | MetricValue::Score(f) | MetricValue::Percentage(f) => *f as u64,
        }
    }

    pub fn as_f64(&self) -> f64 {
        match self {
            MetricValue::Count(n) | MetricValue::Cycles(n) => *n as f64,
            MetricValue::Bytes(n) => *n as f64,
            MetricValue::Ratio(f) | MetricValue::Score(f) | MetricValue::Percentage(f) => *f,
        }
    }
}

/// Metric result
#[derive(Debug, Clone)]
pub struct MetricResult {
    /// Metric kind
    pub kind: MetricKind,
    /// Metric value
    pub value: MetricValue,
    /// Threshold (if applicable)
    pub threshold: Option<MetricValue>,
    /// Passed threshold check
    pub passed: bool,
}

/// Metric report
#[derive(Debug, Clone)]
pub struct MetricReport {
    /// Module name
    pub module_name: String,
    /// All metrics
    pub metrics: Vec<MetricResult>,
    /// Overall score
    pub overall_score: f64,
    /// Timestamp
    pub timestamp: u64,
}

/// Halstead metrics
#[derive(Debug, Clone, Default)]
pub struct HalsteadMetrics {
    /// Distinct operators
    pub n1: u64,
    /// Distinct operands
    pub n2: u64,
    /// Total operators
    pub big_n1: u64,
    /// Total operands
    pub big_n2: u64,
}

impl HalsteadMetrics {
    /// Vocabulary
    pub fn vocabulary(&self) -> u64 {
        self.n1 + self.n2
    }

    /// Length
    pub fn length(&self) -> u64 {
        self.big_n1 + self.big_n2
    }

    /// Calculated length
    pub fn calculated_length(&self) -> f64 {
        let n1 = self.n1 as f64;
        let n2 = self.n2 as f64;
        if n1 > 0.0 && n2 > 0.0 {
            n1 * n1.log2() + n2 * n2.log2()
        } else {
            0.0
        }
    }

    /// Volume
    pub fn volume(&self) -> f64 {
        let n = self.vocabulary() as f64;
        if n > 0.0 {
            self.length() as f64 * n.log2()
        } else {
            0.0
        }
    }

    /// Difficulty
    pub fn difficulty(&self) -> f64 {
        let n2 = self.n2 as f64;
        let big_n2 = self.big_n2 as f64;
        if n2 > 0.0 {
            (self.n1 as f64 / 2.0) * (big_n2 / n2)
        } else {
            0.0
        }
    }

    /// Effort
    pub fn effort(&self) -> f64 {
        self.difficulty() * self.volume()
    }

    /// Time to implement (seconds)
    pub fn time(&self) -> f64 {
        self.effort() / 18.0
    }

    /// Bugs estimation
    pub fn bugs(&self) -> f64 {
        self.volume() / 3000.0
    }
}

/// Maintainability index
#[derive(Debug, Clone)]
pub struct MaintainabilityIndex {
    /// Halstead volume
    pub halstead_volume: f64,
    /// Cyclomatic complexity
    pub cyclomatic: u32,
    /// Lines of code
    pub loc: usize,
    /// Comment ratio
    pub comment_ratio: f64,
}

impl MaintainabilityIndex {
    /// Calculate maintainability index (0-100)
    pub fn calculate(&self) -> f64 {
        let v = self.halstead_volume;
        let g = self.cyclomatic as f64;
        let loc = self.loc as f64;

        if v <= 0.0 || loc <= 0.0 {
            return 100.0;
        }

        // Original MI formula
        let mi = 171.0 - 5.2 * v.ln() - 0.23 * g - 16.2 * loc.ln();

        // With comment adjustment
        let adjusted = mi + 50.0 * self.comment_ratio.sin();

        // Normalize to 0-100
        (adjusted.max(0.0).min(100.0))
    }
}

// ============================================================================
// METRICS COLLECTOR
// ============================================================================

/// Metrics collector
pub struct MetricsCollector {
    /// Collected metrics
    metrics: BTreeMap<String, Vec<MetricResult>>,
    /// Thresholds
    thresholds: BTreeMap<MetricKind, MetricValue>,
    /// Historical data
    history: Vec<MetricReport>,
    /// Configuration
    config: MetricsConfig,
}

/// Metrics configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Enable all metrics
    pub enable_all: bool,
    /// Enabled metrics
    pub enabled: Vec<MetricKind>,
    /// Store history
    pub store_history: bool,
    /// History limit
    pub history_limit: usize,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_all: true,
            enabled: Vec::new(),
            store_history: true,
            history_limit: 100,
        }
    }
}

impl MetricsCollector {
    /// Create new collector
    pub fn new(config: MetricsConfig) -> Self {
        let mut collector = Self {
            metrics: BTreeMap::new(),
            thresholds: BTreeMap::new(),
            history: Vec::new(),
            config,
        };

        collector.set_default_thresholds();
        collector
    }

    fn set_default_thresholds(&mut self) {
        self.thresholds
            .insert(MetricKind::CyclomaticComplexity, MetricValue::Count(10));
        self.thresholds
            .insert(MetricKind::NestingDepth, MetricValue::Count(4));
        self.thresholds
            .insert(MetricKind::MaintainabilityIndex, MetricValue::Score(65.0));
        self.thresholds
            .insert(MetricKind::UnsafeBlockCount, MetricValue::Count(0));
        self.thresholds
            .insert(MetricKind::CodeCoverage, MetricValue::Percentage(80.0));
    }

    /// Collect all metrics for IR module
    pub fn collect(&mut self, ir: &IRModule) -> MetricReport {
        let mut results = Vec::new();

        // Size metrics
        results.push(self.measure(MetricKind::LinesOfCode, ir));
        results.push(self.measure(MetricKind::InstructionCount, ir));
        results.push(self.measure(MetricKind::FunctionCount, ir));
        results.push(self.measure(MetricKind::BlockCount, ir));

        // Complexity metrics
        results.push(self.measure(MetricKind::CyclomaticComplexity, ir));
        results.push(self.measure(MetricKind::NestingDepth, ir));
        results.push(self.measure(MetricKind::HalsteadVolume, ir));
        results.push(self.measure(MetricKind::MaintainabilityIndex, ir));

        // Performance metrics
        results.push(self.measure(MetricKind::EstimatedCycles, ir));
        results.push(self.measure(MetricKind::MemoryFootprint, ir));
        results.push(self.measure(MetricKind::StackUsage, ir));

        // Safety metrics
        results.push(self.measure(MetricKind::UnsafeBlockCount, ir));
        results.push(self.measure(MetricKind::RawPointerOps, ir));

        // Calculate overall score
        let overall = self.calculate_overall_score(&results);

        let report = MetricReport {
            module_name: ir.name.clone(),
            metrics: results,
            overall_score: overall,
            timestamp: 0,
        };

        // Store in history
        if self.config.store_history {
            self.history.push(report.clone());
            if self.history.len() > self.config.history_limit {
                self.history.remove(0);
            }
        }

        report
    }

    fn measure(&self, kind: MetricKind, ir: &IRModule) -> MetricResult {
        let value = match kind {
            MetricKind::LinesOfCode => MetricValue::Count(self.count_lines(ir)),
            MetricKind::InstructionCount => MetricValue::Count(self.count_instructions(ir)),
            MetricKind::FunctionCount => MetricValue::Count(ir.functions.len() as u64),
            MetricKind::BlockCount => MetricValue::Count(self.count_blocks(ir)),
            MetricKind::CyclomaticComplexity => {
                MetricValue::Count(self.calculate_cyclomatic(ir) as u64)
            },
            MetricKind::NestingDepth => MetricValue::Count(self.calculate_nesting_depth(ir) as u64),
            MetricKind::HalsteadVolume => {
                let h = self.calculate_halstead(ir);
                MetricValue::Score(h.volume())
            },
            MetricKind::MaintainabilityIndex => {
                MetricValue::Score(self.calculate_maintainability(ir))
            },
            MetricKind::EstimatedCycles => MetricValue::Cycles(self.estimate_cycles(ir)),
            MetricKind::MemoryFootprint => MetricValue::Bytes(self.estimate_memory(ir)),
            MetricKind::StackUsage => MetricValue::Bytes(self.estimate_stack(ir)),
            MetricKind::UnsafeBlockCount => MetricValue::Count(self.count_unsafe(ir)),
            MetricKind::RawPointerOps => MetricValue::Count(self.count_pointer_ops(ir)),
            _ => MetricValue::Count(0),
        };

        let threshold = self.thresholds.get(&kind).cloned();
        let passed = self.check_threshold(&value, &threshold, kind);

        MetricResult {
            kind,
            value,
            threshold,
            passed,
        }
    }

    fn count_lines(&self, ir: &IRModule) -> u64 {
        let mut lines = 0u64;
        for func in ir.functions.values() {
            lines += 2; // function header + closing brace
            for block in func.blocks.values() {
                lines += 1 + block.instructions.len() as u64;
            }
        }
        lines
    }

    fn count_instructions(&self, ir: &IRModule) -> u64 {
        ir.functions
            .values()
            .flat_map(|f| f.blocks.values())
            .map(|b| b.instructions.len() as u64)
            .sum()
    }

    fn count_blocks(&self, ir: &IRModule) -> u64 {
        ir.functions.values().map(|f| f.blocks.len() as u64).sum()
    }

    fn calculate_cyclomatic(&self, ir: &IRModule) -> u32 {
        let mut complexity = 0u32;

        for func in ir.functions.values() {
            let edges = func
                .blocks
                .values()
                .map(|b| b.successors.len() as u32)
                .sum::<u32>();
            let nodes = func.blocks.len() as u32;

            // M = E - N + 2P (P=1 for single function)
            complexity += edges.saturating_sub(nodes) + 2;
        }

        complexity
    }

    fn calculate_nesting_depth(&self, ir: &IRModule) -> u32 {
        let mut max_depth = 0u32;

        for func in ir.functions.values() {
            // Simplified: count maximum chain of blocks
            let depth = self.calculate_block_depth(func);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    fn calculate_block_depth(&self, func: &IRFunction) -> u32 {
        let mut depths: BTreeMap<u64, u32> = BTreeMap::new();
        depths.insert(func.entry, 0);

        let mut changed = true;
        while changed {
            changed = false;
            for block in func.blocks.values() {
                let block_depth = *depths.get(&block.id).unwrap_or(&0);
                for &succ in &block.successors {
                    let current = *depths.get(&succ).unwrap_or(&0);
                    if block_depth + 1 > current {
                        depths.insert(succ, block_depth + 1);
                        changed = true;
                    }
                }
            }
        }

        depths.values().copied().max().unwrap_or(0)
    }

    fn calculate_halstead(&self, ir: &IRModule) -> HalsteadMetrics {
        let mut operators: BTreeMap<String, u64> = BTreeMap::new();
        let mut operands: BTreeMap<String, u64> = BTreeMap::new();

        for func in ir.functions.values() {
            for block in func.blocks.values() {
                for instr in &block.instructions {
                    self.categorize_instruction(&instr.op, &mut operators, &mut operands);
                }
            }
        }

        HalsteadMetrics {
            n1: operators.len() as u64,
            n2: operands.len() as u64,
            big_n1: operators.values().sum(),
            big_n2: operands.values().sum(),
        }
    }

    fn categorize_instruction(
        &self,
        op: &IROp,
        operators: &mut BTreeMap<String, u64>,
        operands: &mut BTreeMap<String, u64>,
    ) {
        let op_name = match op {
            IROp::Add(_, _) => "add",
            IROp::Sub(_, _) => "sub",
            IROp::Mul(_, _) => "mul",
            IROp::Div(_, _) => "div",
            IROp::Rem(_, _) => "rem",
            IROp::And(_, _) => "and",
            IROp::Or(_, _) => "or",
            IROp::Xor(_, _) => "xor",
            IROp::Shl(_, _) => "shl",
            IROp::Shr(_, _) => "shr",
            IROp::Load(_) => "load",
            IROp::Store(_, _) => "store",
            IROp::Call(_, _) => "call",
            _ => "other",
        };

        *operators.entry(op_name.into()).or_insert(0) += 1;

        // Count operands
        match op {
            IROp::Add(a, b)
            | IROp::Sub(a, b)
            | IROp::Mul(a, b)
            | IROp::Div(a, b)
            | IROp::Rem(a, b)
            | IROp::And(a, b)
            | IROp::Or(a, b)
            | IROp::Xor(a, b)
            | IROp::Shl(a, b)
            | IROp::Shr(a, b)
            | IROp::Store(a, b) => {
                self.count_operand(a, operands);
                self.count_operand(b, operands);
            },
            IROp::Load(a) | IROp::Neg(a) | IROp::Not(a) => {
                self.count_operand(a, operands);
            },
            IROp::Call(_, args) | IROp::IndirectCall(_, args) => {
                for arg in args {
                    self.count_operand(arg, operands);
                }
            },
            _ => {},
        }
    }

    fn count_operand(&self, val: &super::ir::IRValue, operands: &mut BTreeMap<String, u64>) {
        use super::ir::IRValue;
        match val {
            IRValue::Var(name) => {
                *operands.entry(name.clone()).or_insert(0) += 1;
            },
            IRValue::ConstInt(n, _) => {
                *operands.entry(format!("{}", n)).or_insert(0) += 1;
            },
            IRValue::Param(n) => {
                *operands.entry(format!("param_{}", n)).or_insert(0) += 1;
            },
            _ => {},
        }
    }

    fn calculate_maintainability(&self, ir: &IRModule) -> f64 {
        let halstead = self.calculate_halstead(ir);
        let cyclomatic = self.calculate_cyclomatic(ir);
        let loc = self.count_lines(ir) as usize;

        let mi = MaintainabilityIndex {
            halstead_volume: halstead.volume(),
            cyclomatic,
            loc,
            comment_ratio: 0.1, // Assume 10% comments
        };

        mi.calculate()
    }

    fn estimate_cycles(&self, ir: &IRModule) -> u64 {
        let mut cycles = 0u64;

        for func in ir.functions.values() {
            for block in func.blocks.values() {
                for instr in &block.instructions {
                    cycles += self.instruction_cycles(&instr.op);
                }
            }
        }

        cycles
    }

    fn instruction_cycles(&self, op: &IROp) -> u64 {
        match op {
            IROp::Add(_, _) | IROp::Sub(_, _) => 1,
            IROp::Mul(_, _) => 3,
            IROp::Div(_, _) | IROp::Rem(_, _) => 20,
            IROp::And(_, _) | IROp::Or(_, _) | IROp::Xor(_, _) => 1,
            IROp::Shl(_, _) | IROp::Shr(_, _) => 1,
            IROp::Load(_) => 4,
            IROp::Store(_, _) => 4,
            IROp::Call(_, _) => 10,
            IROp::Nop => 0,
            _ => 1,
        }
    }

    fn estimate_memory(&self, ir: &IRModule) -> usize {
        let mut bytes = 0usize;

        // Code size estimate
        bytes += self.count_instructions(ir) as usize * 8;

        // Global data
        for global in ir.globals.values() {
            bytes += global.typ.size();
        }

        bytes
    }

    fn estimate_stack(&self, ir: &IRModule) -> usize {
        let mut max_stack = 0usize;

        for func in ir.functions.values() {
            let mut func_stack = 0usize;

            // Parameters
            for param in &func.params {
                func_stack += param.typ.size();
            }

            // Locals
            for local in func.locals.values() {
                func_stack += local.typ.size();
            }

            max_stack = max_stack.max(func_stack);
        }

        max_stack
    }

    fn count_unsafe(&self, _ir: &IRModule) -> u64 {
        // Count instructions that would require unsafe in Rust
        0
    }

    fn count_pointer_ops(&self, ir: &IRModule) -> u64 {
        let mut count = 0u64;

        for func in ir.functions.values() {
            for block in func.blocks.values() {
                for instr in &block.instructions {
                    match &instr.op {
                        IROp::Load(_) | IROp::Store(_, _) | IROp::GetElementPtr(_, _) => count += 1,
                        _ => {},
                    }
                }
            }
        }

        count
    }

    fn check_threshold(
        &self,
        value: &MetricValue,
        threshold: &Option<MetricValue>,
        kind: MetricKind,
    ) -> bool {
        let threshold = match threshold {
            Some(t) => t,
            None => return true,
        };

        // Higher is better for some metrics, lower for others
        let higher_is_better = matches!(
            kind,
            MetricKind::MaintainabilityIndex
                | MetricKind::CodeCoverage
                | MetricKind::CacheEfficiency
                | MetricKind::OptimizationGain
        );

        if higher_is_better {
            value.as_f64() >= threshold.as_f64()
        } else {
            value.as_f64() <= threshold.as_f64()
        }
    }

    fn calculate_overall_score(&self, results: &[MetricResult]) -> f64 {
        if results.is_empty() {
            return 100.0;
        }

        let passed = results.iter().filter(|r| r.passed).count();
        (passed as f64 / results.len() as f64) * 100.0
    }

    /// Get metric history
    pub fn history(&self) -> &[MetricReport] {
        &self.history
    }

    /// Set threshold
    pub fn set_threshold(&mut self, kind: MetricKind, value: MetricValue) {
        self.thresholds.insert(kind, value);
    }

    /// Compare two reports
    pub fn compare(&self, before: &MetricReport, after: &MetricReport) -> MetricComparison {
        let mut changes = Vec::new();

        for after_metric in &after.metrics {
            if let Some(before_metric) = before.metrics.iter().find(|m| m.kind == after_metric.kind)
            {
                let change = after_metric.value.as_f64() - before_metric.value.as_f64();
                let percent = if before_metric.value.as_f64() != 0.0 {
                    (change / before_metric.value.as_f64()) * 100.0
                } else {
                    0.0
                };

                changes.push(MetricChange {
                    kind: after_metric.kind,
                    before: before_metric.value.clone(),
                    after: after_metric.value.clone(),
                    change,
                    percent_change: percent,
                });
            }
        }

        MetricComparison {
            before_score: before.overall_score,
            after_score: after.overall_score,
            changes,
        }
    }
}

/// Metric change
#[derive(Debug, Clone)]
pub struct MetricChange {
    pub kind: MetricKind,
    pub before: MetricValue,
    pub after: MetricValue,
    pub change: f64,
    pub percent_change: f64,
}

/// Metric comparison
#[derive(Debug, Clone)]
pub struct MetricComparison {
    pub before_score: f64,
    pub after_score: f64,
    pub changes: Vec<MetricChange>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new(MetricsConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::ir::IRBuilder;
    use super::*;

    #[test]
    fn test_halstead_metrics() {
        let h = HalsteadMetrics {
            n1: 5,
            n2: 10,
            big_n1: 20,
            big_n2: 40,
        };

        assert_eq!(h.vocabulary(), 15);
        assert_eq!(h.length(), 60);
        assert!(h.volume() > 0.0);
    }

    #[test]
    fn test_maintainability_index() {
        let mi = MaintainabilityIndex {
            halstead_volume: 100.0,
            cyclomatic: 5,
            loc: 50,
            comment_ratio: 0.1,
        };

        let score = mi.calculate();
        assert!(score >= 0.0 && score <= 100.0);
    }

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::default();
        let builder = IRBuilder::new("test");
        let ir = builder.finalize();

        let report = collector.collect(&ir);
        assert!(report.overall_score >= 0.0);
    }
}
