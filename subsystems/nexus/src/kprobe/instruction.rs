//! Instruction Analysis
//!
//! Analyzing instructions for probeability.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Architecture, ProbeAddress};

/// Instruction type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionType {
    /// Regular instruction
    Normal,
    /// Branch instruction
    Branch,
    /// Call instruction
    Call,
    /// Return instruction
    Return,
    /// Jump instruction (unconditional)
    Jump,
    /// Conditional jump
    ConditionalJump,
    /// Trap/breakpoint
    Trap,
    /// System call
    Syscall,
    /// Privileged instruction
    Privileged,
    /// Memory access
    MemoryAccess,
    /// Unknown/invalid
    Unknown,
}

/// Instruction info
#[derive(Debug, Clone)]
pub struct InstructionInfo {
    /// Address
    pub address: ProbeAddress,
    /// Length in bytes
    pub length: u8,
    /// Instruction type
    pub inst_type: InstructionType,
    /// Original bytes
    pub bytes: Vec<u8>,
    /// Mnemonic
    pub mnemonic: String,
    /// Can be probed
    pub probeable: bool,
    /// Reason if not probeable
    pub probe_reason: Option<String>,
}

impl InstructionInfo {
    /// Create new instruction info
    pub fn new(address: ProbeAddress, bytes: Vec<u8>) -> Self {
        let length = bytes.len() as u8;
        Self {
            address,
            length,
            inst_type: InstructionType::Normal,
            bytes,
            mnemonic: String::new(),
            probeable: true,
            probe_reason: None,
        }
    }

    /// Mark as not probeable
    pub fn mark_not_probeable(&mut self, reason: String) {
        self.probeable = false;
        self.probe_reason = Some(reason);
    }
}

/// Instruction analyzer
pub struct InstructionAnalyzer {
    /// Architecture
    arch: Architecture,
    /// Analyzed instructions cache
    cache: BTreeMap<ProbeAddress, InstructionInfo>,
    /// Cache hits
    cache_hits: AtomicU64,
    /// Total analyses
    total_analyses: AtomicU64,
}

impl InstructionAnalyzer {
    /// Create new instruction analyzer
    pub fn new(arch: Architecture) -> Self {
        Self {
            arch,
            cache: BTreeMap::new(),
            cache_hits: AtomicU64::new(0),
            total_analyses: AtomicU64::new(0),
        }
    }

    /// Analyze instruction at address
    pub fn analyze(&mut self, address: ProbeAddress, bytes: &[u8]) -> InstructionInfo {
        self.total_analyses.fetch_add(1, Ordering::Relaxed);

        // Check cache
        if let Some(cached) = self.cache.get(&address) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return cached.clone();
        }

        let mut info = InstructionInfo::new(address, bytes.to_vec());

        // Analyze based on architecture
        match self.arch {
            Architecture::X86_64 => self.analyze_x86_64(&mut info),
            Architecture::Aarch64 => self.analyze_aarch64(&mut info),
            Architecture::Riscv64 => self.analyze_riscv64(&mut info),
        }

