//! # Bridge Mount Bridge
//!
//! Mount/unmount/pivot_root syscall bridging:
//! - Mount namespace management
//! - Mount point tree tracking
//! - Propagation type handling (shared, private, slave, unbindable)
//! - Filesystem type registration
//! - Mount option parsing and validation
//! - Bind mount and overlay tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Mount propagation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountPropagation {
    Private,
    Shared,
    Slave,
    Unbindable,
}

/// Mount flags
#[derive(Debug, Clone, Copy)]
pub struct MountFlags {
    pub bits: u64,
}

impl MountFlags {
    pub const RDONLY: u64 = 1;
    pub const NOSUID: u64 = 2;
    pub const NODEV: u64 = 4;
    pub const NOEXEC: u64 = 8;
    pub const SYNCHRONOUS: u64 = 16;
    pub const REMOUNT: u64 = 32;
    pub const MANDLOCK: u64 = 64;
    pub const NOATIME: u64 = 1024;
    pub const NODIRATIME: u64 = 2048;
    pub const BIND: u64 = 4096;
    pub const MOVE: u64 = 8192;
    pub const SILENT: u64 = 32768;
    pub const LAZYTIME: u64 = 1 << 25;

    pub fn empty() -> Self { Self { bits: 0 } }
    pub fn new(bits: u64) -> Self { Self { bits } }
    pub fn has(&self, flag: u64) -> bool { self.bits & flag != 0 }
    pub fn is_readonly(&self) -> bool { self.has(Self::RDONLY) }
    pub fn is_bind(&self) -> bool { self.has(Self::BIND) }
    pub fn is_remount(&self) -> bool { self.has(Self::REMOUNT) }
}

/// Filesystem type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsType {
    Ext4,
    Btrfs,
    Xfs,
    Tmpfs,
    Proc,
    Sysfs,
    Devtmpfs,
    Cgroup2,
    Overlay,
    NFS,
    Fuse,
    Other,
}

/// Mount point entry
#[derive(Debug, Clone)]
pub struct MountPoint {
    pub mount_id: u64,
    pub parent_id: u64,
    pub device: String,
    pub mount_path: String,
    pub fs_type: FsType,
    pub flags: MountFlags,
    pub propagation: MountPropagation,
    pub peer_group: u32,
    pub ns_id: u64,
    pub created_ts: u64,
    pub access_count: u64,
    pub children: Vec<u64>,
}

impl MountPoint {
    pub fn new(id: u64, parent: u64, device: String, path: String, fs_type: FsType, ts: u64) -> Self {
        Self {
            mount_id: id, parent_id: parent, device, mount_path: path,
            fs_type, flags: MountFlags::empty(), propagation: MountPropagation::Private,
            peer_group: 0, ns_id: 0, created_ts: ts, access_count: 0,
            children: Vec::new(),
        }
    }

    pub fn is_readonly(&self) -> bool { self.flags.is_readonly() }
    pub fn is_virtual(&self) -> bool { matches!(self.fs_type, FsType::Proc | FsType::Sysfs | FsType::Tmpfs | FsType::Devtmpfs | FsType::Cgroup2) }
}

/// Mount event
#[derive(Debug, Clone)]
pub struct MountEvent {
    pub event_type: MountEventType,
    pub mount_id: u64,
    pub path: String,
    pub pid: u64,
    pub timestamp: u64,
    pub success: bool,
}

/// Mount event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountEventType {
    Mount,
    Unmount,
    Remount,
    Bind,
    Move,
    PivotRoot,
}

/// Mount namespace
#[derive(Debug, Clone)]
pub struct MountNamespace {
    pub ns_id: u64,
    pub root_mount: u64,
    pub mount_count: u32,
    pub owner_pid: u64,
}

impl MountNamespace {
    pub fn new(id: u64, root: u64, owner: u64) -> Self {
        Self { ns_id: id, root_mount: root, mount_count: 1, owner_pid: owner }
    }
}

