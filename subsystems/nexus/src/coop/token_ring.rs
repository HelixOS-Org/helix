//! # Coop Token Ring
//!
//! Token ring protocol for ordered cooperative access:
//! - Ordered token passing between cooperative processes
//! - Priority-based token acceleration
//! - Lost token detection and regeneration
//! - Multi-token support for concurrent access
//! - Token holding time enforcement
//! - Ring topology management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Token state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// Token is free (held by no one)
    Free,
    /// Token is held by a process
    Held,
    /// Token is in transit between nodes
    InTransit,
    /// Token is lost (timeout)
    Lost,
}

/// Ring node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingNodeState {
    /// Active in ring
    Active,
    /// Joining ring
    Joining,
    /// Leaving ring
    Leaving,
    /// Failed/unreachable
    Failed,
}

/// A token
#[derive(Debug, Clone)]
pub struct Token {
    pub token_id: u64,
    pub holder: Option<u64>,
    pub state: TokenState,
    pub priority: u8,
    pub generation: u64,
    pub acquired_ns: u64,
    pub max_hold_ns: u64,
    pub pass_count: u64,
    pub last_pass_ns: u64,
}

impl Token {
    pub fn new(id: u64, max_hold_ns: u64) -> Self {
        Self {
            token_id: id,
            holder: None,
            state: TokenState::Free,
            priority: 0,
            generation: 0,
            acquired_ns: 0,
            max_hold_ns,
            pass_count: 0,
            last_pass_ns: 0,
        }
    }

    pub fn acquire(&mut self, pid: u64, now_ns: u64) {
        self.holder = Some(pid);
        self.state = TokenState::Held;
        self.acquired_ns = now_ns;
    }

    pub fn release(&mut self, now_ns: u64) {
        self.holder = None;
        self.state = TokenState::Free;
        self.pass_count += 1;
        self.last_pass_ns = now_ns;
    }

    pub fn is_expired(&self, now_ns: u64) -> bool {
        self.state == TokenState::Held && now_ns.saturating_sub(self.acquired_ns) > self.max_hold_ns
    }

    pub fn hold_time_ns(&self, now_ns: u64) -> u64 {
        if self.state == TokenState::Held {
            now_ns.saturating_sub(self.acquired_ns)
        } else {
            0
        }
    }

    /// Token rotation rate (passes per second from recent)
    pub fn rotation_rate(&self, now_ns: u64) -> f64 {
        if self.pass_count == 0 || now_ns == 0 {
            return 0.0;
        }
        let elapsed = now_ns.saturating_sub(self.last_pass_ns.saturating_sub(1_000_000_000));
        if elapsed == 0 {
            return 0.0;
        }
        self.pass_count as f64 * 1_000_000_000.0 / elapsed as f64
    }
}

/// Ring node
#[derive(Debug, Clone)]
pub struct RingNode {
    pub pid: u64,
    pub state: RingNodeState,
    pub next_node: Option<u64>,
    pub prev_node: Option<u64>,
    pub tokens_held: u32,
    pub tokens_passed: u64,
    pub total_hold_time_ns: u64,
    pub priority: u8,
    pub last_seen_ns: u64,
    pub join_ns: u64,
}

impl RingNode {
    pub fn new(pid: u64, now_ns: u64) -> Self {
        Self {
            pid,
            state: RingNodeState::Active,
            next_node: None,
            prev_node: None,
            tokens_held: 0,
            tokens_passed: 0,
            total_hold_time_ns: 0,
            priority: 0,
            last_seen_ns: now_ns,
            join_ns: now_ns,
        }
    }

    pub fn avg_hold_time_ns(&self) -> f64 {
        if self.tokens_passed == 0 {
            0.0
        } else {
            self.total_hold_time_ns as f64 / self.tokens_passed as f64
        }
    }
}

/// Token ring stats
#[derive(Debug, Clone, Default)]
pub struct CoopTokenRingStats {
    pub ring_size: usize,
    pub active_tokens: usize,
    pub held_tokens: usize,
    pub lost_tokens: usize,
    pub total_passes: u64,
    pub avg_hold_time_ns: f64,
    pub failed_nodes: usize,
}

/// Coop Token Ring
pub struct CoopTokenRing {
    nodes: BTreeMap<u64, RingNode>,
    tokens: BTreeMap<u64, Token>,
    /// Ring order
    ring_order: Vec<u64>,
    stats: CoopTokenRingStats,
    next_token_id: u64,
    lost_token_timeout_ns: u64,
}

