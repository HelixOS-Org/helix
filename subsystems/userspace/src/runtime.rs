//! # Userspace Runtime
//!
//! Process runtime and management for userspace programs.
//!
//! ## Features
//! - Process spawning and lifecycle management
//! - Userspace memory management
//! - File descriptor tables
//! - Signal handling framework

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use super::elf::ParsedElf;
use super::{UserError, UserResult, STATS};

/// Process ID type
pub type Pid = u64;

/// File descriptor type
pub type Fd = i32;

/// Standard input file descriptor
pub const STDIN_FD: Fd = 0;
/// Standard output file descriptor
pub const STDOUT_FD: Fd = 1;
/// Standard error file descriptor
pub const STDERR_FD: Fd = 2;

/// Maximum file descriptors per process
pub const MAX_FDS: usize = 256;

/// Maximum processes
pub const MAX_PROCESSES: usize = 1024;

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Being created
    Creating,
    /// Ready to run
    Ready,
    /// Currently running
    Running,
    /// Waiting for I/O or event
    Waiting,
    /// Stopped by signal
    Stopped,
    /// Terminated but not yet reaped
    Zombie,
    /// Fully terminated
    Dead,
}

/// Process priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[derive(Default)]
pub enum Priority {
    /// Idle priority
    Idle     = 0,
    /// Low priority
    Low      = 1,
    /// Normal priority
    #[default]
    Normal   = 2,
    /// High priority
    High     = 3,
    /// Real-time priority
    RealTime = 4,
}


/// File descriptor entry
#[derive(Debug, Clone)]
pub struct FdEntry {
    /// File descriptor number
    pub fd: Fd,
    /// File type
    pub fd_type: FdType,
    /// Read offset
    pub offset: u64,
    /// Flags
    pub flags: u32,
}

/// File descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdType {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Pipe
    Pipe,
    /// Socket
    Socket,
    /// Console/TTY
    Console,
    /// Null device
    Null,
}

/// File descriptor table
#[derive(Debug)]
pub struct FdTable {
    /// File descriptors
    entries: BTreeMap<Fd, FdEntry>,
    /// Next available FD
    next_fd: Fd,
}

impl FdTable {
    /// Create new FD table with standard streams
    pub fn new() -> Self {
        let mut table = Self {
            entries: BTreeMap::new(),
            next_fd: 3,
        };

        // Add standard streams
        table.entries.insert(STDIN_FD, FdEntry {
            fd: STDIN_FD,
            fd_type: FdType::Console,
            offset: 0,
            flags: 0,
        });
        table.entries.insert(STDOUT_FD, FdEntry {
            fd: STDOUT_FD,
            fd_type: FdType::Console,
            offset: 0,
            flags: 0,
        });
        table.entries.insert(STDERR_FD, FdEntry {
            fd: STDERR_FD,
            fd_type: FdType::Console,
            offset: 0,
            flags: 0,
        });

        table
    }

    /// Allocate a new file descriptor
    pub fn alloc(&mut self, fd_type: FdType) -> Option<Fd> {
        if self.entries.len() >= MAX_FDS {
            return None;
        }

        let fd = self.next_fd;
        self.next_fd += 1;

        self.entries.insert(fd, FdEntry {
            fd,
            fd_type,
            offset: 0,
            flags: 0,
        });

        Some(fd)
    }

    /// Get file descriptor entry
    pub fn get(&self, fd: Fd) -> Option<&FdEntry> {
        self.entries.get(&fd)
    }

    /// Close a file descriptor
    pub fn close(&mut self, fd: Fd) -> bool {
        self.entries.remove(&fd).is_some()
    }

    /// Duplicate a file descriptor
    pub fn dup(&mut self, old_fd: Fd) -> Option<Fd> {
        let entry = self.entries.get(&old_fd)?.clone();
        let new_fd = self.next_fd;
        self.next_fd += 1;

        self.entries.insert(new_fd, FdEntry {
            fd: new_fd,
            ..entry
        });

        Some(new_fd)
    }
}

impl Default for FdTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Process handle
#[derive(Debug)]
pub struct ProcessHandle {
    /// Process ID
    pub pid: Pid,
    /// Parent PID
    pub ppid: Pid,
    /// Process name
    pub name: String,
    /// State
    state: Mutex<ProcessState>,
    /// Priority
    pub priority: Priority,
    /// File descriptors
    fd_table: Mutex<FdTable>,
    /// Exit code (if terminated)
    exit_code: Mutex<Option<i32>>,
    /// Entry point
    pub entry_point: u64,
    /// Stack pointer
    pub stack_ptr: u64,
    /// Heap base
    pub heap_base: u64,
    /// Heap size
    pub heap_size: u64,
}

impl ProcessHandle {
    /// Create new process handle
    pub fn new(pid: Pid, ppid: Pid, name: impl Into<String>) -> Self {
        STATS.process_spawned();

        Self {
            pid,
            ppid,
            name: name.into(),
            state: Mutex::new(ProcessState::Creating),
            priority: Priority::Normal,
            fd_table: Mutex::new(FdTable::new()),
            exit_code: Mutex::new(None),
            entry_point: 0,
            stack_ptr: 0,
            heap_base: 0,
            heap_size: 0,
        }
    }

    /// Get current state
    pub fn state(&self) -> ProcessState {
        *self.state.lock()
    }

    /// Set state
    pub fn set_state(&self, state: ProcessState) {
        *self.state.lock() = state;
    }