/// Mount bridge stats
#[derive(Debug, Clone, Default)]
pub struct MountBridgeStats {
    pub total_mounts: usize,
    pub virtual_mounts: usize,
    pub readonly_mounts: usize,
    pub bind_mounts: usize,
    pub namespaces: usize,
    pub total_mount_ops: u64,
    pub total_unmount_ops: u64,
    pub overlay_mounts: usize,
}

/// Bridge mount manager
pub struct BridgeMountBridge {
    mounts: BTreeMap<u64, MountPoint>,
    namespaces: BTreeMap<u64, MountNamespace>,
    events: Vec<MountEvent>,
    max_events: usize,
    next_mount_id: u64,
    next_ns_id: u64,
    stats: MountBridgeStats,
}

impl BridgeMountBridge {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(), namespaces: BTreeMap::new(),
            events: Vec::new(), max_events: 512,
            next_mount_id: 1, next_ns_id: 1,
            stats: MountBridgeStats::default(),
        }
    }

    pub fn mount(&mut self, parent: u64, device: String, path: String, fs_type: FsType, flags: MountFlags, pid: u64, ts: u64) -> u64 {
        let id = self.next_mount_id;
        self.next_mount_id += 1;
        let mut mp = MountPoint::new(id, parent, device, path.clone(), fs_type, ts);
        mp.flags = flags;
        self.mounts.insert(id, mp);

        if let Some(p) = self.mounts.get_mut(&parent) {
            p.children.push(id);
        }

        self.events.push(MountEvent {
            event_type: if flags.is_bind() { MountEventType::Bind } else { MountEventType::Mount },
            mount_id: id, path, pid, timestamp: ts, success: true,
        });
        if self.events.len() > self.max_events { self.events.remove(0); }
        id
    }

    pub fn unmount(&mut self, mount_id: u64, pid: u64, ts: u64) -> bool {
        if let Some(mp) = self.mounts.get(&mount_id) {
            if !mp.children.is_empty() { return false; } // busy
            let path = mp.mount_path.clone();
            let parent = mp.parent_id;
            self.mounts.remove(&mount_id);
            if let Some(p) = self.mounts.get_mut(&parent) {
                p.children.retain(|&c| c != mount_id);
            }
            self.events.push(MountEvent { event_type: MountEventType::Unmount, mount_id, path, pid, timestamp: ts, success: true });
            true
        } else { false }
    }

    pub fn remount(&mut self, mount_id: u64, flags: MountFlags, pid: u64, ts: u64) {
        if let Some(mp) = self.mounts.get_mut(&mount_id) {
            let path = mp.mount_path.clone();
            mp.flags = flags;
            self.events.push(MountEvent { event_type: MountEventType::Remount, mount_id, path, pid, timestamp: ts, success: true });
        }
    }

    pub fn set_propagation(&mut self, mount_id: u64, prop: MountPropagation) {
        if let Some(mp) = self.mounts.get_mut(&mount_id) { mp.propagation = prop; }
    }

    pub fn create_namespace(&mut self, root_mount: u64, owner: u64) -> u64 {
        let id = self.next_ns_id;
        self.next_ns_id += 1;
        self.namespaces.insert(id, MountNamespace::new(id, root_mount, owner));
        id
    }

    pub fn find_mount_by_path(&self, path: &str) -> Option<&MountPoint> {
        self.mounts.values().find(|m| m.mount_path == path)
    }

    pub fn recompute(&mut self) {
        self.stats.total_mounts = self.mounts.len();
        self.stats.virtual_mounts = self.mounts.values().filter(|m| m.is_virtual()).count();
        self.stats.readonly_mounts = self.mounts.values().filter(|m| m.is_readonly()).count();
        self.stats.bind_mounts = self.mounts.values().filter(|m| m.flags.is_bind()).count();
        self.stats.namespaces = self.namespaces.len();
        self.stats.total_mount_ops = self.events.iter().filter(|e| matches!(e.event_type, MountEventType::Mount | MountEventType::Bind)).count() as u64;
        self.stats.total_unmount_ops = self.events.iter().filter(|e| e.event_type == MountEventType::Unmount).count() as u64;
        self.stats.overlay_mounts = self.mounts.values().filter(|m| m.fs_type == FsType::Overlay).count();
    }

    pub fn mount_point(&self, id: u64) -> Option<&MountPoint> { self.mounts.get(&id) }
    pub fn stats(&self) -> &MountBridgeStats { &self.stats }
}

