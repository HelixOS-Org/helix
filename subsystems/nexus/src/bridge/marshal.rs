//! # Bridge Marshalling Engine
//!
//! Syscall argument marshalling and serialization:
//! - Type-safe argument encoding/decoding
//! - Buffer management
//! - Pointer validation
//! - Zero-copy transfers
//! - Argument transformation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// MARSHAL TYPES
// ============================================================================

/// Argument type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    /// Integer (register-sized)
    Integer,
    /// Pointer to user buffer
    UserPointer,
    /// File descriptor
    FileDescriptor,
    /// Size/length
    Size,
    /// Flags/bitfield
    Flags,
    /// String pointer
    StringPointer,
    /// Struct pointer
    StructPointer,
    /// Pid/tid
    ProcessId,
}

/// Marshalled value
#[derive(Debug, Clone)]
pub enum MarshalledValue {
    /// Integer
    Int(u64),
    /// Buffer reference
    Buffer { addr: u64, len: usize, writable: bool },
    /// File descriptor
    Fd(i32),
    /// String
    Str { addr: u64, max_len: usize },
    /// Null
    Null,
}

impl MarshalledValue {
    /// As integer
    #[inline]
    pub fn as_int(&self) -> Option<u64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// As fd
    #[inline]
    pub fn as_fd(&self) -> Option<i32> {
        match self {
            Self::Fd(fd) => Some(*fd),
            _ => None,
        }
    }

    /// Is null?
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Is buffer?
    #[inline(always)]
    pub fn is_buffer(&self) -> bool {
        matches!(self, Self::Buffer { .. })
    }
}

// ============================================================================
// ARGUMENT DESCRIPTOR
// ============================================================================

/// Syscall argument descriptor
#[derive(Debug, Clone)]
pub struct ArgDescriptor {
    /// Argument index (0-5 typically)
    pub index: u8,
    /// Type
    pub arg_type: ArgType,
    /// Is optional?
    pub optional: bool,
    /// Valid range for integers
    pub min_value: u64,
    pub max_value: u64,
}

impl ArgDescriptor {
    #[inline]
    pub fn integer(index: u8) -> Self {
        Self {
            index,
            arg_type: ArgType::Integer,
            optional: false,
            min_value: 0,
            max_value: u64::MAX,
        }
    }

    #[inline]
    pub fn user_pointer(index: u8) -> Self {
        Self {
            index,
            arg_type: ArgType::UserPointer,
            optional: false,
            min_value: 0,
            max_value: u64::MAX,
        }
    }

    #[inline]
    pub fn fd(index: u8) -> Self {
        Self {
            index,
            arg_type: ArgType::FileDescriptor,
            optional: false,
            min_value: 0,
            max_value: u64::MAX,
        }
    }

    #[inline]
    pub fn flags(index: u8) -> Self {
        Self {
            index,
            arg_type: ArgType::Flags,
            optional: false,
            min_value: 0,
            max_value: u64::MAX,
        }
    }
}

/// Syscall signature
#[derive(Debug, Clone)]
pub struct SyscallSignature {
    /// Syscall number
    pub syscall_nr: u32,
    /// Arguments
    pub args: Vec<ArgDescriptor>,
    /// Return type
    pub return_type: ArgType,
    /// Has side effects
    pub has_side_effects: bool,
}

impl SyscallSignature {
    pub fn new(syscall_nr: u32) -> Self {
        Self {
            syscall_nr,
            args: Vec::new(),
            return_type: ArgType::Integer,
            has_side_effects: true,
        }
    }

    /// Add argument
    #[inline(always)]
    pub fn arg(mut self, desc: ArgDescriptor) -> Self {
        self.args.push(desc);
        self
    }

    /// Arg count
    #[inline(always)]
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }
}

// ============================================================================
// VALIDATION
// ============================================================================

/// Validation result
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Valid
    Valid,
    /// Invalid argument
    InvalidArg { index: u8, reason: ValidationError },
    /// Missing required argument
    MissingArg { index: u8 },
}

/// Validation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    /// Null pointer
    NullPointer,
    /// Out of range
    OutOfRange,
    /// Invalid alignment
    BadAlignment,
    /// Kernel address in user arg
    KernelAddress,
    /// Invalid file descriptor
    InvalidFd,
    /// Buffer too large
    BufferTooLarge,
    /// Invalid flags
    InvalidFlags,
}

/// Pointer validator
#[derive(Debug, Clone)]
pub struct PointerValidator {
    /// User space start
    pub user_start: u64,
    /// User space end
    pub user_end: u64,
    /// Max buffer size
    pub max_buffer_size: usize,
}

impl PointerValidator {
    pub fn new(user_start: u64, user_end: u64) -> Self {
        Self {
            user_start,
            user_end,
            max_buffer_size: 64 * 1024 * 1024, // 64MB
        }
    }

    /// Validate pointer
    pub fn validate_ptr(&self, addr: u64, size: usize) -> Result<(), ValidationError> {
        if addr == 0 {
            return Err(ValidationError::NullPointer);
        }
        if addr < self.user_start || addr >= self.user_end {
            return Err(ValidationError::KernelAddress);
        }
        if size > self.max_buffer_size {
            return Err(ValidationError::BufferTooLarge);
        }
        let end = addr.checked_add(size as u64).ok_or(ValidationError::BufferTooLarge)?;
        if end > self.user_end {
            return Err(ValidationError::BufferTooLarge);
        }
        Ok(())
    }

