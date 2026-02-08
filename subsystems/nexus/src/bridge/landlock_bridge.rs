// SPDX-License-Identifier: GPL-2.0
//! Bridge landlock — Landlock LSM access control proxy for unprivileged sandboxing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Landlock ABI version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LandlockAbi {
    V1,
    V2,
    V3,
    V4,
}

impl LandlockAbi {
    pub fn version_number(&self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
            Self::V4 => 4,
        }
    }
}

/// Filesystem access rights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsAccessRights(pub u64);

impl FsAccessRights {
    pub const EXECUTE: Self = Self(1 << 0);
    pub const WRITE_FILE: Self = Self(1 << 1);
    pub const READ_FILE: Self = Self(1 << 2);
    pub const READ_DIR: Self = Self(1 << 3);
    pub const REMOVE_DIR: Self = Self(1 << 4);
    pub const REMOVE_FILE: Self = Self(1 << 5);
    pub const MAKE_CHAR: Self = Self(1 << 6);
    pub const MAKE_DIR: Self = Self(1 << 7);
    pub const MAKE_REG: Self = Self(1 << 8);
    pub const MAKE_SOCK: Self = Self(1 << 9);
    pub const MAKE_FIFO: Self = Self(1 << 10);
    pub const MAKE_BLOCK: Self = Self(1 << 11);
    pub const MAKE_SYM: Self = Self(1 << 12);
    pub const REFER: Self = Self(1 << 13);
    pub const TRUNCATE: Self = Self(1 << 14);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn bit_count(&self) -> u32 {
        let mut n = self.0;
        let mut count = 0u32;
        while n != 0 {
            count += 1;
            n &= n - 1;
        }
        count
    }
}

/// Network access rights (V4+)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetAccessRights(pub u64);

impl NetAccessRights {
    pub const BIND_TCP: Self = Self(1 << 0);
    pub const CONNECT_TCP: Self = Self(1 << 1);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

/// A filesystem path rule
#[derive(Debug, Clone)]
pub struct FsPathRule {
    pub path: String,
    pub allowed_access: FsAccessRights,
    pub is_beneath: bool,
}

impl FsPathRule {
    pub fn new(path: String, access: FsAccessRights) -> Self {
        Self {
            path,
            allowed_access: access,
            is_beneath: true,
        }
    }

    pub fn matches(&self, target_path: &str) -> bool {
        if self.is_beneath {
            target_path.starts_with(self.path.as_str())
        } else {
            target_path == self.path.as_str()
        }
    }

    pub fn check_access(&self, requested: FsAccessRights) -> bool {
        self.allowed_access.contains(requested)
    }
}

/// A network port rule
#[derive(Debug, Clone)]
pub struct NetPortRule {
    pub port: u16,
    pub allowed_access: NetAccessRights,
}

impl NetPortRule {
    pub fn new(port: u16, access: NetAccessRights) -> Self {
        Self { port, allowed_access: access }
    }
}

/// A Landlock ruleset
#[derive(Debug)]
pub struct Ruleset {
    pub id: u64,
    pub abi: LandlockAbi,
    pub handled_fs_access: FsAccessRights,
    pub handled_net_access: NetAccessRights,
    pub fs_rules: Vec<FsPathRule>,
    pub net_rules: Vec<NetPortRule>,
    pub enforced: bool,
    creation_ns: u64,
    check_count: u64,
    deny_count: u64,
}

impl Ruleset {
    pub fn new(id: u64, abi: LandlockAbi) -> Self {
        Self {
            id,
            abi,
            handled_fs_access: FsAccessRights(0),
            handled_net_access: NetAccessRights(0),
            fs_rules: Vec::new(),
            net_rules: Vec::new(),
            enforced: false,
            creation_ns: 0,
            check_count: 0,
            deny_count: 0,
        }
    }

    pub fn add_fs_rule(&mut self, rule: FsPathRule) {
        self.fs_rules.push(rule);
    }

    pub fn add_net_rule(&mut self, rule: NetPortRule) {
        self.net_rules.push(rule);
    }

