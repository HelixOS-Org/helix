//! Predictive Compilation System
//!
//! AI-powered predictive compilation that pre-compiles shader variants
//! before they're needed, eliminating runtime hitches.
//!
//! # Features
//!
//! - **Usage Prediction**: Predict which shaders will be needed
//! - **Background Compilation**: Compile variants in background
//! - **Priority Scheduling**: Compile most-likely-needed first
//! - **Variant Pruning**: Remove unused variants to save memory
//! - **Warm Cache**: Keep frequently-used shaders hot

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Shader Variant Types
// ============================================================================

/// Shader variant key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariantKey {
    /// Shader ID
    pub shader_id: u64,
    /// Define flags
    pub defines: u64,
    /// Specialization constants
    pub spec_constants: Vec<(u32, u32)>,
}

impl VariantKey {
    /// Create new key
    pub fn new(shader_id: u64) -> Self {
        Self {
            shader_id,
            defines: 0,
            spec_constants: Vec::new(),
        }
    }

    /// With defines
    pub fn with_defines(mut self, defines: u64) -> Self {
        self.defines = defines;
        self
    }

    /// With specialization constant
    pub fn with_spec_constant(mut self, id: u32, value: u32) -> Self {
        self.spec_constants.push((id, value));
        self
    }
}

/// Compiled variant
#[derive(Debug, Clone)]
pub struct CompiledVariant {
    /// Variant key
    pub key: VariantKey,
    /// SPIR-V binary
    pub spirv: Vec<u8>,
    /// Compilation time in microseconds
    pub compile_time_us: u64,
    /// Binary size
    pub size: u32,
    /// Last access timestamp
    pub last_access: u64,
    /// Access count
    pub access_count: u64,
    /// Target architectures
    pub targets: Vec<CompileTarget>,
}

/// Compile target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompileTarget {
    /// Generic SPIR-V
    SpirV,
    /// Vulkan SPIR-V
    Vulkan,
    /// Metal IR
    Metal,
    /// DirectX DXIL
    Dxil,
    /// AMD GCN
    AmdGcn,
    /// NVIDIA PTX
    NvidiaPtx,
    /// Intel GPU
    IntelGpu,
    /// Apple GPU
    AppleGpu,
}

// ============================================================================
// Usage Prediction
// ============================================================================

/// Usage pattern
#[derive(Debug, Clone)]
pub struct UsagePattern {
    /// Shader ID
    pub shader_id: u64,
    /// Variant key
    pub variant_key: VariantKey,
    /// Usage probability (0.0 - 1.0)
    pub probability: f32,
    /// Predicted time until use (frames)
    pub time_until_use: u32,
    /// Confidence in prediction
    pub confidence: f32,
    /// Pattern source
    pub source: PatternSource,
}

/// Pattern source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternSource {
    /// Historical usage data
    Historical,
    /// Scene analysis
    SceneAnalysis,
    /// User behavior
    UserBehavior,
    /// Level loading hints
    LevelHint,
    /// Explicit prefetch request
    Explicit,
}

/// Scene context for prediction
#[derive(Debug, Clone)]
pub struct SceneContext {
    /// Current scene ID
    pub scene_id: u64,
    /// Active materials
    pub active_materials: Vec<u64>,
    /// Visible objects
    pub visible_objects: u32,
    /// Camera position (for LOD prediction)
    pub camera_position: [f32; 3],
    /// Camera direction
    pub camera_direction: [f32; 3],
    /// Time of day (affects lighting shaders)
    pub time_of_day: f32,
    /// Weather state
    pub weather: WeatherState,
}

/// Weather state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeatherState {
    #[default]
    Clear,
    Cloudy,
    Rain,
    Snow,
    Fog,
    Storm,
}

/// Usage history entry
#[derive(Debug, Clone)]
pub struct UsageHistoryEntry {
    /// Variant key
    pub variant_key: VariantKey,
    /// Frame used
    pub frame: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Scene context hash
    pub scene_hash: u64,
    /// Duration of use (frames)
    pub duration: u32,
}

// ============================================================================
// Compilation Queue
// ============================================================================

/// Compilation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompilePriority {
    /// Low priority (might be needed later)
    Low       = 0,
    /// Normal priority
    Normal    = 1,
    /// High priority (likely needed soon)
    High      = 2,
    /// Critical (needed now or very soon)
    Critical  = 3,
    /// Immediate (blocking)
    Immediate = 4,
}

/// Compilation request
#[derive(Debug, Clone)]
pub struct CompileRequest {
    /// Variant key
    pub variant_key: VariantKey,
    /// Priority
    pub priority: CompilePriority,
    /// Target architectures
    pub targets: Vec<CompileTarget>,
    /// Deadline (timestamp)
    pub deadline: Option<u64>,
    /// Callback ID
    pub callback_id: Option<u64>,
}

