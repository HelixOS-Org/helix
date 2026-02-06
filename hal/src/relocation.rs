//! # Helix Kernel Relocation Engine
//!
//! This module provides comprehensive runtime relocation support for the Helix kernel,
//! enabling Position Independent Executable (PIE) kernels that can load at any address.
//!
//! ## Features
//!
//! - **Full x86_64 Relocation Support**: Handles all common ELF relocation types
//! - **KASLR Ready**: Supports randomized kernel load addresses
//! - **Zero-Copy**: Relocations applied in-place without memory allocation
//! - **Validation**: Comprehensive bounds checking and error detection
//! - **Statistics**: Detailed relocation metrics for debugging
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │   ELF Parser    │────▶│ Reloc Context   │────▶│ Apply Relocs    │
//! │   (find .rela)  │     │ (compute slide) │     │ (patch memory)  │
//! └─────────────────┘     └─────────────────┘     └─────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use hal::relocation::{apply_relocations, RelocationContext};
//!
//! // Create context with slide information
//! let ctx = RelocationContext::new(
//!     0xFFFFFFFF80000000, // Link base (from linker script)
//!     0xFFFFFFFF82000000, // Actual load address
//!     kernel_size,
//! );
//!
//! // Apply relocations
//! unsafe {
//!     let stats = apply_relocations(&ctx, kernel_base, rela_entries)?;
//!     log::info!("Applied {} relocations", stats.total_applied);
//! }
//! ```

#![allow(dead_code)]
// This module extensively uses unsafe for low-level ELF manipulation.
// All unsafe operations are carefully documented and bounds-checked.
#![allow(unsafe_op_in_unsafe_fn)]

use core::{fmt, ptr};

// ============================================================================
// TYPES AND CONSTANTS
// ============================================================================

/// Relocation result type
pub type RelocResult<T> = Result<T, RelocError>;

/// Relocation error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocError {
    /// Invalid ELF magic number
    InvalidElfMagic,
    /// Invalid ELF class (not 64-bit)
    InvalidElfClass,
    /// Invalid ELF machine type (not x86_64)
    InvalidElfMachine,
    /// Unsupported relocation type
    UnsupportedRelocType(u32),
    /// Relocation target out of kernel bounds
    OutOfBounds {
        /// Byte offset of the relocation within the kernel image
        offset: u64,
        /// Total size of the kernel image in bytes
        size: u64,
    },
    /// Overflow during relocation calculation
    Overflow {
        /// Byte offset where the overflow occurred
        offset: u64,
        /// The computed relocation value that caused overflow
        value: i64,
    },
    /// No relocations found (not necessarily an error)
    NoRelocations,
    /// Too many relocation errors
    TooManyErrors(usize),
    /// Validation failed after relocation
    ValidationFailed(&'static str),
    /// Section not found
    SectionNotFound(&'static str),
    /// Symbol not found
    SymbolNotFound(u32),
    /// Invalid addend value
    InvalidAddend,
}

impl fmt::Display for RelocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidElfMagic => write!(f, "Invalid ELF magic number"),
            Self::InvalidElfClass => write!(f, "Invalid ELF class (expected 64-bit)"),
            Self::InvalidElfMachine => write!(f, "Invalid ELF machine (expected x86_64)"),
            Self::UnsupportedRelocType(t) => write!(f, "Unsupported relocation type: {}", t),
            Self::OutOfBounds { offset, size } => {
                write!(f, "Relocation at offset {} exceeds size {}", offset, size)
            },
            Self::Overflow { offset, value } => {
                write!(f, "Overflow at offset {}: value {}", offset, value)
            },
            Self::NoRelocations => write!(f, "No relocations found"),
            Self::TooManyErrors(n) => write!(f, "Too many relocation errors: {}", n),
            Self::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            Self::SectionNotFound(name) => write!(f, "Section not found: {}", name),
            Self::SymbolNotFound(idx) => write!(f, "Symbol not found: index {}", idx),
            Self::InvalidAddend => write!(f, "Invalid relocation addend"),
        }
    }
}

// ============================================================================
// x86_64 RELOCATION TYPES
// ============================================================================

