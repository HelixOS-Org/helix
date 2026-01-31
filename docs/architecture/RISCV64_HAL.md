# RISC-V 64-bit HAL Architecture - Industrial-Grade Design Document

## Executive Summary

This document defines the comprehensive architecture for Helix OS's RISC-V 64-bit (RV64GC) Hardware Abstraction Layer. The design targets **production-grade quality**, capable of supporting research platforms, embedded systems, servers, and development boards with a unified, extensible framework.

**Target Scope:**
- ~12,000+ lines of Rust code
- 6 major frameworks, 40+ modules
- Full SMP support (up to 256 harts)
- Sv39/Sv48/Sv57 page table formats
- CLINT + PLIC interrupt controllers
- M/S/U privilege levels with clean separation
- OpenSBI/UEFI boot support

---

## 1. Architecture Overview

### 1.1 High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        Helix OS - RISC-V 64 HAL Architecture                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   ┌────────────────────────────────────────────────────────────────────────┐    │
│   │                           Kernel Core                                   │    │
│   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │    │
│   │  │Scheduler│  │ Memory  │  │   IPC   │  │  VFS    │  │   Modules   │  │    │
│   │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └──────┬──────┘  │    │
│   └───────┼────────────┼────────────┼────────────┼───────────────┼────────┘    │
│           │            │            │            │               │              │
│   ┌───────┴────────────┴────────────┴────────────┴───────────────┴────────┐    │
│   │                        HAL Abstract Interface                          │    │
│   │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐              │    │
│   │  │   CPU    │  │   MMU    │  │Interrupts│  │ Firmware │              │    │
│   │  │ Trait    │  │  Trait   │  │  Trait   │  │  Trait   │              │    │
│   │  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘              │    │
│   └───────┼─────────────┼─────────────┼─────────────┼────────────────────┘    │
│           │             │             │             │                          │
│   ┌───────┴─────────────┴─────────────┴─────────────┴────────────────────┐    │
│   │                      RISC-V 64 HAL Implementation                     │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                        Core Framework                            │ │    │
│   │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐  │ │    │
│   │  │  │Registers│ │   CSR   │ │Features │ │  Cache  │ │ Barriers │  │ │    │
│   │  │  │ x0-x31  │ │sstatus │ │ MISA/ext│ │FENCE.I  │ │  FENCE   │  │ │    │
│   │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └──────────┘  │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                    Privilege Level Framework                     │ │    │
│   │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────────────┐│ │    │
│   │  │  │Machine │ │Superv. │ │  User  │ │ Traps  │ │ Vector Tables  ││ │    │
│   │  │  │ M-mode │ │ S-mode │ │ U-mode │ │Handler │ │  & Handlers    ││ │    │
│   │  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────────────┘│ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                       MMU Framework                              │ │    │
│   │  │  ┌──────────┐ ┌──────────┐ ┌───────┐ ┌───────┐ ┌────────────┐ │ │    │
│   │  │  │  Page    │ │  Page    │ │ ASID  │ │  TLB  │ │   SATP     │ │ │    │
│   │  │  │ Tables   │ │ Entries  │ │Manager│ │SFENCE │ │  Control   │ │ │    │
│   │  │  │Sv39/48/57│ │  PTE     │ │       │ │ .VMA  │ │            │ │ │    │
│   │  │  └──────────┘ └──────────┘ └───────┘ └───────┘ └────────────┘ │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                    Interrupt Framework                           │ │    │
│   │  │  ┌─────────┐ ┌─────────┐ ┌────────────┐ ┌───────────────────┐ │ │    │
│   │  │  │  CLINT  │ │  PLIC   │ │  Unified   │ │   IRQ Management  │ │ │    │
│   │  │  │ Timer + │ │External │ │    API     │ │   & Handlers      │ │ │    │
│   │  │  │   IPI   │ │Interrupts│ │           │ │                   │ │ │    │
│   │  │  └─────────┘ └─────────┘ └────────────┘ └───────────────────┘ │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                       SMP Framework                              │ │    │
│   │  │  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌────────┐ ┌───────────┐│ │    │
│   │  │  │Hart Info│ │ Startup │ │ Per-Hart │ │Barriers│ │    IPI    ││ │    │
│   │  │  │ hartid  │ │Secondary│ │   Data   │ │        │ │ via CLINT ││ │    │
│   │  │  └─────────┘ └─────────┘ └──────────┘ └────────┘ └───────────┘│ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                      Timer Framework                             │ │    │
│   │  │  ┌────────────┐ ┌─────────────┐ ┌──────────────────────────┐  │ │    │
│   │  │  │   MTIME    │ │  MTIMECMP   │ │    Supervisor Timer      │  │ │    │
│   │  │  │  Counter   │ │  Per-Hart   │ │   (stimecmp if avail)    │  │ │    │
│   │  │  └────────────┘ └─────────────┘ └──────────────────────────┘  │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   └───────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│   ┌────────────────────────────────────────────────────────────────────────┐    │
│   │                         Boot & Firmware                                 │    │
│   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │    │
│   │  │ OpenSBI │  │  UEFI   │  │   DTB   │  │   SBI   │  │ Direct Boot │  │    │
│   │  │ Payload │  │  RISC-V │  │ Parsing │  │  Calls  │  │  (M-mode)   │  │    │
│   │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────┘  │    │
│   └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│   ┌────────────────────────────────────────────────────────────────────────┐    │
│   │                              Hardware                                   │    │
│   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │    │
│   │  │  Hart   │  │   MMU   │  │  CLINT  │  │  PLIC   │  │  Platform   │  │    │
│   │  │  Cores  │  │ Sv39/48 │  │ Timer   │  │  IRQ    │  │   Devices   │  │    │
│   │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────┘  │    │
│   └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. RISC-V Privilege Levels

