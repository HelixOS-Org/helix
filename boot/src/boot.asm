; Helix OS Boot Stub
; This is a minimal multiboot2 header

section .multiboot_header
header_start:
    dd 0xe85250d6                ; magic number
    dd 0                         ; architecture (i386)
    dd header_end - header_start ; header length
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))
    
    ; end tag
    dw 0
    dw 0
    dd 8
header_end:

section .text
global _start
extern kernel_main

_start:
    ; Set up stack
    mov esp, stack_top
    
    ; Call Rust kernel
    call kernel_main
    
    ; Halt
.hang:
    cli
    hlt
    jmp .hang

section .bss
stack_bottom:
    resb 16384
stack_top:
