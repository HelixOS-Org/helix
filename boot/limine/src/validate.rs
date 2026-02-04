//! # Validation Module
//!
//! This module provides comprehensive validation for Limine boot data.
//! It ensures that all responses from the bootloader are valid and safe
//! to use before passing them to kernel code.
//!
//! ## Features
//!
//! - Memory map validation (overlapping regions, valid types)
//! - Address range validation
//! - Pointer safety checks
//! - Invariant verification

use crate::boot_info::BootInfo;
use crate::requests::*;

/// Validation result type
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Validation error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A required response is missing
    MissingResponse(&'static str),
    /// A pointer is null when it shouldn't be
    NullPointer(&'static str),
    /// An address is not properly aligned
    MisalignedAddress {
        address: u64,
        required_alignment: u64,
    },
    /// A memory region is invalid
    InvalidMemoryRegion {
        base: u64,
        length: u64,
        reason: &'static str,
    },
    /// Memory regions overlap
    OverlappingRegions {
        region1: (u64, u64),
        region2: (u64, u64),
    },
    /// An invalid value was encountered
    InvalidValue { field: &'static str, value: u64 },
    /// Structure size mismatch
    SizeMismatch { expected: usize, actual: usize },
    /// Checksum validation failed
    ChecksumFailed(&'static str),
    /// Magic number validation failed
    InvalidMagic { expected: u64, actual: u64 },
    /// Response revision too old
    RevisionTooOld { minimum: u64, actual: u64 },
    /// Custom validation error
    Custom(&'static str),
}

impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingResponse(name) => write!(f, "Missing response: {}", name),
            Self::NullPointer(field) => write!(f, "Null pointer: {}", field),
            Self::MisalignedAddress {
                address,
                required_alignment,
            } => {
                write!(
                    f,
                    "Address {:#x} not aligned to {}",
                    address, required_alignment
                )
            },
            Self::InvalidMemoryRegion {
                base,
                length,
                reason,
            } => {
                write!(
                    f,
                    "Invalid memory region {:#x}-{:#x}: {}",
                    base,
                    base + length,
                    reason
                )
            },
            Self::OverlappingRegions { region1, region2 } => {
                write!(
                    f,
                    "Overlapping regions: {:#x}-{:#x} and {:#x}-{:#x}",
                    region1.0, region1.1, region2.0, region2.1
                )
            },
            Self::InvalidValue { field, value } => {
                write!(f, "Invalid value for {}: {:#x}", field, value)
            },
            Self::SizeMismatch { expected, actual } => {
                write!(f, "Size mismatch: expected {}, got {}", expected, actual)
            },
            Self::ChecksumFailed(structure) => {
                write!(f, "Checksum validation failed for {}", structure)
            },
            Self::InvalidMagic { expected, actual } => {
                write!(
                    f,
                    "Invalid magic: expected {:#x}, got {:#x}",
                    expected, actual
                )
            },
            Self::RevisionTooOld { minimum, actual } => {
                write!(
                    f,
                    "Revision {} too old, minimum required: {}",
                    actual, minimum
                )
            },
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

/// Collection of validation errors
#[derive(Debug, Default)]
pub struct ValidationErrors {
    errors: [Option<ValidationError>; 16],
    count: usize,
}

impl ValidationErrors {
    /// Create a new empty error collection
    pub const fn new() -> Self {
        Self {
            errors: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ],
            count: 0,
        }
    }

    /// Add an error
    pub fn push(&mut self, error: ValidationError) {
        if self.count < self.errors.len() {
            self.errors[self.count] = Some(error);
            self.count += 1;
        }
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        self.count > 0
    }

    /// Get the number of errors
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate over errors
    pub fn iter(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors[..self.count].iter().filter_map(|e| e.as_ref())
    }

    /// Get the first error
    pub fn first(&self) -> Option<&ValidationError> {
        self.errors[0].as_ref()
    }

    /// Convert to result (Ok if no errors, Err with first error otherwise)
    pub fn into_result(self) -> ValidationResult<()> {
        if let Some(error) = self.errors.into_iter().flatten().next() {
            Err(error)
        } else {
            Ok(())
        }
    }
}

/// Validator for boot information
pub struct BootValidator<'a> {
    boot_info: &'a BootInfo<'a>,
    errors: ValidationErrors,
    strict: bool,
}

impl<'a> BootValidator<'a> {
    /// Create a new validator
    pub fn new(boot_info: &'a BootInfo<'a>) -> Self {
        Self {
            boot_info,
            errors: ValidationErrors::new(),
            strict: false,
        }
    }

    /// Enable strict validation mode
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Run all validations
    pub fn validate(mut self) -> ValidationResult<()> {
        self.validate_memory_map();
        self.validate_hhdm();
        self.validate_kernel_address();

        if self.strict {
            self.validate_framebuffer();
            self.validate_smp();
            self.validate_firmware();
        }

        self.errors.into_result()
    }

    /// Validate memory map
    fn validate_memory_map(&mut self) {
        let Some(memmap) = self.boot_info.memory_map() else {
            self.errors
                .push(ValidationError::MissingResponse("memory_map"));
            return;
        };

        let entries: heapless::Vec<MemoryEntry, 256> = memmap.entries().collect();

        // Check for empty memory map
        if entries.is_empty() {
            self.errors
                .push(ValidationError::Custom("Memory map is empty"));
            return;
        }

        // Validate each entry
        for entry in entries.iter() {
            // Check for zero-length regions
            if entry.length() == 0 {
                self.errors.push(ValidationError::InvalidMemoryRegion {
                    base: entry.base(),
                    length: 0,
                    reason: "zero-length region",
                });
            }

            // Check for overflow
            if entry.base().checked_add(entry.length()).is_none() {
                self.errors.push(ValidationError::InvalidMemoryRegion {
                    base: entry.base(),
                    length: entry.length(),
                    reason: "address overflow",
                });
            }

            // Check alignment for usable regions
            if entry.is_usable() && entry.base() & 0xFFF != 0 {
                self.errors.push(ValidationError::MisalignedAddress {
                    address: entry.base(),
                    required_alignment: 4096,
                });
            }
        }

        // Check for overlapping regions
        for i in 0..entries.len() {
            for j in (i + 1)..entries.len() {
                let a = &entries[i];
                let b = &entries[j];

                if a.overlaps(b) {
                    self.errors.push(ValidationError::OverlappingRegions {
                        region1: (a.base(), a.end()),
                        region2: (b.base(), b.end()),
                    });
                }
            }
        }

        // Check for minimum usable memory
        let usable = memmap.total_usable_memory();
        if usable < 1024 * 1024 {
            self.errors
                .push(ValidationError::Custom("Less than 1 MB usable memory"));
        }
    }

    /// Validate HHDM
    fn validate_hhdm(&mut self) {
        let Some(hhdm) = self.boot_info.hhdm() else {
            self.errors.push(ValidationError::MissingResponse("hhdm"));
            return;
        };

        let offset = hhdm.offset();

        // HHDM should be in higher half
        if offset < 0xFFFF_8000_0000_0000 {
            self.errors.push(ValidationError::InvalidValue {
                field: "hhdm_offset",
                value: offset,
            });
        }

        // HHDM should be page-aligned
        if offset & 0xFFF != 0 {
            self.errors.push(ValidationError::MisalignedAddress {
                address: offset,
                required_alignment: 4096,
            });
        }
    }

    /// Validate kernel address
    fn validate_kernel_address(&mut self) {
        let Some(kernel_addr) = self.boot_info.kernel_address() else {
            if self.strict {
                self.errors
                    .push(ValidationError::MissingResponse("kernel_address"));
            }
            return;
        };

        let phys = kernel_addr.physical_base();
        let virt = kernel_addr.virtual_base();

        // Physical address should be reasonably low
        if phys > 0x1_0000_0000_0000 {
            self.errors.push(ValidationError::InvalidValue {
                field: "kernel_physical_base",
                value: phys,
            });
        }

        // Virtual address should be in higher half
        if virt < 0xFFFF_8000_0000_0000 {
            self.errors.push(ValidationError::InvalidValue {
                field: "kernel_virtual_base",
                value: virt,
            });
        }

        // Both should be page-aligned
        if phys & 0xFFF != 0 {
            self.errors.push(ValidationError::MisalignedAddress {
                address: phys,
                required_alignment: 4096,
            });
        }

        if virt & 0xFFF != 0 {
            self.errors.push(ValidationError::MisalignedAddress {
                address: virt,
                required_alignment: 4096,
            });
        }
    }

    /// Validate framebuffer
    fn validate_framebuffer(&mut self) {
        let Some(fb_response) = self.boot_info.framebuffer() else {
            return; // Framebuffer is optional
        };

        for (_i, fb) in fb_response.iter().enumerate() {
            // Check for valid dimensions
            if fb.width() == 0 || fb.height() == 0 {
                self.errors
                    .push(ValidationError::Custom("Framebuffer has zero dimensions"));
            }

            // Check for valid BPP
            if !matches!(fb.bpp(), 15 | 16 | 24 | 32) {
                self.errors.push(ValidationError::InvalidValue {
                    field: "framebuffer_bpp",
                    value: fb.bpp() as u64,
                });
            }

            // Check pitch is at least width * bytes_per_pixel
            let min_pitch = fb.width() * fb.bytes_per_pixel();
            if fb.pitch() < min_pitch {
                self.errors.push(ValidationError::InvalidValue {
                    field: "framebuffer_pitch",
                    value: fb.pitch() as u64,
                });
            }

            // Check address is not null
            if fb.address().is_null() {
                self.errors
                    .push(ValidationError::NullPointer("framebuffer_address"));
            }
        }
    }

    /// Validate SMP
    fn validate_smp(&mut self) {
        let Some(smp) = self.boot_info.smp() else {
            return; // SMP is optional
        };

        // Should have at least one CPU
        if smp.cpu_count() == 0 {
            self.errors
                .push(ValidationError::Custom("SMP reports zero CPUs"));
        }

        // BSP should exist
        if smp.bsp().is_none() {
            self.errors
                .push(ValidationError::Custom("Cannot find BSP in CPU list"));
        }
    }

    /// Validate firmware tables
    fn validate_firmware(&mut self) {
        // Validate RSDP if present
        if let Some(rsdp) = self.boot_info.rsdp() {
            if let Some(sig) = rsdp.signature() {
                if sig != b"RSD PTR " {
                    self.errors
                        .push(ValidationError::Custom("Invalid RSDP signature"));
                }
            }
        }

        // Validate DTB if present
        if let Some(dtb) = self.boot_info.dtb() {
            if !dtb.is_valid() {
                self.errors
                    .push(ValidationError::Custom("Invalid DTB magic"));
            }
        }
    }
}

/// Validate a memory map response
pub fn validate_memory_map(response: &MemoryMapResponse) -> ValidationResult<()> {
    let mut errors = ValidationErrors::new();

    if response.entry_count() == 0 {
        errors.push(ValidationError::Custom("Empty memory map"));
    }

    errors.into_result()
}

/// Validate an HHDM response
pub fn validate_hhdm(response: &HhdmResponse) -> ValidationResult<()> {
    let offset = response.offset();

    if offset < 0xFFFF_8000_0000_0000 {
        return Err(ValidationError::InvalidValue {
            field: "hhdm_offset",
            value: offset,
        });
    }

    if offset & 0xFFF != 0 {
        return Err(ValidationError::MisalignedAddress {
            address: offset,
            required_alignment: 4096,
        });
    }

    Ok(())
}

/// Check if an address is properly aligned
pub fn is_aligned(address: u64, alignment: u64) -> bool {
    address & (alignment - 1) == 0
}

/// Check if an address is page-aligned (4 KB)
pub fn is_page_aligned(address: u64) -> bool {
    is_aligned(address, 4096)
}

/// Check if an address is in the higher half
pub fn is_higher_half(address: u64) -> bool {
    address >= 0xFFFF_8000_0000_0000
}

/// Check if an address range is valid
pub fn is_valid_range(base: u64, length: u64) -> bool {
    length > 0 && base.checked_add(length).is_some()
}

/// Validate a pointer is not null and aligned
pub fn validate_pointer<T>(ptr: *const T) -> ValidationResult<()> {
    if ptr.is_null() {
        return Err(ValidationError::NullPointer("pointer"));
    }

    let addr = ptr as u64;
    let align = core::mem::align_of::<T>() as u64;

    if !is_aligned(addr, align) {
        return Err(ValidationError::MisalignedAddress {
            address: addr,
            required_alignment: align,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0x1000, 0x1000));
        assert!(is_aligned(0x2000, 0x1000));
        assert!(!is_aligned(0x1001, 0x1000));
    }

    #[test]
    fn test_is_higher_half() {
        assert!(is_higher_half(0xFFFF_8000_0000_0000));
        assert!(is_higher_half(0xFFFF_FFFF_FFFF_FFFF));
        assert!(!is_higher_half(0x0000_0000_0000_0000));
        assert!(!is_higher_half(0x0000_7FFF_FFFF_FFFF));
    }

    #[test]
    fn test_is_valid_range() {
        assert!(is_valid_range(0x1000, 0x1000));
        assert!(!is_valid_range(0x1000, 0));
        assert!(!is_valid_range(u64::MAX, 10));
    }

    #[test]
    fn test_validation_errors() {
        let mut errors = ValidationErrors::new();
        assert!(errors.is_empty());

        errors.push(ValidationError::Custom("test error"));
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1);
    }
}
