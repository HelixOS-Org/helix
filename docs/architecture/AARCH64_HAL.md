# AArch64 HAL Architecture - Industrial-Grade Design Document

## Executive Summary

This document defines the comprehensive architecture for Helix OS's AArch64 (ARM64) Hardware Abstraction Layer. The design targets **production-grade quality**, capable of supporting enterprise servers, desktop systems, embedded devices, and SBCs (Single Board Computers) with a unified, extensible framework.

**Target Scope:**
- ~15,000+ lines of Rust code
- 8 major frameworks, 50+ modules
- Full SMP support (up to 256 cores)
- GICv2/GICv3 interrupt controllers
- 4KB/16KB/64KB page support
- Exception Levels EL0-EL3
- PSCI for power management
- Virtualization-ready (EL2 support)

---

## 1. Architecture Overview

### 1.1 High-Level Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         Helix OS - AArch64 HAL Architecture                      │
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
│   │                      AArch64 HAL Implementation                       │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                        Core Framework                            │ │    │
│   │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌──────────┐  │ │    │
│   │  │  │Registers│ │Sys Regs │ │Features │ │  Cache  │ │   FPU    │  │ │    │
│   │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └──────────┘  │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                    Exception Level Framework                     │ │    │
│   │  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────────────┐│ │    │
│   │  │  │  EL0   │ │  EL1   │ │  EL2   │ │  EL3   │ │ Vector Tables  ││ │    │
│   │  │  │ User   │ │ Kernel │ │Hyperv. │ │Secure  │ │   & Handlers   ││ │    │
│   │  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────────────┘│ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                       MMU Framework                              │ │    │
│   │  │  ┌──────────┐ ┌──────────┐ ┌───────┐ ┌───────┐ ┌────────────┐ │ │    │
│   │  │  │Trans.    │ │  Page    │ │ ASID  │ │  TLB  │ │   Walker   │ │ │    │
│   │  │  │Tables    │ │ Entries  │ │Manager│ │Control│ │            │ │ │    │
│   │  │  └──────────┘ └──────────┘ └───────┘ └───────┘ └────────────┘ │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                       GIC Framework                              │ │    │
│   │  │  ┌─────────┐ ┌─────────┐ ┌────────────┐ ┌───────┐ ┌──────────┐│ │    │
│   │  │  │ GICv2   │ │ GICv3   │ │Redistribut.│ │  ICC  │ │ Routing  ││ │    │
│   │  │  │Distrib. │ │Distrib. │ │            │ │ Regs  │ │          ││ │    │
│   │  │  └─────────┘ └─────────┘ └────────────┘ └───────┘ └──────────┘│ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                       SMP Framework                              │ │    │
│   │  │  ┌─────────┐ ┌─────────┐ ┌──────────┐ ┌────────┐ ┌───────────┐│ │    │
│   │  │  │CPU Info │ │ Startup │ │ Per-CPU  │ │Barriers│ │   PSCI    ││ │    │
│   │  │  │Topology │ │  (PSCI) │ │   Data   │ │        │ │Power Mgmt ││ │    │
│   │  │  └─────────┘ └─────────┘ └──────────┘ └────────┘ └───────────┘│ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   │  ┌─────────────────────────────────────────────────────────────────┐ │    │
│   │  │                      Timer Framework                             │ │    │
│   │  │  ┌────────────┐ ┌─────────────┐ ┌──────────┐ ┌───────────────┐ │ │    │
│   │  │  │  Generic   │ │   System    │ │ Watchdog │ │   Per-CPU     │ │ │    │
│   │  │  │   Timer    │ │  Counter    │ │          │ │   Timers      │ │ │    │
│   │  │  └────────────┘ └─────────────┘ └──────────┘ └───────────────┘ │ │    │
│   │  └─────────────────────────────────────────────────────────────────┘ │    │
│   │                                                                       │    │
│   └───────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│   ┌────────────────────────────────────────────────────────────────────────┐    │
│   │                              Hardware                                   │    │
│   │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │    │
│   │  │  CPU    │  │  MMU    │  │   GIC   │  │ Timers  │  │  Platform   │  │    │
│   │  │ Cores   │  │  Unit   │  │ v2/v3   │  │         │  │   Devices   │  │    │
│   │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────┘  │    │
│   └────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Design Principles

1. **Framework-First**: Not a minimal implementation - full production capability
2. **Multi-Platform**: QEMU, Raspberry Pi, ARM servers, embedded SoCs
3. **SMP from Day 1**: Every component designed for multi-core
4. **Virtualization-Ready**: EL2 support for future hypervisor
5. **Security-Conscious**: TrustZone-aware, proper privilege separation
6. **Zero Hardcoded Addresses**: All addresses from DTB/ACPI
7. **Extensible**: New SoCs can be added without core changes

---

## 2. Exception Level Architecture

