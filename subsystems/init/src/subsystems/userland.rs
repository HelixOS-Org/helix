//! # Userland Subsystem
//!
//! Userspace transition and initial process management.
//! Runtime phase subsystem for user-mode initialization.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// =============================================================================
// PROCESS IDENTIFIERS
// =============================================================================

/// Process ID
pub type Pid = u32;

/// User ID
pub type Uid = u32;

/// Group ID
pub type Gid = u32;

/// PID 0 is reserved (scheduler)
pub const PID_IDLE: Pid = 0;

/// PID 1 is init
pub const PID_INIT: Pid = 1;

// =============================================================================
// PROCESS STATE
// =============================================================================

/// Process state
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is being created
    Creating = 0,
    /// Ready to run
    Ready    = 1,
    /// Currently running
    Running  = 2,
    /// Blocked on I/O or wait
    Blocked  = 3,
    /// Stopped (SIGSTOP)
    Stopped  = 4,
    /// Zombie (terminated, waiting for wait())
    Zombie   = 5,
    /// Dead (resources freed)
    Dead     = 6,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self::Creating
    }
}

// =============================================================================
// PROCESS FLAGS
// =============================================================================

bitflags::bitflags! {
    /// Process flags
    #[derive(Debug, Clone, Copy)]
    pub struct ProcessFlags: u32 {
        /// Process is a kernel thread
        const KERNEL = 1 << 0;
        /// Process uses userspace memory
        const USERSPACE = 1 << 1;
        /// Process is the session leader
        const SESSION_LEADER = 1 << 2;
        /// Process is the process group leader
        const PGRP_LEADER = 1 << 3;
        /// Process is being traced (ptrace)
        const TRACED = 1 << 4;
        /// Process is being debugged
        const DEBUGGED = 1 << 5;
        /// Process is a daemon
        const DAEMON = 1 << 6;
        /// Process has called exec
        const EXECED = 1 << 7;
        /// Process is a fork of another
        const FORKED = 1 << 8;
        /// Process is the init process
        const INIT = 1 << 9;
        /// Process cannot be killed
        const UNKILLABLE = 1 << 10;
    }
}

// =============================================================================
// MEMORY REGION
// =============================================================================

/// Memory region permissions
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryPermissions(u8);

impl MemoryPermissions {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn bits(&self) -> u8 {
        self.0
    }
}

impl core::ops::BitOr for MemoryPermissions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Memory region type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    Code,
    Data,
    Bss,
    Heap,
    Stack,
    SharedMemory,
    MappedFile,
    Anonymous,
    DeviceMemory,
}

/// Process memory region
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub permissions: MemoryPermissions,
    pub region_type: RegionType,
    pub offset: u64,
    pub file: Option<String>,
}

impl MemoryRegion {
    /// Create new region
    pub fn new(
        start: u64,
        end: u64,
        permissions: MemoryPermissions,
        region_type: RegionType,
    ) -> Self {
        Self {
            start,
            end,
            permissions,
            region_type,
            offset: 0,
            file: None,
        }
    }

    /// Size of region
    pub fn size(&self) -> u64 {
        self.end - self.start
    }

    /// Check if address is in region
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }
}

// =============================================================================
// ADDRESS SPACE
// =============================================================================

/// Process address space
#[derive(Debug, Clone, Default)]
pub struct AddressSpace {
    /// Memory regions
    regions: Vec<MemoryRegion>,
    /// Page table root (CR3 / TTBR0 / SATP)
    page_table_root: u64,
    /// Heap start
    heap_start: u64,
    /// Heap end (current brk)
    heap_end: u64,
    /// Stack start (grows down)
    stack_start: u64,
    /// Stack end
    stack_end: u64,
    /// Entry point
    entry_point: u64,
}

impl AddressSpace {
    /// Create new address space
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page table root
    pub fn set_page_table(&mut self, root: u64) {
        self.page_table_root = root;
    }

    /// Add region
    pub fn add_region(&mut self, region: MemoryRegion) {
        self.regions.push(region);
    }

    /// Find region containing address
    pub fn find_region(&self, addr: u64) -> Option<&MemoryRegion> {
        self.regions.iter().find(|r| r.contains(addr))
    }

    /// Set heap bounds
    pub fn set_heap(&mut self, start: u64, end: u64) {
        self.heap_start = start;
        self.heap_end = end;
    }

