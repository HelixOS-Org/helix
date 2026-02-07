# Credits

> *"Complex systems are not built — they are grown. One interface at a time,*
> *one invariant at a time, one commit at a time, by people who understand*
> *that the line they are writing will be load-bearing for everything above it."*

Helix OS exists because the initial architecture was forged for a target with
no standard library, no runtime, and no safety net. A bare `x86_64-unknown-none`
entry point, a serial port, and a decision to keep going until the system could
observe its own execution and correct its own trajectory.

This file records who laid the pavement.

---

## The Registry

```
┌────────────────────────────────────────────────────────────────────────────┐
│  HELIX OS — CONTRIBUTOR REGISTRY                                          │
│  Entries appear in the order their first commit reached the trunk.        │
└────────────────────────────────────────────────────────────────────────────┘
```

| Architect | Designation | The Contribution (Foundation) |
|:----------|:------------|:------------------------------|
| **Helix** | The Prime Architect | Defined the HAL trait boundaries for x86_64, aarch64, and riscv64. Designed the 5-phase init framework with DAG dependency resolution and rollback chains. Implemented the initial NEXUS feedback loops — decision trees, anomaly thresholds, and the self-heal watchdog. Built the boot sequence for Limine and UEFI. Wrote the KASLR relocation engine. Authored HelixFS (CoW, B+tree, journal). Created the module hot-reload system with ABI versioning. Established the interfaces and safety contracts that the rest of the system depends on. |
| | | |
| | | |
| | | |
| | | |

---

## The Open Horizon

The table above has empty rows. They are not decoration — they are
load-bearing structure, waiting for the engineers who will fill them.

What exists today is a set of **interfaces and frameworks**. What is
missing is the work that turns frameworks into a running system:

```
┌──────────────────────────────────────────────────────────────────────┐
│  OPEN SUBSYSTEMS — CONTRIBUTIONS NEEDED                              │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Schedulers       CFS, EDF, real-time FIFO — the traits are          │
│                   defined, the implementations are not.              │
│                                                                      │
│  Allocators       Slab, TLSF, zone allocator. The memory             │
│                   framework provides Frame and MemoryZone.           │
│                   Plug in the algorithms.                            │
│                                                                      │
│  Drivers          VirtIO block/net/console, PS/2, framebuffer.       │
│                   The HAL abstracts the hardware. Write the          │
│                   device-specific code.                              │
│                                                                      │
│  Userspace        A shell. Core utilities. Dynamic linking.          │
│                   The ELF loader and syscall table exist.            │
│                   Build what runs on top.                            │
│                                                                      │
│  Networking       TCP/IP stack, socket layer. The UEFI crate         │
│                   has PXE/TFTP. The kernel has nothing yet.          │
│                                                                      │
│  Graphics         Lumina has 197K lines of rendering framework.      │
│                   No compositor. No window manager. No desktop.      │
│                                                                      │
│  Testing          Fuzzing harnesses, integration tests,              │
│                   multi-arch CI. Coverage is low. Raise it.          │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

None of this is a backlog assigned to someone else. It is open ground.
Pick a subsystem. Read the trait definitions in the relevant crate.
Write an implementation that satisfies them. Submit a pull request.

```
Fork.     Read the traits.     Implement.     PR.
```

Your name enters the registry the moment your code enters the trunk.
That is the only credential this project recognizes.

---

<div align="center">

*End of Line.*

</div>
