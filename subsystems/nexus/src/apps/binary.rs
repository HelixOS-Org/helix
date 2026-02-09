//! # Application Executable Format Analyzer
//!
//! ELF/binary format analysis for application understanding:
//! - Section analysis
//! - Symbol table parsing
//! - Relocation tracking
//! - Dynamic linking analysis
//! - Code section characteristics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// BINARY FORMAT TYPES
// ============================================================================

/// Executable format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecFormat {
    /// ELF (Linux native)
    Elf64,
    /// ELF 32-bit
    Elf32,
    /// Flat binary
    FlatBinary,
    /// Script (interpreted)
    Script,
    /// Unknown
    Unknown,
}

/// Section type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SectionType {
    /// Code (.text)
    Text,
    /// Read-only data (.rodata)
    Rodata,
    /// Initialized data (.data)
    Data,
    /// Uninitialized data (.bss)
    Bss,
    /// Symbol table
    Symtab,
    /// String table
    Strtab,
    /// Dynamic linking
    Dynamic,
    /// Relocation
    Reloc,
    /// Debug info
    Debug,
    /// Note
    Note,
    /// Other
    Other,
}

/// Section permissions
#[derive(Debug, Clone, Copy)]
pub struct SectionPerms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl SectionPerms {
    pub fn new(read: bool, write: bool, exec: bool) -> Self {
        Self { read, write, exec }
    }

    /// Is writable and executable? (security concern)
    #[inline(always)]
    pub fn is_wx(&self) -> bool {
        self.write && self.exec
    }
}

// ============================================================================
// SECTION INFO
// ============================================================================

/// Section descriptor
#[derive(Debug, Clone)]
pub struct SectionInfo {
    /// Section name hash
    pub name_hash: u64,
    /// Section type
    pub section_type: SectionType,
    /// Virtual address
    pub vaddr: u64,
    /// File offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Permissions
    pub perms: SectionPerms,
}

impl SectionInfo {
    pub fn new(name_hash: u64, section_type: SectionType, vaddr: u64, size: u64) -> Self {
        Self {
            name_hash,
            section_type,
            vaddr,
            offset: 0,
            size,
            alignment: 4096,
            perms: SectionPerms::new(true, false, false),
        }
    }

    /// End address
    #[inline(always)]
    pub fn end_vaddr(&self) -> u64 {
        self.vaddr + self.size
    }

    /// Contains address?
    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.vaddr && addr < self.end_vaddr()
    }
}

// ============================================================================
// SYMBOL INFO
// ============================================================================

/// Symbol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// Function
    Function,
    /// Object/variable
    Object,
    /// Section
    Section,
    /// File
    File,
    /// TLS
    Tls,
    /// Other
    Other,
}

/// Symbol binding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBinding {
    /// Local
    Local,
    /// Global
    Global,
    /// Weak
    Weak,
}

/// Symbol descriptor
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Name hash
    pub name_hash: u64,
    /// Symbol type
    pub sym_type: SymbolType,
    /// Binding
    pub binding: SymbolBinding,
    /// Value (address)
    pub value: u64,
    /// Size
    pub size: u64,
    /// Section index
    pub section_idx: u16,
}

// ============================================================================
// BINARY PROFILE
// ============================================================================

/// Binary analysis profile
#[derive(Debug)]
pub struct BinaryProfile {
    /// Format
    pub format: ExecFormat,
    /// Entry point
    pub entry_point: u64,
    /// Sections
    pub sections: Vec<SectionInfo>,
    /// Symbol count
    pub symbol_count: u64,
    /// Dynamic libraries referenced
    pub dynamic_libs: Vec<u64>, // name hashes
    /// Total code size
    pub code_size: u64,
    /// Total data size
    pub data_size: u64,
    /// Has debug info
    pub has_debug: bool,
    /// Has RELRO (security)
    pub has_relro: bool,
    /// Has stack canary
    pub has_stack_canary: bool,
    /// Is PIE
    pub is_pie: bool,
    /// WX sections (security risk)
    pub wx_sections: u32,
}

impl BinaryProfile {
    pub fn new(format: ExecFormat) -> Self {
        Self {
            format,
            entry_point: 0,
            sections: Vec::new(),
            symbol_count: 0,
            dynamic_libs: Vec::new(),
            code_size: 0,
            data_size: 0,
            has_debug: false,
            has_relro: false,
            has_stack_canary: false,
            is_pie: false,
            wx_sections: 0,
        }
    }

    /// Add section
    pub fn add_section(&mut self, section: SectionInfo) {
        match section.section_type {
            SectionType::Text => self.code_size += section.size,
            SectionType::Data | SectionType::Bss => self.data_size += section.size,
            SectionType::Debug => self.has_debug = true,
            _ => {}
        }
        if section.perms.is_wx() {
            self.wx_sections += 1;
        }
        self.sections.push(section);
    }

    /// Code to data ratio
    #[inline]
    pub fn code_data_ratio(&self) -> f64 {
        if self.data_size == 0 {
            return f64::MAX;
        }
        self.code_size as f64 / self.data_size as f64
    }

    /// Security score (0-100, higher = more secure)
    #[inline]
    pub fn security_score(&self) -> u32 {
        let mut score = 50u32;
        if self.is_pie { score += 15; }
        if self.has_relro { score += 15; }
        if self.has_stack_canary { score += 10; }
        if self.wx_sections > 0 { score = score.saturating_sub(20); }
        score.min(100)
    }

    /// Find section by address
    #[inline(always)]
    pub fn section_at(&self, addr: u64) -> Option<&SectionInfo> {
        self.sections.iter().find(|s| s.contains(addr))
    }
}

// ============================================================================
// BINARY ANALYZER ENGINE
// ============================================================================

/// Binary analysis stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppBinaryStats {
    /// Profiles analyzed
    pub profiles_analyzed: usize,
    /// Total code size
    pub total_code_size: u64,
    /// Average security score
    pub avg_security_score: f64,
}

/// App binary analyzer
pub struct AppBinaryAnalyzer {
    /// Profiles per pid
    profiles: BTreeMap<u64, BinaryProfile>,
    /// Stats
    stats: AppBinaryStats,
}

impl AppBinaryAnalyzer {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppBinaryStats::default(),
        }
    }

    /// Register binary profile
    #[inline(always)]
    pub fn register(&mut self, pid: u64, profile: BinaryProfile) {
        self.profiles.insert(pid, profile);
        self.update_stats();
    }

    /// Get profile
    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&BinaryProfile> {
        self.profiles.get(&pid)
    }

    /// Remove
    #[inline(always)]
    pub fn remove(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.update_stats();
    }

    /// Insecure binaries
    #[inline]
    pub fn insecure_binaries(&self) -> Vec<u64> {
        self.profiles.iter()
            .filter(|(_, p)| p.security_score() < 50)
            .map(|(&pid, _)| pid)
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.profiles_analyzed = self.profiles.len();
        self.stats.total_code_size = self.profiles.values().map(|p| p.code_size).sum();
        if !self.profiles.is_empty() {
            let sum: f64 = self.profiles.values().map(|p| p.security_score() as f64).sum();
            self.stats.avg_security_score = sum / self.profiles.len() as f64;
        }
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppBinaryStats {
        &self.stats
    }
}
