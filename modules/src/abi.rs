//! # ABI Management
//!
//! Manages ABI versions and compatibility for modules.

use core::cmp::Ordering;

/// ABI version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbiVersion {
    /// Major version (breaking changes)
    pub major: u16,
    /// Minor version (additions)
    pub minor: u16,
}

impl AbiVersion {
    /// Current ABI version
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    /// Create a new ABI version
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Check if this version is compatible with another
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major && self.minor >= other.minor
    }

    /// Check if this version is forward-compatible with another
    pub fn is_forward_compatible_with(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl PartialOrd for AbiVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AbiVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            ord => ord,
        }
    }
}

/// ABI compatibility checker
pub struct AbiChecker {
    /// Minimum supported ABI version
    min_version: AbiVersion,
    /// Maximum supported ABI version
    max_version: AbiVersion,
    /// Known deprecated symbols
    deprecated: spin::RwLock<alloc::vec::Vec<DeprecatedSymbol>>,
}

impl AbiChecker {
    /// Create a new ABI checker
    pub fn new(min: AbiVersion, max: AbiVersion) -> Self {
        Self {
            min_version: min,
            max_version: max,
            deprecated: spin::RwLock::new(alloc::vec::Vec::new()),
        }
    }

    /// Check if a module's ABI is compatible
    pub fn check(&self, module_abi: AbiVersion) -> AbiCheckResult {
        if module_abi.major < self.min_version.major {
            AbiCheckResult::TooOld
        } else if module_abi.major > self.max_version.major {
            AbiCheckResult::TooNew
        } else if module_abi < self.min_version {
            AbiCheckResult::MinorMismatch
        } else {
            AbiCheckResult::Compatible
        }
    }

    /// Register a deprecated symbol
    pub fn register_deprecated(&self, symbol: DeprecatedSymbol) {
        self.deprecated.write().push(symbol);
    }

    /// Check if a symbol is deprecated
    pub fn is_deprecated(&self, name: &str) -> Option<&'static str> {
        let deprecated = self.deprecated.read();
        deprecated
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.replacement)
    }
}

/// ABI check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiCheckResult {
    /// ABI is compatible
    Compatible,
    /// ABI is too old
    TooOld,
    /// ABI is too new
    TooNew,
    /// Minor version mismatch (may work with shims)
    MinorMismatch,
}

/// Deprecated symbol information
#[derive(Debug, Clone)]
pub struct DeprecatedSymbol {
    /// Symbol name
    pub name: &'static str,
    /// Replacement suggestion
    pub replacement: &'static str,
    /// Version when deprecated
    pub since: AbiVersion,
    /// Version when removed (if any)
    pub removed: Option<AbiVersion>,
}

/// Symbol information for linking
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol name
    pub name: alloc::string::String,
    /// Symbol address
    pub address: u64,
    /// Symbol size
    pub size: usize,
    /// Symbol type
    pub kind: SymbolKind,
    /// Is this symbol exported?
    pub exported: bool,
}

/// Symbol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Function
    Function,
    /// Data
    Data,
    /// Read-only data
    RoData,
    /// BSS
    Bss,
    /// Unknown
    Unknown,
}

/// Symbol table
pub struct SymbolTable {
    /// Symbols
    symbols: spin::RwLock<alloc::collections::BTreeMap<alloc::string::String, Symbol>>,
}

impl SymbolTable {
    /// Create a new symbol table
    pub const fn new() -> Self {
        Self {
            symbols: spin::RwLock::new(alloc::collections::BTreeMap::new()),
        }
    }

    /// Add a symbol
    pub fn add(&self, symbol: Symbol) {
        self.symbols.write().insert(symbol.name.clone(), symbol);
    }

    /// Look up a symbol by name
    pub fn lookup(&self, name: &str) -> Option<Symbol> {
        self.symbols.read().get(name).cloned()
    }

    /// Get all exported symbols
    pub fn exported_symbols(&self) -> alloc::vec::Vec<Symbol> {
        self.symbols
            .read()
            .values()
            .filter(|s| s.exported)
            .cloned()
            .collect()
    }
}

/// Global symbol table
static GLOBAL_SYMBOLS: SymbolTable = SymbolTable::new();

/// Get the global symbol table
pub fn global_symbols() -> &'static SymbolTable {
    &GLOBAL_SYMBOLS
}
