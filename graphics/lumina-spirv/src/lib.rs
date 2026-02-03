//! # Lumina SPIR-V
//!
//! SPIR-V code generation, manipulation, and reflection for the Lumina GPU framework.
//!
//! This crate provides:
//! - SPIR-V binary builder
//! - SPIR-V disassembler
//! - SPIR-V reflection for resource binding
//! - SPIR-V optimization passes
//! - SPIR-V validation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    LUMINA SPIR-V PIPELINE                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │   Lumina IR                                                     │
//! │       │                                                         │
//! │       ▼                                                         │
//! │   ┌─────────────────┐                                           │
//! │   │ SPIR-V Builder  │ ◄── Generate SPIR-V from IR               │
//! │   └────────┬────────┘                                           │
//! │            │                                                    │
//! │            ▼                                                    │
//! │   ┌─────────────────┐                                           │
//! │   │ SPIR-V Module   │ ◄── In-memory SPIR-V representation       │
//! │   └────────┬────────┘                                           │
//! │            │                                                    │
//! │     ┌──────┴──────┬──────────────┐                              │
//! │     ▼             ▼              ▼                              │
//! │ ┌────────┐  ┌──────────┐  ┌────────────┐                        │
//! │ │Optimize│  │Reflection│  │ Validation │                        │
//! │ └────┬───┘  └────┬─────┘  └─────┬──────┘                        │
//! │      │           │              │                               │
//! │      └───────────┴──────────────┘                               │
//! │                  │                                              │
//! │                  ▼                                              │
//! │   ┌─────────────────┐                                           │
//! │   │ SPIR-V Binary   │ ◄── Final shader binary                   │
//! │   └─────────────────┘                                           │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};

pub mod binary;
pub mod builder;
pub mod disasm;
pub mod instruction;
pub mod module;
pub mod opcode;
pub mod optimizer;
pub mod reflect;
pub mod types;
pub mod validate;

// Re-exports
pub use binary::*;
pub use builder::SpirVBuilder;
pub use module::SpirVModule;
pub use reflect::Reflection;

/// SPIR-V magic number
pub const SPIRV_MAGIC: u32 = 0x07230203;

/// SPIR-V version 1.0
pub const SPIRV_VERSION_1_0: u32 = 0x00010000;
/// SPIR-V version 1.1
pub const SPIRV_VERSION_1_1: u32 = 0x00010100;
/// SPIR-V version 1.2
pub const SPIRV_VERSION_1_2: u32 = 0x00010200;
/// SPIR-V version 1.3
pub const SPIRV_VERSION_1_3: u32 = 0x00010300;
/// SPIR-V version 1.4
pub const SPIRV_VERSION_1_4: u32 = 0x00010400;
/// SPIR-V version 1.5
pub const SPIRV_VERSION_1_5: u32 = 0x00010500;
/// SPIR-V version 1.6
pub const SPIRV_VERSION_1_6: u32 = 0x00010600;

/// Generator magic number for Lumina
pub const LUMINA_GENERATOR_MAGIC: u32 = 0x4C554D49; // "LUMI"

/// Error type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpirVError {
    /// Invalid SPIR-V magic number
    InvalidMagic(u32),
    /// Unsupported SPIR-V version
    UnsupportedVersion(u32),
    /// Invalid opcode
    InvalidOpcode(u16),
    /// Invalid operand
    InvalidOperand(String),
    /// Missing ID
    MissingId(u32),
    /// Duplicate ID
    DuplicateId(u32),
    /// Type mismatch
    TypeMismatch { expected: u32, found: u32 },
    /// Invalid instruction length
    InvalidInstructionLength { expected: u16, found: u16 },
    /// Unexpected end of input
    UnexpectedEof,
    /// Invalid capability
    InvalidCapability(u32),
    /// Missing capability
    MissingCapability(String),
    /// Invalid decoration
    InvalidDecoration(u32),
    /// Validation error
    ValidationError(String),
    /// Unsupported feature
    Unsupported(String),
}

impl core::fmt::Display for SpirVError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpirVError::InvalidMagic(m) => write!(f, "Invalid SPIR-V magic: 0x{:08x}", m),
            SpirVError::UnsupportedVersion(v) => {
                write!(
                    f,
                    "Unsupported SPIR-V version: {}.{}",
                    v >> 16,
                    (v >> 8) & 0xff
                )
            },
            SpirVError::InvalidOpcode(op) => write!(f, "Invalid opcode: {}", op),
            SpirVError::InvalidOperand(msg) => write!(f, "Invalid operand: {}", msg),
            SpirVError::MissingId(id) => write!(f, "Missing ID: {}", id),
            SpirVError::DuplicateId(id) => write!(f, "Duplicate ID: {}", id),
            SpirVError::TypeMismatch { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            },
            SpirVError::InvalidInstructionLength { expected, found } => {
                write!(
                    f,
                    "Invalid instruction length: expected {}, found {}",
                    expected, found
                )
            },
            SpirVError::UnexpectedEof => write!(f, "Unexpected end of input"),
            SpirVError::InvalidCapability(c) => write!(f, "Invalid capability: {}", c),
            SpirVError::MissingCapability(cap) => write!(f, "Missing capability: {}", cap),
            SpirVError::InvalidDecoration(d) => write!(f, "Invalid decoration: {}", d),
            SpirVError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            SpirVError::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SpirVError {}

/// Result type for SPIR-V operations
pub type SpirVResult<T> = Result<T, SpirVError>;