/// Compilation result
#[derive(Debug, Clone)]
pub struct CompileResult {
    /// Variant key
    pub variant_key: VariantKey,
    /// Success
    pub success: bool,
    /// Compiled variant (if success)
    pub variant: Option<CompiledVariant>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Compilation time in microseconds
    pub compile_time_us: u64,
}

/// Compilation statistics
#[derive(Debug, Clone, Default)]
pub struct CompilationStats {
    /// Total compilations
    pub total_compilations: u64,
    /// Successful compilations
    pub successful: u64,
    /// Failed compilations
    pub failed: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average compile time (microseconds)
    pub avg_compile_time_us: u64,
    /// Predictions made
    pub predictions_made: u64,
    /// Correct predictions
    pub predictions_correct: u64,
    /// Wasted compilations (compiled but never used)
    pub wasted_compilations: u64,
}

impl CompilationStats {
    /// Get prediction accuracy
    pub fn prediction_accuracy(&self) -> f32 {
        if self.predictions_made == 0 {
            0.0
        } else {
            self.predictions_correct as f32 / self.predictions_made as f32
        }
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }
}

// ============================================================================
// Variant Cache
// ============================================================================

/// Cache eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    /// Least Recently Used
    Lru,
    /// Least Frequently Used
    Lfu,
    /// Adaptive Replacement Cache
    Arc,
    /// Time-aware LRU
    TimeLru,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum cache size in bytes
    pub max_size: u64,
    /// Maximum variant count
    pub max_variants: u32,
    /// Eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Minimum access count to keep
    pub min_access_count: u32,
    /// Maximum age before eviction (seconds)
    pub max_age_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 512 * 1024 * 1024, // 512 MB
            max_variants: 10000,
            eviction_policy: EvictionPolicy::Arc,
            min_access_count: 2,
            max_age_secs: 3600, // 1 hour
        }
    }
}

/// Variant cache
pub struct VariantCache {
    /// Configuration
    config: CacheConfig,
    /// Cached variants
    variants: BTreeMap<VariantKey, CompiledVariant>,
    /// Current size
    current_size: u64,
    /// Access order (for LRU)
    access_order: Vec<VariantKey>,
}

impl VariantCache {
    /// Create new cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            variants: BTreeMap::new(),
            current_size: 0,
            access_order: Vec::new(),
        }
    }

    /// Get variant
    pub fn get(&mut self, key: &VariantKey, timestamp: u64) -> Option<&CompiledVariant> {
        if let Some(variant) = self.variants.get_mut(key) {
            variant.last_access = timestamp;
            variant.access_count += 1;

            // Update access order
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());

            Some(variant)
        } else {
            None
        }
    }

    /// Insert variant
    pub fn insert(&mut self, variant: CompiledVariant) {
        let size = variant.size as u64;

        // Evict if necessary
        while self.current_size + size > self.config.max_size
            || self.variants.len() >= self.config.max_variants as usize
        {
            if !self.evict_one() {
                break;
            }
        }

        self.current_size += size;
        self.access_order.push(variant.key.clone());
        self.variants.insert(variant.key.clone(), variant);
    }

    /// Evict one variant
    fn evict_one(&mut self) -> bool {
        match self.config.eviction_policy {
            EvictionPolicy::Lru => self.evict_lru(),
            EvictionPolicy::Lfu => self.evict_lfu(),
            EvictionPolicy::Arc | EvictionPolicy::TimeLru => self.evict_lru(),
        }
    }

    fn evict_lru(&mut self) -> bool {
        if let Some(key) = self.access_order.first().cloned() {
            if let Some(variant) = self.variants.remove(&key) {
                self.current_size -= variant.size as u64;
                self.access_order.remove(0);
                return true;
            }
        }
        false
    }

    fn evict_lfu(&mut self) -> bool {
        let key = self
            .variants
            .iter()
            .min_by_key(|(_, v)| v.access_count)
            .map(|(k, _)| k.clone());

        if let Some(key) = key {
            if let Some(variant) = self.variants.remove(&key) {
                self.current_size -= variant.size as u64;
                self.access_order.retain(|k| k != &key);
                return true;
            }
        }
        false
    }

    /// Contains variant
    pub fn contains(&self, key: &VariantKey) -> bool {
        self.variants.contains_key(key)
    }

    /// Get cache size
    pub fn size(&self) -> u64 {
        self.current_size
    }

    /// Get variant count
    pub fn count(&self) -> usize {
        self.variants.len()
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.variants.clear();
        self.access_order.clear();
        self.current_size = 0;
    }
}