// ============================================================================
// Merged from mount_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountV2Propagation {
    Private,
    Shared,
    Slave,
    Unbindable,
}

/// Mount V2 flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountV2Flag {
    ReadOnly,
    NoSuid,
    NoDev,
    NoExec,
    Synchronous,
    MandLock,
    DirSync,
    NoAtime,
    NoDirAtime,
    Relatime,
    StrictAtime,
    LazyTime,
    IdMapped,
}

/// Filesystem type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountV2FsType {
    Ext4,
    Btrfs,
    Xfs,
    Tmpfs,
    Proc,
    Sysfs,
    Devtmpfs,
    Cgroup2,
    Fuse,
    Nfs,
    Overlay,
    Squashfs,
}

/// ID mapping entry for mapped mounts.
#[derive(Debug, Clone)]
pub struct MountV2IdMap {
    pub lower_id: u32,
    pub upper_id: u32,
    pub count: u32,
}

impl MountV2IdMap {
    pub fn new(lower_id: u32, upper_id: u32, count: u32) -> Self {
        Self {
            lower_id,
            upper_id,
            count,
        }
    }

    pub fn contains_lower(&self, id: u32) -> bool {
        id >= self.lower_id && id < self.lower_id + self.count
    }

    pub fn translate(&self, lower: u32) -> Option<u32> {
        if self.contains_lower(lower) {
            Some(self.upper_id + (lower - self.lower_id))
        } else {
            None
        }
    }
}

/// A mount point entry.
#[derive(Debug, Clone)]
pub struct MountV2Entry {
    pub mount_id: u64,
    pub parent_id: Option<u64>,
    pub source: String,
    pub target: String,
    pub fs_type: MountV2FsType,
    pub flags: Vec<MountV2Flag>,
    pub propagation: MountV2Propagation,
    pub uid_mappings: Vec<MountV2IdMap>,
    pub gid_mappings: Vec<MountV2IdMap>,
    pub namespace_id: u64,
    pub is_idmapped: bool,
    pub children: Vec<u64>,
    pub peer_group: Option<u64>,
}

impl MountV2Entry {
    pub fn new(mount_id: u64, source: String, target: String, fs_type: MountV2FsType) -> Self {
        Self {
            mount_id,
            parent_id: None,
            source,
            target,
            fs_type,
            flags: Vec::new(),
            propagation: MountV2Propagation::Private,
            uid_mappings: Vec::new(),
            gid_mappings: Vec::new(),
            namespace_id: 0,
            is_idmapped: false,
            children: Vec::new(),
            peer_group: None,
        }
    }

    pub fn set_idmapped(&mut self, uid_map: MountV2IdMap, gid_map: MountV2IdMap) {
        self.uid_mappings.push(uid_map);
        self.gid_mappings.push(gid_map);
        self.is_idmapped = true;
        if !self.flags.contains(&MountV2Flag::IdMapped) {
            self.flags.push(MountV2Flag::IdMapped);
        }
    }

    pub fn translate_uid(&self, uid: u32) -> u32 {
        for map in &self.uid_mappings {
            if let Some(mapped) = map.translate(uid) {
                return mapped;
            }
        }
        uid
    }

    pub fn translate_gid(&self, gid: u32) -> u32 {
        for map in &self.gid_mappings {
            if let Some(mapped) = map.translate(gid) {
                return mapped;
            }
        }
        gid
    }

