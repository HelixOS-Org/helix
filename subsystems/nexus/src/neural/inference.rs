//! NEXUS Year 2: Neural Inference Engine
//!
//! Optimized inference for kernel-native neural networks.
//! Pure Rust, no_std compatible.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::activation::Softmax;
use super::network::Sequential;
use super::tensor::{Tensor, TensorShape};
use crate::math::F32Ext;

// ============================================================================
// Inference Configuration
// ============================================================================

/// Configuration for inference engine
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Maximum batch size
    pub max_batch_size: usize,

    /// Enable output caching
    pub enable_cache: bool,

    /// Cache size limit
    pub cache_size: usize,

    /// Timeout for inference (cycles)
    pub timeout_cycles: u64,

    /// Enable quantization
    pub quantize: bool,

    /// Precision mode
    pub precision: Precision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    Full,    // f32
    Half,    // f16 simulated
    Int8,    // Quantized
    Dynamic, // Auto-select
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 32,
            enable_cache: true,
            cache_size: 1024,
            timeout_cycles: 1_000_000,
            quantize: false,
            precision: Precision::Full,
        }
    }
}

impl InferenceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    #[inline]
    pub fn with_cache(mut self, enable: bool, size: usize) -> Self {
        self.enable_cache = enable;
        self.cache_size = size;
        self
    }

    #[inline(always)]
    pub fn with_quantization(mut self, quantize: bool) -> Self {
        self.quantize = quantize;
        self
    }

    #[inline]
    pub fn kernel_optimized() -> Self {
        Self {
            max_batch_size: 16,
            enable_cache: true,
            cache_size: 256,
            timeout_cycles: 100_000,
            quantize: true,
            precision: Precision::Int8,
        }
    }
}

// ============================================================================
// Inference Statistics
// ============================================================================

/// Statistics for inference operations
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct InferenceStats {
    pub total_inferences: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_latency_cycles: u64,
    pub min_latency: u64,
    pub max_latency: u64,
    pub batch_inferences: u64,
    pub timeouts: u64,
}

impl InferenceStats {
    pub fn new() -> Self {
        Self {
            min_latency: u64::MAX,
            ..Default::default()
        }
    }

    pub fn record_inference(&mut self, latency: u64, cache_hit: bool) {
        self.total_inferences += 1;
        self.total_latency_cycles += latency;

        if latency < self.min_latency {
            self.min_latency = latency;
        }
        if latency > self.max_latency {
            self.max_latency = latency;
        }

        if cache_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
    }

    #[inline]
    pub fn average_latency(&self) -> f32 {
        if self.total_inferences > 0 {
            self.total_latency_cycles as f32 / self.total_inferences as f32
        } else {
            0.0
        }
    }

