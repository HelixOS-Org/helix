//! # Kernel Relocation Integration for UEFI Bootloader
//!
//! This module provides the integration layer between the UEFI bootloader and
//! the kernel relocation/KASLR system. It handles:
//!
//! - Determining if relocation is needed
//! - Generating KASLR addresses
//! - Applying relocations before kernel entry
//! - Updating BootInfo with relocation information
//!
//! ## Boot Flow Integration
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     UEFI BOOT WITH RELOCATION                       │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  1. Load kernel ELF from disk                                       │
//! │  2. Parse ELF headers, find .rela.dyn                               │
//! │  3. Calculate memory requirements                                    │
//! │  4. Generate KASLR address (if enabled)                             │
//! │  5. Allocate memory at target address                               │
//! │  6. Copy segments to target                                         │
//! │  7. Apply relocations if slide != 0                                 │
//! │  8. Update BootInfo with relocation info                            │
//! │  9. ExitBootServices()                                              │
//! │  10. Jump to relocated kernel entry point                           │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

use crate::error::{Error, Result};
use crate::handoff::BootInfo;

// ============================================================================
// RELOCATION CONFIGURATION
// ============================================================================

/// Configuration for kernel relocation
#[derive(Debug, Clone)]
pub struct RelocationConfig {
    /// Enable KASLR randomization
    pub kaslr_enabled: bool,
    /// Minimum virtual address for kernel
    pub min_address: u64,
    /// Maximum virtual address for kernel
    pub max_address: u64,
    /// Required alignment (2MB for huge pages)
    pub alignment: u64,
    /// Kernel link address (from linker script)
    pub link_address: u64,
    /// Whether to fail on relocation errors
    pub strict_mode: bool,
}

impl Default for RelocationConfig {
    fn default() -> Self {
        Self {
            kaslr_enabled: true,
            min_address: 0xFFFF_FFFF_8000_0000, // Higher-half start
            max_address: 0xFFFF_FFFF_C000_0000, // 1GB range
            alignment: 0x20_0000,               // 2MB alignment
            link_address: 0xFFFF_FFFF_8000_0000, // Default link address
            strict_mode: false,
        }
    }
}

impl RelocationConfig {
    /// Create config with KASLR disabled
    pub fn no_kaslr() -> Self {
        Self {
            kaslr_enabled: false,
            ..Default::default()
        }
    }

    /// Create config from kernel command line
    pub fn from_cmdline(cmdline: &str) -> Self {
        let mut config = Self::default();

        if cmdline.contains("nokaslr") {
            config.kaslr_enabled = false;
        }

        if cmdline.contains("kaslr_strict") {
            config.strict_mode = true;
        }

        config
    }
}

// ============================================================================
// RELOCATION STATISTICS
// ============================================================================

/// Statistics about the relocation process
#[derive(Debug, Clone, Default)]
pub struct RelocationStats {
    /// Total relocation entries processed
    pub total_entries: usize,
    /// Successfully applied relocations
    pub applied: usize,
    /// Skipped relocations (R_NONE, already correct, etc.)
    pub skipped: usize,
    /// Errors encountered
    pub errors: usize,
    /// Time spent on relocation (microseconds)
    pub time_us: u64,
    /// Entropy quality (0-4)
    pub entropy_quality: u8,
}

// ============================================================================
// KASLR ADDRESS GENERATION
// ============================================================================

/// Generate a KASLR-randomized load address
///
/// Uses hardware RNG (RDRAND/RDSEED) if available, falls back to TSC.
pub fn generate_kaslr_address(config: &RelocationConfig, kernel_size: u64) -> Result<(u64, u8)> {
    if !config.kaslr_enabled {
        return Ok((config.link_address, 0));
    }

    // Align kernel size up
    let aligned_size = (kernel_size + config.alignment - 1) & !(config.alignment - 1);

    // Calculate usable range
    let range = config.max_address.saturating_sub(config.min_address);
    if aligned_size >= range {
        return Err(Error::BufferTooSmall);
    }

    // Calculate number of slots
    let num_slots = (range - aligned_size) / config.alignment;
    if num_slots == 0 {
        return Err(Error::BufferTooSmall);
    }

    // Get entropy
    let (random, quality) = collect_entropy();

    // Select slot
    let slot = random % num_slots;
    let load_address = config.min_address + slot * config.alignment;

    Ok((load_address, quality))
}

/// Collect entropy from available sources
fn collect_entropy() -> (u64, u8) {
    // Try RDSEED first (best quality)
    if let Some(val) = rdseed64() {
        return (val, 4); // Cryptographic quality
    }

    // Try RDRAND
    if let Some(val) = rdrand64() {
        return (val, 3); // Strong quality
    }

    // Fall back to TSC
    (rdtsc(), 1) // Weak quality
}

