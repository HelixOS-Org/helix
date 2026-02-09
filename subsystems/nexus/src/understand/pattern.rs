//! # Pattern Understanding
//!
//! Pattern recognition and matching for code understanding.
//! Identifies recurring structures and idioms.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PATTERN TYPES
// ============================================================================

/// Code pattern
#[derive(Debug, Clone)]
pub struct CodePattern {
    /// Pattern ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Pattern type
    pub pattern_type: PatternType,
    /// Structure
    pub structure: PatternStructure,
    /// Constraints
    pub constraints: Vec<PatternConstraint>,
    /// Examples
    pub examples: Vec<u64>,
    /// Occurrences
    pub occurrences: u64,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Structural pattern
    Structural,
    /// Behavioral pattern
    Behavioral,
    /// Creational pattern
    Creational,
    /// Idiom
    Idiom,
    /// Anti-pattern
    AntiPattern,
    /// Custom
    Custom,
}

/// Pattern structure
#[derive(Debug, Clone)]
pub struct PatternStructure {
    /// Elements
    pub elements: Vec<PatternElement>,
    /// Relations
    pub relations: Vec<PatternRelation>,
}

/// Pattern element
#[derive(Debug, Clone)]
pub struct PatternElement {
    /// Element ID
    pub id: u64,
    /// Name (can be variable)
    pub name: String,
    /// Element type
    pub element_type: ElementType,
    /// Is variable (can match anything)
    pub is_variable: bool,
    /// Cardinality
    pub cardinality: Cardinality,
}

/// Element type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Class,
    Interface,
    Function,
    Method,
    Field,
    Parameter,
    Statement,
    Expression,
    Any,
}

/// Cardinality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    One,
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
}

/// Pattern relation
#[derive(Debug, Clone)]
pub struct PatternRelation {
    /// Source element
    pub source: u64,
    /// Target element
    pub target: u64,
    /// Relation type
    pub relation_type: RelationType,
}

/// Relation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    Contains,
    Implements,
    Extends,
    Uses,
    Creates,
    Calls,
    References,
}

/// Pattern constraint
#[derive(Debug, Clone)]
pub struct PatternConstraint {
    /// Element ID
    pub element_id: u64,
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Value
    pub value: String,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    NamePattern,
    TypePattern,
    Visibility,
    Modifier,
    MinCount,
    MaxCount,
}

/// Pattern match
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Match ID
    pub id: u64,
    /// Pattern ID
    pub pattern_id: u64,
    /// Location
    pub location: MatchLocation,
    /// Bindings
    pub bindings: BTreeMap<String, String>,
    /// Confidence
    pub confidence: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Match location
#[derive(Debug, Clone)]
pub struct MatchLocation {
    /// File
    pub file: String,
    /// Start line
    pub start_line: u32,
    /// End line
    pub end_line: u32,
}

// ============================================================================
// PATTERN MATCHER
// ============================================================================

/// Pattern matcher
pub struct PatternMatcher {
    /// Patterns
    patterns: BTreeMap<u64, CodePattern>,
    /// Matches
    matches: BTreeMap<u64, PatternMatch>,
    /// Pattern index by type
    type_index: BTreeMap<PatternType, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: MatcherConfig,
    /// Statistics
    stats: MatcherStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MatcherConfig {
    /// Minimum confidence
    pub min_confidence: f64,
    /// Enable fuzzy matching
    pub fuzzy_matching: bool,
    /// Maximum matches per pattern
    pub max_matches: usize,
}

impl Default for MatcherConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            fuzzy_matching: true,
            max_matches: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MatcherStats {
    /// Patterns registered
    pub patterns_registered: u64,
    /// Matches found
    pub matches_found: u64,
    /// Searches performed
    pub searches: u64,
}

impl PatternMatcher {
    /// Create new matcher
    pub fn new(config: MatcherConfig) -> Self {
        Self {
            patterns: BTreeMap::new(),
            matches: BTreeMap::new(),
            type_index: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: MatcherStats::default(),
        }
    }

    /// Register pattern
    pub fn register_pattern(&mut self, pattern: CodePattern) -> u64 {
        let id = pattern.id;

        self.type_index
            .entry(pattern.pattern_type)
            .or_insert_with(Vec::new)
            .push(id);

        self.patterns.insert(id, pattern);
        self.stats.patterns_registered += 1;

        id
    }

    /// Create pattern
    #[inline(always)]
    pub fn create_pattern(&mut self, name: &str, pattern_type: PatternType) -> PatternBuilder {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        PatternBuilder::new(id, name, pattern_type)
    }

