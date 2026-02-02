//! Serial I/O Protocol
//!
//! High-level serial port abstraction for debug output and communication.
//!
//! # Safety
//! This module interfaces with UEFI firmware via FFI. All protocol pointers
//! are provided by the firmware and validated before use.
// codeql[rust/access-invalid-pointer] - UEFI FFI pointers validated by firmware

use super::{EnumerableProtocol, Protocol};
use crate::error::{Error, Result};
use crate::raw::protocols::serial::*;
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

/// Serial I/O Protocol GUID
const SERIAL_IO_PROTOCOL_GUID: Guid = guids::SERIAL_IO_PROTOCOL;

// =============================================================================
// SERIAL PORT
// =============================================================================

/// High-level serial port abstraction
pub struct SerialPort {
    /// Raw protocol pointer
    protocol: *mut EfiSerialIoProtocol,
    /// Handle
    handle: Handle,
    /// Configuration
    config: SerialConfig,
}

impl SerialPort {
    /// Create from raw protocol
    ///
    /// # Safety
    /// Protocol pointer must be valid and obtained from UEFI LocateProtocol
    /// or HandleProtocol calls. The pointer must remain valid for the
    /// lifetime of this SerialPort instance.
    ///
    /// # Panics
    /// Panics in debug mode if protocol pointer is null.
    pub unsafe fn from_raw(protocol: *mut EfiSerialIoProtocol, handle: Handle) -> Self {
        debug_assert!(!protocol.is_null(), "SerialPort protocol pointer is null");
        debug_assert!(
            !(*protocol).mode.is_null(),
            "SerialPort mode pointer is null"
        );

        let mode = &*(*protocol).mode;
        let config = SerialConfig {
            baud_rate: mode.baud_rate as u32,
            data_bits: mode.data_bits as u8,
            parity: Parity::from_raw(mode.parity()),
            stop_bits: StopBits::from_raw(mode.stop_bits()),
            timeout: mode.timeout as u32,
            fifo_depth: mode.receive_fifo_depth as u32,
        };

        Self {
            protocol,
            handle,
            config,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &SerialConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: &SerialConfig) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).set_attributes)(
                self.protocol,
                config.baud_rate as u64,
                0, // Receive FIFO depth (0 = use default)
                config.timeout,
                config.parity.to_raw(),
                config.data_bits,
                config.stop_bits.to_raw(),
            )
        };

        if result == Status::SUCCESS {
            self.config = config.clone();
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Set baud rate only
    pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<()> {
        let mut config = self.config.clone();
        config.baud_rate = baud_rate;
        self.set_config(&config)
    }

    /// Reset port
    pub fn reset(&mut self) -> Result<()> {
        let result = unsafe { ((*self.protocol).reset)(self.protocol) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    // =========================================================================
    // I/O OPERATIONS
    // =========================================================================

    /// Write data
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        let mut size = data.len();

        let result = unsafe {
            ((*self.protocol).write)(
                self.protocol,
                &mut size,
                data.as_ptr() as *const core::ffi::c_void,
            )
        };

        if result == Status::SUCCESS {
            Ok(size)
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Write all data
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let written = self.write(&data[offset..])?;
            if written == 0 {
                return Err(Error::DeviceError);
            }
            offset += written;
        }
        Ok(())
    }

    /// Write string
    pub fn write_str(&mut self, s: &str) -> Result<()> {
        self.write_all(s.as_bytes())
    }

    /// Write line
    pub fn writeln(&mut self, s: &str) -> Result<()> {
        self.write_str(s)?;
        self.write_all(b"\r\n")
    }

    /// Read data
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }

        let mut size = buffer.len();

        let result = unsafe {
            ((*self.protocol).read)(
                self.protocol,
                &mut size,
                buffer.as_mut_ptr() as *mut core::ffi::c_void,
            )
        };

        match result {
            Status::SUCCESS => Ok(size),
            Status::TIMEOUT => Ok(0),
            _ => Err(Error::from_status(result)),
        }
    }

    /// Read single byte
    pub fn read_byte(&mut self) -> Result<Option<u8>> {
        let mut byte = [0u8; 1];
        let read = self.read(&mut byte)?;
        if read > 0 {
            Ok(Some(byte[0]))
        } else {
            Ok(None)
        }
    }

    /// Read until newline
    pub fn read_line(&mut self, max_len: usize) -> Result<String> {
        let mut buffer = Vec::with_capacity(max_len);

        loop {
            if buffer.len() >= max_len {
                break;
            }

            if let Some(byte) = self.read_byte()? {
                if byte == b'\n' || byte == b'\r' {
                    break;
                }
                buffer.push(byte);
            }
        }

        String::from_utf8(buffer).map_err(|_| Error::InvalidParameter)
    }

    /// Check if data is available
    pub fn available(&self) -> bool {
        // Check control bits
        let mut control = 0u32;
        let result = unsafe { ((*self.protocol).get_control)(self.protocol, &mut control) };

        if result == Status::SUCCESS {
            // Check if input buffer is not empty
            (control & EFI_SERIAL_INPUT_BUFFER_EMPTY) == 0
        } else {
            false
        }
    }

    /// Flush output buffer
    pub fn flush(&mut self) -> Result<()> {
        // Wait for output to complete
        loop {
            let mut control = 0u32;
            let result = unsafe { ((*self.protocol).get_control)(self.protocol, &mut control) };

            if result != Status::SUCCESS {
                return Err(Error::from_status(result));
            }

            // Check if output buffer is empty
            if (control & EFI_SERIAL_OUTPUT_BUFFER_EMPTY) != 0 {
                break;
            }
        }

        Ok(())
    }

    // =========================================================================
    // CONTROL
    // =========================================================================

    /// Set control bits
    pub fn set_control(&mut self, control: SerialControl) -> Result<()> {
        let result = unsafe { ((*self.protocol).set_control)(self.protocol, control.0) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Get control bits
    pub fn get_control(&self) -> Result<SerialControl> {
        let mut control = 0u32;
        let result = unsafe { ((*self.protocol).get_control)(self.protocol, &mut control) };

        if result == Status::SUCCESS {
            Ok(SerialControl(control))
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Set DTR (Data Terminal Ready)
    pub fn set_dtr(&mut self, value: bool) -> Result<()> {
        let mut control = self.get_control()?;
        if value {
            control.0 |= EFI_SERIAL_DATA_TERMINAL_READY;
        } else {
            control.0 &= !EFI_SERIAL_DATA_TERMINAL_READY;
        }
        self.set_control(control)
    }

    /// Set RTS (Request To Send)
    pub fn set_rts(&mut self, value: bool) -> Result<()> {
        let mut control = self.get_control()?;
        if value {
            control.0 |= EFI_SERIAL_REQUEST_TO_SEND;
        } else {
            control.0 &= !EFI_SERIAL_REQUEST_TO_SEND;
        }
        self.set_control(control)
    }

    /// Enable hardware loopback
    pub fn set_loopback(&mut self, enable: bool) -> Result<()> {
        let mut control = self.get_control()?;
        if enable {
            control.0 |= EFI_SERIAL_HARDWARE_LOOPBACK_ENABLE;
        } else {
            control.0 &= !EFI_SERIAL_HARDWARE_LOOPBACK_ENABLE;
        }
        self.set_control(control)
    }

    /// Enable software loopback
    pub fn set_software_loopback(&mut self, enable: bool) -> Result<()> {
        let mut control = self.get_control()?;
        if enable {
            control.0 |= EFI_SERIAL_SOFTWARE_LOOPBACK_ENABLE;
        } else {
            control.0 &= !EFI_SERIAL_SOFTWARE_LOOPBACK_ENABLE;
        }
        self.set_control(control)
    }

    /// Check CTS (Clear To Send)
    pub fn cts(&self) -> Result<bool> {
        let control = self.get_control()?;
        Ok((control.0 & EFI_SERIAL_CLEAR_TO_SEND) != 0)
    }

    /// Check DSR (Data Set Ready)
    pub fn dsr(&self) -> Result<bool> {
        let control = self.get_control()?;
        Ok((control.0 & EFI_SERIAL_DATA_SET_READY) != 0)
    }

    /// Check carrier detect
    pub fn carrier_detect(&self) -> Result<bool> {
        let control = self.get_control()?;
        Ok((control.0 & EFI_SERIAL_CARRIER_DETECT) != 0)
    }

    /// Check ring indicator
    pub fn ring(&self) -> Result<bool> {
        let control = self.get_control()?;
        Ok((control.0 & EFI_SERIAL_RING_INDICATE) != 0)
    }
}

impl Protocol for SerialPort {
    const GUID: Guid = SERIAL_IO_PROTOCOL_GUID;

    fn open(handle: Handle) -> Result<Self> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };
        let image = crate::services::image_handle().ok_or(Error::NotReady)?;

        let mut protocol: *mut core::ffi::c_void = core::ptr::null_mut();
        let result = unsafe {
            ((*bs).open_protocol)(
                handle,
                &Self::GUID as *const Guid,
                &mut protocol,
                image,
                Handle(core::ptr::null_mut()),
                0x00000002,
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        Ok(unsafe { Self::from_raw(protocol as *mut EfiSerialIoProtocol, handle) })
    }
}

impl EnumerableProtocol for SerialPort {
    fn enumerate() -> Result<Vec<Self>> {
        super::ProtocolLocator::locate_all::<Self>()
            .map(|handles| handles.into_iter().map(|h| h.leak()).collect())
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_all(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

// =============================================================================
// SERIAL CONFIG
// =============================================================================

/// Serial port configuration
#[derive(Debug, Clone)]
pub struct SerialConfig {
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits (5, 6, 7, or 8)
    pub data_bits: u8,
    /// Parity
    pub parity: Parity,
    /// Stop bits
    pub stop_bits: StopBits,
    /// Timeout in microseconds (0 = no timeout)
    pub timeout: u32,
    /// FIFO depth
    pub fifo_depth: u32,
}

impl SerialConfig {
    /// Create default configuration (115200 8N1)
    pub fn default_115200() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: 0,
            fifo_depth: 16,
        }
    }

    /// Create configuration for 9600 baud
    pub fn baud_9600() -> Self {
        Self {
            baud_rate: 9600,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: 0,
            fifo_depth: 16,
        }
    }

    /// Create configuration for 38400 baud
    pub fn baud_38400() -> Self {
        Self {
            baud_rate: 38400,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: 0,
            fifo_depth: 16,
        }
    }

    /// Create configuration for 57600 baud
    pub fn baud_57600() -> Self {
        Self {
            baud_rate: 57600,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: 0,
            fifo_depth: 16,
        }
    }

    /// With baud rate
    pub fn with_baud_rate(mut self, baud_rate: u32) -> Self {
        self.baud_rate = baud_rate;
        self
    }

    /// With data bits
    pub fn with_data_bits(mut self, data_bits: u8) -> Self {
        self.data_bits = data_bits;
        self
    }

    /// With parity
    pub fn with_parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    /// With stop bits
    pub fn with_stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    /// With timeout
    pub fn with_timeout(mut self, timeout_us: u32) -> Self {
        self.timeout = timeout_us;
        self
    }
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self::default_115200()
    }
}

// =============================================================================
// PARITY
// =============================================================================

/// Parity setting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    /// No parity
    None,
    /// Even parity
    Even,
    /// Odd parity
    Odd,
    /// Mark parity
    Mark,
    /// Space parity
    Space,
}

impl Parity {
    fn from_raw(raw: EfiParity) -> Self {
        match raw {
            EfiParity::Default | EfiParity::None => Self::None,
            EfiParity::Even => Self::Even,
            EfiParity::Odd => Self::Odd,
            EfiParity::Mark => Self::Mark,
            EfiParity::Space => Self::Space,
        }
    }

    fn to_raw(&self) -> EfiParity {
        match self {
            Self::None => EfiParity::None,
            Self::Even => EfiParity::Even,
            Self::Odd => EfiParity::Odd,
            Self::Mark => EfiParity::Mark,
            Self::Space => EfiParity::Space,
        }
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Even => "Even",
            Self::Odd => "Odd",
            Self::Mark => "Mark",
            Self::Space => "Space",
        }
    }

    /// Get abbreviation (N, E, O, M, S)
    pub fn abbrev(&self) -> char {
        match self {
            Self::None => 'N',
            Self::Even => 'E',
            Self::Odd => 'O',
            Self::Mark => 'M',
            Self::Space => 'S',
        }
    }
}

// =============================================================================
// STOP BITS
// =============================================================================

/// Stop bits setting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    /// 1 stop bit
    One,
    /// 1.5 stop bits
    OnePointFive,
    /// 2 stop bits
    Two,
}