### 2.1 Privilege Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        RISC-V Privilege Levels                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Level │ Name       │ Encoding │ Purpose                                       │
│   ──────┼────────────┼──────────┼─────────────────────────────────────────────  │
│    3    │ Machine    │   11     │ Full hardware access, boot, firmware          │
│    1    │ Supervisor │   01     │ Kernel, MMU control, interrupts               │
│    0    │ User       │   00     │ Applications, unprivileged execution          │
│                                                                                  │
│   ┌────────────────────────────────────────────────────────────────────────┐    │
│   │                        Execution Flow                                   │    │
│   │                                                                         │    │
│   │   ┌─────────────────────────────────────────────────────────────────┐  │    │
│   │   │                     Machine Mode (M-mode)                        │  │    │
│   │   │                                                                  │  │    │
│   │   │  • First code to run after reset                                │  │    │
│   │   │  • Full access to all CSRs (mstatus, mtvec, mepc, etc.)        │  │    │
│   │   │  • Handles firmware (OpenSBI) / bootloader                      │  │    │
│   │   │  • Traps that can't be delegated land here                     │  │    │
│   │   │  • Controls Physical Memory Protection (PMP)                    │  │    │
│   │   │                                                                  │  │    │
│   │   │                          │ MRET                                  │  │    │
│   │   │                          ▼                                       │  │    │
│   │   └─────────────────────────────────────────────────────────────────┘  │    │
│   │                                                                         │    │
│   │   ┌─────────────────────────────────────────────────────────────────┐  │    │
│   │   │                   Supervisor Mode (S-mode)                       │  │    │
│   │   │                                                                  │  │    │
│   │   │  • Kernel execution                                             │  │    │
│   │   │  • MMU control (satp register)                                  │  │    │
│   │   │  • Interrupt handling (stvec, sie, sip)                        │  │    │
│   │   │  • Page fault handling                                          │  │    │
│   │   │  • Delegated traps from M-mode (via medeleg/mideleg)           │  │    │
│   │   │  • SBI calls to M-mode for privileged operations               │  │    │
│   │   │                                                                  │  │    │
│   │   │                          │ SRET                                  │  │    │
│   │   │                          ▼                                       │  │    │
│   │   └─────────────────────────────────────────────────────────────────┘  │    │
│   │                                                                         │    │
│   │   ┌─────────────────────────────────────────────────────────────────┐  │    │
│   │   │                      User Mode (U-mode)                          │  │    │
│   │   │                                                                  │  │    │
│   │   │  • Application execution                                        │  │    │
│   │   │  • No direct CSR access                                         │  │    │
│   │   │  • ECALL to request kernel services                            │  │    │
│   │   │  • Page-level protection via MMU                                │  │    │
│   │   │  • Exceptions/interrupts trap to S-mode                        │  │    │
│   │   │                                                                  │  │    │
│   │   └─────────────────────────────────────────────────────────────────┘  │    │
│   └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Trap & Exception Flow

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Trap Handling Flow                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Exception/Interrupt occurs                                                     │
│           │                                                                      │
│           ▼                                                                      │
│   ┌───────────────────────────────────────┐                                     │
│   │   Is it delegated? (medeleg/mideleg)  │                                     │
│   └───────────────┬───────────────────────┘                                     │
│                   │                                                              │
│          ┌────────┴────────┐                                                    │
│          │                 │                                                    │
│          ▼ NO              ▼ YES                                                │
│   ┌─────────────┐   ┌─────────────┐                                            │
│   │  M-mode     │   │  S-mode     │                                            │
│   │  Trap       │   │  Trap       │                                            │
│   └──────┬──────┘   └──────┬──────┘                                            │
│          │                 │                                                    │
│          ▼                 ▼                                                    │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                     Hardware Actions                                     │  │
│   │                                                                          │  │
│   │   1. xepc ← PC of faulting instruction (or next for interrupts)        │  │
│   │   2. xcause ← exception/interrupt cause code                            │  │
│   │   3. xtval ← exception-specific value (fault addr, instruction, etc.)   │  │
│   │   4. xstatus.xPP ← previous privilege mode                              │  │
│   │   5. xstatus.xPIE ← xstatus.xIE (save interrupt enable)                │  │
│   │   6. xstatus.xIE ← 0 (disable interrupts)                               │  │
│   │   7. PC ← xtvec (trap vector)                                           │  │
│   │                                                                          │  │
│   │   Where x = M (machine) or S (supervisor) depending on delegation       │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Exception Causes (scause/mcause):                                             │
│   ┌────────────────────────────────────────────────────────────────────────┐   │
│   │  Code │ Description                       │ Type        │ Interrupt?  │   │
│   │  ─────┼───────────────────────────────────┼─────────────┼─────────────│   │
│   │    0  │ Instruction address misaligned    │ Exception   │     No      │   │
│   │    1  │ Instruction access fault          │ Exception   │     No      │   │
│   │    2  │ Illegal instruction               │ Exception   │     No      │   │
│   │    3  │ Breakpoint                        │ Exception   │     No      │   │
│   │    4  │ Load address misaligned           │ Exception   │     No      │   │
│   │    5  │ Load access fault                 │ Exception   │     No      │   │
│   │    6  │ Store/AMO address misaligned      │ Exception   │     No      │   │
│   │    7  │ Store/AMO access fault            │ Exception   │     No      │   │
│   │    8  │ Environment call from U-mode      │ Exception   │     No      │   │
│   │    9  │ Environment call from S-mode      │ Exception   │     No      │   │
│   │   11  │ Environment call from M-mode      │ Exception   │     No      │   │
│   │   12  │ Instruction page fault            │ Exception   │     No      │   │
│   │   13  │ Load page fault                   │ Exception   │     No      │   │
│   │   15  │ Store/AMO page fault              │ Exception   │     No      │   │
│   │  ─────┼───────────────────────────────────┼─────────────┼─────────────│   │
│   │    1  │ Supervisor software interrupt     │ Interrupt   │    Yes      │   │
│   │    3  │ Machine software interrupt        │ Interrupt   │    Yes      │   │
│   │    5  │ Supervisor timer interrupt        │ Interrupt   │    Yes      │   │
│   │    7  │ Machine timer interrupt           │ Interrupt   │    Yes      │   │
│   │    9  │ Supervisor external interrupt     │ Interrupt   │    Yes      │   │
│   │   11  │ Machine external interrupt        │ Interrupt   │    Yes      │   │
│   └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Memory Management Unit (MMU)

