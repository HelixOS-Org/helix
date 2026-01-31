//! # Helix Boot Module - Multiboot2 Header
//!
//! This module provides the Multiboot2 header and low-level boot code
//! required to boot the kernel with GRUB or other Multiboot2 compliant bootloaders.
//!
//! ## Architecture
//!
//! The boot process follows these steps:
//! 1. GRUB loads the kernel and finds the Multiboot2 header
//! 2. GRUB jumps to `_start` in 32-bit protected mode
//! 3. `_start` sets up long mode (64-bit) and calls `kernel_main`
//!
//! ## Safety
//!
//! This module contains unsafe code that directly manipulates CPU state.
//! It must be the first code executed and assumes no runtime environment.

use core::arch::global_asm;

// =============================================================================
// Multiboot2 Header
// =============================================================================

/// Multiboot2 magic number
const MULTIBOOT2_MAGIC: u32 = 0xe85250d6;
/// Multiboot2 architecture: i386 (also used for x86_64 boot)
const MULTIBOOT2_ARCH_I386: u32 = 0;
/// Multiboot2 header checksum calculation
const fn multiboot2_checksum(magic: u32, arch: u32, length: u32) -> u32 {
    (0x100000000u64 - (magic as u64 + arch as u64 + length as u64)) as u32
}

// Header length: 16 bytes (header) + 24 bytes (framebuffer tag) + 8 bytes (end tag) = 48 bytes
const HEADER_LENGTH: u32 = 48;

/// Multiboot2 header wrapper struct for alignment
/// Now includes framebuffer request tag
#[repr(C, align(8))]
struct Multiboot2Header {
    // Header: 4 x u32 = 16 bytes
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // Framebuffer tag: 6 x u32 = 24 bytes
    fb_type: u16,         // Tag type 5 = framebuffer
    fb_flags: u16,        // Flags (0 = required, 1 = optional)
    fb_size: u32,         // Tag size = 20 bytes
    fb_width: u32,        // Preferred width
    fb_height: u32,       // Preferred height
    fb_depth: u32,        // Preferred bpp
    // End tag: 2 x u32 = 8 bytes
    end_type: u32,        // Type 0 = end
    end_size: u32,        // Size 8
}

/// Multiboot2 header structure - placed in a special section
/// This MUST be in the first 32KB of the kernel image
/// Now requests a framebuffer from the bootloader
#[used]
#[link_section = ".multiboot_header"]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MULTIBOOT2_MAGIC,
    architecture: MULTIBOOT2_ARCH_I386,
    header_length: HEADER_LENGTH,
    checksum: multiboot2_checksum(MULTIBOOT2_MAGIC, MULTIBOOT2_ARCH_I386, HEADER_LENGTH),
    // Framebuffer request tag
    fb_type: 5,           // MULTIBOOT_HEADER_TAG_FRAMEBUFFER
    fb_flags: 1,          // Optional (don't fail if unavailable)
    fb_size: 20,          // Tag size: type(2) + flags(2) + size(4) + width(4) + height(4) + depth(4)
    fb_width: 1024,       // Preferred width
    fb_height: 768,       // Preferred height
    fb_depth: 32,         // Preferred depth (32-bit color)
    // End tag
    end_type: 0,
    end_size: 8,
};

// =============================================================================
// Boot Assembly (x86_64)
// =============================================================================

#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
.section .boot, "ax"
.code32
.global _start

// Define fixed addresses for page tables (within first 1MB mappable area)
// These must be page-aligned and in identity-mapped memory
// Need 4 page directories (16KB) for 4GB mapping
.set STACK_TOP, 0x7C00         // Stack at conventional boot area
.set PML4_ADDR, 0x1000         // PML4 (4KB)
.set PDPT_ADDR, 0x2000         // PDPT (4KB)
.set PD_ADDR,   0x3000         // 4 Page Directories (16KB: 0x3000-0x6FFF)
.set GDT_ADDR,  0x7000         // GDT location (moved to avoid conflict)

