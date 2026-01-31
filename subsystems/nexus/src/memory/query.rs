//! # Memory Query Engine
//!
//! Query interface for memory systems.
//! Supports complex queries across memory types.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// QUERY TYPES
// ============================================================================

/// Memory query
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// Query ID
    pub id: u64,
    /// Query type
    pub query_type: QueryType,
    /// Filters
    pub filters: Vec<Filter>,
    /// Sort order
    pub sort: Option<SortOrder>,
    /// Limit
    pub limit: Option<usize>,
    /// Offset
    pub offset: usize,
    /// Include related
    pub include_related: bool,
    /// Memory types to search
    pub memory_types: Vec<MemoryType>,
}

/// Query type
#[derive(Debug, Clone)]
pub enum QueryType {
    /// Exact key lookup
    ByKey(String),
    /// Pattern match
    Pattern(String),
    /// Full-text search
    FullText(String),
    /// Semantic similarity
    Semantic { embedding: Vec<f32>, threshold: f64 },
    /// Time range
    TimeRange { start: Timestamp, end: Timestamp },
    /// Association
    AssociatedWith(u64),
    /// Combined query
    Combined(Vec<MemoryQuery>),
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Working,
}

/// Filter
#[derive(Debug, Clone)]
pub struct Filter {
    /// Field
    pub field: String,
    /// Operator
    pub operator: FilterOp,
    /// Value
    pub value: FilterValue,
}

/// Filter operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Exists,
    NotExists,
}

/// Filter value
#[derive(Debug, Clone)]
pub enum FilterValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<FilterValue>),
    Null,
}

/// Sort order
#[derive(Debug, Clone)]
pub struct SortOrder {
    /// Field
    pub field: String,
    /// Direction
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

// ============================================================================
// QUERY RESULT
// ============================================================================

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Query ID
    pub query_id: u64,
    /// Matches
    pub matches: Vec<MatchedMemory>,
    /// Total count (before limit)
    pub total_count: usize,
    /// Execution time (ns)
    pub execution_time_ns: u64,
    /// Query plan
    pub plan: Option<QueryPlan>,
}

/// Matched memory
#[derive(Debug, Clone)]
pub struct MatchedMemory {
    /// Memory ID
    pub id: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Key
    pub key: String,
    /// Content summary
    pub summary: String,
    /// Relevance score
    pub score: f64,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Related memories
    pub related: Vec<u64>,
}

/// Query plan
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Steps
    pub steps: Vec<PlanStep>,
    /// Estimated cost
    pub estimated_cost: f64,
}

/// Plan step
#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Operation
    pub operation: String,
    /// Index used
    pub index: Option<String>,
    /// Estimated rows
    pub estimated_rows: usize,
}

// ============================================================================
// QUERY BUILDER
// ============================================================================

/// Query builder
pub struct QueryBuilder {
    query_type: Option<QueryType>,
    filters: Vec<Filter>,
    sort: Option<SortOrder>,
    limit: Option<usize>,
    offset: usize,
    include_related: bool,
    memory_types: Vec<MemoryType>,
}

