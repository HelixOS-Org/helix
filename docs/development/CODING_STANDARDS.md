# Helix OS Coding Standards

<div align="center">

📐 **Rust Coding Standards & Best Practices**

*Ensuring consistent, safe, and maintainable code*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [Naming Conventions](#2-naming-conventions)
3. [Code Organization](#3-code-organization)
4. [Documentation Standards](#4-documentation-standards)
5. [Error Handling](#5-error-handling)
6. [Unsafe Code Guidelines](#6-unsafe-code-guidelines)
7. [Performance Guidelines](#7-performance-guidelines)
8. [Testing Standards](#8-testing-standards)
9. [Formatting Rules](#9-formatting-rules)
10. [Anti-Patterns](#10-anti-patterns)

---

## 1. Overview

### 1.1 Core Principles

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        HELIX CODING PRINCIPLES                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐       │
│  │    SAFETY       │     │   CLARITY       │     │  PERFORMANCE    │       │
│  │                 │     │                 │     │                 │       │
│  │  • Minimize     │     │  • Readable     │     │  • Zero-cost    │       │
│  │    unsafe       │     │    code         │     │    abstractions │       │
│  │  • Document     │     │  • Self-        │     │  • Avoid        │       │
│  │    invariants   │     │    documenting  │     │    allocations  │       │
│  │  • Fail fast    │     │  • Clear intent │     │  • Cache-aware  │       │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘       │
│                                                                             │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐       │
│  │ MAINTAINABILITY │     │  MODULARITY     │     │ TESTABILITY     │       │
│  │                 │     │                 │     │                 │       │
│  │  • Consistent   │     │  • Small        │     │  • Unit tests   │       │
│  │    style        │     │    interfaces   │     │  • Integration  │       │
│  │  • DRY code     │     │  • Loose        │     │  • Property     │       │
│  │  • SOLID        │     │    coupling     │     │    tests        │       │
│  └─────────────────┘     └─────────────────┘     └─────────────────┘       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Rust-Specific Considerations

```rust
// Helix is a no_std kernel - these rules apply:

#![no_std]           // No standard library
#![no_main]          // No main function
#![feature(...)]     // Nightly features as needed

// Always available:
use core::*;          // Core library

// With alloc feature:
extern crate alloc;
use alloc::{vec::Vec, string::String, boxed::Box};

// Never available:
// use std::*;        // Standard library
// std::fs, std::net  // OS-dependent features
```

---

## 2. Naming Conventions

### 2.1 General Rules

| Item | Convention | Example |
|------|-----------|---------|
| Crates | `snake_case` (with hyphen in Cargo.toml) | `helix-core`, `helix_core` |
| Modules | `snake_case` | `task_manager`, `memory_allocator` |
| Types | `PascalCase` | `TaskManager`, `MemoryAllocator` |
| Traits | `PascalCase` (adjective/noun) | `Scheduler`, `Allocatable` |
| Functions | `snake_case` | `create_task`, `allocate_page` |
| Methods | `snake_case` | `task.get_state()`, `alloc.free()` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_TASKS`, `PAGE_SIZE` |
| Statics | `SCREAMING_SNAKE_CASE` | `GLOBAL_ALLOCATOR` |
| Variables | `snake_case` | `task_count`, `current_page` |
| Type parameters | `PascalCase` (single letter OK) | `T`, `Item`, `Error` |
| Lifetimes | `lowercase` (short) | `'a`, `'ctx`, `'static` |

### 2.2 Naming Patterns

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                           NAMING PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

// ─── CONSTRUCTORS ───────────────────────────────────────────────────────────

impl Task {
    // Primary constructor
    pub fn new(name: &str) -> Self { ... }
    
    // Alternative constructors
    pub fn with_priority(name: &str, priority: u8) -> Self { ... }
    pub fn from_config(config: TaskConfig) -> Self { ... }
    
    // Fallible constructors
    pub fn try_new(name: &str) -> Result<Self, Error> { ... }
}

// ─── GETTERS & SETTERS ──────────────────────────────────────────────────────

impl Task {
    // Getters: noun or "get_" prefix for computed values
    pub fn name(&self) -> &str { &self.name }
    pub fn state(&self) -> TaskState { self.state }
    pub fn get_priority(&self) -> u8 { self.compute_priority() }
    
    // Setters: "set_" prefix
    pub fn set_state(&mut self, state: TaskState) { self.state = state; }
    pub fn set_priority(&mut self, priority: u8) { self.priority = priority; }
}

// ─── PREDICATES ─────────────────────────────────────────────────────────────

impl Task {
    // Boolean methods: "is_", "has_", "can_", "should_"
    pub fn is_running(&self) -> bool { self.state == TaskState::Running }
    pub fn has_children(&self) -> bool { !self.children.is_empty() }
    pub fn can_schedule(&self) -> bool { self.state == TaskState::Ready }
    pub fn should_preempt(&self) -> bool { self.time_slice == 0 }
}

// ─── CONVERSIONS ────────────────────────────────────────────────────────────

impl Task {
    // Cheap conversions: "as_"
    pub fn as_ref(&self) -> &TaskInner { &self.inner }
    pub fn as_mut(&mut self) -> &mut TaskInner { &mut self.inner }
    pub fn as_ptr(&self) -> *const Task { self as *const _ }
    
    // Expensive conversions: "to_"
    pub fn to_string(&self) -> String { format!("{:?}", self) }
    pub fn to_vec(&self) -> Vec<u8> { self.serialize() }
    
    // Type conversions: "into_"
    pub fn into_inner(self) -> TaskInner { self.inner }
    pub fn into_boxed(self) -> Box<Self> { Box::new(self) }
}

// ─── ITERATION ──────────────────────────────────────────────────────────────

impl TaskList {
    // Iterator methods
    pub fn iter(&self) -> Iter<'_, Task> { self.tasks.iter() }
    pub fn iter_mut(&mut self) -> IterMut<'_, Task> { self.tasks.iter_mut() }
    pub fn into_iter(self) -> IntoIter<Task> { self.tasks.into_iter() }
}

// ─── ACTIONS ────────────────────────────────────────────────────────────────

impl Scheduler {
    // Action verbs
    pub fn schedule(&mut self) -> Option<TaskId> { ... }
    pub fn spawn(&mut self, task: Task) -> TaskId { ... }
    pub fn kill(&mut self, id: TaskId) -> Result<()> { ... }
    pub fn suspend(&mut self, id: TaskId) -> Result<()> { ... }
    pub fn resume(&mut self, id: TaskId) -> Result<()> { ... }
}

// ─── UNSAFE VARIANTS ────────────────────────────────────────────────────────

impl Memory {
    // Safe version
    pub fn read(&self, addr: VirtAddr) -> Result<u8> { ... }
    
    // Unsafe version: "_unchecked" suffix
    pub unsafe fn read_unchecked(&self, addr: VirtAddr) -> u8 { ... }
}
```

### 2.3 Abbreviations

```rust
// Common abbreviations (acceptable)
addr    // address
alloc   // allocator/allocation
buf     // buffer
cfg     // configuration
ctx     // context
fd      // file descriptor
idx     // index
len     // length
msg     // message
ptr     // pointer
ref     // reference
req     // request
res     // result/response
src     // source
dst     // destination

// Domain-specific (acceptable in context)
tcb     // Task Control Block
pcb     // Process Control Block
ipc     // Inter-Process Communication
irq     // Interrupt Request
mmu     // Memory Management Unit
pte     // Page Table Entry
gdt     // Global Descriptor Table
idt     // Interrupt Descriptor Table

// Avoid unclear abbreviations
// tsk     // Use "task"
// mgr     // Use "manager"
// proc    // Use "process" or "processor"
// tmp     // Use "temp" or descriptive name
```

---

## 3. Code Organization

### 3.1 File Structure

```rust
//! Module-level documentation
//!
//! This module provides...
//!
//! # Examples
//!
//! ```
//! use helix_core::task::Task;
//! let task = Task::new("example");
//! ```

// ═══════════════════════════════════════════════════════════════════════════
// IMPORTS (grouped and ordered)
// ═══════════════════════════════════════════════════════════════════════════

// Standard library / core (none for no_std)
use core::sync::atomic::{AtomicU64, Ordering};
use core::mem::MaybeUninit;

// Alloc crate
use alloc::{boxed::Box, string::String, vec::Vec};

// External crates (alphabetical)
use spin::Mutex;

// Crate-level imports
use crate::error::{Error, Result};
use crate::memory::VirtAddr;

// Parent/sibling modules
use super::scheduler::Scheduler;

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/// Maximum number of tasks
pub const MAX_TASKS: usize = 1024;

/// Default time slice in nanoseconds
const DEFAULT_TIME_SLICE: u64 = 10_000_000;

// ═══════════════════════════════════════════════════════════════════════════
// TYPE ALIASES
// ═══════════════════════════════════════════════════════════════════════════

/// Task result type
pub type TaskResult<T> = Result<T, TaskError>;

// ═══════════════════════════════════════════════════════════════════════════
// TRAITS
// ═══════════════════════════════════════════════════════════════════════════

/// Trait for schedulable entities
pub trait Schedulable {
    fn priority(&self) -> u8;
    fn time_slice(&self) -> u64;
}

// ═══════════════════════════════════════════════════════════════════════════
// ENUMS
// ═══════════════════════════════════════════════════════════════════════════

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

// ═══════════════════════════════════════════════════════════════════════════
// STRUCTS
// ═══════════════════════════════════════════════════════════════════════════

/// Task Control Block
#[derive(Debug)]
pub struct Task {
    id: TaskId,
    name: String,
    state: TaskState,
    priority: u8,
}

// ═══════════════════════════════════════════════════════════════════════════
// IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

impl Task {
    /// Create a new task
    pub fn new(name: &str) -> Self {
        Self {
            id: TaskId::new(),
            name: name.into(),
            state: TaskState::Ready,
            priority: 128,
        }
    }
}

impl Schedulable for Task {
    fn priority(&self) -> u8 {
        self.priority
    }
    
    fn time_slice(&self) -> u64 {
        DEFAULT_TIME_SLICE
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Create and schedule a task
pub fn spawn_task(name: &str) -> TaskResult<TaskId> {
    let task = Task::new(name);
    // ...
    Ok(task.id)
}

// ═══════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_creation() {
        let task = Task::new("test");
        assert_eq!(task.state, TaskState::Ready);
    }
}
```

### 3.2 Module Organization

```
src/
├── lib.rs              # Crate root
├── error.rs            # Error types (single file)
├── task/               # Task module (directory)
│   ├── mod.rs          # Module root, re-exports
│   ├── types.rs        # Type definitions
│   ├── manager.rs      # TaskManager implementation
│   └── scheduler.rs    # Scheduling logic
├── memory/             # Memory module
│   ├── mod.rs
│   ├── allocator.rs
│   ├── page_table.rs
│   └── region.rs
└── ipc/                # IPC module
    ├── mod.rs
    ├── channel.rs
    └── message.rs
```

```rust
// In task/mod.rs
//! Task management module

mod types;
mod manager;
mod scheduler;

// Re-export public API
pub use types::{Task, TaskId, TaskState};
pub use manager::TaskManager;
pub use scheduler::{Scheduler, SchedulingPolicy};

// Keep implementation details private
// (types::internal, manager::implementation details)
```

---

## 4. Documentation Standards

### 4.1 Documentation Requirements

```rust
/// Short one-line summary ending with a period.
///
/// Longer description that can span multiple paragraphs.
/// Explains the purpose, behavior, and any important details.
///
/// # Type Parameters
///
/// * `T` - Description of the type parameter
///
/// # Arguments
///
/// * `name` - The name of the task (must not be empty)
/// * `config` - Configuration options for the task
///
/// # Returns
///
/// Returns a new `Task` instance configured with the given parameters.
///
/// # Errors
///
/// Returns [`TaskError::InvalidName`] if `name` is empty.
/// Returns [`TaskError::InvalidConfig`] if config validation fails.
///
/// # Panics
///
/// Panics if the global allocator fails to allocate memory.
///
/// # Safety
///
/// (For unsafe functions only)
/// The caller must ensure:
/// - `ptr` is valid and properly aligned
/// - `ptr` is not used after calling this function
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use helix_core::task::Task;
///
/// let task = Task::new("worker", TaskConfig::default())?;
/// assert_eq!(task.name(), "worker");
/// ```
///
/// With custom configuration:
///
/// ```
/// use helix_core::task::{Task, TaskConfig};
///
/// let config = TaskConfig::builder()
///     .priority(10)
///     .stack_size(64 * 1024)
///     .build();
///
/// let task = Task::new("high-priority", config)?;
/// ```
///
/// # See Also
///
/// * [`TaskManager::spawn`] - For spawning tasks
/// * [`Scheduler`] - For scheduling tasks
pub fn new(name: &str, config: TaskConfig) -> Result<Task, TaskError> {
    // Implementation
}
```

### 4.2 Module Documentation

```rust
//! # Task Management
//!
//! This module provides task creation, scheduling, and lifecycle management.
//!
//! ## Overview
//!
//! The task system is the core of process management in Helix. It provides:
//!
//! - Task creation and destruction
//! - Priority-based scheduling
//! - Context switching
//! - Inter-task communication
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │   TaskManager   │
//! ├─────────────────┤
//! │   Scheduler     │
//! ├─────────────────┤
//! │     Tasks       │
//! └─────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```no_run
//! use helix_core::task::{TaskManager, Intent};
//!
//! let mut manager = TaskManager::new();
//! let task_id = manager.spawn("my_task", my_task_fn, Intent::batch())?;
//! manager.schedule();
//! ```
//!
//! ## Features
//!
//! - **Hot Reload**: Tasks can be updated without restart
//! - **Intent-Based**: Scheduling based on declared intent
//! - **Isolation**: Tasks are isolated from each other
//!
//! ## See Also
//!
//! - [Architecture Overview](../architecture/OVERVIEW.md)
//! - [Scheduler API](../api/SCHEDULER.md)
```

### 4.3 Inline Comments

```rust
impl Scheduler {
    pub fn schedule(&mut self) -> Option<TaskId> {
        // Fast path: check if current task can continue
        if let Some(current) = self.current_task() {
            if current.time_slice > 0 {
                return Some(current.id);
            }
        }
        
        // Slow path: find next task to run
        // We check queues in priority order: RT > Interactive > Batch > Background
        
        // Real-time tasks use EDF (Earliest Deadline First)
        if let Some(task) = self.rt_queue.pop_earliest() {
            return Some(task.id);
        }
        
        // Interactive tasks get priority boost based on wait time
        // This prevents starvation while maintaining responsiveness
        if let Some(task) = self.interactive_queue.pop_highest() {
            // Update boost: longer wait = higher boost
            let boost = self.calculate_boost(task.wait_time);
            task.dynamic_priority = task.base_priority.saturating_sub(boost);
            return Some(task.id);
        }
        
        // Batch and background use simple FIFO
        self.batch_queue.pop()
            .or_else(|| self.background_queue.pop())
            .map(|t| t.id)
    }
}
```

---

## 5. Error Handling

### 5.1 Error Type Design

```rust
/// Task-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskError {
    /// Task with given ID was not found
    NotFound(TaskId),
    
    /// Task name is invalid (empty or too long)
    InvalidName,
    
    /// Task is in wrong state for operation
    InvalidState {
        expected: TaskState,
        actual: TaskState,
    },
    
    /// Maximum task limit reached
    LimitReached,
    
    /// Resource allocation failed
    AllocationFailed,
    
    /// Permission denied for operation
    PermissionDenied,
}

impl core::fmt::Display for TaskError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "task {:?} not found", id),
            Self::InvalidName => write!(f, "invalid task name"),
            Self::InvalidState { expected, actual } => {
                write!(f, "expected state {:?}, got {:?}", expected, actual)
            }
            Self::LimitReached => write!(f, "task limit reached"),
            Self::AllocationFailed => write!(f, "allocation failed"),
            Self::PermissionDenied => write!(f, "permission denied"),
        }
    }
}

// Optional: implement Error trait when available
#[cfg(feature = "std")]
impl std::error::Error for TaskError {}

/// Result type alias for task operations
pub type TaskResult<T> = Result<T, TaskError>;
```

### 5.2 Error Handling Patterns

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                        ERROR HANDLING PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

// ─── PROPAGATION ────────────────────────────────────────────────────────────

pub fn complex_operation() -> TaskResult<Output> {
    // Use ? for propagation
    let task = get_task(id)?;
    let result = task.process()?;
    Ok(result)
}

// ─── MAPPING ERRORS ─────────────────────────────────────────────────────────

pub fn load_module(path: &str) -> Result<Module, ModuleError> {
    // Map one error type to another
    let data = read_file(path)
        .map_err(|e| ModuleError::IoError(e))?;
    
    parse_module(&data)
        .map_err(|e| ModuleError::ParseError(e))
}

// ─── ADDING CONTEXT ─────────────────────────────────────────────────────────

pub fn spawn_task(name: &str) -> TaskResult<TaskId> {
    let task = Task::new(name)
        .map_err(|e| {
            serial_println!("[ERROR] Failed to create task '{}': {}", name, e);
            e
        })?;
    
    Ok(task.id)
}

// ─── FALLBACK VALUES ────────────────────────────────────────────────────────

pub fn get_priority(task_id: TaskId) -> u8 {
    // Use default on error
    get_task(task_id)
        .map(|t| t.priority)
        .unwrap_or(DEFAULT_PRIORITY)
}

// ─── COLLECTING RESULTS ─────────────────────────────────────────────────────

pub fn spawn_all(names: &[&str]) -> TaskResult<Vec<TaskId>> {
    // Collect results, fail on first error
    names.iter()
        .map(|name| spawn_task(name))
        .collect()
}

pub fn spawn_all_best_effort(names: &[&str]) -> Vec<TaskId> {
    // Collect successes, ignore errors
    names.iter()
        .filter_map(|name| spawn_task(name).ok())
        .collect()
}

// ─── PANIC VS RESULT ────────────────────────────────────────────────────────

// Panic: for programming errors / invariant violations
pub fn get_task_unchecked(&self, id: TaskId) -> &Task {
    self.tasks.get(&id).expect("task must exist")
}

// Result: for expected failure modes
pub fn get_task(&self, id: TaskId) -> TaskResult<&Task> {
    self.tasks.get(&id).ok_or(TaskError::NotFound(id))
}

// ─── EARLY RETURNS ──────────────────────────────────────────────────────────

pub fn process_request(&mut self, req: Request) -> TaskResult<Response> {
    // Validate early
    if req.data.is_empty() {
        return Err(TaskError::InvalidRequest);
    }
    
    // Get required resources
    let task = self.get_task(req.task_id)?;
    
    // Check permissions
    if !self.can_access(req.caller, task) {
        return Err(TaskError::PermissionDenied);
    }
    
    // Do actual work
    let result = task.process(&req.data)?;
    
    Ok(Response::new(result))
}
```

---

## 6. Unsafe Code Guidelines

### 6.1 When to Use Unsafe

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                        WHEN UNSAFE IS APPROPRIATE
// ═══════════════════════════════════════════════════════════════════════════

// ✅ APPROPRIATE USES:

// 1. Dereferencing raw pointers (hardware access)
unsafe fn read_mmio(addr: usize) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

// 2. Calling unsafe functions (FFI, hardware)
unsafe fn call_firmware() {
    extern "C" { fn firmware_call(); }
    firmware_call();
}

// 3. Accessing mutable statics (global state)
static mut COUNTER: u32 = 0;
unsafe fn increment() { COUNTER += 1; }

// 4. Implementing unsafe traits
unsafe impl Send for MyType {}

// 5. Inline assembly
unsafe fn enable_interrupts() {
    core::arch::asm!("sti");
}

// ❌ INAPPROPRIATE USES:

// Don't use unsafe just to bypass borrow checker
// Don't use unsafe for "performance" without measurement
// Don't use unsafe when safe alternatives exist
```

### 6.2 Unsafe Code Patterns

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                        UNSAFE CODE PATTERNS
// ═══════════════════════════════════════════════════════════════════════════

// ─── MINIMIZE UNSAFE SCOPE ──────────────────────────────────────────────────

// Bad: large unsafe block
unsafe {
    let ptr = get_pointer();
    let value = *ptr;         // Unsafe operation
    let processed = process(value);  // Safe operation in unsafe block
    store(processed);         // Safe operation in unsafe block
}

// Good: minimal unsafe block
let ptr = get_pointer();
let value = unsafe { *ptr };  // Only the unsafe operation
let processed = process(value);
store(processed);

// ─── DOCUMENT SAFETY REQUIREMENTS ───────────────────────────────────────────

/// Reads a value from the specified address.
///
/// # Safety
///
/// The caller must ensure:
/// - `addr` points to valid, initialized memory
/// - `addr` is properly aligned for type `T`
/// - The memory at `addr` is not concurrently modified
/// - `addr` remains valid for the duration of the read
pub unsafe fn read_addr<T: Copy>(addr: *const T) -> T {
    // SAFETY: Caller guarantees addr is valid, aligned, and not concurrently modified
    core::ptr::read_volatile(addr)
}

// ─── ENCAPSULATE UNSAFETY ───────────────────────────────────────────────────

/// Safe wrapper around MMIO operations
pub struct MmioRegister {
    addr: usize,
}

impl MmioRegister {
    /// Create a new MMIO register.
    ///
    /// # Safety
    ///
    /// `addr` must be a valid MMIO address for the lifetime of this object.
    pub const unsafe fn new(addr: usize) -> Self {
        Self { addr }
    }
    
    /// Read the register value (safe interface)
    pub fn read(&self) -> u32 {
        // SAFETY: Constructor guarantees addr is valid MMIO
        unsafe { core::ptr::read_volatile(self.addr as *const u32) }
    }
    
    /// Write to the register (safe interface)
    pub fn write(&self, value: u32) {
        // SAFETY: Constructor guarantees addr is valid MMIO
        unsafe { core::ptr::write_volatile(self.addr as *mut u32, value) }
    }
}

// Usage is now safe:
const STATUS_REG: MmioRegister = unsafe { MmioRegister::new(0xFEE0_0000) };
let status = STATUS_REG.read();

// ─── USE TYPE SYSTEM FOR SAFETY ─────────────────────────────────────────────

/// Physical address (guarantees valid physical memory)
#[derive(Clone, Copy)]
pub struct PhysAddr(usize);

impl PhysAddr {
    /// Create a new physical address.
    ///
    /// Returns None if the address is not in valid physical memory range.
    pub fn new(addr: usize) -> Option<Self> {
        if addr < PHYS_MEM_END {
            Some(Self(addr))
        } else {
            None
        }
    }
    
    /// Convert to raw address (for hardware operations)
    pub fn as_usize(self) -> usize {
        self.0
    }
}

// Now hardware operations are safer:
pub fn map_page(phys: PhysAddr) {
    // We know phys is valid because PhysAddr::new validated it
    let raw = phys.as_usize();
    // ...
}
```

---

## 7. Performance Guidelines

### 7.1 General Principles

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                      PERFORMANCE GUIDELINES
// ═══════════════════════════════════════════════════════════════════════════

// ─── AVOID ALLOCATIONS IN HOT PATHS ─────────────────────────────────────────

// Bad: allocates on every call
fn process_event(event: Event) -> Vec<Action> {
    let mut actions = Vec::new();  // Allocates!
    // ...
    actions
}

// Good: reuse allocation
fn process_event(event: Event, actions: &mut Vec<Action>) {
    actions.clear();  // Reuse existing capacity
    // ...
}

// Good: use stack allocation
fn process_event(event: Event) -> ArrayVec<Action, 8> {
    let mut actions = ArrayVec::new();  // Stack allocated
    // ...
    actions
}

// ─── USE APPROPRIATE DATA STRUCTURES ────────────────────────────────────────

// For small collections (< 32 elements), linear search is often faster
let small_set: ArrayVec<u32, 32> = ...;
let found = small_set.iter().any(|&x| x == target);

// For larger collections, use HashMap/BTreeMap
let large_set: HashMap<u32, Value> = ...;
let found = large_set.contains_key(&target);

// ─── CACHE-FRIENDLY LAYOUTS ─────────────────────────────────────────────────

// Bad: pointer chasing
struct TaskList {
    tasks: Vec<Box<Task>>,  // Each task is heap-allocated separately
}

// Good: contiguous storage
struct TaskList {
    tasks: Vec<Task>,  // Tasks stored contiguously
}

// Good: separate hot and cold data
struct TaskHot {
    state: TaskState,
    priority: u8,
    time_slice: u64,
}

struct TaskCold {
    name: String,
    created: Timestamp,
    statistics: Stats,
}

struct TaskList {
    hot: Vec<TaskHot>,    // Frequently accessed data together
    cold: Vec<TaskCold>,  // Rarely accessed data separate
}

// ─── MINIMIZE COPYING ───────────────────────────────────────────────────────

// Bad: copies data
fn process(data: Vec<u8>) -> Vec<u8> {
    // ...
}

// Good: borrows data
fn process(data: &[u8]) -> ProcessedData {
    // ...
}

// Good: moves data when ownership needed
fn consume(data: Vec<u8>) {
    // Takes ownership, no copy
}

// ─── USE ITERATORS ──────────────────────────────────────────────────────────

// Bad: index-based loop (bounds checks on each access)
let mut sum = 0;
for i in 0..data.len() {
    sum += data[i];
}

// Good: iterator (no bounds checks)
let sum: u64 = data.iter().sum();

// ─── INLINE SMALL FUNCTIONS ─────────────────────────────────────────────────

#[inline]
pub fn task_id(&self) -> TaskId {
    self.id
}

#[inline(always)]  // Force inline for critical paths
pub fn is_ready(&self) -> bool {
    self.state == TaskState::Ready
}

// ─── CONST EVALUATION ───────────────────────────────────────────────────────

// Compute at compile time
const PAGE_SIZE: usize = 4096;
const PAGE_MASK: usize = !(PAGE_SIZE - 1);

const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

const KERNEL_STACK_SIZE: usize = align_up(64 * 1024, PAGE_SIZE);
```

### 7.2 Benchmarking

```rust
// In benchmarks/scheduler.rs

use criterion::{criterion_group, criterion_main, Criterion};

fn scheduler_benchmark(c: &mut Criterion) {
    c.bench_function("schedule_next_task", |b| {
        let mut scheduler = Scheduler::new();
        for i in 0..100 {
            scheduler.add_task(Task::new(i));
        }
        
        b.iter(|| {
            scheduler.schedule()
        })
    });
    
    c.bench_function("spawn_task", |b| {
        let mut scheduler = Scheduler::new();
        
        b.iter(|| {
            let id = scheduler.spawn("bench_task", Intent::batch());
            scheduler.kill(id);
        })
    });
}

criterion_group!(benches, scheduler_benchmark);
criterion_main!(benches);
```

---

## 8. Testing Standards

### 8.1 Test Organization

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                          TEST ORGANIZATION
// ═══════════════════════════════════════════════════════════════════════════

// ─── UNIT TESTS (in same file) ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test fixtures
    fn create_test_scheduler() -> Scheduler {
        Scheduler::new()
    }
    
    fn create_test_task(name: &str) -> Task {
        Task::new(name)
    }
    
    // Group related tests
    mod creation {
        use super::*;
        
        #[test]
        fn test_new_task_has_ready_state() {
            let task = create_test_task("test");
            assert_eq!(task.state(), TaskState::Ready);
        }
        
        #[test]
        fn test_new_task_has_default_priority() {
            let task = create_test_task("test");
            assert_eq!(task.priority(), DEFAULT_PRIORITY);
        }
    }
    
    mod state_transitions {
        use super::*;
        
        #[test]
        fn test_ready_to_running() {
            let mut task = create_test_task("test");
            task.set_state(TaskState::Running);
            assert_eq!(task.state(), TaskState::Running);
        }
        
        #[test]
        #[should_panic(expected = "invalid transition")]
        fn test_invalid_transition_panics() {
            let mut task = create_test_task("test");
            task.set_state(TaskState::Running);
            task.set_state(TaskState::Ready);  // Invalid!
        }
    }
    
    mod scheduling {
        use super::*;
        
        #[test]
        fn test_higher_priority_scheduled_first() {
            let mut sched = create_test_scheduler();
            
            let low = sched.spawn("low", Intent::background());
            let high = sched.spawn("high", Intent::system());
            
            assert_eq!(sched.next(), Some(high));
            assert_eq!(sched.next(), Some(low));
        }
    }
}

// ─── INTEGRATION TESTS (in tests/ directory) ────────────────────────────────

// tests/scheduler_integration.rs
use helix_core::*;

#[test]
fn test_full_scheduling_cycle() {
    let mut system = System::new();
    
    // Spawn multiple tasks
    let t1 = system.spawn("task1", task1_fn);
    let t2 = system.spawn("task2", task2_fn);
    let t3 = system.spawn("task3", task3_fn);
    
    // Run scheduler
    for _ in 0..100 {
        system.tick();
    }
    
    // Verify all tasks ran
    assert!(system.get_stats(t1).run_count > 0);
    assert!(system.get_stats(t2).run_count > 0);
    assert!(system.get_stats(t3).run_count > 0);
}
```

### 8.2 Test Naming

```rust
#[test]
fn test_<unit>_<scenario>_<expected_result>() {
    // ...
}

// Examples:
fn test_task_creation_sets_ready_state() { }
fn test_scheduler_with_empty_queue_returns_none() { }
fn test_allocator_when_full_returns_error() { }
fn test_context_switch_preserves_registers() { }
```

---

## 9. Formatting Rules

### 9.1 rustfmt Configuration

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"

# Imports
imports_granularity = "Module"
group_imports = "StdExternalCrate"
reorder_imports = true

# Items
reorder_modules = true
reorder_impl_items = false

# Functions
fn_single_line = false
fn_args_layout = "Tall"

# Control flow
control_brace_style = "AlwaysSameLine"
match_arm_blocks = true

# Comments
wrap_comments = true
comment_width = 80
normalize_comments = true
```

### 9.2 Manual Formatting Rules

```rust
// Line length: 100 max, prefer 80
// Exception: URLs, paths that shouldn't wrap

// Indentation: 4 spaces, no tabs

// Braces: same line
fn example() {
    if condition {
        // ...
    } else {
        // ...
    }
}

// Long function signatures: one param per line
fn complex_function(
    first_param: SomeVeryLongType,
    second_param: AnotherLongType,
    third_param: &YetAnotherType,
) -> Result<OutputType, ErrorType> {
    // ...
}

// Chain methods: one per line when long
let result = items
    .iter()
    .filter(|x| x.is_valid())
    .map(|x| x.transform())
    .collect::<Vec<_>>();

// Match arms: align similar patterns
match value {
    Pattern::A        => handle_a(),
    Pattern::B(x)     => handle_b(x),
    Pattern::C(x, y)  => handle_c(x, y),
    _                 => handle_default(),
}
```

---

## 10. Anti-Patterns

### 10.1 Common Mistakes

```rust
// ═══════════════════════════════════════════════════════════════════════════
//                           ANTI-PATTERNS TO AVOID
// ═══════════════════════════════════════════════════════════════════════════

// ─── UNNECESSARY CLONING ────────────────────────────────────────────────────

// Bad
fn process(data: String) {
    let copy = data.clone();  // Why clone if you have ownership?
    use_data(copy);
}

// Good
fn process(data: String) {
    use_data(data);
}

// ─── IGNORING RESULTS ───────────────────────────────────────────────────────

// Bad
fn do_something() {
    might_fail();  // Warning: unused Result
}

// Bad
fn do_something() {
    let _ = might_fail();  // Silently ignoring error
}

// Good
fn do_something() -> Result<()> {
    might_fail()?;  // Propagate error
    Ok(())
}

// Good (when ignoring is intentional)
fn do_something() {
    // Intentionally ignoring cleanup errors
    let _ = cleanup();
}

// ─── MAGIC NUMBERS ──────────────────────────────────────────────────────────

// Bad
if size > 4096 { ... }
buffer[0..16].copy_from_slice(header);

// Good
const PAGE_SIZE: usize = 4096;
const HEADER_SIZE: usize = 16;

if size > PAGE_SIZE { ... }
buffer[0..HEADER_SIZE].copy_from_slice(header);

// ─── BOOLEAN PARAMETERS ─────────────────────────────────────────────────────

// Bad: what does `true` mean?
task.run(true, false);

// Good: use enums or named parameters
task.run(RunMode::Async, Priority::Low);

// Or builder pattern
task.configure()
    .async_mode()
    .low_priority()
    .run();

// ─── STRINGLY TYPED APIS ────────────────────────────────────────────────────

// Bad
fn set_state(state: &str) {
    match state {
        "ready" => ...
        "running" => ...
        _ => panic!("unknown state"),
    }
}

// Good
fn set_state(state: TaskState) {
    match state {
        TaskState::Ready => ...
        TaskState::Running => ...
        // Exhaustive match, no runtime errors
    }
}

// ─── PREMATURE OPTIMIZATION ─────────────────────────────────────────────────

// Bad: complex "optimization" without measurement
fn find_task(id: u32) -> Option<&Task> {
    // Custom hash function, inline assembly, etc.
    // when a simple HashMap would work
}

// Good: simple first, optimize with data
fn find_task(&self, id: TaskId) -> Option<&Task> {
    self.tasks.get(&id)
}

// ─── OVERLY GENERIC CODE ────────────────────────────────────────────────────

// Bad: generic when not needed
fn add<T: Add<Output = T>>(a: T, b: T) -> T {
    a + b
}

// Good: concrete when sufficient
fn add(a: u64, b: u64) -> u64 {
    a + b
}

// ─── DEEPLY NESTED CODE ─────────────────────────────────────────────────────

// Bad
fn process(req: Request) -> Result<Response> {
    if req.is_valid() {
        if let Some(task) = get_task(req.id) {
            if task.can_process() {
                if let Ok(result) = task.process(req.data) {
                    return Ok(Response::new(result));
                }
            }
        }
    }
    Err(Error::Failed)
}

// Good: early returns
fn process(req: Request) -> Result<Response> {
    if !req.is_valid() {
        return Err(Error::InvalidRequest);
    }
    
    let task = get_task(req.id).ok_or(Error::NotFound)?;
    
    if !task.can_process() {
        return Err(Error::NotReady);
    }
    
    let result = task.process(req.data)?;
    Ok(Response::new(result))
}
```

---

## Summary

Following these coding standards ensures:

1. **Consistency**: Code looks similar across the project
2. **Safety**: Fewer bugs, especially with unsafe code
3. **Maintainability**: Easy to understand and modify
4. **Performance**: Efficient by design
5. **Testability**: Easy to test and verify

Always run before committing:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

---

<div align="center">

📐 *Clean code is not written by following a set of rules. Clean code is written by professionals who care.* 📐

</div>
