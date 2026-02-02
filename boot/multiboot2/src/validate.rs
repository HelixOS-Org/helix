//! # Validation and Error Handling
//!
//! This module provides validation utilities and error types for Multiboot2 parsing.

use core::fmt;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur when parsing Multiboot2 boot information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BootInfoError {
    /// The pointer is null
    NullPointer,

    /// The pointer is not properly aligned (must be 8-byte aligned)
    MisalignedPointer {
        /// The actual address
        address: usize,
        /// Required alignment
        required_alignment: usize,
    },

    /// The total size field is invalid
    InvalidTotalSize {
        /// The reported size
        size: u32,
    },

    /// A tag has an invalid size
    InvalidTagSize {
        /// Tag type
        tag_type: u32,
        /// Reported size
        size: u32,
    },

    /// A tag is not properly aligned
    MisalignedTag {
        /// Tag type
        tag_type: u32,
        /// Offset within boot info
        offset: usize,
    },

    /// A tag extends beyond the boot info bounds
    TagOutOfBounds {
        /// Tag type
        tag_type: u32,
        /// Tag offset
        offset: usize,
        /// Tag size
        size: u32,
        /// Total boot info size
        total_size: u32,
    },

    /// The end tag is missing or invalid
    MissingEndTag,

    /// String data is not valid UTF-8
    InvalidUtf8 {
        /// Tag type containing the string
        tag_type: u32,
    },

    /// Memory map entry has invalid type
    InvalidMemoryType {
        /// The invalid type value
        type_value: u32,
    },

    /// Unexpected data in tag
    CorruptedTag {
        /// Tag type
        tag_type: u32,
        /// Description of corruption
        reason: &'static str,
    },
}

impl fmt::Display for BootInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullPointer => write!(f, "boot info pointer is null"),

            Self::MisalignedPointer {
                address,
                required_alignment,
            } => {
                write!(
                    f,
                    "boot info pointer {:#x} is not {}-byte aligned",
                    address, required_alignment
                )
            },

            Self::InvalidTotalSize { size } => {
                write!(f, "invalid total size: {} bytes", size)
            },

            Self::InvalidTagSize { tag_type, size } => {
                write!(f, "tag type {} has invalid size: {} bytes", tag_type, size)
            },

            Self::MisalignedTag { tag_type, offset } => {
                write!(
                    f,
                    "tag type {} at offset {} is not 8-byte aligned",
                    tag_type, offset
                )
            },

            Self::TagOutOfBounds {
                tag_type,
                offset,
                size,
                total_size,
            } => {
                write!(
                    f,
                    "tag type {} at offset {} with size {} extends beyond boot info ({} bytes)",
                    tag_type, offset, size, total_size
                )
            },

            Self::MissingEndTag => write!(f, "end tag not found"),

            Self::InvalidUtf8 { tag_type } => {
                write!(f, "tag type {} contains invalid UTF-8", tag_type)
            },

            Self::InvalidMemoryType { type_value } => {
                write!(f, "invalid memory region type: {}", type_value)
            },

            Self::CorruptedTag { tag_type, reason } => {
                write!(f, "tag type {} is corrupted: {}", tag_type, reason)
            },
        }
    }
}

// =============================================================================
// Result Type
// =============================================================================

/// Result type for boot info operations
pub type ValidationResult<T> = Result<T, BootInfoError>;

// =============================================================================
// Validation Functions
// =============================================================================

/// Validate that a pointer is suitable for Multiboot2 boot info
///
/// # Arguments
///
/// * `ptr` - The pointer to validate
///
/// # Returns
///
/// `Ok(())` if the pointer is valid, or an appropriate error
pub fn validate_pointer(ptr: *const u8) -> ValidationResult<()> {
    // Check for null
    if ptr.is_null() {
        return Err(BootInfoError::NullPointer);
    }

    // Check alignment (must be 8-byte aligned per spec)
    let addr = ptr as usize;
    if addr % 8 != 0 {
        return Err(BootInfoError::MisalignedPointer {
            address: addr,
            required_alignment: 8,
        });
    }

    Ok(())
}

