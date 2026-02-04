//! # Firmware Requests
//!
//! This module provides firmware-related Limine requests:
//! - RSDP (ACPI Root System Description Pointer)
//! - SMBIOS tables
//! - EFI System Table
//! - EFI Memory Map
//! - Device Tree Blob (DTB)

#![allow(clippy::cast_possible_truncation)]

use core::slice;

use super::{LimineRequest, ResponsePtr, SafeResponse};
use crate::protocol::request_ids::{
    DTB_ID, EFI_MEMMAP_ID, EFI_SYSTEM_TABLE_ID, RSDP_ID, SMBIOS_ID,
};

// =============================================================================
// RSDP Request (ACPI)
// =============================================================================

/// RSDP (Root System Description Pointer) request
///
/// Provides access to ACPI tables.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::RsdpRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static RSDP: RsdpRequest = RsdpRequest::new();
///
/// fn get_acpi_tables() -> Option<*const u8> {
///     RSDP.response().map(|r| r.address())
/// }
/// ```
#[repr(C)]
pub struct RsdpRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<RsdpResponse>,
}

impl RsdpRequest {
    /// Create a new RSDP request
    pub const fn new() -> Self {
        Self {
            id: RSDP_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for RsdpRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for RsdpRequest {
    type Response = RsdpResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for RsdpRequest {}

/// RSDP response
#[repr(C)]
pub struct RsdpResponse {
    /// Response revision
    revision: u64,
    /// RSDP address
    address: *const u8,
}

impl RsdpResponse {
    /// Get the RSDP address
    pub fn address(&self) -> *const u8 {
        self.address
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get the RSDP signature
    pub fn signature(&self) -> Option<&[u8; 8]> {
        if self.address.is_null() {
            return None;
        }
        unsafe { Some(&*self.address.cast::<[u8; 8]>()) }
    }

    /// Check if this is ACPI 2.0+ (XSDP)
    pub fn is_xsdp(&self) -> bool {
        if self.address.is_null() {
            return false;
        }
        // XSDP has "RSD PTR " signature and revision >= 2
        unsafe {
            let revision_ptr = self.address.add(15);
            *revision_ptr >= 2
        }
    }

    /// Get the ACPI revision
    pub fn acpi_revision(&self) -> Option<u8> {
        if self.address.is_null() {
            return None;
        }
        unsafe { Some(*self.address.add(15)) }
    }

    /// Get the RSDT address (ACPI 1.0)
    #[allow(clippy::cast_ptr_alignment)]
    pub fn rsdt_address(&self) -> Option<u32> {
        if self.address.is_null() {
            return None;
        }
        unsafe { Some(*self.address.add(16).cast::<u32>()) }
    }

    /// Get the XSDT address (ACPI 2.0+)
    #[allow(clippy::cast_ptr_alignment)]
    pub fn xsdt_address(&self) -> Option<u64> {
        if !self.is_xsdp() {
            return None;
        }
        unsafe { Some(*self.address.add(24).cast::<u64>()) }
    }
}

unsafe impl SafeResponse for RsdpResponse {
    fn validate(&self) -> bool {
        if self.address.is_null() {
            return false;
        }

        // Check signature
        if let Some(sig) = self.signature() {
            sig == b"RSD PTR "
        } else {
            false
        }
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for RsdpResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RsdpResponse")
            .field("address", &self.address)
            .field("is_xsdp", &self.is_xsdp())
            .field("acpi_revision", &self.acpi_revision())
            .finish_non_exhaustive()
    }
}

// =============================================================================
// SMBIOS Request
// =============================================================================

/// SMBIOS tables request
///
/// Provides access to SMBIOS (System Management BIOS) tables.
#[repr(C)]
pub struct SmbiosRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<SmbiosResponse>,
}

impl SmbiosRequest {
    /// Create a new SMBIOS request
    pub const fn new() -> Self {
        Self {
            id: SMBIOS_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for SmbiosRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for SmbiosRequest {
    type Response = SmbiosResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for SmbiosRequest {}

/// SMBIOS response
#[repr(C)]
pub struct SmbiosResponse {
    /// Response revision
    revision: u64,
    /// 32-bit entry point
    entry_32: *const u8,
    /// 64-bit entry point
    entry_64: *const u8,
}

impl SmbiosResponse {
    /// Get the 32-bit entry point
    pub fn entry_32(&self) -> Option<*const u8> {
        if self.entry_32.is_null() {
            None
        } else {
            Some(self.entry_32)
        }
    }

    /// Get the 64-bit entry point
    pub fn entry_64(&self) -> Option<*const u8> {
        if self.entry_64.is_null() {
            None
        } else {
            Some(self.entry_64)
        }
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Check if SMBIOS 3.0+ is available (64-bit)
    pub fn has_smbios3(&self) -> bool {
        !self.entry_64.is_null()
    }

    /// Get the SMBIOS version from 32-bit entry
    pub fn version_32(&self) -> Option<(u8, u8)> {
        if self.entry_32.is_null() {
            return None;
        }

        // _SM_ signature at offset 0, version at offset 6 and 7
        unsafe {
            let major = *self.entry_32.add(6);
            let minor = *self.entry_32.add(7);
            Some((major, minor))
        }
    }

    /// Get the SMBIOS version from 64-bit entry
    pub fn version_64(&self) -> Option<(u8, u8, u8)> {
        if self.entry_64.is_null() {
            return None;
        }

        // _SM3_ signature, version at offset 7, 8, 9
        unsafe {
            let major = *self.entry_64.add(7);
            let minor = *self.entry_64.add(8);
            let docrev = *self.entry_64.add(9);
            Some((major, minor, docrev))
        }
    }
}

unsafe impl SafeResponse for SmbiosResponse {
    fn validate(&self) -> bool {
        !self.entry_32.is_null() || !self.entry_64.is_null()
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for SmbiosResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SmbiosResponse")
            .field("has_32bit", &!self.entry_32.is_null())
            .field("has_64bit", &!self.entry_64.is_null())
            .field("version_32", &self.version_32())
            .field("version_64", &self.version_64())
            .finish_non_exhaustive()
    }
}

// =============================================================================
// EFI System Table Request
// =============================================================================

/// EFI System Table request
///
/// Provides access to the UEFI System Table for EFI runtime services.
#[repr(C)]
pub struct EfiSystemTableRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<EfiSystemTableResponse>,
}

impl EfiSystemTableRequest {
    /// Create a new EFI system table request
    pub const fn new() -> Self {
        Self {
            id: EFI_SYSTEM_TABLE_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for EfiSystemTableRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for EfiSystemTableRequest {
    type Response = EfiSystemTableResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for EfiSystemTableRequest {}

/// EFI System Table response
#[repr(C)]
pub struct EfiSystemTableResponse {
    /// Response revision
    revision: u64,
    /// System table address
    address: *const u8,
}

impl EfiSystemTableResponse {
    /// Get the EFI System Table address
    pub fn address(&self) -> *const u8 {
        self.address
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Check if EFI is available
    pub fn is_available(&self) -> bool {
        !self.address.is_null()
    }

    /// Get the EFI System Table as a typed pointer
    ///
    /// # Safety
    ///
    /// The caller must ensure the type T matches the EFI System Table structure.
    pub unsafe fn as_ptr<T>(&self) -> Option<*const T> {
        if self.address.is_null() {
            None
        } else {
            Some(self.address.cast::<T>())
        }
    }
}

unsafe impl SafeResponse for EfiSystemTableResponse {
    fn validate(&self) -> bool {
        !self.address.is_null()
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for EfiSystemTableResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EfiSystemTableResponse")
            .field("address", &self.address)
            .field("available", &self.is_available())
            .finish_non_exhaustive()
    }
}

// =============================================================================
// EFI Memory Map Request
// =============================================================================

/// EFI Memory Map request
///
/// Provides the EFI memory map for systems booted via UEFI.
#[repr(C)]
pub struct EfiMemmapRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<EfiMemmapResponse>,
}

impl EfiMemmapRequest {
    /// Create a new EFI memory map request
    pub const fn new() -> Self {
        Self {
            id: EFI_MEMMAP_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for EfiMemmapRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for EfiMemmapRequest {
    type Response = EfiMemmapResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for EfiMemmapRequest {}

/// EFI Memory Map response
#[repr(C)]
pub struct EfiMemmapResponse {
    /// Response revision
    revision: u64,
    /// Memory map address
    memmap: *const u8,
    /// Memory map size
    memmap_size: u64,
    /// Descriptor size
    desc_size: u64,
    /// Descriptor version
    desc_version: u64,
}

impl EfiMemmapResponse {
    /// Get the memory map address
    pub fn address(&self) -> *const u8 {
        self.memmap
    }

    /// Get the memory map size in bytes
    pub fn size(&self) -> usize {
        self.memmap_size as usize
    }

    /// Get the descriptor size
    pub fn descriptor_size(&self) -> usize {
        self.desc_size as usize
    }

    /// Get the descriptor version
    pub fn descriptor_version(&self) -> u64 {
        self.desc_version
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get the number of descriptors
    pub fn descriptor_count(&self) -> usize {
        if self.desc_size == 0 {
            0
        } else {
            self.memmap_size as usize / self.desc_size as usize
        }
    }

    /// Get the memory map as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        if self.memmap.is_null() || self.memmap_size == 0 {
            return &[];
        }
        unsafe { slice::from_raw_parts(self.memmap, self.memmap_size as usize) }
    }

    /// Get a specific descriptor by index
    pub fn get_descriptor(&self, index: usize) -> Option<&[u8]> {
        if index >= self.descriptor_count() {
            return None;
        }

        let offset = index * self.desc_size as usize;
        let bytes = self.as_bytes();

        if offset + self.desc_size as usize > bytes.len() {
            return None;
        }

        Some(&bytes[offset..offset + self.desc_size as usize])
    }

    /// Iterate over EFI memory descriptors
    pub fn descriptors(&self) -> EfiDescriptorIterator<'_> {
        EfiDescriptorIterator {
            response: self,
            index: 0,
        }
    }
}

unsafe impl SafeResponse for EfiMemmapResponse {
    fn validate(&self) -> bool {
        !self.memmap.is_null() && self.memmap_size > 0 && self.desc_size > 0
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for EfiMemmapResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EfiMemmapResponse")
            .field("size", &self.size())
            .field("descriptor_size", &self.descriptor_size())
            .field("descriptor_count", &self.descriptor_count())
            .field("descriptor_version", &self.desc_version)
            .finish_non_exhaustive()
    }
}

/// Iterator over EFI memory descriptors
pub struct EfiDescriptorIterator<'a> {
    response: &'a EfiMemmapResponse,
    index: usize,
}

impl Iterator for EfiDescriptorIterator<'_> {
    type Item = EfiMemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.response.get_descriptor(self.index)?;
        self.index += 1;

        // Parse the descriptor (standard EFI_MEMORY_DESCRIPTOR layout)
        if bytes.len() < 40 {
            return None;
        }

        Some(EfiMemoryDescriptor {
            memory_type: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            physical_start: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
            virtual_start: u64::from_le_bytes([
                bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                bytes[23],
            ]),
            number_of_pages: u64::from_le_bytes([
                bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30],
                bytes[31],
            ]),
            attribute: u64::from_le_bytes([
                bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38],
                bytes[39],
            ]),
        })
    }
}

/// EFI Memory Descriptor
#[derive(Debug, Clone, Copy)]
pub struct EfiMemoryDescriptor {
    /// Memory type
    pub memory_type: u32,
    /// Physical start address
    pub physical_start: u64,
    /// Virtual start address
    pub virtual_start: u64,
    /// Number of 4KB pages
    pub number_of_pages: u64,
    /// Memory attributes
    pub attribute: u64,
}

impl EfiMemoryDescriptor {
    /// Get the size in bytes
    pub fn size(&self) -> u64 {
        self.number_of_pages * 4096
    }

    /// Get the end address
    pub fn end(&self) -> u64 {
        self.physical_start + self.size()
    }

    /// Get the memory type as an enum
    pub fn memory_type_enum(&self) -> EfiMemoryType {
        EfiMemoryType::from_raw(self.memory_type)
    }
}

/// EFI Memory Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EfiMemoryType {
    /// Reserved
    ReservedMemoryType,
    /// Loader code
    LoaderCode,
    /// Loader data
    LoaderData,
    /// Boot services code
    BootServicesCode,
    /// Boot services data
    BootServicesData,
    /// Runtime services code
    RuntimeServicesCode,
    /// Runtime services data
    RuntimeServicesData,
    /// Conventional memory (usable)
    ConventionalMemory,
    /// Unusable memory
    UnusableMemory,
    /// ACPI reclaim memory
    ACPIReclaimMemory,
    /// ACPI memory NVS
    ACPIMemoryNVS,
    /// Memory mapped I/O
    MemoryMappedIO,
    /// Memory mapped I/O port space
    MemoryMappedIOPortSpace,
    /// Pal code
    PalCode,
    /// Persistent memory
    PersistentMemory,
    /// Unknown type
    Unknown(u32),
}

impl EfiMemoryType {
    /// Convert from raw value
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::ReservedMemoryType,
            1 => Self::LoaderCode,
            2 => Self::LoaderData,
            3 => Self::BootServicesCode,
            4 => Self::BootServicesData,
            5 => Self::RuntimeServicesCode,
            6 => Self::RuntimeServicesData,
            7 => Self::ConventionalMemory,
            8 => Self::UnusableMemory,
            9 => Self::ACPIReclaimMemory,
            10 => Self::ACPIMemoryNVS,
            11 => Self::MemoryMappedIO,
            12 => Self::MemoryMappedIOPortSpace,
            13 => Self::PalCode,
            14 => Self::PersistentMemory,
            other => Self::Unknown(other),
        }
    }

    /// Check if this memory can be used by the OS
    pub fn is_usable(&self) -> bool {
        matches!(
            self,
            Self::BootServicesCode
                | Self::BootServicesData
                | Self::ConventionalMemory
                | Self::LoaderCode
                | Self::LoaderData
        )
    }
}

// =============================================================================
// DTB Request (Device Tree Blob)
// =============================================================================

/// Device Tree Blob (DTB) request
///
/// Provides access to the Device Tree on ARM/RISC-V systems.
#[repr(C)]
pub struct DtbRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<DtbResponse>,
}

impl DtbRequest {
    /// Create a new DTB request
    pub const fn new() -> Self {
        Self {
            id: DTB_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for DtbRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for DtbRequest {
    type Response = DtbResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for DtbRequest {}

/// DTB response
#[repr(C)]
pub struct DtbResponse {
    /// Response revision
    revision: u64,
    /// DTB address
    dtb: *const u8,
}

impl DtbResponse {
    /// Get the DTB address
    pub fn address(&self) -> *const u8 {
        self.dtb
    }

    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Check if DTB is available
    pub fn is_available(&self) -> bool {
        !self.dtb.is_null()
    }

    /// Get the DTB magic number
    pub fn magic(&self) -> Option<u32> {
        if self.dtb.is_null() {
            return None;
        }
        unsafe {
            Some(u32::from_be_bytes([
                *self.dtb,
                *self.dtb.add(1),
                *self.dtb.add(2),
                *self.dtb.add(3),
            ]))
        }
    }

    /// Get the DTB size from header
    pub fn size(&self) -> Option<u32> {
        if self.dtb.is_null() {
            return None;
        }
        unsafe {
            Some(u32::from_be_bytes([
                *self.dtb.add(4),
                *self.dtb.add(5),
                *self.dtb.add(6),
                *self.dtb.add(7),
            ]))
        }
    }

    /// Validate the DTB magic
    pub fn is_valid(&self) -> bool {
        self.magic() == Some(0xD00D_FEED)
    }

    /// Get the DTB as a byte slice
    pub fn as_bytes(&self) -> Option<&[u8]> {
        let size = self.size()? as usize;
        if self.dtb.is_null() || size == 0 {
            return None;
        }
        unsafe { Some(slice::from_raw_parts(self.dtb, size)) }
    }
}

unsafe impl SafeResponse for DtbResponse {
    fn validate(&self) -> bool {
        self.is_valid()
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl core::fmt::Debug for DtbResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DtbResponse")
            .field("address", &self.dtb)
            .field("magic", &self.magic())
            .field("size", &self.size())
            .field("valid", &self.is_valid())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efi_memory_type() {
        assert_eq!(
            EfiMemoryType::from_raw(7),
            EfiMemoryType::ConventionalMemory
        );
        assert!(EfiMemoryType::ConventionalMemory.is_usable());
        assert!(!EfiMemoryType::RuntimeServicesCode.is_usable());
    }
}