        self.cache.insert(address, info.clone());
        info
    }

    /// Analyze x86_64 instruction
    fn analyze_x86_64(&self, info: &mut InstructionInfo) {
        if info.bytes.is_empty() {
            info.inst_type = InstructionType::Unknown;
            info.mark_not_probeable(String::from("Empty instruction"));
            return;
        }

        let opcode = info.bytes[0];

        info.inst_type = match opcode {
            0xC3 | 0xCB | 0xC2 | 0xCA => {
                info.mnemonic = String::from("ret");
                info.mark_not_probeable(String::from("Return instruction"));
                InstructionType::Return
            }
            0xE8 => {
                info.mnemonic = String::from("call");
                InstructionType::Call
            }
            0xE9 | 0xEB => {
                info.mnemonic = String::from("jmp");
                InstructionType::Jump
            }
            0x70..=0x7F => {
                info.mnemonic = String::from("jcc");
                InstructionType::ConditionalJump
            }
            0x0F if info.bytes.get(1).map_or(false, |&b| (0x80..=0x8F).contains(&b)) => {
                info.mnemonic = String::from("jcc");
                InstructionType::ConditionalJump
            }
            0xCC => {
                info.mnemonic = String::from("int3");
                info.mark_not_probeable(String::from("Already a breakpoint"));
                InstructionType::Trap
            }
            0x0F if info.bytes.get(1) == Some(&0x05) => {
                info.mnemonic = String::from("syscall");
                InstructionType::Syscall
            }
            _ => {
                info.mnemonic = String::from("unknown");
                InstructionType::Normal
            }
        };
    }

    /// Analyze AArch64 instruction
    fn analyze_aarch64(&self, info: &mut InstructionInfo) {
        if info.bytes.len() < 4 {
            info.inst_type = InstructionType::Unknown;
            info.mark_not_probeable(String::from("Instruction too short"));
            return;
        }

        let inst =
            u32::from_le_bytes([info.bytes[0], info.bytes[1], info.bytes[2], info.bytes[3]]);

        info.inst_type = match inst >> 26 {
            0b000101 => {
                info.mnemonic = String::from("b");
                InstructionType::Branch
            }
            0b100101 => {
                info.mnemonic = String::from("bl");
                InstructionType::Call
            }
            _ if (inst & 0xFFFFFC1F) == 0xD65F0000 => {
                info.mnemonic = String::from("ret");
                info.mark_not_probeable(String::from("Return instruction"));
                InstructionType::Return
            }
            _ if (inst & 0xFFE0001F) == 0xD4000001 => {
                info.mnemonic = String::from("svc");
                InstructionType::Syscall
            }
            _ => {
                info.mnemonic = String::from("unknown");
                InstructionType::Normal
            }
        };
    }

    /// Analyze RISC-V instruction
    fn analyze_riscv64(&self, info: &mut InstructionInfo) {
        if info.bytes.len() < 2 {
            info.inst_type = InstructionType::Unknown;
            info.mark_not_probeable(String::from("Instruction too short"));
            return;
        }

        // Check for compressed instruction
        let is_compressed = (info.bytes[0] & 0x03) != 0x03;

        if is_compressed {
            let inst = u16::from_le_bytes([info.bytes[0], info.bytes[1]]);
            info.length = 2;

            info.inst_type = match inst & 0x3 {
                0b01 if (inst >> 13) == 0b101 => {
                    info.mnemonic = String::from("c.j");
                    InstructionType::Jump
                }
                0b10 if (inst >> 12) == 0b1000 && (inst & 0x007C) == 0 => {
                    info.mnemonic = String::from("c.ret");
                    info.mark_not_probeable(String::from("Return instruction"));
                    InstructionType::Return
                }
                _ => {
                    info.mnemonic = String::from("unknown");
                    InstructionType::Normal
                }
            };
        } else if info.bytes.len() >= 4 {
            let inst = u32::from_le_bytes([
                info.bytes[0],
                info.bytes[1],
                info.bytes[2],
                info.bytes[3],
            ]);
            info.length = 4;

            let opcode = inst & 0x7F;
            info.inst_type = match opcode {
                0b1101111 => {
                    info.mnemonic = String::from("jal");
                    InstructionType::Call
                }
                0b1100111 => {
                    info.mnemonic = String::from("jalr");
                    InstructionType::Return
                }
                0b1100011 => {
                    info.mnemonic = String::from("branch");
                    InstructionType::ConditionalJump
                }
                0b1110011 if inst == 0x00100073 => {
                    info.mnemonic = String::from("ebreak");
                    info.mark_not_probeable(String::from("Already a breakpoint"));
                    InstructionType::Trap
                }
                0b1110011 => {
                    info.mnemonic = String::from("ecall");
                    InstructionType::Syscall
                }
                _ => {
                    info.mnemonic = String::from("unknown");
                    InstructionType::Normal
                }
            };
        }
    }

    /// Check if address is probeable
    pub fn is_probeable(&mut self, address: ProbeAddress, bytes: &[u8]) -> bool {
        let info = self.analyze(address, bytes);
        info.probeable
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f32 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let total = self.total_analyses.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