impl StopBits {
    fn from_raw(raw: EfiStopBits) -> Self {
        match raw {
            EfiStopBits::Default | EfiStopBits::One => Self::One,
            EfiStopBits::OneFive => Self::OnePointFive,
            EfiStopBits::Two => Self::Two,
        }
    }

    fn to_raw(&self) -> EfiStopBits {
        match self {
            Self::One => EfiStopBits::One,
            Self::OnePointFive => EfiStopBits::OneFive,
            Self::Two => EfiStopBits::Two,
        }
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::One => "1",
            Self::OnePointFive => "1.5",
            Self::Two => "2",
        }
    }
}

// =============================================================================
// SERIAL CONTROL
// =============================================================================

/// Serial control bits
#[derive(Debug, Clone, Copy)]
pub struct SerialControl(pub u32);

impl SerialControl {
    /// DTR
    pub fn dtr(&self) -> bool {
        (self.0 & EFI_SERIAL_DATA_TERMINAL_READY) != 0
    }

    /// RTS
    pub fn rts(&self) -> bool {
        (self.0 & EFI_SERIAL_REQUEST_TO_SEND) != 0
    }

    /// CTS
    pub fn cts(&self) -> bool {
        (self.0 & EFI_SERIAL_CLEAR_TO_SEND) != 0
    }

