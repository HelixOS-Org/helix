# Helix OS Framework - Roadmap

## Vision

Helix is not an operating system — it's a **framework for creating operating systems**.

The goal is to provide a highly modular, policy-free kernel where every major component
(scheduler, allocator, filesystem, etc.) can be replaced at runtime without rebooting.

---

## Phase 1: Foundation (Months 1-6)

### 1.1 Boot Infrastructure
- [ ] Multiboot2 bootloader support
- [ ] Limine protocol support
- [ ] UEFI boot support
- [ ] Early console (serial, VGA)
- [ ] Memory map parsing
- [ ] Kernel relocation

### 1.2 Hardware Abstraction Layer
- [x] HAL trait definitions
- [ ] x86_64 implementation
  - [ ] GDT/IDT setup
  - [ ] Paging (4-level, 5-level)
  - [ ] APIC/IOAPIC
  - [ ] TSC/HPET timers
- [ ] aarch64 implementation
  - [ ] Exception levels
  - [ ] MMU (4KB/64KB pages)
  - [ ] GIC
- [ ] riscv64 implementation (basic)

### 1.3 Core Kernel
- [x] Orchestrator design
- [x] Capability broker
- [x] Resource broker
- [x] Panic handler
- [ ] Early boot sequence
- [ ] Subsystem initialization order

### 1.4 Memory Subsystem
- [x] Physical allocator framework
- [x] Bitmap allocator
- [x] Buddy allocator
- [x] Virtual memory framework
- [ ] Kernel heap (working)
- [ ] On-demand paging

### 1.5 Execution Subsystem
- [x] Scheduler framework
- [x] Thread abstraction
- [x] Process abstraction
- [x] Round-robin scheduler module
- [ ] Context switching (per-arch)
- [ ] Idle thread
- [ ] Basic SMP support

### 1.6 Module System
- [x] Module trait
- [x] Module loader framework
- [x] Dependency resolution
- [x] Hot reload framework
- [ ] ELF loader (working)
- [ ] Module verification

---

## Phase 2: Core Features (Months 7-12)

### 2.1 IPC / Message Bus
- [ ] Synchronous message passing
- [ ] Asynchronous channels
- [ ] Shared memory
- [ ] Signals
- [ ] Event system

### 2.2 Security Subsystem
- [ ] Capability refinement
- [ ] MAC framework
- [ ] Sandboxing
- [ ] Secure boot integration
- [ ] Audit logging

### 2.3 I/O Subsystem
- [ ] VFS framework
- [ ] Block device framework
- [ ] Character device framework
- [ ] DMA support
- [ ] Interrupt routing

### 2.4 Time Subsystem
- [ ] System clock
- [ ] Timers
- [ ] Watchdog
- [ ] RTC integration

### 2.5 Additional Schedulers
- [ ] CFS (Completely Fair Scheduler)
- [ ] Real-time scheduler (FIFO/RR)
- [ ] Cooperative scheduler
- [ ] Deadline scheduler

### 2.6 Additional Allocators
- [ ] TLSF allocator
- [ ] Slab allocator (full)
- [ ] Zone allocator

### 2.7 Filesystems
- [ ] RamFS
- [ ] DevFS
- [ ] ProcFS
- [ ] Basic ext2 (read-only)

---

## Phase 3: Userland (Months 13-18)

### 3.1 System Call Interface
- [ ] Syscall ABI stabilization
- [ ] POSIX subset implementation
- [ ] Custom Helix syscalls
- [ ] Syscall filtering

### 3.2 Process Management
- [ ] fork/exec
- [ ] Signals (full)
- [ ] Process groups
- [ ] Sessions

### 3.3 User Space
- [ ] ELF loading (user)
- [ ] Dynamic linking
- [ ] Thread-local storage
- [ ] User-space allocator

### 3.4 Shell & Utilities
- [ ] Basic shell
- [ ] Core utilities (ls, cat, etc.)
- [ ] Process viewer

---

## Phase 4: Ecosystem (Months 19-24)

### 4.1 SDK & Tooling
- [ ] `helix-build` - Build profiles
- [ ] `helix-pack` - Package modules
- [ ] `helix-test` - Testing framework
- [ ] Module templates
- [ ] Documentation generator

### 4.2 Additional Profiles
- [ ] Desktop profile (with graphics)
- [ ] Server profile (networking)
- [ ] Embedded profile (minimal)
- [ ] Secure profile (hardened)

### 4.3 Drivers
- [ ] VirtIO (block, net, console)
- [ ] PS/2 keyboard
- [ ] Serial console
- [ ] Framebuffer

### 4.4 Networking (optional)
- [ ] Network stack framework
- [ ] TCP/IP (as module)
- [ ] Sockets

### 4.5 Graphics (optional)
- [ ] Framebuffer abstraction
- [ ] 2D graphics library
- [ ] Window system interface

---

## Milestones

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| M0 | Month 1 | Boot to serial output |
| M1 | Month 3 | Memory management working |
| M2 | Month 6 | Scheduler with context switching |
| M3 | Month 9 | First module hot-reload |
| M4 | Month 12 | Basic file system |
| M5 | Month 15 | First user process |
| M6 | Month 18 | Shell running |
| M7 | Month 24 | SDK release |

---

## Success Criteria

1. **Modularity**: Any major component can be replaced without reboot
2. **Hot Reload**: Scheduler/allocator swap with < 10ms downtime
3. **Policy-Free**: Zero hard-coded policies in kernel core
4. **Multi-Arch**: Same codebase for x86_64, aarch64, riscv64
5. **Documentation**: Every public API documented
6. **Testing**: > 80% test coverage on core components

---

## Non-Goals (for v1.0)

- GUI/Desktop environment
- Real hardware driver coverage (focus on VirtIO)
- Full POSIX compliance
- Binary compatibility with Linux
- Production readiness

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Priority areas for contributions:
1. Architecture-specific HAL implementations
2. Scheduler/allocator modules
3. Filesystem modules
4. Documentation and examples