// ============================================================================
// Predictive Compiler
// ============================================================================

/// Predictive compiler configuration
#[derive(Debug, Clone)]
pub struct PredictiveConfig {
    /// Enable prediction
    pub enabled: bool,
    /// Look-ahead frames
    pub look_ahead_frames: u32,
    /// Maximum concurrent compilations
    pub max_concurrent: u32,
    /// Minimum probability to compile
    pub min_probability: f32,
    /// Cache configuration
    pub cache_config: CacheConfig,
    /// Background thread priority
    pub background_priority: bool,
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            look_ahead_frames: 60,
            max_concurrent: 4,
            min_probability: 0.5,
            cache_config: CacheConfig::default(),
            background_priority: true,
        }
    }
}

/// Predictive compiler
pub struct PredictiveCompiler {
    /// Configuration
    config: PredictiveConfig,
    /// Variant cache
    cache: VariantCache,
    /// Usage history
    history: Vec<UsageHistoryEntry>,
    /// Pending requests
    pending: Vec<CompileRequest>,
    /// In-progress compilations
    in_progress: Vec<VariantKey>,
    /// Statistics
    stats: CompilationStats,
    /// Current frame
    frame: u64,
    /// Current timestamp
    timestamp: u64,
}

impl PredictiveCompiler {
    /// Create new compiler
    pub fn new(config: PredictiveConfig) -> Self {
        let cache = VariantCache::new(config.cache_config.clone());
        Self {
            config,
            cache,
            history: Vec::new(),
            pending: Vec::new(),
            in_progress: Vec::new(),
            stats: CompilationStats::default(),
            frame: 0,
            timestamp: 0,
        }
    }

    /// Update frame
    pub fn update(&mut self, frame: u64, timestamp: u64) {
        self.frame = frame;
        self.timestamp = timestamp;
    }

    /// Get or compile variant
    pub fn get_variant(&mut self, key: &VariantKey) -> Option<&CompiledVariant> {
        // Check cache
        if let Some(variant) = self.cache.get(key, self.timestamp) {
            self.stats.cache_hits += 1;
            return Some(variant);
        }

        self.stats.cache_misses += 1;

        // Check if already in progress
        if self.in_progress.contains(key) {
            return None;
        }

        // Check pending
        if self.pending.iter().any(|r| &r.variant_key == key) {
            // Boost priority
            for req in &mut self.pending {
                if &req.variant_key == key {
                    req.priority = CompilePriority::Immediate;
                }
            }
            return None;
        }

        // Request compilation
        self.request_compile(CompileRequest {
            variant_key: key.clone(),
            priority: CompilePriority::Immediate,
            targets: vec![CompileTarget::SpirV],
            deadline: Some(self.timestamp),
            callback_id: None,
        });

        None
    }

    /// Request compilation
    pub fn request_compile(&mut self, request: CompileRequest) {
        // Skip if already cached
        if self.cache.contains(&request.variant_key) {
            return;
        }

        // Skip if already pending or in progress
        if self
            .pending
            .iter()
            .any(|r| r.variant_key == request.variant_key)
        {
            return;
        }
        if self.in_progress.contains(&request.variant_key) {
            return;
        }

        // Insert sorted by priority
        let pos = self
            .pending
            .iter()
            .position(|r| r.priority < request.priority)
            .unwrap_or(self.pending.len());
        self.pending.insert(pos, request);
    }

    /// Predict and pre-compile based on context
    pub fn predict(&mut self, context: &SceneContext) {
        if !self.config.enabled {
            return;
        }

        let predictions = self.analyze_context(context);
        self.stats.predictions_made += predictions.len() as u64;

        for pattern in predictions {
            if pattern.probability >= self.config.min_probability {
                let priority = if pattern.time_until_use < 10 {
                    CompilePriority::Critical
                } else if pattern.time_until_use < 30 {
                    CompilePriority::High
                } else {
                    CompilePriority::Normal
                };

                self.request_compile(CompileRequest {
                    variant_key: pattern.variant_key,
                    priority,
                    targets: vec![CompileTarget::SpirV],
                    deadline: Some(self.timestamp + pattern.time_until_use as u64 * 16666), // ~60fps
                    callback_id: None,
                });
            }
        }
    }

