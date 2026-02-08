//! # Bridge Module Bridge
//!
//! Bridges kernel module operations:
//! - Module loading/unloading coordination
//! - Symbol resolution and dependency tracking
//! - Module parameter management
//! - Version compatibility checks
//! - Module reference counting

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Live,
    Coming,
    Going,
    Unformed,
    Built,
}

/// Module taint flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleTaint {
    Proprietary,
    ForcedLoad,
    ForcedUnload,
    Staging,
    OutOfTree,
    Unsigned,
    LivePatch,
}

/// Module parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    Bool,
    Int,
    Uint,
    Long,
    Ulong,
    Charp,
    Array,
}

/// Module parameter
#[derive(Debug, Clone)]
pub struct ModuleParam {
    pub name: String,
    pub ptype: ParamType,
    pub value: String,
    pub description: String,
    pub mode: u16,
    pub modified: bool,
}

impl ModuleParam {
    pub fn new(name: String, ptype: ParamType, default: String, desc: String) -> Self {
        Self { name, ptype, value: default, description: desc, mode: 0o644, modified: false }
    }

    pub fn set(&mut self, val: String) { self.value = val; self.modified = true; }
}

/// Exported symbol
#[derive(Debug, Clone)]
pub struct ModuleSymbol {
    pub name: String,
    pub address: u64,
    pub module_id: u64,
    pub is_gpl: bool,
    pub crc: u32,
}

/// Module descriptor
#[derive(Debug, Clone)]
pub struct ModuleDesc {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub state: ModuleState,
    pub size_bytes: u64,
    pub core_size: u64,
    pub init_size: u64,
    pub refcount: u32,
    pub deps: Vec<u64>,
    pub dependents: Vec<u64>,
    pub params: Vec<ModuleParam>,
    pub exported_syms: Vec<String>,
    pub taints: Vec<ModuleTaint>,
    pub loaded_at: u64,
    pub srcversion: String,
}

impl ModuleDesc {
    pub fn new(id: u64, name: String, version: String) -> Self {
        Self {
            id, name, version, state: ModuleState::Unformed, size_bytes: 0,
            core_size: 0, init_size: 0, refcount: 0, deps: Vec::new(),
            dependents: Vec::new(), params: Vec::new(), exported_syms: Vec::new(),
            taints: Vec::new(), loaded_at: 0, srcversion: String::new(),
        }
    }

    pub fn is_live(&self) -> bool { self.state == ModuleState::Live }
    pub fn can_unload(&self) -> bool { self.refcount == 0 && self.dependents.is_empty() && self.state == ModuleState::Live }
    pub fn add_dep(&mut self, dep: u64) { if !self.deps.contains(&dep) { self.deps.push(dep); } }
    pub fn add_taint(&mut self, t: ModuleTaint) { if !self.taints.contains(&t) { self.taints.push(t); } }
}

/// Module load request
#[derive(Debug, Clone)]
pub struct ModuleLoadReq {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub force: bool,
    pub ts: u64,
}

/// Module operation event
#[derive(Debug, Clone)]
pub struct ModuleEvent {
    pub module_id: u64,
    pub kind: ModuleEventKind,
    pub ts: u64,
    pub result: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleEventKind {
    Load,
    Unload,
    Init,
    Exit,
    ParamChange,
    RefAcquire,
    RefRelease,
}

/// Module bridge stats
#[derive(Debug, Clone, Default)]
pub struct ModuleBridgeStats {
    pub total_modules: usize,
    pub live_modules: usize,
    pub total_symbols: usize,
    pub total_loads: u64,
    pub total_unloads: u64,
    pub failed_loads: u64,
    pub tainted: bool,
}

/// Bridge module manager
pub struct BridgeModuleBridge {
    modules: BTreeMap<u64, ModuleDesc>,
    symbols: BTreeMap<String, ModuleSymbol>,
    events: Vec<ModuleEvent>,
    stats: ModuleBridgeStats,
    next_id: u64,
}

impl BridgeModuleBridge {
    pub fn new() -> Self {
        Self { modules: BTreeMap::new(), symbols: BTreeMap::new(), events: Vec::new(), stats: ModuleBridgeStats::default(), next_id: 1 }
    }