/// x86_64 ELF relocation types (from System V AMD64 ABI)
pub mod reloc_x86_64 {
    /// No relocation
    pub const R_X86_64_NONE: u32 = 0;
    /// S + A (64-bit absolute)
    pub const R_X86_64_64: u32 = 1;
    /// S + A - P (32-bit PC-relative)
    pub const R_X86_64_PC32: u32 = 2;
    /// G + A (32-bit GOT entry)
    pub const R_X86_64_GOT32: u32 = 3;
    /// L + A - P (32-bit PLT address)
    pub const R_X86_64_PLT32: u32 = 4;
    /// Copy symbol at runtime
    pub const R_X86_64_COPY: u32 = 5;
    /// Create GOT entry
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    /// Create PLT entry
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    /// B + A (base-relative, most common in PIE)
    pub const R_X86_64_RELATIVE: u32 = 8;
    /// G + GOT + A - P (32-bit PC-relative GOT)
    pub const R_X86_64_GOTPCREL: u32 = 9;
    /// S + A (32-bit zero-extended)
    pub const R_X86_64_32: u32 = 10;
    /// S + A (32-bit sign-extended)
    pub const R_X86_64_32S: u32 = 11;
    /// S + A (16-bit)
    pub const R_X86_64_16: u32 = 12;
    /// S + A - P (16-bit PC-relative)
    pub const R_X86_64_PC16: u32 = 13;
    /// S + A (8-bit)
    pub const R_X86_64_8: u32 = 14;
    /// S + A - P (8-bit PC-relative)
    pub const R_X86_64_PC8: u32 = 15;
    /// PC-relative 64-bit
    pub const R_X86_64_PC64: u32 = 24;
    /// 64-bit GOT offset
    pub const R_X86_64_GOTOFF64: u32 = 25;
    /// PC-relative with 32-bit sign extension
    pub const R_X86_64_GOTPC32: u32 = 26;
    /// 64-bit GOT entry
    pub const R_X86_64_GOT64: u32 = 27;
    /// PC-relative 64-bit GOT entry
    pub const R_X86_64_GOTPCREL64: u32 = 28;
    /// PC-relative 64-bit GOT
    pub const R_X86_64_GOTPC64: u32 = 29;
    /// PC-relative PLT with 64-bit GOT
    pub const R_X86_64_PLTOFF64: u32 = 31;
    /// Size of symbol
    pub const R_X86_64_SIZE32: u32 = 32;
    /// Size of symbol (64-bit)
    pub const R_X86_64_SIZE64: u32 = 33;
    /// PC-relative GOT with relaxation
    pub const R_X86_64_GOTPCRELX: u32 = 41;
    /// Relaxed PC-relative GOT
    pub const R_X86_64_REX_GOTPCRELX: u32 = 42;
}

/// Returns a human-readable name for an x86_64 relocation type.
///
/// Unknown relocation types return `"R_X86_64_UNKNOWN"`.
pub fn reloc_type_name(rtype: u32) -> &'static str {
    use reloc_x86_64::*;
    match rtype {
        R_X86_64_NONE => "R_X86_64_NONE",
        R_X86_64_64 => "R_X86_64_64",
        R_X86_64_PC32 => "R_X86_64_PC32",
        R_X86_64_GOT32 => "R_X86_64_GOT32",
        R_X86_64_PLT32 => "R_X86_64_PLT32",
        R_X86_64_COPY => "R_X86_64_COPY",
        R_X86_64_GLOB_DAT => "R_X86_64_GLOB_DAT",
        R_X86_64_JUMP_SLOT => "R_X86_64_JUMP_SLOT",
        R_X86_64_RELATIVE => "R_X86_64_RELATIVE",
        R_X86_64_GOTPCREL => "R_X86_64_GOTPCREL",
        R_X86_64_32 => "R_X86_64_32",
        R_X86_64_32S => "R_X86_64_32S",
        R_X86_64_PC64 => "R_X86_64_PC64",
        R_X86_64_GOTPCRELX => "R_X86_64_GOTPCRELX",
        R_X86_64_REX_GOTPCRELX => "R_X86_64_REX_GOTPCRELX",
        _ => "R_X86_64_UNKNOWN",
    }
}

// ============================================================================
// ELF STRUCTURES
// ============================================================================

/// ELF64 file header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    /// Magic number and other info
    pub e_ident: [u8; 16],
    /// Object file type
    pub e_type: u16,
    /// Architecture
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u64,
    /// Program header table file offset
    pub e_phoff: u64,
    /// Section header table file offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size in bytes
    pub e_ehsize: u16,
    /// Program header table entry size
    pub e_phentsize: u16,
    /// Program header table entry count
    pub e_phnum: u16,
    /// Section header table entry size
    pub e_shentsize: u16,
    /// Section header table entry count
    pub e_shnum: u16,
    /// Section header string table index
    pub e_shstrndx: u16,
}

impl Elf64Header {
    /// ELF magic number
    pub const MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
    /// 64-bit ELF class
    pub const CLASS64: u8 = 2;
    /// Little endian
    pub const DATA_LSB: u8 = 1;
    /// x86_64 machine type
    pub const EM_X86_64: u16 = 62;
    /// Executable type
    pub const ET_EXEC: u16 = 2;
    /// Shared object / PIE
    pub const ET_DYN: u16 = 3;