### 3.1 Virtual Address Translation

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        RISC-V MMU Architecture                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   SATP Register (Supervisor Address Translation and Protection)                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  63    60 59          44 43                                          0  │  │
│   │ ┌────────┬──────────────┬────────────────────────────────────────────┐ │  │
│   │ │  MODE  │     ASID     │               PPN (Root Page Table)        │ │  │
│   │ └────────┴──────────────┴────────────────────────────────────────────┘ │  │
│   │    4 bits   16 bits                      44 bits                        │  │
│   │                                                                          │  │
│   │  MODE values:                                                            │  │
│   │    0 = Bare (no translation)                                            │  │
│   │    8 = Sv39 (39-bit virtual address, 3-level page table)                │  │
│   │    9 = Sv48 (48-bit virtual address, 4-level page table)                │  │
│   │   10 = Sv57 (57-bit virtual address, 5-level page table)                │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│                                                                                  │
│   Sv39 Virtual Address (39-bit)                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  63           39 38      30 29      21 20      12 11               0    │  │
│   │ ┌──────────────┬──────────┬──────────┬──────────┬────────────────────┐ │  │
│   │ │   Sign Ext   │  VPN[2]  │  VPN[1]  │  VPN[0]  │    Page Offset     │ │  │
│   │ └──────────────┴──────────┴──────────┴──────────┴────────────────────┘ │  │
│   │     25 bits       9 bits     9 bits     9 bits        12 bits          │  │
│   │                                                                          │  │
│   │  • 512 GB total addressable (signed, so ±256 GB)                        │  │
│   │  • 3-level page table (512 entries each level)                          │  │
│   │  • 4 KB pages (12-bit offset)                                           │  │
│   │  • 2 MB megapages (VPN[0] combined with offset)                         │  │
│   │  • 1 GB gigapages (VPN[1:0] combined with offset)                       │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│                                                                                  │
│   Sv48 Virtual Address (48-bit)                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  63    48 47     39 38     30 29     21 20     12 11                 0  │  │
│   │ ┌────────┬─────────┬─────────┬─────────┬─────────┬────────────────────┐│  │
│   │ │SignExt │ VPN[3]  │ VPN[2]  │ VPN[1]  │ VPN[0]  │    Page Offset     ││  │
│   │ └────────┴─────────┴─────────┴─────────┴─────────┴────────────────────┘│  │
│   │  16 bits   9 bits    9 bits    9 bits    9 bits       12 bits          │  │
│   │                                                                          │  │
│   │  • 256 TB total addressable (signed, so ±128 TB)                        │  │
│   │  • 4-level page table                                                   │  │
│   │  • Supports 512 GB terapages at level 3                                 │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Page Table Entry Format

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        Page Table Entry (PTE) Format                             │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   64-bit PTE:                                                                    │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │ 63  62  61  60 54 53    28 27    19 18    10 9  8 7 6 5 4 3 2 1 0      │  │
│   │ ┌───┬───┬───┬─────┬───────┬────────┬────────┬────┬─┬─┬─┬─┬─┬─┬─┬─┐     │  │
│   │ │ N │PBMT│RSV│RSV  │PPN[2] │ PPN[1] │ PPN[0] │RSW │D│A│G│U│X│W│R│V│     │  │
│   │ └───┴───┴───┴─────┴───────┴────────┴────────┴────┴─┴─┴─┴─┴─┴─┴─┴─┘     │  │
│   │  1   2   1   7      26        9        9      2   1 1 1 1 1 1 1 1      │  │
│   │                                                                          │  │
│   │  Bit │ Name │ Description                                               │  │
│   │  ────┼──────┼─────────────────────────────────────────────────────────  │  │
│   │   0  │  V   │ Valid - entry is valid                                    │  │
│   │   1  │  R   │ Read - page is readable                                   │  │
│   │   2  │  W   │ Write - page is writable                                  │  │
│   │   3  │  X   │ Execute - page is executable                              │  │
│   │   4  │  U   │ User - accessible from U-mode                             │  │
│   │   5  │  G   │ Global - mapping exists in all address spaces             │  │
│   │   6  │  A   │ Accessed - page has been accessed                         │  │
│   │   7  │  D   │ Dirty - page has been written                             │  │
│   │ 8-9  │ RSW  │ Reserved for Software                                     │  │
│   │10-53 │ PPN  │ Physical Page Number                                      │  │
│   │54-60 │ RSV  │ Reserved (must be 0)                                      │  │
│   │61-62 │ PBMT │ Page-Based Memory Types (Svpbmt extension)                │  │
│   │  63  │  N   │ NAPOT (Svnapot extension)                                 │  │
│   │                                                                          │  │
│   │  Special combinations of R/W/X:                                          │  │
│   │  ┌─────┬─────┬─────┬────────────────────────────────────────────────┐   │  │
│   │  │  R  │  W  │  X  │  Meaning                                       │   │  │
│   │  │─────┼─────┼─────┼────────────────────────────────────────────────│   │  │
│   │  │  0  │  0  │  0  │  Pointer to next level page table             │   │  │
│   │  │  0  │  0  │  1  │  Execute-only page                            │   │  │
│   │  │  0  │  1  │  0  │  Reserved (invalid)                           │   │  │
│   │  │  0  │  1  │  1  │  Read-write-execute page                      │   │  │
│   │  │  1  │  0  │  0  │  Read-only page                               │   │  │
│   │  │  1  │  0  │  1  │  Read-execute page                            │   │  │
│   │  │  1  │  1  │  0  │  Read-write page                              │   │  │
│   │  │  1  │  1  │  1  │  Read-write-execute page                      │   │  │
│   │  └─────┴─────┴─────┴────────────────────────────────────────────────┘   │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 3.3 Address Translation Walk

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      Sv39 Page Table Walk                                        │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Virtual Address                                                                │
│   ┌──────────────┬──────────┬──────────┬──────────┬────────────────────┐        │
│   │   Sign Ext   │  VPN[2]  │  VPN[1]  │  VPN[0]  │    Page Offset     │        │
│   └──────────────┴────┬─────┴────┬─────┴────┬─────┴────────────────────┘        │
│                       │          │          │                                    │
│                       │          │          │     ┌─────────────────────────┐   │
│   SATP.PPN ──────────►│          │          │     │   Physical Address      │   │
│        │              │          │          │     │                         │   │
│        ▼              │          │          │     │  ┌──────────┬────────┐  │   │
│   ┌──────────┐        │          │          │     │  │   PPN    │ Offset │  │   │
│   │ Root PT  │◄───────┘          │          │     │  └──────────┴────────┘  │   │
│   │ (Level 2)│                   │          │     └─────────────────────────┘   │
│   │          │                   │          │              ▲                    │
│   │ 512 PTEs │                   │          │              │                    │
│   └────┬─────┘                   │          │              │                    │
│        │ PTE[VPN[2]]             │          │              │                    │
│        │                         │          │              │                    │
│        ▼                         │          │              │                    │
│   ┌──────────────────────────────┴──────────┴──────────────┘                    │
│   │  Check PTE:                                                                  │
│   │  • V=0 → Page Fault                                                         │
│   │  • R=0, W=0, X=0 → It's a pointer, go to next level                        │
│   │  • R|W|X ≠ 0 → It's a leaf (superpage if not at level 0)                   │
│   └──────────────────────────────────────────────────────────────────────────┘  │
│        │                                                                         │
│        ▼ (if pointer)                                                           │
│   ┌──────────┐                                                                  │
│   │ Level 1  │◄─── VPN[1] selects entry                                         │
│   │ Page Tbl │                                                                  │
│   │          │                                                                  │
│   │ 512 PTEs │                                                                  │
│   └────┬─────┘                                                                  │
│        │ PTE[VPN[1]]                                                            │
│        │                                                                         │
│        ▼ (if pointer)                                                           │
│   ┌──────────┐                                                                  │
│   │ Level 0  │◄─── VPN[0] selects entry                                         │
│   │ Page Tbl │                                                                  │
│   │          │                                                                  │
│   │ 512 PTEs │                                                                  │
│   └────┬─────┘                                                                  │
│        │ PTE[VPN[0]] (must be leaf)                                             │
│        │                                                                         │
│        ▼                                                                         │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  Leaf PTE found:                                                         │  │
│   │  Physical Address = (PTE.PPN << 12) | VA.offset                          │  │
│   │                                                                          │  │
│   │  For superpages (leaf at level > 0):                                     │  │
│   │  - 2MB page (level 1): PA = (PTE.PPN[2:1] << 21) | VA[20:0]             │  │
│   │  - 1GB page (level 2): PA = (PTE.PPN[2] << 30) | VA[29:0]               │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Interrupt Architecture