    pub fn check_fs_access(&mut self, path: &str, requested: FsAccessRights) -> bool {
        self.check_count += 1;
        if !self.enforced {
            return true;
        }
        // If the access type isn't handled, allow
        if !self.handled_fs_access.contains(requested) {
            return true;
        }
        // Must match at least one rule
        let allowed = self.fs_rules.iter().any(|rule| {
            rule.matches(path) && rule.check_access(requested)
        });
        if !allowed {
            self.deny_count += 1;
        }
        allowed
    }

    pub fn check_net_access(&mut self, port: u16, requested: NetAccessRights) -> bool {
        self.check_count += 1;
        if !self.enforced {
            return true;
        }
        if !self.handled_net_access.contains(requested) {
            return true;
        }
        let allowed = self.net_rules.iter().any(|rule| {
            rule.port == port && rule.allowed_access.contains(requested)
        });
        if !allowed {
            self.deny_count += 1;
        }
        allowed
    }

    pub fn deny_rate(&self) -> f64 {
        if self.check_count == 0 { return 0.0; }
        self.deny_count as f64 / self.check_count as f64
    }

    pub fn rule_count(&self) -> usize {
        self.fs_rules.len() + self.net_rules.len()
    }
}

/// Per-process landlock domain (stacked rulesets)
#[derive(Debug)]
pub struct LandlockDomain {
    pub pid: u64,
    pub rulesets: Vec<u64>,
    pub depth: u32,
    pub max_depth: u32,
}

impl LandlockDomain {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            rulesets: Vec::new(),
            depth: 0,
            max_depth: 16,
        }
    }

    pub fn push_ruleset(&mut self, ruleset_id: u64) -> bool {
        if self.depth >= self.max_depth {
            return false;
        }
        self.rulesets.push(ruleset_id);
        self.depth += 1;
        true
    }
}

/// Landlock bridge stats
#[derive(Debug, Clone)]
pub struct LandlockBridgeStats {
    pub rulesets_created: u64,
    pub domains_active: u64,
    pub access_checks: u64,
    pub access_denied: u64,
    pub fs_rules_total: u64,
    pub net_rules_total: u64,
}

/// Main landlock bridge manager
pub struct BridgeLandlock {
    rulesets: BTreeMap<u64, Ruleset>,
    domains: BTreeMap<u64, LandlockDomain>,
    next_ruleset_id: u64,
    supported_abi: LandlockAbi,
    stats: LandlockBridgeStats,
}

impl BridgeLandlock {
    pub fn new() -> Self {
        Self {
            rulesets: BTreeMap::new(),
            domains: BTreeMap::new(),
            next_ruleset_id: 1,
            supported_abi: LandlockAbi::V4,
            stats: LandlockBridgeStats {
                rulesets_created: 0,
                domains_active: 0,
                access_checks: 0,
                access_denied: 0,
                fs_rules_total: 0,
                net_rules_total: 0,
            },
        }
    }

    pub fn create_ruleset(
        &mut self,
        handled_fs: FsAccessRights,
        handled_net: NetAccessRights,
    ) -> u64 {
        let id = self.next_ruleset_id;
        self.next_ruleset_id += 1;
        let mut rs = Ruleset::new(id, self.supported_abi);
        rs.handled_fs_access = handled_fs;
        rs.handled_net_access = handled_net;
        self.rulesets.insert(id, rs);
        self.stats.rulesets_created += 1;
        id
    }

    pub fn add_fs_rule(&mut self, ruleset_id: u64, path: String, access: FsAccessRights) -> bool {
        if let Some(rs) = self.rulesets.get_mut(&ruleset_id) {
            if rs.enforced {
                return false;
            }
            rs.add_fs_rule(FsPathRule::new(path, access));
            self.stats.fs_rules_total += 1;
            true
        } else {
            false
        }
    }

    pub fn add_net_rule(&mut self, ruleset_id: u64, port: u16, access: NetAccessRights) -> bool {
        if let Some(rs) = self.rulesets.get_mut(&ruleset_id) {
            if rs.enforced {
                return false;
            }
            rs.add_net_rule(NetPortRule::new(port, access));
            self.stats.net_rules_total += 1;
            true
        } else {
            false
        }
    }