    /// DSR
    pub fn dsr(&self) -> bool {
        (self.0 & EFI_SERIAL_DATA_SET_READY) != 0
    }

    /// Ring indicator
    pub fn ring(&self) -> bool {
        (self.0 & EFI_SERIAL_RING_INDICATE) != 0
    }

    /// Carrier detect
    pub fn carrier_detect(&self) -> bool {
        (self.0 & EFI_SERIAL_CARRIER_DETECT) != 0
    }

    /// Input buffer empty
    pub fn input_empty(&self) -> bool {
        (self.0 & EFI_SERIAL_INPUT_BUFFER_EMPTY) != 0
    }

    /// Output buffer empty
    pub fn output_empty(&self) -> bool {
        (self.0 & EFI_SERIAL_OUTPUT_BUFFER_EMPTY) != 0
    }

    /// Hardware loopback enabled
    pub fn hardware_loopback(&self) -> bool {
        (self.0 & EFI_SERIAL_HARDWARE_LOOPBACK_ENABLE) != 0
    }

    /// Software loopback enabled
    pub fn software_loopback(&self) -> bool {
        (self.0 & EFI_SERIAL_SOFTWARE_LOOPBACK_ENABLE) != 0
    }
}

// Control bit constants
const EFI_SERIAL_CLEAR_TO_SEND: u32 = 0x0010;
const EFI_SERIAL_DATA_SET_READY: u32 = 0x0020;
const EFI_SERIAL_RING_INDICATE: u32 = 0x0040;
const EFI_SERIAL_CARRIER_DETECT: u32 = 0x0080;
const EFI_SERIAL_REQUEST_TO_SEND: u32 = 0x0002;
const EFI_SERIAL_DATA_TERMINAL_READY: u32 = 0x0001;
const EFI_SERIAL_INPUT_BUFFER_EMPTY: u32 = 0x0100;
const EFI_SERIAL_OUTPUT_BUFFER_EMPTY: u32 = 0x0200;
const EFI_SERIAL_HARDWARE_LOOPBACK_ENABLE: u32 = 0x1000;
const EFI_SERIAL_SOFTWARE_LOOPBACK_ENABLE: u32 = 0x2000;
const EFI_SERIAL_HARDWARE_FLOW_CONTROL_ENABLE: u32 = 0x4000;

