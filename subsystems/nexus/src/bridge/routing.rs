//! # Syscall Routing Engine
//!
//! Intelligent routing of syscalls to optimal handlers:
//! - Multi-path routing (fast path, slow path, async path)
//! - Handler specialization per workload
//! - Route caching and prediction
//! - Load-aware routing
//! - Fallback chains
//! - Route metrics collection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ROUTE TYPES
// ============================================================================

/// Route path type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoutePath {
    /// Ultra-fast path - no validation, trusted callers
    UltraFast = 0,
    /// Fast path - minimal validation
    Fast      = 1,
    /// Normal path - full validation
    Normal    = 2,
    /// Slow path - extra checking, logging
    Slow      = 3,
    /// Async path - queued for later
    Async     = 4,
    /// Batch path - coalesced with similar calls
    Batch     = 5,
    /// Emulation path - compatibility layer
    Emulation = 6,
    /// Redirect path - forwarded to another handler
    Redirect  = 7,
}

impl RoutePath {
    /// Relative cost (lower is cheaper)
    pub fn cost(&self) -> u32 {
        match self {
            Self::UltraFast => 1,
            Self::Fast => 5,
            Self::Normal => 20,
            Self::Slow => 100,
            Self::Async => 15,
            Self::Batch => 10,
            Self::Emulation => 200,
            Self::Redirect => 50,
        }
    }

    /// Is synchronous
    pub fn is_synchronous(&self) -> bool {
        matches!(
            self,
            Self::UltraFast | Self::Fast | Self::Normal | Self::Slow | Self::Emulation
        )
    }
}

/// Route selection reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteReason {
    /// Default route
    Default,
    /// Selected by prediction
    Predicted,
    /// Workload-specific
    WorkloadSpecific,
    /// Load balancing
    LoadBalanced,
    /// Security constraint
    SecurityConstraint,
    /// Compatibility requirement
    Compatibility,
    /// Manual override
    Override,
    /// Fallback
    Fallback,
    /// Cache hit
    CacheHit,
}

// ============================================================================
// ROUTE TABLE
// ============================================================================

/// Route entry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// Syscall number
    pub syscall_nr: u32,
    /// Selected path
    pub path: RoutePath,
    /// Handler ID
    pub handler_id: u32,
    /// Priority (higher = preferred)
    pub priority: u32,
    /// Conditions for this route
    pub conditions: RouteConditions,
    /// Route statistics
    pub stats: RouteStats,
}

/// Conditions for route selection
#[derive(Debug, Clone)]
pub struct RouteConditions {
    /// Required process trust level (0 = any)
    pub min_trust_level: u32,
    /// Maximum CPU load for this route (percent, 0 = any)
    pub max_cpu_load: u32,
    /// Required capability flags
    pub required_capabilities: u64,
    /// Process group filter (0 = any)
    pub process_group: u64,
    /// Only for specific architecture
    pub arch_specific: bool,
}

impl Default for RouteConditions {
    fn default() -> Self {
        Self {
            min_trust_level: 0,
            max_cpu_load: 0,
            required_capabilities: 0,
            process_group: 0,
            arch_specific: false,
        }
    }
}

/// Route statistics
#[derive(Debug, Clone, Default)]
pub struct RouteStats {
    /// Total calls through this route
    pub total_calls: u64,
    /// Average latency (nanoseconds)
    pub avg_latency_ns: u64,
    /// Maximum latency (nanoseconds)
    pub max_latency_ns: u64,
    /// Error count
    pub errors: u64,
    /// Last used timestamp
    pub last_used: u64,
}

// ============================================================================
// ROUTE CACHE
// ============================================================================

/// Cached route decision
#[derive(Debug, Clone, Copy)]
pub struct CachedRoute {
    /// Syscall number
    pub syscall_nr: u32,
    /// Process ID
    pub pid: u64,
    /// Selected path
    pub path: RoutePath,
    /// Handler ID
    pub handler_id: u32,
    /// Cache timestamp
    pub cached_at: u64,
    /// Expiry timestamp
    pub expires_at: u64,
    /// Hit count
    pub hits: u64,
}