    /// Match code (simplified representation)
    pub fn match_code(&mut self, code: &CodeRepresentation) -> Vec<PatternMatch> {
        self.stats.searches += 1;
        let mut all_matches = Vec::new();

        for pattern in self.patterns.values() {
            let matches = self.find_matches(pattern, code);
            all_matches.extend(matches);
        }

        // Store matches
        for match_ in &all_matches {
            self.matches.insert(match_.id, match_.clone());
        }

        self.stats.matches_found += all_matches.len() as u64;

        all_matches
    }

    fn find_matches(
        &mut self,
        pattern: &CodePattern,
        code: &CodeRepresentation,
    ) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        // Match structure
        for node in &code.nodes {
            if let Some(bindings) = self.try_match_structure(&pattern.structure, node, code) {
                // Check constraints
                if self.check_constraints(&pattern.constraints, &bindings) {
                    let confidence = self.compute_confidence(pattern, &bindings);

                    if confidence >= self.config.min_confidence {
                        let match_id = self.next_id.fetch_add(1, Ordering::Relaxed);

                        matches.push(PatternMatch {
                            id: match_id,
                            pattern_id: pattern.id,
                            location: MatchLocation {
                                file: node.file.clone(),
                                start_line: node.line,
                                end_line: node.line,
                            },
                            bindings,
                            confidence,
                            timestamp: Timestamp::now(),
                        });

                        if matches.len() >= self.config.max_matches {
                            break;
                        }
                    }
                }
            }
        }

        matches
    }

    fn try_match_structure(
        &self,
        structure: &PatternStructure,
        node: &CodeNode,
        code: &CodeRepresentation,
    ) -> Option<BTreeMap<String, String>> {
        let mut bindings = BTreeMap::new();

        // Match first element
        if let Some(first_elem) = structure.elements.first() {
            if !self.matches_element(first_elem, node) {
                return None;
            }

            if first_elem.is_variable {
                bindings.insert(first_elem.name.clone(), node.name.clone());
            }

            // Match relations
            for relation in &structure.relations {
                if relation.source == first_elem.id {
                    // Find target element
                    if let Some(target_elem) =
                        structure.elements.iter().find(|e| e.id == relation.target)
                    {
                        // Find matching child node
                        let found = code.nodes.iter().any(|child| {
                            self.matches_element(target_elem, child)
                                && self.matches_relation(relation.relation_type, node, child)
                        });

                        if !found {
                            return None;
                        }
                    }
                }
            }
        }

        Some(bindings)
    }

    fn matches_element(&self, element: &PatternElement, node: &CodeNode) -> bool {
        if element.element_type != ElementType::Any && element.element_type != node.node_type {
            return false;
        }

        if !element.is_variable && element.name != node.name {
            return false;
        }

        true
    }

    fn matches_relation(
        &self,
        relation: RelationType,
        source: &CodeNode,
        target: &CodeNode,
    ) -> bool {
        // Simplified relation matching
        source.children.contains(&target.id) || source.references.contains(&target.id)
    }

    fn check_constraints(
        &self,
        constraints: &[PatternConstraint],
        bindings: &BTreeMap<String, String>,
    ) -> bool {
        for constraint in constraints {
            match constraint.constraint_type {
                ConstraintType::NamePattern => {
                    // Check if bound name matches pattern
                    let matches = bindings.values().any(|v| v.contains(&constraint.value));
                    if !matches {
                        return false;
                    }
                },
                _ => {},
            }
        }
        true
    }

    fn compute_confidence(
        &self,
        pattern: &CodePattern,
        bindings: &BTreeMap<String, String>,
    ) -> f64 {
        let mut score = 0.7; // Base score

        // More bindings = higher confidence
        score += bindings.len() as f64 * 0.05;

        // Pattern occurrence history
        if pattern.occurrences > 0 {
            score += 0.1;
        }

        score.min(1.0)
    }

    /// Get pattern
    #[inline(always)]
    pub fn get_pattern(&self, id: u64) -> Option<&CodePattern> {
        self.patterns.get(&id)
    }