_start:
    // Disable interrupts
    cli

    // Save multiboot info pointer (ebx) and magic (eax) in callee-saved registers
    // EBX = multiboot2 info pointer, EAX = magic (0x36d76289)
    // Save in EBP (callee-saved, will survive all operations)
    mov ebp, ebx        // Save multiboot2 info pointer in EBP
    mov esi, eax        // Save magic in ESI

    // Check for multiboot2 magic
    cmp eax, 0x36d76289
    jne .no_multiboot

    // Set up stack at fixed address
    mov esp, STACK_TOP

    // Check for CPUID support
    pushfd
    pop eax
    mov ecx, eax
    xor eax, 0x200000
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
    xor eax, ecx
    jz .no_cpuid

    // Check for extended CPUID
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_long_mode

    // Check for long mode
    mov eax, 0x80000001
    cpuid
    test edx, (1 << 29)
    jz .no_long_mode

    // Set up paging for long mode
    // Need to map at least 4GB for framebuffer at ~0xFD000000
    // Using 1GB huge pages via PDPT entries (if supported) or 2MB pages

    // Point CR3 to PML4
    mov eax, PML4_ADDR
    mov cr3, eax

    // Clear PML4
    mov edi, PML4_ADDR
    xor eax, eax
    mov ecx, 4096
    rep stosb

    // Clear PDPT
    mov edi, PDPT_ADDR
    xor eax, eax
    mov ecx, 4096
    rep stosb

    // Clear PD (we need 4 page directories for 4GB)
    mov edi, PD_ADDR
    xor eax, eax
    mov ecx, 4096 * 4    // 4 page directories
    rep stosb

    // Set up identity mapping for first 4GB+ (for framebuffer at ~0xFD000000)
    // PML4[0] -> PDPT
    mov eax, PDPT_ADDR
    or eax, 0x03        // Present + Writable
    mov edi, PML4_ADDR
    mov [edi], eax

    // PDPT[0] -> PD (for 0-1GB)
    mov eax, PD_ADDR
    or eax, 0x03        // Present + Writable
    mov edi, PDPT_ADDR
    mov [edi], eax

    // PDPT[1] -> PD+4096 (for 1-2GB)
    mov eax, PD_ADDR + 0x1000
    or eax, 0x03
    mov [edi + 8], eax

    // PDPT[2] -> PD+8192 (for 2-3GB)
    mov eax, PD_ADDR + 0x2000
    or eax, 0x03
    mov [edi + 16], eax

    // PDPT[3] -> PD+12288 (for 3-4GB, contains framebuffer at ~0xFD000000)
    mov eax, PD_ADDR + 0x3000
    or eax, 0x03
    mov [edi + 24], eax

    // PD entries: map all 4GB with 2MB pages (512 entries * 4 PDs = 2048 entries)
    mov edi, PD_ADDR
    mov eax, 0x83       // Present + Writable + Page Size (2MB)
    mov ecx, 2048       // 2048 entries = 4GB
.set_pd_entry:
    mov [edi], eax
    add eax, 0x200000   // Next 2MB
    add edi, 8
    loop .set_pd_entry

    // Enable PAE
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    // Set long mode enable in EFER MSR
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)    // LME bit
    wrmsr

    // Enable paging
    mov eax, cr0
    or eax, (1 << 31)   // PG bit
    mov cr0, eax

    // Set up GDT at fixed address
    // Copy GDT data to GDT_ADDR
    mov edi, GDT_ADDR
    // Null descriptor
    mov dword ptr [edi], 0
    mov dword ptr [edi+4], 0
    // 64-bit code segment: 0x00AF9A000000FFFF
    mov dword ptr [edi+8], 0x0000FFFF
    mov dword ptr [edi+12], 0x00AF9A00
    // Data segment: 0x00CF92000000FFFF
    mov dword ptr [edi+16], 0x0000FFFF
    mov dword ptr [edi+20], 0x00CF9200

    // Set up GDT pointer at GDT_ADDR + 0x100
    mov edi, GDT_ADDR + 0x100
    mov word ptr [edi], 23        // Limit (3 entries * 8 - 1)
    mov dword ptr [edi+2], GDT_ADDR   // Base address

    // Load GDT
    lgdt [edi]

    // Far jump to 64-bit code
    // Use a direct far jump with hardcoded segment:offset
    // We'll construct the far pointer on the stack
    push 0x08           // Code segment selector
    .byte 0x68          // push imm32 opcode
    .long _start64      // Will be relocated to absolute address
    retf