    /// Validate ELF header
    pub fn validate(&self) -> RelocResult<()> {
        // Check magic
        if self.e_ident[0..4] != Self::MAGIC {
            return Err(RelocError::InvalidElfMagic);
        }

        // Check 64-bit
        if self.e_ident[4] != Self::CLASS64 {
            return Err(RelocError::InvalidElfClass);
        }

        // Check x86_64
        if self.e_machine != Self::EM_X86_64 {
            return Err(RelocError::InvalidElfMachine);
        }

        Ok(())
    }

    /// Check if this is a PIE/shared object
    pub fn is_pie(&self) -> bool {
        self.e_type == Self::ET_DYN
    }
}

/// ELF64 program header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Segment file offset
    pub p_offset: u64,
    /// Segment virtual address
    pub p_vaddr: u64,
    /// Segment physical address
    pub p_paddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Segment alignment
    pub p_align: u64,
}

impl Elf64ProgramHeader {
    /// Loadable segment
    pub const PT_LOAD: u32 = 1;
    /// Dynamic linking info
    pub const PT_DYNAMIC: u32 = 2;
    /// GNU relro
    pub const PT_GNU_RELRO: u32 = 0x6474E552;
}

/// ELF64 section header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64SectionHeader {
    /// Section name (string table index)
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Section virtual address
    pub sh_addr: u64,
    /// Section file offset
    pub sh_offset: u64,
    /// Section size in bytes
    pub sh_size: u64,
    /// Link to another section
    pub sh_link: u32,
    /// Additional section info
    pub sh_info: u32,
    /// Section alignment
    pub sh_addralign: u64,
    /// Entry size if section holds table
    pub sh_entsize: u64,
}

impl Elf64SectionHeader {
    /// Relocation entries with addends
    pub const SHT_RELA: u32 = 4;
    /// Dynamic linking info
    pub const SHT_DYNAMIC: u32 = 6;
    /// Symbol table
    pub const SHT_DYNSYM: u32 = 11;
}

/// ELF64 relocation entry with addend
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Rela {
    /// Location at which to apply the relocation
    pub r_offset: u64,
    /// Relocation type and symbol index
    pub r_info: u64,
    /// Constant addend used to compute value
    pub r_addend: i64,
}

impl Elf64Rela {
    /// Size of a relocation entry
    pub const SIZE: usize = 24;

    /// Extract relocation type from r_info
    #[inline]
    pub fn r_type(&self) -> u32 {
        (self.r_info & 0xFFFFFFFF) as u32
    }

    /// Extract symbol index from r_info
    #[inline]
    pub fn r_sym(&self) -> u32 {
        (self.r_info >> 32) as u32
    }

    /// Create r_info from type and symbol
    #[inline]
    pub fn make_info(sym: u32, rtype: u32) -> u64 {
        ((sym as u64) << 32) | (rtype as u64)
    }
}

/// ELF64 symbol table entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Sym {
    /// Symbol name (string table index)
    pub st_name: u32,
    /// Symbol type and binding
    pub st_info: u8,
    /// Symbol visibility
    pub st_other: u8,
    /// Section index
    pub st_shndx: u16,
    /// Symbol value
    pub st_value: u64,
    /// Symbol size
    pub st_size: u64,
}

impl Elf64Sym {
    /// Size of a symbol entry
    pub const SIZE: usize = 24;
}

/// Dynamic section entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Dyn {
    /// Dynamic entry type
    pub d_tag: i64,
    /// Integer or pointer value
    pub d_val: u64,
}

impl Elf64Dyn {
    /// End of dynamic section
    pub const DT_NULL: i64 = 0;
    /// Address of relocation table
    pub const DT_RELA: i64 = 7;
    /// Size of relocation table
    pub const DT_RELASZ: i64 = 8;
    /// Size of relocation entry
    pub const DT_RELAENT: i64 = 9;
    /// Address of symbol table
    pub const DT_SYMTAB: i64 = 6;
    /// Address of string table
    pub const DT_STRTAB: i64 = 5;
    /// Count of RELA relocations
    pub const DT_RELACOUNT: i64 = 0x6FFFFFF9;
}

// ============================================================================
// RELOCATION CONTEXT
// ============================================================================

/// Relocation context containing all information needed for relocation
#[derive(Debug, Clone)]
pub struct RelocationContext {
    /// Base address where kernel was linked (from linker script)
    pub link_base: u64,
    /// Actual address where kernel was loaded
    pub load_base: u64,
    /// Total kernel size in bytes
    pub kernel_size: usize,
    /// Slide offset (load_base - link_base)
    pub slide: i64,
    /// GOT base address (if applicable)
    pub got_base: Option<u64>,
    /// Whether strict mode is enabled (fail on unknown relocs)
    pub strict_mode: bool,
    /// Maximum allowed errors before failing
    pub max_errors: usize,
}

