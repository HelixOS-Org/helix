/* =============================================================================
 * Helix OS - PIE (Position Independent) Linker Script for x86_64
 * =============================================================================
 * This script creates a RELOCATABLE kernel that can load at any address.
 *
 * KEY FEATURES:
 * - Position Independent Executable (PIE) format
 * - .rela.dyn section for runtime relocations
 * - Higher-half kernel virtual address layout
 * - KASLR-ready (kernel can be loaded at random addresses)
 * - Multiboot2 compatible (header in first 32KB)
 *
 * RELOCATION NOTES:
 * - The kernel is linked at KERNEL_VIRT_BASE (0xFFFFFFFF80000000)
 * - At runtime, the bootloader can load it anywhere and apply relocations
 * - All internal pointers use R_X86_64_RELATIVE relocations
 * - RIP-relative addressing makes most code position-independent
 * =============================================================================
 */

OUTPUT_FORMAT(elf64-x86-64)
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

/* =============================================================================
 * MEMORY CONFIGURATION
 * =============================================================================
 * KERNEL_VIRT_BASE: Higher-half virtual address where kernel expects to run
 * KERNEL_PHYS_BASE: Default physical load address (can be overridden by KASLR)
 * =============================================================================
 */
KERNEL_VIRT_BASE = 0xFFFFFFFF80000000;  /* -2GB in 64-bit address space */
KERNEL_PHYS_BASE = 0x100000;             /* 1MB physical (Multiboot standard) */

/* Offset for converting virtual to physical during early boot */
KERNEL_OFFSET = KERNEL_VIRT_BASE - KERNEL_PHYS_BASE;

/* =============================================================================
 * PROGRAM HEADERS
 * =============================================================================
 * Define segment types for the ELF loader:
 * - boot:    Multiboot header (must be loadable in first 32KB)
 * - text:    Executable code (R-X)
 * - rodata:  Read-only data (R--)
 * - data:    Read-write data (RW-)
 * - dynamic: Dynamic linking info (for relocations)
 * =============================================================================
 */
PHDRS
{
    boot    PT_LOAD FLAGS(5);       /* R-X: Multiboot header + early boot */
    text    PT_LOAD FLAGS(5);       /* R-X: Executable code */
    rodata  PT_LOAD FLAGS(4);       /* R--: Read-only data */
    data    PT_LOAD FLAGS(6);       /* RW-: Read-write data */
    dynamic PT_DYNAMIC FLAGS(6);    /* RW-: Dynamic info for relocation */
}

/* =============================================================================
 * SECTIONS
 * =============================================================================
 */
