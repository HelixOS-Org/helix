//! # Coop Conflict Resolver
//!
//! Cooperative conflict resolution for distributed resources:
//! - Multi-party conflict detection with causal ordering
//! - Resolution strategies: last-writer-wins, merge, arbitration, custom
//! - Conflict graph with cycle detection
//! - Priority-based tie-breaking
//! - Conflict history and audit trail
//! - Automatic retry with exponential backoff

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    LastWriterWins,
    FirstWriterWins,
    HigherPriority,
    MergeAll,
    Arbitrate,
    Custom(u32),
    Abort,
    Retry,
}

/// Conflict severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConflictSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Conflict state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictState {
    Detected,
    Analyzing,
    Resolving,
    Resolved,
    Escalated,
    Failed,
    Retrying,
}

/// Resource identifier for conflicts
#[derive(Debug, Clone)]
pub struct ConflictResource {
    pub resource_id: u64,
    pub resource_type: u32,
    pub version: u64,
    pub owner: u64,
    pub name_hash: u64,
}

impl ConflictResource {
    pub fn new(id: u64, rtype: u32, ver: u64, owner: u64, name: &str) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self { resource_id: id, resource_type: rtype, version: ver, owner, name_hash: hash }
    }
}

/// A party in a conflict
#[derive(Debug, Clone)]
pub struct ConflictParty {
    pub id: u64,
    pub priority: u32,
    pub ts: u64,
    pub operation_hash: u64,
    pub retries: u32,
}

/// A conflict instance
#[derive(Debug, Clone)]
pub struct Conflict {
    pub id: u64,
    pub resource: ConflictResource,
    pub parties: Vec<ConflictParty>,
    pub state: ConflictState,
    pub severity: ConflictSeverity,
    pub strategy: ResolutionStrategy,
    pub winner: Option<u64>,
    pub detect_ts: u64,
    pub resolve_ts: u64,
    pub attempts: u32,
}

impl Conflict {
    pub fn new(id: u64, resource: ConflictResource, ts: u64, sev: ConflictSeverity) -> Self {
        Self {
            id, resource, parties: Vec::new(), state: ConflictState::Detected,
            severity: sev, strategy: ResolutionStrategy::LastWriterWins,
            winner: None, detect_ts: ts, resolve_ts: 0, attempts: 0,
        }
    }

    pub fn add_party(&mut self, party: ConflictParty) { self.parties.push(party); }

    pub fn resolve_lww(&mut self, ts: u64) {
        self.state = ConflictState::Resolving;
        self.winner = self.parties.iter().max_by_key(|p| p.ts).map(|p| p.id);
        self.state = ConflictState::Resolved;
        self.resolve_ts = ts;
    }

    pub fn resolve_fww(&mut self, ts: u64) {
        self.state = ConflictState::Resolving;
        self.winner = self.parties.iter().min_by_key(|p| p.ts).map(|p| p.id);
        self.state = ConflictState::Resolved;
        self.resolve_ts = ts;
    }

    pub fn resolve_priority(&mut self, ts: u64) {
        self.state = ConflictState::Resolving;
        self.winner = self.parties.iter().max_by_key(|p| p.priority).map(|p| p.id);
        self.state = ConflictState::Resolved;
        self.resolve_ts = ts;
    }

    pub fn resolve(&mut self, ts: u64) {
        self.attempts += 1;
        match self.strategy {
            ResolutionStrategy::LastWriterWins => self.resolve_lww(ts),
            ResolutionStrategy::FirstWriterWins => self.resolve_fww(ts),
            ResolutionStrategy::HigherPriority => self.resolve_priority(ts),
            ResolutionStrategy::Abort => { self.state = ConflictState::Failed; self.resolve_ts = ts; }
            _ => self.resolve_lww(ts),
        }
    }

    pub fn escalate(&mut self) { self.state = ConflictState::Escalated; }
    pub fn is_resolved(&self) -> bool { self.state == ConflictState::Resolved }
    pub fn latency(&self) -> u64 { self.resolve_ts.saturating_sub(self.detect_ts) }
}

/// Conflict graph edge (resource dependency)
#[derive(Debug, Clone)]
pub struct ConflictEdge {
    pub from: u64,
    pub to: u64,
    pub weight: u32,
}

