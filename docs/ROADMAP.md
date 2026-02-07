# Helix OS — Roadmap

Current state of the project, tracked as a checklist. Items marked `[x]` have
corresponding implementation code in the tree. Items marked `[ ]` are planned
but not yet implemented or only partially scaffolded.

Last updated from source audit: 2026-02.

---

## Phase 1 — Foundation

### 1.1 Boot Infrastructure

- [x] Multiboot2 bootloader support (`boot/multiboot2/`)
- [x] Limine protocol support (`boot/limine/`)
  - [x] All 18 request types (bootloader info, memory map, HHDM, SMP, framebuffer)
  - [x] Safe response wrappers
  - [x] Memory management (PhysAddr, VirtAddr, HHDM translation)
  - [x] SMP support (per-CPU data, barriers)
  - [x] Framebuffer (console, graphics, double buffering)
  - [x] Firmware handoff (ACPI, SMBIOS, EFI, DTB)
  - [x] File/module loading (CPIO, ELF parsing)
  - [x] Multi-architecture (x86_64, aarch64, riscv64)
- [x] UEFI boot support (`boot/uefi/`)
  - [x] Core UEFI (raw types, GUIDs, status codes, handles)
  - [x] Boot Services, Runtime Services, System Table
  - [x] Protocols (GOP, File, Block I/O, Network, USB, PCI, Serial)
  - [x] Memory (allocator, page tables, virtual mapping, pool)
  - [x] Handoff (boot info structure for kernel entry)
  - [x] Multi-architecture (x86_64, aarch64)
  - [x] Secure Boot + TPM 2.0 (crypto, signature verification)
  - [x] FAT12/16/32 (read/write, LFN)
  - [x] GPT + MBR partition parsing
  - [x] Binary loaders (ELF64, PE/COFF, relocations)
  - [x] System tables (ACPI with FADT/MADT/DSDT, SMBIOS)
  - [x] Cryptography (SHA-256, RSA, HMAC)
  - [x] Network boot (PXE, DHCP, TFTP, HTTP/HTTPS)
  - [x] Console (text, graphics, framebuffer, PSF fonts)
  - [x] Boot manager (multi-boot, fallback, chainload)
  - [x] Recovery mode (diagnostic, repair, emergency shell)
- [x] Early console (serial, VGA)
- [x] Memory map parsing
- [x] Kernel relocation & KASLR (`subsystems/relocation/`, ~4K lines)
  - [x] PIE linker script (`profiles/common/linker_base.ld`)
  - [x] Two-stage relocation engine (early pre-MMU + full post-MMU)
  - [x] ELF64 (R_RELATIVE, R_64, R_PC32, R_32, R_32S, GOT, PLT)
  - [x] KASLR (RDSEED/RDRAND/TSC entropy, 2MB-aligned, 1GB window)
  - [x] Multi-protocol (UEFI, Limine, Multiboot2, DirectBoot)
  - [x] Validation levels (None, Quick, Standard, Full, Paranoid)
  - [ ] PIE production activation for Limine/UEFI (Multiboot2 = static)
- [x] Early boot subsystem (`subsystems/early_boot/`, ~23K lines)
  - [x] 8-stage boot sequence (PreInit → Handoff)
  - [x] x86_64 early boot (GDT, IDT, 4/5-level paging, xAPIC/x2APIC, I/O APIC, TSC/HPET/APIC timers, SMP via INIT-SIPI-SIPI, COM1 serial)
  - [x] AArch64 early boot (EL0-EL3, exception vectors, 4KB MMU, GICv2/GICv3, Generic Timer, PSCI SMP, PL011 UART)
  - [x] RISC-V early boot (M/S/U modes, Sv39/Sv48/Sv57, PLIC, CLINT, SBI v0.2+, UART 16550)
  - [x] Drivers (unified console, framebuffer, boot splash)
  - [x] KASLR + kernel handoff

### 1.2 Hardware Abstraction Layer (`hal/`, ~64K lines)

