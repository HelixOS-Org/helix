//! BPF Instructions
//!
//! Low-level BPF instruction definitions for seccomp filters.

/// BPF instruction
#[derive(Debug, Clone, Copy)]
pub struct BpfInsn {
    /// Opcode
    pub code: u16,
    /// Jump true
    pub jt: u8,
    /// Jump false
    pub jf: u8,
    /// Constant
    pub k: u32,
}

impl BpfInsn {
    /// Create new instruction
    #[inline(always)]
    pub const fn new(code: u16, jt: u8, jf: u8, k: u32) -> Self {
        Self { code, jt, jf, k }
    }

    // BPF opcodes
    pub const BPF_LD: u16 = 0x00;
    pub const BPF_LDX: u16 = 0x01;
    pub const BPF_ST: u16 = 0x02;
    pub const BPF_STX: u16 = 0x03;
    pub const BPF_ALU: u16 = 0x04;
    pub const BPF_JMP: u16 = 0x05;
    pub const BPF_RET: u16 = 0x06;
    pub const BPF_MISC: u16 = 0x07;

    pub const BPF_W: u16 = 0x00;
    pub const BPF_H: u16 = 0x08;
    pub const BPF_B: u16 = 0x10;

    pub const BPF_ABS: u16 = 0x20;

    pub const BPF_JEQ: u16 = 0x10;
    pub const BPF_JGT: u16 = 0x20;
    pub const BPF_JGE: u16 = 0x30;
    pub const BPF_JSET: u16 = 0x40;

    pub const BPF_K: u16 = 0x00;
    pub const BPF_A: u16 = 0x10;

    /// Load architecture
    #[inline(always)]
    pub fn load_arch() -> Self {
        Self::new(Self::BPF_LD | Self::BPF_W | Self::BPF_ABS, 0, 0, 4)
    }

    /// Load syscall number
    #[inline(always)]
    pub fn load_syscall() -> Self {
        Self::new(Self::BPF_LD | Self::BPF_W | Self::BPF_ABS, 0, 0, 0)
    }

    /// Jump if equal
    #[inline(always)]
    pub fn jeq(k: u32, jt: u8, jf: u8) -> Self {
        Self::new(Self::BPF_JMP | Self::BPF_JEQ | Self::BPF_K, jt, jf, k)
    }

    /// Return action
    #[inline(always)]
    pub fn ret(action: u32) -> Self {
        Self::new(Self::BPF_RET | Self::BPF_K, 0, 0, action)
    }
}
