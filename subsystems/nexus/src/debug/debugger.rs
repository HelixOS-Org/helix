//! Main debugger implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::context::{DebugContext, StackFrame};
use super::diagnosis::{Diagnosis, Fix, FixType};
use super::pattern::{BugCategory, BugPattern, BugSeverity};

// ============================================================================
// DEBUGGER STATS
// ============================================================================

/// Debugger statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DebuggerStats {
    /// Number of patterns
    pub pattern_count: usize,
    /// Total diagnoses
    pub total_diagnoses: u64,
    /// Successful diagnoses
    pub successful_diagnoses: u64,
    /// Success rate
    pub success_rate: f64,
}

// ============================================================================
// DEBUGGER
// ============================================================================

/// The AI-powered debugger
pub struct Debugger {
    /// Known bug patterns
    patterns: Vec<BugPattern>,
    /// Diagnosis history
    history: VecDeque<Diagnosis>,
    /// Maximum history
    max_history: usize,
    /// Total diagnoses
    total_diagnoses: AtomicU64,
    /// Successful diagnoses
    successful_diagnoses: AtomicU64,
}

impl Debugger {
    /// Create a new debugger
    pub fn new() -> Self {
        let mut debugger = Self {
            patterns: Vec::new(),
            history: VecDeque::new(),
            max_history: 1000,
            total_diagnoses: AtomicU64::new(0),
            successful_diagnoses: AtomicU64::new(0),
        };

        // Load default patterns
        debugger.load_default_patterns();

        debugger
    }

    /// Load default bug patterns
    fn load_default_patterns(&mut self) {
        // Memory patterns
        self.add_pattern(
            BugPattern::new("Null Pointer Dereference", BugCategory::Memory)
                .with_description("Attempted to dereference a null pointer")
                .with_severity(BugSeverity::Critical)
                .with_symptom("null pointer")
                .with_symptom("page fault at 0x0")
                .with_fix("Check for null before dereferencing")
                .with_fix("Add null guard to function entry"),
        );

        self.add_pattern(
            BugPattern::new("Use After Free", BugCategory::Memory)
                .with_description("Memory was accessed after being freed")
                .with_severity(BugSeverity::Critical)
                .with_symptom("use after free")
                .with_symptom("invalid memory access")
                .with_fix("Review object lifetime")
                .with_fix("Use reference counting or arena allocation"),
        );

        self.add_pattern(
            BugPattern::new("Memory Leak", BugCategory::Memory)
                .with_description("Memory was allocated but never freed")
                .with_severity(BugSeverity::High)
                .with_symptom("memory leak")
                .with_symptom("out of memory")
                .with_symptom("allocation failed")
                .with_fix("Add cleanup code")
                .with_fix("Use RAII pattern"),
        );

        self.add_pattern(
            BugPattern::new("Buffer Overflow", BugCategory::Memory)
                .with_description("Write exceeded buffer bounds")
                .with_severity(BugSeverity::Critical)
                .with_symptom("buffer overflow")
                .with_symptom("stack smashing")
                .with_symptom("heap corruption")
                .with_fix("Add bounds checking")
                .with_fix("Use safe string functions"),
        );

        // Concurrency patterns
        self.add_pattern(
            BugPattern::new("Deadlock", BugCategory::Concurrency)
                .with_description("Circular wait on resources")
                .with_severity(BugSeverity::High)
                .with_symptom("deadlock")
                .with_symptom("lock timeout")
                .with_symptom("circular wait")
                .with_fix("Use consistent lock ordering")
                .with_fix("Use try-lock with timeout"),
        );

        self.add_pattern(
            BugPattern::new("Data Race", BugCategory::Concurrency)
                .with_description("Unsynchronized access to shared data")
                .with_severity(BugSeverity::High)
                .with_symptom("data race")
                .with_symptom("inconsistent state")
                .with_fix("Add proper synchronization")
                .with_fix("Use atomic operations"),
        );

        // Resource patterns
        self.add_pattern(
            BugPattern::new("Resource Exhaustion", BugCategory::Resource)
                .with_description("System resource exhausted")
                .with_severity(BugSeverity::High)
                .with_symptom("resource exhausted")
                .with_symptom("too many")
                .with_symptom("limit exceeded")
                .with_fix("Increase resource limits")
                .with_fix("Implement resource pooling"),
        );

        // Logic patterns
        self.add_pattern(
            BugPattern::new("Integer Overflow", BugCategory::Logic)
                .with_description("Arithmetic overflow occurred")
                .with_severity(BugSeverity::High)
                .with_symptom("overflow")
                .with_symptom("arithmetic error")
                .with_fix("Use checked arithmetic")
                .with_fix("Use larger integer type"),
        );

        self.add_pattern(
            BugPattern::new("Division by Zero", BugCategory::Logic)
                .with_description("Attempted division by zero")
                .with_severity(BugSeverity::High)
                .with_symptom("division by zero")
                .with_symptom("divide error")
                .with_fix("Add zero check before division"),
        );
    }

