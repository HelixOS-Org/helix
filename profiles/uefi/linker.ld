/*
 * Helix OS UEFI Kernel Linker Script
 *
 * PIE (Position Independent Executable) linker script for UEFI-booted kernels.
 * Supports higher-half kernel with full relocation capability.
 *
 * Features:
 * - Higher-half kernel mapping (0xFFFFFFFF80000000)
 * - PIE support with .rela.dyn and .dynamic sections
 * - RELRO security (Read-Only Relocations)
 * - Proper section alignment for UEFI
 * - Non-executable stack
 */

/* Entry point - kernel main function */
ENTRY(_start)

/* Higher-half kernel virtual address */
KERNEL_VMA = 0xFFFFFFFF80000000;

/* Physical load address (will be determined by UEFI at runtime) */
KERNEL_LMA = 0x100000;

/* Page size for alignment */
PAGE_SIZE = 0x1000;

/* Large page size for performance-critical sections */
LARGE_PAGE = 0x200000;

/* Program headers for the ELF */
PHDRS
{
    text    PT_LOAD FLAGS(5);   /* R-X: Code segment */
    rodata  PT_LOAD FLAGS(4);   /* R--: Read-only data */
    data    PT_LOAD FLAGS(6);   /* RW-: Data segment */
    bss     PT_LOAD FLAGS(6);   /* RW-: BSS segment */
    dynamic PT_DYNAMIC FLAGS(6); /* Dynamic section */
    relro   PT_GNU_RELRO FLAGS(4); /* Read-only after relocation */
}

SECTIONS
{
    /* ====================================================================== */
    /* TEXT SECTION (Executable Code)                                         */
    /* ====================================================================== */

    . = KERNEL_VMA;
    __kernel_start = .;

    .text ALIGN(PAGE_SIZE) : AT(ADDR(.text) - KERNEL_VMA + KERNEL_LMA)
    {
        __text_start = .;

        /* Entry point - must be first */
        *(.text.entry)
        *(.text._start)

        /* Interrupt handlers (aligned for performance) */
        . = ALIGN(16);
        *(.text.interrupt*)
        *(.text.exception*)
        *(.text.syscall*)

        /* Hot code paths */
        *(.text.hot)

        /* Regular code */
        *(.text .text.*)

        /* Cold code paths */
        *(.text.cold)
        *(.text.unlikely)

        /* PLT stubs for dynamic linking */
        *(.plt)
        *(.plt.*)

        __text_end = .;
    } :text

    /* ====================================================================== */
    /* READ-ONLY DATA                                                          */
    /* ====================================================================== */

    . = ALIGN(PAGE_SIZE);

    .rodata ALIGN(PAGE_SIZE) : AT(ADDR(.rodata) - KERNEL_VMA + KERNEL_LMA)
    {
        __rodata_start = .;

        *(.rodata .rodata.*)

        /* Rust string slices */
        *(.rodata.str1.*)

        /* Rust panic info */
        *(.rodata.cst*)

        /* Exception handling tables */
        *(.eh_frame_hdr)
        *(.eh_frame)
        *(.gcc_except_table)

        __rodata_end = .;
    } :rodata

    /* Build ID for debugging */
    .note.gnu.build-id : AT(ADDR(.note.gnu.build-id) - KERNEL_VMA + KERNEL_LMA)
    {
        *(.note.gnu.build-id)
    } :rodata

    /* ====================================================================== */
    /* RELRO SECTION (Read-Only After Relocation)                              */
    /* ====================================================================== */

    . = ALIGN(PAGE_SIZE);
    __relro_start = .;

    /* Dynamic section */
    .dynamic ALIGN(8) : AT(ADDR(.dynamic) - KERNEL_VMA + KERNEL_LMA)
    {
        __dynamic_start = .;
        *(.dynamic)
        __dynamic_end = .;
    } :relro :dynamic

    /* Global Offset Table */
    .got ALIGN(8) : AT(ADDR(.got) - KERNEL_VMA + KERNEL_LMA)
    {
        __got_start = .;
        *(.got)
        __got_end = .;
    } :relro

    /* GOT for PLT entries */
    .got.plt ALIGN(8) : AT(ADDR(.got.plt) - KERNEL_VMA + KERNEL_LMA)
    {
        __got_plt_start = .;
        *(.got.plt)
        __got_plt_end = .;
    } :relro

    __relro_end = .;

    /* ====================================================================== */
    /* RELOCATION SECTIONS                                                     */
    /* ====================================================================== */

    . = ALIGN(8);

    /* ELF Relocations (addend) */
    .rela.dyn ALIGN(8) : AT(ADDR(.rela.dyn) - KERNEL_VMA + KERNEL_LMA)
    {
        __rela_start = .;
        *(.rela.init)
        *(.rela.text .rela.text.*)
        *(.rela.fini)
        *(.rela.rodata .rela.rodata.*)
        *(.rela.data .rela.data.*)
        *(.rela.tdata .rela.tdata.*)
        *(.rela.tbss .rela.tbss.*)
        *(.rela.bss .rela.bss.*)
        *(.rela.got)
        *(.rela.dyn)
        *(.rela.plt)
        *(.rela*)
        __rela_end = .;
    } :rodata

    /* PLT relocations */
    .rela.plt ALIGN(8) : AT(ADDR(.rela.plt) - KERNEL_VMA + KERNEL_LMA)
    {
        *(.rela.plt)
    } :rodata

    /* ====================================================================== */
    /* DATA SECTION (Initialized Read-Write Data)                              */
    /* ====================================================================== */

    . = ALIGN(PAGE_SIZE);

    .data ALIGN(PAGE_SIZE) : AT(ADDR(.data) - KERNEL_VMA + KERNEL_LMA)
    {
        __data_start = .;

        /* Constructors (C++ and Rust static initialization) */
        __init_array_start = .;
        KEEP(*(.init_array .init_array.*))
        __init_array_end = .;

        /* Destructors */
        __fini_array_start = .;
        KEEP(*(.fini_array .fini_array.*))
        __fini_array_end = .;

        /* Regular data */
        *(.data .data.*)

        /* Small data sections */
        *(.sdata .sdata.*)

        __data_end = .;
    } :data

    /* Thread-local storage template */
    .tdata ALIGN(8) : AT(ADDR(.tdata) - KERNEL_VMA + KERNEL_LMA)
    {
        __tdata_start = .;
        *(.tdata .tdata.*)
        __tdata_end = .;
    } :data

    /* ====================================================================== */
    /* BSS SECTION (Uninitialized Data)                                        */
    /* ====================================================================== */

    . = ALIGN(PAGE_SIZE);

    .tbss (NOLOAD) : AT(ADDR(.tbss) - KERNEL_VMA + KERNEL_LMA)
    {
        __tbss_start = .;
        *(.tbss .tbss.*)
        __tbss_end = .;
    } :bss

    .bss (NOLOAD) ALIGN(PAGE_SIZE) : AT(ADDR(.bss) - KERNEL_VMA + KERNEL_LMA)
    {
        __bss_start = .;

        /* Small BSS */
        *(.sbss .sbss.*)

        /* Regular BSS */
        *(.bss .bss.*)
        *(COMMON)

        __bss_end = .;
    } :bss

    /* ====================================================================== */
    /* KERNEL HEAP AND STACK REGIONS                                           */
    /* ====================================================================== */

    . = ALIGN(PAGE_SIZE);
    __kernel_end = .;

    /* Reserve space for early boot stack (16KB) */
    . = ALIGN(PAGE_SIZE);
    __boot_stack_bottom = .;
    . += 0x4000;
    __boot_stack_top = .;

    /* Reserve space for early kernel heap (1MB) */
    . = ALIGN(PAGE_SIZE);
    __heap_start = .;
    . += 0x100000;
    __heap_end = .;

    /* ====================================================================== */
    /* SYMBOL TABLE AND DEBUG INFO                                             */
    /* ====================================================================== */

    /* Symbol table (stripped in release builds) */
    .symtab 0 : { *(.symtab) }
    .strtab 0 : { *(.strtab) }
    .shstrtab 0 : { *(.shstrtab) }

    /* Debug sections (stripped in release builds) */
    .debug_info 0 : { *(.debug_info .debug_info.*) }
    .debug_abbrev 0 : { *(.debug_abbrev) }
    .debug_line 0 : { *(.debug_line .debug_line.*) }
    .debug_str 0 : { *(.debug_str) }
    .debug_ranges 0 : { *(.debug_ranges) }
    .debug_frame 0 : { *(.debug_frame) }
    .debug_loc 0 : { *(.debug_loc) }
    .debug_aranges 0 : { *(.debug_aranges) }

    /* ====================================================================== */
    /* DISCARDED SECTIONS                                                      */
    /* ====================================================================== */

    /DISCARD/ :
    {
        /* Discard unwanted sections */
        *(.note.GNU-stack)
        *(.comment)
        *(.gnu.hash)
        *(.hash)
        *(.interp)
    }
}