### 2.1 ARM Exception Levels

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      ARM Exception Levels                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Privilege        Exception Level         Helix OS Usage                 │
│  ─────────        ───────────────         ──────────────                 │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  EL3 - Secure Monitor                                           │    │
│  │  ┌─────────────────────────────────────────────────────────┐   │    │
│  │  │ • Highest privilege level                                │   │    │
│  │  │ • Secure/Non-secure world switching                      │   │    │
│  │  │ • Usually firmware (ARM Trusted Firmware)                │   │    │
│  │  │ • Helix: Not used directly (firmware provided)           │   │    │
│  │  └─────────────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                              │                                           │
│                              ▼ SMC (Secure Monitor Call)                 │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  EL2 - Hypervisor                                               │    │
│  │  ┌─────────────────────────────────────────────────────────┐   │    │
│  │  │ • Virtualization support                                 │   │    │
│  │  │ • Stage 2 translation control                            │   │    │
│  │  │ • Virtual interrupt handling                             │   │    │
│  │  │ • Helix: Optional hypervisor mode (future)               │   │    │
│  │  └─────────────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                              │                                           │
│                              ▼ HVC (Hypervisor Call)                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  EL1 - Kernel (Privileged OS)                                   │    │
│  │  ┌─────────────────────────────────────────────────────────┐   │    │
│  │  │ • Full hardware access                                   │   │    │
│  │  │ • MMU configuration                                      │   │    │
│  │  │ • Interrupt handling                                     │   │    │
│  │  │ • Helix: PRIMARY KERNEL EXECUTION LEVEL                  │   │    │
│  │  └─────────────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                              │                                           │
│                              ▼ SVC (Supervisor Call)                     │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  EL0 - User (Unprivileged)                                      │    │
│  │  ┌─────────────────────────────────────────────────────────┐   │    │
│  │  │ • Application execution                                  │   │    │
│  │  │ • No direct hardware access                              │   │    │
│  │  │ • Memory isolation via MMU                               │   │    │
│  │  │ • Helix: USER PROCESSES                                  │   │    │
│  │  └─────────────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Exception Vector Table

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Exception Vector Table Layout                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  VBAR_EL1 + Offset    │ Exception Type     │ Source                     │
│  ────────────────────────────────────────────────────────────────────   │
│                                                                          │
│  ┌─── Current EL with SP0 (SP_EL0) ───────────────────────────────┐    │
│  │  0x000  │  Synchronous         │  From EL1, using SP_EL0       │    │
│  │  0x080  │  IRQ                 │  From EL1, using SP_EL0       │    │
│  │  0x100  │  FIQ                 │  From EL1, using SP_EL0       │    │
│  │  0x180  │  SError              │  From EL1, using SP_EL0       │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─── Current EL with SPx (SP_EL1) ───────────────────────────────┐    │
│  │  0x200  │  Synchronous         │  From EL1, using SP_EL1       │    │
│  │  0x280  │  IRQ                 │  From EL1, using SP_EL1       │    │
│  │  0x300  │  FIQ                 │  From EL1, using SP_EL1       │    │
│  │  0x380  │  SError              │  From EL1, using SP_EL1       │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─── Lower EL using AArch64 ─────────────────────────────────────┐    │
│  │  0x400  │  Synchronous         │  From EL0 (64-bit)            │    │
│  │  0x480  │  IRQ                 │  From EL0 (64-bit)            │    │
│  │  0x500  │  FIQ                 │  From EL0 (64-bit)            │    │
│  │  0x580  │  SError              │  From EL0 (64-bit)            │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─── Lower EL using AArch32 ─────────────────────────────────────┐    │
│  │  0x600  │  Synchronous         │  From EL0 (32-bit)            │    │
│  │  0x680  │  IRQ                 │  From EL0 (32-bit)            │    │
│  │  0x700  │  FIQ                 │  From EL0 (32-bit)            │    │
│  │  0x780  │  SError              │  From EL0 (32-bit)            │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Total Size: 0x800 (2048 bytes)                                         │
│  Each entry: 0x80 (128 bytes = 32 instructions max)                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.3 Exception Syndrome Register (ESR_EL1)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ESR_EL1 Structure                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   63      32 31    26 25 24                                    0        │
│  ┌─────────┬────────┬──┬────────────────────────────────────────┐       │
│  │   RES0  │   EC   │IL│                  ISS                   │       │
│  └─────────┴────────┴──┴────────────────────────────────────────┘       │
│                                                                          │
│  EC (Exception Class) - bits [31:26]:                                   │
│  ──────────────────────────────────                                     │
│  0b000000  Unknown reason                                               │
│  0b000001  Trapped WFI/WFE                                              │
│  0b000011  Trapped MCR/MRC (CP15)                                       │
│  0b000100  Trapped MCRR/MRRC (CP15)                                     │
│  0b000101  Trapped MCR/MRC (CP14)                                       │
│  0b000110  Trapped LDC/STC (CP14)                                       │
│  0b000111  Access to SVE/SIMD/FP                                        │
│  0b001100  Trapped MRRC (CP14)                                          │
│  0b001110  Illegal Execution state                                      │
│  0b010001  SVC in AArch32                                               │
│  0b010101  SVC in AArch64  ◄── System calls                             │
│  0b010110  HVC in AArch64                                               │
│  0b010111  SMC in AArch64                                               │
│  0b011000  MSR/MRS (64-bit)                                             │
│  0b011001  SVE access                                                   │
│  0b100000  Instruction Abort (lower EL)                                 │
│  0b100001  Instruction Abort (same EL)                                  │
│  0b100010  PC alignment fault                                           │
│  0b100100  Data Abort (lower EL)  ◄── Page faults (user)                │
│  0b100101  Data Abort (same EL)   ◄── Page faults (kernel)              │
│  0b100110  SP alignment fault                                           │
│  0b101100  Floating-point exception                                     │
│  0b101111  SError                                                       │
│  0b110000  Breakpoint (lower EL)                                        │
│  0b110001  Breakpoint (same EL)                                         │
│  0b110010  Software Step (lower EL)                                     │
│  0b110011  Software Step (same EL)                                      │
│  0b110100  Watchpoint (lower EL)                                        │
│  0b110101  Watchpoint (same EL)                                         │
│  0b111000  BKPT in AArch32                                              │
│  0b111100  BRK in AArch64                                               │
│                                                                          │
│  IL (Instruction Length) - bit [25]:                                    │
│  ─────────────────────────────────                                      │
│  0 = 16-bit instruction                                                 │
│  1 = 32-bit instruction                                                 │
│                                                                          │
│  ISS (Instruction Specific Syndrome) - bits [24:0]:                     │
│  ─────────────────────────────────────────────────                      │
│  Contains exception-specific information                                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Memory Management Unit (MMU)

