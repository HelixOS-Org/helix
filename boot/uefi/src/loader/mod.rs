//! Kernel Loader Module
//!
//! This module provides comprehensive kernel loading facilities for the UEFI
//! boot environment. It supports multiple executable formats, relocation,
//! verification, and preparation for kernel handoff.
//!
//! # Architecture
//!
//! The loader module is organized in layers:
//! - ELF parsing and loading
//! - PE/COFF support (for Windows compatibility)
//! - Image verification (signatures, hashes)
//! - Relocation engine
//! - Module loading
//!
//! # Features
//!
//! - Full ELF64 support with all relocation types
//! - PE32+ support for UEFI-style images
//! - Cryptographic verification (SHA-256, RSA)
//! - Dynamic relocation for KASLR
//! - Module dependency resolution
//! - Symbol table management
//!
//! # KASLR Support
//!
//! The `relocation` module provides comprehensive KASLR (Kernel Address Space
//! Layout Randomization) support:
//! - Hardware entropy collection (RDRAND/RDSEED)
//! - Randomized kernel load addresses
//! - Full ELF64 relocation support for PIE kernels

#![allow(dead_code)]

pub mod elf;
pub mod image;
pub mod pe;
pub mod relocate;
pub mod relocation;
pub mod symbols;
pub mod verify;

// Explicit re-exports to avoid ambiguity
// Both elf and pe modules export a `machine` submodule
// Re-export them as elf_machine and pe_machine to avoid conflict
pub use elf::{
    class, data, elf_type, machine as elf_machine, osabi, pf, pt, r_x86_64, shf, sht, stb, stt,
    Elf64Dyn, Elf64Header, Elf64ProgramHeader, Elf64Rel, Elf64Rela, Elf64SectionHeader,
    Elf64Symbol, ElfLoader, ElfRelocation, ElfSymbol, ELF_MAGIC,
};
pub use image::*;
pub use pe::{
    characteristics, directory, dll_characteristics, machine as pe_machine, reloc_type,
    section_characteristics, subsystem, BaseRelocationBlock, CoffHeader, DataDirectory, DosHeader,
    ExportDirectory, ImportDirectory, OptionalHeader64, PeExport, PeImport, PeLoader, PeRelocation,
    PeSectionHeader, DOS_MAGIC, PE32PLUS_MAGIC, PE32_MAGIC, PE_SIGNATURE,
};
pub use relocate::*;
pub use relocation::{RelocationConfig, RelocationStats};
pub use symbols::*;
pub use verify::*;

use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// KERNEL LOADER
// =============================================================================

/// Main kernel loader
pub struct KernelLoader {
    /// ELF loader
    elf_loader: ElfLoader,
    /// PE loader
    pe_loader: PeLoader,
    /// Image verifier
    verifier: ImageVerifier,
    /// Relocation engine
    relocator: RelocationEngine,
    /// Symbol manager
    symbols: SymbolManager,
    /// Loaded image info
    loaded_image: Option<LoadedImage>,
    /// Load configuration
    config: LoaderConfig,
}

impl KernelLoader {
    /// Create new kernel loader
    pub fn new() -> Self {
        Self {
            elf_loader: ElfLoader::new(),
            pe_loader: PeLoader::new(),
            verifier: ImageVerifier::new(),
            relocator: RelocationEngine::new(),
            symbols: SymbolManager::new(),
            loaded_image: None,
            config: LoaderConfig::default(),
        }
    }

    /// Create with configuration
    pub fn with_config(config: LoaderConfig) -> Self {
        let mut loader = Self::new();
        loader.config = config;
        loader
    }

    /// Load kernel from buffer
    pub fn load(&mut self, data: &[u8]) -> Result<&LoadedImage> {
        // Detect format
        let format = ImageFormat::detect(data)?;

        // Load based on format
        let image = match format {
            ImageFormat::Elf64 => self.elf_loader.load(data)?,
            ImageFormat::Pe32Plus => self.pe_loader.load(data)?,
            _ => return Err(Error::UnsupportedFormat),
        };

        // Verify if enabled
        if self.config.verify_signature {
            self.verifier.verify_image(&image)?;
        }

        // Apply relocations if needed
        if self.config.apply_relocations {
            self.relocator
                .relocate(&mut self.elf_loader, self.config.base_address)?;
        }

        // Extract symbols if enabled
        if self.config.load_symbols {
            self.symbols.load_from_image(&image)?;
        }

        self.loaded_image = Some(image);
        Ok(self.loaded_image.as_ref().unwrap())
    }