impl RelocationContext {
    /// Create a new relocation context
    pub fn new(link_base: u64, load_base: u64, kernel_size: usize) -> Self {
        let slide = (load_base as i128 - link_base as i128) as i64;
        Self {
            link_base,
            load_base,
            kernel_size,
            slide,
            got_base: None,
            strict_mode: false,
            max_errors: 16, // Allow some non-critical errors
        }
    }

    /// Sets the Global Offset Table (GOT) base address and returns self.
    pub fn with_got(mut self, got_base: u64) -> Self {
        self.got_base = Some(got_base);
        self
    }

    /// Enables strict mode where any relocation error causes immediate failure.
    pub fn strict(mut self) -> Self {
        self.strict_mode = true;
        self.max_errors = 0;
        self
    }

    /// Returns `true` if the given offset is within the kernel's memory bounds.
    #[inline]
    pub fn in_bounds(&self, offset: u64) -> bool {
        offset < self.kernel_size as u64
    }

    /// Translates a linked (compile-time) address to the actual loaded address.
    #[inline]
    pub fn translate(&self, linked_addr: u64) -> u64 {
        (linked_addr as i128 + self.slide as i128) as u64
    }

    /// Converts a linked address to an offset from the kernel base, if within bounds.
    #[inline]
    pub fn linked_to_offset(&self, linked_addr: u64) -> Option<u64> {
        if linked_addr >= self.link_base {
            let offset = linked_addr - self.link_base;
            if offset < self.kernel_size as u64 {
                return Some(offset);
            }
        }
        None
    }
}

// ============================================================================
// RELOCATION STATISTICS
// ============================================================================

/// Detailed relocation statistics
#[derive(Debug, Default, Clone)]
pub struct RelocStats {
    /// Total relocation entries processed
    pub total_entries: usize,
    /// Successfully applied relocations
    pub total_applied: usize,
    /// Skipped relocations (R_NONE, etc.)
    pub skipped: usize,
    /// Errors encountered
    pub errors: usize,

    // Per-type counts
    /// Count of R_X86_64_NONE relocations
    pub r_none: usize,
    /// Count of R_X86_64_RELATIVE relocations
    pub r_relative: usize,
    /// Count of R_X86_64_64 relocations
    pub r_64: usize,
    /// Count of R_X86_64_32 relocations
    pub r_32: usize,
    /// Count of R_X86_64_32S relocations
    pub r_32s: usize,
    /// Count of R_X86_64_PC32 relocations
    pub r_pc32: usize,
    /// Count of R_X86_64_PC64 relocations
    pub r_pc64: usize,
    /// Count of GOT-related relocations
    pub r_got: usize,
    /// Count of PLT-related relocations
    pub r_plt: usize,
    /// Count of other/unknown relocations
    pub r_other: usize,
}

impl RelocStats {
    /// Creates a new `RelocStats` with all counters initialized to zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the number of errors is within the acceptable limit.
    pub fn is_success(&self, max_errors: usize) -> bool {
        self.errors <= max_errors
    }

    /// Calculates the success rate as a percentage (0.0 to 100.0).
    pub fn success_rate(&self) -> f32 {
        if self.total_entries == 0 {
            100.0
        } else {
            ((self.total_applied + self.skipped) as f32 / self.total_entries as f32) * 100.0
        }
    }
}

impl fmt::Display for RelocStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Relocation Statistics:")?;
        writeln!(f, "  Total entries:  {}", self.total_entries)?;
        writeln!(f, "  Applied:        {}", self.total_applied)?;
        writeln!(f, "  Skipped:        {}", self.skipped)?;
        writeln!(f, "  Errors:         {}", self.errors)?;
        writeln!(f, "  Success rate:   {:.1}%", self.success_rate())?;
        writeln!(f, "  By type:")?;
        writeln!(f, "    R_RELATIVE:   {}", self.r_relative)?;
        writeln!(f, "    R_64:         {}", self.r_64)?;
        writeln!(f, "    R_PC32:       {}", self.r_pc32)?;
        writeln!(f, "    R_32/32S:     {}/{}", self.r_32, self.r_32s)?;
        writeln!(f, "    R_GOT:        {}", self.r_got)?;
        writeln!(f, "    R_PLT:        {}", self.r_plt)?;
        writeln!(f, "    R_NONE:       {}", self.r_none)?;
        write!(f, "    Other:        {}", self.r_other)
    }
}

// ============================================================================
// CORE RELOCATION ENGINE
// ============================================================================

