//! # RPC Messages
//!
//! GSP RPC message encoding and decoding.

use magma_core::{Error, Result};

// =============================================================================
// MESSAGE TYPES
// =============================================================================

/// RPC message function codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum RpcFunction {
    // =================================
    // System Functions (0x0000-0x00FF)
    // =================================
    /// Initialize GSP
    SystemInit          = 0x0001,
    /// Shutdown GSP
    SystemShutdown      = 0x0002,
    /// Get GSP version
    SystemGetVersion    = 0x0003,
    /// Get capabilities
    SystemGetCaps       = 0x0004,
    /// Heartbeat/ping
    SystemHeartbeat     = 0x0005,

    // =================================
    // Memory Functions (0x0100-0x01FF)
    // =================================
    /// Allocate memory
    MemAlloc            = 0x0100,
    /// Free memory
    MemFree             = 0x0101,
    /// Map memory
    MemMap              = 0x0102,
    /// Unmap memory
    MemUnmap            = 0x0103,
    /// Get memory info
    MemGetInfo          = 0x0104,
    /// Flush caches
    MemFlush            = 0x0105,

    // =================================
    // Engine Functions (0x0200-0x02FF)
    // =================================
    /// Allocate channel
    EngineAllocChannel  = 0x0200,
    /// Free channel
    EngineFreeChannel   = 0x0201,
    /// Bind context
    EngineBindContext   = 0x0202,
    /// Unbind context
    EngineUnbindContext = 0x0203,
    /// Schedule work
    EngineSchedule      = 0x0204,
    /// Get engine status
    EngineGetStatus     = 0x0205,

    // =================================
    // Graphics Functions (0x0300-0x03FF)
    // =================================
    /// Create graphics context
    GrCreateContext     = 0x0300,
    /// Destroy graphics context
    GrDestroyContext    = 0x0301,
    /// Set graphics object
    GrSetObject         = 0x0302,
    /// Preempt graphics
    GrPreempt           = 0x0303,

    // =================================
    // Compute Functions (0x0400-0x04FF)
    // =================================
    /// Create compute context
    CeCreateContext     = 0x0400,
    /// Destroy compute context
    CeDestroyContext    = 0x0401,
    /// Configure compute
    CeConfigure         = 0x0402,

    // =================================
    // Display Functions (0x0500-0x05FF)
    // =================================
    /// Setup display
    DispSetup           = 0x0500,
    /// Mode set
    DispModeSet         = 0x0501,
    /// Flip
    DispFlip            = 0x0502,
    /// Cursor update
    DispCursor          = 0x0503,

    // =================================
    // VBIOS Functions (0x0600-0x06FF)
    // =================================
    /// Get VBIOS info
    VbiosGetInfo        = 0x0600,
    /// Call VBIOS
    VbiosCall           = 0x0601,

    // =================================
    // Fault Functions (0x0700-0x07FF)
    // =================================
    /// Get fault info
    FaultGetInfo        = 0x0700,
    /// Clear fault
    FaultClear          = 0x0701,

    // =================================
    // Telemetry (0x0800-0x08FF)
    // =================================
    /// Get temperature
    TelemetryGetTemp    = 0x0800,
    /// Get power
    TelemetryGetPower   = 0x0801,
    /// Get clocks
    TelemetryGetClocks  = 0x0802,

    /// Unknown function
    Unknown             = 0xFFFF,
}

impl From<u32> for RpcFunction {
    fn from(value: u32) -> Self {
        match value {
            0x0001 => Self::SystemInit,
            0x0002 => Self::SystemShutdown,
            0x0003 => Self::SystemGetVersion,
            // Add more mappings as needed
            _ => Self::Unknown,
        }
    }
}

// =============================================================================
// RPC HEADER
// =============================================================================

/// RPC message header (32 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RpcHeader {
    /// Message length (including header)
    pub length: u32,
    /// Function code
    pub function: u32,
    /// Sequence number for request/response matching
    pub sequence: u32,
    /// Flags
    pub flags: u32,
    /// Return code (0 = success, filled in response)
    pub status: u32,
    /// Reserved for alignment
    pub reserved: [u32; 3],
}

impl RpcHeader {
    /// Header size in bytes
    pub const SIZE: usize = 32;

    /// Create new request header
    pub fn new_request(function: RpcFunction, sequence: u32, payload_len: u32) -> Self {
        Self {
            length: Self::SIZE as u32 + payload_len,
            function: function as u32,
            sequence,
            flags: RpcFlags::REQUEST.bits(),
            status: 0,
            reserved: [0; 3],
        }
    }

