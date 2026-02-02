//! Serial I/O Protocol
//!
//! Provides access to serial devices for debugging and communication.

use core::fmt;

use crate::raw::types::*;

// =============================================================================
// SERIAL I/O PROTOCOL
// =============================================================================

/// Serial I/O Protocol
#[repr(C)]
pub struct EfiSerialIoProtocol {
    /// Revision
    pub revision: u32,

    /// Reset the device
    pub reset: unsafe extern "efiapi" fn(this: *mut Self) -> Status,

    /// Set control attributes
    pub set_attributes: unsafe extern "efiapi" fn(
        this: *mut Self,
        baud_rate: u64,
        receive_fifo_depth: u32,
        timeout: u32,
        parity: EfiParity,
        data_bits: u8,
        stop_bits: EfiStopBits,
    ) -> Status,

    /// Set control bits
    pub set_control: unsafe extern "efiapi" fn(this: *mut Self, control: u32) -> Status,

    /// Get control bits
    pub get_control: unsafe extern "efiapi" fn(this: *mut Self, control: *mut u32) -> Status,

    /// Write data
    pub write: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *const core::ffi::c_void,
    ) -> Status,

    /// Read data
    pub read: unsafe extern "efiapi" fn(
        this: *mut Self,
        buffer_size: *mut usize,
        buffer: *mut core::ffi::c_void,
    ) -> Status,

    /// Mode
    pub mode: *mut EfiSerialIoMode,
}

impl EfiSerialIoProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::SERIAL_IO_PROTOCOL;

    /// Revision 1.0
    pub const REVISION_1_0: u32 = 0x00010000;

    /// Revision 1.1
    pub const REVISION_1_1: u32 = 0x00010001;

    /// Reset the serial device
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn reset_device(&mut self) -> Result<(), Status> {
        let status = (self.reset)(self);
        status.to_status_result()
    }

    /// Set serial port attributes
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn set_port_attributes(
        &mut self,
        baud_rate: u64,
        parity: EfiParity,
        data_bits: u8,
        stop_bits: EfiStopBits,
    ) -> Result<(), Status> {
        let status = (self.set_attributes)(
            self, baud_rate, 0, // Default FIFO depth
            0, // Default timeout
            parity, data_bits, stop_bits,
        );
        status.to_status_result()
    }

    /// Set control signals
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn set_control_bits(&mut self, control: SerialControl) -> Result<(), Status> {
        let status = (self.set_control)(self, control.bits());
        status.to_status_result()
    }

    /// Get control signals
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_control_bits(&self) -> Result<SerialControl, Status> {
        let mut control = 0u32;
        let status = (self.get_control)(self as *const _ as *mut _, &mut control);
        status.to_status_result_with(SerialControl::from_bits_truncate(control))
    }

    /// Write bytes to serial port
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn write_bytes(&mut self, data: &[u8]) -> Result<usize, Status> {
        let mut size = data.len();
        let status = (self.write)(self, &mut size, data.as_ptr() as *const core::ffi::c_void);
        status.to_status_result_with(size)
    }

    /// Read bytes from serial port
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<usize, Status> {
        let mut size = buffer.len();
        let status = (self.read)(
            self,
            &mut size,
            buffer.as_mut_ptr() as *mut core::ffi::c_void,
        );
        status.to_status_result_with(size)
    }

    /// Write a string to serial port
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn write_str(&mut self, s: &str) -> Result<usize, Status> {
        self.write_bytes(s.as_bytes())
    }

    /// Get current mode
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn get_mode(&self) -> Option<&EfiSerialIoMode> {
        if self.mode.is_null() {
            None
        } else {
            Some(&*self.mode)
        }
    }

    /// Check if data is available
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn data_available(&self) -> Result<bool, Status> {
        let control = self.get_control_bits()?;
        Ok(control.contains(SerialControl::INPUT_BUFFER_FULL))
    }
}

impl fmt::Debug for EfiSerialIoProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiSerialIoProtocol")
            .field("revision", &self.revision)
            .finish()
    }
}

// =============================================================================
// SERIAL I/O MODE
// =============================================================================

/// Serial I/O Mode
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiSerialIoMode {
    /// Control mask
    pub control_mask: u32,
    /// Timeout
    pub timeout: u32,
    /// Baud rate
    pub baud_rate: u64,
    /// Receive FIFO depth
    pub receive_fifo_depth: u32,
    /// Data bits
    pub data_bits: u32,
    /// Parity
    pub parity: u32,
    /// Stop bits
    pub stop_bits: u32,
}

impl EfiSerialIoMode {
    /// Get parity setting
    pub fn parity(&self) -> EfiParity {
        match self.parity {
            0 => EfiParity::Default,
            1 => EfiParity::None,
            2 => EfiParity::Even,
            3 => EfiParity::Odd,
            4 => EfiParity::Mark,
            5 => EfiParity::Space,
            _ => EfiParity::Default,
        }
    }