- [x] HAL trait definitions (Cpu, Mmu, InterruptController, Firmware)
- [x] x86_64 implementation
  - [x] Core (CPUID, MSR, CR0-CR4, XCR0, EFER, FPU/SSE/AVX state, cache control)
  - [x] Segmentation (type-safe selectors, 64-bit TSS with IST, per-CPU GDT)
  - [x] Interrupts (IDT 256 entries, gates, vector management, exception handlers)
  - [x] Paging (4-level + LA57 5-level, TLB with PCID, page walker, huge pages)
  - [x] APIC (xAPIC, x2APIC, I/O APIC, IPI, MSI/MSI-X)
  - [x] Timers (TSC, HPET, per-CPU APIC timer, PIT for calibration)
  - [x] SMP (topology detection, INIT-SIPI-SIPI, per-CPU GS, spinlocks, rwlocks, seqlocks)
- [x] AArch64 implementation
  - [x] Core (system registers, feature detection, cache maintenance, memory barriers, FPU/NEON/SVE)
  - [x] Exception levels (EL0-EL3, VBAR, context save/restore, SVC/IRQ/FIQ)
  - [x] MMU (4KB/16KB/64KB granule, 4-level, ASID, TLB, MAIR)
  - [x] GIC (GICv2 + GICv3, auto-detection, SGI/PPI/SPI/LPI, affinity routing)
  - [x] SMP (MPIDR, PSCI, SMC/HVC, per-CPU TPIDR_EL1, IPI via SGI)
  - [x] Timers (Generic Timer, physical/virtual/hypervisor counters)
  - [x] Platform support (QEMU virt, Raspberry Pi 4/5, ARM FVP, AWS Graviton, Ampere Altra)
- [x] RISC-V 64 implementation
  - [x] Core (x0-x31, CSRs, feature detection, FENCE)
  - [x] Privilege (M/S/U modes, trap handling, ECALL, trap frames)
  - [x] MMU (Sv39/Sv48/Sv57, SATP, TLB via SFENCE.VMA, 16-bit ASID)
  - [x] Interrupts (CLINT, PLIC, timer/software/external IRQs)
  - [x] SMP (hart ID via TP, per-hart data, SBI HSM startup, IPI, TLB shootdown)
  - [x] Timers (MTIME/MTIMECMP, SBI timer, rdtime/rdcycle/rdinstret)
  - [x] SBI (BASE, TIME, IPI, HSM, RFENCE, PMU, SRST, legacy fallbacks)
  - [x] Platform support (QEMU virt, SiFive HiFive Unmatched, OpenSBI/RustSBI)

### 1.3 Core Kernel (`core/`, ~6.4K lines)

- [x] Orchestrator design
- [x] Capability broker
- [x] Resource broker
- [x] Panic handler
- [x] `KernelComponent` trait (init, start, stop, status, health_check, reset)
- [x] Self-heal (watchdog, health monitor, recovery manager)
- [x] IPC subsystem
- [x] Syscall dispatch

### 1.4 Init Framework (`subsystems/init/`, ~17K lines)