    /// Extend heap (brk)
    pub fn extend_heap(&mut self, new_end: u64) -> bool {
        if new_end < self.heap_start || new_end > self.stack_end - 0x10000 {
            return false;
        }
        self.heap_end = new_end;
        true
    }

    /// Set stack bounds
    pub fn set_stack(&mut self, start: u64, end: u64) {
        self.stack_start = start;
        self.stack_end = end;
    }

    /// Set entry point
    pub fn set_entry(&mut self, entry: u64) {
        self.entry_point = entry;
    }
}

// =============================================================================
// FILE DESCRIPTOR
// =============================================================================

/// File descriptor
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub fd: i32,
    pub path: String,
    pub flags: u32,
    pub offset: u64,
    pub is_directory: bool,
    pub is_socket: bool,
    pub is_pipe: bool,
}

impl FileDescriptor {
    /// Create standard input
    pub fn stdin() -> Self {
        Self {
            fd: 0,
            path: String::from("/dev/stdin"),
            flags: 0,
            offset: 0,
            is_directory: false,
            is_socket: false,
            is_pipe: false,
        }
    }

    /// Create standard output
    pub fn stdout() -> Self {
        Self {
            fd: 1,
            path: String::from("/dev/stdout"),
            flags: 1,
            offset: 0,
            is_directory: false,
            is_socket: false,
            is_pipe: false,
        }
    }

    /// Create standard error
    pub fn stderr() -> Self {
        Self {
            fd: 2,
            path: String::from("/dev/stderr"),
            flags: 1,
            offset: 0,
            is_directory: false,
            is_socket: false,
            is_pipe: false,
        }
    }
}

// =============================================================================
// PROCESS
// =============================================================================

/// Process control block
#[derive(Debug, Clone)]
pub struct Process {
    /// Process ID
    pub pid: Pid,
    /// Parent process ID
    pub ppid: Pid,
    /// Process group ID
    pub pgid: Pid,
    /// Session ID
    pub sid: Pid,

    /// User ID (real)
    pub uid: Uid,
    /// Effective user ID
    pub euid: Uid,
    /// Saved user ID
    pub suid: Uid,

    /// Group ID (real)
    pub gid: Gid,
    /// Effective group ID
    pub egid: Gid,
    /// Saved group ID
    pub sgid: Gid,

    /// Supplementary groups
    pub groups: Vec<Gid>,

    /// Process name
    pub name: String,
    /// Command line arguments
    pub argv: Vec<String>,
    /// Environment variables
    pub envp: Vec<String>,
    /// Current working directory
    pub cwd: String,

    /// Process state
    pub state: ProcessState,
    /// Process flags
    pub flags: ProcessFlags,

    /// Address space
    pub address_space: AddressSpace,
    /// File descriptors
    pub file_descriptors: Vec<FileDescriptor>,

    /// Exit code
    pub exit_code: i32,

    /// CPU time used (nanoseconds)
    pub cpu_time: u64,
    /// Start time (timestamp)
    pub start_time: u64,

    /// Priority (nice value)
    pub priority: i8,

    /// Signal mask
    pub signal_mask: u64,
    /// Pending signals
    pub pending_signals: u64,
}

impl Process {
    /// Create new process
    pub fn new(pid: Pid, ppid: Pid, name: &str) -> Self {
        Self {
            pid,
            ppid,
            pgid: pid,
            sid: pid,
            uid: 0,
            euid: 0,
            suid: 0,
            gid: 0,
            egid: 0,
            sgid: 0,
            groups: Vec::new(),
            name: String::from(name),
            argv: Vec::new(),
            envp: Vec::new(),
            cwd: String::from("/"),
            state: ProcessState::Creating,
            flags: ProcessFlags::empty(),
            address_space: AddressSpace::new(),
            file_descriptors: Vec::new(),
            exit_code: 0,
            cpu_time: 0,
            start_time: 0,
            priority: 0,
            signal_mask: 0,
            pending_signals: 0,
        }
    }

    /// Create init process
    pub fn init() -> Self {
        let mut p = Self::new(PID_INIT, PID_IDLE, "init");
        p.flags = ProcessFlags::USERSPACE | ProcessFlags::INIT | ProcessFlags::UNKILLABLE;
        p.file_descriptors.push(FileDescriptor::stdin());
        p.file_descriptors.push(FileDescriptor::stdout());
        p.file_descriptors.push(FileDescriptor::stderr());
        p
    }