    pub fn load_module(&mut self, name: String, version: String, size: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut m = ModuleDesc::new(id, name, version);
        m.size_bytes = size;
        m.loaded_at = ts;
        m.state = ModuleState::Coming;
        self.modules.insert(id, m);
        self.events.push(ModuleEvent { module_id: id, kind: ModuleEventKind::Load, ts, result: 0 });
        id
    }

    pub fn init_complete(&mut self, id: u64, ts: u64) {
        if let Some(m) = self.modules.get_mut(&id) { m.state = ModuleState::Live; }
        self.events.push(ModuleEvent { module_id: id, kind: ModuleEventKind::Init, ts, result: 0 });
    }

    pub fn unload_module(&mut self, id: u64, ts: u64) -> bool {
        let can = self.modules.get(&id).map(|m| m.can_unload()).unwrap_or(false);
        if !can { self.events.push(ModuleEvent { module_id: id, kind: ModuleEventKind::Unload, ts, result: -1 }); return false; }
        if let Some(m) = self.modules.get_mut(&id) { m.state = ModuleState::Going; }
        let syms: Vec<String> = self.symbols.iter().filter(|(_, s)| s.module_id == id).map(|(k, _)| k.clone()).collect();
        for sym in syms { self.symbols.remove(&sym); }
        self.modules.remove(&id);
        self.events.push(ModuleEvent { module_id: id, kind: ModuleEventKind::Unload, ts, result: 0 });
        true
    }

    pub fn export_symbol(&mut self, mod_id: u64, name: String, addr: u64, gpl: bool, crc: u32) {
        let sym = ModuleSymbol { name: name.clone(), address: addr, module_id: mod_id, is_gpl: gpl, crc };
        if let Some(m) = self.modules.get_mut(&mod_id) { m.exported_syms.push(name.clone()); }
        self.symbols.insert(name, sym);
    }

    pub fn resolve_symbol(&self, name: &str) -> Option<&ModuleSymbol> { self.symbols.get(name) }

    pub fn add_dependency(&mut self, mod_id: u64, dep_id: u64) {
        if let Some(m) = self.modules.get_mut(&mod_id) { m.add_dep(dep_id); }
        if let Some(d) = self.modules.get_mut(&dep_id) { d.dependents.push(mod_id); d.refcount += 1; }
    }

    pub fn set_param(&mut self, mod_id: u64, param: &str, val: String, ts: u64) {
        if let Some(m) = self.modules.get_mut(&mod_id) {
            for p in m.params.iter_mut() { if p.name == param { p.set(val); break; } }
        }
        self.events.push(ModuleEvent { module_id: mod_id, kind: ModuleEventKind::ParamChange, ts, result: 0 });
    }

    pub fn recompute(&mut self) {
        self.stats.total_modules = self.modules.len();
        self.stats.live_modules = self.modules.values().filter(|m| m.is_live()).count();
        self.stats.total_symbols = self.symbols.len();
        self.stats.total_loads = self.events.iter().filter(|e| e.kind == ModuleEventKind::Load && e.result == 0).count() as u64;
        self.stats.total_unloads = self.events.iter().filter(|e| e.kind == ModuleEventKind::Unload && e.result == 0).count() as u64;
        self.stats.failed_loads = self.events.iter().filter(|e| e.kind == ModuleEventKind::Load && e.result < 0).count() as u64;
        self.stats.tainted = self.modules.values().any(|m| !m.taints.is_empty());
    }

    pub fn module(&self, id: u64) -> Option<&ModuleDesc> { self.modules.get(&id) }
    pub fn stats(&self) -> &ModuleBridgeStats { &self.stats }
}
