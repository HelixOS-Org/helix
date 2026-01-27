# Helix OS Framework

<div align="center">

🧬 **A Framework for Creating Operating Systems**

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![Architecture](https://img.shields.io/badge/arch-x86__64%20%7C%20aarch64%20%7C%20riscv64-green.svg)](#supported-architectures)

</div>

---

## 🎯 What is Helix?

Helix is **not an operating system** — it's a **framework for creating operating systems**.

Think of it as a kernel construction kit where:
- Every major component is a **replaceable module**
- The kernel core is **policy-free**
- Modules can be **hot-reloaded** without rebooting
- You can build **desktop**, **server**, **embedded**, or **secure** OS variants from the same codebase

## ✨ Key Features

| Feature | Description |
|---------|-------------|
| 🔌 **Modular** | Schedulers, allocators, filesystems are all swappable modules |
| 🔄 **Hot Reload** | Replace kernel components at runtime |
| 🏗️ **Policy-Free** | Kernel makes no policy decisions — modules do |
| 🎯 **Multi-Target** | Same codebase for x86_64, aarch64, riscv64 |
| 🛡️ **Capability-Based** | Fine-grained security model |
| 📦 **SDK Included** | Tools to build and package your own OS |

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Layer 7: Userland Interface              │
├─────────────────────────────────────────────────────────────┤
│                    Layer 6: Policy Layer                    │
├─────────────────────────────────────────────────────────────┤
│                    Layer 5: IPC / Message Bus               │
├─────────────────────────────────────────────────────────────┤
│                 Layer 4: Module Runtime & Loader            │
├─────────────────────────────────────────────────────────────┤
│                  Layer 3: Subsystems Framework              │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐  │
│  │Execution │  Memory  │   I/O    │ Security │  Time    │  │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘  │
├─────────────────────────────────────────────────────────────┤
│               Layer 2: Kernel Core (Orchestrator)           │
├─────────────────────────────────────────────────────────────┤
│                Layer 1: Boot & HAL (minimal ASM)            │
└─────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- Rust nightly (`rustup default nightly`)
- QEMU for testing
- GNU make or cargo-make

### Build

```bash
# Clone the repository
git clone https://github.com/helix-os/helix
cd helix

# Build the minimal profile
cargo build --release -p helix-minimal-os --target x86_64-unknown-none

# Run in QEMU
qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/helix-minimal-os \
    -serial stdio -m 256M
```

### Create Your Own OS

1. Create a new profile in `profiles/myos/`
2. Configure `helix.toml`
3. Select your modules
4. Build and run!

See the [OS Builder Guide](docs/OS_BUILDER_GUIDE.md) for details.

## 📁 Project Structure

```
helix/
├── boot/               # Boot protocols (multiboot2, limine, uefi)
├── hal/                # Hardware Abstraction Layer
│   └── arch/          # Architecture-specific (x86_64, aarch64, riscv64)
├── core/              # Kernel core (orchestrator)
├── subsystems/        # Subsystem frameworks
│   ├── execution/     # Threads, processes, scheduling
│   ├── memory/        # Memory management
│   ├── io/            # I/O framework
│   ├── security/      # Security framework
│   └── time/          # Time management
├── modules/           # Module system
├── ipc/               # Inter-process communication
├── policy/            # Policy layer
├── profiles/          # OS profiles
│   ├── minimal/       # Minimal embedded OS
│   ├── desktop/       # Full-featured desktop
│   └── server/        # Server configuration
├── modules_impl/      # Module implementations
│   ├── schedulers/    # Scheduler modules
│   ├── allocators/    # Allocator modules
│   └── filesystems/   # Filesystem modules
└── docs/              # Documentation
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/ARCHITECTURE.md) | Technical architecture overview |
| [Project Structure](docs/PROJECT_STRUCTURE.md) | Complete file structure |
| [Module Guide](docs/MODULE_GUIDE.md) | How to write modules |
| [OS Builder Guide](docs/OS_BUILDER_GUIDE.md) | How to build your own OS |
| [Roadmap](docs/ROADMAP.md) | Development roadmap |

## 🔧 Available Modules

### Schedulers
- **Round-Robin** - Simple time-sharing scheduler
- **CFS** - Completely Fair Scheduler (Linux-like)
- **Real-Time** - FIFO/RR real-time scheduling
- **Cooperative** - Non-preemptive scheduling

### Allocators
- **Bitmap** - Simple bitmap allocator
- **Buddy** - Buddy system allocator
- **Slab** - Slab allocator for kernel objects
- **TLSF** - Two-Level Segregate Fit (real-time)

### Filesystems
- **RamFS** - RAM-based filesystem
- **DevFS** - Device filesystem
- **ProcFS** - Process information

## 🎯 Supported Architectures

| Architecture | Status | Notes |
|-------------|--------|-------|
| x86_64 | 🟡 In Progress | Primary target |
| aarch64 | 🔴 Planned | ARM64 support |
| riscv64 | 🔴 Planned | RISC-V 64-bit |

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Priority areas:
- Architecture-specific HAL implementations
- Scheduler/allocator modules
- Filesystem modules
- Documentation and examples

## 📝 License

Helix is dual-licensed under:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

Choose whichever license works best for your project.

## 🙏 Acknowledgments

Inspired by:
- seL4 (capability-based security)
- Zephyr (modularity)
- Redox (Rust OS design)
- Fuchsia (microkernel architecture)

---

<div align="center">

**[Documentation](docs/)** · **[Examples](profiles/)** · **[Contributing](CONTRIBUTING.md)**

</div>