SECTIONS
{
    /* =========================================================================
     * PHYSICAL LAYOUT (for Multiboot2 compatibility)
     * The first 32KB must contain the Multiboot2 header
     * =========================================================================
     */
    . = KERNEL_PHYS_BASE;

    _kernel_phys_start = .;
    _kernel_start = .;

    /* -------------------------------------------------------------------------
     * MULTIBOOT2 HEADER (MUST be in first 32KB from load address)
     * -------------------------------------------------------------------------
     */
    .multiboot_header : ALIGN(8)
    {
        _multiboot_header_start = .;
        KEEP(*(.multiboot_header))
        KEEP(*(.multiboot2_header))
        _multiboot_header_end = .;
    } :boot

    /* -------------------------------------------------------------------------
     * EARLY BOOT CODE (runs before paging is enabled)
     * This code must be position-independent or use physical addresses
     * -------------------------------------------------------------------------
     */
    .boot : ALIGN(4K)
    {
        _boot_start = .;
        *(.boot)
        *(.boot.*)
        *(.text.boot)
        *(.text.early)
        _boot_end = .;
    } :boot

    /* =========================================================================
     * VIRTUAL LAYOUT (after paging is enabled)
     * Switch to higher-half virtual addresses
     * =========================================================================
     */
    . = KERNEL_VIRT_BASE + (_boot_end - _kernel_phys_start);

    /* -------------------------------------------------------------------------
     * EXECUTABLE CODE
     * RIP-relative addressing makes this naturally position-independent
     * -------------------------------------------------------------------------
     */
    .text : AT(_boot_end)  ALIGN(4K)
    {
        _text_start = .;
        *(.text)
        *(.text.*)
        _text_end = .;
    } :text

    /* -------------------------------------------------------------------------
     * READ-ONLY DATA
     * -------------------------------------------------------------------------
     */
    .rodata : ALIGN(4K)
    {
        _rodata_start = .;
        *(.rodata)
        *(.rodata.*)
        _rodata_end = .;
    } :rodata

    /* Exception handling frames (needed for panic/unwinding) */
    .eh_frame : ALIGN(8)
    {
        _eh_frame_start = .;
        KEEP(*(.eh_frame))
        KEEP(*(.eh_frame.*))
        _eh_frame_end = .;
    } :rodata

    .eh_frame_hdr : ALIGN(8)
    {
        KEEP(*(.eh_frame_hdr))
        KEEP(*(.eh_frame_hdr.*))
    } :rodata

    /* GCC exception tables */
    .gcc_except_table : ALIGN(8)
    {
        *(.gcc_except_table)
        *(.gcc_except_table.*)
    } :rodata

    /* -------------------------------------------------------------------------
     * READ-WRITE DATA
     * -------------------------------------------------------------------------
     */

    /* Data that needs relocation but becomes read-only after boot */
    .data.rel.ro : ALIGN(4K)
    {
        _data_rel_ro_start = .;
        *(.data.rel.ro)
        *(.data.rel.ro.*)
        _data_rel_ro_end = .;
    } :data

    /* Global Offset Table (for position-independent code) */
    .got : ALIGN(8)
    {
        _got_start = .;
        *(.got)
        _got_end = .;
    } :data

    .got.plt : ALIGN(8)
    {
        _got_plt_start = .;
        *(.got.plt)
        *(.igot.plt)
        _got_plt_end = .;
    } :data

    /* -------------------------------------------------------------------------
     * DYNAMIC SECTION (Required for relocation)
     * Contains pointers to .rela.dyn, .dynsym, etc.
     * -------------------------------------------------------------------------
     */
    .dynamic : ALIGN(8)
    {
        _dynamic_start = .;
        *(.dynamic)
        _dynamic_end = .;
    } :data :dynamic

    /* -------------------------------------------------------------------------
     * INITIALIZED DATA
     * -------------------------------------------------------------------------
     */
    .data : ALIGN(4K)
    {
        _data_start = .;
        *(.data)
        *(.data.*)
        _data_end = .;
    } :data

    /* -------------------------------------------------------------------------
     * UNINITIALIZED DATA (BSS)
     * -------------------------------------------------------------------------
     */
    .bss (NOLOAD) : ALIGN(4K)
    {
        _bss_start = .;
        *(.bss)
        *(.bss.*)
        *(COMMON)
        . = ALIGN(8);
        _bss_end = .;
    } :data

    _kernel_end = .;

    /* =========================================================================
     * RELOCATION SECTIONS (Not loaded into memory, used by bootloader)
     * =========================================================================
     * These sections contain relocation entries that the bootloader uses
     * to fix up addresses when loading at a non-default address.
     * =========================================================================
     */

    /* Dynamic relocations (most important for PIE) */
    .rela.dyn :
    {
        _rela_dyn_start = .;
        *(.rela.dyn)
        *(.rela.dyn.*)
        _rela_dyn_end = .;
    }

    /* PLT relocations (for external function calls) */
    .rela.plt :
    {
        _rela_plt_start = .;
        *(.rela.plt)
        *(.rela.plt.*)
        _rela_plt_end = .;
    }

    /* Other relocation sections */
    .rela.text : { *(.rela.text) *(.rela.text.*) }
    .rela.rodata : { *(.rela.rodata) *(.rela.rodata.*) }
    .rela.data : { *(.rela.data) *(.rela.data.*) }
    .rela.got : { *(.rela.got) }
    .rela.bss : { *(.rela.bss) *(.rela.bss.*) }

    /* Dynamic symbol table */
    .dynsym :
    {
        _dynsym_start = .;
        *(.dynsym)
        _dynsym_end = .;
    }

    /* Dynamic string table */
    .dynstr :
    {
        _dynstr_start = .;
        *(.dynstr)
        _dynstr_end = .;
    }

    /* Hash tables for symbol lookup */
    .hash : { *(.hash) }
    .gnu.hash : { *(.gnu.hash) }

    /* =========================================================================
     * DISCARDED SECTIONS
     * =========================================================================
     */
    /DISCARD/ :
    {
        *(.comment)
        *(.note)
        *(.note.*)
        *(.gnu.build-id)
    }
}

/* =============================================================================
 * EXPORTED SYMBOLS
 * =============================================================================
 * These symbols are used by the kernel and bootloader for relocation
 * =============================================================================
 */

/* Kernel boundaries */
PROVIDE(_kernel_size = _kernel_end - _kernel_start);
PROVIDE(_kernel_virt_size = _kernel_end - KERNEL_VIRT_BASE);

/* Physical/Virtual base addresses */
PROVIDE(_kernel_phys_base = KERNEL_PHYS_BASE);
PROVIDE(_kernel_virt_base = KERNEL_VIRT_BASE);
PROVIDE(_kernel_offset = KERNEL_OFFSET);

/* Section sizes for validation */
PROVIDE(_text_size = _text_end - _text_start);
PROVIDE(_rodata_size = _rodata_end - _rodata_start);
PROVIDE(_data_size = _data_end - _data_start);
PROVIDE(_bss_size = _bss_end - _bss_start);

/* Relocation section info */
PROVIDE(_rela_dyn_size = _rela_dyn_end - _rela_dyn_start);
PROVIDE(_rela_plt_size = _rela_plt_end - _rela_plt_start);

/* Dynamic section pointer (used by relocation engine) */
PROVIDE(_DYNAMIC = _dynamic_start);

/* GOT info for debugging */
PROVIDE(_got_size = _got_end - _got_start);
PROVIDE(_got_plt_size = _got_plt_end - _got_plt_start);