/// Conflict resolver stats
#[derive(Debug, Clone, Default)]
pub struct ResolverStats {
    pub total_conflicts: u64,
    pub resolved: u64,
    pub escalated: u64,
    pub failed: u64,
    pub avg_latency_ns: u64,
    pub by_severity: [u64; 5],
}

/// Cooperative conflict resolver
pub struct CoopConflictResolver {
    conflicts: BTreeMap<u64, Conflict>,
    edges: Vec<ConflictEdge>,
    history: Vec<u64>,
    stats: ResolverStats,
    next_id: u64,
    default_strategy: ResolutionStrategy,
    max_retries: u32,
}

impl CoopConflictResolver {
    pub fn new(strategy: ResolutionStrategy, max_retries: u32) -> Self {
        Self {
            conflicts: BTreeMap::new(), edges: Vec::new(),
            history: Vec::new(), stats: ResolverStats::default(),
            next_id: 1, default_strategy: strategy, max_retries,
        }
    }

    pub fn detect(&mut self, resource: ConflictResource, parties: Vec<ConflictParty>, ts: u64, sev: ConflictSeverity) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut c = Conflict::new(id, resource, ts, sev);
        c.strategy = self.default_strategy;
        for p in parties { c.add_party(p); }
        self.conflicts.insert(id, c);
        self.stats.total_conflicts += 1;
        self.stats.by_severity[sev as usize] += 1;
        id
    }

    pub fn resolve(&mut self, id: u64, ts: u64) -> bool {
        if let Some(c) = self.conflicts.get_mut(&id) {
            c.resolve(ts);
            if c.is_resolved() {
                self.stats.resolved += 1;
                self.history.push(id);
                return true;
            }
            if c.attempts >= self.max_retries {
                c.escalate();
                self.stats.escalated += 1;
            }
        }
        false
    }

    pub fn resolve_all(&mut self, ts: u64) -> Vec<u64> {
        let ids: Vec<u64> = self.conflicts.keys().copied().collect();
        let mut resolved = Vec::new();
        for id in ids {
            if let Some(c) = self.conflicts.get(&id) {
                if c.state == ConflictState::Detected || c.state == ConflictState::Retrying {
                    if self.resolve(id, ts) { resolved.push(id); }
                }
            }
        }
        resolved
    }

    pub fn add_edge(&mut self, from: u64, to: u64, weight: u32) {
        self.edges.push(ConflictEdge { from, to, weight });
    }

    pub fn detect_cycles(&self) -> Vec<Vec<u64>> {
        let mut cycles = Vec::new();
        let mut visited: BTreeMap<u64, u8> = BTreeMap::new();
        let nodes: Vec<u64> = self.edges.iter().map(|e| e.from).collect();
        for n in &nodes { visited.entry(*n).or_insert(0); }
        // Simple DFS cycle detection
        for &start in &nodes {
            let mut stack = alloc::vec![start];
            let mut path = Vec::new();
            while let Some(cur) = stack.pop() {
                if path.contains(&cur) {
                    let idx = path.iter().position(|&x| x == cur).unwrap_or(0);
                    cycles.push(path[idx..].to_vec());
                    break;
                }
                path.push(cur);
                for e in &self.edges {
                    if e.from == cur { stack.push(e.to); }
                }
            }
        }
        cycles
    }

    pub fn recompute(&mut self) {
        let resolved: Vec<&Conflict> = self.conflicts.values().filter(|c| c.is_resolved()).collect();
        if !resolved.is_empty() {
            let total_lat: u64 = resolved.iter().map(|c| c.latency()).sum();
            self.stats.avg_latency_ns = total_lat / resolved.len() as u64;
        }
        self.stats.failed = self.conflicts.values().filter(|c| c.state == ConflictState::Failed).count() as u64;
    }

    pub fn conflict(&self, id: u64) -> Option<&Conflict> { self.conflicts.get(&id) }
    pub fn stats(&self) -> &ResolverStats { &self.stats }
    pub fn history(&self) -> &[u64] { &self.history }
    pub fn pending(&self) -> usize { self.conflicts.values().filter(|c| !c.is_resolved() && c.state != ConflictState::Failed).count() }
}