    /// Get stop bits setting
    pub fn stop_bits(&self) -> EfiStopBits {
        match self.stop_bits {
            0 => EfiStopBits::Default,
            1 => EfiStopBits::One,
            2 => EfiStopBits::OneFive,
            3 => EfiStopBits::Two,
            _ => EfiStopBits::Default,
        }
    }
}

// =============================================================================
// PARITY
// =============================================================================

/// Parity type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiParity {
    /// Default parity
    Default = 0,
    /// No parity
    None    = 1,
    /// Even parity
    Even    = 2,
    /// Odd parity
    Odd     = 3,
    /// Mark parity
    Mark    = 4,
    /// Space parity
    Space   = 5,
}

impl Default for EfiParity {
    fn default() -> Self {
        Self::Default
    }
}

// =============================================================================
// STOP BITS
// =============================================================================

/// Stop bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiStopBits {
    /// Default stop bits
    Default = 0,
    /// One stop bit
    One     = 1,
    /// One and a half stop bits
    OneFive = 2,
    /// Two stop bits
    Two     = 3,
}

impl Default for EfiStopBits {
    fn default() -> Self {
        Self::Default
    }
}

// =============================================================================
// CONTROL BITS
// =============================================================================

/// Serial control bits
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SerialControl(u32);

impl SerialControl {
    /// Data terminal ready
    pub const DTR: Self = Self(0x0001);
    /// Request to send
    pub const RTS: Self = Self(0x0002);
    /// Clear to send
    pub const CTS: Self = Self(0x0010);
    /// Data set ready
    pub const DSR: Self = Self(0x0020);
    /// Ring indicator
    pub const RI: Self = Self(0x0040);
    /// Carrier detect
    pub const DCD: Self = Self(0x0080);
    /// Input buffer full
    pub const INPUT_BUFFER_FULL: Self = Self(0x0100);
    /// Output buffer empty
    pub const OUTPUT_BUFFER_EMPTY: Self = Self(0x0200);
    /// Hardware loopback enable
    pub const HARDWARE_LOOPBACK: Self = Self(0x1000);
    /// Software loopback enable
    pub const SOFTWARE_LOOPBACK: Self = Self(0x2000);
    /// Hardware flow control enable
    pub const HARDWARE_FLOW_CONTROL: Self = Self(0x4000);

    /// Create empty control bits
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create from raw bits
    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits)
    }

    /// Get raw bits
    pub const fn bits(&self) -> u32 {
        self.0
    }

    /// Check if contains flag
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Insert flag
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Remove flag
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

impl core::ops::BitOr for SerialControl {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for SerialControl {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl fmt::Debug for SerialControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_set();
        if self.contains(Self::DTR) {
            list.entry(&"DTR");
        }
        if self.contains(Self::RTS) {
            list.entry(&"RTS");
        }
        if self.contains(Self::CTS) {
            list.entry(&"CTS");
        }
        if self.contains(Self::DSR) {
            list.entry(&"DSR");
        }
        if self.contains(Self::RI) {
            list.entry(&"RI");
        }
        if self.contains(Self::DCD) {
            list.entry(&"DCD");
        }
        if self.contains(Self::INPUT_BUFFER_FULL) {
            list.entry(&"INPUT_BUFFER_FULL");
        }
        if self.contains(Self::OUTPUT_BUFFER_EMPTY) {
            list.entry(&"OUTPUT_BUFFER_EMPTY");
        }
        if self.contains(Self::HARDWARE_LOOPBACK) {
            list.entry(&"HARDWARE_LOOPBACK");
        }
        if self.contains(Self::SOFTWARE_LOOPBACK) {
            list.entry(&"SOFTWARE_LOOPBACK");
        }
        if self.contains(Self::HARDWARE_FLOW_CONTROL) {
            list.entry(&"HARDWARE_FLOW_CONTROL");
        }
        list.finish()
    }
}

// =============================================================================
// COMMON BAUD RATES
// =============================================================================

/// Common serial baud rates
pub mod baud_rates {
    /// 9600 baud
    pub const B9600: u64 = 9600;
    /// 19200 baud
    pub const B19200: u64 = 19200;
    /// 38400 baud
    pub const B38400: u64 = 38400;
    /// 57600 baud
    pub const B57600: u64 = 57600;
    /// 115200 baud
    pub const B115200: u64 = 115200;
    /// 230400 baud
    pub const B230400: u64 = 230400;
    /// 460800 baud
    pub const B460800: u64 = 460800;
    /// 921600 baud
    pub const B921600: u64 = 921600;
    /// 1.5M baud
    pub const B1500000: u64 = 1_500_000;
    /// 3M baud
    pub const B3000000: u64 = 3_000_000;
}