### 3.1 Translation Table Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                   ARM64 Translation Table Levels                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │                  4KB Granule (Most Common)                      │    │
│  ├────────────────────────────────────────────────────────────────┤    │
│  │                                                                 │    │
│  │  Virtual Address (48-bit, 4-level):                            │    │
│  │                                                                 │    │
│  │   63    48 47    39 38    30 29    21 20    12 11         0    │    │
│  │  ┌───────┬────────┬────────┬────────┬────────┬────────────┐   │    │
│  │  │ TTBR  │  L0    │   L1   │   L2   │   L3   │   Offset   │   │    │
│  │  │Select │ Index  │  Index │  Index │  Index │  (4KB)     │   │    │
│  │  └───────┴────────┴────────┴────────┴────────┴────────────┘   │    │
│  │     1      9 bits   9 bits   9 bits   9 bits    12 bits       │    │
│  │                                                                 │    │
│  │  L0: 512GB regions (512 entries)                               │    │
│  │  L1: 1GB regions   (512 entries) → Can be 1GB block            │    │
│  │  L2: 2MB regions   (512 entries) → Can be 2MB block            │    │
│  │  L3: 4KB pages     (512 entries) → Final page                  │    │
│  │                                                                 │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │                  64KB Granule (Large Pages)                     │    │
│  ├────────────────────────────────────────────────────────────────┤    │
│  │                                                                 │    │
│  │  Virtual Address (48-bit, 3-level):                            │    │
│  │                                                                 │    │
│  │   63    48 47    42 41    29 28    16 15              0        │    │
│  │  ┌───────┬────────┬────────┬────────┬─────────────────┐       │    │
│  │  │ TTBR  │   L1   │   L2   │   L3   │     Offset      │       │    │
│  │  │Select │  Index │  Index │  Index │     (64KB)      │       │    │
│  │  └───────┴────────┴────────┴────────┴─────────────────┘       │    │
│  │     1      6 bits  13 bits  13 bits     16 bits               │    │
│  │                                                                 │    │
│  │  L1: 4TB regions   (64 entries)  → Can be 4TB block (rare)     │    │
│  │  L2: 512MB regions (8192 entries) → Can be 512MB block         │    │
│  │  L3: 64KB pages    (8192 entries) → Final page                 │    │
│  │                                                                 │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │                  16KB Granule (iOS/macOS)                       │    │
│  ├────────────────────────────────────────────────────────────────┤    │
│  │                                                                 │    │
│  │  Virtual Address (47-bit, 4-level):                            │    │
│  │                                                                 │    │
│  │   63    47 46    36 35    25 24    14 13            0          │    │
│  │  ┌───────┬────────┬────────┬────────┬───────────────┐         │    │
│  │  │ TTBR  │   L0   │   L1   │   L2   │    Offset     │         │    │
│  │  │Select │  Index │  Index │  Index │    (16KB)     │         │    │
│  │  └───────┴────────┴────────┴────────┴───────────────┘         │    │
│  │     1     11 bits  11 bits  11 bits    14 bits                │    │
│  │                                                                 │    │
│  │  L0: 128TB regions (2048 entries)                              │    │
│  │  L1: 64GB regions  (2048 entries) → Can be 64GB block          │    │
│  │  L2: 32MB regions  (2048 entries) → Can be 32MB block          │    │
│  │  L3: 16KB pages    (2048 entries) → Final page                 │    │
│  │                                                                 │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Page Table Entry Format

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Page Table Entry Formats                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─── Table Descriptor (L0-L2) ───────────────────────────────────┐    │
│  │                                                                 │    │
│  │   63  62 61 60 59 58       52 51 50 49 48 47       12 11  2 1 0│    │
│  │  ┌──┬──┬──┬──┬──┬───────────┬──┬──┬──┬──┬───────────┬─────┬───┐│    │
│  │  │NS│AP│XN│PX│IG│   Rsvd    │  │  │  │  │   Addr    │ Ign │ TT││    │
│  │  │  │  │  │N │  │           │  │  │  │  │ [47:12]   │     │   ││    │
│  │  └──┴──┴──┴──┴──┴───────────┴──┴──┴──┴──┴───────────┴─────┴───┘│    │
│  │                                                         │       │    │
│  │  bits [1:0] = 0b11 → Table descriptor                 ──┘       │    │
│  │                                                                 │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─── Block Descriptor (L1: 1GB, L2: 2MB) ────────────────────────┐    │
│  │                                                                 │    │
│  │   63 62 55 54 53 52 51 50 49 48 47       30/21 ... 12 11 10 ... 0│   │
│  │  ┌──┬─────┬──┬──┬──┬──┬──┬──┬──┬───────────────┬─────┬──┬─────┐│    │
│  │  │  │ Ign │PX│Cn│  │GP│DB│  │  │  Output Addr  │ Rsvd│AF│ Attr││    │
│  │  │  │     │N │tg│  │  │M │  │  │               │     │  │     ││    │
│  │  └──┴─────┴──┴──┴──┴──┴──┴──┴──┴───────────────┴─────┴──┴─────┘│    │
│  │                                                         │       │    │
│  │  bits [1:0] = 0b01 → Block descriptor                 ──┘       │    │
│  │                                                                 │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  ┌─── Page Descriptor (L3) ───────────────────────────────────────┐    │
│  │                                                                 │    │
│  │   63 62 55 54 53 52 51 50 49 48 47         12 11 10 ... 2 1 0 │    │
│  │  ┌──┬─────┬──┬──┬──┬──┬──┬──┬──┬─────────────┬──┬───────┬───┐ │    │
│  │  │  │ Ign │PX│Cn│  │GP│DB│  │  │ Output Addr │AF│ Attr  │ TT│ │    │
│  │  │  │     │N │tg│  │  │M │  │  │   [47:12]   │  │       │   │ │    │
│  │  └──┴─────┴──┴──┴──┴──┴──┴──┴──┴─────────────┴──┴───────┴───┘ │    │
│  │                                                         │       │    │
│  │  bits [1:0] = 0b11 → Page descriptor                  ──┘       │    │
│  │                                                                 │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  Attribute Bits:                                                        │
│  ──────────────                                                         │
│  [54] PXN     - Privileged Execute Never                                │
│  [53] Contiguous - Part of contiguous set (TLB optimization)            │
│  [52] GP      - Guarded Page (BTI)                                      │
│  [51] DBM     - Dirty Bit Modifier                                      │
│  [10] AF      - Access Flag (must be set for valid entry)               │
│  [9:8] SH     - Shareability (00=Non, 10=Outer, 11=Inner)               │
│  [7:6] AP     - Access Permissions                                      │
│                 00 = EL1 R/W, EL0 None                                  │
│                 01 = EL1 R/W, EL0 R/W                                   │
│                 10 = EL1 R/O, EL0 None                                  │
│                 11 = EL1 R/O, EL0 R/O                                   │
│  [5] NS       - Non-Secure (for Secure state)                           │
│  [4:2] AttrIndx - MAIR index (memory type)                              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.3 TTBR0/TTBR1 Split

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Virtual Address Space Split                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  48-bit Virtual Address Space:                                          │
│                                                                          │
│  0xFFFF_FFFF_FFFF_FFFF ┌─────────────────────────────────────────────┐ │
│                        │                                              │ │
│                        │              Kernel Space                    │ │
│                        │              (TTBR1_EL1)                     │ │
│                        │                                              │ │
│                        │  • Kernel code and data                      │ │
│                        │  • Kernel heap                               │ │
│                        │  • Device mappings                           │ │
│                        │  • Per-CPU data                              │ │
│                        │                                              │ │
│  0xFFFF_0000_0000_0000 ├─────────────────────────────────────────────┤ │
│                        │                                              │ │
│                        │           Canonical Hole                     │ │
│                        │           (Invalid)                          │ │
│                        │                                              │ │
│  0x0000_FFFF_FFFF_FFFF ├─────────────────────────────────────────────┤ │
│                        │                                              │ │
│                        │              User Space                      │ │
│                        │              (TTBR0_EL1)                     │ │
│                        │                                              │ │
│                        │  • User code and data                        │ │
│                        │  • User heap and stack                       │ │
│                        │  • Shared libraries                          │ │
│                        │  • mmap regions                              │ │
│                        │                                              │ │
│  0x0000_0000_0000_0000 └─────────────────────────────────────────────┘ │
│                                                                          │
│  TCR_EL1.T0SZ controls TTBR0 size (user)                                │
│  TCR_EL1.T1SZ controls TTBR1 size (kernel)                              │
│                                                                          │
│  Common configurations:                                                  │
│  ─────────────────────                                                  │
│  T0SZ = T1SZ = 16 → 48-bit addresses (256TB each)                       │
│  T0SZ = T1SZ = 25 → 39-bit addresses (512GB each)                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Generic Interrupt Controller (GIC)

