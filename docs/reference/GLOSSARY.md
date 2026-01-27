# Helix OS Glossary

<div align="center">

📖 **Technical Terms and Definitions**

*A comprehensive reference for Helix OS terminology*

</div>

---

## Table of Contents

1. [Architecture Terms](#architecture-terms)
2. [Memory Management](#memory-management)
3. [Scheduling](#scheduling)
4. [Filesystem](#filesystem)
5. [Hardware Abstraction](#hardware-abstraction)
6. [Module System](#module-system)
7. [Inter-Process Communication](#inter-process-communication)
8. [Boot Process](#boot-process)
9. [Security](#security)
10. [Debugging](#debugging)
11. [General OS Terms](#general-os-terms)
12. [Rust-Specific Terms](#rust-specific-terms)
13. [Abbreviations](#abbreviations)

---

## Architecture Terms

### Capability Broker
A Helix component responsible for managing and distributing capabilities (tokens representing access rights) to modules. The capability broker ensures that modules only access resources they are authorized to use.

```
┌──────────────────┐
│ Capability Broker│
├──────────────────┤
│ - Grant caps     │
│ - Revoke caps    │
│ - Validate caps  │
└──────────────────┘
```

### Differentiated Intent Scheduler (DIS)
Helix's custom scheduling subsystem that uses "intents" to make intelligent scheduling decisions. Instead of raw priorities, tasks declare their behavioral intent (realtime, interactive, batch, background) and the scheduler optimizes accordingly.

### HAL (Hardware Abstraction Layer)
The `helix-hal` crate providing trait-based abstractions over hardware functionality. This enables platform-independent kernel code by defining interfaces for CPU, MMU, interrupts, and firmware.

### Helix Core
The central `helix-core` crate containing fundamental kernel infrastructure including IPC, orchestration, interrupt handling, syscall dispatch, and module management.

### Higher-Half Kernel
A kernel mapping scheme where the kernel is mapped to the upper portion of virtual memory (typically above `0xFFFF_8000_0000_0000` on x86_64), leaving the lower half available for user space.

### Hot-Reload
The ability to replace or update kernel modules at runtime without requiring a full system restart. Helix supports hot-reload for scheduler and other extensible components.

### Microkernel Architecture
An OS architecture where minimal functionality runs in kernel mode, with most services implemented as user-space processes. Helix uses a hybrid approach with modular kernel-space components.

### Orchestrator
The Helix component responsible for managing the lifecycle of modules and coordinating interactions between kernel subsystems. Includes panic handling, lifecycle management, and resource brokering.

### Resource Broker
A Helix component that allocates and manages system resources (memory, I/O ports, IRQs) on behalf of modules, ensuring fair distribution and preventing conflicts.

---

## Memory Management

### Buddy Allocator
A memory allocation algorithm that divides memory into power-of-two sized blocks. When a block is needed, larger blocks are split ("buddied") until an appropriately sized block is available. Merging happens when both buddies are free.

```
                Order 3 (8 pages)
        ┌───────────────────────────────┐
        │                               │
        └───────────────────────────────┘
                      ↓ Split
        Order 2 (4 pages)    Order 2 (4 pages)
    ┌───────────────┐    ┌───────────────┐
    │   Buddy A     │    │   Buddy B     │
    └───────────────┘    └───────────────┘
```

### Copy-on-Write (CoW)
An optimization technique where shared memory pages are only duplicated when one of the sharers attempts to write. Until then, all processes share the same physical page.

### Demand Paging
A technique where pages are only loaded into memory when accessed, triggering a page fault that loads the page from disk. This reduces memory usage for large mappings.

### Frame
A fixed-size unit of physical memory, typically 4 KiB on x86_64. Also called a "physical page" or "page frame."

### Frame Allocator
The kernel component responsible for managing physical memory frames. Helix uses a bitmap-based allocator for simple allocation and a buddy allocator for more complex needs.

### Higher-Half Offset
The constant offset added to physical addresses to calculate their corresponding virtual addresses in the higher-half mapping. In Helix: `HIGHER_HALF_OFFSET = 0xFFFF_8000_0000_0000`.

### Huge Page
A memory page larger than the standard 4 KiB. x86_64 supports 2 MiB and 1 GiB huge pages. They reduce TLB pressure but increase internal fragmentation.

### Identity Mapping
A virtual memory mapping where virtual addresses equal physical addresses (VA = PA). Used during early boot before full virtual memory is established.

### Page
A fixed-size unit of virtual memory, typically 4 KiB on x86_64.

### Page Fault
A CPU exception (#PF, vector 14) triggered when accessing a page that is not present, has insufficient permissions, or violates protection rules.

### Page Table
A hierarchical data structure used by the CPU to translate virtual addresses to physical addresses. x86_64 uses 4 levels: PML4 → PDP → PD → PT.

### Slab Allocator
A memory allocator optimized for frequently allocated fixed-size objects. It maintains pre-allocated "slabs" of objects to reduce allocation overhead.

### TLB (Translation Lookaside Buffer)
A CPU cache for virtual-to-physical address translations. When page tables change, the TLB must be flushed to ensure correct translations.

---

## Scheduling

### Batch Task
A task type in DIS for CPU-intensive, non-interactive work. Batch tasks receive larger time slices but may be delayed in favor of interactive tasks.

### Background Task
The lowest priority task type in DIS, intended for maintenance operations that should only run when the system is otherwise idle.

### Context Switch
The process of saving one task's state and loading another's, allowing multiple tasks to share a single CPU. Helix preserves: registers, stack pointer, instruction pointer, and flags.

### Deadline
In realtime scheduling, the time by which a task must complete its work. DIS uses EDF (Earliest Deadline First) for tasks with deadlines.

### EDF (Earliest Deadline First)
A scheduling algorithm that always runs the task with the closest deadline. Optimal for periodic realtime tasks when total utilization ≤ 100%.

### Fair Scheduling
A scheduling policy that attempts to give each task an equal share of CPU time proportional to its weight. Similar to CFS in Linux.

### Interactive Task
A task type in DIS for user-facing operations requiring low latency. These tasks get high priority and shorter but more frequent time slices.

### Intent
A high-level description of a task's behavioral requirements. Includes class (realtime, interactive, batch, background), constraints, and QoS parameters.

### Priority Inversion
A pathological scheduling condition where a high-priority task waits for a low-priority task that is preempted by a medium-priority task. DIS uses priority inheritance to mitigate this.

### Quantum
The maximum continuous CPU time allocated to a task before it can be preempted. Also called a "time slice."

### Realtime Task
A task type in DIS with strict timing requirements. Realtime tasks have deadlines and are scheduled using EDF.

### Runqueue
A queue of tasks ready to execute on a CPU. DIS maintains separate runqueues for each scheduling class.

### Scheduler Module
An external module implementing a scheduling policy that can be loaded into DIS. Enables custom scheduling without kernel modification.

### Task
The unit of scheduling in Helix. Contains execution context, state, intent, and resources.

### Task State
The current status of a task:
- **Ready**: Waiting in runqueue
- **Running**: Currently executing
- **Blocked**: Waiting for resource or event
- **Suspended**: Paused by request
- **Zombie**: Terminated, awaiting cleanup

---

## Filesystem

### B-Tree
A self-balancing tree data structure used by HelixFS for directory indexing and extent management. Provides O(log n) operations and is optimized for disk access patterns.

### Block
The basic unit of disk I/O, typically 4 KiB in HelixFS. Data is read and written in block-sized chunks.

### Block Device
An abstract interface for devices that read and write fixed-size blocks. Examples: disk drives, SSDs, RAM disks.

### Compression
Optional data compression in HelixFS. Supports LZ4 (fast) and Zstd (high ratio) algorithms.

### Copy-on-Write (CoW)
In HelixFS, a write strategy where modified blocks are written to new locations instead of overwriting existing data. Enables atomic updates and snapshots.

### Directory Entry (DirEntry)
A record mapping a filename to an inode number. HelixFS stores directory entries in B-tree indexed directories.

### Extent
A contiguous range of disk blocks. HelixFS uses extents instead of block pointers to efficiently represent large files.

```
Extent: start_block=1000, count=100
Represents: blocks 1000, 1001, 1002, ..., 1099
```

### Extent Tree
A B-tree structure storing file extents, enabling O(log n) lookup of the block containing a given file offset.

### Inode
A data structure containing a file's metadata (size, permissions, timestamps) and storage information (extents or inline data). Each file and directory has exactly one inode.

### Inline Data
Small file data stored directly in the inode rather than in separate blocks. Reduces I/O for very small files.

### Journal
A log of pending filesystem changes used for crash recovery. HelixFS journals metadata changes to ensure consistency after unexpected shutdown.

### Mount
The process of attaching a filesystem to the directory hierarchy. After mounting, files in the filesystem become accessible at the mount point.

### Snapshot
A point-in-time copy of the filesystem state. HelixFS uses CoW to create space-efficient snapshots.

### Superblock
The root metadata structure of a filesystem containing: magic number, version, configuration, root inode, and allocation state.

### VFS (Virtual File System)
An abstraction layer allowing different filesystem implementations (HelixFS, ext4, FAT) to provide a uniform interface.

---

## Hardware Abstraction

### ACPI (Advanced Configuration and Power Interface)
An open standard for hardware configuration, power management, and thermal control. Helix parses ACPI tables for hardware discovery.

### APIC (Advanced Programmable Interrupt Controller)
The modern interrupt controller on x86 systems. Includes Local APIC (per-CPU) and I/O APIC (for device interrupts).

### CR3
x86 control register containing the physical address of the top-level page table (PML4). Changing CR3 switches address spaces.

### GDT (Global Descriptor Table)
An x86 data structure defining memory segments. In 64-bit mode, primarily used for CPU privilege transitions and TSS.

### IDT (Interrupt Descriptor Table)
An x86 data structure mapping interrupt vectors to handler functions. Helix configures the IDT for exceptions and device interrupts.

### I/O APIC
The I/O interrupt controller that routes device interrupts to CPUs. Replaces the legacy 8259 PIC.

### IST (Interrupt Stack Table)
x86_64 feature allowing specific exceptions to switch to dedicated stacks. Used for double fault handling.

### Local APIC
The per-CPU interrupt controller handling IPIs and local timer interrupts.

### Long Mode
x86_64 64-bit operating mode. Enables 64-bit registers, larger virtual address space, and modern features.

### MMU (Memory Management Unit)
Hardware that translates virtual addresses to physical addresses using page tables.

### MSI (Message Signaled Interrupts)
A modern interrupt delivery mechanism where devices write to special memory addresses instead of using interrupt lines.

### PIC (Programmable Interrupt Controller)
Legacy 8259 interrupt controller. Helix disables it in favor of the APIC.

### TSS (Task State Segment)
x86 structure containing interrupt and exception stack pointers. Required even in 64-bit mode.

---

## Module System

### ABI (Application Binary Interface)
The low-level calling conventions and data layout used when modules interact. Helix uses `extern "C"` ABI for stability.

### Dependency Graph
A directed graph representing module dependencies. Used to determine load/unload order.

### Module Interface
A trait or set of functions that a module implements and exports. Defines the module's capabilities.

### Module Registry
The central database of loaded modules, their interfaces, and their states.

### Symbol Resolution
The process of finding the address of a function or data symbol during module loading.

### Version Compatibility
Rules determining which module versions can work together. Helix uses semantic versioning for compatibility checks.

---

## Inter-Process Communication

### Channel
A typed, unidirectional communication link between two endpoints. Supports synchronous and asynchronous message passing.

### Event Bus
A pub/sub system where producers publish events and subscribers receive matching events. Used for decoupled communication.

### Message Router
The IPC component that directs messages between channels based on routing rules.

---

## Boot Process

### Boot Stub
Minimal assembly code that transitions from bootloader environment to kernel initialization.

### GRUB
GRand Unified Bootloader - the bootloader used to load Helix. Supports Multiboot2 protocol.

### Kernel Entry
The first Rust function (`kernel_main`) called after boot assembly completes.

### Multiboot2
A boot protocol defining how bootloaders pass information to kernels. Provides memory map, command line, and boot module information.

### Protected Mode
x86 32-bit operating mode. Used briefly during boot before transitioning to Long Mode.

---

## Security

### Capability
A transferable token representing the right to perform specific operations. More flexible than traditional permission bits.

### Ring
x86 privilege level (0-3). Kernel runs in Ring 0; user space runs in Ring 3.

### SMEP (Supervisor Mode Execution Prevention)
CPU feature preventing Ring 0 from executing code in user-space pages.

### SMAP (Supervisor Mode Access Prevention)
CPU feature preventing Ring 0 from reading/writing user-space pages except when explicitly enabled.

---

## Debugging

### Breakpoint
A location in code where execution pauses for debugging. Set using `int3` instruction or GDB.

### Panic
An unrecoverable error condition causing kernel shutdown. Helix's panic handler prints diagnostics before halting.

### Serial Console
Text output via COM1 serial port. Primary debugging output for kernel development.

### Stack Trace
A list of function call frames from current execution point to entry. Used to diagnose panics.

### Triple Fault
Three consecutive CPU faults without successful handling, causing system reset. Usually indicates kernel bug.

---

## General OS Terms

### Kernel
The core of an operating system, running in privileged mode and managing hardware and resources.

### Kernel Space
The virtual memory region reserved for kernel code and data. Protected from user-space access.

### Process
An instance of a program execution with its own address space and resources.

### System Call
The interface through which user-space programs request kernel services.

### User Space
The virtual memory region where user programs execute with restricted privileges.

---

## Rust-Specific Terms

### `#![no_std]`
A crate attribute indicating the code runs without the Rust standard library. Required for kernel development.

### Alloc Crate
The `alloc` crate providing heap allocation types (Box, Vec, String) without the full std library.

### Global Allocator
The memory allocator used by the `alloc` crate. Helix provides a custom `#[global_allocator]`.

### Never Type (`!`)
A type representing computations that never complete (infinite loops, panics). Used for diverging functions.

### Unsafe
Rust keyword marking code that the compiler cannot verify for memory safety. Required for hardware access.

---

## Abbreviations

| Abbrev. | Full Form |
|---------|-----------|
| ACPI | Advanced Configuration and Power Interface |
| APIC | Advanced Programmable Interrupt Controller |
| CoW | Copy-on-Write |
| CPU | Central Processing Unit |
| DIS | Differentiated Intent Scheduler |
| EDF | Earliest Deadline First |
| GDT | Global Descriptor Table |
| HAL | Hardware Abstraction Layer |
| IDT | Interrupt Descriptor Table |
| I/O | Input/Output |
| IPC | Inter-Process Communication |
| IRQ | Interrupt Request |
| ISR | Interrupt Service Routine |
| IST | Interrupt Stack Table |
| MMU | Memory Management Unit |
| MSI | Message Signaled Interrupts |
| NMI | Non-Maskable Interrupt |
| OS | Operating System |
| PA | Physical Address |
| PD | Page Directory |
| PDP | Page Directory Pointer |
| PF | Page Fault |
| PIC | Programmable Interrupt Controller |
| PML4 | Page Map Level 4 |
| PT | Page Table |
| QEMU | Quick EMUlator |
| RAM | Random Access Memory |
| RDTSC | Read Time-Stamp Counter |
| RSP | Register Stack Pointer |
| SMAP | Supervisor Mode Access Prevention |
| SMEP | Supervisor Mode Execution Prevention |
| SMP | Symmetric Multi-Processing |
| TLB | Translation Lookaside Buffer |
| TSS | Task State Segment |
| VA | Virtual Address |
| VFS | Virtual File System |
| VM | Virtual Machine / Virtual Memory |

---

## See Also

- [Architecture Overview](../architecture/OVERVIEW.md)
- [Memory Model](../architecture/MEMORY_MODEL.md)
- [API Reference](../api/)
- [FAQ](FAQ.md)

---

<div align="center">

📖 *Understanding the language is the first step to mastery* 📖

</div>