// Hardware RNG wrappers
#[cfg(target_arch = "x86_64")]
fn rdseed64() -> Option<u64> {
    // Check CPUID for RDSEED support
    let cpuid = unsafe { core::arch::x86_64::__cpuid_count(7, 0) };
    if (cpuid.ebx & (1 << 18)) == 0 {
        return None;
    }

    let mut value: u64;
    let mut success: u8;

    for _ in 0..10 {
        unsafe {
            core::arch::asm!(
                "rdseed {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) success,
                options(nomem, nostack)
            );
        }
        if success != 0 {
            return Some(value);
        }
        core::hint::spin_loop();
    }

    None
}

#[cfg(target_arch = "x86_64")]
fn rdrand64() -> Option<u64> {
    // Check CPUID for RDRAND support
    let cpuid = unsafe { core::arch::x86_64::__cpuid(1) };
    if (cpuid.ecx & (1 << 30)) == 0 {
        return None;
    }

    let mut value: u64;
    let mut success: u8;

    for _ in 0..10 {
        unsafe {
            core::arch::asm!(
                "rdrand {0}",
                "setc {1}",
                out(reg) value,
                out(reg_byte) success,
                options(nomem, nostack)
            );
        }
        if success != 0 {
            return Some(value);
        }
        core::hint::spin_loop();
    }

    None
}

#[cfg(target_arch = "x86_64")]
fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack)
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

#[cfg(not(target_arch = "x86_64"))]
fn rdseed64() -> Option<u64> {
    None
}
#[cfg(not(target_arch = "x86_64"))]
fn rdrand64() -> Option<u64> {
    None
}
#[cfg(not(target_arch = "x86_64"))]
fn rdtsc() -> u64 {
    0
}

// ============================================================================
// RELOCATION APPLICATION
// ============================================================================

/// x86_64 relocation types
mod reloc_types {
    pub const R_X86_64_NONE: u32 = 0;
    pub const R_X86_64_64: u32 = 1;
    pub const R_X86_64_PC32: u32 = 2;
    pub const R_X86_64_RELATIVE: u32 = 8;
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
}

/// ELF64 relocation entry with addend
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Rela {
    pub r_offset: u64,
    pub r_info: u64,
    pub r_addend: i64,
}

impl Elf64Rela {
    pub fn r_type(&self) -> u32 {
        (self.r_info & 0xFFFFFFFF) as u32
    }
}

/// Apply relocations to a loaded kernel
///
/// # Safety
///
/// - `kernel_base` must point to valid, writable kernel memory
/// - `rela_entries` must contain valid relocation entries
pub unsafe fn apply_relocations(
    kernel_base: *mut u8,
    kernel_size: usize,
    link_base: u64,
    load_base: u64,
    rela_entries: &[Elf64Rela],
) -> Result<RelocationStats> {
    use core::ptr;

    use reloc_types::*;

    let mut stats = RelocationStats::default();
    stats.total_entries = rela_entries.len();

    let slide = (load_base as i128 - link_base as i128) as i64;

    // Fast path: no relocation needed
    if slide == 0 {
        stats.skipped = stats.total_entries;
        return Ok(stats);
    }

    for rela in rela_entries {
        let rtype = rela.r_type();
        let offset = rela.r_offset;

        // Calculate target offset from load base
        let target_offset = if offset >= link_base {
            offset - link_base
        } else {
            offset
        };

        // Bounds check
        if target_offset >= kernel_size as u64 - 8 {
            stats.errors += 1;
            continue;
        }

        let target_ptr = kernel_base.add(target_offset as usize);

        match rtype {
            R_X86_64_NONE => {
                stats.skipped += 1;
            },

            R_X86_64_RELATIVE => {
                // B + A: Most common for PIE
                let value = (load_base as i64 + rela.r_addend) as u64;
                ptr::write_unaligned(target_ptr as *mut u64, value);
                stats.applied += 1;
            },

            R_X86_64_64 => {
                // S + A: Add slide to current value
                let current = ptr::read_unaligned(target_ptr as *const u64);
                let new_value = (current as i64 + slide) as u64;
                ptr::write_unaligned(target_ptr as *mut u64, new_value);
                stats.applied += 1;
            },

            R_X86_64_PC32 => {
                // PC-relative, usually no adjustment needed
                stats.skipped += 1;
            },

            R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                // GOT/PLT entries
                let current = ptr::read_unaligned(target_ptr as *const u64);
                let new_value = (current as i64 + slide) as u64;
                ptr::write_unaligned(target_ptr as *mut u64, new_value);
                stats.applied += 1;
            },

            _ => {
                stats.errors += 1;
            },
        }
    }

    Ok(stats)
}

