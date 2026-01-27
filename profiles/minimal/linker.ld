/* =============================================================================
 * Helix OS - Linker Script for x86_64
 * =============================================================================
 * This script ensures proper layout for Multiboot2 boot.
 * The multiboot header MUST be in the first 32KB of the kernel.
 * =============================================================================
 */

ENTRY(_start)

SECTIONS
{
    /* Start at 1MB - standard location for protected mode kernels */
    . = 1M;

    /* Multiboot header MUST be first and within 32KB */
    .multiboot_header : ALIGN(8)
    {
        KEEP(*(.multiboot_header))
    }

    /* Boot code - comes right after header */
    .boot : ALIGN(4K)
    {
        *(.boot)
    }

    /* Main text section */
    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    /* BSS (uninitialized data) */
    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* End of kernel */
    _kernel_end = .;

    /* Discard unnecessary sections */
    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.*)
    }
}