/// Route cache (per-process recent routes)
pub struct RouteCache {
    /// Cache entries (pid -> syscall_nr -> cached route)
    entries: BTreeMap<u64, BTreeMap<u32, CachedRoute>>,
    /// Max entries per process
    max_per_process: usize,
    /// Default TTL (milliseconds)
    default_ttl_ms: u64,
    /// Total hits
    pub total_hits: u64,
    /// Total misses
    pub total_misses: u64,
}

impl RouteCache {
    pub fn new(max_per_process: usize, default_ttl_ms: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_per_process,
            default_ttl_ms,
            total_hits: 0,
            total_misses: 0,
        }
    }

    /// Lookup cached route
    pub fn lookup(&mut self, pid: u64, syscall_nr: u32, current_time: u64) -> Option<CachedRoute> {
        if let Some(process_cache) = self.entries.get_mut(&pid) {
            if let Some(cached) = process_cache.get_mut(&syscall_nr) {
                if current_time < cached.expires_at {
                    cached.hits += 1;
                    self.total_hits += 1;
                    return Some(*cached);
                } else {
                    // Expired
                    process_cache.remove(&syscall_nr);
                }
            }
        }
        self.total_misses += 1;
        None
    }

    /// Insert cached route
    pub fn insert(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        path: RoutePath,
        handler_id: u32,
        timestamp: u64,
    ) {
        let process_cache = self.entries.entry(pid).or_insert_with(BTreeMap::new);

        // Evict if full
        if process_cache.len() >= self.max_per_process {
            // Remove oldest
            let oldest = process_cache
                .iter()
                .min_by_key(|(_, v)| v.cached_at)
                .map(|(&k, _)| k);
            if let Some(key) = oldest {
                process_cache.remove(&key);
            }
        }

        process_cache.insert(syscall_nr, CachedRoute {
            syscall_nr,
            pid,
            path,
            handler_id,
            cached_at: timestamp,
            expires_at: timestamp + self.default_ttl_ms,
            hits: 0,
        });
    }

    /// Invalidate routes for process
    pub fn invalidate_process(&mut self, pid: u64) {
        self.entries.remove(&pid);
    }

    /// Invalidate specific syscall across all processes
    pub fn invalidate_syscall(&mut self, syscall_nr: u32) {
        for process_cache in self.entries.values_mut() {
            process_cache.remove(&syscall_nr);
        }
    }

    /// Hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_hits + self.total_misses;
        if total == 0 {
            return 0.0;
        }
        self.total_hits as f64 / total as f64
    }
}

// ============================================================================
// FALLBACK CHAIN
// ============================================================================

/// Fallback handler
#[derive(Debug, Clone)]
pub struct FallbackHandler {
    /// Handler ID
    pub handler_id: u32,
    /// Path type
    pub path: RoutePath,
    /// Order in chain (lower = tried first)
    pub order: u32,
    /// Enabled
    pub enabled: bool,
}

/// Fallback chain for a syscall
#[derive(Debug, Clone)]
pub struct FallbackChain {
    /// Syscall number
    pub syscall_nr: u32,
    /// Handlers in order
    handlers: Vec<FallbackHandler>,
}

