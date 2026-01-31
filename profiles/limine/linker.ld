/* ============================================================================
 * HELIX OS - LIMINE PIE LINKER SCRIPT
 * ============================================================================
 *
 * This linker script is designed for PIE (Position Independent Executable)
 * kernels loaded by the Limine bootloader.
 *
 * Features:
 *   - Full PIE support with .rela.dyn relocations
 *   - KASLR-compatible memory layout
 *   - Limine-specific section requirements
 *   - Higher-half kernel (HHDM compatible)
 *
 * Usage:
 *   Use this with: rustflags = ["-C", "link-arg=-T", "-C",
 *                   "link-arg=profiles/limine/linker.ld"]
 *
 * ============================================================================
 */

/* Output format */
OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

/* Limine loads kernel at higher-half virtual address */
/* Physical address is provided at runtime via kernel_address response */
KERNEL_VMA = 0xFFFFFFFF80000000;

/* Page sizes */
PAGE_SIZE = 0x1000;          /* 4 KiB */
HUGE_PAGE_SIZE = 0x200000;   /* 2 MiB */

/* ============================================================================
 * PROGRAM HEADERS
 * ============================================================================
 */

PHDRS
{
    /* Limine-specific sections */
    limine   PT_LOAD FLAGS(4);   /* R--: Limine requests */

    /* Loadable segments */
    text     PT_LOAD FLAGS(5);   /* R-X: Executable code */
    rodata   PT_LOAD FLAGS(4);   /* R--: Read-only data */
    data     PT_LOAD FLAGS(6);   /* RW-: Initialized data */
    bss      PT_LOAD FLAGS(6);   /* RW-: Zero-initialized data */

    /* Dynamic linking for PIE */
    dynamic  PT_DYNAMIC FLAGS(6); /* RW-: Dynamic section */
    relro    PT_GNU_RELRO FLAGS(4); /* R--: RELRO after relocation */

    /* Non-executable stack */
    gnu_stack PT_GNU_STACK FLAGS(6);
}

/* ============================================================================
 * SECTIONS
 * ============================================================================
 */

