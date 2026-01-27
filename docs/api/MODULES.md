# Helix Module System API Reference

<div align="center">

🔧 **Complete Module System Documentation**

*Loading, Registration, Dependencies, and Hot-Reload*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [Module Interface](#2-module-interface)
3. [Module Loader](#3-module-loader)
4. [Module Registry](#4-module-registry)
5. [Dependency Management](#5-dependency-management)
6. [ABI Compatibility](#6-abi-compatibility)
7. [Hot-Reload Integration](#7-hot-reload-integration)
8. [Module Development](#8-module-development)

---

## 1. Overview

### 1.1 Module System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      HELIX MODULE SYSTEM                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Module Lifecycle:                                                          │
│  ════════════════                                                           │
│                                                                             │
│     ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    │
│     │ Binary │───▶│ Loader │───▶│Registry│───▶│  Init  │───▶│ Active │    │
│     │  File  │    │        │    │        │    │        │    │        │    │
│     └────────┘    └────────┘    └────────┘    └────────┘    └────────┘    │
│                                                                             │
│                                       │                                     │
│                                       ▼                                     │
│                                 ┌────────────┐                              │
│                                 │ Hot-Reload │                              │
│                                 │  (Swap)    │                              │
│                                 └────────────┘                              │
│                                                                             │
│  Components:                                                                │
│  ═══════════                                                                │
│                                                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                                                                      │  │
│  │  ┌─────────────────┐     ┌─────────────────┐     ┌───────────────┐  │  │
│  │  │  Module Loader  │     │ Module Registry │     │   Dependency  │  │  │
│  │  │                 │     │                 │     │    Resolver   │  │  │
│  │  │  • ELF parsing  │     │  • Name lookup  │     │               │  │  │
│  │  │  • Relocation   │     │  • Version mgmt │     │  • Graph      │  │  │
│  │  │  • Symbol res.  │     │  • State track  │     │  • Ordering   │  │  │
│  │  └─────────────────┘     └─────────────────┘     └───────────────┘  │  │
│  │                                                                      │  │
│  │  ┌─────────────────┐     ┌─────────────────┐     ┌───────────────┐  │  │
│  │  │   ABI Layer     │     │   Hot-Reload    │     │   Interface   │  │  │
│  │  │                 │     │    Engine       │     │   Checker     │  │  │
│  │  │  • Versioning   │     │                 │     │               │  │  │
│  │  │  • Compat check │     │  • State xfer   │     │  • Validate   │  │  │
│  │  │  • Marshalling  │     │  • Atomic swap  │     │  • Match      │  │  │
│  │  └─────────────────┘     └─────────────────┘     └───────────────┘  │  │
│  │                                                                      │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Crate Structure

```
helix-modules/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs           # Crate root
    ├── interface.rs     # Module interface trait
    ├── loader.rs        # Module loader
    ├── registry.rs      # Module registry
    ├── dependencies.rs  # Dependency management
    ├── abi.rs           # ABI definitions
    ├── hot_reload.rs    # Hot-reload support
    ├── v2.rs            # V2 module system
    └── v2_tests.rs      # Tests
```

---

## 2. Module Interface

### 2.1 Module Trait

```rust
/// Core module interface trait
/// 
/// All modules must implement this trait to be loadable
/// by the Helix module system.
pub trait Module: Send + Sync {
    /// Get module name
    fn name(&self) -> &'static str;
    
    /// Get module version
    fn version(&self) -> Version;
    
    /// Get module description
    fn description(&self) -> &'static str {
        ""
    }
    
    /// Get module author
    fn author(&self) -> &'static str {
        "Unknown"
    }
    
    /// Get module dependencies
    fn dependencies(&self) -> &[Dependency] {
        &[]
    }
    
    /// Get provided interfaces
    fn provides(&self) -> &[InterfaceId] {
        &[]
    }
    
    /// Initialize the module
    fn init(&mut self) -> ModuleResult<()>;
    
    /// Cleanup before unload
    fn cleanup(&mut self) -> ModuleResult<()>;
    
    /// Handle incoming message
    fn handle_message(&mut self, msg: &Message) -> ModuleResult<Option<Message>> {
        Ok(None) // Default: ignore messages
    }
    
    /// Get module capabilities
    fn capabilities(&self) -> ModuleCapabilities {
        ModuleCapabilities::default()
    }
    
    /// Check if module supports hot-reload
    fn supports_hot_reload(&self) -> bool {
        false
    }
    
    /// Serialize state for hot-reload
    fn serialize_state(&self) -> Option<Vec<u8>> {
        None
    }
    
    /// Deserialize state after hot-reload
    fn deserialize_state(&mut self, _state: &[u8]) -> ModuleResult<()> {
        Ok(())
    }
}

/// Module version
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Create new version
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    /// Parse version from string "X.Y.Z"
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
    
    /// Check if compatible with requirement
    pub fn is_compatible(&self, requirement: &VersionRequirement) -> bool {
        match requirement {
            VersionRequirement::Exact(v) => self == v,
            VersionRequirement::Compatible(v) => {
                self.major == v.major && *self >= *v
            }
            VersionRequirement::AtLeast(v) => *self >= *v,
            VersionRequirement::Range { min, max } => *self >= *min && *self <= *max,
        }
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Version requirement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionRequirement {
    /// Exact version match
    Exact(Version),
    /// Compatible version (same major, >= specified)
    Compatible(Version),
    /// At least this version
    AtLeast(Version),
    /// Version range
    Range { min: Version, max: Version },
}

/// Module dependency
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Dependency name
    pub name: &'static str,
    /// Version requirement
    pub version: VersionRequirement,
    /// Is this optional?
    pub optional: bool,
}

impl Dependency {
    /// Create required dependency
    pub const fn required(name: &'static str, version: VersionRequirement) -> Self {
        Self {
            name,
            version,
            optional: false,
        }
    }
    
    /// Create optional dependency
    pub const fn optional(name: &'static str, version: VersionRequirement) -> Self {
        Self {
            name,
            version,
            optional: true,
        }
    }
}

/// Module capabilities
#[derive(Debug, Clone, Default)]
pub struct ModuleCapabilities {
    /// Can handle interrupts
    pub interrupts: bool,
    /// Needs direct memory access
    pub dma: bool,
    /// Needs I/O port access
    pub io_ports: bool,
    /// Provides filesystem
    pub filesystem: bool,
    /// Provides networking
    pub networking: bool,
    /// Provides scheduling
    pub scheduling: bool,
}
```

### 2.2 Interface Definition

```rust
/// Interface identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InterfaceId(u64);

impl InterfaceId {
    /// Create from name hash
    pub const fn from_name(name: &str) -> Self {
        // Simple hash for const context
        let bytes = name.as_bytes();
        let mut hash = 0u64;
        let mut i = 0;
        while i < bytes.len() {
            hash = hash.wrapping_mul(31).wrapping_add(bytes[i] as u64);
            i += 1;
        }
        Self(hash)
    }
}

/// Interface definition
pub struct Interface {
    /// Interface ID
    pub id: InterfaceId,
    
    /// Interface name
    pub name: &'static str,
    
    /// Interface version
    pub version: Version,
    
    /// Required methods
    pub methods: &'static [MethodSignature],
}

/// Method signature
#[derive(Debug, Clone)]
pub struct MethodSignature {
    /// Method name
    pub name: &'static str,
    
    /// Argument types
    pub args: &'static [TypeId],
    
    /// Return type
    pub returns: Option<TypeId>,
}

/// Type identifier for interface methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeId {
    Void,
    Bool,
    U8, U16, U32, U64,
    I8, I16, I32, I64,
    F32, F64,
    Ptr,
    Slice,
    Custom(u64),
}

/// Macro to define interfaces
#[macro_export]
macro_rules! define_interface {
    (
        $name:ident {
            $(fn $method:ident($($arg:ident: $arg_ty:ty),*) $(-> $ret:ty)?;)*
        }
    ) => {
        pub struct $name;
        
        impl $name {
            pub const ID: InterfaceId = InterfaceId::from_name(stringify!($name));
        }
    };
}

// Example interface definitions
define_interface! {
    SchedulerInterface {
        fn add_task(task: Task);
        fn remove_task(id: TaskId) -> bool;
        fn next() -> Option<Task>;
    }
}

define_interface! {
    FilesystemInterface {
        fn open(path: &str, flags: u32) -> Result<FileHandle, Error>;
        fn close(handle: FileHandle) -> Result<(), Error>;
        fn read(handle: FileHandle, buf: &mut [u8]) -> Result<usize, Error>;
        fn write(handle: FileHandle, buf: &[u8]) -> Result<usize, Error>;
    }
}
```

---

## 3. Module Loader

### 3.1 Loader Implementation

```rust
/// Module loader
pub struct ModuleLoader {
    /// Loaded modules
    modules: HashMap<ModuleId, LoadedModule>,
    
    /// Module search paths
    search_paths: Vec<&'static str>,
    
    /// Symbol table
    symbols: HashMap<String, usize>,
}

/// Loaded module representation
struct LoadedModule {
    /// Module ID
    id: ModuleId,
    
    /// Module instance
    instance: Box<dyn Module>,
    
    /// Code region
    code_region: MemoryRegion,
    
    /// Data region
    data_region: Option<MemoryRegion>,
    
    /// Entry point
    entry_point: usize,
    
    /// Exported symbols
    exports: HashMap<String, usize>,
    
    /// Load time
    loaded_at: u64,
}

/// Memory region for module
struct MemoryRegion {
    start: usize,
    size: usize,
    permissions: u32,
}

impl ModuleLoader {
    /// Create new module loader
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            search_paths: vec!["/modules/", "/lib/modules/"],
            symbols: HashMap::new(),
        }
    }
    
    /// Load module from path
    pub fn load(&mut self, path: &str) -> ModuleResult<ModuleId> {
        // Read module binary
        let binary = self.read_module_file(path)?;
        
        // Parse ELF header
        let elf = self.parse_elf(&binary)?;
        
        // Allocate memory for code and data
        let code_region = self.allocate_code_region(elf.code_size)?;
        let data_region = if elf.data_size > 0 {
            Some(self.allocate_data_region(elf.data_size)?)
        } else {
            None
        };
        
        // Load sections
        self.load_sections(&binary, &elf, code_region.start, data_region.as_ref().map(|r| r.start))?;
        
        // Perform relocations
        self.perform_relocations(&binary, &elf, code_region.start)?;
        
        // Resolve symbols
        self.resolve_symbols(&elf)?;
        
        // Get module instance
        let instance = self.get_module_instance(&elf, code_region.start)?;
        
        // Generate module ID
        let id = ModuleId::generate();
        
        // Store loaded module
        let loaded = LoadedModule {
            id,
            instance,
            code_region,
            data_region,
            entry_point: elf.entry_point,
            exports: elf.exports.clone(),
            loaded_at: current_tick(),
        };
        
        self.modules.insert(id, loaded);
        
        serial_println!("[LOADER] Module loaded: {} (ID: {:?})", path, id);
        
        Ok(id)
    }
    
    /// Load module from memory
    pub fn load_from_memory(&mut self, binary: &[u8]) -> ModuleResult<ModuleId> {
        // Parse ELF
        let elf = self.parse_elf(binary)?;
        
        // Same process as load()
        let code_region = self.allocate_code_region(elf.code_size)?;
        
        self.load_sections(binary, &elf, code_region.start, None)?;
        self.perform_relocations(binary, &elf, code_region.start)?;
        
        let instance = self.get_module_instance(&elf, code_region.start)?;
        let id = ModuleId::generate();
        
        let loaded = LoadedModule {
            id,
            instance,
            code_region,
            data_region: None,
            entry_point: elf.entry_point,
            exports: elf.exports.clone(),
            loaded_at: current_tick(),
        };
        
        self.modules.insert(id, loaded);
        
        Ok(id)
    }
    
    /// Unload a module
    pub fn unload(&mut self, id: ModuleId) -> ModuleResult<()> {
        let loaded = self.modules.remove(&id)
            .ok_or(ModuleError::NotFound)?;
        
        // Check for dependents
        if self.has_dependents(id) {
            self.modules.insert(id, loaded);
            return Err(ModuleError::HasDependents);
        }
        
        // Call cleanup
        // loaded.instance.cleanup()?;
        
        // Free memory
        self.free_memory_region(loaded.code_region)?;
        if let Some(data) = loaded.data_region {
            self.free_memory_region(data)?;
        }
        
        // Remove exported symbols
        for symbol in loaded.exports.keys() {
            self.symbols.remove(symbol);
        }
        
        serial_println!("[LOADER] Module unloaded: {:?}", id);
        
        Ok(())
    }
    
    /// Get module instance
    pub fn get_module(&self, id: ModuleId) -> Option<&dyn Module> {
        self.modules.get(&id).map(|m| m.instance.as_ref())
    }
    
    /// Get mutable module instance
    pub fn get_module_mut(&mut self, id: ModuleId) -> Option<&mut dyn Module> {
        self.modules.get_mut(&id).map(|m| m.instance.as_mut())
    }
    
    /// Resolve symbol address
    pub fn resolve_symbol(&self, name: &str) -> Option<usize> {
        self.symbols.get(name).copied()
    }
    
    /// Add search path
    pub fn add_search_path(&mut self, path: &'static str) {
        self.search_paths.push(path);
    }
    
    // Internal methods
    
    fn parse_elf(&self, binary: &[u8]) -> ModuleResult<ElfInfo> {
        // Parse ELF header
        if binary.len() < 64 {
            return Err(ModuleError::InvalidFormat);
        }
        
        // Check magic
        if &binary[0..4] != b"\x7FELF" {
            return Err(ModuleError::InvalidFormat);
        }
        
        // Parse header (simplified)
        let elf = ElfInfo {
            code_size: 0, // Would parse from section headers
            data_size: 0,
            entry_point: 0,
            exports: HashMap::new(),
            imports: Vec::new(),
        };
        
        Ok(elf)
    }
    
    fn allocate_code_region(&self, size: usize) -> ModuleResult<MemoryRegion> {
        // Allocate executable memory
        // let addr = memory::allocate_pages(pages, EXEC | READ)?;
        
        Ok(MemoryRegion {
            start: 0, // Placeholder
            size,
            permissions: 0x5, // R-X
        })
    }
    
    fn allocate_data_region(&self, size: usize) -> ModuleResult<MemoryRegion> {
        // Allocate data memory
        
        Ok(MemoryRegion {
            start: 0,
            size,
            permissions: 0x3, // RW-
        })
    }
    
    fn load_sections(
        &self,
        binary: &[u8],
        elf: &ElfInfo,
        code_base: usize,
        data_base: Option<usize>,
    ) -> ModuleResult<()> {
        // Copy sections to allocated memory
        // ...
        Ok(())
    }
    
    fn perform_relocations(
        &self,
        binary: &[u8],
        elf: &ElfInfo,
        base: usize,
    ) -> ModuleResult<()> {
        // Apply relocations
        // ...
        Ok(())
    }
    
    fn resolve_symbols(&mut self, elf: &ElfInfo) -> ModuleResult<()> {
        // Check that all imports can be resolved
        for import in &elf.imports {
            if !self.symbols.contains_key(import) {
                return Err(ModuleError::UnresolvedSymbol(import.clone()));
            }
        }
        
        // Add exports to symbol table
        for (name, addr) in &elf.exports {
            self.symbols.insert(name.clone(), *addr);
        }
        
        Ok(())
    }
    
    fn get_module_instance(
        &self,
        elf: &ElfInfo,
        base: usize,
    ) -> ModuleResult<Box<dyn Module>> {
        // Call module constructor
        // let constructor: fn() -> Box<dyn Module> = unsafe {
        //     core::mem::transmute(base + elf.entry_point)
        // };
        // Ok(constructor())
        
        Err(ModuleError::LoadFailed("Not implemented".into()))
    }
    
    fn has_dependents(&self, id: ModuleId) -> bool {
        // Check if any loaded module depends on this one
        false
    }
    
    fn free_memory_region(&self, region: MemoryRegion) -> ModuleResult<()> {
        // Free allocated memory
        Ok(())
    }
    
    fn read_module_file(&self, path: &str) -> ModuleResult<Vec<u8>> {
        // Read from filesystem
        Err(ModuleError::NotFound)
    }
}

/// ELF information (simplified)
struct ElfInfo {
    code_size: usize,
    data_size: usize,
    entry_point: usize,
    exports: HashMap<String, usize>,
    imports: Vec<String>,
}
```

---

## 4. Module Registry

### 4.1 Registry Implementation

```rust
/// Module registry for name/version lookup
pub struct ModuleRegistry {
    /// Registered modules by name
    by_name: HashMap<String, Vec<RegisteredModule>>,
    
    /// Registered modules by ID
    by_id: HashMap<ModuleId, RegistryEntry>,
    
    /// Interface providers
    interfaces: HashMap<InterfaceId, Vec<ModuleId>>,
    
    /// Registry lock
    lock: RwLock<()>,
}

/// Registered module info
#[derive(Clone)]
struct RegisteredModule {
    id: ModuleId,
    version: Version,
    state: ModuleState,
}

/// Registry entry
struct RegistryEntry {
    name: String,
    version: Version,
    description: String,
    author: String,
    dependencies: Vec<Dependency>,
    provides: Vec<InterfaceId>,
    state: ModuleState,
    load_time: u64,
}

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Registered but not loaded
    Registered,
    /// Being loaded
    Loading,
    /// Loaded and active
    Active,
    /// Paused
    Paused,
    /// Being unloaded
    Unloading,
    /// Error state
    Error,
}

impl ModuleRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            by_id: HashMap::new(),
            interfaces: HashMap::new(),
            lock: RwLock::new(()),
        }
    }
    
    /// Register a module
    pub fn register(
        &mut self,
        module: &dyn Module,
        id: ModuleId,
    ) -> RegistryResult<()> {
        let _guard = self.lock.write();
        
        let entry = RegistryEntry {
            name: module.name().to_string(),
            version: module.version(),
            description: module.description().to_string(),
            author: module.author().to_string(),
            dependencies: module.dependencies().to_vec(),
            provides: module.provides().to_vec(),
            state: ModuleState::Registered,
            load_time: current_tick(),
        };
        
        // Add to by_name
        self.by_name
            .entry(entry.name.clone())
            .or_default()
            .push(RegisteredModule {
                id,
                version: entry.version.clone(),
                state: ModuleState::Registered,
            });
        
        // Add to by_id
        self.by_id.insert(id, entry);
        
        // Register interfaces
        for interface in module.provides() {
            self.interfaces
                .entry(*interface)
                .or_default()
                .push(id);
        }
        
        Ok(())
    }
    
    /// Unregister a module
    pub fn unregister(&mut self, id: ModuleId) -> RegistryResult<()> {
        let _guard = self.lock.write();
        
        let entry = self.by_id.remove(&id)
            .ok_or(RegistryError::NotFound)?;
        
        // Remove from by_name
        if let Some(modules) = self.by_name.get_mut(&entry.name) {
            modules.retain(|m| m.id != id);
            if modules.is_empty() {
                self.by_name.remove(&entry.name);
            }
        }
        
        // Remove from interfaces
        for interface in &entry.provides {
            if let Some(providers) = self.interfaces.get_mut(interface) {
                providers.retain(|&m| m != id);
            }
        }
        
        Ok(())
    }
    
    /// Find module by name
    pub fn find_by_name(&self, name: &str) -> Option<ModuleId> {
        let _guard = self.lock.read();
        
        self.by_name.get(name)
            .and_then(|modules| {
                // Return latest version
                modules.iter()
                    .filter(|m| m.state == ModuleState::Active)
                    .max_by(|a, b| a.version.cmp(&b.version))
                    .map(|m| m.id)
            })
    }
    
    /// Find module by name and version
    pub fn find_by_name_version(&self, name: &str, version: &VersionRequirement) -> Option<ModuleId> {
        let _guard = self.lock.read();
        
        self.by_name.get(name)
            .and_then(|modules| {
                modules.iter()
                    .filter(|m| m.version.is_compatible(version))
                    .max_by(|a, b| a.version.cmp(&b.version))
                    .map(|m| m.id)
            })
    }
    
    /// Find modules providing an interface
    pub fn find_by_interface(&self, interface: InterfaceId) -> Vec<ModuleId> {
        let _guard = self.lock.read();
        
        self.interfaces.get(&interface)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Get module info
    pub fn get_info(&self, id: ModuleId) -> Option<ModuleInfo> {
        let _guard = self.lock.read();
        
        self.by_id.get(&id).map(|entry| ModuleInfo {
            name: entry.name.clone(),
            version: entry.version.clone(),
            description: entry.description.clone(),
            author: entry.author.clone(),
            state: entry.state,
            load_time: entry.load_time,
        })
    }
    
    /// Update module state
    pub fn set_state(&mut self, id: ModuleId, state: ModuleState) -> RegistryResult<()> {
        let _guard = self.lock.write();
        
        let entry = self.by_id.get_mut(&id)
            .ok_or(RegistryError::NotFound)?;
        
        entry.state = state;
        
        // Update by_name as well
        if let Some(modules) = self.by_name.get_mut(&entry.name) {
            if let Some(m) = modules.iter_mut().find(|m| m.id == id) {
                m.state = state;
            }
        }
        
        Ok(())
    }
    
    /// List all modules
    pub fn list_all(&self) -> Vec<ModuleInfo> {
        let _guard = self.lock.read();
        
        self.by_id.values()
            .map(|entry| ModuleInfo {
                name: entry.name.clone(),
                version: entry.version.clone(),
                description: entry.description.clone(),
                author: entry.author.clone(),
                state: entry.state,
                load_time: entry.load_time,
            })
            .collect()
    }
    
    /// List active modules
    pub fn list_active(&self) -> Vec<ModuleId> {
        let _guard = self.lock.read();
        
        self.by_id.iter()
            .filter(|(_, entry)| entry.state == ModuleState::Active)
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Module info (public)
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub state: ModuleState,
    pub load_time: u64,
}
```

---

## 5. Dependency Management

### 5.1 Dependency Resolver

```rust
/// Dependency resolver
pub struct DependencyResolver {
    /// Module registry reference
    registry: &'static ModuleRegistry,
    
    /// Dependency graph
    graph: DependencyGraph,
}

/// Dependency graph
struct DependencyGraph {
    /// Nodes (module IDs)
    nodes: HashSet<ModuleId>,
    
    /// Edges (dependencies)
    edges: HashMap<ModuleId, Vec<ModuleId>>,
}

impl DependencyResolver {
    /// Create new resolver
    pub fn new(registry: &'static ModuleRegistry) -> Self {
        Self {
            registry,
            graph: DependencyGraph {
                nodes: HashSet::new(),
                edges: HashMap::new(),
            },
        }
    }
    
    /// Resolve dependencies for a module
    pub fn resolve(&mut self, module: &dyn Module) -> ResolveResult<Vec<ModuleId>> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        
        self.resolve_recursive(module, &mut order, &mut visited, &mut stack)?;
        
        Ok(order)
    }
    
    fn resolve_recursive(
        &mut self,
        module: &dyn Module,
        order: &mut Vec<ModuleId>,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> ResolveResult<()> {
        let name = module.name().to_string();
        
        if stack.contains(&name) {
            return Err(ResolveError::CyclicDependency(name));
        }
        
        if visited.contains(&name) {
            return Ok(());
        }
        
        stack.insert(name.clone());
        
        for dep in module.dependencies() {
            // Find dependency in registry
            let dep_id = self.registry
                .find_by_name_version(dep.name, &dep.version)
                .ok_or_else(|| {
                    if dep.optional {
                        return ResolveError::OptionalNotFound(dep.name.to_string());
                    }
                    ResolveError::NotFound(dep.name.to_string())
                });
            
            match dep_id {
                Ok(id) => {
                    order.push(id);
                }
                Err(ResolveError::OptionalNotFound(_)) => {
                    // Optional dependency not found, continue
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        
        stack.remove(&name);
        visited.insert(name);
        
        Ok(())
    }
    
    /// Get load order for multiple modules
    pub fn get_load_order(&self, modules: &[ModuleId]) -> ResolveResult<Vec<ModuleId>> {
        // Topological sort
        let mut in_degree: HashMap<ModuleId, usize> = HashMap::new();
        let mut adjacency: HashMap<ModuleId, Vec<ModuleId>> = HashMap::new();
        
        // Build graph
        for &id in modules {
            in_degree.entry(id).or_insert(0);
            
            if let Some(info) = self.registry.get_info(id) {
                // Get dependencies and add edges
                // (simplified - would need full dependency info)
            }
        }
        
        // Kahn's algorithm
        let mut queue: Vec<ModuleId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&id, _)| id)
            .collect();
        
        let mut result = Vec::new();
        
        while let Some(id) = queue.pop() {
            result.push(id);
            
            if let Some(deps) = adjacency.get(&id) {
                for &dep in deps {
                    if let Some(degree) = in_degree.get_mut(&dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dep);
                        }
                    }
                }
            }
        }
        
        if result.len() != modules.len() {
            return Err(ResolveError::CyclicDependency("unknown".to_string()));
        }
        
        Ok(result)
    }
    
    /// Get unload order (reverse of load order)
    pub fn get_unload_order(&self, modules: &[ModuleId]) -> ResolveResult<Vec<ModuleId>> {
        let mut order = self.get_load_order(modules)?;
        order.reverse();
        Ok(order)
    }
    
    /// Check if a module can be unloaded
    pub fn can_unload(&self, id: ModuleId) -> bool {
        // Check if any active module depends on this one
        for active_id in self.registry.list_active() {
            if active_id == id {
                continue;
            }
            
            // Would need to check if active_id depends on id
        }
        
        true
    }
}

/// Resolve error
#[derive(Debug)]
pub enum ResolveError {
    /// Dependency not found
    NotFound(String),
    /// Optional dependency not found (not an error)
    OptionalNotFound(String),
    /// Cyclic dependency detected
    CyclicDependency(String),
    /// Version conflict
    VersionConflict {
        module: String,
        required: Version,
        available: Version,
    },
}
```

---

## 6. ABI Compatibility

### 6.1 ABI Definition

```rust
/// ABI version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiVersion {
    pub major: u16,
    pub minor: u16,
}

impl AbiVersion {
    /// Current ABI version
    pub const CURRENT: AbiVersion = AbiVersion { major: 1, minor: 0 };
    
    /// Check compatibility
    pub fn is_compatible(&self, other: &AbiVersion) -> bool {
        // Major must match, minor can differ
        self.major == other.major
    }
}

/// Module ABI header (at start of module binary)
#[repr(C)]
pub struct ModuleHeader {
    /// Magic number: "HXMD" (0x444D5848)
    pub magic: u32,
    
    /// ABI version
    pub abi_version: AbiVersion,
    
    /// Header size
    pub header_size: u32,
    
    /// Module flags
    pub flags: ModuleFlags,
    
    /// Name offset (from header start)
    pub name_offset: u32,
    
    /// Name length
    pub name_length: u32,
    
    /// Version
    pub version: Version,
    
    /// Init function offset
    pub init_offset: u32,
    
    /// Cleanup function offset
    pub cleanup_offset: u32,
    
    /// Module interface table offset
    pub interface_table_offset: u32,
    
    /// Interface table entries
    pub interface_count: u32,
    
    /// Dependency table offset
    pub dependency_table_offset: u32,
    
    /// Dependency count
    pub dependency_count: u32,
}

impl ModuleHeader {
    /// Magic number
    pub const MAGIC: u32 = 0x444D5848; // "HXMD"
    
    /// Validate header
    pub fn validate(&self) -> bool {
        self.magic == Self::MAGIC &&
        self.abi_version.is_compatible(&AbiVersion::CURRENT) &&
        self.header_size >= core::mem::size_of::<Self>() as u32
    }
}

bitflags! {
    /// Module flags
    pub struct ModuleFlags: u32 {
        /// Module supports hot-reload
        const HOT_RELOAD = 1 << 0;
        /// Module is a driver
        const DRIVER = 1 << 1;
        /// Module provides scheduling
        const SCHEDULER = 1 << 2;
        /// Module provides filesystem
        const FILESYSTEM = 1 << 3;
        /// Module is privileged
        const PRIVILEGED = 1 << 4;
        /// Module is optional
        const OPTIONAL = 1 << 5;
    }
}

/// Check ABI compatibility between modules
pub fn check_abi_compatibility(
    provider: &ModuleHeader,
    consumer: &ModuleHeader,
) -> AbiCompatibility {
    if provider.abi_version.major != consumer.abi_version.major {
        return AbiCompatibility::Incompatible {
            reason: "Major ABI version mismatch",
        };
    }
    
    if provider.abi_version.minor < consumer.abi_version.minor {
        return AbiCompatibility::PartiallyCompatible {
            warning: "Consumer uses newer ABI features",
        };
    }
    
    AbiCompatibility::Compatible
}

/// ABI compatibility result
pub enum AbiCompatibility {
    /// Fully compatible
    Compatible,
    /// Compatible with warnings
    PartiallyCompatible { warning: &'static str },
    /// Not compatible
    Incompatible { reason: &'static str },
}
```

### 6.2 Calling Convention

```rust
/// Module calling convention
/// 
/// Helix modules use the System V AMD64 ABI:
/// - Arguments: RDI, RSI, RDX, RCX, R8, R9, then stack
/// - Return: RAX (and RDX for 128-bit)
/// - Caller-saved: RAX, RCX, RDX, RSI, RDI, R8-R11
/// - Callee-saved: RBX, RBP, R12-R15
/// - Stack: 16-byte aligned before call

/// Module entry point signature
pub type ModuleEntry = extern "C" fn() -> *mut dyn Module;

/// Module init signature
pub type ModuleInit = extern "C" fn(*mut dyn Module) -> i32;

/// Module cleanup signature
pub type ModuleCleanup = extern "C" fn(*mut dyn Module) -> i32;

/// Wrapper for safe module calls
pub struct ModuleCaller {
    module: *mut dyn Module,
}

impl ModuleCaller {
    /// Create new caller
    pub fn new(module: *mut dyn Module) -> Self {
        Self { module }
    }
    
    /// Call init safely
    pub fn call_init(&self) -> ModuleResult<()> {
        let module = unsafe { &mut *self.module };
        module.init()
    }
    
    /// Call cleanup safely
    pub fn call_cleanup(&self) -> ModuleResult<()> {
        let module = unsafe { &mut *self.module };
        module.cleanup()
    }
}

/// FFI exports macro
#[macro_export]
macro_rules! export_module {
    ($ty:ty) => {
        #[no_mangle]
        pub extern "C" fn _helix_module_create() -> *mut dyn $crate::Module {
            let module = Box::new(<$ty>::new());
            Box::into_raw(module)
        }
        
        #[no_mangle]
        pub extern "C" fn _helix_module_destroy(module: *mut dyn $crate::Module) {
            if !module.is_null() {
                unsafe { drop(Box::from_raw(module)); }
            }
        }
    };
}
```

---

## 7. Hot-Reload Integration

### 7.1 Hot-Reload Support

```rust
/// Hot-reloadable module trait extension
pub trait HotReloadable: Module {
    /// Get state size estimate
    fn state_size(&self) -> usize {
        0
    }
    
    /// Prepare for hot-reload (pause operations)
    fn prepare_reload(&mut self) -> ModuleResult<()> {
        Ok(())
    }
    
    /// Complete hot-reload (resume operations)
    fn complete_reload(&mut self) -> ModuleResult<()> {
        Ok(())
    }
    
    /// Verify state after reload
    fn verify_state(&self) -> ModuleResult<()> {
        Ok(())
    }
}

/// Hot-reload state transfer
pub struct StateTransfer {
    /// Serialized state
    data: Vec<u8>,
    
    /// State version
    version: u32,
    
    /// Checksum
    checksum: u32,
}

impl StateTransfer {
    /// Create from module state
    pub fn from_module(module: &dyn Module) -> Option<Self> {
        let data = module.serialize_state()?;
        let checksum = Self::compute_checksum(&data);
        
        Some(Self {
            data,
            version: 1,
            checksum,
        })
    }
    
    /// Apply to new module
    pub fn apply_to(&self, module: &mut dyn Module) -> ModuleResult<()> {
        // Verify checksum
        if Self::compute_checksum(&self.data) != self.checksum {
            return Err(ModuleError::StateCorrupted);
        }
        
        module.deserialize_state(&self.data)
    }
    
    fn compute_checksum(data: &[u8]) -> u32 {
        // Simple CRC32-like checksum
        let mut crc = 0u32;
        for &byte in data {
            crc = crc.wrapping_add(byte as u32);
            crc = crc.rotate_left(5);
        }
        crc
    }
}

/// Module hot-reload manager
pub struct ModuleHotReloader {
    /// Module loader
    loader: ModuleLoader,
    
    /// Registry
    registry: ModuleRegistry,
    
    /// Pending reloads
    pending: Vec<PendingReload>,
}

struct PendingReload {
    old_id: ModuleId,
    new_binary: Vec<u8>,
}

impl ModuleHotReloader {
    /// Queue a module for hot-reload
    pub fn queue_reload(&mut self, id: ModuleId, new_binary: Vec<u8>) -> ModuleResult<()> {
        // Verify module supports hot-reload
        let module = self.loader.get_module(id)
            .ok_or(ModuleError::NotFound)?;
        
        if !module.supports_hot_reload() {
            return Err(ModuleError::HotReloadNotSupported);
        }
        
        self.pending.push(PendingReload {
            old_id: id,
            new_binary,
        });
        
        Ok(())
    }
    
    /// Execute pending reloads
    pub fn execute_pending(&mut self) -> Vec<ReloadResult> {
        let mut results = Vec::new();
        
        while let Some(reload) = self.pending.pop() {
            let result = self.execute_reload(reload);
            results.push(result);
        }
        
        results
    }
    
    fn execute_reload(&mut self, reload: PendingReload) -> ReloadResult {
        // 1. Get old module and save state
        let old_module = match self.loader.get_module(reload.old_id) {
            Some(m) => m,
            None => return ReloadResult::Failed {
                error: "Module not found".to_string(),
            },
        };
        
        let state = StateTransfer::from_module(old_module);
        
        // 2. Load new module
        let new_id = match self.loader.load_from_memory(&reload.new_binary) {
            Ok(id) => id,
            Err(e) => return ReloadResult::Failed {
                error: format!("Load failed: {:?}", e),
            },
        };
        
        // 3. Transfer state
        if let Some(state) = state {
            let new_module = self.loader.get_module_mut(new_id).unwrap();
            if let Err(e) = state.apply_to(new_module) {
                // Rollback: unload new, keep old
                let _ = self.loader.unload(new_id);
                return ReloadResult::Failed {
                    error: format!("State transfer failed: {:?}", e),
                };
            }
        }
        
        // 4. Unload old module
        if let Err(e) = self.loader.unload(reload.old_id) {
            // Warning but continue
            serial_println!("[HOT-RELOAD] Warning: failed to unload old module: {:?}", e);
        }
        
        ReloadResult::Success {
            old_id: reload.old_id,
            new_id,
        }
    }
}

/// Reload result
pub enum ReloadResult {
    Success {
        old_id: ModuleId,
        new_id: ModuleId,
    },
    Failed {
        error: String,
    },
}
```

---

## 8. Module Development

### 8.1 Creating a Module

```rust
//! Example module implementation

use helix_modules::*;

/// Example scheduler module
pub struct RoundRobinScheduler {
    /// Task queue
    tasks: Vec<Task>,
    
    /// Current index
    current: usize,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current: 0,
        }
    }
}

impl Module for RoundRobinScheduler {
    fn name(&self) -> &'static str {
        "round_robin_scheduler"
    }
    
    fn version(&self) -> Version {
        Version::new(1, 0, 0)
    }
    
    fn description(&self) -> &'static str {
        "Simple round-robin scheduler for task management"
    }
    
    fn author(&self) -> &'static str {
        "Helix Team"
    }
    
    fn dependencies(&self) -> &[Dependency] {
        &[] // No dependencies
    }
    
    fn provides(&self) -> &[InterfaceId] {
        &[SchedulerInterface::ID]
    }
    
    fn init(&mut self) -> ModuleResult<()> {
        serial_println!("[RR-SCHED] Initializing round-robin scheduler");
        Ok(())
    }
    
    fn cleanup(&mut self) -> ModuleResult<()> {
        serial_println!("[RR-SCHED] Cleaning up");
        self.tasks.clear();
        Ok(())
    }
    
    fn supports_hot_reload(&self) -> bool {
        true
    }
    
    fn serialize_state(&self) -> Option<Vec<u8>> {
        // Serialize task queue
        let mut data = Vec::new();
        
        // Write task count
        data.extend_from_slice(&(self.tasks.len() as u32).to_le_bytes());
        
        // Write each task (simplified)
        for task in &self.tasks {
            data.extend_from_slice(&task.id.to_le_bytes());
            data.extend_from_slice(&task.priority.to_le_bytes());
        }
        
        // Write current index
        data.extend_from_slice(&(self.current as u32).to_le_bytes());
        
        Some(data)
    }
    
    fn deserialize_state(&mut self, state: &[u8]) -> ModuleResult<()> {
        if state.len() < 4 {
            return Err(ModuleError::StateCorrupted);
        }
        
        let count = u32::from_le_bytes([state[0], state[1], state[2], state[3]]) as usize;
        
        self.tasks.clear();
        
        let mut offset = 4;
        for _ in 0..count {
            if offset + 8 > state.len() {
                return Err(ModuleError::StateCorrupted);
            }
            
            let id = u32::from_le_bytes([
                state[offset], state[offset+1], state[offset+2], state[offset+3]
            ]);
            let priority = u32::from_le_bytes([
                state[offset+4], state[offset+5], state[offset+6], state[offset+7]
            ]);
            
            self.tasks.push(Task { id, priority });
            offset += 8;
        }
        
        if offset + 4 > state.len() {
            return Err(ModuleError::StateCorrupted);
        }
        
        self.current = u32::from_le_bytes([
            state[offset], state[offset+1], state[offset+2], state[offset+3]
        ]) as usize;
        
        Ok(())
    }
}

// Export the module
export_module!(RoundRobinScheduler);

/// Task for scheduler
struct Task {
    id: u32,
    priority: u32,
}
```

### 8.2 Module Cargo.toml

```toml
[package]
name = "round_robin_scheduler"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
helix-modules = { path = "../../modules" }

[profile.release]
opt-level = "z"
lto = true
panic = "abort"
```

### 8.3 Building Modules

```bash
#!/bin/bash
# Build a Helix module

MODULE_NAME=$1

if [ -z "$MODULE_NAME" ]; then
    echo "Usage: $0 <module_name>"
    exit 1
fi

# Build module
cd modules_impl/$MODULE_NAME
cargo build --release --target x86_64-unknown-none

# Copy to modules directory
cp target/x86_64-unknown-none/release/lib${MODULE_NAME}.so \
   ../../build/modules/${MODULE_NAME}.hmod

echo "Module built: build/modules/${MODULE_NAME}.hmod"
```

---

## Summary

The Helix Module System provides:

1. **Module Interface**: Standard trait for all modules
2. **Module Loader**: ELF parsing, loading, symbol resolution
3. **Module Registry**: Name/version lookup, interface discovery
4. **Dependency Management**: Graph-based resolution, ordering
5. **ABI Compatibility**: Version checking, calling conventions
6. **Hot-Reload**: Live module updates with state transfer

For implementation details, see [modules/src/](../../modules/src/).

---

<div align="center">

🔧 *Modular by design, extensible by nature* 🔧

</div>