    /// Is kernel process?
    pub fn is_kernel(&self) -> bool {
        self.flags.contains(ProcessFlags::KERNEL)
    }

    /// Is userspace process?
    pub fn is_userspace(&self) -> bool {
        self.flags.contains(ProcessFlags::USERSPACE)
    }

    /// Is init process?
    pub fn is_init(&self) -> bool {
        self.pid == PID_INIT
    }

    /// Is root?
    pub fn is_root(&self) -> bool {
        self.euid == 0
    }

    /// Can signal target?
    pub fn can_signal(&self, target: &Process) -> bool {
        if target.flags.contains(ProcessFlags::UNKILLABLE) {
            return false;
        }

        self.is_root() || self.uid == target.uid || self.euid == target.uid
    }

    /// Add file descriptor
    pub fn add_fd(&mut self, fd: FileDescriptor) {
        self.file_descriptors.push(fd);
    }

    /// Get file descriptor
    pub fn get_fd(&self, fd: i32) -> Option<&FileDescriptor> {
        self.file_descriptors.iter().find(|f| f.fd == fd)
    }

    /// Close file descriptor
    pub fn close_fd(&mut self, fd: i32) -> bool {
        if let Some(pos) = self.file_descriptors.iter().position(|f| f.fd == fd) {
            self.file_descriptors.remove(pos);
            true
        } else {
            false
        }
    }
}

// =============================================================================
// EXEC INFO
// =============================================================================

/// Executable information (parsed from ELF/etc)
#[derive(Debug, Clone)]
pub struct ExecInfo {
    /// Entry point
    pub entry: u64,
    /// Program headers
    pub program_headers: Vec<ProgramHeader>,
    /// Interpreter path (for dynamic linking)
    pub interpreter: Option<String>,
    /// Architecture
    pub arch: ExecArch,
    /// Is position-independent?
    pub is_pie: bool,
}

/// Program header
#[derive(Debug, Clone)]
pub struct ProgramHeader {
    pub ph_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

/// Executable architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecArch {
    X86_64,
    AArch64,
    RiscV64,
    Unknown,
}

impl Default for ExecArch {
    fn default() -> Self {
        #[cfg(target_arch = "x86_64")]
        return Self::X86_64;

        #[cfg(target_arch = "aarch64")]
        return Self::AArch64;

        #[cfg(target_arch = "riscv64")]
        return Self::RiscV64;

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        Self::Unknown
    }
}

// =============================================================================
// USERLAND SUBSYSTEM
// =============================================================================

/// Userland Subsystem
///
/// Manages userspace transition and process initialization.
pub struct UserlandSubsystem {
    info: SubsystemInfo,

    /// Process table
    processes: BTreeMap<Pid, Process>,
    /// Next PID to assign
    next_pid: AtomicU32,

    /// Init process path
    init_path: String,
    /// Init process arguments
    init_args: Vec<String>,

    /// Userspace entry enabled
    userspace_enabled: AtomicBool,

    /// Statistics
    total_processes: AtomicU64,
    total_forks: AtomicU64,
    total_execs: AtomicU64,
    total_exits: AtomicU64,
}

static USERLAND_DEPS: [Dependency; 6] = [
    Dependency::required("vmm"),
    Dependency::required("scheduler"),
    Dependency::required("filesystem"),
    Dependency::required("security"),
    Dependency::optional("network"),
    Dependency::optional("debug"),
];