    /// Validate alignment
    #[inline]
    pub fn validate_aligned(&self, addr: u64, alignment: usize) -> Result<(), ValidationError> {
        if alignment > 0 && (addr % alignment as u64) != 0 {
            Err(ValidationError::BadAlignment)
        } else {
            Ok(())
        }
    }
}

// ============================================================================
// MARSHAL ENGINE
// ============================================================================

/// Marshal stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MarshalStats {
    /// Signatures registered
    pub signatures: usize,
    /// Marshals performed
    pub marshals: u64,
    /// Validations performed
    pub validations: u64,
    /// Validation failures
    pub validation_failures: u64,
}

/// Marshalling engine
#[repr(align(64))]
pub struct BridgeMarshalEngine {
    /// Registered signatures
    signatures: BTreeMap<u32, SyscallSignature>,
    /// Pointer validator
    validator: PointerValidator,
    /// Stats
    stats: MarshalStats,
}

impl BridgeMarshalEngine {
    pub fn new(user_start: u64, user_end: u64) -> Self {
        Self {
            signatures: BTreeMap::new(),
            validator: PointerValidator::new(user_start, user_end),
            stats: MarshalStats::default(),
        }
    }

    /// Register signature
    #[inline(always)]
    pub fn register(&mut self, sig: SyscallSignature) {
        self.signatures.insert(sig.syscall_nr, sig);
        self.stats.signatures = self.signatures.len();
    }

    /// Marshal arguments
    pub fn marshal(&mut self, syscall_nr: u32, raw_args: &[u64]) -> Vec<MarshalledValue> {
        self.stats.marshals += 1;

        let sig = match self.signatures.get(&syscall_nr) {
            Some(s) => s,
            None => {
                // Unknown syscall, treat all as integers
                return raw_args.iter().map(|&v| MarshalledValue::Int(v)).collect();
            }
        };

        let mut result = Vec::with_capacity(sig.args.len());
        for desc in &sig.args {
            let idx = desc.index as usize;
            let raw = if idx < raw_args.len() {
                raw_args[idx]
            } else if desc.optional {
                result.push(MarshalledValue::Null);
                continue;
            } else {
                0
            };

            let value = match desc.arg_type {
                ArgType::Integer | ArgType::Size | ArgType::ProcessId => {
                    MarshalledValue::Int(raw)
                }
                ArgType::UserPointer | ArgType::StructPointer => {
                    if raw == 0 {
                        MarshalledValue::Null
                    } else {
                        MarshalledValue::Buffer {
                            addr: raw,
                            len: 0,
                            writable: false,
                        }
                    }
                }
                ArgType::FileDescriptor => MarshalledValue::Fd(raw as i32),
                ArgType::Flags => MarshalledValue::Int(raw),
                ArgType::StringPointer => {
                    if raw == 0 {
                        MarshalledValue::Null
                    } else {
                        MarshalledValue::Str {
                            addr: raw,
                            max_len: 4096,
                        }
                    }
                }
            };
            result.push(value);
        }

        result
    }

    /// Validate arguments
    pub fn validate(
        &mut self,
        syscall_nr: u32,
        raw_args: &[u64],
    ) -> Vec<ValidationResult> {
        self.stats.validations += 1;

        let sig = match self.signatures.get(&syscall_nr) {
            Some(s) => s,
            None => return alloc::vec![ValidationResult::Valid],
        };

        let mut results = Vec::new();
        for desc in &sig.args {
            let idx = desc.index as usize;
            if idx >= raw_args.len() {
                if !desc.optional {
                    results.push(ValidationResult::MissingArg { index: desc.index });
                    self.stats.validation_failures += 1;
                }
                continue;
            }

            let raw = raw_args[idx];

            let result = match desc.arg_type {
                ArgType::Integer | ArgType::Size => {
                    if raw < desc.min_value || raw > desc.max_value {
                        self.stats.validation_failures += 1;
                        ValidationResult::InvalidArg {
                            index: desc.index,
                            reason: ValidationError::OutOfRange,
                        }
                    } else {
                        ValidationResult::Valid
                    }
                }
                ArgType::UserPointer | ArgType::StringPointer | ArgType::StructPointer => {
                    if raw == 0 && !desc.optional {
                        self.stats.validation_failures += 1;
                        ValidationResult::InvalidArg {
                            index: desc.index,
                            reason: ValidationError::NullPointer,
                        }
                    } else if raw != 0 {
                        match self.validator.validate_ptr(raw, 1) {
                            Ok(_) => ValidationResult::Valid,
                            Err(e) => {
                                self.stats.validation_failures += 1;
                                ValidationResult::InvalidArg {
                                    index: desc.index,
                                    reason: e,
                                }
                            }
                        }
                    } else {
                        ValidationResult::Valid
                    }
                }
                _ => ValidationResult::Valid,
            };
            results.push(result);
        }

        results
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &MarshalStats {
        &self.stats
    }
}