/// Apply a single relocation entry
///
/// # Safety
///
/// - `kernel_base` must point to valid, writable memory
/// - The target address must be within the kernel's memory bounds
#[inline]
unsafe fn apply_single_reloc(
    ctx: &RelocationContext,
    kernel_base: *mut u8,
    rela: &Elf64Rela,
    stats: &mut RelocStats,
) -> RelocResult<()> {
    use reloc_x86_64::*;

    let rtype = rela.r_type();
    let offset = rela.r_offset;

    // Calculate the offset within the loaded kernel
    // The r_offset is relative to link_base, we need offset from load_base
    let target_offset = match ctx.linked_to_offset(offset) {
        Some(off) => off,
        None => {
            // Try direct offset (for non-PIE sections)
            if offset < ctx.kernel_size as u64 {
                offset
            } else {
                stats.errors += 1;
                return Err(RelocError::OutOfBounds {
                    offset,
                    size: ctx.kernel_size as u64,
                });
            }
        },
    };

    // Bounds check
    if target_offset >= ctx.kernel_size as u64 - 8 {
        stats.errors += 1;
        return Err(RelocError::OutOfBounds {
            offset: target_offset,
            size: ctx.kernel_size as u64,
        });
    }

    // SAFETY: We have verified target_offset is within bounds
    let target_ptr = unsafe { kernel_base.add(target_offset as usize) };

    match rtype {
        R_X86_64_NONE => {
            // No action needed
            stats.r_none += 1;
            stats.skipped += 1;
        },

        R_X86_64_RELATIVE => {
            // B + A: Base-relative relocation
            // This is the most common type for PIE kernels
            // *target = load_base + addend
            let value = (ctx.load_base as i128 + rela.r_addend as i128) as u64;
            // SAFETY: target_ptr is valid and within bounds
            unsafe { ptr::write_unaligned(target_ptr as *mut u64, value) };
            stats.r_relative += 1;
            stats.total_applied += 1;
        },

        R_X86_64_64 => {
            // S + A: 64-bit absolute address
            // Add slide to existing value
            // SAFETY: target_ptr is valid and within bounds
            let current = unsafe { ptr::read_unaligned(target_ptr as *const u64) };
            let new_value = (current as i128 + ctx.slide as i128) as u64;
            unsafe { ptr::write_unaligned(target_ptr as *mut u64, new_value) };
            stats.r_64 += 1;
            stats.total_applied += 1;
        },

        R_X86_64_32 => {
            // S + A: 32-bit zero-extended
            // SAFETY: target_ptr is valid and within bounds
            let current = unsafe { ptr::read_unaligned(target_ptr as *const u32) };
            let new_value = (current as i64 + ctx.slide) as u32;

            // Check for overflow (value must fit in 32 bits unsigned)
            let full_value = current as i64 + ctx.slide;
            if full_value < 0 || full_value > u32::MAX as i64 {
                stats.errors += 1;
                return Err(RelocError::Overflow {
                    offset: target_offset,
                    value: full_value,
                });
            }

            unsafe { ptr::write_unaligned(target_ptr as *mut u32, new_value) };
            stats.r_32 += 1;
            stats.total_applied += 1;
        },

        R_X86_64_32S => {
            // S + A: 32-bit sign-extended
            // SAFETY: target_ptr is valid and within bounds
            let current = unsafe { ptr::read_unaligned(target_ptr as *const i32) };
            let new_value = (current as i64 + ctx.slide) as i32;

            // Check for overflow (value must fit in 32 bits signed)
            let full_value = current as i64 + ctx.slide;
            if full_value < i32::MIN as i64 || full_value > i32::MAX as i64 {
                stats.errors += 1;
                return Err(RelocError::Overflow {
                    offset: target_offset,
                    value: full_value,
                });
            }

            unsafe { ptr::write_unaligned(target_ptr as *mut i32, new_value) };
            stats.r_32s += 1;
            stats.total_applied += 1;
        },

        R_X86_64_PC32 | R_X86_64_PLT32 => {
            // S + A - P: 32-bit PC-relative
            // For internal references, no adjustment needed (slide cancels out)
            // For external symbols, would need symbol resolution
            stats.r_pc32 += 1;
            stats.skipped += 1; // Usually no action needed for internal refs
        },

        R_X86_64_PC64 => {
            // S + A - P: 64-bit PC-relative
            // Similar to PC32, usually no adjustment needed
            stats.r_pc64 += 1;
            stats.skipped += 1;
        },

        R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
            // GOT/PLT entries - would need symbol resolution
            // For a self-contained kernel, these typically point to internal symbols
            // SAFETY: target_ptr is valid and within bounds
            let current = unsafe { ptr::read_unaligned(target_ptr as *const u64) };
            let new_value = (current as i128 + ctx.slide as i128) as u64;
            unsafe { ptr::write_unaligned(target_ptr as *mut u64, new_value) };
            stats.r_got += 1;
            stats.total_applied += 1;
        },

        R_X86_64_GOTPCREL | R_X86_64_GOTPCRELX | R_X86_64_REX_GOTPCRELX => {
            // GOT-relative PC-relative
            // These are used for accessing global variables through GOT
            // In a PIE kernel, GOT entries need adjustment
            stats.r_got += 1;
            stats.skipped += 1; // GOT entries handled by GLOB_DAT
        },

        R_X86_64_COPY => {
            // Symbol copy - not expected in kernel
            stats.r_other += 1;
            stats.skipped += 1;
        },

        _ => {
            // Unknown/unsupported relocation type
            stats.r_other += 1;
            stats.errors += 1;

            if ctx.strict_mode {
                return Err(RelocError::UnsupportedRelocType(rtype));
            }
        },
    }

    Ok(())
}

