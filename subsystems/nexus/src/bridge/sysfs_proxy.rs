//! # Bridge Sysfs Proxy
//!
//! Bridges sysfs attribute operations between kernel and userspace:
//! - Attribute read/write forwarding
//! - Kobject hierarchy traversal
//! - Uevent relay and filtering
//! - Device attribute caching
//! - Binary attribute support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Sysfs attribute type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Binary,
    Group,
}

/// Sysfs subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysfsSubsystem {
    Block,
    Bus,
    Class,
    Devices,
    Firmware,
    Fs,
    Kernel,
    Module,
    Power,
}

/// Kobject entry
#[derive(Debug, Clone)]
pub struct KObject {
    pub id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub subsystem: SysfsSubsystem,
    pub children: Vec<u64>,
    pub attrs: Vec<SysfsAttr>,
    pub uevent_suppress: bool,
    pub refcount: u32,
}

impl KObject {
    pub fn new(id: u64, name: String, subsys: SysfsSubsystem, parent: Option<u64>) -> Self {
        Self { id, name, parent_id: parent, subsystem: subsys, children: Vec::new(), attrs: Vec::new(), uevent_suppress: false, refcount: 1 }
    }
}

/// Sysfs attribute
#[derive(Debug, Clone)]
pub struct SysfsAttr {
    pub name: String,
    pub attr_type: AttrType,
    pub mode: u16,
    pub value: Vec<u8>,
    pub size: usize,
    pub read_count: u64,
    pub write_count: u64,
}

impl SysfsAttr {
    pub fn new(name: String, atype: AttrType, mode: u16) -> Self {
        Self { name, attr_type: atype, mode, value: Vec::new(), size: 0, read_count: 0, write_count: 0 }
    }

    #[inline]
    pub fn write(&mut self, data: &[u8]) -> bool {
        match self.attr_type {
            AttrType::ReadOnly => false,
            _ => { self.value = data.into(); self.size = data.len(); self.write_count += 1; true }
        }
    }

    #[inline]
    pub fn read(&mut self) -> Option<&[u8]> {
        match self.attr_type {
            AttrType::WriteOnly => None,
            _ => { self.read_count += 1; Some(&self.value) }
        }
    }
}

/// Uevent action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UeventAction {
    Add,
    Remove,
    Change,
    Move,
    Online,
    Offline,
    Bind,
    Unbind,
}

/// Uevent
#[derive(Debug, Clone)]
pub struct Uevent {
    pub kobject_id: u64,
    pub action: UeventAction,
    pub subsystem: SysfsSubsystem,
    pub ts: u64,
    pub env_vars: Vec<(String, String)>,
    pub seq: u64,
}

/// Uevent filter
#[derive(Debug, Clone)]
pub struct UeventFilter {
    pub subsystem: Option<SysfsSubsystem>,
    pub action: Option<UeventAction>,
    pub match_count: u64,
}

/// Sysfs proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SysfsProxyStats {
    pub total_kobjects: usize,
    pub total_attrs: usize,
    pub total_reads: u64,
    pub total_writes: u64,
    pub total_uevents: u64,
    pub filtered_uevents: u64,
}

/// Bridge sysfs proxy
#[repr(align(64))]
pub struct BridgeSysfsProxy {
    kobjects: BTreeMap<u64, KObject>,
    uevents: Vec<Uevent>,
    filters: Vec<UeventFilter>,
    stats: SysfsProxyStats,
    next_id: u64,
    uevent_seq: u64,
}

impl BridgeSysfsProxy {
    pub fn new() -> Self {
        Self { kobjects: BTreeMap::new(), uevents: Vec::new(), filters: Vec::new(), stats: SysfsProxyStats::default(), next_id: 1, uevent_seq: 0 }
    }

    #[inline]
    pub fn create_kobject(&mut self, name: String, subsys: SysfsSubsystem, parent: Option<u64>) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let kobj = KObject::new(id, name, subsys, parent);
        self.kobjects.insert(id, kobj);
        if let Some(pid) = parent { if let Some(p) = self.kobjects.get_mut(&pid) { p.children.push(id); } }
        id
    }

    #[inline]
    pub fn destroy_kobject(&mut self, id: u64) -> bool {
        let has_children = self.kobjects.get(&id).map(|k| !k.children.is_empty()).unwrap_or(true);
        if has_children { return false; }
        let parent = self.kobjects.get(&id).and_then(|k| k.parent_id);
        self.kobjects.remove(&id);
        if let Some(pid) = parent { if let Some(p) = self.kobjects.get_mut(&pid) { p.children.retain(|&c| c != id); } }
        true
    }

    #[inline(always)]
    pub fn add_attr(&mut self, kobj_id: u64, attr: SysfsAttr) {
        if let Some(k) = self.kobjects.get_mut(&kobj_id) { k.attrs.push(attr); }
    }

    #[inline]
    pub fn write_attr(&mut self, kobj_id: u64, attr_name: &str, data: &[u8]) -> bool {
        if let Some(k) = self.kobjects.get_mut(&kobj_id) {
            for a in k.attrs.iter_mut() { if a.name == attr_name { return a.write(data); } }
        }
        false
    }

    #[inline]
    pub fn read_attr(&mut self, kobj_id: u64, attr_name: &str) -> Option<Vec<u8>> {
        if let Some(k) = self.kobjects.get_mut(&kobj_id) {
            for a in k.attrs.iter_mut() { if a.name == attr_name { return a.read().map(|v| v.to_vec()); } }
        }
        None
    }

    pub fn emit_uevent(&mut self, kobj_id: u64, action: UeventAction, ts: u64) {
        let subsys = self.kobjects.get(&kobj_id).map(|k| k.subsystem);
        let suppress = self.kobjects.get(&kobj_id).map(|k| k.uevent_suppress).unwrap_or(true);
        if suppress { return; }
        if let Some(ss) = subsys {
            self.uevent_seq += 1;
            let ev = Uevent { kobject_id: kobj_id, action, subsystem: ss, ts, env_vars: Vec::new(), seq: self.uevent_seq };
            let filtered = self.filters.iter().any(|f| {
                (f.subsystem.is_none() || f.subsystem == Some(ss)) && (f.action.is_none() || f.action == Some(action))
            });
            if !filtered { self.uevents.push(ev); } else { self.stats.filtered_uevents += 1; }
        }
    }

    #[inline(always)]
    pub fn add_filter(&mut self, subsys: Option<SysfsSubsystem>, action: Option<UeventAction>) {
        self.filters.push(UeventFilter { subsystem: subsys, action, match_count: 0 });
    }

    #[inline(always)]
    pub fn children(&self, id: u64) -> Vec<u64> { self.kobjects.get(&id).map(|k| k.children.clone()).unwrap_or_default() }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_kobjects = self.kobjects.len();
        self.stats.total_attrs = self.kobjects.values().map(|k| k.attrs.len()).sum();
        self.stats.total_reads = self.kobjects.values().flat_map(|k| k.attrs.iter()).map(|a| a.read_count).sum();
        self.stats.total_writes = self.kobjects.values().flat_map(|k| k.attrs.iter()).map(|a| a.write_count).sum();
        self.stats.total_uevents = self.uevent_seq;
    }

    #[inline(always)]
    pub fn kobject(&self, id: u64) -> Option<&KObject> { self.kobjects.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &SysfsProxyStats { &self.stats }
}