    /// Add a pattern
    #[inline(always)]
    pub fn add_pattern(&mut self, pattern: BugPattern) {
        self.patterns.push(pattern);
    }

    /// Diagnose an issue
    pub fn diagnose(&mut self, context: &DebugContext) -> Diagnosis {
        self.total_diagnoses.fetch_add(1, Ordering::Relaxed);

        // Try to match against known patterns
        let mut best_match: Option<(&BugPattern, f64)> = None;

        for pattern in &self.patterns {
            if pattern.matches(&context.error) {
                let confidence = self.calculate_confidence(pattern, context);
                if confidence > best_match.map(|(_, c)| c).unwrap_or(0.0) {
                    best_match = Some((pattern, confidence));
                }
            }
        }

        let diagnosis = if let Some((pattern, confidence)) = best_match {
            self.successful_diagnoses.fetch_add(1, Ordering::Relaxed);

            let mut diag =
                Diagnosis::new(&pattern.description, confidence).with_pattern(pattern.clone());

            // Add fixes from pattern
            for fix_desc in &pattern.fixes {
                diag.fixes
                    .push(Fix::new(fix_desc, FixType::Code).with_confidence(confidence * 0.8));
            }

            diag
        } else {
            // Generic diagnosis
            Diagnosis::new("Unknown issue - further investigation needed", 0.3)
        };

        // Add to history
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(diagnosis.clone());

        diagnosis
    }

    /// Calculate confidence for a pattern match
    fn calculate_confidence(&self, pattern: &BugPattern, context: &DebugContext) -> f64 {
        let mut confidence = 0.5;

        // More matching symptoms = higher confidence
        let matching_symptoms = pattern
            .symptoms
            .iter()
            .filter(|s| context.error.to_lowercase().contains(&s.to_lowercase()))
            .count();
        confidence += 0.1 * matching_symptoms as f64;

        // Stack trace available = higher confidence
        if !context.stack_trace.is_empty() {
            confidence += 0.1;
        }

        // Memory context available = higher confidence for memory bugs
        if !context.memory_context.is_empty() && pattern.category == BugCategory::Memory {
            confidence += 0.1;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Analyze a stack trace
    pub fn analyze_stack(&self, frames: &[StackFrame]) -> Vec<String> {
        let mut insights = Vec::new();

        if frames.is_empty() {
            insights.push("No stack trace available".into());
            return insights;
        }

        // Look for known problematic functions
        for (i, frame) in frames.iter().enumerate() {
            if let Some(ref func) = frame.function {
                if func.contains("malloc") || func.contains("alloc") {
                    insights.push(format!("Frame {}: Memory allocation detected", i));
                }
                if func.contains("lock") || func.contains("mutex") {
                    insights.push(format!("Frame {}: Lock operation detected", i));
                }
                if func.contains("free") || func.contains("dealloc") {
                    insights.push(format!("Frame {}: Memory deallocation detected", i));
                }
            }
        }

        if insights.is_empty() {
            insights.push("Stack trace appears normal".into());
        }

        insights
    }

    /// Get diagnosis history
    #[inline(always)]
    pub fn history(&self) -> &[Diagnosis] {
        &self.history
    }

    /// Get patterns by category
    #[inline]
    pub fn patterns_by_category(&self, category: BugCategory) -> Vec<&BugPattern> {
        self.patterns
            .iter()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> DebuggerStats {
        let total = self.total_diagnoses.load(Ordering::Relaxed);
        let successful = self.successful_diagnoses.load(Ordering::Relaxed);

        DebuggerStats {
            pattern_count: self.patterns.len(),
            total_diagnoses: total,
            successful_diagnoses: successful,
            success_rate: if total > 0 {
                successful as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}