impl CoopTokenRing {
    pub fn new(default_hold_ns: u64) -> Self {
        Self {
            nodes: BTreeMap::new(),
            tokens: BTreeMap::new(),
            ring_order: Vec::new(),
            stats: CoopTokenRingStats::default(),
            next_token_id: 1,
            lost_token_timeout_ns: default_hold_ns * 3,
        }
    }

    /// Add a node to the ring
    pub fn join(&mut self, pid: u64, now_ns: u64) {
        let node = RingNode::new(pid, now_ns);
        self.nodes.insert(pid, node);
        self.ring_order.push(pid);
        self.rebuild_links();
        self.update_stats();
    }

    /// Remove a node from the ring
    pub fn leave(&mut self, pid: u64) {
        self.nodes.remove(&pid);
        self.ring_order.retain(|&p| p != pid);
        // Release any tokens held by this node
        for token in self.tokens.values_mut() {
            if token.holder == Some(pid) {
                token.release(0);
            }
        }
        self.rebuild_links();
        self.update_stats();
    }

    fn rebuild_links(&mut self) {
        let n = self.ring_order.len();
        if n == 0 {
            return;
        }
        for i in 0..n {
            let pid = self.ring_order[i];
            let next = self.ring_order[(i + 1) % n];
            let prev = self.ring_order[(i + n - 1) % n];
            if let Some(node) = self.nodes.get_mut(&pid) {
                node.next_node = Some(next);
                node.prev_node = Some(prev);
            }
        }
    }

    /// Create a token
    pub fn create_token(&mut self, max_hold_ns: u64) -> u64 {
        let id = self.next_token_id;
        self.next_token_id += 1;
        self.tokens.insert(id, Token::new(id, max_hold_ns));
        self.update_stats();
        id
    }

    /// Try to acquire a token
    pub fn acquire(&mut self, pid: u64, token_id: u64, now_ns: u64) -> bool {
        let token = match self.tokens.get_mut(&token_id) {
            Some(t) => t,
            None => return false,
        };

        if token.state != TokenState::Free {
            return false;
        }

        // Check if it's this node's turn (if ring order matters)
        token.acquire(pid, now_ns);
        if let Some(node) = self.nodes.get_mut(&pid) {
            node.tokens_held += 1;
            node.last_seen_ns = now_ns;
        }
        self.update_stats();
        true
    }

    /// Release a token (pass to next)
    pub fn release(&mut self, pid: u64, token_id: u64, now_ns: u64) -> bool {
        let hold_time = {
            let token = match self.tokens.get_mut(&token_id) {
                Some(t) => t,
                None => return false,
            };
            if token.holder != Some(pid) {
                return false;
            }
            let ht = token.hold_time_ns(now_ns);
            token.release(now_ns);
            ht
        };

        if let Some(node) = self.nodes.get_mut(&pid) {
            if node.tokens_held > 0 {
                node.tokens_held -= 1;
            }
            node.tokens_passed += 1;
            node.total_hold_time_ns += hold_time;
        }
        self.update_stats();
        true
    }

    /// Detect and regenerate lost tokens
    pub fn detect_lost_tokens(&mut self, now_ns: u64) {
        for token in self.tokens.values_mut() {
            if token.is_expired(now_ns) {
                token.state = TokenState::Lost;
                token.holder = None;
                token.generation += 1;
                // Regenerate as free
                token.state = TokenState::Free;
            }
        }

        // Detect failed nodes
        for node in self.nodes.values_mut() {
            if now_ns.saturating_sub(node.last_seen_ns) > self.lost_token_timeout_ns * 2 {
                node.state = RingNodeState::Failed;
            }
        }
        self.update_stats();
    }

    /// Get next in ring
    pub fn next_after(&self, pid: u64) -> Option<u64> {
        self.nodes.get(&pid)?.next_node
    }

    fn update_stats(&mut self) {
        self.stats.ring_size = self.ring_order.len();
        self.stats.active_tokens = self.tokens.len();
        self.stats.held_tokens = self
            .tokens
            .values()
            .filter(|t| t.state == TokenState::Held)
            .count();
        self.stats.lost_tokens = self
            .tokens
            .values()
            .filter(|t| t.state == TokenState::Lost)
            .count();
        self.stats.total_passes = self.tokens.values().map(|t| t.pass_count).sum();
        self.stats.failed_nodes = self
            .nodes
            .values()
            .filter(|n| n.state == RingNodeState::Failed)
            .count();
        let total_hold: f64 = self.nodes.values().map(|n| n.avg_hold_time_ns()).sum();
        if !self.nodes.is_empty() {
            self.stats.avg_hold_time_ns = total_hold / self.nodes.len() as f64;
        }
    }