/// Validate the boot info header (total_size and reserved fields)
///
/// # Arguments
///
/// * `total_size` - The total size field from the boot info
///
/// # Returns
///
/// `Ok(())` if the header is valid
pub fn validate_header(total_size: u32) -> ValidationResult<()> {
    // Minimum size: 8 bytes (header) + 8 bytes (end tag) = 16 bytes
    if total_size < 16 {
        return Err(BootInfoError::InvalidTotalSize { size: total_size });
    }

    // Maximum reasonable size (prevent obviously corrupted values)
    // Boot info should fit in the first few MB
    const MAX_REASONABLE_SIZE: u32 = 16 * 1024 * 1024; // 16 MB
    if total_size > MAX_REASONABLE_SIZE {
        return Err(BootInfoError::InvalidTotalSize { size: total_size });
    }

    Ok(())
}

/// Validate a tag header
///
/// # Arguments
///
/// * `tag_type` - The tag type
/// * `tag_size` - The tag size
/// * `offset` - Offset of tag within boot info
/// * `total_size` - Total boot info size
///
/// # Returns
///
/// `Ok(())` if the tag is valid
pub fn validate_tag(
    tag_type: u32,
    tag_size: u32,
    offset: usize,
    total_size: u32,
) -> ValidationResult<()> {
    // Check alignment
    if offset % 8 != 0 {
        return Err(BootInfoError::MisalignedTag { tag_type, offset });
    }

    // Minimum tag size is 8 bytes (type + size)
    if tag_size < 8 {
        return Err(BootInfoError::InvalidTagSize {
            tag_type,
            size: tag_size,
        });
    }

    // Check bounds
    let end_offset = offset.saturating_add(tag_size as usize);
    if end_offset > total_size as usize {
        return Err(BootInfoError::TagOutOfBounds {
            tag_type,
            offset,
            size: tag_size,
            total_size,
        });
    }

    Ok(())
}

// =============================================================================
// Safety Invariants
// =============================================================================

/// Document the safety invariants for Multiboot2 parsing
///
/// This module uses unsafe code to interpret raw memory as Multiboot2
/// structures. The following invariants must hold:
///
/// 1. **Pointer Validity**: The boot info pointer must point to valid,
///    readable memory that remains valid for the lifetime of parsing.
///
/// 2. **Alignment**: The boot info pointer must be 8-byte aligned.
///    All tags within the boot info are also 8-byte aligned.
///
/// 3. **Size Bounds**: The `total_size` field accurately reflects the
///    total size of the boot info structure. No tag extends beyond this.
///
/// 4. **Tag Validity**: Each tag's `size` field accurately reflects the
///    tag's actual size. Tag data is valid for the tag type.
///
/// 5. **Termination**: The boot info ends with a valid end tag (type=0, size=8).
///
/// 6. **No Aliasing**: The boot info memory is not modified during parsing.
///    The lifetime `'boot` ensures parsed references don't outlive the data.
pub mod safety_invariants {
    /// Marker type asserting pointer is valid
    pub struct ValidPointer;

    /// Marker type asserting alignment is correct
    pub struct Aligned;

    /// Marker type asserting bounds are checked
    pub struct BoundsChecked;

    /// Combined safety proof
    pub struct SafetyProof {
        _pointer: ValidPointer,
        _aligned: Aligned,
        _bounds: BoundsChecked,
    }

    impl SafetyProof {
        /// Create a new safety proof (internal use only)
        #[allow(dead_code)]
        pub(crate) const fn new() -> Self {
            Self {
                _pointer: ValidPointer,
                _aligned: Aligned,
                _bounds: BoundsChecked,
            }
        }
    }
}