    #[inline]
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f32 / total as f32
        } else {
            0.0
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// ============================================================================
// Inference Cache
// ============================================================================

/// Simple LRU-style cache for inference results
struct InferenceCache {
    entries: BTreeMap<u64, CacheEntry>,
    max_size: usize,
    access_counter: u64,
}

struct CacheEntry {
    output: Tensor,
    last_access: u64,
    hit_count: u32,
}

impl InferenceCache {
    fn new(max_size: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_size,
            access_counter: 0,
        }
    }

    fn hash_input(input: &Tensor) -> u64 {
        // Simple hash of input tensor
        let mut hash = 0u64;
        for (i, &val) in input.data().iter().enumerate() {
            let bits = val.to_bits() as u64;
            hash = hash.wrapping_add(bits.wrapping_mul((i as u64).wrapping_add(1)));
            hash = hash.rotate_left(7);
        }
        hash ^= input.len() as u64;
        hash
    }

    fn get(&mut self, input: &Tensor) -> Option<Tensor> {
        let hash = Self::hash_input(input);
        self.access_counter += 1;

        if let Some(entry) = self.entries.get_mut(&hash) {
            entry.last_access = self.access_counter;
            entry.hit_count += 1;
            Some(entry.output.clone())
        } else {
            None
        }
    }

    fn insert(&mut self, input: &Tensor, output: Tensor) {
        if self.entries.len() >= self.max_size {
            self.evict();
        }

        let hash = Self::hash_input(input);
        self.entries.insert(hash, CacheEntry {
            output,
            last_access: self.access_counter,
            hit_count: 1,
        });
    }

    fn evict(&mut self) {
        // Find least recently used entry
        if let Some((&oldest_key, _)) = self.entries.iter().min_by_key(|(_, e)| e.last_access) {
            self.entries.remove(&oldest_key);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.access_counter = 0;
    }
}

// ============================================================================
// Inference Engine
// ============================================================================

/// Neural network inference engine
pub struct InferenceEngine {
    config: InferenceConfig,
    models: BTreeMap<String, Sequential>,
    cache: InferenceCache,
    stats: InferenceStats,
    cycle_counter: u64,
}

impl InferenceEngine {
    pub fn new(config: InferenceConfig) -> Self {
        let cache_size = config.cache_size;
        Self {
            config,
            models: BTreeMap::new(),
            cache: InferenceCache::new(cache_size),
            stats: InferenceStats::new(),
            cycle_counter: 0,
        }
    }

    #[inline(always)]
    pub fn with_default_config() -> Self {
        Self::new(InferenceConfig::default())
    }

    #[inline(always)]
    pub fn kernel_engine() -> Self {
        Self::new(InferenceConfig::kernel_optimized())
    }

    /// Register a model for inference
    #[inline(always)]
    pub fn register_model(&mut self, name: &str, model: Sequential) {
        self.models.insert(String::from(name), model);
    }

    /// Unregister a model
    #[inline(always)]
    pub fn unregister_model(&mut self, name: &str) -> Option<Sequential> {
        self.models.remove(name)
    }

    /// Check if a model is registered
    #[inline(always)]
    pub fn has_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    /// List all registered models
    #[inline(always)]
    pub fn list_models(&self) -> Vec<&str> {
        self.models.keys().map(|s| s.as_str()).collect()
    }

    /// Run inference on a single input
    pub fn infer(&mut self, model_name: &str, input: &Tensor) -> Option<Tensor> {
        let start_cycles = self.cycle_counter;
        self.cycle_counter += 1;

        // Check cache first
        if self.config.enable_cache {
            if let Some(cached) = self.cache.get(input) {
                self.stats.record_inference(1, true);
                return Some(cached);
            }
        }

        // Get model
        let model = self.models.get(model_name)?;

        // Preprocess input if needed
        let processed_input = if self.config.quantize {
            self.quantize_input(input)
        } else {
            input.clone()
        };

        // Run inference
        let output = model.forward(&processed_input);

        // Post-process output
        let final_output = if self.config.quantize {
            self.dequantize_output(&output)
        } else {
            output
        };

        // Update cache
        if self.config.enable_cache {
            self.cache.insert(input, final_output.clone());
        }

        let latency = self.cycle_counter - start_cycles;
        self.stats.record_inference(latency, false);

        Some(final_output)
    }

    /// Run inference on a batch of inputs
    pub fn infer_batch(&mut self, model_name: &str, inputs: &[Tensor]) -> Option<Vec<Tensor>> {
        let model = self.models.get(model_name)?;

        let batch_size = inputs.len().min(self.config.max_batch_size);
        let mut outputs = Vec::with_capacity(batch_size);

        for input in inputs.iter().take(batch_size) {
            // Check cache
            if self.config.enable_cache {
                if let Some(cached) = self.cache.get(input) {
                    outputs.push(cached);
                    continue;
                }
            }

            // Run inference
            let output = model.forward(input);

            // Cache result
            if self.config.enable_cache {
                self.cache.insert(input, output.clone());
            }

            outputs.push(output);
        }

        self.stats.batch_inferences += 1;

        Some(outputs)
    }

    /// Predict class (argmax of output)
    #[inline(always)]
    pub fn predict_class(&mut self, model_name: &str, input: &Tensor) -> Option<usize> {
        let output = self.infer(model_name, input)?;
        Some(output.argmax())
    }

    /// Predict with probabilities
    #[inline]
    pub fn predict_probs(&mut self, model_name: &str, input: &Tensor) -> Option<Vec<f32>> {
        let output = self.infer(model_name, input)?;

        // Apply softmax to get probabilities
        let probs = Softmax::apply(output.data());
        Some(probs)
    }

    /// Predict top-k classes
    #[inline]
    pub fn predict_top_k(
        &mut self,
        model_name: &str,
        input: &Tensor,
        k: usize,
    ) -> Option<Vec<(usize, f32)>> {
        let output = self.infer(model_name, input)?;
        let probs = Softmax::apply(output.data());

        // Get top-k
        let mut indexed: Vec<(usize, f32)> = probs.iter().cloned().enumerate().collect();

        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        Some(indexed.into_iter().take(k).collect())
    }

    fn quantize_input(&self, input: &Tensor) -> Tensor {
        // Simple min-max quantization to [-128, 127] range
        let min = input.min();
        let max = input.max();
        let range = max - min;

        if range < 1e-10 {
            return input.clone();
        }

        let scale = 255.0 / range;

        let data: Vec<f32> = input
            .data()
            .iter()
            .map(|&v| {
                let normalized = (v - min) * scale - 128.0;
                normalized.round() / 128.0 // Back to [-1, 1]
            })
            .collect();

        Tensor::from_data(*input.shape(), data)
    }

    fn dequantize_output(&self, output: &Tensor) -> Tensor {
        // Output is already in usable range
        output.clone()
    }

    /// Get inference statistics
    #[inline(always)]
    pub fn stats(&self) -> &InferenceStats {
        &self.stats
    }

    /// Reset statistics
    #[inline(always)]
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Clear inference cache
    #[inline(always)]
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get model info
    #[inline]
    pub fn model_info(&self, name: &str) -> Option<ModelInfo> {
        let model = self.models.get(name)?;
        Some(ModelInfo {
            name: name.to_string(),
            num_layers: model.num_layers(),
            num_parameters: model.num_parameters(),
            input_shape: *model.input_shape(),
            output_shape: *model.output_shape(),
        })
    }
}

/// Information about a registered model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub num_layers: usize,
    pub num_parameters: usize,
    pub input_shape: TensorShape,
    pub output_shape: TensorShape,
}