    pub fn enforce_ruleset(&mut self, pid: u64, ruleset_id: u64) -> bool {
        if let Some(rs) = self.rulesets.get_mut(&ruleset_id) {
            rs.enforced = true;
        } else {
            return false;
        }
        let domain = self.domains.entry(pid).or_insert_with(|| {
            self.stats.domains_active += 1;
            LandlockDomain::new(pid)
        });
        domain.push_ruleset(ruleset_id)
    }

    pub fn check_fs_access(&mut self, pid: u64, path: &str, access: FsAccessRights) -> bool {
        self.stats.access_checks += 1;
        let domain = match self.domains.get(&pid) {
            Some(d) => d,
            None => return true, // No landlock domain
        };
        let ruleset_ids: Vec<u64> = domain.rulesets.clone();
        for rs_id in &ruleset_ids {
            if let Some(rs) = self.rulesets.get_mut(rs_id) {
                if !rs.check_fs_access(path, access) {
                    self.stats.access_denied += 1;
                    return false;
                }
            }
        }
        true
    }

    pub fn check_net_access(&mut self, pid: u64, port: u16, access: NetAccessRights) -> bool {
        self.stats.access_checks += 1;
        let domain = match self.domains.get(&pid) {
            Some(d) => d,
            None => return true,
        };
        let ruleset_ids: Vec<u64> = domain.rulesets.clone();
        for rs_id in &ruleset_ids {
            if let Some(rs) = self.rulesets.get_mut(rs_id) {
                if !rs.check_net_access(port, access) {
                    self.stats.access_denied += 1;
                    return false;
                }
            }
        }
        true
    }