    /// Load kernel from file path
    pub fn load_file(&mut self, _path: &str) -> Result<&LoadedImage> {
        // This would use filesystem protocol
        // For now just return error
        Err(Error::NotSupported)
    }

    /// Set base address for relocation
    pub fn set_base_address(&mut self, address: VirtualAddress) {
        self.config.base_address = Some(address);
    }

    /// Enable KASLR
    pub fn enable_kaslr(&mut self, seed: u64) {
        self.config.kaslr_enabled = true;
        self.config.kaslr_seed = seed;
    }

    /// Get loaded image
    pub fn image(&self) -> Option<&LoadedImage> {
        self.loaded_image.as_ref()
    }

    /// Get entry point
    pub fn entry_point(&self) -> Option<VirtualAddress> {
        self.loaded_image.as_ref().map(|i| i.entry_point)
    }

    /// Get symbol manager
    pub fn symbols(&self) -> &SymbolManager {
        &self.symbols
    }

    /// Prepare for execution
    pub fn prepare_execution(&self) -> Result<ExecutionContext> {
        let image = self.loaded_image.as_ref().ok_or(Error::NotLoaded)?;

        Ok(ExecutionContext {
            entry_point: image.entry_point,
            stack_top: image.stack_top.unwrap_or(VirtualAddress(0)),
            page_table_root: PhysicalAddress(0),
            boot_info_address: VirtualAddress(0),
        })
    }
}

impl Default for KernelLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// LOADER CONFIGURATION
// =============================================================================

/// Loader configuration
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Verify image signature
    pub verify_signature: bool,
    /// Apply relocations
    pub apply_relocations: bool,
    /// Load debug symbols
    pub load_symbols: bool,
    /// Base address override
    pub base_address: Option<VirtualAddress>,
    /// Enable KASLR
    pub kaslr_enabled: bool,
    /// KASLR seed
    pub kaslr_seed: u64,
    /// Maximum image size
    pub max_image_size: u64,
    /// Stack size
    pub stack_size: u64,
    /// Heap size
    pub heap_size: u64,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            verify_signature: false,
            apply_relocations: true,
            load_symbols: true,
            base_address: None,
            kaslr_enabled: false,
            kaslr_seed: 0,
            max_image_size: 256 * 1024 * 1024, // 256 MiB
            stack_size: 2 * 1024 * 1024,       // 2 MiB
            heap_size: 16 * 1024 * 1024,       // 16 MiB
        }
    }
}

// =============================================================================
// IMAGE FORMAT
// =============================================================================

/// Executable image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// ELF 32-bit
    Elf32,
    /// ELF 64-bit
    Elf64,
    /// PE32
    Pe32,
    /// PE32+
    Pe32Plus,
    /// Mach-O
    MachO,
    /// Raw binary
    Binary,
    /// Unknown
    Unknown,
}

impl ImageFormat {
    /// Detect format from magic bytes
    pub fn detect(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(Error::InvalidData);
        }

        // ELF magic
        if data[0..4] == [0x7F, b'E', b'L', b'F'] {
            if data.len() < 5 {
                return Err(Error::InvalidData);
            }
            return Ok(match data[4] {
                1 => Self::Elf32,
                2 => Self::Elf64,
                _ => Self::Unknown,
            });
        }

        // PE magic (MZ)
        if data[0..2] == [b'M', b'Z'] {
            if data.len() < 64 {
                return Err(Error::InvalidData);
            }
            // Get PE offset
            let pe_offset = u32::from_le_bytes([data[60], data[61], data[62], data[63]]) as usize;
            if data.len() < pe_offset + 6 {
                return Err(Error::InvalidData);
            }
            // Check PE signature
            if data[pe_offset..pe_offset + 4] == [b'P', b'E', 0, 0] {
                // Check machine type for PE32 vs PE32+
                let machine = u16::from_le_bytes([data[pe_offset + 4], data[pe_offset + 5]]);
                return Ok(match machine {
                    0x8664 => Self::Pe32Plus, // AMD64
                    0x014c => Self::Pe32,     // i386
                    0xAA64 => Self::Pe32Plus, // ARM64
                    _ => Self::Unknown,
                });
            }
        }