### 4.1 CLINT (Core Local Interruptor)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              CLINT Architecture                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   CLINT provides per-hart timer and software interrupts                         │
│                                                                                  │
│   Memory Map (default base: 0x0200_0000):                                       │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  Offset       │ Width  │ Register                                       │  │
│   │  ─────────────┼────────┼───────────────────────────────────────────────│  │
│   │  0x0000       │ 4B     │ MSIP[0]     - Machine Software Interrupt hart0 │  │
│   │  0x0004       │ 4B     │ MSIP[1]     - Machine Software Interrupt hart1 │  │
│   │  ...          │ ...    │ ...                                            │  │
│   │  0x0000+4*N   │ 4B     │ MSIP[N]     - Machine Software Interrupt hartN │  │
│   │  ─────────────┼────────┼───────────────────────────────────────────────│  │
│   │  0x4000       │ 8B     │ MTIMECMP[0] - Timer compare hart 0             │  │
│   │  0x4008       │ 8B     │ MTIMECMP[1] - Timer compare hart 1             │  │
│   │  ...          │ ...    │ ...                                            │  │
│   │  0x4000+8*N   │ 8B     │ MTIMECMP[N] - Timer compare hart N             │  │
│   │  ─────────────┼────────┼───────────────────────────────────────────────│  │
│   │  0xBFF8       │ 8B     │ MTIME       - Timer register (global)          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Timer Interrupt Logic:                                                         │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │     MTIME (64-bit counter, always incrementing)                         │  │
│   │         │                                                                │  │
│   │         ▼                                                                │  │
│   │   ┌───────────────┐                                                      │  │
│   │   │  MTIME >=     │──── YES ───► Assert MTI (Machine Timer Interrupt)   │  │
│   │   │  MTIMECMP[N]? │                    │                                 │  │
│   │   └───────────────┘                    ▼                                 │  │
│   │                              If delegated via mideleg.STIP:              │  │
│   │                              ───► Assert STI (Supervisor Timer Int)      │  │
│   │                                                                          │  │
│   │   To clear: Write MTIMECMP[N] > MTIME                                   │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Software Interrupt Logic:                                                      │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │   MSIP[N] register (only bit 0 matters):                                 │  │
│   │   ┌─────────────────────────────────────────────────────────────────┐   │  │
│   │   │  31                                                           0 │   │  │
│   │   │ ┌───────────────────────────────────────────────────────────┬───┐   │  │
│   │   │ │                      Reserved                             │MSI│   │  │
│   │   │ └───────────────────────────────────────────────────────────┴───┘   │  │
│   │   └─────────────────────────────────────────────────────────────────┘   │  │
│   │                                                                          │  │
│   │   Write 1 to MSIP[N] ───► Assert MSI on hart N                          │  │
│   │   Write 0 to MSIP[N] ───► Clear MSI on hart N                           │  │
│   │                                                                          │  │
│   │   Used for Inter-Processor Interrupts (IPI)                             │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 4.2 PLIC (Platform-Level Interrupt Controller)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              PLIC Architecture                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   PLIC handles external interrupts and routes them to harts                     │
│                                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                          PLIC Block Diagram                              │  │
│   │                                                                          │  │
│   │   External Sources              Gateway            Core Interface        │  │
│   │   ┌───┐                        ┌───────┐          ┌─────────────┐       │  │
│   │   │ 1 │──────────────────────► │       │          │             │       │  │
│   │   └───┘                        │       │          │   Hart 0    │       │  │
│   │   ┌───┐                        │       │──────────│   M-mode    │       │  │
│   │   │ 2 │──────────────────────► │       │          │   context   │       │  │
│   │   └───┘                        │       │          │             │       │  │
│   │   ┌───┐                        │       │          ├─────────────┤       │  │
│   │   │ 3 │──────────────────────► │ PLIC  │          │             │       │  │
│   │   └───┘                        │       │──────────│   Hart 0    │       │  │
│   │    ...                         │Gateway│          │   S-mode    │       │  │
│   │   ┌───┐                        │   +   │          │   context   │       │  │
│   │   │N-1│──────────────────────► │Priority│         │             │       │  │
│   │   └───┘                        │   +   │          ├─────────────┤       │  │
│   │                                │Enable │──────────│   Hart 1    │       │  │
│   │   (up to 1023 sources)        │       │          │   M-mode    │       │  │
│   │                                │       │          │   ...       │       │  │
│   │                                └───────┘          └─────────────┘       │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Memory Map (default base: 0x0C00_0000):                                       │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  Offset         │ Size    │ Description                                 │  │
│   │  ───────────────┼─────────┼─────────────────────────────────────────── │  │
│   │  0x000000       │ 4B      │ Reserved (source 0 does not exist)         │  │
│   │  0x000004       │ 4B      │ Priority for source 1                      │  │
│   │  0x000008       │ 4B      │ Priority for source 2                      │  │
│   │  ...            │ ...     │ ...                                         │  │
│   │  0x000FFC       │ 4B      │ Priority for source 1023                   │  │
│   │  ───────────────┼─────────┼─────────────────────────────────────────── │  │
│   │  0x001000       │ 128B    │ Pending bits (1 bit per source)            │  │
│   │  ───────────────┼─────────┼─────────────────────────────────────────── │  │
│   │  0x002000       │ 128B    │ Enable bits for context 0                  │  │
│   │  0x002080       │ 128B    │ Enable bits for context 1                  │  │
│   │  ...            │ ...     │ (0x80 bytes per context)                   │  │
│   │  ───────────────┼─────────┼─────────────────────────────────────────── │  │
│   │  0x200000       │ 4B      │ Priority threshold for context 0           │  │
│   │  0x200004       │ 4B      │ Claim/Complete for context 0               │  │
│   │  0x201000       │ 4B      │ Priority threshold for context 1           │  │
│   │  0x201004       │ 4B      │ Claim/Complete for context 1               │  │
│   │  ...            │ ...     │ (0x1000 bytes per context)                 │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Interrupt Flow:                                                                │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │   1. External device asserts interrupt line N                           │  │
│   │                    │                                                     │  │
│   │                    ▼                                                     │  │
│   │   2. PLIC sets pending[N] = 1                                           │  │
│   │                    │                                                     │  │
│   │                    ▼                                                     │  │
│   │   3. For each context C where enable[C][N] = 1:                         │  │
│   │      If priority[N] > threshold[C]:                                     │  │
│   │          Assert external interrupt to hart                              │  │
│   │                    │                                                     │  │
│   │                    ▼                                                     │  │
│   │   4. Hart reads CLAIM register → returns highest priority pending       │  │
│   │      (atomically clears pending bit for that source)                    │  │
│   │                    │                                                     │  │
│   │                    ▼                                                     │  │
│   │   5. Hart services interrupt                                            │  │
│   │                    │                                                     │  │
│   │                    ▼                                                     │  │
│   │   6. Hart writes source ID to COMPLETE register                         │  │
│   │      (allows source to interrupt again)                                 │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 5. SMP Architecture