    pub fn remove_domain(&mut self, pid: u64) -> bool {
        if self.domains.remove(&pid).is_some() {
            self.stats.domains_active = self.stats.domains_active.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> &LandlockBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from landlock_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV2RuleType {
    PathBeneath,
    PortAccess,
    FileIoctl,
}

/// Access flags for filesystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandlockFsFlags(pub u64);

impl LandlockFsFlags {
    pub const EXECUTE: u64 = 1 << 0;
    pub const WRITE_FILE: u64 = 1 << 1;
    pub const READ_FILE: u64 = 1 << 2;
    pub const READ_DIR: u64 = 1 << 3;
    pub const REMOVE_DIR: u64 = 1 << 4;
    pub const REMOVE_FILE: u64 = 1 << 5;
    pub const MAKE_CHAR: u64 = 1 << 6;
    pub const MAKE_DIR: u64 = 1 << 7;
    pub const MAKE_REG: u64 = 1 << 8;
    pub const MAKE_SOCK: u64 = 1 << 9;
    pub const MAKE_FIFO: u64 = 1 << 10;
    pub const MAKE_BLOCK: u64 = 1 << 11;
    pub const MAKE_SYM: u64 = 1 << 12;
    pub const REFER: u64 = 1 << 13;
    pub const TRUNCATE: u64 = 1 << 14;
    pub const IOCTL_DEV: u64 = 1 << 15;

    pub fn new() -> Self { Self(0) }
    pub fn set(&mut self, f: u64) { self.0 |= f; }
    pub fn has(&self, f: u64) -> bool { self.0 & f != 0 }
}

/// Landlock rule
#[derive(Debug, Clone)]
pub struct LandlockV2Rule {
    pub id: u64,
    pub rule_type: LandlockV2RuleType,
    pub allowed_access: LandlockFsFlags,
    pub parent_fd: i32,
    pub port: u16,
    pub match_count: u64,
    pub deny_count: u64,
}

impl LandlockV2Rule {
    pub fn path_beneath(id: u64, parent_fd: i32, access: LandlockFsFlags) -> Self {
        Self { id, rule_type: LandlockV2RuleType::PathBeneath, allowed_access: access, parent_fd, port: 0, match_count: 0, deny_count: 0 }
    }

    pub fn check(&mut self, requested: u64) -> bool {
        self.match_count += 1;
        if self.allowed_access.has(requested) { true }
        else { self.deny_count += 1; false }
    }
}

/// Landlock ruleset
#[derive(Debug)]
pub struct LandlockV2Ruleset {
    pub id: u64,
    pub handled_fs: LandlockFsFlags,
    pub rules: Vec<LandlockV2Rule>,
    pub enforced: bool,
    pub created_at: u64,
    pub total_checks: u64,
    pub total_denials: u64,
}

impl LandlockV2Ruleset {
    pub fn new(id: u64, handled_fs: LandlockFsFlags, now: u64) -> Self {
        Self { id, handled_fs, rules: Vec::new(), enforced: false, created_at: now, total_checks: 0, total_denials: 0 }
    }

    pub fn add_rule(&mut self, rule: LandlockV2Rule) { self.rules.push(rule); }
    pub fn enforce(&mut self) { self.enforced = true; }

    pub fn check_access(&mut self, requested: u64) -> bool {
        if !self.enforced { return true; }
        self.total_checks += 1;
        if !self.handled_fs.has(requested) { return true; }
        for rule in &mut self.rules {
            if rule.check(requested) { return true; }
        }
        self.total_denials += 1;
        false
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct LandlockV2BridgeStats {
    pub total_rulesets: u32,
    pub enforced_rulesets: u32,
    pub total_rules: u32,
    pub total_checks: u64,
    pub total_denials: u64,
    pub denial_rate: f64,
}

/// Main Landlock v2 bridge
pub struct BridgeLandlockV2 {
    rulesets: BTreeMap<u64, LandlockV2Ruleset>,
    next_id: u64,
    next_rule_id: u64,
}

impl BridgeLandlockV2 {
    pub fn new() -> Self { Self { rulesets: BTreeMap::new(), next_id: 1, next_rule_id: 1 } }

    pub fn create_ruleset(&mut self, handled_fs: LandlockFsFlags, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.rulesets.insert(id, LandlockV2Ruleset::new(id, handled_fs, now));
        id
    }

    pub fn add_path_rule(&mut self, ruleset: u64, parent_fd: i32, access: LandlockFsFlags) {
        let rid = self.next_rule_id; self.next_rule_id += 1;
        if let Some(rs) = self.rulesets.get_mut(&ruleset) {
            rs.add_rule(LandlockV2Rule::path_beneath(rid, parent_fd, access));
        }
    }

    pub fn enforce(&mut self, ruleset: u64) {
        if let Some(rs) = self.rulesets.get_mut(&ruleset) { rs.enforce(); }
    }

    pub fn stats(&self) -> LandlockV2BridgeStats {
        let enforced = self.rulesets.values().filter(|r| r.enforced).count() as u32;
        let rules: u32 = self.rulesets.values().map(|r| r.rules.len() as u32).sum();
        let checks: u64 = self.rulesets.values().map(|r| r.total_checks).sum();
        let denials: u64 = self.rulesets.values().map(|r| r.total_denials).sum();
        let rate = if checks == 0 { 0.0 } else { denials as f64 / checks as f64 };
        LandlockV2BridgeStats {
            total_rulesets: self.rulesets.len() as u32, enforced_rulesets: enforced,
            total_rules: rules, total_checks: checks, total_denials: denials, denial_rate: rate,
        }
    }
}

// ============================================================================
// Merged from landlock_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV3RuleType {
    PathBeneath,
    Net,
}

/// Landlock v3 FS access
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV3FsAccess {
    Execute,
    WriteFile,
    ReadFile,
    ReadDir,
    RemoveDir,
    RemoveFile,
    MakeChar,
    MakeDir,
    MakeReg,
    MakeSock,
    MakeFifo,
    MakeBlock,
    MakeSym,
    Refer,
    Truncate,
    IoctlDev,
}

/// Landlock v3 net access
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV3NetAccess {
    BindTcp,
    ConnectTcp,
}

/// Rule v3
#[derive(Debug)]
pub struct LandlockV3Rule {
    pub rule_type: LandlockV3RuleType,
    pub path_hash: u64,
    pub allowed_fs: u64,
    pub allowed_net: u32,
    pub port: u16,
}

/// Ruleset v3
#[derive(Debug)]
pub struct LandlockV3Ruleset {
    pub id: u64,
    pub abi_version: u32,
    pub rules: Vec<LandlockV3Rule>,
    pub enforced: bool,
    pub denials: u64,
    pub allows: u64,
}

impl LandlockV3Ruleset {
    pub fn new(id: u64, abi: u32) -> Self {
        Self { id, abi_version: abi, rules: Vec::new(), enforced: false, denials: 0, allows: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct LandlockV3BridgeStats {
    pub total_rulesets: u32,
    pub enforced: u32,
    pub total_rules: u32,
    pub total_denials: u64,
    pub total_allows: u64,
}

/// Main bridge Landlock v3
pub struct BridgeLandlockV3 {
    rulesets: BTreeMap<u64, LandlockV3Ruleset>,
    next_id: u64,
}

impl BridgeLandlockV3 {
    pub fn new() -> Self { Self { rulesets: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, abi: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.rulesets.insert(id, LandlockV3Ruleset::new(id, abi));
        id
    }

    pub fn add_fs_rule(&mut self, rs_id: u64, path_hash: u64, allowed: u64) {
        if let Some(rs) = self.rulesets.get_mut(&rs_id) {
            rs.rules.push(LandlockV3Rule { rule_type: LandlockV3RuleType::PathBeneath, path_hash, allowed_fs: allowed, allowed_net: 0, port: 0 });
        }
    }

    pub fn add_net_rule(&mut self, rs_id: u64, allowed: u32, port: u16) {
        if let Some(rs) = self.rulesets.get_mut(&rs_id) {
            rs.rules.push(LandlockV3Rule { rule_type: LandlockV3RuleType::Net, path_hash: 0, allowed_fs: 0, allowed_net: allowed, port });
        }
    }

    pub fn enforce(&mut self, rs_id: u64) {
        if let Some(rs) = self.rulesets.get_mut(&rs_id) { rs.enforced = true; }
    }

    pub fn stats(&self) -> LandlockV3BridgeStats {
        let enforced = self.rulesets.values().filter(|r| r.enforced).count() as u32;
        let rules: u32 = self.rulesets.values().map(|r| r.rules.len() as u32).sum();
        let denials: u64 = self.rulesets.values().map(|r| r.denials).sum();
        let allows: u64 = self.rulesets.values().map(|r| r.allows).sum();
        LandlockV3BridgeStats { total_rulesets: self.rulesets.len() as u32, enforced, total_rules: rules, total_denials: denials, total_allows: allows }
    }
}

// ============================================================================
// Merged from landlock_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV4FsAccess {
    Execute,
    WriteFile,
    ReadFile,
    ReadDir,
    RemoveDir,
    RemoveFile,
    MakeChar,
    MakeDir,
    MakeReg,
    MakeSock,
    MakeFifo,
    MakeBlock,
    MakeSym,
    Refer,
    Truncate,
    IoctlDev,
}

/// Landlock V4 network access rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV4NetAccess {
    BindTcp,
    ConnectTcp,
    ListenTcp,
    BindUdp,
    ConnectUdp,
    SendTo,
    RecvFrom,
}

/// Rule type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV4RuleType {
    PathBeneath,
    NetPort,
}

/// A filesystem rule.
#[derive(Debug, Clone)]
pub struct LandlockV4FsRule {
    pub rule_id: u64,
    pub parent_fd: i32,
    pub allowed_access: Vec<LandlockV4FsAccess>,
}

impl LandlockV4FsRule {
    pub fn new(rule_id: u64, parent_fd: i32) -> Self {
        Self {
            rule_id,
            parent_fd,
            allowed_access: Vec::new(),
        }
    }

    pub fn allow(&mut self, access: LandlockV4FsAccess) {
        if !self.allowed_access.contains(&access) {
            self.allowed_access.push(access);
        }
    }

    pub fn check(&self, access: LandlockV4FsAccess) -> bool {
        self.allowed_access.contains(&access)
    }
}

/// A network rule.
#[derive(Debug, Clone)]
pub struct LandlockV4NetRule {
    pub rule_id: u64,
    pub port: u16,
    pub allowed_access: Vec<LandlockV4NetAccess>,
}

impl LandlockV4NetRule {
    pub fn new(rule_id: u64, port: u16) -> Self {
        Self {
            rule_id,
            port,
            allowed_access: Vec::new(),
        }
    }

    pub fn allow(&mut self, access: LandlockV4NetAccess) {
        if !self.allowed_access.contains(&access) {
            self.allowed_access.push(access);
        }
    }

    pub fn check(&self, access: LandlockV4NetAccess) -> bool {
        self.allowed_access.contains(&access)
    }
}

/// A Landlock ruleset (domain).
#[derive(Debug, Clone)]
pub struct LandlockV4Ruleset {
    pub ruleset_id: u64,
    pub name: String,
    pub fs_rules: Vec<LandlockV4FsRule>,
    pub net_rules: Vec<LandlockV4NetRule>,
    pub handled_fs_access: Vec<LandlockV4FsAccess>,
    pub handled_net_access: Vec<LandlockV4NetAccess>,
    pub is_enforced: bool,
    pub parent_ruleset: Option<u64>,
    pub deny_count: u64,
    pub allow_count: u64,
}

impl LandlockV4Ruleset {
    pub fn new(ruleset_id: u64, name: String) -> Self {
        Self {
            ruleset_id,
            name,
            fs_rules: Vec::new(),
            net_rules: Vec::new(),
            handled_fs_access: Vec::new(),
            handled_net_access: Vec::new(),
            is_enforced: false,
            parent_ruleset: None,
            deny_count: 0,
            allow_count: 0,
        }
    }

    pub fn add_fs_rule(&mut self, rule: LandlockV4FsRule) {
        self.fs_rules.push(rule);
    }

    pub fn add_net_rule(&mut self, rule: LandlockV4NetRule) {
        self.net_rules.push(rule);
    }

    pub fn check_fs_access(&mut self, access: LandlockV4FsAccess) -> bool {
        if !self.handled_fs_access.contains(&access) {
            self.allow_count += 1;
            return true; // Not handled = allowed
        }
        for rule in &self.fs_rules {
            if rule.check(access) {
                self.allow_count += 1;
                return true;
            }
        }
        self.deny_count += 1;
        false
    }

    pub fn check_net_access(&mut self, access: LandlockV4NetAccess, port: u16) -> bool {
        if !self.handled_net_access.contains(&access) {
            self.allow_count += 1;
            return true;
        }
        for rule in &self.net_rules {
            if rule.port == port && rule.check(access) {
                self.allow_count += 1;
                return true;
            }
        }
        self.deny_count += 1;
        false
    }

    pub fn enforce(&mut self) {
        self.is_enforced = true;
    }
}

/// Statistics for Landlock V4 bridge.
#[derive(Debug, Clone)]
pub struct LandlockV4BridgeStats {
    pub total_rulesets: u64,
    pub total_fs_rules: u64,
    pub total_net_rules: u64,
    pub total_denials: u64,
    pub total_allows: u64,
    pub enforced_domains: u64,
    pub stacked_domains: u64,
}

/// Main bridge Landlock V4 manager.
pub struct BridgeLandlockV4 {
    pub rulesets: BTreeMap<u64, LandlockV4Ruleset>,
    pub process_domains: BTreeMap<u64, Vec<u64>>, // pid → stacked ruleset ids
    pub next_ruleset_id: u64,
    pub next_rule_id: u64,
    pub stats: LandlockV4BridgeStats,
}

impl BridgeLandlockV4 {
    pub fn new() -> Self {
        Self {
            rulesets: BTreeMap::new(),
            process_domains: BTreeMap::new(),
            next_ruleset_id: 1,
            next_rule_id: 1,
            stats: LandlockV4BridgeStats {
                total_rulesets: 0,
                total_fs_rules: 0,
                total_net_rules: 0,
                total_denials: 0,
                total_allows: 0,
                enforced_domains: 0,
                stacked_domains: 0,
            },
        }
    }

    pub fn create_ruleset(&mut self, name: String) -> u64 {
        let id = self.next_ruleset_id;
        self.next_ruleset_id += 1;
        let rs = LandlockV4Ruleset::new(id, name);
        self.rulesets.insert(id, rs);
        self.stats.total_rulesets += 1;
        id
    }

    pub fn add_fs_rule(
        &mut self,
        ruleset_id: u64,
        parent_fd: i32,
        access: Vec<LandlockV4FsAccess>,
    ) -> Option<u64> {
        let rule_id = self.next_rule_id;
        self.next_rule_id += 1;
        if let Some(rs) = self.rulesets.get_mut(&ruleset_id) {
            let mut rule = LandlockV4FsRule::new(rule_id, parent_fd);
            for a in access {
                rule.allow(a);
            }
            rs.add_fs_rule(rule);
            self.stats.total_fs_rules += 1;
            Some(rule_id)
        } else {
            None
        }
    }

    pub fn add_net_rule(
        &mut self,
        ruleset_id: u64,
        port: u16,
        access: Vec<LandlockV4NetAccess>,
    ) -> Option<u64> {
        let rule_id = self.next_rule_id;
        self.next_rule_id += 1;
        if let Some(rs) = self.rulesets.get_mut(&ruleset_id) {
            let mut rule = LandlockV4NetRule::new(rule_id, port);
            for a in access {
                rule.allow(a);
            }
            rs.add_net_rule(rule);
            self.stats.total_net_rules += 1;
            Some(rule_id)
        } else {
            None
        }
    }

    pub fn enforce_on_process(&mut self, pid: u64, ruleset_id: u64) -> bool {
        if let Some(rs) = self.rulesets.get_mut(&ruleset_id) {
            rs.enforce();
            self.stats.enforced_domains += 1;
            let domains = self.process_domains.entry(pid).or_insert_with(Vec::new);
            domains.push(ruleset_id);
            if domains.len() > 1 {
                self.stats.stacked_domains += 1;
            }
            true
        } else {
            false
        }
    }

    pub fn ruleset_count(&self) -> usize {
        self.rulesets.len()
    }
}

// ============================================================================
// Merged from landlock_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV5Access {
    FsExecute,
    FsWriteFile,
    FsReadFile,
    FsReadDir,
    FsRemoveDir,
    FsRemoveFile,
    FsMakeChar,
    FsMakeDir,
    FsMakeReg,
    FsMakeSock,
    FsMakeFifo,
    FsMakeBlock,
    FsMakeSym,
    FsTruncate,
    NetBindTcp,
    NetConnectTcp,
}

/// Landlock v5 operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV5Op {
    CreateRuleset,
    AddRule,
    RestrictSelf,
    Check,
}

/// Landlock v5 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockV5Result {
    Allowed,
    Denied,
    RulesetCreated,
    RuleAdded,
    Restricted,
    Error,
}

/// Landlock v5 record
#[derive(Debug, Clone)]
pub struct LandlockV5Record {
    pub op: LandlockV5Op,
    pub result: LandlockV5Result,
    pub access_mask: u64,
    pub path_hash: u64,
    pub port: u16,
    pub pid: u32,
}

impl LandlockV5Record {
    pub fn new(op: LandlockV5Op) -> Self {
        Self { op, result: LandlockV5Result::Allowed, access_mask: 0, path_hash: 0, port: 0, pid: 0 }
    }
}

/// Landlock v5 bridge stats
#[derive(Debug, Clone)]
pub struct LandlockV5BridgeStats {
    pub total_ops: u64,
    pub rulesets_created: u64,
    pub rules_added: u64,
    pub restrictions: u64,
    pub denials: u64,
    pub errors: u64,
}

/// Main bridge Landlock v5
#[derive(Debug)]
pub struct BridgeLandlockV5 {
    pub stats: LandlockV5BridgeStats,
}

impl BridgeLandlockV5 {
    pub fn new() -> Self {
        Self { stats: LandlockV5BridgeStats { total_ops: 0, rulesets_created: 0, rules_added: 0, restrictions: 0, denials: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &LandlockV5Record) {
        self.stats.total_ops += 1;
        match rec.op {
            LandlockV5Op::CreateRuleset => self.stats.rulesets_created += 1,
            LandlockV5Op::AddRule => self.stats.rules_added += 1,
            LandlockV5Op::RestrictSelf => self.stats.restrictions += 1,
            _ => {}
        }
        if rec.result == LandlockV5Result::Denied { self.stats.denials += 1; }
        if rec.result == LandlockV5Result::Error { self.stats.errors += 1; }
    }
}