    pub fn stats(&self) -> &CoopTokenRingStats {
        &self.stats
    }
}

// ============================================================================
// Merged from token_ring_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenV2State {
    Free,
    Held,
    Passing,
    Lost,
}

/// A participant in the token ring
#[derive(Debug, Clone)]
pub struct TokenV2Participant {
    pub id: u64,
    pub position: u32,
    pub tokens_held: u64,
    pub tokens_passed: u64,
    pub hold_ticks: u64,
    pub active: bool,
}

/// A token ring V2 instance
#[derive(Debug, Clone)]
pub struct TokenRingV2Instance {
    pub id: u64,
    pub participants: Vec<TokenV2Participant>,
    pub current_holder: Option<u32>,
    pub token_state: TokenV2State,
    pub rotations: u64,
    pub max_hold_ticks: u64,
    pub total_passes: u64,
    pub lost_tokens: u64,
}

impl TokenRingV2Instance {
    pub fn new(id: u64) -> Self {
        Self {
            id, participants: Vec::new(),
            current_holder: None,
            token_state: TokenV2State::Free,
            rotations: 0, max_hold_ticks: 100,
            total_passes: 0, lost_tokens: 0,
        }
    }

    pub fn add_participant(&mut self, pid: u64) -> u32 {
        let pos = self.participants.len() as u32;
        self.participants.push(TokenV2Participant {
            id: pid, position: pos,
            tokens_held: 0, tokens_passed: 0,
            hold_ticks: 0, active: true,
        });
        pos
    }

    pub fn pass_token(&mut self) -> Option<u32> {
        if self.participants.is_empty() { return None; }
        let next_pos = if let Some(current) = self.current_holder {
            let mut next = (current + 1) % self.participants.len() as u32;
            let start = next;
            loop {
                if self.participants[next as usize].active { break; }
                next = (next + 1) % self.participants.len() as u32;
                if next == start { return None; }
            }
            next
        } else { 0 };

        if let Some(old) = self.current_holder {
            self.participants[old as usize].tokens_passed += 1;
        }
        self.current_holder = Some(next_pos);
        self.participants[next_pos as usize].tokens_held += 1;
        self.token_state = TokenV2State::Held;
        self.total_passes += 1;
        if next_pos == 0 { self.rotations += 1; }
        Some(next_pos)
    }
}

/// Statistics for token ring V2
#[derive(Debug, Clone)]
pub struct TokenRingV2Stats {
    pub rings_created: u64,
    pub total_passes: u64,
    pub total_rotations: u64,
    pub lost_tokens: u64,
    pub participants_total: u64,
}

/// Main token ring V2 coop manager
#[derive(Debug)]
pub struct CoopTokenRingV2 {
    rings: BTreeMap<u64, TokenRingV2Instance>,
    next_id: u64,
    stats: TokenRingV2Stats,
}

impl CoopTokenRingV2 {
    pub fn new() -> Self {
        Self {
            rings: BTreeMap::new(),
            next_id: 1,
            stats: TokenRingV2Stats {
                rings_created: 0, total_passes: 0,
                total_rotations: 0, lost_tokens: 0,
                participants_total: 0,
            },
        }
    }

    pub fn create_ring(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.rings.insert(id, TokenRingV2Instance::new(id));
        self.stats.rings_created += 1;
        id
    }

    pub fn add_participant(&mut self, ring_id: u64, pid: u64) -> Option<u32> {
        if let Some(ring) = self.rings.get_mut(&ring_id) {
            let pos = ring.add_participant(pid);
            self.stats.participants_total += 1;
            Some(pos)
        } else { None }
    }

    pub fn pass_token(&mut self, ring_id: u64) -> Option<u32> {
        if let Some(ring) = self.rings.get_mut(&ring_id) {
            let result = ring.pass_token();
            if result.is_some() {
                self.stats.total_passes += 1;
            }
            result
        } else { None }
    }

    pub fn stats(&self) -> &TokenRingV2Stats {
        &self.stats
    }
}