/// Apply all relocations from a slice of Elf64Rela entries
///
/// # Safety
///
/// - `kernel_base` must point to valid, writable kernel memory
/// - `rela_entries` must contain valid relocation entries
///
/// # Arguments
///
/// * `ctx` - Relocation context with slide information
/// * `kernel_base` - Pointer to the start of the loaded kernel
/// * `rela_entries` - Slice of relocation entries to apply
///
/// # Returns
///
/// Statistics about the relocation process, or an error if relocation failed
pub unsafe fn apply_relocations(
    ctx: &RelocationContext,
    kernel_base: *mut u8,
    rela_entries: &[Elf64Rela],
) -> RelocResult<RelocStats> {
    let mut stats = RelocStats::new();
    stats.total_entries = rela_entries.len();

    // Fast path: no slide means no relocation needed
    if ctx.slide == 0 {
        stats.skipped = stats.total_entries;
        return Ok(stats);
    }

    for rela in rela_entries {
        // SAFETY: caller guarantees kernel_base and rela are valid
        if let Err(_e) = unsafe { apply_single_reloc(ctx, kernel_base, rela, &mut stats) } {
            // Error already counted in stats
            if stats.errors > ctx.max_errors {
                return Err(RelocError::TooManyErrors(stats.errors));
            }
        }
    }

    if stats.errors > ctx.max_errors {
        Err(RelocError::TooManyErrors(stats.errors))
    } else {
        Ok(stats)
    }
}

// ============================================================================
// ELF PARSING HELPERS
// ============================================================================

/// Find the .rela.dyn section in an ELF file
///
/// # Safety
///
/// - `elf_base` must point to a valid ELF file
/// - `elf_size` must be the actual size of the ELF data
pub unsafe fn find_rela_dyn_section(
    elf_base: *const u8,
    elf_size: usize,
) -> RelocResult<(*const Elf64Rela, usize)> {
    // Validate minimum size
    if elf_size < core::mem::size_of::<Elf64Header>() {
        return Err(RelocError::InvalidElfMagic);
    }

    let header = &*(elf_base as *const Elf64Header);
    header.validate()?;

    // Get section headers
    let shoff = header.e_shoff as usize;
    let shnum = header.e_shnum as usize;
    let shentsize = header.e_shentsize as usize;
    let shstrndx = header.e_shstrndx as usize;

    if shoff == 0 || shnum == 0 {
        return Err(RelocError::SectionNotFound(".rela.dyn"));
    }

    if shoff + shnum * shentsize > elf_size {
        return Err(RelocError::OutOfBounds {
            offset: shoff as u64,
            size: elf_size as u64,
        });
    }

    // Get string table section
    let shstrtab_hdr = &*(elf_base.add(shoff + shstrndx * shentsize) as *const Elf64SectionHeader);
    let shstrtab_offset = shstrtab_hdr.sh_offset as usize;
    let shstrtab_size = shstrtab_hdr.sh_size as usize;

    if shstrtab_offset + shstrtab_size > elf_size {
        return Err(RelocError::SectionNotFound(".shstrtab"));
    }

    let shstrtab = core::slice::from_raw_parts(elf_base.add(shstrtab_offset), shstrtab_size);

    // Search for .rela.dyn section
    for i in 0..shnum {
        let sh = &*(elf_base.add(shoff + i * shentsize) as *const Elf64SectionHeader);

        // Only look at RELA sections
        if sh.sh_type != Elf64SectionHeader::SHT_RELA {
            continue;
        }

        // Get section name
        let name_offset = sh.sh_name as usize;
        if name_offset >= shstrtab_size {
            continue;
        }

        let name_bytes = &shstrtab[name_offset..];
        let name_end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let name = match core::str::from_utf8(&name_bytes[..name_end]) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if name == ".rela.dyn" {
            let rela_offset = sh.sh_offset as usize;
            let rela_size = sh.sh_size as usize;
            let rela_count = rela_size / Elf64Rela::SIZE;

            if rela_offset + rela_size > elf_size {
                return Err(RelocError::OutOfBounds {
                    offset: rela_offset as u64,
                    size: elf_size as u64,
                });
            }

            let rela_ptr = elf_base.add(rela_offset) as *const Elf64Rela;
            return Ok((rela_ptr, rela_count));
        }
    }

    Err(RelocError::SectionNotFound(".rela.dyn"))
}

