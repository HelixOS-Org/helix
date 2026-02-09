//! Kprobe Intelligence
//!
//! Central coordinator for kprobe analysis.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    Architecture, FunctionTracer, KprobeAction, KprobeAnalysis, KprobeContext, KprobeId,
    KprobeIssue, KprobeIssueType, KprobeManager, KprobeRecommendation, ProbeAddress, SymbolInfo,
};

/// Kprobe Intelligence - comprehensive kprobe analysis and optimization
pub struct KprobeIntelligence {
    /// Kprobe manager
    manager: KprobeManager,
    /// Function tracer
    tracer: FunctionTracer,
    /// Symbol table
    symbols: BTreeMap<ProbeAddress, SymbolInfo>,
}

impl KprobeIntelligence {
    /// Create new kprobe intelligence
    pub fn new(arch: Architecture) -> Self {
        Self {
            manager: KprobeManager::new(arch),
            tracer: FunctionTracer::new(10000),
            symbols: BTreeMap::new(),
        }
    }

    /// Add symbol
    #[inline(always)]
    pub fn add_symbol(&mut self, symbol: SymbolInfo) {
        self.symbols.insert(symbol.address, symbol);
    }

    /// Lookup symbol
    pub fn lookup_symbol(&self, address: ProbeAddress) -> Option<&SymbolInfo> {
        // Exact match first
        if let Some(sym) = self.symbols.get(&address) {
            return Some(sym);
        }

        // Search for containing symbol
        for sym in self.symbols.values() {
            if sym.contains(address) {
                return Some(sym);
            }
        }

        None
    }

    /// Register kprobe by symbol
    pub fn register_by_symbol(
        &mut self,
        symbol_name: &str,
        offset: u64,
        timestamp: u64,
    ) -> Result<KprobeId, &'static str> {
        // Find symbol
        let symbol = self
            .symbols
            .values()
            .find(|s| s.name == symbol_name)
            .ok_or("Symbol not found")?
            .clone();

        let address = ProbeAddress::new(symbol.address.raw() + offset);
        let id = self.manager.register(address, timestamp)?;

        // Update kprobe with symbol info
        if let Some(def) = self.manager.get_mut(id) {
            def.symbol = Some(symbol);
            def.offset = offset;
        }

        Ok(id)
    }

    /// Register kprobe by address
    pub fn register_by_address(
        &mut self,
        address: ProbeAddress,
        timestamp: u64,
    ) -> Result<KprobeId, &'static str> {
        let id = self.manager.register(address, timestamp)?;

        // Try to find symbol
        if let Some(symbol) = self.lookup_symbol(address) {
            if let Some(def) = self.manager.get_mut(id) {
                def.symbol = Some(symbol.clone());
                def.offset = symbol.offset(address).unwrap_or(0);
            }
        }

        Ok(id)
    }

    /// Handle kprobe hit
    #[inline]
    pub fn handle_hit(&mut self, id: KprobeId, ctx: &KprobeContext) {
        if let Some(def) = self.manager.get_mut(id) {
            def.hit();

            // Record in function tracer
            let symbol_name = def.symbol.as_ref().map(|s| s.name.as_str());
            self.tracer.record_entry(id, ctx, symbol_name);
        }
    }

    /// Handle kretprobe hit
    #[inline(always)]
    pub fn handle_return(&mut self, id: KprobeId, ctx: &KprobeContext) {
        self.tracer.record_exit(id, ctx);
    }

    /// Analyze kprobe
    pub fn analyze(&self, id: KprobeId) -> Option<KprobeAnalysis> {
        let def = self.manager.get(id)?;
        let stats = self.tracer.get_stats(id).cloned();

        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check hit count
        let hits = def.hit_count();
        if hits == 0 {
            health_score -= 10.0;
            issues.push(KprobeIssue {
                issue_type: KprobeIssueType::NeverHit,
                severity: 3,
                description: String::from("Kprobe has never been hit"),
            });
            recommendations.push(KprobeRecommendation {
                action: KprobeAction::RemoveUnused,
                expected_improvement: 5.0,
                reason: String::from("Consider removing unused probe"),
            });
        }

        // Check miss rate
        let misses = def.miss_count();
        if misses > 0 && hits > 0 {
            let miss_rate = misses as f32 / (hits + misses) as f32;
            if miss_rate > 0.5 {
                health_score -= 20.0;
                issues.push(KprobeIssue {
                    issue_type: KprobeIssueType::TooManyMisses,
                    severity: 6,
                    description: format!("High miss rate: {:.1}%", miss_rate * 100.0),
                });
                recommendations.push(KprobeRecommendation {
                    action: KprobeAction::AddFilter,
                    expected_improvement: 15.0,
                    reason: String::from("Add filter to reduce misses"),
                });
            }
        }

        // Check performance impact
        if let Some(ref s) = stats {
            if s.avg_time > 100000.0 {
                // > 100us average
                health_score -= 25.0;
                issues.push(KprobeIssue {
                    issue_type: KprobeIssueType::PerformanceImpact,
                    severity: 8,
                    description: format!("High overhead: {:.0}ns average", s.avg_time),
                });
                recommendations.push(KprobeRecommendation {
                    action: KprobeAction::OptimizeHandler,
                    expected_improvement: 20.0,
                    reason: String::from("Optimize probe handler to reduce overhead"),
                });
            }
        }

        health_score = health_score.max(0.0);

        let hit_rate = if let Some(ref s) = stats {
            if s.last_call > def.registered_at {
                let elapsed_s = (s.last_call - def.registered_at) as f32 / 1_000_000_000.0;
                if elapsed_s > 0.0 {
                    hits as f32 / elapsed_s
                } else {
                    0.0
                }
            } else {
                0.0
            }
        } else {
            0.0
        };

        Some(KprobeAnalysis {
            kprobe_id: id,
            health_score,
            hit_rate,
            function_stats: stats,
            issues,
            recommendations,
        })
    }

    /// Get kprobe manager
    #[inline(always)]
    pub fn manager(&self) -> &KprobeManager {
        &self.manager
    }

    /// Get kprobe manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut KprobeManager {
        &mut self.manager
    }

    /// Get function tracer
    #[inline(always)]
    pub fn tracer(&self) -> &FunctionTracer {
        &self.tracer
    }

    /// Get function tracer mutably
    #[inline(always)]
    pub fn tracer_mut(&mut self) -> &mut FunctionTracer {
        &mut self.tracer
    }
}
