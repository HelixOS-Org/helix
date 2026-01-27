# Helix OS Framework Documentation

<div align="center">

<img src="assets/helix-logo.svg" alt="Helix Logo" width="200"/>

🧬 **A Framework for Building Operating Systems**

[![Version](https://img.shields.io/badge/version-0.1.0--alpha-blue.svg)](CHANGELOG.md)
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green.svg)](../LICENSE)
[![Documentation](https://img.shields.io/badge/docs-complete-brightgreen.svg)](#documentation-index)

**75,000+ Lines of Rust** | **Modular Architecture** | **Hot-Reload Support** | **Multi-Architecture**

[Quick Start](#quick-start) • [Documentation](#documentation-index) • [Examples](#examples) • [Contributing](#contributing)

</div>

---

## Welcome to Helix

**Helix is not an operating system** — it's a **framework for building operating systems**.

Think of it as a kernel construction kit where:
- Every major component is a **replaceable module**
- The core kernel is **policy-free**
- Modules can be **hot-reloaded** without rebooting
- You can build **desktop**, **server**, **embedded**, or **secure** OS variants from the same codebase

Whether you're a researcher exploring new scheduling algorithms, an embedded developer building custom firmware, or a student learning OS internals, Helix provides the foundation you need.

---

## Quick Start

### Prerequisites

```bash
# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools-preview

# Install build tools (Ubuntu/Debian)
sudo apt install qemu-system-x86 xorriso grub-pc-bin grub-common make
```

### Build and Run

```bash
# Clone the repository
git clone https://github.com/helix-os/helix
cd helix

# Build the complete system
bash scripts/build.sh all

# Run in QEMU
bash scripts/run_qemu.sh
```

### Expected Output

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                          HELIX OS FRAMEWORK v0.1.0                           ║
║                     A Framework for Building Operating Systems               ║
╚══════════════════════════════════════════════════════════════════════════════╝

[BOOT] Multiboot2 boot successful
[INIT] Serial port initialized at 115200 baud
[HEAP] Kernel heap: 1048576 bytes @ 0x200000
[MEM]  Memory subsystem initialized
[INT]  Interrupt handlers installed
[SCHED] Scheduler framework ready
[FS]   HelixFS initialized: 4096 KB total, 4032 KB free

helix> _
```

---

## Documentation Index

### 📚 User Guides

| Document | Description | Audience |
|----------|-------------|----------|
| [Introduction](guide/INTRODUCTION.md) | Complete project overview, philosophy, and goals | Everyone |
| [Getting Started](guide/GETTING_STARTED.md) | Installation, setup, and first steps | Beginners |

### 🏗️ Architecture

| Document | Description | Audience |
|----------|-------------|----------|
| [Architecture Overview](architecture/OVERVIEW.md) | Complete system architecture and design | All developers |
| [Boot Process](architecture/BOOT_PROCESS.md) | Detailed boot sequence from BIOS to kernel | Kernel developers |
| [Memory Model](architecture/MEMORY_MODEL.md) | Physical and virtual memory management | Kernel developers |

### 📖 API Reference

| Document | Description | Lines |
|----------|-------------|-------|
| [Core API](api/CORE.md) | Kernel core, orchestrator, IPC, syscalls | ~800 |
| [HAL API](api/HAL.md) | Hardware Abstraction Layer traits | ~600 |
| [Module System](api/MODULES.md) | Module loading, registry, hot-reload | ~700 |
| [Filesystem API](api/FILESYSTEM.md) | HelixFS complete reference | ~900 |
| [Scheduler API](api/SCHEDULER.md) | DIS, scheduling, execution | ~700 |

### 🔧 Development

| Document | Description | Audience |
|----------|-------------|----------|
| [Contributing](development/CONTRIBUTING.md) | How to contribute to Helix | Contributors |
| [Coding Standards](development/CODING_STANDARDS.md) | Code style and conventions | All developers |
| [Debugging Guide](development/DEBUGGING.md) | Debug techniques and tools | Developers |

### 📋 Reference

| Document | Description |
|----------|-------------|
| [Changelog](reference/CHANGELOG.md) | Version history and release notes |
| [Glossary](reference/GLOSSARY.md) | Terms and definitions |
| [FAQ](reference/FAQ.md) | Frequently asked questions |

---

## Key Features

### 🔄 Hot-Reload Kernel Modules

Replace kernel components at runtime without rebooting:

```rust
// Swap scheduler while the system is running
registry.hot_swap(scheduler_slot, Box::new(PriorityScheduler::new()))?;
// Result: Zero downtime, state preserved
```

### 🧠 Dynamic Intent Scheduling (DIS)

Revolutionary scheduling based on application intentions:

```rust
// Applications declare what they WANT
let intent = Intent::new()
    .class(IntentClass::Interactive)
    .latency_target(Duration::from_millis(16))  // 60 FPS
    .cpu_budget(CpuBudget::Percent(30))
    .build();

// The scheduler LEARNS and ADAPTS
scheduler.spawn_with_intent("ui_render", render_fn, intent);
```

### 📁 HelixFS — Modern Filesystem

Copy-on-Write filesystem with advanced features:

- **Instant Snapshots**: O(1) via CoW semantics
- **Journaling**: Automatic crash recovery
- **Compression**: LZ4, ZSTD per-extent
- **Encryption**: AES-GCM, ChaCha20-Poly1305

### 🩺 Self-Healing Kernel

Automatic recovery from module failures:

```rust
// Module crashes? Auto-recovery!
if module.health_check() == HealthStatus::Crashed {
    self_heal.recover("scheduler")?;
    // Module restarted with preserved state
}
```

---

## Project Structure

```
helix/
├── boot/           # Boot protocols (Multiboot2)
├── hal/            # Hardware Abstraction Layer
├── core/           # Kernel core (orchestrator, IPC, syscall)
├── subsystems/
│   ├── execution/  # Thread/process management
│   ├── memory/     # Memory management
│   ├── dis/        # Dynamic Intent Scheduling
│   └── userspace/  # Userspace support
├── modules/        # Module system
├── fs/             # HelixFS (42,500+ lines)
├── benchmarks/     # Performance benchmarks
├── profiles/       # OS profiles (minimal, desktop, server)
├── scripts/        # Build and run scripts
└── docs/           # Documentation (you are here)
```

---

## Examples

### Creating a Custom Scheduler Module

```rust
use helix_modules::{Module, ModuleMetadata, ModuleContext};

pub struct MyScheduler {
    // Your scheduler state
}

impl Module for MyScheduler {
    fn metadata(&self) -> &ModuleMetadata { &METADATA }
    
    fn init(&mut self, ctx: &ModuleContext) -> Result<(), &'static str> {
        log::info!("MyScheduler initialized");
        Ok(())
    }
    
    fn start(&mut self) -> Result<(), &'static str> {
        log::info!("MyScheduler started");
        Ok(())
    }
    
    fn provides(&self) -> &[&str] { &["scheduler"] }
}
```

### Working with HelixFS

```rust
use helix_fs::api::{create_file, write_file, read_file, create_snapshot};

// Create and write a file
let inode = create_file("/documents", "report.txt")?;
write_file(inode, b"Hello, HelixFS!")?;

// Create instant snapshot
let snapshot = create_snapshot("before-changes")?;

// Modify file
write_file(inode, b"Modified content")?;

// Restore to snapshot if needed
restore_snapshot(snapshot)?;
```

### Using Dynamic Intent Scheduling

```rust
use helix_dis::{Intent, IntentClass, spawn_with_intent};

// Spawn a real-time audio task
let audio_intent = Intent::new()
    .class(IntentClass::RealTime)
    .latency_target(Duration::from_micros(100))
    .priority_boost(true)
    .build();

spawn_with_intent("audio_mixer", audio_process, audio_intent);

// Spawn a background indexing task
let index_intent = Intent::new()
    .class(IntentClass::Background)
    .cpu_budget(CpuBudget::Percent(5))
    .build();

spawn_with_intent("file_indexer", index_files, index_intent);
```

---

## Benchmarks

Performance measurements on x86_64 (3.6 GHz):

| Benchmark | Cycles | Time | Notes |
|-----------|--------|------|-------|
| Context Switch (minimal) | ~21 | 8.4 ns | Register save/restore only |
| Context Switch (full) | ~180 | 72 ns | Full state + TLB |
| Thread Yield | ~43 | 17 ns | Cooperative yield |
| Syscall Latency | ~95 | 38 ns | Round-trip |
| DIS Intent Evaluation | ~18 | 7 ns | Intent matching |
| Memory Allocation (4KB) | ~150 | 60 ns | Bump allocator |
| IPC Round-trip | ~500 | 200 ns | Channel message |

Run benchmarks:
```bash
helix> bench full
```

---

## Supported Architectures

| Architecture | Status | Notes |
|--------------|--------|-------|
| x86_64 | ✅ Primary | Full implementation |
| aarch64 | 🔄 In Progress | HAL stubs ready |
| riscv64 | 📋 Planned | HAL traits defined |

---

## Contributing

We welcome contributions! See [Contributing Guide](development/CONTRIBUTING.md) for details.

### Quick Contribution Steps

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Write tests for your changes
4. Ensure all tests pass: `cargo test --workspace`
5. Format code: `cargo fmt`
6. Lint: `cargo clippy`
7. Submit a pull request

### Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/helix
cd helix

# Build and test
cargo build --workspace
cargo test --workspace

# Run in QEMU for integration testing
bash scripts/build.sh all
bash scripts/run_qemu.sh
```

---

## Community

- **Discussions**: [GitHub Discussions](https://github.com/helix-os/helix/discussions)
- **Issues**: [GitHub Issues](https://github.com/helix-os/helix/issues)
- **Chat**: [Discord Server](https://discord.gg/helix-os)

---

## License

Helix is dual-licensed under:

- [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](http://opensource.org/licenses/MIT)

You may choose either license.

---

## Acknowledgments

Helix builds upon the work of many open-source projects and research papers:

- [Rust](https://www.rust-lang.org/) — Systems programming language
- [QEMU](https://www.qemu.org/) — Machine emulator
- [OSDev Wiki](https://wiki.osdev.org/) — OS development resources
- [Writing an OS in Rust](https://os.phil-opp.com/) — Blog series by Philipp Oppermann

---

<div align="center">

**Helix OS Framework** — *Build Different. Build Revolutionary.*

🧬

*Made with ❤️ by the Helix community*

</div>