    pub fn is_read_only(&self) -> bool {
        self.flags.contains(&MountV2Flag::ReadOnly)
    }
}

/// Filesystem context for fsmount-style mount creation.
#[derive(Debug, Clone)]
pub struct MountV2FsContext {
    pub ctx_id: u64,
    pub fs_type: MountV2FsType,
    pub source: Option<String>,
    pub options: BTreeMap<String, String>,
    pub is_configured: bool,
}

impl MountV2FsContext {
    pub fn new(ctx_id: u64, fs_type: MountV2FsType) -> Self {
        Self {
            ctx_id,
            fs_type,
            source: None,
            options: BTreeMap::new(),
            is_configured: false,
        }
    }

    pub fn set_option(&mut self, key: String, value: String) {
        self.options.insert(key, value);
    }

    pub fn finalize(&mut self) {
        self.is_configured = true;
    }
}

/// Statistics for mount V2 bridge.
#[derive(Debug, Clone)]
pub struct MountV2BridgeStats {
    pub total_mounts: u64,
    pub total_unmounts: u64,
    pub idmapped_mounts: u64,
    pub shared_mounts: u64,
    pub bind_mounts: u64,
    pub move_mounts: u64,
    pub fs_contexts_created: u64,
    pub namespace_clones: u64,
}

/// Main bridge mount V2 manager.
pub struct BridgeMountV2 {
    pub mounts: BTreeMap<u64, MountV2Entry>,
    pub fs_contexts: BTreeMap<u64, MountV2FsContext>,
    pub next_mount_id: u64,
    pub next_ctx_id: u64,
    pub stats: MountV2BridgeStats,
}

impl BridgeMountV2 {
    pub fn new() -> Self {
        Self {
            mounts: BTreeMap::new(),
            fs_contexts: BTreeMap::new(),
            next_mount_id: 1,
            next_ctx_id: 1,
            stats: MountV2BridgeStats {
                total_mounts: 0,
                total_unmounts: 0,
                idmapped_mounts: 0,
                shared_mounts: 0,
                bind_mounts: 0,
                move_mounts: 0,
                fs_contexts_created: 0,
                namespace_clones: 0,
            },
        }
    }

    pub fn create_mount(
        &mut self,
        source: String,
        target: String,
        fs_type: MountV2FsType,
    ) -> u64 {
        let id = self.next_mount_id;
        self.next_mount_id += 1;
        let entry = MountV2Entry::new(id, source, target, fs_type);
        self.mounts.insert(id, entry);
        self.stats.total_mounts += 1;
        id
    }

    pub fn create_fs_context(&mut self, fs_type: MountV2FsType) -> u64 {
        let id = self.next_ctx_id;
        self.next_ctx_id += 1;
        let ctx = MountV2FsContext::new(id, fs_type);
        self.fs_contexts.insert(id, ctx);
        self.stats.fs_contexts_created += 1;
        id
    }

    pub fn set_idmapped(
        &mut self,
        mount_id: u64,
        uid_lower: u32,
        uid_upper: u32,
        gid_lower: u32,
        gid_upper: u32,
        count: u32,
    ) -> bool {
        if let Some(entry) = self.mounts.get_mut(&mount_id) {
            let uid_map = MountV2IdMap::new(uid_lower, uid_upper, count);
            let gid_map = MountV2IdMap::new(gid_lower, gid_upper, count);
            entry.set_idmapped(uid_map, gid_map);
            self.stats.idmapped_mounts += 1;
            true
        } else {
            false
        }
    }

    pub fn mount_count(&self) -> usize {
        self.mounts.len()
    }

    pub fn context_count(&self) -> usize {
        self.fs_contexts.len()
    }
}

// ============================================================================
// Merged from mount_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountV3BridgeOp {
    Mount,
    Umount,
    Remount,
    Bind,
    Move,
    MountAt,
    FsOpen,
    FsConfig,
    FsMount,
    MoveMount,
    IdmapMount,
    OpenTree,
}