impl FallbackChain {
    pub fn new(syscall_nr: u32) -> Self {
        Self {
            syscall_nr,
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler_id: u32, path: RoutePath, order: u32) {
        self.handlers.push(FallbackHandler {
            handler_id,
            path,
            order,
            enabled: true,
        });
        self.handlers.sort_by_key(|h| h.order);
    }

    /// Get next handler after current
    pub fn next_after(&self, current_handler: u32) -> Option<&FallbackHandler> {
        let mut found = false;
        for handler in &self.handlers {
            if found && handler.enabled {
                return Some(handler);
            }
            if handler.handler_id == current_handler {
                found = true;
            }
        }
        None
    }

    /// Get first enabled handler
    pub fn first(&self) -> Option<&FallbackHandler> {
        self.handlers.iter().find(|h| h.enabled)
    }
}

// ============================================================================
// ROUTING ENGINE
// ============================================================================

/// Syscall routing engine
pub struct RoutingEngine {
    /// Route table (syscall_nr -> routes)
    routes: BTreeMap<u32, Vec<RouteEntry>>,
    /// Route cache
    cache: RouteCache,
    /// Fallback chains
    fallbacks: BTreeMap<u32, FallbackChain>,
    /// Current system CPU load (percent)
    pub cpu_load: u32,
    /// Total routes
    pub total_decisions: u64,
    /// Total fallbacks used
    pub total_fallbacks: u64,
}

impl RoutingEngine {
    pub fn new() -> Self {
        Self {
            routes: BTreeMap::new(),
            cache: RouteCache::new(64, 5000),
            fallbacks: BTreeMap::new(),
            cpu_load: 0,
            total_decisions: 0,
            total_fallbacks: 0,
        }
    }

    /// Register route
    pub fn add_route(&mut self, entry: RouteEntry) {
        self.routes
            .entry(entry.syscall_nr)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    /// Register fallback chain
    pub fn add_fallback(&mut self, chain: FallbackChain) {
        self.fallbacks.insert(chain.syscall_nr, chain);
    }

    /// Route a syscall
    pub fn route(
        &mut self,
        syscall_nr: u32,
        pid: u64,
        trust_level: u32,
        capabilities: u64,
        timestamp: u64,
    ) -> Option<(RoutePath, u32, RouteReason)> {
        self.total_decisions += 1;

        // Check cache first
        if let Some(cached) = self.cache.lookup(pid, syscall_nr, timestamp) {
            return Some((cached.path, cached.handler_id, RouteReason::CacheHit));
        }

        // Find best route
        if let Some(routes) = self.routes.get(&syscall_nr) {
            let mut best: Option<(RoutePath, u32, u32)> = None;

            for route in routes {
                // Check conditions
                if route.conditions.min_trust_level > 0
                    && trust_level < route.conditions.min_trust_level
                {
                    continue;
                }
                if route.conditions.max_cpu_load > 0
                    && self.cpu_load > route.conditions.max_cpu_load
                {
                    continue;
                }
                if route.conditions.required_capabilities != 0
                    && (capabilities & route.conditions.required_capabilities)
                        != route.conditions.required_capabilities
                {
                    continue;
                }

                match best {
                    None => best = Some((route.path, route.handler_id, route.priority)),
                    Some((_, _, p)) if route.priority > p => {
                        best = Some((route.path, route.handler_id, route.priority))
                    },
                    _ => {},
                }
            }

            if let Some((path, handler, _)) = best {
                self.cache.insert(pid, syscall_nr, path, handler, timestamp);
                return Some((path, handler, RouteReason::Default));
            }
        }

        // Try fallback
        if let Some(chain) = self.fallbacks.get(&syscall_nr) {
            if let Some(handler) = chain.first() {
                self.total_fallbacks += 1;
                return Some((handler.path, handler.handler_id, RouteReason::Fallback));
            }
        }

        None
    }

    /// Update CPU load
    pub fn update_load(&mut self, cpu_load: u32) {
        self.cpu_load = cpu_load;
    }

    /// Update route stats
    pub fn record_completion(
        &mut self,
        syscall_nr: u32,
        handler_id: u32,
        latency_ns: u64,
        error: bool,
    ) {
        if let Some(routes) = self.routes.get_mut(&syscall_nr) {
            for route in routes {
                if route.stats.total_calls == 0 || route.handler_id == handler_id {
                    route.stats.total_calls += 1;
                    route.stats.avg_latency_ns = (route.stats.avg_latency_ns * 7 + latency_ns) / 8;
                    if latency_ns > route.stats.max_latency_ns {
                        route.stats.max_latency_ns = latency_ns;
                    }
                    if error {
                        route.stats.errors += 1;
                    }
                    break;
                }
            }
        }
    }

    /// Get cache stats
    pub fn cache_hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }
}
