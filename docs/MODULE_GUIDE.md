# Helix Module Development Guide

This guide explains how to develop modules for the Helix OS Framework.

## What is a Module?

A module is a self-contained unit of functionality that can be:
- Loaded at boot time (static linking)
- Loaded at runtime (dynamic loading)
- Hot-reloaded without system restart
- Run in kernel space or user space

Examples of modules:
- Schedulers (round-robin, CFS, real-time)
- Memory allocators (buddy, slab, TLSF)
- File systems (ramfs, ext2, fat32)
- Device drivers (keyboard, serial, virtio)

## Module Structure

### Directory Layout

```
modules_impl/
└── schedulers/
    └── my_scheduler/
        ├── Cargo.toml
        └── src/
            ├── lib.rs          # Module entry point
            ├── scheduler.rs    # Main implementation
            └── config.rs       # Configuration
```

### Cargo.toml

```toml
[package]
name = "helix-scheduler-mine"
version.workspace = true
edition.workspace = true

[dependencies]
helix-modules = { workspace = true }
helix-execution = { workspace = true }
helix-hal = { workspace = true }
log = { workspace = true }
spin = { workspace = true }

[features]
default = []
```

### Module Entry Point (lib.rs)

```rust
#![no_std]
extern crate alloc;

use helix_modules::{
    Module, ModuleContext, ModuleId, ModuleVersion,
    ModuleFlags, ModuleMetadata, ModuleDependency, define_module,
};

/// Module metadata (constant, used for discovery)
pub static METADATA: ModuleMetadata = ModuleMetadata {
    id: ModuleId::from_raw(0x12345678),
    name: "My Scheduler",
    version: ModuleVersion::new(1, 0, 0),
    description: "A custom scheduler implementation",
    author: "Your Name",
    license: "MIT",
    flags: ModuleFlags::KERNEL_SPACE,
};

/// The module struct
pub struct MySchedulerModule {
    // Your state here
}

impl Module for MySchedulerModule {
    fn metadata(&self) -> &ModuleMetadata {
        &METADATA
    }

    fn init(&mut self, ctx: &ModuleContext) -> Result<(), &'static str> {
        log::info!("Initializing My Scheduler");
        // Initialization code
        Ok(())
    }

    fn start(&mut self) -> Result<(), &'static str> {
        log::info!("Starting My Scheduler");
        // Register with the subsystem
        Ok(())
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        log::info!("Stopping My Scheduler");
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), &'static str> {
        log::info!("Cleaning up My Scheduler");
        Ok(())
    }

    fn dependencies(&self) -> &[ModuleDependency] {
        &[] // List dependencies here
    }

    fn provides(&self) -> &[&str] {
        &["scheduler"] // Interfaces this module provides
    }

    // Optional: Hot reload support
    fn save_state(&self) -> Option<alloc::vec::Vec<u8>> {
        // Serialize state for hot reload
        None
    }

    fn restore_state(&mut self, _state: &[u8]) -> Result<(), &'static str> {
        // Restore state after hot reload
        Ok(())
    }
}

// Required: Module entry point macro
define_module!(MySchedulerModule);
```

## Module Lifecycle

```
                    ┌─────────────┐
                    │   Created   │
                    └──────┬──────┘
                           │ init()
                           ▼
                    ┌─────────────┐
                    │ Initialized │
                    └──────┬──────┘
                           │ start()
                           ▼
                    ┌─────────────┐
            ┌───────│   Running   │◄──────┐
            │       └──────┬──────┘       │
            │              │              │
    save_state()      stop()        restore_state()
            │              │              │
            ▼              ▼              │
     ┌──────────┐   ┌─────────────┐       │
     │  State   │   │   Stopped   │───────┘
     │  Saved   │   └──────┬──────┘ start()
     └──────────┘          │ cleanup()
                           ▼
                    ┌─────────────┐
                    │  Unloaded   │
                    └─────────────┘
```

## Implementing a Scheduler

To implement a scheduler, you need to implement the `Scheduler` trait:

```rust
use helix_execution::scheduler::{
    Scheduler, SchedulableThread, SchedulerStats,
    Priority, SchedulingPolicy,
};
use helix_execution::{ThreadId, ExecResult};

pub struct MyScheduler {
    // Per-CPU run queues, etc.
}

impl Scheduler for MyScheduler {
    fn name(&self) -> &'static str {
        "My Scheduler"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn init(&mut self, cpu_count: usize) -> ExecResult<()> {
        // Initialize per-CPU data structures
        Ok(())
    }

    fn pick_next(&self, cpu: usize) -> Option<ThreadId> {
        // Return the next thread to run on this CPU
        None
    }

    fn add_thread(&self, thread: SchedulableThread) -> ExecResult<()> {
        // Add a new thread to the scheduler
        Ok(())
    }

    fn remove_thread(&self, id: ThreadId) -> ExecResult<()> {
        // Remove a thread from the scheduler
        Ok(())
    }

    fn thread_ready(&self, id: ThreadId) -> ExecResult<()> {
        // Mark thread as ready to run
        Ok(())
    }

    fn thread_block(&self, id: ThreadId) -> ExecResult<()> {
        // Mark thread as blocked
        Ok(())
    }

    fn yield_thread(&self, cpu: usize) {
        // Current thread voluntarily yields
    }

    fn tick(&self, cpu: usize) {
        // Timer tick - handle time slicing
    }

    fn set_priority(&self, id: ThreadId, priority: Priority) -> ExecResult<()> {
        Ok(())
    }

    fn get_priority(&self, id: ThreadId) -> Option<Priority> {
        None
    }

    fn needs_reschedule(&self, cpu: usize) -> bool {
        false
    }

    fn stats(&self) -> SchedulerStats {
        SchedulerStats::default()
    }
}
```

