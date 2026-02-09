//! # App Executable Profiler
//!
//! Profile application executable characteristics:
//! - ELF header analysis
//! - Section/segment classification
//! - Symbol table profiling
//! - Code density metrics
//! - Library dependency tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// EXECUTABLE TYPES
// ============================================================================

/// Executable format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableFormat {
    /// ELF executable
    Elf,
    /// Shared object
    SharedObject,
    /// Static executable
    StaticExe,
    /// Script (interpreter)
    Script,
    /// Unknown
    Unknown,
}

/// Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExeArchitecture {
    /// x86_64
    X86_64,
    /// AArch64
    Aarch64,
    /// RISC-V 64
    Riscv64,
    /// Unknown
    Unknown,
}

/// Section type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Relocation
    Rela,
    /// Dynamic
    Dynamic,
    /// Debug info
    Debug,
    /// Other
    Other,
}

// ============================================================================
// SECTION INFO
// ============================================================================

/// Section descriptor
#[derive(Debug, Clone)]
pub struct SectionInfo {
    /// Name hash
    pub name_hash: u64,
    /// Section type
    pub section_type: SectionType,
    /// Virtual address
    pub vaddr: u64,
    /// Size in file
    pub file_size: u64,
    /// Size in memory
    pub mem_size: u64,
    /// Flags (read/write/exec)
    pub flags: u32,
}

impl SectionInfo {
    /// Is executable
    #[inline(always)]
    pub fn is_executable(&self) -> bool {
        self.flags & 0x1 != 0
    }

    /// Is writable
    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.flags & 0x2 != 0
    }

    /// Is readable
    #[inline(always)]
    pub fn is_readable(&self) -> bool {
        self.flags & 0x4 != 0
    }
}

// ============================================================================
// LIBRARY DEPENDENCY
// ============================================================================

/// Library dependency
#[derive(Debug, Clone)]
pub struct LibraryDep {
    /// Library name hash
    pub name_hash: u64,
    /// Library name
    pub name: String,
    /// Version
    pub version: String,
    /// Symbols imported
    pub imported_symbols: u32,
    /// Is loaded
    pub loaded: bool,
}

// ============================================================================
// EXECUTABLE PROFILE
// ============================================================================

/// Executable profile
#[derive(Debug)]
pub struct ExecutableProfile {
    /// PID
    pub pid: u64,
    /// Format
    pub format: ExecutableFormat,
    /// Architecture
    pub architecture: ExeArchitecture,
    /// Entry point
    pub entry_point: u64,
    /// Sections
    pub sections: Vec<SectionInfo>,
    /// Library dependencies
    pub libraries: Vec<LibraryDep>,
    /// Total code size
    pub code_size: u64,
    /// Total data size
    pub data_size: u64,
    /// Total BSS size
    pub bss_size: u64,
    /// Is PIE (position independent)
    pub is_pie: bool,
    /// Has stack canary
    pub has_stack_canary: bool,
    /// Has RELRO
    pub has_relro: bool,
    /// Symbol count
    pub symbol_count: u32,
}

impl ExecutableProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            format: ExecutableFormat::Unknown,
            architecture: ExeArchitecture::Unknown,
            entry_point: 0,
            sections: Vec::new(),
            libraries: Vec::new(),
            code_size: 0,
            data_size: 0,
            bss_size: 0,
            is_pie: false,
            has_stack_canary: false,
            has_relro: false,
            symbol_count: 0,
        }
    }

    /// Add section
    #[inline]
    pub fn add_section(&mut self, section: SectionInfo) {
        match section.section_type {
            SectionType::Text => self.code_size += section.file_size,
            SectionType::Data | SectionType::Rodata => self.data_size += section.file_size,
            SectionType::Bss => self.bss_size += section.mem_size,
            _ => {},
        }
        self.sections.push(section);
    }

    /// Add library dependency
    #[inline(always)]
    pub fn add_library(&mut self, lib: LibraryDep) {
        self.libraries.push(lib);
    }

    /// Code density (code / total)
    #[inline]
    pub fn code_density(&self) -> f64 {
        let total = self.code_size + self.data_size + self.bss_size;
        if total == 0 {
            return 0.0;
        }
        self.code_size as f64 / total as f64
    }

    /// Library count
    #[inline(always)]
    pub fn library_count(&self) -> usize {
        self.libraries.len()
    }

    /// Security score (0-100)
    pub fn security_score(&self) -> u32 {
        let mut score = 0u32;
        if self.is_pie {
            score += 30;
        }
        if self.has_stack_canary {
            score += 30;
        }
        if self.has_relro {
            score += 20;
        }
        if self.format == ExecutableFormat::StaticExe {
            score += 10; // fewer dynamic loading risks
        }
        score.min(100)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Executable profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppExeProfilerStats {
    /// Profiled executables
    pub profiled: usize,
    /// Total libraries tracked
    pub total_libraries: usize,
    /// PIE executables
    pub pie_count: usize,
    /// Average security score
    pub avg_security_score: f64,
}

/// App executable profiler
pub struct AppExeProfiler {
    /// Profiles, keyed by PID
    profiles: BTreeMap<u64, ExecutableProfile>,
    /// Stats
    stats: AppExeProfilerStats,
}

impl AppExeProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppExeProfilerStats::default(),
        }
    }

    /// Create profile
    #[inline]
    pub fn create_profile(&mut self, pid: u64) -> &mut ExecutableProfile {
        self.profiles
            .entry(pid)
            .or_insert_with(|| ExecutableProfile::new(pid))
    }

    /// Get profile
    #[inline(always)]
    pub fn get(&self, pid: u64) -> Option<&ExecutableProfile> {
        self.profiles.get(&pid)
    }

    /// Remove profile
    #[inline(always)]
    pub fn remove(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.update_stats();
    }

    /// Find processes using library
    #[inline]
    pub fn processes_using_library(&self, lib_hash: u64) -> Vec<u64> {
        self.profiles
            .iter()
            .filter(|(_, p)| p.libraries.iter().any(|l| l.name_hash == lib_hash))
            .map(|(&pid, _)| pid)
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.profiled = self.profiles.len();
        self.stats.total_libraries = self.profiles.values().map(|p| p.libraries.len()).sum();
        self.stats.pie_count = self.profiles.values().filter(|p| p.is_pie).count();
        if !self.profiles.is_empty() {
            let total: f64 = self
                .profiles
                .values()
                .map(|p| p.security_score() as f64)
                .sum();
            self.stats.avg_security_score = total / self.profiles.len() as f64;
        }
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppExeProfilerStats {
        &self.stats
    }
}