### 5.1 Hart Topology

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           RISC-V SMP Architecture                                │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                         Hart Organization                                │  │
│   │                                                                          │  │
│   │   ┌─────────┐  ┌─────────┐  ┌─────────┐       ┌─────────┐              │  │
│   │   │ Hart 0  │  │ Hart 1  │  │ Hart 2  │  ...  │ Hart N  │              │  │
│   │   │ (Boot)  │  │         │  │         │       │         │              │  │
│   │   ├─────────┤  ├─────────┤  ├─────────┤       ├─────────┤              │  │
│   │   │ mhartid │  │ mhartid │  │ mhartid │       │ mhartid │              │  │
│   │   │  = 0    │  │  = 1    │  │  = 2    │       │  = N    │              │  │
│   │   └────┬────┘  └────┬────┘  └────┬────┘       └────┬────┘              │  │
│   │        │            │            │                  │                   │  │
│   │        └────────────┴────────────┴──────────────────┘                   │  │
│   │                              │                                           │  │
│   │                              ▼                                           │  │
│   │   ┌──────────────────────────────────────────────────────────────────┐  │  │
│   │   │                    Shared Resources                               │  │  │
│   │   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐ │  │  │
│   │   │  │  CLINT  │  │  PLIC   │  │  MTIME  │  │    Memory (DRAM)    │ │  │  │
│   │   │  │  (IPI)  │  │  (IRQ)  │  │ Counter │  │                     │ │  │  │
│   │   │  └─────────┘  └─────────┘  └─────────┘  └─────────────────────┘ │  │  │
│   │   └──────────────────────────────────────────────────────────────────┘  │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Boot Sequence:                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │   1. All harts start in M-mode                                          │  │
│   │                │                                                         │  │
│   │                ▼                                                         │  │
│   │   2. OpenSBI/Firmware selects boot hart (usually hartid 0)              │  │
│   │                │                                                         │  │
│   │                ▼                                                         │  │
│   │   3. Boot hart jumps to kernel, others wait in WFI loop or spin         │  │
│   │                │                                                         │  │
│   │                ▼                                                         │  │
│   │   4. Boot hart initializes kernel, brings up secondary harts via:       │  │
│   │      - SBI HSM (Hart State Management) extension                        │  │
│   │      - Or direct CLINT IPI + spin table                                 │  │
│   │                │                                                         │  │
│   │                ▼                                                         │  │
│   │   5. Secondary harts initialize per-hart state and join scheduler       │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Per-Hart Data (via sscratch or tp register):                                  │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │   ┌──────────────────────────────────────────────────────────────────┐  │  │
│   │   │  PerHartData Structure (one per hart)                            │  │  │
│   │   │                                                                   │  │  │
│   │   │  struct PerHartData {                                            │  │  │
│   │   │      self_ptr: *mut PerHartData,   // For validation             │  │  │
│   │   │      hart_id: usize,               // Hardware hart ID           │  │  │
│   │   │      kernel_sp: usize,             // Kernel stack pointer       │  │  │
│   │   │      user_sp: usize,               // Saved user SP              │  │  │
│   │   │      current_task: *mut Task,      // Current running task       │  │  │
│   │   │      preempt_count: u32,           // Preemption disable depth   │  │  │
│   │   │      irq_count: u32,               // IRQ nesting depth          │  │  │
│   │   │      timer_pending: bool,          // Timer IRQ pending          │  │  │
│   │   │      // ... more fields ...                                      │  │  │
│   │   │  }                                                               │  │  │
│   │   │                                                                   │  │  │
│   │   │  Access via: TP register (thread pointer)                        │  │  │
│   │   │      - Set at boot: mv tp, a0  (where a0 = &per_hart_data)       │  │  │
│   │   │      - Access: ld t0, offset(tp)                                 │  │  │
│   │   │                                                                   │  │  │
│   │   └──────────────────────────────────────────────────────────────────┘  │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. SBI (Supervisor Binary Interface)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              SBI Interface                                       │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   SBI provides standardized interface between S-mode (kernel) and M-mode        │
│   (firmware like OpenSBI)                                                        │
│                                                                                  │
│   Call Convention:                                                               │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │   Arguments:    a0-a5 (function arguments)                              │  │
│   │                 a6 = FID (Function ID)                                   │  │
│   │                 a7 = EID (Extension ID)                                  │  │
│   │                                                                          │  │
│   │   Returns:      a0 = error code (0 = success)                           │  │
│   │                 a1 = return value                                        │  │
│   │                                                                          │  │
│   │   Invocation:   ECALL (from S-mode to M-mode)                           │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Key Extensions:                                                                │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  EID  │  Name  │  Description                                           │  │
│   │  ─────┼────────┼──────────────────────────────────────────────────────  │  │
│   │  0x00 │ Legacy │ Legacy console (deprecated)                            │  │
│   │  0x01 │ Timer  │ Set timer (sbi_set_timer)                              │  │
│   │  0x02 │ sPI    │ Send IPI to harts                                      │  │
│   │  0x03 │ RFENCE │ Remote fence operations                                │  │
│   │  0x04 │ HSM    │ Hart State Management (start/stop/suspend harts)       │  │
│   │  0x05 │ SRST   │ System Reset                                           │  │
│   │  0x10 │ BASE   │ Probe extensions, get SBI version                      │  │
│   │  0x48534D│ HSM │ Full Hart State Management                             │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
│   Common SBI Calls:                                                              │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                                                                          │  │
│   │   // Set timer for current hart                                         │  │
│   │   sbi_set_timer(stime_value: u64) → SbiRet                              │  │
│   │                                                                          │  │
│   │   // Send IPI to specified harts                                        │  │
│   │   sbi_send_ipi(hart_mask: usize, hart_mask_base: usize) → SbiRet        │  │
│   │                                                                          │  │
│   │   // Remote SFENCE.VMA                                                  │  │
│   │   sbi_remote_sfence_vma(hart_mask, start, size) → SbiRet                │  │
│   │                                                                          │  │
│   │   // Hart State Management                                              │  │
│   │   sbi_hart_start(hartid, start_addr, opaque) → SbiRet                   │  │
│   │   sbi_hart_stop() → SbiRet                                              │  │
│   │   sbi_hart_get_status(hartid) → SbiRet                                  │  │
│   │                                                                          │  │
│   │   // System reset                                                        │  │
│   │   sbi_system_reset(reset_type, reason) → SbiRet                         │  │
│   │                                                                          │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Directory Structure

