# Helix Boot Process

<div align="center">

⚡ **Complete Boot Sequence Documentation**

*From power-on to kernel_main*

</div>

---

## Table of Contents

1. [Boot Overview](#1-boot-overview)
2. [BIOS/UEFI Stage](#2-biosuefi-stage)
3. [GRUB Bootloader](#3-grub-bootloader)
4. [Assembly Boot Code](#4-assembly-boot-code)
5. [Long Mode Transition](#5-long-mode-transition)
6. [Kernel Entry](#6-kernel-entry)
7. [Initialization Phases](#7-initialization-phases)
8. [Memory Map](#8-memory-map)
9. [Debug and Troubleshooting](#9-debug-and-troubleshooting)

---

## 1. Boot Overview

### 1.1 Complete Boot Timeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        HELIX BOOT TIMELINE                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Time (ms)  Event                                                           │
│  ─────────  ─────                                                           │
│                                                                             │
│  0          Power On                                                        │
│             └── CPU starts in Real Mode (16-bit)                            │
│             └── Executes BIOS/UEFI from ROM                                 │
│                                                                             │
│  ~500       BIOS POST                                                       │
│             └── Hardware initialization                                     │
│             └── Memory detection                                            │
│             └── Boot device selection                                       │
│                                                                             │
│  ~1000      GRUB Loaded                                                     │
│             └── Stage 1: MBR/GPT boot code                                  │
│             └── Stage 1.5: Filesystem drivers                               │
│             └── Stage 2: Main GRUB                                          │
│                                                                             │
│  ~1200      grub.cfg Parsed                                                 │
│             └── Menu displayed (if timeout > 0)                             │
│             └── Helix kernel selected                                       │
│                                                                             │
│  ~1500      Kernel Loading                                                  │
│             └── ELF header parsed                                           │
│             └── Segments loaded to memory                                   │
│             └── Multiboot2 info structure created                           │
│                                                                             │
│  ~1800      _start (Assembly)                                               │
│             └── Stack setup                                                 │
│             └── GDT installation                                            │
│             └── Long mode enable                                            │
│             └── Paging setup                                                │
│                                                                             │
│  ~1850      kernel_main (Rust)                                              │
│             └── Serial initialization                                       │
│             └── Heap initialization                                         │
│             └── Subsystem initialization                                    │
│             └── Shell/main loop                                             │
│                                                                             │
│  ~2000      System Ready                                                    │
│             └── Shell prompt displayed                                      │
│             └── System fully operational                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Boot Stages

| Stage | Mode | Component | Description |
|-------|------|-----------|-------------|
| 1 | Real (16-bit) | BIOS/UEFI | Hardware init, boot device |
| 2 | Real/Protected | GRUB Stage 1 | Disk read, load Stage 2 |
| 3 | Protected (32-bit) | GRUB Stage 2 | Parse config, load kernel |
| 4 | Protected (32-bit) | _start (ASM) | Setup, enter long mode |
| 5 | Long (64-bit) | _start64 (ASM) | Final setup, jump to Rust |
| 6 | Long (64-bit) | kernel_main | Rust kernel initialization |

---

## 2. BIOS/UEFI Stage

### 2.1 BIOS Boot (Legacy)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          BIOS BOOT PROCESS                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  1. Power-On Self-Test (POST)                                               │
│     └── CPU reset vector: 0xFFFFFFF0                                        │
│     └── Jump to BIOS ROM                                                    │
│     └── Initialize chipset, memory controller                               │
│     └── Detect installed memory                                             │
│     └── Initialize video                                                    │
│                                                                             │
│  2. Device Enumeration                                                      │
│     └── Detect hard drives, CD-ROMs                                         │
│     └── Detect USB devices                                                  │
│     └── Assign legacy interrupts                                            │
│                                                                             │
│  3. Boot Device Selection                                                   │
│     └── Check boot order (BIOS settings)                                    │
│     └── Try each device in order                                            │
│     └── Load first sector (MBR) to 0x7C00                                   │
│                                                                             │
│  4. MBR Execution                                                           │
│     └── CPU jumps to 0x7C00                                                 │
│     └── MBR code loads GRUB Stage 1                                         │
│                                                                             │
│  Memory Map at this stage:                                                  │
│  ─────────────────────────                                                  │
│  0x00000000 - 0x000003FF : Interrupt Vector Table                           │
│  0x00000400 - 0x000004FF : BIOS Data Area                                   │
│  0x00000500 - 0x00007BFF : Free                                             │
│  0x00007C00 - 0x00007DFF : MBR (loaded by BIOS)                             │
│  0x00007E00 - 0x0009FBFF : Free (conventional memory)                       │
│  0x0009FC00 - 0x0009FFFF : Extended BIOS Data Area                          │
│  0x000A0000 - 0x000BFFFF : Video memory                                     │
│  0x000C0000 - 0x000FFFFF : BIOS ROM, option ROMs                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 UEFI Boot (Modern)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          UEFI BOOT PROCESS                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Note: Helix currently uses BIOS boot via GRUB.                             │
│  UEFI support is planned for future versions.                               │
│                                                                             │
│  UEFI Boot Flow (for reference):                                            │
│  ────────────────────────────────                                           │
│                                                                             │
│  1. SEC (Security Phase)                                                    │
│     └── Platform initialization                                             │
│     └── Verify firmware integrity                                           │
│                                                                             │
│  2. PEI (Pre-EFI Initialization)                                            │
│     └── Initialize memory                                                   │
│     └── Prepare for DXE                                                     │
│                                                                             │
│  3. DXE (Driver Execution Environment)                                      │
│     └── Load UEFI drivers                                                   │
│     └── Initialize boot services                                            │
│                                                                             │
│  4. BDS (Boot Device Selection)                                             │
│     └── Find ESP (EFI System Partition)                                     │
│     └── Load EFI bootloader                                                 │
│     └── Or load GRUB EFI                                                    │
│                                                                             │
│  5. TSL (Transient System Load)                                             │
│     └── Bootloader runs                                                     │
│     └── Loads kernel                                                        │
│     └── Calls ExitBootServices()                                            │
│                                                                             │
│  6. RT (Runtime)                                                            │
│     └── Kernel takes over                                                   │
│     └── UEFI runtime services available                                     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. GRUB Bootloader

### 3.1 GRUB Configuration

```bash
# /boot/grub/grub.cfg

# Set timeout (0 for instant boot)
set timeout=0

# Set default boot entry
set default=0

# Helix boot entry
menuentry "Helix OS Framework" {
    # Load Multiboot2 kernel
    multiboot2 /boot/helix-kernel
    
    # Boot the kernel
    boot
}

# Helix with debug options
menuentry "Helix OS Framework (Debug)" {
    multiboot2 /boot/helix-kernel debug serial=on
    boot
}
```

### 3.2 Multiboot2 Header

Helix uses the Multiboot2 specification:

```rust
// profiles/minimal/src/boot.rs

/// Multiboot2 magic number (must be at start of header)
const MULTIBOOT2_MAGIC: u32 = 0xE85250D6;

/// Architecture: x86 protected mode
const ARCHITECTURE: u32 = 0;

/// Header length
const HEADER_LENGTH: u32 = 24;

/// Checksum: -(magic + arch + length)
const CHECKSUM: u32 = 0x100000000 - (MULTIBOOT2_MAGIC + ARCHITECTURE + HEADER_LENGTH);

#[repr(C, align(8))]
pub struct Multiboot2Header {
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
    // End tag
    end_tag_type: u16,
    end_tag_flags: u16,
    end_tag_size: u32,
}

#[used]
#[link_section = ".multiboot2"]
static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header {
    magic: MULTIBOOT2_MAGIC,
    architecture: ARCHITECTURE,
    header_length: HEADER_LENGTH,
    checksum: CHECKSUM,
    end_tag_type: 0,
    end_tag_flags: 0,
    end_tag_size: 8,
};
```

### 3.3 Multiboot2 Information Structure

GRUB passes information to the kernel:

```rust
/// Multiboot2 information passed by bootloader
pub struct Multiboot2Info {
    pub total_size: u32,
    pub reserved: u32,
    // Followed by tags...
}

/// Tag types
pub const TAG_TYPE_END: u32 = 0;
pub const TAG_TYPE_CMDLINE: u32 = 1;
pub const TAG_TYPE_BOOTLOADER: u32 = 2;
pub const TAG_TYPE_MODULE: u32 = 3;
pub const TAG_TYPE_MEMORY: u32 = 4;
pub const TAG_TYPE_BOOTDEV: u32 = 5;
pub const TAG_TYPE_MMAP: u32 = 6;
pub const TAG_TYPE_FRAMEBUFFER: u32 = 8;
pub const TAG_TYPE_ELF_SECTIONS: u32 = 9;
pub const TAG_TYPE_APM: u32 = 10;
pub const TAG_TYPE_ACPI_OLD: u32 = 14;
pub const TAG_TYPE_ACPI_NEW: u32 = 15;

/// Memory map entry
#[repr(C)]
pub struct MemoryMapEntry {
    pub base_addr: u64,
    pub length: u64,
    pub entry_type: u32,  // 1 = available, 2 = reserved, etc.
    pub reserved: u32,
}
```

---

## 4. Assembly Boot Code

### 4.1 Entry Point (_start)

```asm
; boot/src/boot.asm

section .multiboot2
align 8
multiboot2_header:
    dd 0xE85250D6           ; Magic
    dd 0                    ; Architecture (x86)
    dd multiboot2_header_end - multiboot2_header  ; Length
    dd -(0xE85250D6 + 0 + (multiboot2_header_end - multiboot2_header)) ; Checksum
    
    ; End tag
    dw 0                    ; Type
    dw 0                    ; Flags
    dd 8                    ; Size
multiboot2_header_end:

section .bss
align 16
stack_bottom:
    resb 16384              ; 16 KB stack
stack_top:

section .text
global _start
extern kernel_main

_start:
    ; Disable interrupts
    cli
    
    ; Save Multiboot2 info pointer (passed in EBX)
    mov edi, ebx
    
    ; Set up stack
    lea esp, [stack_top]
    
    ; Clear direction flag
    cld
    
    ; Check for long mode support
    call check_long_mode
    
    ; Set up paging
    call setup_paging
    
    ; Load GDT
    lgdt [gdt64.pointer]
    
    ; Enable PAE
    mov eax, cr4
    or eax, 1 << 5          ; PAE bit
    mov cr4, eax
    
    ; Set LME (Long Mode Enable)
    mov ecx, 0xC0000080     ; EFER MSR
    rdmsr
    or eax, 1 << 8          ; LME bit
    wrmsr
    
    ; Enable paging
    mov eax, cr0
    or eax, 1 << 31         ; PG bit
    mov cr0, eax
    
    ; Far jump to 64-bit code
    jmp gdt64.code:long_mode_start

check_long_mode:
    ; Check CPUID availability
    pushfd
    pop eax
    mov ecx, eax
    xor eax, 1 << 21        ; Flip ID bit
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
    cmp eax, ecx
    je .no_cpuid
    
    ; Check extended CPUID
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_long_mode
    
    ; Check long mode
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29       ; LM bit
    jz .no_long_mode
    
    ret
    
.no_cpuid:
.no_long_mode:
    hlt
    jmp $

setup_paging:
    ; Identity map first 2MB using 2MB pages
    
    ; PML4[0] -> PDPT
    mov eax, pdpt
    or eax, 0x03            ; Present + Writable
    mov [pml4], eax
    
    ; PDPT[0] -> PD
    mov eax, pd
    or eax, 0x03
    mov [pdpt], eax
    
    ; PD[0] -> 2MB page at 0x0
    mov dword [pd], 0x83    ; Present + Writable + Huge (2MB)
    
    ; PD[1] -> 2MB page at 0x200000 (for kernel)
    mov dword [pd + 8], 0x200083
    
    ; Load PML4 into CR3
    mov eax, pml4
    mov cr3, eax
    
    ret

section .rodata
gdt64:
    dq 0                    ; Null descriptor
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53)  ; Code segment
.data: equ $ - gdt64
    dq (1<<44) | (1<<47)    ; Data segment
.pointer:
    dw $ - gdt64 - 1        ; Limit
    dq gdt64                ; Base

section .bss
align 4096
pml4:
    resb 4096
pdpt:
    resb 4096
pd:
    resb 4096

section .text
bits 64
long_mode_start:
    ; Reload segment registers
    mov ax, gdt64.data
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ; Set up 64-bit stack
    mov rsp, stack_top
    
    ; Multiboot2 info pointer is in EDI (zero-extended to RDI)
    
    ; Call Rust kernel_main
    call kernel_main
    
    ; Halt if kernel returns
.halt:
    cli
    hlt
    jmp .halt
```

### 4.2 Key Assembly Concepts

| Instruction | Purpose |
|-------------|---------|
| `cli` | Disable interrupts |
| `lgdt` | Load Global Descriptor Table |
| `mov cr3, eax` | Load page table base |
| `mov cr0, eax` | Enable/disable paging, protection |
| `mov cr4, eax` | Enable PAE, other features |
| `wrmsr` | Write to Model-Specific Register |
| `jmp far` | Jump with segment change |

---

## 5. Long Mode Transition

### 5.1 CPU Mode Transitions

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       CPU MODE TRANSITIONS                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐                                                            │
│  │  Real Mode  │  16-bit, 1MB address space                                 │
│  │   (BIOS)    │  Segment:Offset addressing                                 │
│  └──────┬──────┘                                                            │
│         │                                                                   │
│         │ Set PE bit in CR0                                                 │
│         ▼                                                                   │
│  ┌─────────────┐                                                            │
│  │ Protected   │  32-bit, 4GB address space                                 │
│  │    Mode     │  Segment descriptors, GDT                                  │
│  │   (GRUB)    │  No paging yet                                             │
│  └──────┬──────┘                                                            │
│         │                                                                   │
│         │ 1. Set up page tables                                             │
│         │ 2. Enable PAE (CR4.PAE = 1)                                       │
│         │ 3. Load CR3 with PML4 address                                     │
│         │ 4. Enable LME (EFER.LME = 1)                                      │
│         │ 5. Enable paging (CR0.PG = 1)                                     │
│         │ 6. Far jump to 64-bit code segment                                │
│         ▼                                                                   │
│  ┌─────────────┐                                                            │
│  │  Long Mode  │  64-bit, 256TB virtual address space                       │
│  │   (Helix)   │  4-level paging, RIP-relative addressing                   │
│  └─────────────┘                                                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 Page Table Setup

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      4-LEVEL PAGE TABLE STRUCTURE                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Virtual Address (64-bit, canonical):                                       │
│  ┌────────────┬─────────┬─────────┬─────────┬─────────┬──────────────┐     │
│  │  Sign Ext  │  PML4   │  PDPT   │   PD    │   PT    │    Offset    │     │
│  │   16 bits  │  9 bits │  9 bits │  9 bits │  9 bits │    12 bits   │     │
│  │    63-48   │  47-39  │  38-30  │  29-21  │  20-12  │     11-0     │     │
│  └────────────┴────┬────┴────┬────┴────┬────┴────┬────┴──────────────┘     │
│                    │         │         │         │                         │
│                    ▼         ▼         ▼         ▼                         │
│               ┌─────────┐                                                   │
│               │  PML4   │  512 entries, each points to PDPT                 │
│               │  Table  │  Entry = PDPT address | flags                     │
│               └────┬────┘                                                   │
│                    │                                                        │
│                    ▼                                                        │
│               ┌─────────┐                                                   │
│               │  PDPT   │  512 entries, each points to PD                   │
│               │  Table  │  Or 1GB page (if PS bit set)                      │
│               └────┬────┘                                                   │
│                    │                                                        │
│                    ▼                                                        │
│               ┌─────────┐                                                   │
│               │   PD    │  512 entries, each points to PT                   │
│               │  Table  │  Or 2MB page (if PS bit set)                      │
│               └────┬────┘                                                   │
│                    │                                                        │
│                    ▼                                                        │
│               ┌─────────┐                                                   │
│               │   PT    │  512 entries, each points to 4KB page             │
│               │  Table  │  Entry = Physical address | flags                 │
│               └────┬────┘                                                   │
│                    │                                                        │
│                    ▼                                                        │
│               ┌─────────┐                                                   │
│               │ Physical│  4KB page in physical memory                      │
│               │  Page   │                                                   │
│               └─────────┘                                                   │
│                                                                             │
│  Page Table Entry Flags:                                                    │
│  ───────────────────────                                                    │
│  Bit 0  (P):   Present                                                      │
│  Bit 1  (RW):  Read/Write                                                   │
│  Bit 2  (US):  User/Supervisor                                              │
│  Bit 3  (PWT): Page Write-Through                                           │
│  Bit 4  (PCD): Page Cache Disable                                           │
│  Bit 5  (A):   Accessed                                                     │
│  Bit 6  (D):   Dirty                                                        │
│  Bit 7  (PS):  Page Size (1 = huge page)                                    │
│  Bit 8  (G):   Global                                                       │
│  Bit 63 (NX):  No Execute                                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 5.3 GDT for Long Mode

```rust
/// Global Descriptor Table for 64-bit mode
#[repr(C, packed)]
pub struct Gdt64 {
    null: u64,        // Null descriptor (required)
    code: u64,        // Code segment
    data: u64,        // Data segment
}

impl Gdt64 {
    pub const fn new() -> Self {
        Self {
            null: 0,
            // Code segment: Executable, Readable, Present, 64-bit
            code: (1 << 43)  // Executable
                | (1 << 44)  // Code/Data (1 = code)
                | (1 << 47)  // Present
                | (1 << 53), // Long mode
            // Data segment: Writable, Present
            data: (1 << 44)  // Code/Data (0 in bit 43 = data)
                | (1 << 47), // Present
        }
    }
}

#[repr(C, packed)]
pub struct GdtPointer {
    limit: u16,
    base: u64,
}
```

---

## 6. Kernel Entry

### 6.1 kernel_main Function

```rust
// profiles/minimal/src/main.rs

/// Kernel entry point - called from assembly
#[no_mangle]
pub extern "C" fn kernel_main(multiboot_info: u64) -> ! {
    // Phase 1: Early initialization (no allocator yet)
    early_init();
    
    // Phase 2: Parse Multiboot2 information
    let mb_info = unsafe { parse_multiboot2(multiboot_info) };
    
    // Phase 3: Initialize memory
    init_heap();
    init_memory(&mb_info);
    
    // Phase 4: Initialize subsystems
    init_interrupts();
    init_scheduler();
    init_modules();
    
    // Phase 5: Initialize filesystem
    init_helixfs();
    
    // Phase 6: Start system
    start_kernel();
    
    // Should never reach here
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

fn early_init() {
    // Initialize serial port for output
    serial::init();
    
    // Clear screen (if VGA available)
    vga::clear();
    
    // Print banner
    print_banner();
}

fn print_banner() {
    serial_println!("");
    serial_println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    serial_println!("║                          HELIX OS FRAMEWORK v0.1.0                           ║");
    serial_println!("║                     A Framework for Building Operating Systems               ║");
    serial_println!("╚══════════════════════════════════════════════════════════════════════════════╝");
    serial_println!("");
}
```

### 6.2 Serial Port Initialization

```rust
// Early serial port initialization (before allocator)

const SERIAL_PORT: u16 = 0x3F8;  // COM1

pub fn init() {
    unsafe {
        // Disable interrupts
        outb(SERIAL_PORT + 1, 0x00);
        
        // Enable DLAB (set baud rate divisor)
        outb(SERIAL_PORT + 3, 0x80);
        
        // Set divisor to 1 (115200 baud)
        outb(SERIAL_PORT + 0, 0x01);  // Low byte
        outb(SERIAL_PORT + 1, 0x00);  // High byte
        
        // 8 bits, no parity, one stop bit
        outb(SERIAL_PORT + 3, 0x03);
        
        // Enable FIFO, clear, 14-byte threshold
        outb(SERIAL_PORT + 2, 0xC7);
        
        // IRQs enabled, RTS/DSR set
        outb(SERIAL_PORT + 4, 0x0B);
    }
}

pub fn write_byte(byte: u8) {
    unsafe {
        // Wait for transmit buffer to be empty
        while (inb(SERIAL_PORT + 5) & 0x20) == 0 {}
        outb(SERIAL_PORT, byte);
    }
}

unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
    );
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
    );
    value
}
```

---

## 7. Initialization Phases

### 7.1 Phase Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      KERNEL INITIALIZATION PHASES                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  PHASE 1: EARLY INITIALIZATION                                              │
│  ═══════════════════════════════                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  [✓] Initialize serial port (debug output)                          │   │
│  │  [✓] Initialize VGA console (if available)                          │   │
│  │  [✓] Print boot banner                                              │   │
│  │  [✓] Display kernel version                                         │   │
│  │                                                                     │   │
│  │  Constraints: No allocator, no interrupts, static data only         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                      │                                      │
│                                      ▼                                      │
│  PHASE 2: MEMORY SETUP                                                      │
│  ═════════════════════                                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  [✓] Parse Multiboot2 memory map                                    │   │
│  │  [✓] Initialize bump allocator (kernel heap)                        │   │
│  │  [✓] Initialize physical memory manager                             │   │
│  │  [✓] Set up kernel page tables                                      │   │
│  │                                                                     │   │
│  │  Output: Working allocator, known memory layout                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                      │                                      │
│                                      ▼                                      │
│  PHASE 3: INTERRUPT SETUP                                                   │
│  ════════════════════════                                                   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  [✓] Initialize IDT (Interrupt Descriptor Table)                    │   │
│  │  [✓] Install exception handlers (0-31)                              │   │
│  │  [✓] Install IRQ handlers (32-47)                                   │   │
│  │  [✓] Initialize PIC (Programmable Interrupt Controller)             │   │
│  │  [✓] Enable interrupts (STI)                                        │   │
│  │                                                                     │   │
│  │  Output: Interrupt handling operational                             │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                      │                                      │
│                                      ▼                                      │
│  PHASE 4: SUBSYSTEM INITIALIZATION                                          │
│  ═════════════════════════════════                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  [✓] Initialize scheduler framework                                 │   │
│  │  [✓] Load default scheduler (RoundRobin)                            │   │
│  │  [✓] Initialize module registry                                     │   │
│  │  [✓] Initialize hot-reload engine                                   │   │
│  │  [✓] Initialize self-healing system                                 │   │
│  │  [✓] Initialize IPC subsystem                                       │   │
│  │                                                                     │   │
│  │  Output: Core subsystems operational                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                      │                                      │
│                                      ▼                                      │
│  PHASE 5: FILESYSTEM INITIALIZATION                                         │
│  ══════════════════════════════════                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  [✓] Initialize HelixFS                                             │   │
│  │  [✓] Set up RAM disk backend                                        │   │
│  │  [✓] Format filesystem                                              │   │
│  │  [✓] Create root directory                                          │   │
│  │  [✓] Mount filesystem                                               │   │
│  │                                                                     │   │
│  │  Output: Filesystem ready for I/O                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                      │                                      │
│                                      ▼                                      │
│  PHASE 6: SYSTEM START                                                      │
│  ═════════════════════                                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  [✓] Run filesystem demo (create sample files)                      │   │
│  │  [✓] Run hot-reload demo                                            │   │
│  │  [✓] Run benchmarks                                                 │   │
│  │  [✓] Start shell                                                    │   │
│  │  [✓] Enter main loop                                                │   │
│  │                                                                     │   │
│  │  Output: System fully operational                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.2 Initialization Code

```rust
/// Phase 2: Memory initialization
fn init_memory(mb_info: &Multiboot2Info) {
    serial_println!("[MEM] Initializing memory subsystem...");
    
    // Parse memory map from Multiboot2
    let memory_map = parse_memory_map(mb_info);
    
    // Initialize physical allocator
    let total_memory = memory_map.total_available();
    serial_println!("[MEM] Total physical memory: {} MB", total_memory / 1024 / 1024);
    
    // Reserve kernel memory
    let kernel_end = get_kernel_end();
    reserve_kernel_memory(kernel_end);
    
    // Initialize page allocator
    init_page_allocator(&memory_map);
    
    serial_println!("[MEM] Memory subsystem ready");
}

/// Phase 3: Interrupt initialization
fn init_interrupts() {
    serial_println!("[INT] Installing interrupt handlers...");
    
    // Create IDT
    let mut idt = InterruptDescriptorTable::new();
    
    // Install exception handlers
    idt.divide_error.set_handler(divide_error_handler);
    idt.debug.set_handler(debug_handler);
    idt.breakpoint.set_handler(breakpoint_handler);
    idt.double_fault.set_handler(double_fault_handler);
    idt.page_fault.set_handler(page_fault_handler);
    idt.general_protection.set_handler(gp_fault_handler);
    
    // Install IRQ handlers
    idt[32].set_handler(timer_handler);
    idt[33].set_handler(keyboard_handler);
    
    // Load IDT
    idt.load();
    
    // Initialize PIC
    init_pic();
    
    // Enable interrupts
    unsafe { core::arch::asm!("sti"); }
    
    serial_println!("[INT] Interrupts enabled");
}

/// Phase 4: Scheduler initialization
fn init_scheduler() {
    serial_println!("[SCHED] Initializing scheduler...");
    
    // Create scheduler registry slot
    let slot = HOTRELOAD_REGISTRY.lock().create_slot(ModuleCategory::Scheduler);
    
    // Load default scheduler
    let scheduler = Box::new(RoundRobinScheduler::new());
    HOTRELOAD_REGISTRY.lock().load_module(slot, scheduler).unwrap();
    
    serial_println!("[SCHED] Default scheduler: RoundRobin v0.1.0");
    serial_println!("[SCHED] Scheduler ready");
}

/// Phase 5: Filesystem initialization
fn init_helixfs() {
    serial_println!("[FS] Initializing HelixFS...");
    
    // Initialize RAM disk
    filesystem::init_ramdisk();
    
    // Format and mount
    filesystem::format_filesystem().unwrap();
    
    // Create root directory
    filesystem::create_root().unwrap();
    
    let stats = filesystem::get_stats();
    serial_println!("[FS] Total: {} KB, Free: {} KB", 
        stats.total_blocks * 4, 
        stats.free_blocks * 4);
    serial_println!("[FS] HelixFS ready");
}
```

---

## 8. Memory Map

### 8.1 Physical Memory Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    PHYSICAL MEMORY LAYOUT (256MB)                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Address        Size       Description                                      │
│  ───────────────────────────────────────────────────────────────────────    │
│                                                                             │
│  0x00000000    1 MB       Reserved / BIOS                                   │
│  ├─ 0x00000000  1 KB      Real Mode IVT                                     │
│  ├─ 0x00000400  256 B     BIOS Data Area                                    │
│  ├─ 0x00000500  ~30 KB    Free (real mode)                                  │
│  ├─ 0x00007C00  512 B     Boot sector (temporary)                           │
│  ├─ 0x00007E00  ~480 KB   Free (real mode)                                  │
│  ├─ 0x00080000  128 KB    EBDA (Extended BIOS Data)                         │
│  ├─ 0x000A0000  128 KB    Video RAM                                         │
│  └─ 0x000C0000  256 KB    ROM (BIOS, Option ROMs)                           │
│                                                                             │
│  0x00100000    ~2 MB      Kernel Image                                      │
│  ├─ .text                 Executable code                                   │
│  ├─ .rodata               Read-only data                                    │
│  ├─ .data                 Initialized data                                  │
│  └─ .bss                  Zero-initialized data                             │
│                                                                             │
│  0x00300000    1 MB       Kernel Heap                                       │
│  ├─ Bump allocator region                                                   │
│  └─ Dynamic allocations                                                     │
│                                                                             │
│  0x00400000    256 KB     Page Tables                                       │
│  ├─ PML4                                                                    │
│  ├─ PDPTs                                                                   │
│  ├─ PDs                                                                     │
│  └─ PTs                                                                     │
│                                                                             │
│  0x00440000    4 MB       RAM Disk (HelixFS)                                │
│  └─ Filesystem blocks                                                       │
│                                                                             │
│  0x00840000    ~247 MB    Free Physical Memory                              │
│  └─ Available for allocation                                                │
│                                                                             │
│  0x10000000                End of 256 MB                                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Virtual Memory Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     VIRTUAL MEMORY LAYOUT (x86_64)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  0xFFFF_FFFF_FFFF_FFFF  ┌────────────────────────────────────────────────┐ │
│                         │                                                │ │
│  0xFFFF_FFFF_8000_0000  │  Kernel Code/Data (mapped from 0x100000)       │ │
│                         │  Identity mapped for simplicity                │ │
│                         │                                                │ │
│  0xFFFF_FF80_0000_0000  ├────────────────────────────────────────────────┤ │
│                         │                                                │ │
│                         │  Physical Memory Direct Map                    │ │
│                         │  All physical memory mapped here              │ │
│                         │                                                │ │
│  0xFFFF_8000_0000_0000  ├────────────────────────────────────────────────┤ │
│                         │                                                │ │
│                         │            KERNEL SPACE                        │ │
│                         │                                                │ │
│  ────────────────────── ├────────────────────────────────────────────────┤ │
│                         │                                                │ │
│                         │     NON-CANONICAL HOLE                         │ │
│                         │     (Not addressable)                          │ │
│                         │                                                │ │
│  ────────────────────── ├────────────────────────────────────────────────┤ │
│                         │                                                │ │
│                         │             USER SPACE                         │ │
│                         │                                                │ │
│  0x0000_7FFF_FFFF_FFFF  ├────────────────────────────────────────────────┤ │
│                         │  User Stack (grows down)                       │ │
│  0x0000_7FFF_0000_0000  ├────────────────────────────────────────────────┤ │
│                         │                                                │ │
│                         │  User Heap (grows up)                          │ │
│                         │                                                │ │
│                         ├────────────────────────────────────────────────┤ │
│                         │  User Code/Data                                │ │
│  0x0000_0000_0040_0000  ├────────────────────────────────────────────────┤ │
│                         │  Reserved                                      │ │
│  0x0000_0000_0000_0000  └────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Debug and Troubleshooting

### 9.1 Common Boot Issues

| Symptom | Cause | Solution |
|---------|-------|----------|
| Blank screen | VGA not initialized | Check serial output |
| Triple fault | Invalid page table | Debug with QEMU `-d int` |
| Hang after GRUB | Stack overflow | Increase stack size |
| No serial output | Wrong port | Try 0x3F8 (COM1) |
| "No multiboot" | Header not found | Check linker script |

### 9.2 Debug with QEMU

```bash
# Enable interrupt logging
qemu-system-x86_64 -cdrom helix.iso -d int -D /tmp/int.log

# Enable CPU reset logging
qemu-system-x86_64 -cdrom helix.iso -d cpu_reset -D /tmp/reset.log

# Full debug (verbose)
qemu-system-x86_64 -cdrom helix.iso -d int,cpu_reset,exec -D /tmp/full.log

# GDB debugging
qemu-system-x86_64 -cdrom helix.iso -s -S
# Then in another terminal:
gdb build/output/helix-kernel -ex "target remote :1234"
```

### 9.3 Boot Checkpoints

Add these to track boot progress:

```rust
fn kernel_main(multiboot_info: u64) -> ! {
    serial_println!("[BOOT] Checkpoint 1: Entered kernel_main");
    
    early_init();
    serial_println!("[BOOT] Checkpoint 2: Early init complete");
    
    init_heap();
    serial_println!("[BOOT] Checkpoint 3: Heap initialized");
    
    init_interrupts();
    serial_println!("[BOOT] Checkpoint 4: Interrupts ready");
    
    // ... etc
}
```

---

## Summary

The Helix boot process:

1. **BIOS/UEFI** initializes hardware
2. **GRUB** loads the kernel with Multiboot2
3. **Assembly code** sets up long mode and paging
4. **kernel_main** initializes subsystems in phases
5. **Shell** becomes available for interaction

Key files:
- `boot/src/boot.asm` - Assembly entry point
- `profiles/minimal/src/boot.rs` - Multiboot2 header
- `profiles/minimal/src/main.rs` - Rust kernel entry

---

<div align="center">

⚡ *Understanding the boot process is essential for kernel development* ⚡

</div>