/// Find relocations using the DYNAMIC segment (alternative method)
///
/// This is more reliable for PIE executables as it uses runtime info.
///
/// # Safety
///
/// Same requirements as `find_rela_dyn_section`
pub unsafe fn find_rela_from_dynamic(
    elf_base: *const u8,
    elf_size: usize,
) -> RelocResult<(*const Elf64Rela, usize)> {
    if elf_size < core::mem::size_of::<Elf64Header>() {
        return Err(RelocError::InvalidElfMagic);
    }

    let header = &*(elf_base as *const Elf64Header);
    header.validate()?;

    // Find PT_DYNAMIC segment
    let phoff = header.e_phoff as usize;
    let phnum = header.e_phnum as usize;
    let phentsize = header.e_phentsize as usize;

    if phoff + phnum * phentsize > elf_size {
        return Err(RelocError::OutOfBounds {
            offset: phoff as u64,
            size: elf_size as u64,
        });
    }

    let mut dynamic_offset = None;
    let mut dynamic_size = None;

    for i in 0..phnum {
        let ph = &*(elf_base.add(phoff + i * phentsize) as *const Elf64ProgramHeader);
        if ph.p_type == Elf64ProgramHeader::PT_DYNAMIC {
            dynamic_offset = Some(ph.p_offset as usize);
            dynamic_size = Some(ph.p_memsz as usize);
            break;
        }
    }

    let (dyn_off, dyn_sz) = match (dynamic_offset, dynamic_size) {
        (Some(off), Some(sz)) => (off, sz),
        _ => return Err(RelocError::SectionNotFound(".dynamic")),
    };

    if dyn_off + dyn_sz > elf_size {
        return Err(RelocError::OutOfBounds {
            offset: dyn_off as u64,
            size: elf_size as u64,
        });
    }

    // Parse dynamic section
    let mut rela_addr: Option<u64> = None;
    let mut rela_size: Option<u64> = None;
    let mut rela_ent: Option<u64> = None;

    let dyn_entries = dyn_sz / core::mem::size_of::<Elf64Dyn>();
    for i in 0..dyn_entries {
        let dyn_entry =
            &*(elf_base.add(dyn_off + i * core::mem::size_of::<Elf64Dyn>()) as *const Elf64Dyn);

        match dyn_entry.d_tag {
            Elf64Dyn::DT_NULL => break,
            Elf64Dyn::DT_RELA => rela_addr = Some(dyn_entry.d_val),
            Elf64Dyn::DT_RELASZ => rela_size = Some(dyn_entry.d_val),
            Elf64Dyn::DT_RELAENT => rela_ent = Some(dyn_entry.d_val),
            _ => {},
        }
    }

    match (rela_addr, rela_size, rela_ent) {
        (Some(addr), Some(size), Some(ent)) => {
            // The address in DT_RELA is a virtual address, need to convert to file offset
            // For PIE, this is typically just an offset from the start
            let rela_offset = addr as usize;
            let rela_count = (size / ent) as usize;

            if rela_offset + size as usize > elf_size {
                return Err(RelocError::OutOfBounds {
                    offset: rela_offset as u64,
                    size: elf_size as u64,
                });
            }

            let rela_ptr = elf_base.add(rela_offset) as *const Elf64Rela;
            Ok((rela_ptr, rela_count))
        },
        _ => Err(RelocError::SectionNotFound(".rela.dyn (from DYNAMIC)")),
    }
}

// ============================================================================
// HIGH-LEVEL API
// ============================================================================