### 4.1 GIC Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       GIC Architecture                                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                         GIC Distributor                          │   │
│  │                         (GICD_*)                                 │   │
│  │                                                                  │   │
│  │  • Global interrupt configuration                                │   │
│  │  • Interrupt enable/disable                                      │   │
│  │  • Priority configuration                                        │   │
│  │  • Target CPU routing                                            │   │
│  │  • Interrupt pending/active status                               │   │
│  │                                                                  │   │
│  │  Interrupt Types:                                                │   │
│  │  ┌─────────────────────────────────────────────────────────┐   │   │
│  │  │ SGI: 0-15    │ Software Generated (IPI)                 │   │   │
│  │  │ PPI: 16-31   │ Private Peripheral (per-CPU)             │   │   │
│  │  │ SPI: 32-1019 │ Shared Peripheral (global)               │   │   │
│  │  │ LPI: 8192+   │ Locality-specific (GICv3 only)           │   │   │
│  │  └─────────────────────────────────────────────────────────┘   │   │
│  └──────────────────────────────┬──────────────────────────────────┘   │
│                                 │                                       │
│                                 ▼                                       │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    GIC Redistributors (GICv3)                     │  │
│  │                         (GICR_*)                                  │  │
│  │                                                                   │  │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │  │
│  │   │   GICR 0    │  │   GICR 1    │  │   GICR n    │   ...       │  │
│  │   │   (CPU 0)   │  │   (CPU 1)   │  │   (CPU n)   │             │  │
│  │   │             │  │             │  │             │             │  │
│  │   │  • PPI/SGI  │  │  • PPI/SGI  │  │  • PPI/SGI  │             │  │
│  │   │    config   │  │    config   │  │    config   │             │  │
│  │   │  • LPI      │  │  • LPI      │  │  • LPI      │             │  │
│  │   │    pending  │  │    pending  │  │    pending  │             │  │
│  │   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │  │
│  └──────────┼────────────────┼────────────────┼─────────────────────┘  │
│             │                │                │                         │
│             ▼                ▼                ▼                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                    CPU Interfaces                                 │  │
│  │                                                                   │  │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │  │
│  │   │   GICv2:    │  │   GICv2:    │  │   GICv2:    │             │  │
│  │   │  GICC_* or  │  │  GICC_* or  │  │  GICC_* or  │             │  │
│  │   │             │  │             │  │             │             │  │
│  │   │   GICv3:    │  │   GICv3:    │  │   GICv3:    │             │  │
│  │   │  ICC_* SRs  │  │  ICC_* SRs  │  │  ICC_* SRs  │             │  │
│  │   └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │  │
│  └──────────┼────────────────┼────────────────┼─────────────────────┘  │
│             │                │                │                         │
│             ▼                ▼                ▼                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                         CPU Cores                                 │  │
│  │                                                                   │  │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │  │
│  │   │   Core 0    │  │   Core 1    │  │   Core n    │   ...       │  │
│  │   │             │  │             │  │             │             │  │
│  │   │  IRQ/FIQ    │  │  IRQ/FIQ    │  │  IRQ/FIQ    │             │  │
│  │   │  Signals    │  │  Signals    │  │  Signals    │             │  │
│  │   └─────────────┘  └─────────────┘  └─────────────┘             │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.2 GIC Register Maps

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     GIC Register Summary                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  GIC Distributor (GICD_*) - Base from DTB/ACPI                          │
│  ─────────────────────────────────────────────                          │
│  Offset    │ Register        │ Description                              │
│  ──────────┼─────────────────┼──────────────────────────────────────    │
│  0x0000    │ GICD_CTLR       │ Distributor Control                      │
│  0x0004    │ GICD_TYPER      │ Interrupt Controller Type                │
│  0x0008    │ GICD_IIDR       │ Distributor Implementer ID               │
│  0x0080    │ GICD_IGROUPR    │ Interrupt Group (n*32 bits)              │
│  0x0100    │ GICD_ISENABLER  │ Set-Enable (n*32 bits)                   │
│  0x0180    │ GICD_ICENABLER  │ Clear-Enable (n*32 bits)                 │
│  0x0200    │ GICD_ISPENDR    │ Set-Pending (n*32 bits)                  │
│  0x0280    │ GICD_ICPENDR    │ Clear-Pending (n*32 bits)                │
│  0x0300    │ GICD_ISACTIVER  │ Set-Active (n*32 bits)                   │
│  0x0380    │ GICD_ICACTIVER  │ Clear-Active (n*32 bits)                 │
│  0x0400    │ GICD_IPRIORITYR │ Priority (n*8 bits)                      │
│  0x0800    │ GICD_ITARGETSR  │ Target CPU (n*8 bits, GICv2)             │
│  0x0C00    │ GICD_ICFGR      │ Configuration (n*2 bits)                 │
│  0x0F00    │ GICD_SGIR       │ Software Generated Int (GICv2)           │
│  0x6000    │ GICD_IROUTER    │ Affinity Routing (GICv3, 64-bit)         │
│                                                                          │
│  GICv3 Redistributor (GICR_*) - Per CPU                                 │
│  ──────────────────────────────────────                                 │
│  Offset    │ Register        │ Description                              │
│  ──────────┼─────────────────┼──────────────────────────────────────    │
│  0x0000    │ GICR_CTLR       │ Redistributor Control                    │
│  0x0004    │ GICR_IIDR       │ Implementer ID                           │
│  0x0008    │ GICR_TYPER      │ Redistributor Type                       │
│  0x0014    │ GICR_WAKER      │ Wake Request                             │
│  0x10000   │ GICR_IGROUPR0   │ SGI/PPI Group                            │
│  0x10100   │ GICR_ISENABLER0 │ SGI/PPI Set-Enable                       │
│  0x10180   │ GICR_ICENABLER0 │ SGI/PPI Clear-Enable                     │
│  0x10400   │ GICR_IPRIORITYR │ SGI/PPI Priority                         │
│  0x10C00   │ GICR_ICFGR0/1   │ SGI/PPI Configuration                    │
│                                                                          │
│  GICv3 CPU Interface (ICC_* System Registers)                           │
│  ────────────────────────────────────────────                           │
│  Register      │ Encoding              │ Description                    │
│  ──────────────┼───────────────────────┼────────────────────────────    │
│  ICC_PMR_EL1   │ S3_0_C4_C6_0          │ Priority Mask                  │
│  ICC_IAR1_EL1  │ S3_0_C12_C12_0        │ Interrupt Acknowledge (Grp1)   │
│  ICC_EOIR1_EL1 │ S3_0_C12_C12_1        │ End of Interrupt (Grp1)        │
│  ICC_BPR1_EL1  │ S3_0_C12_C12_3        │ Binary Point (Grp1)            │
│  ICC_CTLR_EL1  │ S3_0_C12_C12_4        │ Control                        │
│  ICC_SRE_EL1   │ S3_0_C12_C12_5        │ System Register Enable         │
│  ICC_IGRPEN1_EL1│ S3_0_C12_C12_7       │ Group 1 Enable                 │
│  ICC_SGI1R_EL1 │ S3_0_C12_C11_5        │ SGI Generation (Grp1)          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 5. SMP and Power Management

