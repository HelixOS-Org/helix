// SPDX-License-Identifier: MIT
//! # Holistic Memory Protection Analysis
//!
//! System-wide memory protection optimization:
//! - Global W^X enforcement dashboard
//! - Protection key (PKU) domain optimization
//! - System-wide ASLR entropy analysis
//! - Cross-process guard page efficiency
//! - Security posture scoring

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityRating { Excellent, Good, Fair, Poor, Critical }

impl SecurityRating {
    #[inline]
    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 { Self::Excellent }
        else if score >= 0.7 { Self::Good }
        else if score >= 0.5 { Self::Fair }
        else if score >= 0.3 { Self::Poor }
        else { Self::Critical }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessSecurityProfile {
    pub pid: u64,
    pub wx_compliant: bool,
    pub aslr_bits: u32,
    pub guard_pages: u64,
    pub writable_exec_pages: u64,
    pub pku_domain: Option<u32>,
    pub stack_canary: bool,
    pub relro_active: bool,
}

impl ProcessSecurityProfile {
    pub fn security_score(&self) -> f64 {
        let mut score = 0.0f64;
        if self.wx_compliant { score += 0.25; }
        if self.aslr_bits >= 28 { score += 0.20; }
        else if self.aslr_bits >= 20 { score += 0.10; }
        if self.guard_pages > 0 { score += 0.15; }
        if self.writable_exec_pages == 0 { score += 0.15; }
        if self.stack_canary { score += 0.10; }
        if self.relro_active { score += 0.10; }
        if self.pku_domain.is_some() { score += 0.05; }
        score
    }
}

#[derive(Debug, Clone)]
pub struct PkuDomainMap {
    pub domain_id: u32,
    pub pids: Vec<u64>,
    pub total_pages: u64,
    pub permission_bits: u32,
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MprotectHolisticStats {
    pub total_processes: u64,
    pub wx_compliant_count: u64,
    pub wx_violation_count: u64,
    pub avg_aslr_bits: f64,
    pub total_guard_pages: u64,
    pub system_security_score: f64,
    pub pku_domains_active: u64,
}

pub struct MprotectHolisticManager {
    profiles: BTreeMap<u64, ProcessSecurityProfile>,
    pku_domains: BTreeMap<u32, PkuDomainMap>,
    /// Historic W^X violation timeline: (timestamp, pid, page_addr)
    wx_timeline: Vec<(u64, u64, u64)>,
    stats: MprotectHolisticStats,
}

impl MprotectHolisticManager {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            pku_domains: BTreeMap::new(),
            wx_timeline: Vec::new(),
            stats: MprotectHolisticStats::default(),
        }
    }

    #[inline]
    pub fn register_process(&mut self, profile: ProcessSecurityProfile) {
        if profile.wx_compliant { self.stats.wx_compliant_count += 1; }
        self.stats.total_guard_pages += profile.guard_pages;
        self.stats.total_processes += 1;
        self.profiles.insert(profile.pid, profile);
        self.recompute_scores();
    }

    #[inline]
    pub fn record_wx_violation(&mut self, pid: u64, page_addr: u64, now: u64) {
        self.stats.wx_violation_count += 1;
        self.wx_timeline.push((now, pid, page_addr));
        if self.wx_timeline.len() > 1024 { self.wx_timeline.drain(..512); }
        if let Some(p) = self.profiles.get_mut(&pid) {
            p.wx_compliant = false;
            p.writable_exec_pages += 1;
        }
        self.recompute_scores();
    }

    fn recompute_scores(&mut self) {
        if self.profiles.is_empty() { return; }
        let total_score: f64 = self.profiles.values()
            .map(|p| p.security_score()).sum();
        self.stats.system_security_score = total_score / self.profiles.len() as f64;

        let total_aslr: u32 = self.profiles.values().map(|p| p.aslr_bits).sum();
        self.stats.avg_aslr_bits = total_aslr as f64 / self.profiles.len() as f64;
    }

    /// System security rating
    #[inline(always)]
    pub fn system_rating(&self) -> SecurityRating {
        SecurityRating::from_score(self.stats.system_security_score)
    }

    /// Find processes with weakest security
    #[inline]
    pub fn weakest_processes(&self, n: usize) -> Vec<(u64, f64)> {
        let mut scores: Vec<_> = self.profiles.iter()
            .map(|(&pid, p)| (pid, p.security_score()))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        scores.into_iter().take(n).collect()
    }

    /// Register a PKU domain
    #[inline]
    pub fn register_pku_domain(&mut self, domain_id: u32, pids: Vec<u64>, pages: u64, perms: u32) {
        self.pku_domains.insert(domain_id, PkuDomainMap {
            domain_id, pids, total_pages: pages, permission_bits: perms,
        });
        self.stats.pku_domains_active = self.pku_domains.len() as u64;
    }

    /// Get recent W^X violation trends
    #[inline]
    pub fn wx_violation_rate(&self, window_ns: u64, now: u64) -> f64 {
        let cutoff = now.saturating_sub(window_ns);
        let recent = self.wx_timeline.iter()
            .filter(|&&(ts, _, _)| ts >= cutoff)
            .count();
        if window_ns == 0 { return 0.0; }
        recent as f64 / (window_ns as f64 / 1_000_000_000.0)
    }

    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessSecurityProfile> { self.profiles.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &MprotectHolisticStats { &self.stats }
}