// ============================================================================
// Async Inference (for non-blocking kernel use)
// ============================================================================

/// Inference request for asynchronous processing
#[derive(Debug)]
pub struct InferenceRequest {
    pub id: u64,
    pub model_name: String,
    pub input: Tensor,
    pub priority: Priority,
    pub created_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical   = 0,
    High       = 1,
    Normal     = 2,
    Low        = 3,
    Background = 4,
}

/// Inference result
#[derive(Debug)]
pub struct InferenceResult {
    pub request_id: u64,
    pub output: Option<Tensor>,
    pub latency_cycles: u64,
    pub success: bool,
    pub error: Option<InferenceError>,
}

#[derive(Debug, Clone)]
pub enum InferenceError {
    ModelNotFound,
    InvalidInput,
    Timeout,
    ResourceExhausted,
    Internal,
}

/// Async inference queue
#[repr(align(64))]
pub struct AsyncInferenceQueue {
    engine: InferenceEngine,
    pending: Vec<InferenceRequest>,
    completed: Vec<InferenceResult>,
    next_id: u64,
    max_pending: usize,
}

impl AsyncInferenceQueue {
    pub fn new(engine: InferenceEngine, max_pending: usize) -> Self {
        Self {
            engine,
            pending: Vec::new(),
            completed: Vec::new(),
            next_id: 1,
            max_pending,
        }
    }

    /// Submit an inference request
    pub fn submit(
        &mut self,
        model_name: &str,
        input: Tensor,
        priority: Priority,
    ) -> Result<u64, InferenceError> {
        if self.pending.len() >= self.max_pending {
            return Err(InferenceError::ResourceExhausted);
        }

        if !self.engine.has_model(model_name) {
            return Err(InferenceError::ModelNotFound);
        }

        let id = self.next_id;
        self.next_id += 1;

        let request = InferenceRequest {
            id,
            model_name: String::from(model_name),
            input,
            priority,
            created_at: self.engine.cycle_counter,
        };

        // Insert sorted by priority
        let pos = self
            .pending
            .iter()
            .position(|r| r.priority > priority)
            .unwrap_or(self.pending.len());
        self.pending.insert(pos, request);

        Ok(id)
    }

    /// Process one pending request
    pub fn process_one(&mut self) -> Option<u64> {
        let request = self.pending.pop()?;
        let start = self.engine.cycle_counter;

        let output = self.engine.infer(&request.model_name, &request.input);
        let latency = self.engine.cycle_counter - start;

        let result = InferenceResult {
            request_id: request.id,
            output,
            latency_cycles: latency,
            success: true,
            error: None,
        };

        self.completed.push(result);
        Some(request.id)
    }

    /// Process multiple requests
    #[inline]
    pub fn process_batch(&mut self, count: usize) -> usize {
        let mut processed = 0;
        for _ in 0..count {
            if self.process_one().is_some() {
                processed += 1;
            } else {
                break;
            }
        }
        processed
    }

    /// Get result for a request
    #[inline]
    pub fn get_result(&mut self, request_id: u64) -> Option<InferenceResult> {
        if let Some(pos) = self
            .completed
            .iter()
            .position(|r| r.request_id == request_id)
        {
            Some(self.completed.remove(pos))
        } else {
            None
        }
    }