    /// Get patterns by type
    #[inline]
    pub fn get_patterns_by_type(&self, pattern_type: PatternType) -> Vec<&CodePattern> {
        self.type_index
            .get(&pattern_type)
            .map(|ids| ids.iter().filter_map(|id| self.patterns.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get match
    #[inline(always)]
    pub fn get_match(&self, id: u64) -> Option<&PatternMatch> {
        self.matches.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &MatcherStats {
        &self.stats
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new(MatcherConfig::default())
    }
}

// ============================================================================
// CODE REPRESENTATION
// ============================================================================

/// Code representation (simplified AST)
#[derive(Debug, Clone, Default)]
pub struct CodeRepresentation {
    /// Nodes
    pub nodes: Vec<CodeNode>,
}

/// Code node
#[derive(Debug, Clone)]
pub struct CodeNode {
    /// Node ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Node type
    pub node_type: ElementType,
    /// File
    pub file: String,
    /// Line
    pub line: u32,
    /// Children
    pub children: Vec<u64>,
    /// References
    pub references: Vec<u64>,
}

// ============================================================================
// PATTERN BUILDER
// ============================================================================

/// Pattern builder
pub struct PatternBuilder {
    id: u64,
    name: String,
    pattern_type: PatternType,
    elements: Vec<PatternElement>,
    relations: Vec<PatternRelation>,
    constraints: Vec<PatternConstraint>,
    next_elem_id: u64,
}

impl PatternBuilder {
    /// Create new builder
    pub fn new(id: u64, name: &str, pattern_type: PatternType) -> Self {
        Self {
            id,
            name: name.into(),
            pattern_type,
            elements: Vec::new(),
            relations: Vec::new(),
            constraints: Vec::new(),
            next_elem_id: 1,
        }
    }

    /// Add element
    #[inline]
    pub fn element(mut self, name: &str, elem_type: ElementType, is_variable: bool) -> Self {
        self.elements.push(PatternElement {
            id: self.next_elem_id,
            name: name.into(),
            element_type: elem_type,
            is_variable,
            cardinality: Cardinality::One,
        });
        self.next_elem_id += 1;
        self
    }

    /// Add relation
    #[inline]
    pub fn relation(mut self, source: u64, target: u64, rel_type: RelationType) -> Self {
        self.relations.push(PatternRelation {
            source,
            target,
            relation_type: rel_type,
        });
        self
    }

    /// Add constraint
    #[inline]
    pub fn constraint(mut self, element_id: u64, const_type: ConstraintType, value: &str) -> Self {
        self.constraints.push(PatternConstraint {
            element_id,
            constraint_type: const_type,
            value: value.into(),
        });
        self
    }

    /// Build
    pub fn build(self) -> CodePattern {
        CodePattern {
            id: self.id,
            name: self.name,
            pattern_type: self.pattern_type,
            structure: PatternStructure {
                elements: self.elements,
                relations: self.relations,
            },
            constraints: self.constraints,
            examples: Vec::new(),
            occurrences: 0,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pattern() {
        let mut matcher = PatternMatcher::default();

        let pattern = matcher
            .create_pattern("Singleton", PatternType::Creational)
            .element("class", ElementType::Class, true)
            .element("instance", ElementType::Field, false)
            .relation(1, 2, RelationType::Contains)
            .build();

        matcher.register_pattern(pattern);

        assert_eq!(matcher.stats.patterns_registered, 1);
    }

    #[test]
    fn test_pattern_matching() {
        let mut matcher = PatternMatcher::default();

        let pattern = matcher
            .create_pattern("Test", PatternType::Structural)
            .element("MyClass", ElementType::Class, false)
            .build();

        matcher.register_pattern(pattern);

        let code = CodeRepresentation {
            nodes: vec![CodeNode {
                id: 1,
                name: "MyClass".into(),
                node_type: ElementType::Class,
                file: "test.rs".into(),
                line: 1,
                children: Vec::new(),
                references: Vec::new(),
            }],
        };

        let matches = matcher.match_code(&code);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_variable_binding() {
        let mut matcher = PatternMatcher::default();

        let pattern = matcher.create_pattern("AnyClass", PatternType::Structural)
            .element("X", ElementType::Class, true) // Variable
            .build();

        matcher.register_pattern(pattern);

        let code = CodeRepresentation {
            nodes: vec![CodeNode {
                id: 1,
                name: "SpecificClass".into(),
                node_type: ElementType::Class,
                file: "test.rs".into(),
                line: 1,
                children: Vec::new(),
                references: Vec::new(),
            }],
        };

        let matches = matcher.match_code(&code);
        assert!(!matches.is_empty());
        assert!(matches[0].bindings.contains_key("X"));
    }

    #[test]
    fn test_get_patterns_by_type() {
        let mut matcher = PatternMatcher::default();

        let p1 = matcher
            .create_pattern("P1", PatternType::Structural)
            .build();
        let p2 = matcher
            .create_pattern("P2", PatternType::Behavioral)
            .build();
        let p3 = matcher
            .create_pattern("P3", PatternType::Structural)
            .build();

        matcher.register_pattern(p1);
        matcher.register_pattern(p2);
        matcher.register_pattern(p3);

        let structural = matcher.get_patterns_by_type(PatternType::Structural);
        assert_eq!(structural.len(), 2);
    }
}