impl QueryBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            query_type: None,
            filters: Vec::new(),
            sort: None,
            limit: None,
            offset: 0,
            include_related: false,
            memory_types: Vec::new(),
        }
    }

    /// Lookup by key
    pub fn by_key(mut self, key: &str) -> Self {
        self.query_type = Some(QueryType::ByKey(key.into()));
        self
    }

    /// Pattern match
    pub fn pattern(mut self, pattern: &str) -> Self {
        self.query_type = Some(QueryType::Pattern(pattern.into()));
        self
    }

    /// Full-text search
    pub fn full_text(mut self, query: &str) -> Self {
        self.query_type = Some(QueryType::FullText(query.into()));
        self
    }

    /// Time range
    pub fn time_range(mut self, start: Timestamp, end: Timestamp) -> Self {
        self.query_type = Some(QueryType::TimeRange { start, end });
        self
    }

    /// Filter by field
    pub fn filter(mut self, field: &str, operator: FilterOp, value: FilterValue) -> Self {
        self.filters.push(Filter {
            field: field.into(),
            operator,
            value,
        });
        self
    }

    /// Filter equals
    pub fn where_eq(self, field: &str, value: &str) -> Self {
        self.filter(field, FilterOp::Equals, FilterValue::String(value.into()))
    }

    /// Filter contains
    pub fn where_contains(self, field: &str, value: &str) -> Self {
        self.filter(field, FilterOp::Contains, FilterValue::String(value.into()))
    }

    /// Sort by
    pub fn order_by(mut self, field: &str, direction: SortDirection) -> Self {
        self.sort = Some(SortOrder {
            field: field.into(),
            direction,
        });
        self
    }

    /// Limit results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Skip results
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Include related
    pub fn with_related(mut self) -> Self {
        self.include_related = true;
        self
    }

    /// Search in memory types
    pub fn in_types(mut self, types: Vec<MemoryType>) -> Self {
        self.memory_types = types;
        self
    }

    /// Build query
    pub fn build(self, id: u64) -> MemoryQuery {
        MemoryQuery {
            id,
            query_type: self.query_type.unwrap_or(QueryType::Pattern("*".into())),
            filters: self.filters,
            sort: self.sort,
            limit: self.limit,
            offset: self.offset,
            include_related: self.include_related,
            memory_types: if self.memory_types.is_empty() {
                vec![
                    MemoryType::Episodic,
                    MemoryType::Semantic,
                    MemoryType::Procedural,
                ]
            } else {
                self.memory_types
            },
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// QUERY ENGINE
// ============================================================================

/// Memory query engine
pub struct MemoryQueryEngine {
    /// Memory index (simplified)
    memories: BTreeMap<u64, StoredMemory>,
    /// Key index
    by_key: BTreeMap<String, u64>,
    /// Type index
    by_type: BTreeMap<MemoryType, Vec<u64>>,
    /// Tag index
    by_tag: BTreeMap<String, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: QueryConfig,
    /// Statistics
    stats: QueryStats,
}

/// Stored memory (simplified)
#[derive(Debug, Clone)]
struct StoredMemory {
    id: u64,
    memory_type: MemoryType,
    key: String,
    content: String,
    metadata: BTreeMap<String, String>,
    tags: Vec<String>,
    related: Vec<u64>,
    created: Timestamp,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Maximum results
    pub max_results: usize,
    /// Enable query planning
    pub enable_planning: bool,
    /// Timeout (ns)
    pub timeout_ns: u64,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            max_results: 1000,
            enable_planning: true,
            timeout_ns: 5_000_000_000, // 5 seconds
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct QueryStats {
    /// Queries executed
    pub queries_executed: u64,
    /// Average execution time (ns)
    pub avg_execution_ns: f64,
    /// Cache hits
    pub cache_hits: u64,
    /// Full scans
    pub full_scans: u64,
}

impl MemoryQueryEngine {
    /// Create new engine
    pub fn new(config: QueryConfig) -> Self {
        Self {
            memories: BTreeMap::new(),
            by_key: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: QueryStats::default(),
        }
    }

    /// Index memory
    pub fn index(
        &mut self,
        memory_type: MemoryType,
        key: &str,
        content: &str,
        metadata: BTreeMap<String, String>,
        tags: Vec<String>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let memory = StoredMemory {
            id,
            memory_type,
            key: key.into(),
            content: content.into(),
            metadata,
            tags: tags.clone(),
            related: Vec::new(),
            created: Timestamp::now(),
        };

        self.by_key.insert(key.into(), id);
        self.by_type
            .entry(memory_type)
            .or_insert_with(Vec::new)
            .push(id);

        for tag in &tags {
            self.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.memories.insert(id, memory);
        id
    }

    /// Link memories
    pub fn link(&mut self, from: u64, to: u64) {
        if let Some(memory) = self.memories.get_mut(&from) {
            if !memory.related.contains(&to) {
                memory.related.push(to);
            }
        }
    }

    /// Execute query
    pub fn execute(&mut self, query: MemoryQuery) -> QueryResult {
        let start = Timestamp::now();
        self.stats.queries_executed += 1;

        let plan = if self.config.enable_planning {
            Some(self.plan_query(&query))
        } else {
            None
        };

        let mut matches = self.execute_query_type(&query);

        // Apply filters
        matches = self.apply_filters(matches, &query.filters);

        // Get total before limit
        let total_count = matches.len();

        // Apply sort
        if let Some(ref sort) = query.sort {
            self.apply_sort(&mut matches, sort);
        }

        // Apply offset and limit
        if query.offset > 0 {
            matches = matches.into_iter().skip(query.offset).collect();
        }
        if let Some(limit) = query.limit {
            matches.truncate(limit);
        }

        // Add related if requested
        if query.include_related {
            for m in &mut matches {
                if let Some(memory) = self.memories.get(&m.id) {
                    m.related = memory.related.clone();
                }
            }
        }

        let execution_time_ns = Timestamp::now().0 - start.0;

        // Update stats
        let n = self.stats.queries_executed as f64;
        self.stats.avg_execution_ns =
            (self.stats.avg_execution_ns * (n - 1.0) + execution_time_ns as f64) / n;

        QueryResult {
            query_id: query.id,
            matches,
            total_count,
            execution_time_ns,
            plan,
        }
    }

    fn execute_query_type(&self, query: &MemoryQuery) -> Vec<MatchedMemory> {
        match &query.query_type {
            QueryType::ByKey(key) => self
                .by_key
                .get(key)
                .and_then(|id| self.memories.get(id))
                .map(|m| vec![self.to_matched(m, 1.0)])
                .unwrap_or_default(),
            QueryType::Pattern(pattern) => self.search_pattern(pattern, &query.memory_types),
            QueryType::FullText(text) => self.search_full_text(text, &query.memory_types),
            QueryType::TimeRange { start, end } => {
                self.search_time_range(*start, *end, &query.memory_types)
            },
            QueryType::AssociatedWith(id) => self.find_associated(*id),
            _ => Vec::new(),
        }
    }

    fn search_pattern(&self, pattern: &str, types: &[MemoryType]) -> Vec<MatchedMemory> {
        let pattern = pattern.to_lowercase();
        let is_wildcard = pattern == "*";

        self.memories
            .values()
            .filter(|m| types.is_empty() || types.contains(&m.memory_type))
            .filter(|m| is_wildcard || m.key.to_lowercase().contains(&pattern))
            .map(|m| {
                let score = if is_wildcard { 0.5 } else { 1.0 };
                self.to_matched(m, score)
            })
            .collect()
    }

    fn search_full_text(&self, query: &str, types: &[MemoryType]) -> Vec<MatchedMemory> {
        let query_lower = query.to_lowercase();
        let words: Vec<&str> = query_lower.split_whitespace().collect();

        self.memories
            .values()
            .filter(|m| types.is_empty() || types.contains(&m.memory_type))
            .filter_map(|m| {
                let content_lower = m.content.to_lowercase();
                let matches: usize = words.iter().filter(|w| content_lower.contains(*w)).count();

                if matches > 0 {
                    let score = matches as f64 / words.len() as f64;
                    Some(self.to_matched(m, score))
                } else {
                    None
                }
            })
            .collect()
    }

    fn search_time_range(
        &self,
        start: Timestamp,
        end: Timestamp,
        types: &[MemoryType],
    ) -> Vec<MatchedMemory> {
        self.memories
            .values()
            .filter(|m| types.is_empty() || types.contains(&m.memory_type))
            .filter(|m| m.created.0 >= start.0 && m.created.0 <= end.0)
            .map(|m| self.to_matched(m, 1.0))
            .collect()
    }

    fn find_associated(&self, id: u64) -> Vec<MatchedMemory> {
        if let Some(memory) = self.memories.get(&id) {
            memory
                .related
                .iter()
                .filter_map(|&related_id| self.memories.get(&related_id))
                .map(|m| self.to_matched(m, 0.8))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn apply_filters(&self, matches: Vec<MatchedMemory>, filters: &[Filter]) -> Vec<MatchedMemory> {
        matches
            .into_iter()
            .filter(|m| filters.iter().all(|f| self.matches_filter(m, f)))
            .collect()
    }

    fn matches_filter(&self, matched: &MatchedMemory, filter: &Filter) -> bool {
        let value = matched.metadata.get(&filter.field);

        match (&filter.operator, &filter.value, value) {
            (FilterOp::Exists, _, Some(_)) => true,
            (FilterOp::NotExists, _, None) => true,
            (FilterOp::Equals, FilterValue::String(expected), Some(actual)) => actual == expected,
            (FilterOp::NotEquals, FilterValue::String(expected), Some(actual)) => {
                actual != expected
            },
            (FilterOp::Contains, FilterValue::String(expected), Some(actual)) => {
                actual.contains(expected.as_str())
            },
            (FilterOp::StartsWith, FilterValue::String(expected), Some(actual)) => {
                actual.starts_with(expected.as_str())
            },
            (FilterOp::EndsWith, FilterValue::String(expected), Some(actual)) => {
                actual.ends_with(expected.as_str())
            },
            _ => false,
        }
    }

    fn apply_sort(&self, matches: &mut [MatchedMemory], sort: &SortOrder) {
        matches.sort_by(|a, b| {
            let a_val = a.metadata.get(&sort.field);
            let b_val = b.metadata.get(&sort.field);

            let cmp = a_val.cmp(&b_val);

            match sort.direction {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
    }

    fn to_matched(&self, memory: &StoredMemory, score: f64) -> MatchedMemory {
        MatchedMemory {
            id: memory.id,
            memory_type: memory.memory_type,
            key: memory.key.clone(),
            summary: if memory.content.len() > 100 {
                format!("{}...", &memory.content[..100])
            } else {
                memory.content.clone()
            },
            score,
            metadata: memory.metadata.clone(),
            related: Vec::new(),
        }
    }

    fn plan_query(&self, query: &MemoryQuery) -> QueryPlan {
        let mut steps = Vec::new();

        match &query.query_type {
            QueryType::ByKey(_) => {
                steps.push(PlanStep {
                    operation: "Index Lookup".into(),
                    index: Some("by_key".into()),
                    estimated_rows: 1,
                });
            },
            QueryType::Pattern(_) | QueryType::FullText(_) => {
                steps.push(PlanStep {
                    operation: "Full Scan".into(),
                    index: None,
                    estimated_rows: self.memories.len(),
                });
            },
            _ => {
                steps.push(PlanStep {
                    operation: "Scan".into(),
                    index: None,
                    estimated_rows: self.memories.len(),
                });
            },
        }

        if !query.filters.is_empty() {
            steps.push(PlanStep {
                operation: "Filter".into(),
                index: None,
                estimated_rows: self.memories.len() / 2,
            });
        }

        QueryPlan {
            steps,
            estimated_cost: steps.iter().map(|s| s.estimated_rows as f64).sum(),
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &QueryStats {
        &self.stats
    }
}

impl Default for MemoryQueryEngine {
    fn default() -> Self {
        Self::new(QueryConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_and_query() {
        let mut engine = MemoryQueryEngine::default();

        engine.index(
            MemoryType::Semantic,
            "cat",
            "A cat is a small furry animal",
            BTreeMap::new(),
            vec!["animal".into()],
        );

        let query = QueryBuilder::new().by_key("cat").build(1);

        let result = engine.execute(query);
        assert_eq!(result.matches.len(), 1);
    }

    #[test]
    fn test_full_text_search() {
        let mut engine = MemoryQueryEngine::default();

        engine.index(
            MemoryType::Semantic,
            "k1",
            "hello world",
            BTreeMap::new(),
            vec![],
        );
        engine.index(
            MemoryType::Semantic,
            "k2",
            "hello there",
            BTreeMap::new(),
            vec![],
        );
        engine.index(
            MemoryType::Semantic,
            "k3",
            "goodbye",
            BTreeMap::new(),
            vec![],
        );

        let query = QueryBuilder::new().full_text("hello").build(1);

        let result = engine.execute(query);
        assert_eq!(result.matches.len(), 2);
    }

    #[test]
    fn test_filters() {
        let mut engine = MemoryQueryEngine::default();

        let mut meta = BTreeMap::new();
        meta.insert("category".into(), "animal".into());
        engine.index(MemoryType::Semantic, "cat", "cat", meta.clone(), vec![]);

        meta.insert("category".into(), "plant".into());
        engine.index(MemoryType::Semantic, "tree", "tree", meta, vec![]);

        let query = QueryBuilder::new()
            .pattern("*")
            .where_eq("category", "animal")
            .build(1);

        let result = engine.execute(query);
        assert_eq!(result.matches.len(), 1);
    }

    #[test]
    fn test_limit_offset() {
        let mut engine = MemoryQueryEngine::default();

        for i in 0..10 {
            engine.index(
                MemoryType::Semantic,
                &format!("k{}", i),
                "content",
                BTreeMap::new(),
                vec![],
            );
        }

        let query = QueryBuilder::new().pattern("*").limit(3).offset(2).build(1);

        let result = engine.execute(query);
        assert_eq!(result.matches.len(), 3);
        assert_eq!(result.total_count, 10);
    }
}
