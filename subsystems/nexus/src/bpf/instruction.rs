//! BPF Instructions
//!
//! BPF instruction representation and opcodes.

/// BPF instruction opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfOpcode {
    /// Add
    Add,
    /// Subtract
    Sub,
    /// Multiply
    Mul,
    /// Divide
    Div,
    /// Or
    Or,
    /// And
    And,
    /// Left shift
    Lsh,
    /// Right shift
    Rsh,
    /// Negate
    Neg,
    /// Modulo
    Mod,
    /// Xor
    Xor,
    /// Move
    Mov,
    /// Signed right shift
    Arsh,
    /// Jump always
    Ja,
    /// Jump if equal
    Jeq,
    /// Jump if greater than
    Jgt,
    /// Jump if greater or equal
    Jge,
    /// Jump if set
    Jset,
    /// Jump if not equal
    Jne,
    /// Jump if signed greater
    Jsgt,
    /// Jump if signed greater or equal
    Jsge,
    /// Call helper
    Call,
    /// Exit
    Exit,
    /// Jump if less than
    Jlt,
    /// Jump if less or equal
    Jle,
    /// Jump if signed less
    Jslt,
    /// Jump if signed less or equal
    Jsle,
    /// Load
    Ld,
    /// Load absolute
    Ldx,
    /// Store
    St,
    /// Store extend
    Stx,
    /// Atomic
    Atomic,
    /// Unknown
    Unknown,
}

/// BPF instruction
#[derive(Debug, Clone)]
pub struct BpfInsn {
    /// Opcode
    pub opcode: u8,
    /// Destination register
    pub dst_reg: u8,
    /// Source register
    pub src_reg: u8,
    /// Offset
    pub off: i16,
    /// Immediate value
    pub imm: i32,
}

impl BpfInsn {
    /// Create new instruction
    pub fn new(opcode: u8, dst_reg: u8, src_reg: u8, off: i16, imm: i32) -> Self {
        Self {
            opcode,
            dst_reg,
            src_reg,
            off,
            imm,
        }
    }

    /// Decode opcode class
    #[inline(always)]
    pub fn opcode_class(&self) -> u8 {
        self.opcode & 0x07
    }

    /// Is ALU operation
    #[inline(always)]
    pub fn is_alu(&self) -> bool {
        matches!(self.opcode_class(), 0x04 | 0x07)
    }

    /// Is jump operation
    #[inline(always)]
    pub fn is_jump(&self) -> bool {
        matches!(self.opcode_class(), 0x05 | 0x06)
    }

    /// Is memory operation
    #[inline(always)]
    pub fn is_mem(&self) -> bool {
        matches!(self.opcode_class(), 0x00 | 0x01 | 0x02 | 0x03)
    }

    /// Is call operation
    #[inline(always)]
    pub fn is_call(&self) -> bool {
        self.opcode == 0x85
    }

    /// Is exit operation
    #[inline(always)]
    pub fn is_exit(&self) -> bool {
        self.opcode == 0x95
    }
}