/// Relocate an ELF kernel loaded at a given base address
///
/// This is the main entry point for kernel relocation.
///
/// # Safety
///
/// - `elf_data` must point to valid, writable ELF data
/// - `elf_size` must be the actual size of the data
///
/// # Arguments
///
/// * `elf_data` - Pointer to the loaded kernel ELF data
/// * `elf_size` - Size of the ELF data
/// * `link_base` - Virtual address where the kernel was linked
/// * `load_base` - Virtual address where the kernel is actually loaded
///
/// # Returns
///
/// Relocation statistics on success, or an error
pub unsafe fn relocate_kernel(
    elf_data: *mut u8,
    elf_size: usize,
    link_base: u64,
    load_base: u64,
) -> RelocResult<RelocStats> {
    // Create relocation context
    let ctx = RelocationContext::new(link_base, load_base, elf_size);

    // Fast path: no relocation needed if loaded at link address
    if ctx.slide == 0 {
        return Ok(RelocStats {
            total_entries: 0,
            skipped: 0,
            total_applied: 0,
            ..Default::default()
        });
    }

    // Find relocations - try section headers first, then DYNAMIC
    let (rela_ptr, rela_count) = find_rela_dyn_section(elf_data, elf_size)
        .or_else(|_| find_rela_from_dynamic(elf_data, elf_size))?;

    if rela_count == 0 {
        return Err(RelocError::NoRelocations);
    }

    // Create slice from raw pointer
    let rela_entries = core::slice::from_raw_parts(rela_ptr, rela_count);

    // Apply relocations
    apply_relocations(&ctx, elf_data, rela_entries)
}

/// Validate that relocation was successful
///
/// Performs basic sanity checks on the relocated kernel.
pub fn validate_relocation(ctx: &RelocationContext, kernel_base: *const u8) -> RelocResult<()> {
    // Check kernel is accessible
    if kernel_base.is_null() {
        return Err(RelocError::ValidationFailed("kernel base is null"));
    }

    // Check size is reasonable
    if ctx.kernel_size == 0 || ctx.kernel_size > 1024 * 1024 * 1024 {
        // > 1GB
        return Err(RelocError::ValidationFailed("invalid kernel size"));
    }

    // Check slide is reasonable
    let max_slide = 1i64 << 40; // 1TB
    if ctx.slide.abs() > max_slide {
        return Err(RelocError::ValidationFailed("slide too large"));
    }

    // TODO: Add more validation:
    // - Check ELF magic at kernel_base
    // - Verify critical symbols are valid
    // - Check function pointers in vtables

    Ok(())
}

// ============================================================================
// DEBUG HELPERS
// ============================================================================

/// Prints detailed information about a single relocation entry.
///
/// Outputs the offset, type, symbol index, and addend for debugging purposes.
#[cfg(feature = "debug_reloc")]
pub fn debug_print_rela(rela: &Elf64Rela, index: usize) {
    let rtype = rela.r_type();
    let sym = rela.r_sym();

    log::debug!(
        "  [{:4}] offset=0x{:016x} type={:2} ({}) sym={} addend={}",
        index,
        rela.r_offset,
        rtype,
        reloc_type_name(rtype),
        sym,
        rela.r_addend
    );
}

/// Dumps all relocation entries to the debug log.
///
/// Iterates through all entries and prints their details using `debug_print_rela`.
#[cfg(feature = "debug_reloc")]
pub fn debug_dump_relocations(rela_entries: &[Elf64Rela]) {
    log::debug!("Relocation entries ({}):", rela_entries.len());
    for (i, rela) in rela_entries.iter().enumerate() {
        debug_print_rela(rela, i);
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reloc_context() {
        let ctx = RelocationContext::new(0x1000, 0x2000, 0x1000);
        assert_eq!(ctx.slide, 0x1000);
        assert_eq!(ctx.translate(0x1000), 0x2000);
        assert_eq!(ctx.translate(0x1500), 0x2500);
    }

    #[test]
    fn test_zero_slide() {
        let ctx = RelocationContext::new(0x1000, 0x1000, 0x1000);
        assert_eq!(ctx.slide, 0);
    }

    #[test]
    fn test_negative_slide() {
        let ctx = RelocationContext::new(0x2000, 0x1000, 0x1000);
        assert_eq!(ctx.slide, -0x1000);
        assert_eq!(ctx.translate(0x2000), 0x1000);
    }

    #[test]
    fn test_rela_info() {
        let info = Elf64Rela::make_info(42, 8);
        let rela = Elf64Rela {
            r_offset: 0,
            r_info: info,
            r_addend: 0,
        };
        assert_eq!(rela.r_type(), 8);
        assert_eq!(rela.r_sym(), 42);
    }

    #[test]
    fn test_reloc_type_names() {
        assert_eq!(reloc_type_name(0), "R_X86_64_NONE");
        assert_eq!(reloc_type_name(8), "R_X86_64_RELATIVE");
        assert_eq!(reloc_type_name(1), "R_X86_64_64");
        assert_eq!(reloc_type_name(999), "R_X86_64_UNKNOWN");
    }

    #[test]
    fn test_stats_display() {
        let stats = RelocStats {
            total_entries: 100,
            total_applied: 80,
            skipped: 15,
            errors: 5,
            r_relative: 70,
            r_64: 10,
            ..Default::default()
        };
        let display = format!("{}", stats);
        assert!(display.contains("Total entries:  100"));
        assert!(display.contains("R_RELATIVE:   70"));
    }
}
