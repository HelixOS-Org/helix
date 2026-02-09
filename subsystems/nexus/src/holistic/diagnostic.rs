//! # Holistic Diagnostic Engine
//!
//! System-wide diagnostic and root-cause analysis:
//! - Symptom collection and correlation
//! - Fault tree analysis
//! - Timeline reconstruction
//! - Impact assessment
//! - Remediation suggestions

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// DIAGNOSTIC TYPES
// ============================================================================

/// Symptom severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymptomSeverity {
    /// Info
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
    /// Fatal
    Fatal,
}

/// Symptom category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymptomCategory {
    /// CPU-related
    Cpu,
    /// Memory-related
    Memory,
    /// I/O related
    Io,
    /// Network-related
    Network,
    /// Scheduling-related
    Scheduling,
    /// Lock/sync related
    Synchronization,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Hardware error
    Hardware,
}

/// Diagnosis confidence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosisConfidence {
    /// Low confidence
    Low,
    /// Medium
    Medium,
    /// High
    High,
    /// Certain
    Certain,
}

// ============================================================================
// SYMPTOM
// ============================================================================

/// Observed symptom
#[derive(Debug, Clone)]
pub struct Symptom {
    /// Symptom id
    pub id: u64,
    /// Category
    pub category: SymptomCategory,
    /// Severity
    pub severity: SymptomSeverity,
    /// Description
    pub description: String,
    /// Affected process (0 = system-wide)
    pub affected_pid: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Metric name
    pub metric: String,
    /// Metric value
    pub value: f64,
    /// Expected range min
    pub expected_min: f64,
    /// Expected range max
    pub expected_max: f64,
}

impl Symptom {
    /// Is anomalous? (value outside expected range)
    #[inline(always)]
    pub fn is_anomalous(&self) -> bool {
        self.value < self.expected_min || self.value > self.expected_max
    }

    /// Deviation from expected
    #[inline]
    pub fn deviation(&self) -> f64 {
        if self.value > self.expected_max {
            self.value - self.expected_max
        } else if self.value < self.expected_min {
            self.expected_min - self.value
        } else {
            0.0
        }
    }
}

// ============================================================================
// FAULT TREE
// ============================================================================

/// Fault tree node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultNodeType {
    /// AND gate (all children must be true)
    And,
    /// OR gate (any child must be true)
    Or,
    /// Basic event (leaf)
    BasicEvent,
    /// Intermediate event
    Intermediate,
}

/// Fault tree node
#[derive(Debug, Clone)]
pub struct FaultNode {
    /// Node id
    pub id: u64,
    /// Type
    pub node_type: FaultNodeType,
    /// Description
    pub description: String,
    /// Is active (symptom observed)
    pub is_active: bool,
    /// Children ids
    pub children: Vec<u64>,
    /// Associated symptom ids
    pub symptoms: Vec<u64>,
    /// Probability (0.0-1.0)
    pub probability: f64,
}

impl FaultNode {
    pub fn new(id: u64, node_type: FaultNodeType, description: String) -> Self {
        Self {
            id,
            node_type,
            description,
            is_active: false,
            children: Vec::new(),
            symptoms: Vec::new(),
            probability: 0.0,
        }
    }
}

/// Fault tree
#[derive(Debug)]
pub struct FaultTree {
    /// Nodes
    nodes: BTreeMap<u64, FaultNode>,
    /// Root node id
    pub root: u64,
}

impl FaultTree {
    pub fn new(root: FaultNode) -> Self {
        let root_id = root.id;
        let mut nodes = BTreeMap::new();
        nodes.insert(root_id, root);
        Self {
            nodes,
            root: root_id,
        }
    }

    /// Add node
    #[inline]
    pub fn add_node(&mut self, node: FaultNode, parent: u64) {
        let id = node.id;
        self.nodes.insert(id, node);
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.children.push(id);
        }
    }

    /// Evaluate tree
    #[inline(always)]
    pub fn evaluate(&mut self) -> bool {
        let root_id = self.root;
        self.evaluate_node(root_id)
    }

    fn evaluate_node(&mut self, id: u64) -> bool {
        let node = match self.nodes.get(&id) {
            Some(n) => n.clone(),
            None => return false,
        };

        let result = match node.node_type {
            FaultNodeType::BasicEvent => node.is_active,
            FaultNodeType::And => {
                if node.children.is_empty() {
                    false
                } else {
                    node.children.iter().all(|&c| self.evaluate_node(c))
                }
            }
            FaultNodeType::Or => {
                node.children.iter().any(|&c| self.evaluate_node(c))
            }
            FaultNodeType::Intermediate => {
                node.is_active || node.children.iter().any(|&c| self.evaluate_node(c))
            }
        };

        if let Some(n) = self.nodes.get_mut(&id) {
            n.is_active = result;
        }
        result
    }

    /// Get active paths (root-cause chain)
    #[inline]
    pub fn active_paths(&self) -> Vec<Vec<u64>> {
        let mut paths = Vec::new();
        let mut current = Vec::new();
        self.collect_paths(self.root, &mut current, &mut paths);
        paths
    }

    fn collect_paths(&self, id: u64, current: &mut Vec<u64>, paths: &mut Vec<Vec<u64>>) {
        if let Some(node) = self.nodes.get(&id) {
            if !node.is_active {
                return;
            }
            current.push(id);
            if node.children.is_empty() || node.node_type == FaultNodeType::BasicEvent {
                paths.push(current.clone());
            } else {
                for &child in &node.children {
                    self.collect_paths(child, current, paths);
                }
            }
            current.pop();
        }
    }
}