- [x] 5-phase init (Boot → Early → Core → Late → Runtime)
- [x] DAG dependency graph (Kahn's algorithm, cycle detection)
- [x] SubsystemRegistry with global registration
- [x] InitExecutor (sequential, parallel, lazy, conditional modes)
- [x] RollbackChain for failure recovery
- [x] 50+ error variants with context chain
- [x] Concrete initializers: firmware, memory, CPU, interrupts, timers, scheduler, IPC, drivers, filesystem, network, security, debug, userland

### 1.5 Memory Subsystem (`subsystems/memory/`, ~2K lines)

- [x] Physical allocator framework (Frame, MemoryZone)
- [x] Bitmap allocator (in init subsystem)
- [x] Buddy allocator (in init subsystem)
- [x] Virtual memory framework (traits)
- [ ] Kernel heap (working integration)
- [ ] On-demand paging

### 1.6 Execution Subsystem (`subsystems/execution/`, ~2K lines)

- [x] Scheduler framework (traits)
- [x] Thread abstraction (ThreadId, atomic counter)
- [x] Process abstraction (ProcessId, atomic counter)
- [x] Round-robin scheduler module (`modules_impl/schedulers/round_robin/`)
- [x] DIS — Dynamic Intent Scheduling (`subsystems/dis/`, ~11K lines)
  - [x] Multi-level queues, per-CPU run queues
  - [x] Intent classification, priority inheritance
  - [x] Deadline-aware dispatch
- [ ] Context switching (per-arch, wired)
- [ ] Idle thread
- [x] SMP support (via HAL SMP framework)

### 1.7 Module System (`modules/`, ~2.5K lines)

- [x] Module trait and lifecycle (9 states)
- [x] Module loader framework
- [x] Dependency resolution
- [x] Hot-reload framework (pause → snapshot → unload → load → restore → resume)
- [x] ABI versioning with semver compatibility
- [ ] ELF loader (working integration)
- [ ] Module signature verification

---

## Phase 2 — Core Features

### 2.1 IPC / Message Bus

- [ ] Synchronous message passing
- [ ] Asynchronous channels
- [ ] Shared memory
- [ ] Signals
- [ ] Event system

### 2.2 Security

- [x] Secure Boot integration (via UEFI)
- [x] TPM 2.0 support (via UEFI)
- [x] Cryptographic primitives (SHA-256, RSA, HMAC)
- [ ] Capability refinement
- [ ] MAC framework
- [ ] Sandboxing
- [ ] Audit logging

### 2.3 I/O

- [x] Block device framework (via UEFI Block I/O)
- [x] Character device framework (via UEFI Serial I/O)
- [ ] VFS framework (separate from HelixFS)
- [ ] DMA support
- [ ] Interrupt routing (IRQ → handler mapping)

### 2.4 Time

- [x] System clock framework (via HAL timers)
- [x] TSC, HPET, APIC, PIT timers
- [ ] Watchdog integration
- [ ] RTC

### 2.5 Additional Schedulers

- [ ] CFS (Completely Fair Scheduler)
- [ ] Real-time scheduler (FIFO/RR)
- [ ] Cooperative scheduler
- [ ] Deadline scheduler (EDF)

### 2.6 Additional Allocators

- [ ] TLSF allocator
- [ ] Slab allocator (full)
- [ ] Zone allocator

### 2.7 Filesystems

- [x] HelixFS (`fs/`, ~42K lines) — CoW, B+tree, journal, snapshots, crypto, compression
- [x] FAT12/16/32 read/write (via UEFI, with LFN)
- [ ] RamFS
- [ ] DevFS
- [ ] ProcFS
- [ ] ext2 (read-only)

---

## Phase 3 — Userland

### 3.1 System Call Interface

- [ ] Syscall ABI stabilization
- [ ] POSIX subset
- [ ] Custom Helix syscalls
- [ ] Syscall filtering

### 3.2 Process Management

- [ ] fork/exec (scaffolded in init/userland subsystem, not wired)
- [ ] Full signal handling
- [ ] Process groups
- [ ] Sessions

### 3.3 User Space (`subsystems/userspace/`, ~3.4K lines)

- [x] ELF loader
- [x] Environment setup
- [x] Process runtime
- [x] Syscall table
- [x] Basic shell infrastructure
- [ ] Dynamic linking
- [ ] Thread-local storage
- [ ] User-space allocator

### 3.4 Shell & Utilities

- [ ] Interactive shell
- [ ] Core utilities (ls, cat, echo, etc.)
- [ ] Process viewer

---

## Phase 4 — Ecosystem

### 4.1 SDK & Tooling

- [ ] `helix-build` — build profiles tool
- [ ] `helix-pack` — package modules
- [ ] `helix-test` — testing framework
- [ ] Module templates
- [ ] Documentation generator

### 4.2 Additional Profiles

- [x] Minimal profile (`profiles/minimal/`)
- [x] Limine profile (`profiles/limine/`)
- [x] UEFI profile (`profiles/uefi/`)
- [ ] Desktop profile (with graphics)
- [ ] Server profile (networking)
- [ ] Embedded profile (minimal + drivers)
- [ ] Secure profile (hardened)

### 4.3 Drivers

- [x] GPU — Magma (`drivers/gpu/magma/`, ~17K lines, 7 crates)
  - [x] Core engine traits, GPU address types
  - [x] Command buffers
  - [x] PCI/MMIO/IOMMU HAL
  - [x] Memory management (buddy allocator)
  - [x] OpenGL-like API
  - [x] Vulkan API layer
  - [x] GSP communication (RPC)
- [ ] VirtIO (block, net, console)
- [ ] PS/2 keyboard
- [ ] Serial console (standalone driver, not boot-only)
- [ ] Framebuffer (standalone)

### 4.4 Graphics — Lumina (`graphics/`, ~197K lines)

- [x] Core API (device, queue, command buffers, descriptors, render passes, pipelines)
- [x] Shader compiler (custom IR → SPIR-V codegen via `lumina-spirv`)
- [x] Material system
- [x] Mesh pipeline
- [x] Scene graph
- [x] UI toolkit
- [x] Debug overlay
- [x] Synchronization primitives
- [x] Math library
- [x] Memory allocator
- [ ] Backend integration with Magma
- [ ] Window system interface

### 4.5 Networking

- [x] Network boot framework (PXE, TFTP, HTTP/HTTPS, DHCP — via UEFI)
- [ ] Network stack framework
- [ ] TCP/IP (as module)
- [ ] Sockets

---

## Phase 5 — NEXUS Intelligence (`subsystems/nexus/`, ~320K lines)

See [NEXUS_EVOLUTION.md](NEXUS_EVOLUTION.md) for the full AI subsystem roadmap.

- [x] Q1 — Testing, fuzzing, chaos engineering, formal proofs, benchmarking
- [x] Q2 — Prediction engines, degradation detection, canary analysis, anomaly detection
- [x] Q3 — Healing engines, micro-rollback, state reconstruction, quarantine, hot substitution
- [x] Q4 — SIMD-accelerated optimization paths
- [x] Cognition pipeline (~30K lines: coordinator, executor, oracle, fusion, insight)
- [x] Prediction engine (decision trees, feature tracking, confidence thresholds)
- [x] Healing engine (checkpoint store, quarantine manager, rollback history)
- [x] Telemetry, formal verification, policy engine, adaptive learning
- [x] Shared types (`nexus-types/`, ~3.5K lines)
- [x] Core orchestrator types (`nexus-core/`, 437 lines, 7-level operational model)
- [ ] Cognitive layer extraction (`nexus-cognitive/` — scaffolded, empty modules)
- [ ] Evolution layer extraction (`nexus-evolution/` — scaffolded, sandbox only)

---

## Milestones

| # | Target | Description | Status |
|:--|:-------|:------------|:-------|
| M0 | Month 1 | Boot to serial output | Done |
| M1 | Month 3 | Memory management working | In progress |
| M2 | Month 6 | Scheduler with context switching | Pending |
| M3 | Month 9 | First module hot-reload | Pending |
| M4 | Month 12 | Basic file system operational | In progress (HelixFS + FAT32 exist) |
| M5 | Month 15 | First user process | Pending |
| M6 | Month 18 | Shell running | Pending |
| M7 | Month 24 | SDK release | Pending |

---

## Non-Goals (v1.0)

- GUI / desktop environment (Lumina is a framework, not a compositor)
- Real hardware driver coverage (focus on QEMU + VirtIO)
- Full POSIX compliance
- Binary compatibility with Linux
- Production readiness

---

## Success Criteria

1. Any major component can be replaced without reboot
2. Scheduler/allocator swap with < 10ms downtime
3. Zero hard-coded policies in kernel core
4. Same codebase for x86_64, aarch64, riscv64
5. Every public API documented
6. &gt; 80% test coverage on core components

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md). Priority areas:

1. Architecture-specific HAL implementations
2. Scheduler and allocator modules
3. Filesystem modules
4. Wiring existing frameworks to concrete implementations
5. Testing and CI
