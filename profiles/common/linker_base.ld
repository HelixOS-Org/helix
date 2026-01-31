/* ============================================================================
 * HELIX OS - UNIVERSAL LINKER SCRIPT BASE
 * ============================================================================
 *
 * This linker script provides the foundation for PIE-compatible kernel linking.
 * It can be included by profile-specific linker scripts.
 *
 * Features:
 *   - PIE/KASLR compatible section layout
 *   - Proper .dynamic section for runtime relocation
 *   - GOT/PLT handling for dynamic symbols
 *   - RELRO (Relocation Read-Only) support
 *   - Minimal early-boot requirements
 *
 * Usage:
 *   INCLUDE "profiles/common/linker_base.ld"
 *
 * ============================================================================
 */

/* Output format */
OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)

/* Memory regions - these should be overridden by profile-specific scripts */
HIDDEN(__KERNEL_VMA_BASE = 0xFFFFFFFF80000000);
HIDDEN(__KERNEL_LMA_BASE = 0x0000000000200000);
HIDDEN(__PAGE_SIZE       = 0x1000);
HIDDEN(__HUGE_PAGE_SIZE  = 0x200000);

/* ============================================================================
 * SECTION DEFINITIONS
 * ============================================================================
 */

PHDRS
{
    /* Loadable segments */
    text    PT_LOAD FLAGS(5);    /* R-X: Code */
    rodata  PT_LOAD FLAGS(4);    /* R--: Read-only data */
    data    PT_LOAD FLAGS(6);    /* RW-: Initialized data */
    bss     PT_LOAD FLAGS(6);    /* RW-: Zero-initialized data */

    /* Dynamic linking */
    dynamic PT_DYNAMIC FLAGS(6); /* RW-: Dynamic section */
    relro   PT_GNU_RELRO FLAGS(4); /* R--: RELRO segment */

    /* Stack (no execute) */
    gnu_stack PT_GNU_STACK FLAGS(6); /* RW-: Stack is non-executable */
}

SECTIONS
{
    /* ========================================================================
     * TEXT SEGMENT - Executable Code
     * ========================================================================
     */
    . = __KERNEL_VMA_BASE;
    __kernel_start = .;
    __text_start = .;

    /* Early boot code (must be first, position-independent) */
    .boot ALIGN(__PAGE_SIZE) : AT(__KERNEL_LMA_BASE)
    {
        KEEP(*(.boot.entry))      /* Entry point */
        KEEP(*(.boot.early))      /* Early initialization */
        KEEP(*(.boot.gdt))        /* GDT for mode switch */
        KEEP(*(.boot.paging))     /* Early paging setup */
    } :text

    /* Multiboot headers (if used) */
    .multiboot ALIGN(8) :
    {
        KEEP(*(.multiboot2_header))
        KEEP(*(.multiboot_header))
    } :text

    /* Limine requests (if used) */
    .limine_reqs ALIGN(8) :
    {
        KEEP(*(.limine_requests))
        KEEP(*(.limine_requests_start))
        KEEP(*(.limine_requests_end))
    } :text

    /* Main kernel code */
    .text ALIGN(__PAGE_SIZE) :
    {
        *(.text .text.*)
        *(.gnu.linkonce.t.*)
    } :text

    /* PLT for dynamic linking */
    .plt ALIGN(16) :
    {
        *(.plt)
        *(.plt.*)
    } :text

    __text_end = .;

    /* ========================================================================
     * RODATA SEGMENT - Read-Only Data
     * ========================================================================
     */
    . = ALIGN(__HUGE_PAGE_SIZE);
    __rodata_start = .;

    .rodata ALIGN(__PAGE_SIZE) :
    {
        *(.rodata .rodata.*)
        *(.gnu.linkonce.r.*)
    } :rodata

    /* Exception handling tables */
    .eh_frame_hdr ALIGN(8) :
    {
        KEEP(*(.eh_frame_hdr))
        *(.eh_frame_entry .eh_frame_entry.*)
    } :rodata

    .eh_frame ALIGN(8) :
    {
        KEEP(*(.eh_frame))
        *(.eh_frame.*)
    } :rodata

    /* GCC exception tables */
    .gcc_except_table ALIGN(8) :
    {
        *(.gcc_except_table .gcc_except_table.*)
    } :rodata

    __rodata_end = .;

    /* ========================================================================
     * RELRO SEGMENT - Relocated, then Read-Only
     * ========================================================================
     */
    . = ALIGN(__HUGE_PAGE_SIZE);
    __relro_start = .;

    /* Dynamic linking information */
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
    . = ALIGN(__HUGE_PAGE_SIZE);
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
        *(.rela.plt)
        *(.rela.bss .rela.bss.*)
        *(.rela*)
        __rela_end = .;
    } :data

    .rela.plt ALIGN(8) :
    {
        __rela_plt_start = .;
        *(.rela.plt)
        __rela_plt_end = .;
    } :data

    /* Symbol tables (for debugging/modules) */
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

    /* Hash tables */
    .gnu.hash ALIGN(8) :
    {
        *(.gnu.hash)
    } :data

    .hash ALIGN(8) :
    {
        *(.hash)
    } :data

    /* Initialized data */
    .data ALIGN(__PAGE_SIZE) :
    {
        *(.data .data.*)
        *(.gnu.linkonce.d.*)
    } :data

    /* Thread-local storage */
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
    . = ALIGN(__PAGE_SIZE);
    __bss_start = .;

    .tbss ALIGN(8) :
    {
        __tbss_start = .;
        *(.tbss .tbss.*)
        *(.gnu.linkonce.tb.*)
        __tbss_end = .;
    } :bss

    .bss ALIGN(__PAGE_SIZE) :
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
    . = ALIGN(__PAGE_SIZE);
    __kernel_end = .;

    /* Calculate sizes */
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
     * DEBUG SECTIONS (optional, stripped in release)
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

/* Verify sections are properly aligned */
ASSERT(__text_start % __PAGE_SIZE == 0, "Text section not page-aligned")
ASSERT(__rodata_start % __PAGE_SIZE == 0, "Rodata section not page-aligned")
ASSERT(__data_start % __PAGE_SIZE == 0, "Data section not page-aligned")
ASSERT(__bss_start % __PAGE_SIZE == 0, "BSS section not page-aligned")

/* Verify dynamic section exists for PIE */
ASSERT(DEFINED(__dynamic_start), "No .dynamic section - PIE may not work")

/* Verify relocation tables exist */
ASSERT(DEFINED(__rela_start), "No .rela section - relocations may fail")

/* ============================================================================
 * EXPORTED SYMBOLS FOR RELOCATION
 * ============================================================================
 *
 * These symbols are used by the relocation subsystem:
 *
 * __kernel_start    - Start of kernel (virtual address)
 * __kernel_end      - End of kernel
 * __kernel_size     - Total kernel size
 *
 * __dynamic_start   - Start of .dynamic section
 * __dynamic_end     - End of .dynamic section
 *
 * __rela_start      - Start of relocations
 * __rela_end        - End of relocations
 *
 * __got_start       - Start of GOT
 * __got_end         - End of GOT
 *
 * __bss_start       - Start of BSS (for zeroing)
 * __bss_end         - End of BSS
 *
 * ============================================================================
 */