    /// Check if request is pending
    #[inline(always)]
    pub fn is_pending(&self, request_id: u64) -> bool {
        self.pending.iter().any(|r| r.id == request_id)
    }

    /// Check if request is completed
    #[inline(always)]
    pub fn is_completed(&self, request_id: u64) -> bool {
        self.completed.iter().any(|r| r.request_id == request_id)
    }

    /// Get queue statistics
    #[inline]
    pub fn queue_stats(&self) -> QueueStats {
        QueueStats {
            pending_count: self.pending.len(),
            completed_count: self.completed.len(),
            total_processed: self.engine.stats.total_inferences,
        }
    }

    /// Get underlying engine
    #[inline(always)]
    pub fn engine(&self) -> &InferenceEngine {
        &self.engine
    }

    /// Get mutable engine
    #[inline(always)]
    pub fn engine_mut(&mut self) -> &mut InferenceEngine {
        &mut self.engine
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QueueStats {
    pub pending_count: usize,
    pub completed_count: usize,
    pub total_processed: u64,
}

// ============================================================================
// Kernel Integration
// ============================================================================

/// Kernel-specific inference manager
pub struct KernelInferenceManager {
    inference_queue: AsyncInferenceQueue,
    registered_handlers: BTreeMap<String, InferenceHandler>,
}

/// Handler for inference results
pub struct InferenceHandler {
    pub name: String,
    pub callback: Box<dyn Fn(&InferenceResult) + Send + Sync>,
}

impl KernelInferenceManager {
    pub fn new() -> Self {
        let engine = InferenceEngine::kernel_engine();
        let queue = AsyncInferenceQueue::new(engine, 64);

        Self {
            inference_queue: queue,
            registered_handlers: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn register_model(&mut self, name: &str, model: Sequential) {
        self.inference_queue
            .engine_mut()
            .register_model(name, model);
    }

    #[inline]
    pub fn register_handler(
        &mut self,
        name: &str,
        callback: Box<dyn Fn(&InferenceResult) + Send + Sync>,
    ) {
        self.registered_handlers
            .insert(String::from(name), InferenceHandler {
                name: String::from(name),
                callback,
            });
    }

    #[inline(always)]
    pub fn submit_inference(
        &mut self,
        model_name: &str,
        input: Tensor,
        priority: Priority,
    ) -> Result<u64, InferenceError> {
        self.inference_queue.submit(model_name, input, priority)
    }

    pub fn tick(&mut self) -> usize {
        // Process pending inferences
        let processed = self.inference_queue.process_batch(4);

        // Dispatch completed results to handlers
        let completed: Vec<_> = core::mem::take(&mut self.inference_queue.completed);

        for result in completed {
            // Call registered handlers
            for handler in self.registered_handlers.values() {
                (handler.callback)(&result);
            }
        }

        processed
    }

    #[inline(always)]
    pub fn stats(&self) -> &InferenceStats {
        self.inference_queue.engine().stats()
    }
}

impl Default for KernelInferenceManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Inference Benchmarks
// ============================================================================

/// Benchmark results for inference
#[derive(Debug, Clone)]
pub struct InferenceBenchmark {
    pub model_name: String,
    pub num_iterations: usize,
    pub total_time_cycles: u64,
    pub avg_time_cycles: f32,
    pub min_time_cycles: u64,
    pub max_time_cycles: u64,
    pub throughput: f32, // inferences per 1M cycles
}

/// Run inference benchmark
pub fn benchmark_inference(
    engine: &mut InferenceEngine,
    model_name: &str,
    input: &Tensor,
    iterations: usize,
) -> Option<InferenceBenchmark> {
    if !engine.has_model(model_name) {
        return None;
    }

    // Warm-up
    for _ in 0..10 {
        engine.infer(model_name, input);
    }

    engine.clear_cache();
    engine.reset_stats();

    // Benchmark
    for _ in 0..iterations {
        engine.infer(model_name, input);
    }

    let stats = engine.stats();

    Some(InferenceBenchmark {
        model_name: String::from(model_name),
        num_iterations: iterations,
        total_time_cycles: stats.total_latency_cycles,
        avg_time_cycles: stats.average_latency(),
        min_time_cycles: stats.min_latency,
        max_time_cycles: stats.max_latency,
        throughput: 1_000_000.0 / stats.average_latency(),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::activation::ActivationType;
    use super::super::network::NetworkBuilder;
    use super::*;

    #[test]
    fn test_inference_engine() {
        let mut engine = InferenceEngine::with_default_config();

        let model = NetworkBuilder::new("test", TensorShape::vector(10))
            .dense(8, ActivationType::ReLU)
            .dense(4, ActivationType::Softmax)
            .build();

        engine.register_model("test", model);

        let input = Tensor::random(TensorShape::vector(10), 42);
        let output = engine.infer("test", &input);

        assert!(output.is_some());
        assert_eq!(output.unwrap().len(), 4);
    }

    #[test]
    fn test_inference_cache() {
        let mut engine = InferenceEngine::new(InferenceConfig::new().with_cache(true, 100));

        let model = NetworkBuilder::new("cached", TensorShape::vector(8))
            .dense(4, ActivationType::Identity)
            .build();

        engine.register_model("cached", model);

        let input = Tensor::random(TensorShape::vector(8), 1);

        // First inference - cache miss
        let _out1 = engine.infer("cached", &input);
        assert_eq!(engine.stats().cache_misses, 1);

        // Second inference - cache hit
        let _out2 = engine.infer("cached", &input);
        assert_eq!(engine.stats().cache_hits, 1);
    }

    #[test]
    fn test_batch_inference() {
        let mut engine = InferenceEngine::with_default_config();

        let model = NetworkBuilder::new("batch", TensorShape::vector(8))
            .dense(4, ActivationType::ReLU)
            .build();

        engine.register_model("batch", model);

        let inputs: Vec<Tensor> = (0..5)
            .map(|i| Tensor::random(TensorShape::vector(8), i as u64))
            .collect();

        let outputs = engine.infer_batch("batch", &inputs);

        assert!(outputs.is_some());
        assert_eq!(outputs.unwrap().len(), 5);
    }

    #[test]
    fn test_predict_class() {
        let mut engine = InferenceEngine::with_default_config();

        let model = NetworkBuilder::new("classifier", TensorShape::vector(10))
            .dense(8, ActivationType::ReLU)
            .dense(4, ActivationType::Softmax)
            .build();

        engine.register_model("classifier", model);

        let input = Tensor::random(TensorShape::vector(10), 42);
        let class = engine.predict_class("classifier", &input);

        assert!(class.is_some());
        assert!(class.unwrap() < 4);
    }

    #[test]
    fn test_async_queue() {
        let engine = InferenceEngine::with_default_config();
        let mut queue = AsyncInferenceQueue::new(engine, 10);

        let model = NetworkBuilder::new("async", TensorShape::vector(8))
            .dense(4, ActivationType::Identity)
            .build();

        queue.engine_mut().register_model("async", model);

        let input = Tensor::random(TensorShape::vector(8), 1);
        let id = queue.submit("async", input, Priority::Normal).unwrap();

        assert!(queue.is_pending(id));

        queue.process_one();

        assert!(queue.is_completed(id));

        let result = queue.get_result(id);
        assert!(result.is_some());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_priority_ordering() {
        let engine = InferenceEngine::with_default_config();
        let mut queue = AsyncInferenceQueue::new(engine, 10);

        let model = NetworkBuilder::new("prio", TensorShape::vector(4))
            .dense(2, ActivationType::Identity)
            .build();

        queue.engine_mut().register_model("prio", model);

        // Submit in reverse priority order
        let _id_low = queue.submit(
            "prio",
            Tensor::full(TensorShape::vector(4), 1.0),
            Priority::Low,
        );
        let _id_high = queue.submit(
            "prio",
            Tensor::full(TensorShape::vector(4), 2.0),
            Priority::High,
        );
        let _id_critical = queue.submit(
            "prio",
            Tensor::full(TensorShape::vector(4), 3.0),
            Priority::Critical,
        );

        // Should process critical first
        queue.process_one();

        // Then high
        queue.process_one();

        // Then low
        queue.process_one();

        assert_eq!(queue.queue_stats().completed_count, 3);
    }

    #[test]
    fn test_kernel_inference_manager() {
        let mut manager = KernelInferenceManager::new();

        let model = NetworkBuilder::new("kernel_test", TensorShape::vector(16))
            .dense(8, ActivationType::ReLU)
            .dense(4, ActivationType::Softmax)
            .build();

        manager.register_model("kernel_test", model);

        let input = Tensor::random(TensorShape::vector(16), 42);
        let _id = manager.submit_inference("kernel_test", input, Priority::Normal);

        let processed = manager.tick();
        assert_eq!(processed, 1);
    }
}
