//! # Cognitive Pipeline
//!
//! Defines processing pipelines for cognitive data.
//! Supports staged processing with transformations.

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// PIPELINE TYPES
// ============================================================================

/// Pipeline stage result
#[derive(Debug, Clone)]
pub enum StageResult {
    /// Continue to next stage
    Continue(PipelineData),
    /// Skip remaining stages
    Skip,
    /// Stop with error
    Error(String),
    /// Fork into multiple items
    Fork(Vec<PipelineData>),
}

/// Data flowing through pipeline
#[derive(Debug, Clone)]
pub struct PipelineData {
    /// Data ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Data payload
    pub payload: DataPayload,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Timestamps
    pub timestamps: Vec<(String, Timestamp)>,
}

/// Data payload types
#[derive(Debug, Clone)]
pub enum DataPayload {
    /// Empty
    Empty,
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Numeric value
    Numeric(f64),
    /// Text
    Text(String),
    /// Key-value map
    Map(BTreeMap<String, DataPayload>),
    /// Array
    Array(Vec<DataPayload>),
    /// Signal
    Signal(SignalPayload),
    /// Pattern
    Pattern(PatternPayload),
    /// Causal
    Causal(CausalPayload),
}

/// Signal payload
#[derive(Debug, Clone)]
pub struct SignalPayload {
    pub signal_type: u32,
    pub value: f64,
    pub component_id: u64,
    pub severity: u8,
}

/// Pattern payload
#[derive(Debug, Clone)]
pub struct PatternPayload {
    pub pattern_type: u32,
    pub confidence: f32,
    pub components: Vec<u64>,
}

/// Causal payload
#[derive(Debug, Clone)]
pub struct CausalPayload {
    pub cause_id: u64,
    pub effect_id: u64,
    pub strength: f32,
}

/// Pipeline stage
pub trait PipelineStage: Send + Sync {
    /// Stage name
    fn name(&self) -> &str;

    /// Process data
    fn process(&mut self, data: PipelineData) -> StageResult;

    /// Check if stage should be skipped
    fn should_skip(&self, _data: &PipelineData) -> bool {
        false
    }
}

// ============================================================================
// BUILT-IN STAGES
// ============================================================================

/// Filter stage
pub struct FilterStage {
    name: String,
    predicate: Box<dyn Fn(&PipelineData) -> bool + Send + Sync>,
}

impl FilterStage {
    pub fn new<F>(name: &str, predicate: F) -> Self
    where
        F: Fn(&PipelineData) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            predicate: Box::new(predicate),
        }
    }
}

impl PipelineStage for FilterStage {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&mut self, data: PipelineData) -> StageResult {
        if (self.predicate)(&data) {
            StageResult::Continue(data)
        } else {
            StageResult::Skip
        }
    }
}

/// Transform stage
pub struct TransformStage {
    name: String,
    transform: Box<dyn Fn(PipelineData) -> PipelineData + Send + Sync>,
}

impl TransformStage {
    pub fn new<F>(name: &str, transform: F) -> Self
    where
        F: Fn(PipelineData) -> PipelineData + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            transform: Box::new(transform),
        }
    }
}

impl PipelineStage for TransformStage {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&mut self, data: PipelineData) -> StageResult {
        StageResult::Continue((self.transform)(data))
    }
}

/// Aggregate stage
pub struct AggregateStage {
    name: String,
    buffer: Vec<PipelineData>,
    threshold: usize,
    aggregator: Box<dyn Fn(Vec<PipelineData>) -> PipelineData + Send + Sync>,
}

impl AggregateStage {
    pub fn new<F>(name: &str, threshold: usize, aggregator: F) -> Self
    where
        F: Fn(Vec<PipelineData>) -> PipelineData + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            buffer: Vec::new(),
            threshold,
            aggregator: Box::new(aggregator),
        }
    }

    /// Force flush buffer
    pub fn flush(&mut self) -> Option<PipelineData> {
        if self.buffer.is_empty() {
            None
        } else {
            let items = core::mem::take(&mut self.buffer);
            Some((self.aggregator)(items))
        }
    }
}

impl PipelineStage for AggregateStage {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&mut self, data: PipelineData) -> StageResult {
        self.buffer.push(data);