SECTIONS
{
    /* Start at higher-half address */
    . = KERNEL_VMA;
    __kernel_start = .;

    /* ========================================================================
     * LIMINE REQUESTS SECTION
     * ========================================================================
     * Limine looks for requests in this section. Must be early in the binary.
     */
    .limine_requests ALIGN(PAGE_SIZE) :
    {
        KEEP(*(.limine_requests_start))
        KEEP(*(.limine_requests))
        KEEP(*(.limine_requests_end))
    } :limine

    /* ========================================================================
     * TEXT SEGMENT - Executable Code
     * ========================================================================
     */
    . = ALIGN(HUGE_PAGE_SIZE);
    __text_start = .;

    .text ALIGN(PAGE_SIZE) :
    {
        *(.text.boot)         /* Boot code first */
        *(.text.hot)          /* Hot path code */
        *(.text .text.*)
        *(.gnu.linkonce.t.*)
    } :text

    /* PLT for dynamic linking */
    .plt ALIGN(16) :
    {
        *(.plt)
        *(.plt.*)
        *(.iplt)
    } :text

    __text_end = .;

    /* ========================================================================
     * RODATA SEGMENT - Read-Only Data
     * ========================================================================
     */
    . = ALIGN(HUGE_PAGE_SIZE);
    __rodata_start = .;

    .rodata ALIGN(PAGE_SIZE) :
    {
        *(.rodata .rodata.*)
        *(.gnu.linkonce.r.*)
    } :rodata

    /* Exception handling */
    .eh_frame_hdr ALIGN(8) :
    {
        *(.eh_frame_hdr)
        *(.eh_frame_entry .eh_frame_entry.*)
    } :rodata

    .eh_frame ALIGN(8) :
    {
        KEEP(*(.eh_frame))
        *(.eh_frame.*)
    } :rodata

    .gcc_except_table ALIGN(8) :
    {
        *(.gcc_except_table .gcc_except_table.*)
    } :rodata

    __rodata_end = .;

    /* ========================================================================
     * RELRO SEGMENT - Relocated Read-Only
     * ========================================================================
     * These sections are writable during relocation, then made read-only.
     */
    . = ALIGN(HUGE_PAGE_SIZE);
    __relro_start = .;

    /* Dynamic section for PIE */
    .dynamic ALIGN(8) :
    {
        __dynamic_start = .;
        *(.dynamic)
        __dynamic_end = .;
    } :data :dynamic :relro

    /* Global Offset Table */
    .got ALIGN(8) :
    {
        __got_start = .;
        *(.got)
        __got_end = .;
    } :data :relro

    .got.plt ALIGN(8) :
    {
        __got_plt_start = .;
        *(.got.plt)
        __got_plt_end = .;
    } :data :relro

    __relro_end = .;

    /* ========================================================================
     * DATA SEGMENT - Initialized Data
     * ========================================================================
     */
    . = ALIGN(HUGE_PAGE_SIZE);
    __data_start = .;

    /* Relocation tables */
    .rela.dyn ALIGN(8) :
    {
        __rela_start = .;
        *(.rela.init)
        *(.rela.text .rela.text.*)
        *(.rela.rodata .rela.rodata.*)
        *(.rela.data .rela.data.*)
        *(.rela.got)
        *(.rela.bss .rela.bss.*)
        *(.rela.plt)
        *(.rela.iplt)
        *(.rela*)
        __rela_end = .;
    } :data

    .rela.plt ALIGN(8) :
    {
        __rela_plt_start = .;
        *(.rela.plt)
        __rela_plt_end = .;
    } :data

    /* Symbol tables for modules/debugging */
    .dynsym ALIGN(8) :
    {
        __dynsym_start = .;
        *(.dynsym)
        __dynsym_end = .;
    } :data

    .dynstr ALIGN(1) :
    {
        __dynstr_start = .;
        *(.dynstr)
        __dynstr_end = .;
    } :data

    .gnu.hash ALIGN(8) :
    {
        *(.gnu.hash)
    } :data

    .hash ALIGN(8) :
    {
        *(.hash)
    } :data

    /* Initialized data */
    .data ALIGN(PAGE_SIZE) :
    {
        *(.data .data.*)
        *(.gnu.linkonce.d.*)
    } :data

    /* Thread-local storage template */
    .tdata ALIGN(8) :
    {
        __tdata_start = .;
        *(.tdata .tdata.*)
        *(.gnu.linkonce.td.*)
        __tdata_end = .;
    } :data

    __data_end = .;

    /* ========================================================================
     * BSS SEGMENT - Zero-Initialized Data
     * ========================================================================
     */
    . = ALIGN(PAGE_SIZE);
    __bss_start = .;

    .tbss ALIGN(8) :
    {
        __tbss_start = .;
        *(.tbss .tbss.*)
        *(.gnu.linkonce.tb.*)
        __tbss_end = .;
    } :bss

    .bss ALIGN(PAGE_SIZE) :
    {
        *(.bss .bss.*)
        *(.gnu.linkonce.b.*)
        *(COMMON)
    } :bss

    __bss_end = .;

    /* ========================================================================
     * KERNEL END
     * ========================================================================
     */
    . = ALIGN(PAGE_SIZE);
    __kernel_end = .;

    /* Size calculations */
    __kernel_size = __kernel_end - __kernel_start;
    __text_size = __text_end - __text_start;
    __rodata_size = __rodata_end - __rodata_start;
    __data_size = __data_end - __data_start;
    __bss_size = __bss_end - __bss_start;
    __rela_size = __rela_end - __rela_start;

    /* ========================================================================
     * DISCARDED SECTIONS
     * ========================================================================
     */
    /DISCARD/ :
    {
        *(.comment)
        *(.note.GNU-stack)
        *(.note.gnu.build-id)
        *(.gnu_debuglink)
        *(.gnu.lto_*)
    }

    /* ========================================================================
     * DEBUG SECTIONS (stripped in release builds)
     * ========================================================================
     */
    .debug_info     0 : { *(.debug_info .gnu.linkonce.wi.*) }
    .debug_abbrev   0 : { *(.debug_abbrev) }
    .debug_line     0 : { *(.debug_line .debug_line.* .debug_line_end) }
    .debug_frame    0 : { *(.debug_frame) }
    .debug_str      0 : { *(.debug_str) }
    .debug_loc      0 : { *(.debug_loc) }
    .debug_ranges   0 : { *(.debug_ranges) }
    .debug_aranges  0 : { *(.debug_aranges) }
    .debug_pubnames 0 : { *(.debug_pubnames) }
    .debug_pubtypes 0 : { *(.debug_pubtypes) }
}

/* ============================================================================
 * ASSERTIONS
 * ============================================================================
 */

ASSERT(__text_start % PAGE_SIZE == 0, "Text section must be page-aligned")
ASSERT(__rodata_start % PAGE_SIZE == 0, "Rodata section must be page-aligned")
ASSERT(__data_start % PAGE_SIZE == 0, "Data section must be page-aligned")
ASSERT(__bss_start % PAGE_SIZE == 0, "BSS section must be page-aligned")

/* Verify PIE sections exist */
ASSERT(DEFINED(__dynamic_start), ".dynamic section required for PIE")
ASSERT(DEFINED(__rela_start), ".rela section required for PIE")
ASSERT(DEFINED(__got_start), ".got section required for PIE")

/* Verify Limine requests section */
ASSERT(SIZEOF(.limine_requests) > 0, "Limine requests section is empty")

/* ============================================================================
 * EXPORTED SYMBOLS
 * ============================================================================
 *
 * These symbols are used by the relocation subsystem and kernel:
 *
 * __kernel_start/__kernel_end  - Kernel image boundaries
 * __kernel_size                - Total kernel size
 *
 * __dynamic_start/__dynamic_end - Dynamic section (PT_DYNAMIC)
 * __rela_start/__rela_end       - Relocation tables
 * __got_start/__got_end         - Global Offset Table
 *
 * __bss_start/__bss_end         - BSS (needs zeroing)
 *
 * __text_start/__text_end       - Code section (make executable)
 * __rodata_start/__rodata_end   - Read-only data
 * __data_start/__data_end       - Read-write data
 *
 * ============================================================================
 */