### 5.1 PSCI (Power State Coordination Interface)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       PSCI Interface                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  PSCI provides a standard interface for power management on ARM:        │
│                                                                          │
│  ┌────────────────────────────────────────────────────────────────┐    │
│  │                     PSCI Functions                              │    │
│  ├────────────────────────────────────────────────────────────────┤    │
│  │                                                                 │    │
│  │  Function ID (SMC/HVC)        │ Description                    │    │
│  │  ─────────────────────────────┼─────────────────────────────── │    │
│  │  0x8400_0000 PSCI_VERSION     │ Get PSCI version               │    │
│  │  0x8400_0001 CPU_SUSPEND      │ Suspend current CPU            │    │
│  │  0x8400_0002 CPU_OFF          │ Power off current CPU          │    │
│  │  0xC400_0003 CPU_ON           │ Power on a CPU (64-bit)        │    │
│  │  0x8400_0004 AFFINITY_INFO    │ Get CPU affinity state         │    │
│  │  0x8400_0005 MIGRATE          │ Migrate trusted OS             │    │
│  │  0x8400_0006 MIGRATE_INFO_TYPE│ Migrate info type              │    │
│  │  0x8400_0007 MIGRATE_INFO_UP  │ Migrate info UP CPU            │    │
│  │  0x8400_0008 SYSTEM_OFF       │ System power off               │    │
│  │  0x8400_0009 SYSTEM_RESET     │ System reset                   │    │
│  │  0x8400_000A PSCI_FEATURES    │ Query PSCI features            │    │
│  │  0x8400_000B CPU_FREEZE       │ Freeze current CPU             │    │
│  │  0x8400_000C CPU_DEFAULT_SUSP │ Default suspend                │    │
│  │  0x8400_000D NODE_HW_STATE    │ Hardware state of node         │    │
│  │  0x8400_000E SYSTEM_SUSPEND   │ System suspend                 │    │
│  │  0x8400_0010 STAT_RESIDENCY   │ Power state residency          │    │
│  │  0x8400_0011 STAT_COUNT       │ Power state count              │    │
│  │  0xC400_0012 SYSTEM_RESET2    │ System reset with type         │    │
│  │                                                                 │    │
│  └────────────────────────────────────────────────────────────────┘    │
│                                                                          │
│  CPU_ON Sequence for Secondary CPUs:                                    │
│  ──────────────────────────────────                                     │
│                                                                          │
│  ┌─────────────┐         ┌─────────────┐         ┌─────────────┐       │
│  │   Primary   │         │  Firmware   │         │  Secondary  │       │
│  │    CPU      │         │  (EL3/EL2)  │         │    CPU      │       │
│  └──────┬──────┘         └──────┬──────┘         └──────┬──────┘       │
│         │                       │                       │               │
│         │ SMC CPU_ON(mpidr,     │                       │               │
│         │     entry, ctx)       │                       │               │
│         ├──────────────────────►│                       │               │
│         │                       │ Power on CPU          │               │
│         │                       ├──────────────────────►│               │
│         │                       │                       │               │
│         │                       │                       │ Jump to entry │
│         │                       │                       │ at EL1/EL2    │
│         │                       │                       ├──────┐        │
│         │                       │                       │      │        │
│         │                       │                       │◄─────┘        │
│         │                       │                       │               │
│         │ PSCI_SUCCESS          │                       │ Execute       │
│         │◄──────────────────────│                       │ kernel        │
│         │                       │                       │               │
│                                                                          │
│  MPIDR (Multiprocessor Affinity Register):                              │
│  ─────────────────────────────────────────                              │
│   63      40 39 32 31 24 23 16 15  8 7   0                              │
│  ┌──────────┬─────┬─────┬─────┬─────┬─────┐                             │
│  │   RES0   │Aff3 │Aff2 │Aff1 │Aff0 │ RES │                             │
│  └──────────┴─────┴─────┴─────┴─────┴─────┘                             │
│                                                                          │
│  Aff0: Core ID within cluster                                           │
│  Aff1: Cluster ID                                                       │
│  Aff2: Cluster group                                                    │
│  Aff3: Node (socket)                                                    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Module Organization