        if self.buffer.len() >= self.threshold {
            let items = core::mem::take(&mut self.buffer);
            StageResult::Continue((self.aggregator)(items))
        } else {
            StageResult::Skip
        }
    }
}

/// Branch stage - routes to different pipelines
pub struct BranchStage {
    name: String,
    router: Box<dyn Fn(&PipelineData) -> usize + Send + Sync>,
}

impl BranchStage {
    pub fn new<F>(name: &str, router: F) -> Self
    where
        F: Fn(&PipelineData) -> usize + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            router: Box::new(router),
        }
    }
}

impl PipelineStage for BranchStage {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&mut self, data: PipelineData) -> StageResult {
        let _branch = (self.router)(&data);
        // In a full implementation, this would route to different sub-pipelines
        StageResult::Continue(data)
    }
}

// ============================================================================
// PIPELINE
// ============================================================================

/// A processing pipeline
pub struct Pipeline {
    /// Pipeline ID
    id: u64,
    /// Pipeline name
    name: String,
    /// Stages
    stages: Vec<Box<dyn PipelineStage>>,
    /// Statistics
    stats: PipelineStats,
    /// Enabled
    enabled: bool,
}

/// Pipeline statistics
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Items processed
    pub items_processed: u64,
    /// Items filtered
    pub items_filtered: u64,
    /// Items errored
    pub items_errored: u64,
    /// Total processing time (ns)
    pub total_time_ns: u64,
    /// Average processing time (ns)
    pub avg_time_ns: u64,
    /// Stage timings
    pub stage_times: BTreeMap<String, u64>,
}

impl Pipeline {
    /// Create a new pipeline
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            stages: Vec::new(),
            stats: PipelineStats::default(),
            enabled: true,
        }
    }

    /// Add a stage
    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    /// Process data through pipeline
    pub fn process(&mut self, mut data: PipelineData) -> Vec<PipelineData> {
        if !self.enabled {
            return vec![data];
        }

        let start = Timestamp::now();
        let mut results = Vec::new();
        let mut pending = vec![data];

        while let Some(item) = pending.pop() {
            let mut current = item;
            let mut skip = false;

            for stage in &mut self.stages {
                if stage.should_skip(&current) {
                    continue;
                }

                let stage_start = Timestamp::now();
                let result = stage.process(current);
                let stage_time = Timestamp::now().elapsed_since(stage_start);

                *self
                    .stats
                    .stage_times
                    .entry(stage.name().into())
                    .or_default() += stage_time;

                match result {
                    StageResult::Continue(d) => {
                        current = d;
                    },
                    StageResult::Skip => {
                        skip = true;
                        self.stats.items_filtered += 1;
                        break;
                    },
                    StageResult::Error(_e) => {
                        self.stats.items_errored += 1;
                        skip = true;
                        break;
                    },
                    StageResult::Fork(items) => {
                        // Add forked items to pending
                        for item in items {
                            pending.push(item);
                        }
                        skip = true;
                        break;
                    },
                }
            }

            if !skip {
                results.push(current);
            }
        }

        self.stats.items_processed += 1;
        self.stats.total_time_ns += Timestamp::now().elapsed_since(start);
        if self.stats.items_processed > 0 {
            self.stats.avg_time_ns = self.stats.total_time_ns / self.stats.items_processed;
        }

        results
    }

    /// Get pipeline ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get pipeline name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get stage count
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Get statistics
    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }

    /// Enable/disable pipeline
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = PipelineStats::default();
    }
}

// ============================================================================
// PIPELINE MANAGER
// ============================================================================

/// Manages multiple pipelines
pub struct PipelineManager {
    /// Pipelines
    pipelines: BTreeMap<u64, Pipeline>,
    /// Next pipeline ID
    next_id: AtomicU64,
    /// Pipeline routing
    routes: BTreeMap<String, u64>,
}