    /// Mark as ready
    pub fn ready(&self) {
        self.set_state(ProcessState::Ready);
    }

    /// Mark as running
    pub fn running(&self) {
        self.set_state(ProcessState::Running);
    }

    /// Exit with code
    pub fn exit(&self, code: i32) {
        *self.exit_code.lock() = Some(code);
        self.set_state(ProcessState::Zombie);
    }

    /// Reap the process
    pub fn reap(&self) -> Option<i32> {
        let code = *self.exit_code.lock();
        if code.is_some() {
            self.set_state(ProcessState::Dead);
        }
        code
    }

    /// Allocate a file descriptor
    pub fn alloc_fd(&self, fd_type: FdType) -> Option<Fd> {
        self.fd_table.lock().alloc(fd_type)
    }

    /// Get file descriptor
    pub fn get_fd(&self, fd: Fd) -> Option<FdEntry> {
        self.fd_table.lock().get(fd).cloned()
    }

    /// Close file descriptor
    pub fn close_fd(&self, fd: Fd) -> bool {
        self.fd_table.lock().close(fd)
    }
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Default stack size
    pub default_stack_size: usize,
    /// Default heap size
    pub default_heap_size: usize,
    /// Maximum processes
    pub max_processes: usize,
    /// Enable ASLR (Address Space Layout Randomization)
    pub aslr_enabled: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_stack_size: 8 * 1024 * 1024, // 8 MB
            default_heap_size: 16 * 1024 * 1024, // 16 MB
            max_processes: MAX_PROCESSES,
            aslr_enabled: true,
        }
    }
}

/// The userspace runtime
pub struct Runtime {
    /// Configuration
    _config: RuntimeConfig,
    /// Process table
    processes: RwLock<BTreeMap<Pid, Arc<ProcessHandle>>>,
    /// Next PID
    next_pid: AtomicU64,
    /// Initialized flag
    initialized: AtomicBool,
}

impl Runtime {
    /// Create new runtime
    pub const fn new() -> Self {
        Self {
            _config: RuntimeConfig {
                default_stack_size: 8 * 1024 * 1024,
                default_heap_size: 16 * 1024 * 1024,
                max_processes: MAX_PROCESSES,
                aslr_enabled: true,
            },
            processes: RwLock::new(BTreeMap::new()),
            next_pid: AtomicU64::new(1),
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize with config
    pub fn init_with_config(&self, _config: RuntimeConfig) {
        // Note: can't actually change config in const fn created struct
        // In real impl, would use UnsafeCell or similar
        self.initialized.store(true, Ordering::SeqCst);
    }

    /// Create a new process from ELF
    pub fn spawn(&self, elf: &ParsedElf, name: &str) -> UserResult<Arc<ProcessHandle>> {
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst);

        let mut process = ProcessHandle::new(pid, 0, name);
        process.entry_point = elf.entry_point;

        // In real OS, would:
        // 1. Allocate address space
        // 2. Map ELF segments
        // 3. Set up stack
        // 4. Set up heap

        let handle = Arc::new(process);
        self.processes.write().insert(pid, handle.clone());

        handle.ready();

        Ok(handle)
    }

    /// Spawn a simple process (without ELF)
    pub fn spawn_simple(&self, name: &str, entry: u64) -> UserResult<Arc<ProcessHandle>> {
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst);

        let mut process = ProcessHandle::new(pid, 0, name);
        process.entry_point = entry;

        let handle = Arc::new(process);
        self.processes.write().insert(pid, handle.clone());

        handle.ready();

        Ok(handle)
    }

    /// Get process by PID
    pub fn get_process(&self, pid: Pid) -> Option<Arc<ProcessHandle>> {
        self.processes.read().get(&pid).cloned()
    }

    /// List all processes
    pub fn list_processes(&self) -> Vec<Arc<ProcessHandle>> {
        self.processes.read().values().cloned().collect()
    }

    /// Kill a process
    pub fn kill(&self, pid: Pid, signal: i32) -> UserResult<()> {
        if let Some(process) = self.get_process(pid) {
            if signal == 9 || signal == 15 {
                process.exit(128 + signal);
            }
            Ok(())
        } else {
            Err(UserError::InvalidArgument)
        }
    }

    /// Reap zombie processes
    pub fn reap_zombies(&self) {
        let mut processes = self.processes.write();
        processes.retain(|_, p| p.state() != ProcessState::Dead);
    }

    /// Get number of active processes
    pub fn process_count(&self) -> usize {
        self.processes.read().len()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Global runtime instance
pub static RUNTIME: Runtime = Runtime::new();

/// Initialize runtime subsystem
pub fn init() -> UserResult<()> {
    RUNTIME.initialized.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fd_table() {
        let mut table = FdTable::new();

        // Standard streams should exist
        assert!(table.get(STDIN_FD).is_some());
        assert!(table.get(STDOUT_FD).is_some());
        assert!(table.get(STDERR_FD).is_some());

        // Allocate new FD
        let fd = table.alloc(FdType::File).unwrap();
        assert_eq!(fd, 3);

        // Close it
        assert!(table.close(fd));
        assert!(table.get(fd).is_none());
    }

    #[test]
    fn test_process_handle() {
        let process = ProcessHandle::new(1, 0, "test");
        assert_eq!(process.state(), ProcessState::Creating);

        process.ready();
        assert_eq!(process.state(), ProcessState::Ready);

        process.exit(0);
        assert_eq!(process.state(), ProcessState::Zombie);
    }
}