impl UserlandSubsystem {
    /// Create new userland subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("userland", InitPhase::Runtime)
                .with_priority(2000)
                .with_description("Userspace management")
                .with_dependencies(&USERLAND_DEPS)
                .provides(PhaseCapabilities::USERSPACE),
            processes: BTreeMap::new(),
            next_pid: AtomicU32::new(PID_INIT),
            init_path: String::from("/sbin/init"),
            init_args: Vec::new(),
            userspace_enabled: AtomicBool::new(false),
            total_processes: AtomicU64::new(0),
            total_forks: AtomicU64::new(0),
            total_execs: AtomicU64::new(0),
            total_exits: AtomicU64::new(0),
        }
    }

    /// Allocate new PID
    pub fn alloc_pid(&self) -> Pid {
        self.next_pid.fetch_add(1, Ordering::SeqCst)
    }

    /// Get process by PID
    pub fn get_process(&self, pid: Pid) -> Option<&Process> {
        self.processes.get(&pid)
    }

    /// Get mutable process
    pub fn get_process_mut(&mut self, pid: Pid) -> Option<&mut Process> {
        self.processes.get_mut(&pid)
    }

    /// Create process
    pub fn create_process(&mut self, name: &str, ppid: Pid) -> Pid {
        let pid = self.alloc_pid();
        let process = Process::new(pid, ppid, name);
        self.processes.insert(pid, process);
        self.total_processes.fetch_add(1, Ordering::Relaxed);
        pid
    }

    /// Fork process
    pub fn fork(&mut self, parent_pid: Pid) -> Option<Pid> {
        let parent = self.processes.get(&parent_pid)?;
        let child_pid = self.alloc_pid();

        let mut child = parent.clone();
        child.pid = child_pid;
        child.ppid = parent_pid;
        child.state = ProcessState::Ready;
        child.flags |= ProcessFlags::FORKED;
        child.cpu_time = 0;

        self.processes.insert(child_pid, child);
        self.total_forks.fetch_add(1, Ordering::Relaxed);
        self.total_processes.fetch_add(1, Ordering::Relaxed);

        Some(child_pid)
    }

    /// Exit process
    pub fn exit_process(&mut self, pid: Pid, exit_code: i32) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            if process.flags.contains(ProcessFlags::UNKILLABLE) {
                return false;
            }

            process.state = ProcessState::Zombie;
            process.exit_code = exit_code;
            self.total_exits.fetch_add(1, Ordering::Relaxed);

            // Reparent children to init
            let children: Vec<Pid> = self
                .processes
                .values()
                .filter(|p| p.ppid == pid)
                .map(|p| p.pid)
                .collect();

            for child_pid in children {
                if let Some(child) = self.processes.get_mut(&child_pid) {
                    child.ppid = PID_INIT;
                }
            }

            true
        } else {
            false
        }
    }

    /// Reap zombie process
    pub fn reap(&mut self, pid: Pid) -> Option<i32> {
        let process = self.processes.get(&pid)?;
        if process.state != ProcessState::Zombie {
            return None;
        }

        let exit_code = process.exit_code;
        self.processes.remove(&pid);
        Some(exit_code)
    }

    /// Get process list
    pub fn process_list(&self) -> Vec<Pid> {
        self.processes.keys().copied().collect()
    }

    /// Count processes by state
    pub fn count_by_state(&self) -> BTreeMap<ProcessState, usize> {
        let mut counts = BTreeMap::new();
        for process in self.processes.values() {
            *counts.entry(process.state).or_insert(0) += 1;
        }
        counts
    }

    /// Initialize userspace
    fn init_userspace(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Preparing userspace environment");

        // Create init process
        let init = Process::init();
        self.processes.insert(PID_INIT, init);

        ctx.debug(alloc::format!("Init process path: {}", self.init_path));

        // Setup user-mode entry
        self.setup_user_mode_entry(ctx)?;

        self.userspace_enabled.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Setup user-mode entry point
    fn setup_user_mode_entry(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        #[cfg(target_arch = "x86_64")]
        {
            ctx.debug("Setting up SYSCALL/SYSRET");
            // In real kernel: setup MSRs for syscall
            // STAR, LSTAR, FMASK, etc.
        }

        #[cfg(target_arch = "aarch64")]
        {
            ctx.debug("Setting up EL0 transition");
            // Setup exception return to EL0
        }

        #[cfg(target_arch = "riscv64")]
        {
            ctx.debug("Setting up U-mode transition");
            // Setup supervisor to user mode transition
        }

        Ok(())
    }

    /// Transition to userspace
    pub fn enter_userspace(&self, entry: u64, stack: u64) -> ! {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // Set up IRET frame for ring 0 -> ring 3 transition
            core::arch::asm!(
                "cli",
                "push {ss}",      // SS (user data segment)
                "push {sp}",      // RSP (user stack)
                "push 0x202",     // RFLAGS (IF enabled)
                "push {cs}",      // CS (user code segment)
                "push {ip}",      // RIP (entry point)
                "iretq",
                ss = in(reg) 0x23u64,  // User data segment selector
                sp = in(reg) stack,
                cs = in(reg) 0x1Bu64,  // User code segment selector
                ip = in(reg) entry,
                options(noreturn)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                "msr elr_el1, {entry}",
                "msr sp_el0, {stack}",
                "mov x0, #0",
                "msr spsr_el1, x0",
                "eret",
                entry = in(reg) entry,
                stack = in(reg) stack,
                options(noreturn)
            );
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            core::arch::asm!(
                "csrw sepc, {entry}",
                "li t0, 0x100",    // Set SPP to 0 (user mode)
                "csrc sstatus, t0",
                "mv sp, {stack}",
                "sret",
                entry = in(reg) entry,
                stack = in(reg) stack,
                options(noreturn)
            );
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            loop {}
        }
    }
}