/// Mount v3 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountV3Result {
    Success,
    PermissionDenied,
    Busy,
    NotFound,
    InvalidFs,
    NotMountPoint,
    AlreadyMounted,
    NsConflict,
    Error,
}

/// Mount v3 bridge record
#[derive(Debug, Clone)]
pub struct MountV3BridgeRecord {
    pub op: MountV3BridgeOp,
    pub result: MountV3Result,
    pub source_hash: u64,
    pub target_hash: u64,
    pub fs_type_hash: u64,
    pub flags: u64,
    pub ns_id: u64,
    pub idmap_ns: u64,
    pub duration_ns: u64,
}

impl MountV3BridgeRecord {
    pub fn new(op: MountV3BridgeOp, source: &[u8], target: &[u8]) -> Self {
        let hash = |d: &[u8]| -> u64 {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in d { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            h
        };
        Self { op, result: MountV3Result::Success, source_hash: hash(source), target_hash: hash(target), fs_type_hash: 0, flags: 0, ns_id: 0, idmap_ns: 0, duration_ns: 0 }
    }

    pub fn is_idmapped(&self) -> bool { self.idmap_ns != 0 }
}

/// Mount v3 bridge stats
#[derive(Debug, Clone)]
pub struct MountV3BridgeStats {
    pub total_ops: u64,
    pub mounts: u64,
    pub umounts: u64,
    pub idmap_mounts: u64,
    pub failures: u64,
}

/// Main bridge mount v3
#[derive(Debug)]
pub struct BridgeMountV3 {
    pub stats: MountV3BridgeStats,
}

impl BridgeMountV3 {
    pub fn new() -> Self {
        Self { stats: MountV3BridgeStats { total_ops: 0, mounts: 0, umounts: 0, idmap_mounts: 0, failures: 0 } }
    }

    pub fn record(&mut self, rec: &MountV3BridgeRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            MountV3BridgeOp::Mount | MountV3BridgeOp::Bind | MountV3BridgeOp::FsMount => self.stats.mounts += 1,
            MountV3BridgeOp::Umount => self.stats.umounts += 1,
            MountV3BridgeOp::IdmapMount => { self.stats.mounts += 1; self.stats.idmap_mounts += 1; }
            _ => {}
        }
        if rec.result != MountV3Result::Success { self.stats.failures += 1; }
    }
}

// ============================================================================
// Merged from mount_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountV4Event { Mount, Unmount, Remount, BindMount, MoveMount, Pivot }

/// Mount v4 record
#[derive(Debug, Clone)]
pub struct MountV4Record {
    pub event: MountV4Event,
    pub fs_type_hash: u64,
    pub mount_id: u32,
    pub flags: u32,
    pub propagation: u32,
}

impl MountV4Record {
    pub fn new(event: MountV4Event) -> Self { Self { event, fs_type_hash: 0, mount_id: 0, flags: 0, propagation: 0 } }
}

/// Mount v4 bridge stats
#[derive(Debug, Clone)]
pub struct MountV4BridgeStats { pub total_events: u64, pub mounts: u64, pub unmounts: u64, pub binds: u64 }

/// Main bridge mount v4
#[derive(Debug)]
pub struct BridgeMountV4 { pub stats: MountV4BridgeStats }

impl BridgeMountV4 {
    pub fn new() -> Self { Self { stats: MountV4BridgeStats { total_events: 0, mounts: 0, unmounts: 0, binds: 0 } } }
    pub fn record(&mut self, rec: &MountV4Record) {
        self.stats.total_events += 1;
        match rec.event {
            MountV4Event::Mount | MountV4Event::Pivot => self.stats.mounts += 1,
            MountV4Event::Unmount => self.stats.unmounts += 1,
            MountV4Event::BindMount | MountV4Event::MoveMount => self.stats.binds += 1,
            _ => {}
        }
    }
}