impl PipelineManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            pipelines: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            routes: BTreeMap::new(),
        }
    }

    /// Create a new pipeline
    pub fn create_pipeline(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let pipeline = Pipeline::new(id, name);
        self.pipelines.insert(id, pipeline);
        id
    }

    /// Get pipeline
    pub fn get(&self, id: u64) -> Option<&Pipeline> {
        self.pipelines.get(&id)
    }

    /// Get mutable pipeline
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Pipeline> {
        self.pipelines.get_mut(&id)
    }

    /// Add stage to pipeline
    pub fn add_stage(&mut self, pipeline_id: u64, stage: Box<dyn PipelineStage>) -> bool {
        if let Some(pipeline) = self.pipelines.get_mut(&pipeline_id) {
            pipeline.add_stage(stage);
            true
        } else {
            false
        }
    }

    /// Route a topic to a pipeline
    pub fn route(&mut self, topic: &str, pipeline_id: u64) {
        self.routes.insert(topic.into(), pipeline_id);
    }

    /// Get pipeline for topic
    pub fn get_route(&self, topic: &str) -> Option<u64> {
        self.routes.get(topic).copied()
    }

    /// Process data through appropriate pipeline
    pub fn process(&mut self, topic: &str, data: PipelineData) -> Vec<PipelineData> {
        if let Some(&pipeline_id) = self.routes.get(topic) {
            if let Some(pipeline) = self.pipelines.get_mut(&pipeline_id) {
                return pipeline.process(data);
            }
        }
        vec![data]
    }

    /// Get all pipeline stats
    pub fn all_stats(&self) -> BTreeMap<u64, &PipelineStats> {
        self.pipelines
            .iter()
            .map(|(id, p)| (*id, p.stats()))
            .collect()
    }

    /// Remove pipeline
    pub fn remove(&mut self, id: u64) -> bool {
        self.pipelines.remove(&id).is_some()
    }
}

impl Default for PipelineManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_stage() {
        let mut stage = FilterStage::new(
            "test_filter",
            |data| matches!(&data.payload, DataPayload::Numeric(v) if *v > 0.0),
        );

        let positive = PipelineData {
            id: 1,
            source: DomainId::new(1),
            payload: DataPayload::Numeric(10.0),
            metadata: BTreeMap::new(),
            timestamps: Vec::new(),
        };

        let negative = PipelineData {
            id: 2,
            source: DomainId::new(1),
            payload: DataPayload::Numeric(-10.0),
            metadata: BTreeMap::new(),
            timestamps: Vec::new(),
        };

        assert!(matches!(stage.process(positive), StageResult::Continue(_)));
        assert!(matches!(stage.process(negative), StageResult::Skip));
    }

    #[test]
    fn test_transform_stage() {
        let mut stage = TransformStage::new("double", |mut data| {
            if let DataPayload::Numeric(ref mut v) = data.payload {
                *v *= 2.0;
            }
            data
        });

        let data = PipelineData {
            id: 1,
            source: DomainId::new(1),
            payload: DataPayload::Numeric(5.0),
            metadata: BTreeMap::new(),
            timestamps: Vec::new(),
        };

        let result = stage.process(data);
        if let StageResult::Continue(d) = result {
            if let DataPayload::Numeric(v) = d.payload {
                assert_eq!(v, 10.0);
            }
        }
    }

    #[test]
    fn test_pipeline() {
        let mut pipeline = Pipeline::new(1, "test");

        pipeline.add_stage(Box::new(FilterStage::new(
            "positive_only",
            |data| matches!(&data.payload, DataPayload::Numeric(v) if *v > 0.0),
        )));

        pipeline.add_stage(Box::new(TransformStage::new("double", |mut data| {
            if let DataPayload::Numeric(ref mut v) = data.payload {
                *v *= 2.0;
            }
            data
        })));

        let data = PipelineData {
            id: 1,
            source: DomainId::new(1),
            payload: DataPayload::Numeric(5.0),
            metadata: BTreeMap::new(),
            timestamps: Vec::new(),
        };

        let results = pipeline.process(data);
        assert_eq!(results.len(), 1);

        if let DataPayload::Numeric(v) = &results[0].payload {
            assert_eq!(*v, 10.0);
        }
    }

    #[test]
    fn test_pipeline_manager() {
        let mut manager = PipelineManager::new();

        let id = manager.create_pipeline("signals");
        manager.route("signals", id);

        let data = PipelineData {
            id: 1,
            source: DomainId::new(1),
            payload: DataPayload::Text("test".into()),
            metadata: BTreeMap::new(),
            timestamps: Vec::new(),
        };

        let results = manager.process("signals", data);
        assert_eq!(results.len(), 1);
    }
}