```
hal/src/arch/riscv64/
├── mod.rs                     # Main module, HAL entry point
│
├── core/                      # Core CPU Framework
│   ├── mod.rs                 # Core module root
│   ├── registers.rs           # x0-x31 general purpose registers
│   ├── csr.rs                 # Control & Status Registers
│   ├── features.rs            # ISA extension detection (MISA, etc.)
│   ├── cache.rs               # Cache operations (FENCE.I)
│   └── barriers.rs            # Memory barriers (FENCE)
│
├── privilege/                 # Privilege Level Framework
│   ├── mod.rs                 # Privilege module root
│   ├── modes.rs               # M/S/U mode definitions
│   ├── traps.rs               # Trap handling (sync/async)
│   ├── vectors.rs             # Trap vector table
│   └── syscall.rs             # ECALL handling
│
├── mmu/                       # Memory Management Unit
│   ├── mod.rs                 # MMU module root
│   ├── entries.rs             # Page Table Entry format
│   ├── tables.rs              # Page table management
│   ├── tlb.rs                 # TLB operations (SFENCE.VMA)
│   ├── asid.rs                # ASID management
│   └── satp.rs                # SATP register control
│
├── interrupts/                # Interrupt Framework
│   ├── mod.rs                 # Interrupt module root
│   ├── clint.rs               # Core Local Interruptor
│   ├── plic.rs                # Platform-Level Interrupt Controller
│   └── irq.rs                 # IRQ management
│
├── smp/                       # SMP Framework
│   ├── mod.rs                 # SMP module root
│   ├── hartid.rs              # Hart ID handling
│   ├── percpu.rs              # Per-hart data
│   ├── startup.rs             # Secondary hart startup
│   └── ipi.rs                 # Inter-Processor Interrupts
│
├── timers/                    # Timer Framework
│   ├── mod.rs                 # Timer module root
│   ├── mtime.rs               # Machine timer (MTIME)
│   └── sstimer.rs             # Supervisor timer
│
└── sbi/                       # SBI Interface
    ├── mod.rs                 # SBI module root
    ├── base.rs                # Base extension
    ├── timer.rs               # Timer extension
    ├── ipi.rs                 # IPI extension
    ├── hsm.rs                 # Hart State Management
    └── rfence.rs              # Remote fence
```