/* ========================================================================== */
/* EXPORTED SYMBOLS FOR RELOCATION                                            */
/* ========================================================================== */

/* These symbols are used by the relocation code to find section boundaries */
PROVIDE(__kernel_start = __kernel_start);
PROVIDE(__kernel_end = __kernel_end);
PROVIDE(__text_start = __text_start);
PROVIDE(__text_end = __text_end);
PROVIDE(__rodata_start = __rodata_start);
PROVIDE(__rodata_end = __rodata_end);
PROVIDE(__data_start = __data_start);
PROVIDE(__data_end = __data_end);
PROVIDE(__bss_start = __bss_start);
PROVIDE(__bss_end = __bss_end);
PROVIDE(__rela_start = __rela_start);
PROVIDE(__rela_end = __rela_end);
PROVIDE(__dynamic_start = __dynamic_start);
PROVIDE(__dynamic_end = __dynamic_end);
PROVIDE(__got_start = __got_start);
PROVIDE(__got_end = __got_end);
PROVIDE(__relro_start = __relro_start);
PROVIDE(__relro_end = __relro_end);
PROVIDE(__boot_stack_top = __boot_stack_top);
PROVIDE(__boot_stack_bottom = __boot_stack_bottom);
PROVIDE(__heap_start = __heap_start);
PROVIDE(__heap_end = __heap_end);

/* ========================================================================== */
/* SIZE CALCULATIONS                                                           */
/* ========================================================================== */

__kernel_size = __kernel_end - __kernel_start;
__text_size = __text_end - __text_start;
__rodata_size = __rodata_end - __rodata_start;
__data_size = __data_end - __data_start;
__bss_size = __bss_end - __bss_start;
__rela_size = __rela_end - __rela_start;
__dynamic_size = __dynamic_end - __dynamic_start;

/* Assertions for sanity checks */
ASSERT(__kernel_start == KERNEL_VMA, "Kernel must start at KERNEL_VMA")
ASSERT(__text_start >= __kernel_start, "Text section must be within kernel")
ASSERT(__rela_end <= __kernel_end, "Relocation section must be within kernel")
ASSERT(__boot_stack_top > __boot_stack_bottom, "Stack must grow upward in memory")