        // Mach-O magic
        if data[0..4] == [0xFE, 0xED, 0xFA, 0xCE]
            || data[0..4] == [0xFE, 0xED, 0xFA, 0xCF]
            || data[0..4] == [0xCE, 0xFA, 0xED, 0xFE]
            || data[0..4] == [0xCF, 0xFA, 0xED, 0xFE]
        {
            return Ok(Self::MachO);
        }

        Ok(Self::Unknown)
    }

    /// Check if format is supported
    pub fn is_supported(&self) -> bool {
        matches!(self, Self::Elf64 | Self::Pe32Plus)
    }

    /// Get format name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Elf32 => "ELF32",
            Self::Elf64 => "ELF64",
            Self::Pe32 => "PE32",
            Self::Pe32Plus => "PE32+",
            Self::MachO => "Mach-O",
            Self::Binary => "Binary",
            Self::Unknown => "Unknown",
        }
    }
}

// =============================================================================
// LOADED IMAGE
// =============================================================================

/// Loaded kernel image
#[derive(Debug, Clone)]
pub struct LoadedImage {
    /// Image format
    pub format: ImageFormat,
    /// Entry point virtual address
    pub entry_point: VirtualAddress,
    /// Load address
    pub load_address: VirtualAddress,
    /// Image size
    pub image_size: u64,
    /// Sections
    pub sections: Vec<ImageSection>,
    /// Stack top (if allocated)
    pub stack_top: Option<VirtualAddress>,
    /// BSS start
    pub bss_start: Option<VirtualAddress>,
    /// BSS size
    pub bss_size: u64,
    /// Image name
    pub name: String,
    /// Machine type
    pub machine: MachineType,
    /// Flags
    pub flags: ImageFlags,
}

impl LoadedImage {
    /// Get section by name
    pub fn section(&self, name: &str) -> Option<&ImageSection> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// Get code sections
    pub fn code_sections(&self) -> Vec<&ImageSection> {
        self.sections
            .iter()
            .filter(|s| s.flags.executable)
            .collect()
    }

    /// Get data sections
    pub fn data_sections(&self) -> Vec<&ImageSection> {
        self.sections
            .iter()
            .filter(|s| s.flags.writable && !s.flags.executable)
            .collect()
    }

    /// Get read-only sections
    pub fn readonly_sections(&self) -> Vec<&ImageSection> {
        self.sections
            .iter()
            .filter(|s| !s.flags.writable && !s.flags.executable)
            .collect()
    }

    /// Calculate memory requirements
    pub fn memory_requirements(&self) -> MemoryRequirements {
        let mut code_size = 0u64;
        let mut data_size = 0u64;
        let mut rodata_size = 0u64;

        for section in &self.sections {
            if section.flags.executable {
                code_size += section.size;
            } else if section.flags.writable {
                data_size += section.size;
            } else {
                rodata_size += section.size;
            }
        }

        MemoryRequirements {
            code_size,
            data_size,
            rodata_size,
            bss_size: self.bss_size,
            total_size: self.image_size,
        }
    }
}

/// Image section
#[derive(Debug, Clone)]
pub struct ImageSection {
    /// Section name
    pub name: String,
    /// Virtual address
    pub virtual_address: VirtualAddress,
    /// Size in memory
    pub size: u64,
    /// File offset
    pub file_offset: u64,
    /// File size
    pub file_size: u64,
    /// Alignment
    pub alignment: u64,
    /// Flags
    pub flags: SectionFlags,
}

/// Section flags
#[derive(Debug, Clone, Copy, Default)]
pub struct SectionFlags {
    /// Readable
    pub readable: bool,
    /// Writable
    pub writable: bool,
    /// Executable
    pub executable: bool,
    /// Allocated
    pub allocated: bool,
    /// Contains code
    pub code: bool,
    /// Contains data
    pub data: bool,
    /// BSS section
    pub bss: bool,
}

/// Image flags
#[derive(Debug, Clone, Copy, Default)]
pub struct ImageFlags {
    /// Position independent
    pub pie: bool,
    /// No execute stack
    pub nx_stack: bool,
    /// Relocatable
    pub relocatable: bool,
    /// Has symbols
    pub has_symbols: bool,
    /// Stripped
    pub stripped: bool,
}

/// Machine type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineType {
    /// Unknown
    Unknown,
    /// x86 (i386)
    X86,
    /// x86-64 (AMD64)
    X86_64,
    /// ARM 32-bit
    Arm,
    /// ARM 64-bit
    Aarch64,
    /// RISC-V 32-bit
    RiscV32,
    /// RISC-V 64-bit
    RiscV64,
}