### 6.1 Directory Structure

```
hal/src/arch/aarch64/
├── mod.rs                      # Module root, exports
│
├── core/                       # CPU Core Primitives
│   ├── mod.rs                  # Core module root
│   ├── registers.rs            # General purpose registers (X0-X30, SP, PC)
│   ├── system_regs.rs          # System registers (SCTLR, TCR, MAIR, etc.)
│   ├── features.rs             # CPU feature detection (ID_AA64*)
│   ├── cache.rs                # Cache maintenance operations
│   ├── barriers.rs             # Memory barriers (DMB, DSB, ISB)
│   └── fpu.rs                  # NEON/SVE state management
│
├── exception/                  # Exception Level Framework
│   ├── mod.rs                  # Exception module root
│   ├── el.rs                   # Exception level utilities
│   ├── vectors.rs              # Vector table (assembly)
│   ├── handlers.rs             # Exception handlers
│   ├── sync.rs                 # Synchronous exception handling
│   ├── irq.rs                  # IRQ handling
│   ├── fiq.rs                  # FIQ handling (optional)
│   ├── serror.rs               # SError handling
│   ├── syscall.rs              # SVC handler for system calls
│   └── frame.rs                # Exception frame structure
│
├── mmu/                        # Memory Management Unit
│   ├── mod.rs                  # MMU module root
│   ├── translation_table.rs   # Translation table management
│   ├── entries.rs              # Page/block descriptor formats
│   ├── granule.rs              # 4KB/16KB/64KB granule support
│   ├── asid.rs                 # ASID management
│   ├── tlb.rs                  # TLB maintenance
│   ├── walker.rs               # Page table walker
│   ├── mair.rs                 # Memory Attribute Indirection
│   └── tcr.rs                  # Translation Control Register
│
├── gic/                        # Generic Interrupt Controller
│   ├── mod.rs                  # GIC module root
│   ├── version.rs              # GIC version detection
│   ├── distributor.rs          # GICD_* registers
│   ├── redistributor.rs        # GICR_* registers (GICv3)
│   ├── cpu_interface.rs        # GICC_* or ICC_* registers
│   ├── gicv2.rs                # GICv2 specific
│   ├── gicv3.rs                # GICv3 specific
│   ├── routing.rs              # Interrupt routing/affinity
│   ├── sgi.rs                  # Software Generated Interrupts
│   └── lpi.rs                  # LPI support (GICv3, optional)
│
├── smp/                        # Symmetric Multi-Processing
│   ├── mod.rs                  # SMP module root
│   ├── cpu_info.rs             # CPU enumeration and topology
│   ├── mpidr.rs                # MPIDR handling
│   ├── startup.rs              # Secondary CPU startup
│   ├── per_cpu.rs              # Per-CPU data (TPIDR_EL1)
│   ├── barriers.rs             # SMP synchronization
│   ├── ipi.rs                  # Inter-processor interrupts
│   └── spin_table.rs           # Spin-table boot method
│
├── psci/                       # Power State Coordination
│   ├── mod.rs                  # PSCI module root
│   ├── conduit.rs              # SMC/HVC selection
│   ├── functions.rs            # PSCI function calls
│   ├── cpu_ops.rs              # CPU on/off/suspend
│   └── system_ops.rs           # System power operations
│
├── timers/                     # ARM Timer Framework
│   ├── mod.rs                  # Timer module root
│   ├── generic_timer.rs        # ARM Generic Timer
│   ├── system_counter.rs       # CNTPCT_EL0 / CNTVCT_EL0
│   ├── physical_timer.rs       # CNTP_* registers
│   ├── virtual_timer.rs        # CNTV_* registers
│   └── watchdog.rs             # Watchdog timer (optional)
│
└── boot/                       # Boot Support
    ├── mod.rs                  # Boot module root
    ├── dtb.rs                  # Device Tree Blob parsing
    ├── acpi.rs                 # ACPI table support
    └── early_init.rs           # Early initialization
```