// =============================================================================
// DEBUG OUTPUT
// =============================================================================

/// Global debug serial port
static mut DEBUG_PORT: Option<SerialPort> = None;

/// Initialize debug output
///
/// # Safety
/// Must be called before any debug output and only once
pub unsafe fn init_debug_port() -> Result<()> {
    let ports = SerialPort::enumerate()?;
    if let Some(port) = ports.into_iter().next() {
        DEBUG_PORT = Some(port);
        Ok(())
    } else {
        Err(Error::NotFound)
    }
}

/// Write to debug port
pub fn debug_write(s: &str) {
    unsafe {
        if let Some(ref mut port) = DEBUG_PORT {
            let _ = port.write_str(s);
        }
    }
}

/// Write line to debug port
pub fn debug_writeln(s: &str) {
    unsafe {
        if let Some(ref mut port) = DEBUG_PORT {
            let _ = port.writeln(s);
        }
    }
}

/// Debug print macro
#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut s = alloc::string::String::new();
        let _ = write!(&mut s, $($arg)*);
        $crate::protocols::serial::debug_write(&s);
    }};
}

/// Debug println macro
#[macro_export]
macro_rules! debug_println {
    () => ($crate::protocols::serial::debug_writeln(""));
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        let mut s = alloc::string::String::new();
        let _ = write!(&mut s, $($arg)*);
        $crate::protocols::serial::debug_writeln(&s);
    }};
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity() {
        assert_eq!(Parity::None.abbrev(), 'N');
        assert_eq!(Parity::Even.abbrev(), 'E');
        assert_eq!(Parity::Odd.abbrev(), 'O');
    }

    #[test]
    fn test_stop_bits() {
        assert_eq!(StopBits::One.name(), "1");
        assert_eq!(StopBits::Two.name(), "2");
    }

    #[test]
    fn test_serial_config() {
        let config = SerialConfig::default_115200();
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.data_bits, 8);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.stop_bits, StopBits::One);
    }

    #[test]
    fn test_config_builder() {
        let config = SerialConfig::default_115200()
            .with_baud_rate(9600)
            .with_parity(Parity::Even);

        assert_eq!(config.baud_rate, 9600);
        assert_eq!(config.parity, Parity::Even);
    }
}