impl Default for UserlandSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for UserlandSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing userland subsystem");

        // Get init path from config
        if let Some(path) = ctx.config().get_string("init_path") {
            self.init_path = path;
        }

        // Initialize userspace
        self.init_userspace(ctx)?;

        ctx.info(alloc::format!(
            "Userland: {} processes, init={}",
            self.processes.len(),
            self.init_path
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info(alloc::format!(
            "Userland shutdown: {} total processes, {} forks, {} execs, {} exits",
            self.total_processes.load(Ordering::Relaxed),
            self.total_forks.load(Ordering::Relaxed),
            self.total_execs.load(Ordering::Relaxed),
            self.total_exits.load(Ordering::Relaxed)
        ));

        // Terminate all processes
        for (_, process) in self.processes.iter_mut() {
            if !process.flags.contains(ProcessFlags::KERNEL) {
                process.state = ProcessState::Dead;
            }
        }

        Ok(())
    }

    fn health_check(&self, ctx: &mut InitContext) -> InitResult<bool> {
        // Check init process exists
        if !self.processes.contains_key(&PID_INIT) {
            ctx.error("Init process not found!");
            return Ok(false);
        }

        // Check for too many zombies
        let zombie_count = self
            .processes
            .values()
            .filter(|p| p.state == ProcessState::Zombie)
            .count();

        if zombie_count > 1000 {
            ctx.warn(alloc::format!(
                "Too many zombie processes: {}",
                zombie_count
            ));
        }

        Ok(true)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_userland_subsystem() {
        let sub = UserlandSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Runtime);
        assert!(sub.info().provides.contains(PhaseCapabilities::USERSPACE));
    }

    #[test]
    fn test_process_creation() {
        let mut sub = UserlandSubsystem::new();

        let pid = sub.create_process("test", 0);
        assert!(pid >= PID_INIT);

        let process = sub.get_process(pid).unwrap();
        assert_eq!(process.name, "test");
        assert_eq!(process.state, ProcessState::Creating);
    }

    #[test]
    fn test_fork() {
        let mut sub = UserlandSubsystem::new();

        let parent = sub.create_process("parent", 0);
        let child = sub.fork(parent).unwrap();

        let child_proc = sub.get_process(child).unwrap();
        assert_eq!(child_proc.ppid, parent);
        assert!(child_proc.flags.contains(ProcessFlags::FORKED));
    }

    #[test]
    fn test_exit_and_reap() {
        let mut sub = UserlandSubsystem::new();

        let pid = sub.create_process("test", 0);
        sub.exit_process(pid, 42);

        let process = sub.get_process(pid).unwrap();
        assert_eq!(process.state, ProcessState::Zombie);

        let exit_code = sub.reap(pid).unwrap();
        assert_eq!(exit_code, 42);
        assert!(sub.get_process(pid).is_none());
    }

    #[test]
    fn test_init_process() {
        let init = Process::init();
        assert_eq!(init.pid, PID_INIT);
        assert!(init.is_init());
        assert!(init.flags.contains(ProcessFlags::UNKILLABLE));
    }

    #[test]
    fn test_address_space() {
        let mut space = AddressSpace::new();

        space.set_heap(0x10000, 0x20000);
        assert!(space.extend_heap(0x30000));

        space.add_region(MemoryRegion::new(
            0x1000,
            0x2000,
            MemoryPermissions::READ | MemoryPermissions::EXECUTE,
            RegionType::Code,
        ));

        assert!(space.find_region(0x1500).is_some());
        assert!(space.find_region(0x5000).is_none());
    }
}
