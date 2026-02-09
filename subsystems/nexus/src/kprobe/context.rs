//! Kprobe Context
//!
//! Execution context when kprobe fires.

/// Kprobe context (registers and state)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KprobeContext {
    /// Instruction pointer
    pub ip: u64,
    /// Stack pointer
    pub sp: u64,
    /// General purpose registers (architecture-specific)
    pub regs: [u64; 16],
    /// Flags/status register
    pub flags: u64,
    /// CPU ID
    pub cpu: u32,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl KprobeContext {
    /// Create empty context
    pub fn new() -> Self {
        Self {
            ip: 0,
            sp: 0,
            regs: [0; 16],
            flags: 0,
            cpu: 0,
            pid: 0,
            tid: 0,
            timestamp: 0,
        }
    }

    /// Get argument (by index)
    pub fn arg(&self, index: usize) -> u64 {
        // x86_64 calling convention: rdi, rsi, rdx, rcx, r8, r9
        match index {
            0 => self.regs[0], // rdi
            1 => self.regs[1], // rsi
            2 => self.regs[2], // rdx
            3 => self.regs[3], // rcx
            4 => self.regs[4], // r8
            5 => self.regs[5], // r9
            _ => 0,            // Stack args not supported
        }
    }

    /// Get return value (after function returns)
    #[inline(always)]
    pub fn return_value(&self) -> u64 {
        self.regs[0] // rax on x86_64
    }
}

impl Default for KprobeContext {
    fn default() -> Self {
        Self::new()
    }
}