---

## 8. Platform Support Matrix

| Platform          | Boot Method  | CLINT    | PLIC     | MMU      | Notes                    |
|-------------------|--------------|----------|----------|----------|--------------------------|
| QEMU virt         | OpenSBI      | 0x2000000| 0xC000000| Sv39/48  | Primary dev target       |
| SiFive HiFive     | U-Boot+SBI   | Standard | Standard | Sv39     | Real hardware            |
| StarFive VF2      | U-Boot+SBI   | Standard | Standard | Sv39/48  | StarFive JH7110          |
| Kendryte K210     | Direct       | 0x2000000| 0xC000000| Sv39     | Dual-core, no S-mode MMU |
| Milk-V Mars       | U-Boot+SBI   | Standard | Standard | Sv39/48  | JH7110 based             |
| Sipeed LicheeRV   | U-Boot+SBI   | Standard | Standard | Sv39     | Allwinner D1             |
| PolarFire SoC     | HSS+OpenSBI  | Standard | Standard | Sv39/48  | Microchip                |

---

## 9. Implementation Priorities

### Phase 1: Core Foundation
1. CSR framework with type-safe accessors
2. Trap/exception handling infrastructure
3. Basic CLINT timer support
4. S-mode initialization

### Phase 2: Memory Management
5. Sv39 page table implementation
6. SFENCE.VMA TLB management
7. ASID support
8. Kernel/user address space separation

### Phase 3: Interrupts
9. PLIC driver
10. IRQ registration and dispatch
11. Nested interrupt support

### Phase 4: SMP
12. Per-hart data structures
13. Secondary hart startup via SBI HSM
14. IPI mechanism
15. Synchronization primitives

### Phase 5: Integration
16. SBI wrapper library
17. Platform auto-detection
18. Integration with kernel core

---

## 10. Key Design Decisions

1. **S-mode Focus**: Kernel runs in S-mode, not M-mode. M-mode is left to OpenSBI.

2. **SBI Dependency**: Use SBI for:
   - Timer setup (until Sstc extension is common)
   - IPI sending (until AIA is common)
   - Hart management
   - System reset

3. **Sv39 Default**: Start with Sv39 (3-level, 39-bit VA), add Sv48 as optional.

4. **Per-Hart via TP**: Use `tp` register for per-hart data (standard RISC-V convention).

5. **PLIC Contexts**: Map hart N M-mode to context 2N, S-mode to context 2N+1.

6. **No Sv32**: Focus on RV64 only, no RV32 support planned.