impl MachineType {
    /// Get pointer size in bytes
    pub fn pointer_size(&self) -> usize {
        match self {
            Self::X86 | Self::Arm | Self::RiscV32 => 4,
            Self::X86_64 | Self::Aarch64 | Self::RiscV64 => 8,
            Self::Unknown => 0,
        }
    }

    /// Check if 64-bit
    pub fn is_64bit(&self) -> bool {
        matches!(self, Self::X86_64 | Self::Aarch64 | Self::RiscV64)
    }
}

/// Memory requirements
#[derive(Debug, Clone, Default)]
pub struct MemoryRequirements {
    /// Code size
    pub code_size: u64,
    /// Data size
    pub data_size: u64,
    /// Read-only data size
    pub rodata_size: u64,
    /// BSS size
    pub bss_size: u64,
    /// Total size
    pub total_size: u64,
}

// =============================================================================
// EXECUTION CONTEXT
// =============================================================================

/// Context for kernel execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Entry point address
    pub entry_point: VirtualAddress,
    /// Stack top
    pub stack_top: VirtualAddress,
    /// Page table root
    pub page_table_root: PhysicalAddress,
    /// Boot info address
    pub boot_info_address: VirtualAddress,
}

impl ExecutionContext {
    /// Jump to kernel (never returns)
    pub unsafe fn execute(&self, boot_info: *const u8) -> ! {
        #[cfg(target_arch = "x86_64")]
        {
            core::arch::asm!(
                "mov rsp, {stack}",
                "mov rdi, {boot_info}",
                "jmp {entry}",
                stack = in(reg) self.stack_top.0 as usize,
                boot_info = in(reg) boot_info,
                entry = in(reg) self.entry_point.0 as usize,
                options(noreturn)
            );
        }

        #[cfg(not(target_arch = "x86_64"))]
        loop {}
    }
}

// =============================================================================
// MODULE LOADER
// =============================================================================

/// Kernel module loader
pub struct ModuleLoader {
    /// Loaded modules
    modules: Vec<LoadedModule>,
    /// Base loader
    loader: KernelLoader,
}

impl ModuleLoader {
    /// Create new module loader
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            loader: KernelLoader::new(),
        }
    }

    /// Load module
    pub fn load_module(&mut self, name: &str, data: &[u8]) -> Result<usize> {
        let mut loader = KernelLoader::new();
        loader.load(data)?;

        let image = loader.image().ok_or(Error::NotLoaded)?;

        let module = LoadedModule {
            name: String::from(name),
            image: image.clone(),
            id: self.modules.len(),
            state: ModuleState::Loaded,
            dependencies: Vec::new(),
        };

        self.modules.push(module);
        Ok(self.modules.len() - 1)
    }

    /// Get module by ID
    pub fn get_module(&self, id: usize) -> Option<&LoadedModule> {
        self.modules.get(id)
    }

    /// Get module by name
    pub fn find_module(&self, name: &str) -> Option<&LoadedModule> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Get all modules
    pub fn modules(&self) -> &[LoadedModule] {
        &self.modules
    }

    /// Module count
    pub fn count(&self) -> usize {
        self.modules.len()
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Loaded module
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// Module name
    pub name: String,
    /// Loaded image
    pub image: LoadedImage,
    /// Module ID
    pub id: usize,
    /// Module state
    pub state: ModuleState,
    /// Dependencies
    pub dependencies: Vec<usize>,
}

/// Module state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    /// Loaded but not initialized
    Loaded,
    /// Initializing
    Initializing,
    /// Ready
    Ready,
    /// Failed
    Failed,
    /// Unloading
    Unloading,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_detection() {
        // ELF magic
        let elf64 = [0x7F, b'E', b'L', b'F', 2, 1, 1, 0];
        assert_eq!(ImageFormat::detect(&elf64).unwrap(), ImageFormat::Elf64);

        let elf32 = [0x7F, b'E', b'L', b'F', 1, 1, 1, 0];
        assert_eq!(ImageFormat::detect(&elf32).unwrap(), ImageFormat::Elf32);
    }

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert!(!config.verify_signature);
        assert!(config.apply_relocations);
        assert!(config.load_symbols);
    }

    #[test]
    fn test_machine_type() {
        assert_eq!(MachineType::X86_64.pointer_size(), 8);
        assert_eq!(MachineType::X86.pointer_size(), 4);
        assert!(MachineType::X86_64.is_64bit());
        assert!(!MachineType::X86.is_64bit());
    }
}