## Implementing an Allocator

For memory allocators, implement the `PhysicalAllocator` trait:

```rust
use helix_memory::{Frame, MemResult, MemoryZone};
use helix_memory::physical::{
    PhysicalAllocator, PhysicalRegion, AllocatorStats,
};
use helix_hal::PageSize;

pub struct MyAllocator {
    // Your allocator state
}

impl PhysicalAllocator for MyAllocator {
    fn name(&self) -> &'static str {
        "My Allocator"
    }

    fn init(&mut self, regions: &[PhysicalRegion]) -> MemResult<()> {
        // Initialize with available memory regions
        Ok(())
    }

    fn allocate(&self, size: PageSize) -> MemResult<Frame> {
        // Allocate a frame
        Err(helix_memory::MemError::OutOfMemory)
    }

    fn allocate_contiguous(&self, count: usize, size: PageSize) -> MemResult<Frame> {
        // Allocate contiguous frames
        Err(helix_memory::MemError::OutOfMemory)
    }

    fn allocate_zone(&self, size: PageSize, zone: MemoryZone) -> MemResult<Frame> {
        // Allocate from specific zone
        Err(helix_memory::MemError::OutOfMemory)
    }

    fn deallocate(&self, frame: Frame) -> MemResult<()> {
        // Free a frame
        Ok(())
    }

    fn free_frames(&self) -> usize {
        0
    }

    fn total_frames(&self) -> usize {
        0
    }

    fn stats(&self) -> AllocatorStats {
        AllocatorStats::default()
    }
}
```

## Hot Reload Support

To support hot reloading, your module must:

1. Implement `save_state()` to serialize current state
2. Implement `restore_state()` to deserialize state
3. Be stateless or have serializable state

```rust
impl Module for MySchedulerModule {
    fn save_state(&self) -> Option<Vec<u8>> {
        // Serialize your run queues, thread states, etc.
        let state = SchedulerState {
            threads: self.get_all_threads(),
            current: self.get_current_threads(),
        };
        
        // Use a simple serialization format
        Some(serialize(&state))
    }

    fn restore_state(&mut self, state: &[u8]) -> Result<(), &'static str> {
        let state: SchedulerState = deserialize(state)
            .map_err(|_| "Failed to deserialize state")?;
        
        // Restore run queues
        for thread in state.threads {
            self.add_thread_internal(thread)?;
        }
        
        Ok(())
    }
}
```

## Configuration

Modules can receive configuration through `ModuleContext`:

```rust
fn init(&mut self, ctx: &ModuleContext) -> Result<(), &'static str> {
    // Get configuration values
    let time_slice: u64 = ctx.get_config("time_slice_ms")
        .unwrap_or(10);
    
    let priority_levels: usize = ctx.get_config("priority_levels")
        .unwrap_or(140);
    
    self.config = Config {
        time_slice_ns: time_slice * 1_000_000,
        priority_levels,
    };
    
    Ok(())
}
```

## Testing Modules

Use the Helix testing framework:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use helix_modules::testing::*;

    #[test]
    fn test_scheduler_init() {
        let mut scheduler = MyScheduler::new();
        assert!(scheduler.init(4).is_ok());
    }

    #[test]
    fn test_thread_scheduling() {
        let mut scheduler = MyScheduler::new();
        scheduler.init(1).unwrap();
        
        let thread = SchedulableThread::new(
            ThreadId::new(),
            ProcessId::kernel(),
            Priority::DEFAULT,
        );
        
        scheduler.add_thread(thread.clone()).unwrap();
        
        assert_eq!(scheduler.pick_next(0), Some(thread.id));
    }
}
```

## Best Practices

1. **No Panics**: Use `Result` for error handling
2. **No Blocking**: Kernel modules must not block
3. **Lock Ordering**: Document and follow lock ordering to prevent deadlocks
4. **Minimal State**: Keep module state minimal for hot reload
5. **Logging**: Use `log` crate for debugging
6. **Documentation**: Document all public APIs

## Common Patterns

### Per-CPU Data

```rust
use alloc::vec::Vec;

struct PerCpuData {
    queue: Mutex<VecDeque<ThreadId>>,
    current: RwLock<Option<ThreadId>>,
}

struct MyScheduler {
    cpu_data: Vec<PerCpuData>,
}

impl MyScheduler {
    fn init(&mut self, cpu_count: usize) {
        self.cpu_data = (0..cpu_count)
            .map(|_| PerCpuData::new())
            .collect();
    }
}
```

### Statistics Tracking

```rust
use core::sync::atomic::{AtomicU64, Ordering};

struct Stats {
    allocations: AtomicU64,
    deallocations: AtomicU64,
}

impl Stats {
    fn record_alloc(&self) {
        self.allocations.fetch_add(1, Ordering::Relaxed);
    }
    
    fn get(&self) -> (u64, u64) {
        (
            self.allocations.load(Ordering::Relaxed),
            self.deallocations.load(Ordering::Relaxed),
        )
    }
}
```

## See Also

- [Architecture Overview](ARCHITECTURE.md)
- [Project Structure](PROJECT_STRUCTURE.md)
- [API Reference](API.md)