    fn analyze_context(&self, context: &SceneContext) -> Vec<UsagePattern> {
        // Analyze history and context to predict needed variants
        let mut predictions = Vec::new();

        // Weather-based predictions
        match context.weather {
            WeatherState::Rain => {
                predictions.push(UsagePattern {
                    shader_id: 1, // Rain shader
                    variant_key: VariantKey::new(1).with_defines(0x1),
                    probability: 0.9,
                    time_until_use: 5,
                    confidence: 0.8,
                    source: PatternSource::SceneAnalysis,
                });
            },
            WeatherState::Snow => {
                predictions.push(UsagePattern {
                    shader_id: 2, // Snow shader
                    variant_key: VariantKey::new(2).with_defines(0x2),
                    probability: 0.9,
                    time_until_use: 5,
                    confidence: 0.8,
                    source: PatternSource::SceneAnalysis,
                });
            },
            _ => {},
        }

        // Material-based predictions
        for &material_id in &context.active_materials {
            predictions.push(UsagePattern {
                shader_id: material_id,
                variant_key: VariantKey::new(material_id),
                probability: 0.7,
                time_until_use: 1,
                confidence: 0.6,
                source: PatternSource::SceneAnalysis,
            });
        }

        predictions
    }

    /// Process compilation queue
    pub fn process(&mut self) -> Vec<CompileResult> {
        let mut results = Vec::new();
        let available_slots = self.config.max_concurrent as usize - self.in_progress.len();

        for _ in 0..available_slots {
            if let Some(request) = self.pending.pop() {
                // Simulate compilation (in real implementation, this would be async)
                let result = self.compile_variant(&request);

                if result.success {
                    self.stats.successful += 1;
                    if let Some(variant) = result.variant.clone() {
                        self.cache.insert(variant);
                    }
                } else {
                    self.stats.failed += 1;
                }

                self.stats.total_compilations += 1;
                results.push(result);
            }
        }

        results
    }

    fn compile_variant(&self, request: &CompileRequest) -> CompileResult {
        // In real implementation, this would compile the shader
        let compile_time_us = 1000; // Simulated

        CompileResult {
            variant_key: request.variant_key.clone(),
            success: true,
            variant: Some(CompiledVariant {
                key: request.variant_key.clone(),
                spirv: Vec::new(),
                compile_time_us,
                size: 1024,
                last_access: self.timestamp,
                access_count: 0,
                targets: request.targets.clone(),
            }),
            error: None,
            compile_time_us,
        }
    }

    /// Record usage
    pub fn record_usage(&mut self, key: &VariantKey) {
        self.history.push(UsageHistoryEntry {
            variant_key: key.clone(),
            frame: self.frame,
            timestamp: self.timestamp,
            scene_hash: 0, // Would be calculated from scene context
            duration: 1,
        });

        // Trim history
        const MAX_HISTORY: usize = 10000;
        while self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &CompilationStats {
        &self.stats
    }

    /// Get cache
    pub fn cache(&self) -> &VariantCache {
        &self.cache
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for PredictiveCompiler {
    fn default() -> Self {
        Self::new(PredictiveConfig::default())
    }
}

// ============================================================================
// Warm Cache System
// ============================================================================

/// Warm cache entry
#[derive(Debug, Clone)]
pub struct WarmCacheEntry {
    /// Variant key
    pub key: VariantKey,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
    /// Last used timestamp
    pub last_used: u64,
    /// Usage frequency
    pub frequency: f32,
}

/// Warm cache for frequently used shaders
pub struct WarmCache {
    /// Maximum entries
    max_entries: u32,
    /// Entries
    entries: Vec<WarmCacheEntry>,
}

impl WarmCache {
    /// Create new warm cache
    pub fn new(max_entries: u32) -> Self {
        Self {
            max_entries,
            entries: Vec::new(),
        }
    }

    /// Add entry
    pub fn add(&mut self, key: VariantKey, importance: f32, timestamp: u64) {
        // Update existing
        for entry in &mut self.entries {
            if entry.key == key {
                entry.importance = importance.max(entry.importance);
                entry.last_used = timestamp;
                entry.frequency += 1.0;
                return;
            }
        }

        // Add new
        if self.entries.len() >= self.max_entries as usize {
            // Remove least important
            self.entries.sort_by(|a, b| {
                b.importance
                    .partial_cmp(&a.importance)
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
            self.entries.pop();
        }

        self.entries.push(WarmCacheEntry {
            key,
            importance,
            last_used: timestamp,
            frequency: 1.0,
        });
    }

    /// Get entries for pre-warming
    pub fn get_entries(&self) -> &[WarmCacheEntry] {
        &self.entries
    }

    /// Decay frequencies
    pub fn decay(&mut self, factor: f32) {
        for entry in &mut self.entries {
            entry.frequency *= factor;
        }
        // Remove entries with very low frequency
        self.entries.retain(|e| e.frequency > 0.01);
    }
}