    /// Check if this is a request
    pub fn is_request(&self) -> bool {
        (self.flags & RpcFlags::REQUEST.bits()) != 0
    }

    /// Check if this is a response
    pub fn is_response(&self) -> bool {
        (self.flags & RpcFlags::RESPONSE.bits()) != 0
    }

    /// Get function code
    pub fn function(&self) -> RpcFunction {
        RpcFunction::from(self.function)
    }

    /// Get payload length
    pub fn payload_len(&self) -> usize {
        self.length.saturating_sub(Self::SIZE as u32) as usize
    }
}

/// RPC message flags
bitflags::bitflags! {
    /// RPC header flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RpcFlags: u32 {
        /// Message is a request
        const REQUEST = 1 << 0;
        /// Message is a response
        const RESPONSE = 1 << 1;
        /// Async (no response expected)
        const ASYNC = 1 << 2;
        /// High priority
        const HIGH_PRIORITY = 1 << 3;
        /// Error in response
        const ERROR = 1 << 4;
    }
}

// =============================================================================
// RPC MESSAGE
// =============================================================================

/// Maximum RPC message size (64KB)
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// Maximum payload size
pub const MAX_PAYLOAD_SIZE: usize = MAX_MESSAGE_SIZE - RpcHeader::SIZE;

/// RPC message with payload
#[derive(Debug)]
pub struct RpcMessage {
    /// Message header
    pub header: RpcHeader,
    /// Payload data
    pub payload: alloc::vec::Vec<u8>,
}

impl RpcMessage {
    /// Create new request message
    pub fn new_request(function: RpcFunction, sequence: u32, payload: alloc::vec::Vec<u8>) -> Self {
        let header = RpcHeader::new_request(function, sequence, payload.len() as u32);
        Self { header, payload }
    }

    /// Create empty request
    pub fn empty_request(function: RpcFunction, sequence: u32) -> Self {
        Self::new_request(function, sequence, alloc::vec::Vec::new())
    }

    /// Get total message size
    pub fn size(&self) -> usize {
        RpcHeader::SIZE + self.payload.len()
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec::Vec::with_capacity(self.size());

        // Serialize header
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &self.header as *const RpcHeader as *const u8,
                RpcHeader::SIZE,
            )
        };
        bytes.extend_from_slice(header_bytes);

        // Append payload
        bytes.extend_from_slice(&self.payload);

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < RpcHeader::SIZE {
            return Err(Error::InvalidParameter);
        }

        // Parse header
        let header = unsafe { *(bytes.as_ptr() as *const RpcHeader) };

        // Validate length
        if header.length as usize > bytes.len() {
            return Err(Error::InvalidParameter);
        }

        // Extract payload
        let payload_len = header.payload_len();
        let payload = bytes[RpcHeader::SIZE..RpcHeader::SIZE + payload_len].to_vec();

        Ok(Self { header, payload })
    }
}

// =============================================================================
// RPC RESULT
// =============================================================================

/// RPC call result
#[derive(Debug)]
pub enum RpcResult {
    /// Success with optional response data
    Success(Option<alloc::vec::Vec<u8>>),
    /// Error with status code
    Error(RpcStatus),
    /// Timeout waiting for response
    Timeout,
    /// Channel closed
    ChannelClosed,
}

impl RpcResult {
    /// Check if successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Get response data if successful
    pub fn data(self) -> Option<alloc::vec::Vec<u8>> {
        match self {
            Self::Success(data) => data,
            _ => None,
        }
    }
}

/// RPC status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RpcStatus {
    /// Success
    Ok             = 0,
    /// Generic error
    Error          = 1,
    /// Invalid parameter
    InvalidParam   = 2,
    /// Out of memory
    OutOfMemory    = 3,
    /// Not supported
    NotSupported   = 4,
    /// Object not found
    NotFound       = 5,
    /// Already exists
    AlreadyExists  = 6,
    /// Busy/in use
    Busy           = 7,
    /// Timeout
    Timeout        = 8,
    /// Not initialized
    NotInitialized = 9,
    /// Invalid state
    InvalidState   = 10,
    /// Hardware error
    HwError        = 11,
    /// GSP error
    GspError       = 12,
}

impl From<u32> for RpcStatus {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Ok,
            1 => Self::Error,
            2 => Self::InvalidParam,
            3 => Self::OutOfMemory,
            4 => Self::NotSupported,
            5 => Self::NotFound,
            6 => Self::AlreadyExists,
            7 => Self::Busy,
            8 => Self::Timeout,
            9 => Self::NotInitialized,
            10 => Self::InvalidState,
            11 => Self::HwError,
            12 => Self::GspError,
            _ => Self::Error,
        }
    }
}
