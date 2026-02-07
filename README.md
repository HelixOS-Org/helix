<div align="center">

<!-- [Helix Logo] -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-banner.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo-banner.svg">
  <img alt="Helix OS" src="assets/logo-banner.svg" width="480">
</picture>

<br/>
<br/>

**A modular, capability-based kernel framework written in Rust.**

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=flat-square)](LICENSE)
[![Rust: nightly-2025-01-15](https://img.shields.io/badge/rust-nightly--2025--01--15-orange?style=flat-square&logo=rust)](rust-toolchain.toml)
[![Target: x86_64-unknown-none](https://img.shields.io/badge/target-x86__64--unknown--none-green?style=flat-square)](#)

</div>

---

## Synopsis

Helix is a `no_std` kernel framework targeting `x86_64-unknown-none`. It boots via
**Limine**, **UEFI**, or **Multiboot2**, and is built entirely in Rust (edition 2021,
nightly channel). The workspace is structured as a Cargo workspace of ~20 crates where
subsystems provide trait-based *frameworks* and pluggable modules provide
*implementations*. The core kernel is policy-free by design.

> **Status:** Research / pre-alpha (`v0.1.0-alpha`). Not suitable for production use.

---

## Architecture & Subsystems

The codebase separates into distinct layers. Below is what each directory in the tree
provides, based on a direct audit of the source modules.

### `boot/` — Boot Protocol Implementations

Three bootloader frontends, each a standalone crate:

| Crate | Protocol | Notes |
|:------|:---------|:------|
| `boot/limine/` | [Limine](https://limine-bootloader.org/) | Request/response model, SMP, framebuffer, HHDM, ACPI/SMBIOS handoff |
| `boot/multiboot2/` | Multiboot2 | Standard header parsing, memory map extraction |
| `boot/uefi/` | UEFI | Boot Services, Runtime Services, GOP, Secure Boot, TPM 2.0, FAT, GPT, ELF/PE loaders, PXE/TFTP |

An early-boot assembly stub (`boot/src/boot.asm`) handles stack setup and BSP handoff
before transferring control to Rust.

### `hal/` — Hardware Abstraction Layer

Defines the trait interface that all architecture backends must satisfy:

```rust
// hal/src/lib.rs (simplified)
pub trait Cpu: Send + Sync { /* ... */ }
pub trait Mmu: Send + Sync { /* ... */ }
pub trait InterruptController: Send + Sync { /* ... */ }
pub trait Firmware: Send + Sync { /* ... */ }
```

Modules: `cpu`, `mmu`, `interrupts`, `firmware`, `relocation` (ELF relocation engine),
`kaslr` (Kernel Address Space Layout Randomization via RDSEED/RDRAND/TSC), and `arch/`
containing the x86_64 implementation (APIC, x2APIC, GDT/IDT/TSS, 4-/5-level paging,
PIT/HPET/TSC timers). AArch64 and RISC-V targets are declared in `rust-toolchain.toml`
and have stub adapters in `arch_stubs/`.

### `core/` — Kernel Core (Orchestrator)

The minimal trusted computing base. Policy-free — provides mechanisms only.

| Module | Role |
|:-------|:-----|
| `orchestrator/` | Lifecycle management (boot, shutdown, suspend/resume), capability broker, resource broker |
| `syscall/` | Syscall gateway, dispatcher, argument validation, registration |
| `interrupts/` | Interrupt routing, handler registration, exception dispatch |
| `ipc/` | Inter-process communication primitives |
| `selfheal.rs` | Watchdog + health monitor + recovery manager: detects crashed/hung modules, auto-restarts them, migrates state |
| `hotreload/` | Atomic module swap at runtime — pause → snapshot → unload → load → restore → resume — with ABI versioning and rollback |
| `debug/` | Kernel console, GDB stub, kprobes |

### `subsystems/` — Subsystem Frameworks

Each subsystem exposes traits; implementations are provided by modules under
`modules_impl/`.

| Crate | Responsibility |
|:------|:---------------|
| `execution/` | `ThreadId`/`ProcessId` types, scheduler trait + run-queue abstraction, context switching, execution domains (kernel/user) |
| `memory/` | Physical frame allocator trait, virtual memory mapper, memory region tracking, protection flags |
| `nexus/` | Kernel-level observability and intelligence — failure prediction, self-healing hooks, anomaly detection, tracing/causal graphs. Split into four sub-crates: `nexus-types`, `nexus-core`, `nexus-cognitive`, `nexus-evolution` |
| `dis/` | Distributed intent scheduler — policy engine, isolation, IPC queues, optimizer, statistics collection |
| `relocation/` | Two-stage ELF relocation engine (pre-MMU and post-MMU), multi-protocol support (UEFI, Limine, Multiboot2) |
| `userspace/` | User-space process support |
| `early_boot/` | Early boot sequence coordination |
| `init/` | System initialization subsystem |

### `modules/` & `modules_impl/` — Module System

`modules/` defines the module lifecycle (registration → dependency resolution →
loading → init → running → shutdown → unloading), a module registry, a dependency
resolver, ABI version compatibility checks, and the hot-reload protocol.

`modules_impl/` contains concrete implementations. Currently ships:

- `schedulers/round_robin/` — A round-robin scheduler conforming to the execution
  subsystem's scheduler trait.

### `fs/` — HelixFS

A copy-on-write filesystem with:

- Transactional writes (journal engine, atomic commits, rollback)
- B+Tree and radix tree metadata indexing
- Extent-based allocation with adaptive per-extent compression
- AEAD encryption with per-file keys
- Merkle DAG integrity verification
- Snapshot management (O(1) via CoW semantics)

On-disk layout: superblock (blocks 0–15, 8× replicated), allocation bitmap
(16–1023), CoW data region (1024+).

### `graphics/` — Lumina

A workspace of 21 `lumina-*` crates providing a `no_std` GPU rendering stack.
Includes a shader IR, SPIR-V code generation, material/mesh/scene-graph
abstractions, a pipeline compiler, and a debug inspector.

### `profiles/` — OS Profiles

Profiles compose framework components into a bootable kernel image. Each profile
selects which modules, boot protocol, and configuration to wire together.

| Profile | Purpose |
|:--------|:--------|
| `minimal/` | Bare-minimum kernel: bump allocator, serial console, round-robin scheduler |
| `limine/` | Limine-specific boot configuration |
| `uefi/` | UEFI-specific boot configuration |
| `common/` | Shared linker scripts and base configuration |

### `benchmarks/`

A `no_std` benchmarking harness with configurable warmup/iteration counts and
statistical output. Modules: `engine`, `scheduler`, `memory`, `ipc`, `irq`,
`timing`, `stress`, `results`.

---

## Development Philosophy

Helix exists to explore a specific question: **what does a kernel look like when
every subsystem is a trait and every implementation is a replaceable module?**

The design constraints are deliberate:

1. **Mechanism, not policy.** The core kernel (Layer 2) provides interrupt dispatch,
   capability validation, and context switching. All scheduling policies, allocation
   strategies, and filesystem behavior live in swappable modules outside the TCB.

2. **`no_std` everywhere.** Every crate compiles with `#![no_std]`. The workspace
   depends on `spin`, `bitflags`, `hashbrown`, and `heapless` — nothing that
   touches a system allocator or OS API.

3. **Trait-driven HAL.** Architecture-specific code is isolated behind trait
   boundaries in `hal/`. Adding a new target means implementing `Cpu`, `Mmu`,
   `InterruptController`, and `Firmware` — the rest of the kernel compiles
   unchanged.

4. **Hot-reload as a first-class primitive.** Module replacement at runtime is
   not an afterthought. The hot-reload protocol (state snapshot, atomic swap,
   ABI compatibility check, automatic rollback) is built into `core/hotreload/`
   and tested via fault injection in `core/hotreload/chaos/`.

5. **Self-healing by default.** `core/selfheal.rs` implements a watchdog +
   health-monitor + recovery-manager pipeline. When a module crashes or stops
   responding, the kernel can restart it and attempt state migration — without
   requiring a full reboot.

This is a research project. The goal is not to replace Linux. It is to build a
test bed where ideas like cognitive scheduling (`nexus/`), runtime module
evolution, and zero-downtime kernel patching can be prototyped in a memory-safe
language.

---

## Building

### Prerequisites

- **Rust** nightly (`nightly-2025-01-15`) — `rust-toolchain.toml` handles this
  automatically via `rustup`.
- **QEMU** — for `x86_64` emulation.
- Components: `rust-src`, `llvm-tools-preview` (also pinned in the toolchain
  file).

### Quick start

```bash
git clone https://github.com/helix-os/helix.git
cd helix

# Build the kernel (release, x86_64-unknown-none)
./scripts/build.sh

# Boot in QEMU
./scripts/run_qemu.sh
```

### Other targets

```bash
./scripts/build.sh --debug          # Debug build
./scripts/build.sh --iso            # Bootable ISO image

cargo test --target x86_64-unknown-linux-gnu --lib   # Unit tests (host)
./scripts/test.sh                                     # Full test suite

cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo doc --no-deps --document-private-items
```

---

## Contributing

Patches are welcome. The codebase is large but modular — you can work on a
single crate without understanding the full kernel.

**Before submitting a PR:**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --target x86_64-unknown-linux-gnu --lib
```

**Where to start:**

- Implement a new scheduler module under `modules_impl/schedulers/` (the
  round-robin implementation serves as a reference).
- Add a VirtIO driver or a keyboard driver.
- Improve the filesystem (`fs/`) — e.g., implement a ramfs or extend VFS
  coverage.
- Write tests or benchmarks for existing subsystems.
- Improve documentation — `docs/MODULE_GUIDE.md` and `docs/OS_BUILDER_GUIDE.md`
  explain the framework's extension points.

See [CONTRIBUTING.md](docs/development/CONTRIBUTING.md) for full guidelines.

---

## Documentation

| Document | Description |
|:---------|:------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Layered architecture overview |
| [docs/PROJECT_STRUCTURE.md](docs/PROJECT_STRUCTURE.md) | Crate-by-crate walkthrough |
| [docs/MODULE_GUIDE.md](docs/MODULE_GUIDE.md) | Writing and hot-reloading kernel modules |
| [docs/OS_BUILDER_GUIDE.md](docs/OS_BUILDER_GUIDE.md) | Composing a custom OS from Helix profiles |

---

## License

Licensed under either of:

- [MIT License](LICENSE)
- [Apache License, Version 2.0](LICENSE)

at your option.

---

<div align="center">

<!-- [Helix Logo] -->
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo-banner.svg">
  <source media="(prefers-color-scheme: light)" srcset="assets/logo-banner.svg">
  <img alt="Helix OS" src="assets/logo-banner.svg" width="120">
</picture>

<br/>

<sub>Helix OS — a Rust kernel framework.</sub>

</div>