// ============================================================================
// DIAGNOSIS
// ============================================================================

/// Root cause hypothesis
#[derive(Debug, Clone)]
pub struct RootCause {
    /// Description
    pub description: String,
    /// Category
    pub category: SymptomCategory,
    /// Confidence
    pub confidence: DiagnosisConfidence,
    /// Supporting symptoms
    pub supporting_symptoms: Vec<u64>,
    /// Suggested remediations
    pub remediations: Vec<String>,
}

/// Diagnosis report
#[derive(Debug)]
pub struct DiagnosisReport {
    /// Report id
    pub id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Symptoms analyzed
    pub symptom_count: usize,
    /// Root causes found
    pub root_causes: Vec<RootCause>,
    /// Affected processes
    pub affected_pids: Vec<u64>,
    /// Overall severity
    pub severity: SymptomSeverity,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Diagnostic stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticDiagnosticStats {
    /// Total symptoms collected
    pub total_symptoms: u64,
    /// Active symptoms
    pub active_symptoms: usize,
    /// Total diagnoses
    pub total_diagnoses: u64,
    /// Root causes identified
    pub root_causes_found: u64,
}

/// Holistic diagnostic engine
pub struct HolisticDiagnosticEngine {
    /// Collected symptoms
    symptoms: BTreeMap<u64, Symptom>,
    /// Fault trees
    fault_trees: Vec<FaultTree>,
    /// Reports
    reports: Vec<DiagnosisReport>,
    /// Next ids
    next_symptom_id: u64,
    next_report_id: u64,
    /// Max symptoms
    max_symptoms: usize,
    /// Stats
    stats: HolisticDiagnosticStats,
}

impl HolisticDiagnosticEngine {
    pub fn new() -> Self {
        Self {
            symptoms: BTreeMap::new(),
            fault_trees: Vec::new(),
            reports: Vec::new(),
            next_symptom_id: 1,
            next_report_id: 1,
            max_symptoms: 10000,
            stats: HolisticDiagnosticStats::default(),
        }
    }

    /// Report symptom
    pub fn report_symptom(&mut self, mut symptom: Symptom) -> u64 {
        let id = self.next_symptom_id;
        self.next_symptom_id += 1;
        symptom.id = id;
        if self.symptoms.len() >= self.max_symptoms {
            // Remove oldest
            if let Some(&oldest) = self.symptoms.keys().next() {
                self.symptoms.remove(&oldest);
            }
        }
        self.symptoms.insert(id, symptom);
        self.stats.total_symptoms += 1;
        self.update_stats();
        id
    }

    /// Run diagnosis
    pub fn diagnose(&mut self, now: u64) -> DiagnosisReport {
        let id = self.next_report_id;
        self.next_report_id += 1;

        // Correlate symptoms by category
        let mut by_category: BTreeMap<u8, Vec<&Symptom>> = BTreeMap::new();
        for sym in self.symptoms.values() {
            if sym.is_anomalous() {
                by_category.entry(sym.category as u8)
                    .or_insert_with(Vec::new)
                    .push(sym);
            }
        }

        let mut root_causes = Vec::new();
        let mut affected = Vec::new();
        let mut max_severity = SymptomSeverity::Info;

        for (_, symptoms) in &by_category {
            if symptoms.is_empty() {
                continue;
            }

            let category = symptoms[0].category;
            let max_sev = symptoms.iter()
                .map(|s| s.severity)
                .max()
                .unwrap_or(SymptomSeverity::Info);

            if max_sev > max_severity {
                max_severity = max_sev;
            }

            let confidence = if symptoms.len() >= 3 {
                DiagnosisConfidence::High
            } else if symptoms.len() >= 2 {
                DiagnosisConfidence::Medium
            } else {
                DiagnosisConfidence::Low
            };

            let supporting: Vec<u64> = symptoms.iter().map(|s| s.id).collect();
            for s in symptoms {
                if s.affected_pid != 0 && !affected.contains(&s.affected_pid) {
                    affected.push(s.affected_pid);
                }
            }

            let description = match category {
                SymptomCategory::Cpu => String::from("CPU contention detected"),
                SymptomCategory::Memory => String::from("Memory pressure detected"),
                SymptomCategory::Io => String::from("I/O bottleneck detected"),
                SymptomCategory::Network => String::from("Network congestion detected"),
                SymptomCategory::Scheduling => String::from("Scheduling anomaly detected"),
                SymptomCategory::Synchronization => String::from("Lock contention detected"),
                SymptomCategory::ResourceExhaustion => String::from("Resource exhaustion detected"),
                SymptomCategory::Hardware => String::from("Hardware issue detected"),
            };

            root_causes.push(RootCause {
                description,
                category,
                confidence,
                supporting_symptoms: supporting,
                remediations: Vec::new(),
            });
        }

        self.stats.total_diagnoses += 1;
        self.stats.root_causes_found += root_causes.len() as u64;

        let report = DiagnosisReport {
            id,
            timestamp: now,
            symptom_count: self.symptoms.len(),
            root_causes,
            affected_pids: affected,
            severity: max_severity,
        };

        self.update_stats();
        report
    }

    /// Clear old symptoms
    #[inline(always)]
    pub fn clear_old(&mut self, before: u64) {
        self.symptoms.retain(|_, s| s.timestamp >= before);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.active_symptoms = self.symptoms.values()
            .filter(|s| s.is_anomalous()).count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticDiagnosticStats {
        &self.stats
    }
}