.no_multiboot:
    // Print 'M' error to VGA
    mov eax, 0x4F4D     // 'M' with white on red
    mov [0xB8000], eax
    jmp .halt32

.no_cpuid:
    // Print 'C' error to VGA
    mov eax, 0x4F43     // 'C' with white on red
    mov [0xB8000], eax
    jmp .halt32

.no_long_mode:
    // Print 'L' error to VGA
    mov eax, 0x4F4C     // 'L' with white on red
    mov [0xB8000], eax
    jmp .halt32

.halt32:
    cli
    hlt
    jmp .halt32

.code64
.global _start64
_start64:
    // Clear segment registers
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    // Set up 64-bit stack using known location in BSS
    // The kernel is loaded at 1MB, stack space follows in BSS
    mov rsp, 0x200000   // Use 2MB as temporary stack (in identity-mapped area)

    // Restore multiboot info pointer from EBP (saved in 32-bit mode)
    // EBP was preserved through the mode switch
    // Zero-extend 32-bit EBP to 64-bit RDI for first argument
    mov edi, ebp        // RDI = multiboot2 info pointer (zero-extended)
    mov esi, esi        // RSI = magic (zero-extended from 32-bit)

    // Call Rust kernel_main(multiboot2_info_ptr)
    call kernel_main

    // If kernel_main returns, halt
.halt64:
    cli
    hlt
    jmp .halt64

// =============================================================================
// Data Sections
// =============================================================================

.section .rodata
.align 16
_gdt64:
    .quad 0x0000000000000000    // Null descriptor
    .quad 0x00AF9A000000FFFF    // 64-bit code segment
    .quad 0x00CF92000000FFFF    // Data segment
_gdt64_end:

.align 8
_gdt64_ptr:
    .word _gdt64_end - _gdt64 - 1
    .long _gdt64
    .long 0

.section .bss
.align 4096

// Page tables (must be page-aligned)
.global _pml4
_pml4:
    .space 4096
.global _pdpt
_pdpt:
    .space 4096
.global _pd
_pd:
    .space 4096 * 4     // 4 page directories for 4GB mapping

// Stack (16KB)
.global _stack_bottom
_stack_bottom:
    .space 16384
.global _stack_top
_stack_top:
"#
);

// =============================================================================
// ARM64 Boot (placeholder)
// =============================================================================

#[cfg(target_arch = "aarch64")]
global_asm!(
    r#"
.section .text._start
.global _start

_start:
    // Set up stack
    ldr x0, =_stack_top
    mov sp, x0

    // Clear BSS
    ldr x0, =__bss_start
    ldr x1, =__bss_end
1:  cmp x0, x1
    b.ge 2f
    str xzr, [x0], #8
    b 1b
2:
    // Call Rust entry
    bl kernel_main

    // Halt
3:  wfi
    b 3b

.section .bss
.align 16
_stack_bottom:
    .space 16384
_stack_top:
"#
);

// =============================================================================
// RISC-V Boot (placeholder)
// =============================================================================

#[cfg(target_arch = "riscv64")]
global_asm!(
    r#"
.section .text._start
.global _start

_start:
    // Set up stack
    la sp, _stack_top

    // Call Rust entry
    call kernel_main

    // Halt
1:  wfi
    j 1b

.section .bss
.align 16
_stack_bottom:
    .space 16384
_stack_top:
"#
);

/// Verify that multiboot info is valid
/// Returns the physical address of multiboot info structure
#[inline]
pub fn validate_multiboot(magic: u32, info_addr: usize) -> Option<usize> {
    // Multiboot2 bootloader magic (passed in EAX)
    const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36d76289;

    if magic == MULTIBOOT2_BOOTLOADER_MAGIC {
        Some(info_addr)
    } else {
        None
    }
}