// ============================================================================
// ELF PARSING HELPERS
// ============================================================================

/// ELF64 header
#[repr(C, packed)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF64 section header
#[repr(C, packed)]
pub struct Elf64SectionHeader {
    pub sh_name: u32,
    pub sh_type: u32,
    pub sh_flags: u64,
    pub sh_addr: u64,
    pub sh_offset: u64,
    pub sh_size: u64,
    pub sh_link: u32,
    pub sh_info: u32,
    pub sh_addralign: u64,
    pub sh_entsize: u64,
}

const SHT_RELA: u32 = 4;

/// Find .rela.dyn section in ELF
pub unsafe fn find_rela_section(
    elf_data: *const u8,
    elf_size: usize,
) -> Option<(*const Elf64Rela, usize)> {
    if elf_size < core::mem::size_of::<Elf64Header>() {
        return None;
    }

    let header = &*(elf_data as *const Elf64Header);

    // Check magic
    if header.e_ident[0..4] != [0x7F, b'E', b'L', b'F'] {
        return None;
    }

    let shoff = header.e_shoff as usize;
    let shnum = header.e_shnum as usize;
    let shentsize = header.e_shentsize as usize;
    let shstrndx = header.e_shstrndx as usize;

    if shoff == 0 || shnum == 0 || shoff + shnum * shentsize > elf_size {
        return None;
    }

    // Get string table
    let shstrtab_hdr = &*(elf_data.add(shoff + shstrndx * shentsize) as *const Elf64SectionHeader);
    let shstrtab = core::slice::from_raw_parts(
        elf_data.add(shstrtab_hdr.sh_offset as usize),
        shstrtab_hdr.sh_size as usize,
    );

    // Find .rela.dyn
    for i in 0..shnum {
        let sh = &*(elf_data.add(shoff + i * shentsize) as *const Elf64SectionHeader);

        if sh.sh_type != SHT_RELA {
            continue;
        }

        let name_offset = sh.sh_name as usize;
        if name_offset >= shstrtab.len() {
            continue;
        }

        let name_bytes = &shstrtab[name_offset..];
        let name_end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let name = core::str::from_utf8(&name_bytes[..name_end]).ok()?;

        if name == ".rela.dyn" {
            let rela_ptr = elf_data.add(sh.sh_offset as usize) as *const Elf64Rela;
            let rela_count = sh.sh_size as usize / core::mem::size_of::<Elf64Rela>();
            return Some((rela_ptr, rela_count));
        }
    }

    None
}

// ============================================================================
// HIGH-LEVEL API
// ============================================================================

/// Relocate a kernel and update boot info
///
/// This is the main entry point for kernel relocation from the bootloader.
pub unsafe fn relocate_kernel_and_update_bootinfo(
    kernel_data: *mut u8,
    kernel_size: usize,
    config: &RelocationConfig,
    boot_info: &mut BootInfo,
) -> Result<()> {
    // Generate KASLR address
    let (load_address, entropy_quality) = generate_kaslr_address(config, kernel_size as u64)?;

    let slide = (load_address as i128 - config.link_address as i128) as i64;

    // If no relocation needed, just update boot info
    if slide == 0 {
        boot_info.set_kaslr_info(0, config.link_address, 0, 0);
        return Ok(());
    }

    // Find relocation section
    let (rela_ptr, rela_count) =
        find_rela_section(kernel_data, kernel_size).ok_or(Error::NotFound)?;

    let rela_entries = core::slice::from_raw_parts(rela_ptr, rela_count);

    // Apply relocations
    let stats = apply_relocations(
        kernel_data,
        kernel_size,
        config.link_address,
        load_address,
        rela_entries,
    )?;

    // Update boot info
    boot_info.set_kaslr_info(
        slide,
        config.link_address,
        entropy_quality,
        stats.applied as u64,
    );

    boot_info.kernel_virtual_address = Some(crate::raw::types::VirtualAddress(load_address));

    if stats.errors > 0 && config.strict_mode {
        return Err(Error::InvalidData);
    }

    Ok(())
}

// ============================================================================
// BOOTINFO EXTENSION
// ============================================================================

/// Extension trait for BootInfo to add relocation support
pub trait BootInfoRelocationExt {
    /// Print relocation summary to console
    fn print_relocation_summary(&self);
}

impl BootInfoRelocationExt for BootInfo {
    fn print_relocation_summary(&self) {
        // This would use the console/framebuffer to print
        // For now, it's a placeholder for integration
        // TODO: Implement debug logging when console is available
        let _ = self.kaslr_enabled; // Silence unused warning
    }
}