// =============================================================================
// DEBUG PORT
// =============================================================================

/// Debug port table (ACPI DBG2)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DebugPortTable {
    /// ACPI header
    pub header: DebugPortHeader,
    /// Offset to debug device information
    pub info_offset: u32,
    /// Number of debug devices
    pub num_entries: u32,
}

/// Debug port header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DebugPortHeader {
    /// Signature
    pub signature: [u8; 4],
    /// Length
    pub length: u32,
    /// Revision
    pub revision: u8,
    /// Checksum
    pub checksum: u8,
    /// OEM ID
    pub oem_id: [u8; 6],
    /// OEM table ID
    pub oem_table_id: [u8; 8],
    /// OEM revision
    pub oem_revision: u32,
    /// Creator ID
    pub creator_id: [u8; 4],
    /// Creator revision
    pub creator_revision: u32,
}

/// Debug device information
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DebugDeviceInfo {
    /// Revision
    pub revision: u8,
    /// Length
    pub length: u16,
    /// Number of generic address registers
    pub num_registers: u8,
    /// Namespace string length
    pub namespace_length: u16,
    /// Namespace string offset
    pub namespace_offset: u16,
    /// OEM data length
    pub oem_data_length: u16,
    /// OEM data offset
    pub oem_data_offset: u16,
    /// Port type
    pub port_type: u16,
    /// Port subtype
    pub port_subtype: u16,
    /// Reserved
    pub reserved: [u8; 2],
    /// Base address register offset
    pub base_address_offset: u16,
    /// Address size offset
    pub address_size_offset: u16,
}

/// Debug port types
pub mod debug_port_type {
    /// Serial port (16550-compatible)
    pub const SERIAL_16550: u16 = 0x8000;
    /// Serial port (16550 with GAS)
    pub const SERIAL_16550_GAS: u16 = 0x8001;
    /// ARM PL011
    pub const ARM_PL011: u16 = 0x8003;
    /// ARM SBSA 32-bit
    pub const ARM_SBSA_32: u16 = 0x800D;
    /// ARM SBSA
    pub const ARM_SBSA: u16 = 0x800E;
}

// =============================================================================
// SERIAL PORT CONFIGURATION
// =============================================================================

/// Serial port configuration
#[derive(Debug, Clone, Copy)]
pub struct SerialPortConfig {
    /// Baud rate
    pub baud_rate: u64,
    /// Data bits
    pub data_bits: u8,
    /// Parity
    pub parity: EfiParity,
    /// Stop bits
    pub stop_bits: EfiStopBits,
    /// Flow control
    pub flow_control: bool,
}

impl Default for SerialPortConfig {
    fn default() -> Self {
        Self {
            baud_rate: baud_rates::B115200,
            data_bits: 8,
            parity: EfiParity::None,
            stop_bits: EfiStopBits::One,
            flow_control: false,
        }
    }
}

impl SerialPortConfig {
    /// Create with custom baud rate
    pub const fn with_baud_rate(baud_rate: u64) -> Self {
        Self {
            baud_rate,
            data_bits: 8,
            parity: EfiParity::None,
            stop_bits: EfiStopBits::One,
            flow_control: false,
        }
    }

    /// Standard 9600 8N1
    pub const fn standard_9600() -> Self {
        Self::with_baud_rate(baud_rates::B9600)
    }

    /// Standard 115200 8N1
    pub const fn standard_115200() -> Self {
        Self::with_baud_rate(baud_rates::B115200)
    }
}

// =============================================================================
// SERIAL WRITE HELPER
// =============================================================================

/// Helper for writing formatted output to serial port
pub struct SerialWriter<'a> {
    protocol: &'a mut EfiSerialIoProtocol,
}

impl<'a> SerialWriter<'a> {
    /// Create a new serial writer
    pub fn new(protocol: &'a mut EfiSerialIoProtocol) -> Self {
        Self { protocol }
    }

    /// Write bytes
    pub fn write(&mut self, data: &[u8]) -> Result<usize, Status> {
        unsafe { self.protocol.write_bytes(data) }
    }

    /// Write string
    pub fn write_str(&mut self, s: &str) -> Result<usize, Status> {
        unsafe { self.protocol.write_str(s) }
    }
}

impl<'a> fmt::Write for SerialWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s).map_err(|_| fmt::Error)?;
        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_control() {
        let control = SerialControl::DTR | SerialControl::RTS;
        assert!(control.contains(SerialControl::DTR));
        assert!(control.contains(SerialControl::RTS));
        assert!(!control.contains(SerialControl::CTS));
    }

    #[test]
    fn test_serial_config() {
        let config = SerialPortConfig::standard_115200();
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.data_bits, 8);
    }

    #[test]
    fn test_parity() {
        assert_eq!(EfiParity::default(), EfiParity::Default);
    }
}
