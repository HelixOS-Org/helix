//! # Module Loader
//!
//! Handles loading module binaries from various sources.

use crate::{Module, ModuleMetadata, ModuleResult, ModuleError, ModuleState};
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

/// Module binary format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleFormat {
    /// ELF format
    Elf,
    /// Helix native format (HMF)
    HelixNative,
    /// WebAssembly
    Wasm,
}

/// Loaded module handle
pub struct LoadedModule {
    /// Module instance
    pub module: Arc<RwLock<dyn Module>>,
    /// Load address (for kernel modules)
    pub load_address: Option<u64>,
    /// Module size
    pub size: usize,
    /// Current state
    pub state: ModuleState,
}

/// Module loader trait
pub trait ModuleLoader: Send + Sync {
    /// Get the format this loader supports
    fn format(&self) -> ModuleFormat;

    /// Check if binary is valid
    fn validate(&self, binary: &[u8]) -> ModuleResult<()>;

    /// Extract metadata without fully loading
    fn extract_metadata(&self, binary: &[u8]) -> ModuleResult<ModuleMetadata>;

    /// Load a module from binary
    fn load(&self, binary: &[u8]) -> ModuleResult<LoadedModule>;

    /// Unload a module
    fn unload(&self, module: &LoadedModule) -> ModuleResult<()>;
}

/// ELF module loader
pub struct ElfLoader {
    /// Allocate kernel memory
    allocate: Box<dyn Fn(usize, usize) -> Option<u64> + Send + Sync>,
    /// Free kernel memory
    free: Box<dyn Fn(u64, usize) + Send + Sync>,
}

impl ElfLoader {
    /// Create a new ELF loader
    pub fn new(
        allocate: impl Fn(usize, usize) -> Option<u64> + Send + Sync + 'static,
        free: impl Fn(u64, usize) + Send + Sync + 'static,
    ) -> Self {
        Self {
            allocate: Box::new(allocate),
            free: Box::new(free),
        }
    }
}

impl ModuleLoader for ElfLoader {
    fn format(&self) -> ModuleFormat {
        ModuleFormat::Elf
    }

    fn validate(&self, binary: &[u8]) -> ModuleResult<()> {
        // Check ELF magic
        if binary.len() < 4 || &binary[0..4] != b"\x7fELF" {
            return Err(ModuleError::LoadError("Invalid ELF magic".into()));
        }

        // Check 64-bit
        if binary.len() < 5 || binary[4] != 2 {
            return Err(ModuleError::LoadError("Not 64-bit ELF".into()));
        }

        Ok(())
    }

    fn extract_metadata(&self, binary: &[u8]) -> ModuleResult<ModuleMetadata> {
        self.validate(binary)?;
        
        // TODO: Parse ELF sections to extract .helix_meta section
        
        Err(ModuleError::Internal("ELF metadata extraction not yet implemented".into()))
    }

    fn load(&self, binary: &[u8]) -> ModuleResult<LoadedModule> {
        self.validate(binary)?;
        
        // TODO: Full ELF loading
        // 1. Parse ELF headers
        // 2. Allocate memory for segments
        // 3. Copy segments
        // 4. Perform relocations
        // 5. Resolve symbols
        // 6. Call module init function
        
        Err(ModuleError::Internal("ELF loading not yet implemented".into()))
    }

    fn unload(&self, module: &LoadedModule) -> ModuleResult<()> {
        if let Some(addr) = module.load_address {
            (self.free)(addr, module.size);
        }
        Ok(())
    }
}

/// Module loader registry
pub struct LoaderRegistry {
    loaders: RwLock<Vec<Arc<dyn ModuleLoader>>>,
}

impl LoaderRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        Self {
            loaders: RwLock::new(Vec::new()),
        }
    }

    /// Register a loader
    pub fn register(&self, loader: Arc<dyn ModuleLoader>) {
        self.loaders.write().push(loader);
    }

    /// Get a loader for a format
    pub fn get(&self, format: ModuleFormat) -> Option<Arc<dyn ModuleLoader>> {
        self.loaders.read()
            .iter()
            .find(|l| l.format() == format)
            .cloned()
    }

    /// Detect format and get appropriate loader
    pub fn detect_and_get(&self, binary: &[u8]) -> Option<Arc<dyn ModuleLoader>> {
        for loader in self.loaders.read().iter() {
            if loader.validate(binary).is_ok() {
                return Some(loader.clone());
            }
        }
        None
    }
}

/// Global loader registry
static REGISTRY: LoaderRegistry = LoaderRegistry::new();

/// Get the loader registry
pub fn registry() -> &'static LoaderRegistry {
    &REGISTRY
}