### 6.2 Module Dependencies

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Module Dependencies                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│                              ┌──────────┐                               │
│                              │   boot   │                               │
│                              └────┬─────┘                               │
│                                   │                                      │
│               ┌───────────────────┼───────────────────┐                 │
│               ▼                   ▼                   ▼                 │
│         ┌──────────┐       ┌──────────┐        ┌──────────┐            │
│         │   core   │       │   mmu    │        │   psci   │            │
│         └────┬─────┘       └────┬─────┘        └────┬─────┘            │
│              │                  │                   │                   │
│              │                  │                   │                   │
│              ▼                  ▼                   ▼                   │
│         ┌──────────┐       ┌──────────┐        ┌──────────┐            │
│         │exception │◄──────│   gic    │◄───────│   smp    │            │
│         └────┬─────┘       └────┬─────┘        └────┬─────┘            │
│              │                  │                   │                   │
│              └──────────────────┼───────────────────┘                   │
│                                 ▼                                       │
│                          ┌──────────┐                                   │
│                          │  timers  │                                   │
│                          └──────────┘                                   │
│                                                                          │
│  Legend:                                                                │
│  ───────                                                                │
│  ──────► depends on                                                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Boot Sequence

### 7.1 ARM64 Boot Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       ARM64 Boot Sequence                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 1. Firmware (EL3)                                                │   │
│  │    • Hardware initialization                                     │   │
│  │    • Load bootloader/kernel                                      │   │
│  │    • Configure secure state                                      │   │
│  │    • Drop to EL2 or EL1                                          │   │
│  └──────────────────────────────────┬──────────────────────────────┘   │
│                                     │                                   │
│                                     ▼                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 2. Bootloader (UEFI/U-Boot) at EL2/EL1                           │   │
│  │    • Parse memory map                                            │   │
│  │    • Load kernel image                                           │   │
│  │    • Prepare DTB/ACPI                                            │   │
│  │    • Jump to kernel entry                                        │   │
│  └──────────────────────────────────┬──────────────────────────────┘   │
│                                     │                                   │
│                                     ▼                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 3. Kernel Entry (EL1)                                            │   │
│  │    • Primary CPU starts here                                     │   │
│  │    • X0 = DTB physical address                                   │   │
│  │    • Other CPUs in WFE/spin-table                                │   │
│  └──────────────────────────────────┬──────────────────────────────┘   │
│                                     │                                   │
│                                     ▼                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 4. Early Init (Assembly)                                         │   │
│  │    • Verify EL1                                                  │   │
│  │    • Set up initial stack                                        │   │
│  │    • Clear BSS                                                   │   │
│  │    • Enable MMU with identity mapping                            │   │
│  │    • Jump to Rust entry                                          │   │
│  └──────────────────────────────────┬──────────────────────────────┘   │
│                                     │                                   │
│                                     ▼                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 5. Rust Kernel Init                                              │   │
│  │    • Parse DTB                                                   │   │
│  │    • Initialize console                                          │   │
│  │    • Set up GIC                                                  │   │
│  │    • Initialize MMU with full mappings                           │   │
│  │    • Start secondary CPUs via PSCI                               │   │
│  │    • Initialize kernel subsystems                                │   │
│  └──────────────────────────────────┬──────────────────────────────┘   │
│                                     │                                   │
│                                     ▼                                   │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 6. Scheduler Start                                               │   │
│  │    • Create idle threads                                         │   │
│  │    • Enable interrupts                                           │   │
│  │    • Start scheduling                                            │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 8. Key System Registers

### 8.1 Essential System Registers

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Critical System Registers                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Control Registers:                                                     │
│  ──────────────────                                                     │
│  SCTLR_EL1    - System Control (MMU enable, caches, alignment)          │
│  CPACR_EL1    - Coprocessor Access Control (FP/SIMD enable)             │
│  SCR_EL3      - Secure Configuration (NS bit, EL2 enable)               │
│  HCR_EL2      - Hypervisor Configuration                                │
│                                                                          │
│  MMU Registers:                                                         │
│  ──────────────                                                         │
│  TTBR0_EL1    - Translation Table Base 0 (user space)                   │
│  TTBR1_EL1    - Translation Table Base 1 (kernel space)                 │
│  TCR_EL1      - Translation Control (granule, size, etc.)               │
│  MAIR_EL1     - Memory Attribute Indirection                            │
│                                                                          │
│  Exception Registers:                                                   │
│  ────────────────────                                                   │
│  VBAR_EL1     - Vector Base Address                                     │
│  ESR_EL1      - Exception Syndrome                                      │
│  FAR_EL1      - Fault Address                                           │
│  ELR_EL1      - Exception Link Register (return address)                │
│  SPSR_EL1     - Saved Processor State                                   │
│                                                                          │
│  Timer Registers:                                                       │
│  ────────────────                                                       │
│  CNTFRQ_EL0   - Counter Frequency                                       │
│  CNTPCT_EL0   - Physical Counter                                        │
│  CNTVCT_EL0   - Virtual Counter                                         │
│  CNTP_CTL_EL0 - Physical Timer Control                                  │
│  CNTP_CVAL_EL0- Physical Timer Compare Value                            │
│  CNTV_CTL_EL0 - Virtual Timer Control                                   │
│  CNTV_CVAL_EL0- Virtual Timer Compare Value                             │
│                                                                          │
│  CPU Identification:                                                    │
│  ───────────────────                                                    │
│  MPIDR_EL1    - Multiprocessor Affinity                                 │
│  MIDR_EL1     - Main ID (implementer, variant, part)                    │
│  ID_AA64PFR0_EL1 - Processor Feature 0                                  │
│  ID_AA64PFR1_EL1 - Processor Feature 1                                  │
│  ID_AA64MMFR0_EL1 - Memory Model Feature 0                              │
│  ID_AA64MMFR1_EL1 - Memory Model Feature 1                              │
│  ID_AA64ISAR0_EL1 - Instruction Set Attribute 0                         │
│  ID_AA64ISAR1_EL1 - Instruction Set Attribute 1                         │
│                                                                          │
│  Per-CPU Data:                                                          │
│  ─────────────                                                          │
│  TPIDR_EL1    - Thread Pointer (kernel per-CPU data)                    │
│  TPIDR_EL0    - Thread Pointer (user TLS)                               │
│  TPIDRRO_EL0  - Thread Pointer Read-Only (user)                         │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 9. Platform Support Matrix

| Platform | Type | GIC | Boot | PSCI | Priority |
|----------|------|-----|------|------|----------|
| QEMU virt | Virtual | v2/v3 | UEFI/DTB | Yes | P0 (Primary) |
| Raspberry Pi 4 | SBC | BCM2711 | Custom | Partial | P1 |
| Raspberry Pi 5 | SBC | v3 | UEFI | Yes | P1 |
| AWS Graviton | Server | v3 | UEFI/ACPI | Yes | P2 |
| Ampere Altra | Server | v3 | UEFI/ACPI | Yes | P2 |
| Apple M1/M2 | Desktop | AIC | Proprietary | No | P3 (Future) |
| Rockchip RK3588 | SoC | v3 | U-Boot | Yes | P2 |
| NXP i.MX8 | Embedded | v3 | U-Boot | Yes | P2 |

---

## 10. Implementation Phases

### Phase 1: Core Foundation (Week 1-2)
- [ ] Core CPU primitives (registers, system_regs, features)
- [ ] Exception framework (vectors, handlers, frame)
- [ ] Basic MMU (4KB granule, identity mapping)

### Phase 2: Interrupt System (Week 3-4)
- [ ] GICv2 support
- [ ] GICv3 support
- [ ] IRQ routing and handling

### Phase 3: SMP (Week 5-6)
- [ ] PSCI interface
- [ ] Secondary CPU startup
- [ ] Per-CPU data
- [ ] IPI support

### Phase 4: Full MMU (Week 7-8)
- [ ] User/kernel split
- [ ] ASID support
- [ ] Multiple granule support
- [ ] TLB management

### Phase 5: Timers & Polish (Week 9-10)
- [ ] Generic timer support
- [ ] Integration testing
- [ ] Documentation
- [ ] QEMU validation

---

## 11. Testing Strategy

### 11.1 QEMU Testing
```bash
# Basic boot test
qemu-system-aarch64 \
    -M virt,gic-version=3 \
    -cpu cortex-a72 \
    -smp 4 \
    -m 2G \
    -kernel helix-kernel \
    -nographic \
    -d int,cpu_reset

# GICv2 test
qemu-system-aarch64 \
    -M virt,gic-version=2 \
    -cpu cortex-a53 \
    -smp 2 \
    -m 1G \
    -kernel helix-kernel \
    -nographic
```

### 11.2 Unit Tests
- System register access
- Exception handling
- Page table construction
- GIC register access
- PSCI calls

---

## 12. References

1. **ARM Architecture Reference Manual for A-profile architecture** (DDI0487)
2. **ARM Generic Interrupt Controller Architecture Specification** (IHI0069)
3. **ARM Power State Coordination Interface** (DEN0022)
4. **ARM Server Base System Architecture** (DEN0029)
5. **Devicetree Specification**
6. **UEFI Specification** (ARM binding)

---

*Document Version: 1.0*
*Created: 2026-01-30*
*Author: Helix OS Architecture Team*
