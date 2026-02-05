//! UEFI Network Boot Support
//!
//! PXE, HTTP, and TFTP boot protocols for network booting.

use core::fmt;

// =============================================================================
// NETWORK TYPES
// =============================================================================

/// MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Zero MAC address
    pub const ZERO: Self = Self([0; 6]);

    /// Broadcast MAC address
    pub const BROADCAST: Self = Self([0xFF; 6]);

    /// Create from bytes
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Check if broadcast
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }

    /// Check if multicast
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    /// Check if unicast
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }

    /// Check if locally administered
    pub fn is_local(&self) -> bool {
        self.0[0] & 0x02 != 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// Zero address (0.0.0.0)
    pub const ZERO: Self = Self([0; 4]);

    /// Loopback address (127.0.0.1)
    pub const LOOPBACK: Self = Self([127, 0, 0, 1]);

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Self = Self([255; 4]);

    /// Create from parts
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// To u32 (big endian)
    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    /// From u32 (big endian)
    pub fn from_u32(v: u32) -> Self {
        Self(v.to_be_bytes())
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.0 == [0; 4]
    }

    /// Check if loopback
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    /// Check if broadcast
    pub fn is_broadcast(&self) -> bool {
        self.0 == [255; 4]
    }

    /// Check if link-local (169.254.x.x)
    pub fn is_link_local(&self) -> bool {
        self.0[0] == 169 && self.0[1] == 254
    }

    /// Check if private
    pub fn is_private(&self) -> bool {
        // 10.0.0.0/8
        self.0[0] == 10 ||
        // 172.16.0.0/12
        (self.0[0] == 172 && (self.0[1] & 0xF0) == 16) ||
        // 192.168.0.0/16
        (self.0[0] == 192 && self.0[1] == 168)
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Address(pub [u8; 16]);

impl Ipv6Address {
    /// Zero address (::)
    pub const ZERO: Self = Self([0; 16]);

    /// Loopback address (::1)
    pub const LOOPBACK: Self = Self([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

    /// Create from parts (8 x u16)
    pub const fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        let bytes = [
            (a >> 8) as u8,
            a as u8,
            (b >> 8) as u8,
            b as u8,
            (c >> 8) as u8,
            c as u8,
            (d >> 8) as u8,
            d as u8,
            (e >> 8) as u8,
            e as u8,
            (f >> 8) as u8,
            f as u8,
            (g >> 8) as u8,
            g as u8,
            (h >> 8) as u8,
            h as u8,
        ];
        Self(bytes)
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.0 == [0; 16]
    }

    /// Check if loopback
    pub fn is_loopback(&self) -> bool {
        *self == Self::LOOPBACK
    }

    /// Check if link-local (fe80::/10)
    pub fn is_link_local(&self) -> bool {
        self.0[0] == 0xFE && (self.0[1] & 0xC0) == 0x80
    }

    /// Check if multicast (ff00::/8)
    pub fn is_multicast(&self) -> bool {
        self.0[0] == 0xFF
    }

    /// Create link-local from MAC
    pub fn from_mac_link_local(mac: &MacAddress) -> Self {
        let mut addr = [0u8; 16];
        addr[0] = 0xFE;
        addr[1] = 0x80;
        // Modified EUI-64
        addr[8] = mac.0[0] ^ 0x02;
        addr[9] = mac.0[1];
        addr[10] = mac.0[2];
        addr[11] = 0xFF;
        addr[12] = 0xFE;
        addr[13] = mac.0[3];
        addr[14] = mac.0[4];
        addr[15] = mac.0[5];
        Self(addr)
    }
}

/// IP address (v4 or v6)
#[derive(Debug, Clone, Copy)]
pub enum IpAddress {
    V4(Ipv4Address),
    V6(Ipv6Address),
}

impl IpAddress {
    /// Check if zero
    pub fn is_zero(&self) -> bool {
        match self {
            Self::V4(a) => a.is_zero(),
            Self::V6(a) => a.is_zero(),
        }
    }
}

// =============================================================================
// DHCP
// =============================================================================

/// DHCP message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DhcpMessageType {
    Discover = 1,
    Offer    = 2,
    Request  = 3,
    Decline  = 4,
    Ack      = 5,
    Nak      = 6,
    Release  = 7,
    Inform   = 8,
}

/// DHCP options
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum DhcpOption {
    Pad                  = 0,
    SubnetMask           = 1,
    Router               = 3,
    DnsServer            = 6,
    HostName             = 12,
    DomainName           = 15,
    BroadcastAddr        = 28,
    RequestedIp          = 50,
    LeaseTime            = 51,
    MessageType          = 53,
    ServerIdentifier     = 54,
    ParameterRequestList = 55,
    MaxMessageSize       = 57,
    VendorClassId        = 60,
    ClientId             = 61,
    TftpServerName       = 66,
    BootFileName         = 67,
    ClasslessStaticRoute = 121,
    End                  = 255,
}

/// DHCP packet header
#[repr(C, packed)]
pub struct DhcpPacket {
    /// Message op code
    pub op: u8,
    /// Hardware address type (1 = Ethernet)
    pub htype: u8,
    /// Hardware address length
    pub hlen: u8,
    /// Hops
    pub hops: u8,
    /// Transaction ID
    pub xid: u32,
    /// Seconds elapsed
    pub secs: u16,
    /// Flags
    pub flags: u16,
    /// Client IP address
    pub ciaddr: [u8; 4],
    /// Your (client) IP address
    pub yiaddr: [u8; 4],
    /// Server IP address
    pub siaddr: [u8; 4],
    /// Gateway IP address
    pub giaddr: [u8; 4],
    /// Client hardware address
    pub chaddr: [u8; 16],
    /// Server host name
    pub sname: [u8; 64],
    /// Boot file name
    pub file: [u8; 128],
    /// DHCP magic cookie
    pub magic: [u8; 4],
    // Options follow...
}

impl DhcpPacket {
    /// DHCP magic cookie
    pub const MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

    /// Boot request
    pub const BOOT_REQUEST: u8 = 1;

    /// Boot reply
    pub const BOOT_REPLY: u8 = 2;

    /// Validate packet
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC_COOKIE
    }
}

/// DHCP lease information
#[derive(Debug, Clone)]
pub struct DhcpLease {
    /// Assigned IP address
    pub ip: Ipv4Address,
    /// Subnet mask
    pub subnet_mask: Ipv4Address,
    /// Default gateway
    pub gateway: Ipv4Address,
    /// DNS servers
    pub dns_servers: [Ipv4Address; 4],
    /// Number of DNS servers
    pub dns_count: usize,
    /// Lease time in seconds
    pub lease_time: u32,
    /// DHCP server
    pub server: Ipv4Address,
    /// Boot server name
    pub boot_server: [u8; 64],
    /// Boot file name
    pub boot_file: [u8; 128],
}

impl Default for DhcpLease {
    fn default() -> Self {
        Self {
            ip: Ipv4Address::ZERO,
            subnet_mask: Ipv4Address::ZERO,
            gateway: Ipv4Address::ZERO,
            dns_servers: [Ipv4Address::ZERO; 4],
            dns_count: 0,
            lease_time: 0,
            server: Ipv4Address::ZERO,
            boot_server: [0; 64],
            boot_file: [0; 128],
        }
    }
}

// =============================================================================
// PXE (Preboot eXecution Environment)
// =============================================================================

/// EFI PXE Base Code Protocol GUID
pub const EFI_PXE_BASE_CODE_PROTOCOL_GUID: [u8; 16] = [
    0xBC, 0x8A, 0x36, 0x03, 0xB0, 0x76, 0xD2, 0x11, 0x9E, 0x35, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81,
];

/// PXE packet type
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum PxePacketType {
    DhcpDiscover = 1,
    DhcpAck      = 2,
    ProxyOffer   = 3,
    PxeDiscover  = 4,
    PxeOffer     = 5,
    PxeAck       = 6,
}

/// PXE mode data
#[repr(C)]
pub struct PxeMode {
    /// Started
    pub started: bool,
    /// IPv6 available
    pub ipv6_available: bool,
    /// IPv6 supported
    pub ipv6_supported: bool,
    /// Using IPv6
    pub using_ipv6: bool,
    /// BIS supported
    pub bis_supported: bool,
    /// BIS detected
    pub bis_detected: bool,
    /// Auto ARP enabled
    pub auto_arp: bool,
    /// Send GUID
    pub send_guid: bool,
    /// DHCP discover valid
    pub dhcp_discover_valid: bool,
    /// DHCP ack received
    pub dhcp_ack_received: bool,
    /// Proxy offer received
    pub proxy_offer_received: bool,
    /// PXE discover valid
    pub pxe_discover_valid: bool,
    /// PXE reply received
    pub pxe_reply_received: bool,
    /// PXE bis reply received
    pub pxe_bis_reply_received: bool,
    /// ICMP error received
    pub icmp_error_received: bool,
    /// TFTP error received
    pub tftp_error_received: bool,
    /// Make callbacks
    pub make_callbacks: bool,
    /// TTL
    pub ttl: u8,
    /// ToS
    pub tos: u8,
    /// Station IP
    pub station_ip: [u8; 16],
    /// Subnet mask
    pub subnet_mask: [u8; 16],
}

/// PXE boot server type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum PxeServerType {
    /// Bootstrap server
    Bootstrap   = 0,
    /// Microsoft Windows NT
    WindowsNt   = 1,
    /// Intel LCM
    IntelLcm    = 2,
    /// DOS/UNDI
    DosUndi     = 3,
    /// NEC ESMPRO
    NecEsmpro   = 4,
    /// IBM WSoD
    IbmWsod     = 5,
    /// IBM LCCM
    IbmLccm     = 6,
    /// CA Unicenter
    CaUnicenter = 7,
    /// HP OpenView
    HpOpenview  = 8,
    /// Reserved
    Reserved    = 9,
}

/// PXE discover info
#[derive(Debug, Clone)]
pub struct PxeDiscoverInfo {
    /// Use M-Cast
    pub use_mcast: bool,
    /// Use B-Cast
    pub use_bcast: bool,
    /// Use U-Cast
    pub use_ucast: bool,
    /// Must use list
    pub must_use_list: bool,
    /// Server list
    pub servers: [Ipv4Address; 16],
    /// Server count
    pub server_count: usize,
}

/// PXE TFTP info
#[derive(Debug, Clone)]
pub struct PxeTftpInfo {
    /// Server IP
    pub server_ip: IpAddress,
    /// File name
    pub filename: [u8; 128],
    /// Filename length
    pub filename_len: usize,
    /// Block size
    pub block_size: u16,
    /// Window size
    pub window_size: u16,
}

/// PXE boot client
pub struct PxeClient {
    /// Protocol pointer (opaque)
    _protocol: usize,
    /// MAC address
    mac: MacAddress,
    /// Current IP
    ip: Ipv4Address,
    /// Mode data
    _mode: PxeMode,
    /// DHCP lease
    lease: DhcpLease,
    /// Started
    started: bool,
}

impl PxeClient {
    /// Create new PXE client
    pub fn new(protocol: usize, mac: MacAddress) -> Self {
        Self {
            _protocol: protocol,
            mac,
            ip: Ipv4Address::ZERO,
            _mode: unsafe { core::mem::zeroed() },
            lease: DhcpLease::default(),
            started: false,
        }
    }

    /// Start PXE client
    pub fn start(&mut self, _use_ipv6: bool) -> Result<(), NetworkError> {
        if self.started {
            return Ok(());
        }

        // Would call EFI_PXE_BASE_CODE.Start()
        self.started = true;
        Ok(())
    }

    /// Stop PXE client
    pub fn stop(&mut self) -> Result<(), NetworkError> {
        if !self.started {
            return Ok(());
        }

        // Would call EFI_PXE_BASE_CODE.Stop()
        self.started = false;
        Ok(())
    }

    /// Perform DHCP
    pub fn dhcp(&mut self, _sort_offers: bool) -> Result<&DhcpLease, NetworkError> {
        if !self.started {
            return Err(NetworkError::NotStarted);
        }

        // Would call EFI_PXE_BASE_CODE.Dhcp()
        // Parse DHCP response and populate lease

        Ok(&self.lease)
    }

    /// Discover boot server
    pub fn discover(
        &mut self,
        _server_type: PxeServerType,
        _layer: u16,
        _bis_verify: bool,
    ) -> Result<(), NetworkError> {
        if !self.started {
            return Err(NetworkError::NotStarted);
        }

        // Would call EFI_PXE_BASE_CODE.Discover()

        Ok(())
    }

    /// Download file via MTFTP
    pub fn mtftp(
        &mut self,
        _operation: MtftpOperation,
        _buffer: &mut [u8],
        _server: &Ipv4Address,
        _filename: &[u8],
    ) -> Result<usize, NetworkError> {
        if !self.started {
            return Err(NetworkError::NotStarted);
        }

        // Would call EFI_PXE_BASE_CODE.Mtftp()

        Ok(0)
    }

    /// Set station IP
    pub fn set_station_ip(
        &mut self,
        ip: Ipv4Address,
        _subnet: Ipv4Address,
    ) -> Result<(), NetworkError> {
        self.ip = ip;
        Ok(())
    }

    /// Get MAC address
    pub fn mac_address(&self) -> MacAddress {
        self.mac
    }

    /// Get current IP
    pub fn ip_address(&self) -> Ipv4Address {
        self.ip
    }

    /// Get lease info
    pub fn lease(&self) -> &DhcpLease {
        &self.lease
    }
}

/// MTFTP operation
#[derive(Debug, Clone, Copy)]
pub enum MtftpOperation {
    /// Read file
    ReadFile,
    /// Write file
    WriteFile,
    /// Read directory
    ReadDirectory,
    /// Get file size
    GetFileSize,
}

// =============================================================================
// TFTP
// =============================================================================

/// TFTP opcodes
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum TftpOpcode {
    /// Read request
    Rrq   = 1,
    /// Write request
    Wrq   = 2,
    /// Data
    Data  = 3,
    /// Acknowledgment
    Ack   = 4,
    /// Error
    Error = 5,
    /// Option acknowledgment
    Oack  = 6,
}

/// TFTP error codes
#[derive(Debug, Clone, Copy)]
#[repr(u16)]
pub enum TftpErrorCode {
    /// Not defined
    Undefined       = 0,
    /// File not found
    FileNotFound    = 1,
    /// Access violation
    AccessViolation = 2,
    /// Disk full
    DiskFull        = 3,
    /// Illegal operation
    IllegalOp       = 4,
    /// Unknown transfer ID
    UnknownTid      = 5,
    /// File already exists
    FileExists      = 6,
    /// No such user
    NoSuchUser      = 7,
    /// Option negotiation failed
    OptionsFailed   = 8,
}

/// TFTP transfer mode
#[derive(Debug, Clone, Copy)]
pub enum TftpMode {
    /// Binary/octet mode
    Octet,
    /// ASCII/netascii mode
    NetAscii,
}

impl TftpMode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Octet => "octet",
            Self::NetAscii => "netascii",
        }
    }
}

/// TFTP options
#[derive(Debug, Clone)]
pub struct TftpOptions {
    /// Block size (default 512)
    pub blksize: u16,
    /// Transfer size (for RRQ)
    pub tsize: Option<u64>,
    /// Timeout in seconds
    pub timeout: u8,
    /// Window size (RFC 7440)
    pub windowsize: u16,
}

impl Default for TftpOptions {
    fn default() -> Self {
        Self {
            blksize: 512,
            tsize: None,
            timeout: 5,
            windowsize: 1,
        }
    }
}

/// TFTP client state
#[derive(Debug, Clone, Copy)]
pub enum TftpState {
    /// Idle
    Idle,
    /// Waiting for option ack
    WaitOack,
    /// Transferring data
    Transferring,
    /// Complete
    Complete,
    /// Error
    Error,
}

/// TFTP client
pub struct TftpClient {
    /// Server IP
    _server: Ipv4Address,
    /// Server port (usually 69)
    _port: u16,
    /// Local port
    _local_port: u16,
    /// Options
    options: TftpOptions,
    /// State
    state: TftpState,
    /// Current block number
    block: u16,
    /// Transfer size
    transfer_size: u64,
    /// Bytes transferred
    bytes_transferred: u64,
}

impl TftpClient {
    /// Default TFTP port
    pub const DEFAULT_PORT: u16 = 69;

    /// Create new TFTP client
    pub fn new(server: Ipv4Address, port: u16) -> Self {
        Self {
            _server: server,
            _port: port,
            _local_port: 0,
            options: TftpOptions::default(),
            state: TftpState::Idle,
            block: 0,
            transfer_size: 0,
            bytes_transferred: 0,
        }
    }

    /// Set options
    pub fn set_options(&mut self, options: TftpOptions) {
        self.options = options;
    }

    /// Get file size
    pub fn get_file_size(&mut self, _filename: &str) -> Result<u64, NetworkError> {
        // Build RRQ with tsize=0 option
        // Send and wait for OACK

        Ok(0)
    }

    /// Download file
    pub fn download(
        &mut self,
        filename: &str,
        _buffer: &mut [u8],
        mode: TftpMode,
    ) -> Result<usize, NetworkError> {
        self.state = TftpState::WaitOack;
        self.block = 0;
        self.bytes_transferred = 0;

        // Build RRQ packet
        let _packet = self.build_rrq(filename, mode)?;

        // Send request and receive data
        // Process blocks until complete

        self.state = TftpState::Complete;
        Ok(self.bytes_transferred as usize)
    }

    /// Build RRQ packet
    fn build_rrq(&self, filename: &str, mode: TftpMode) -> Result<[u8; 512], NetworkError> {
        let mut packet = [0u8; 512];

        // Opcode
        packet[0..2].copy_from_slice(&(TftpOpcode::Rrq as u16).to_be_bytes());
        let mut pos = 2;

        // Filename
        let fname_bytes = filename.as_bytes();
        packet[pos..pos + fname_bytes.len()].copy_from_slice(fname_bytes);
        pos += fname_bytes.len();
        packet[pos] = 0; // Null terminator
        pos += 1;

        // Mode
        let mode_str = mode.as_str();
        let mode_bytes = mode_str.as_bytes();
        packet[pos..pos + mode_bytes.len()].copy_from_slice(mode_bytes);
        pos += mode_bytes.len();
        packet[pos] = 0;

        Ok(packet)
    }

    /// Process received packet
    fn _process_packet(&mut self, packet: &[u8], buffer: &mut [u8]) -> Result<bool, NetworkError> {
        if packet.len() < 4 {
            return Err(NetworkError::InvalidPacket);
        }

        let opcode = u16::from_be_bytes([packet[0], packet[1]]);

        match opcode {
            3 => {
                // DATA
                let block = u16::from_be_bytes([packet[2], packet[3]]);
                let data = &packet[4..];

                if block == self.block.wrapping_add(1) {
                    self.block = block;

                    let offset = (block as usize - 1) * self.options.blksize as usize;
                    let len = data.len().min(buffer.len().saturating_sub(offset));
                    buffer[offset..offset + len].copy_from_slice(&data[..len]);

                    self.bytes_transferred += data.len() as u64;

                    // Send ACK

                    // Check if last block
                    if data.len() < self.options.blksize as usize {
                        self.state = TftpState::Complete;
                        return Ok(true);
                    }
                }
            },
            5 => {
                // ERROR
                let code = u16::from_be_bytes([packet[2], packet[3]]);
                self.state = TftpState::Error;
                return Err(NetworkError::TftpError(code));
            },
            6 => {
                // OACK
                self.state = TftpState::Transferring;
                self.block = 0;
                // Parse options and send ACK 0
            },
            _ => {
                return Err(NetworkError::InvalidPacket);
            },
        }

        Ok(false)
    }

    /// Get transfer progress
    pub fn progress(&self) -> (u64, u64) {
        (self.bytes_transferred, self.transfer_size)
    }

    /// Get state
    pub fn state(&self) -> TftpState {
        self.state
    }
}

// =============================================================================
// HTTP BOOT
// =============================================================================

/// HTTP method
#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Options,
    Trace,
    Connect,
    Patch,
}

impl HttpMethod {
    fn _as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
            Self::Trace => "TRACE",
            Self::Connect => "CONNECT",
            Self::Patch => "PATCH",
        }
    }
}

/// HTTP status code categories
#[derive(Debug, Clone, Copy)]
pub enum HttpStatusCategory {
    Informational, // 1xx
    Success,       // 2xx
    Redirection,   // 3xx
    ClientError,   // 4xx
    ServerError,   // 5xx
    Unknown,
}

/// HTTP response status
#[derive(Debug, Clone, Copy)]
pub struct HttpStatus {
    pub code: u16,
}

impl HttpStatus {
    /// Create new status
    pub const fn new(code: u16) -> Self {
        Self { code }
    }

    /// Get category
    pub fn category(&self) -> HttpStatusCategory {
        match self.code / 100 {
            1 => HttpStatusCategory::Informational,
            2 => HttpStatusCategory::Success,
            3 => HttpStatusCategory::Redirection,
            4 => HttpStatusCategory::ClientError,
            5 => HttpStatusCategory::ServerError,
            _ => HttpStatusCategory::Unknown,
        }
    }

    /// Is success (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.code)
    }

    /// Is redirect (3xx)
    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.code)
    }

    /// Is error (4xx or 5xx)
    pub fn is_error(&self) -> bool {
        self.code >= 400
    }

    // Common status codes
    pub const OK: Self = Self::new(200);
    pub const CREATED: Self = Self::new(201);
    pub const ACCEPTED: Self = Self::new(202);
    pub const NO_CONTENT: Self = Self::new(204);
    pub const MOVED_PERMANENTLY: Self = Self::new(301);
    pub const FOUND: Self = Self::new(302);
    pub const NOT_MODIFIED: Self = Self::new(304);
    pub const TEMPORARY_REDIRECT: Self = Self::new(307);
    pub const PERMANENT_REDIRECT: Self = Self::new(308);
    pub const BAD_REQUEST: Self = Self::new(400);
    pub const UNAUTHORIZED: Self = Self::new(401);
    pub const FORBIDDEN: Self = Self::new(403);
    pub const NOT_FOUND: Self = Self::new(404);
    pub const METHOD_NOT_ALLOWED: Self = Self::new(405);
    pub const REQUEST_TIMEOUT: Self = Self::new(408);
    pub const INTERNAL_SERVER_ERROR: Self = Self::new(500);
    pub const NOT_IMPLEMENTED: Self = Self::new(501);
    pub const BAD_GATEWAY: Self = Self::new(502);
    pub const SERVICE_UNAVAILABLE: Self = Self::new(503);
    pub const GATEWAY_TIMEOUT: Self = Self::new(504);
}

/// HTTP header
#[derive(Debug, Clone)]
pub struct HttpHeader {
    /// Header name
    pub name: [u8; 64],
    pub name_len: usize,
    /// Header value
    pub value: [u8; 256],
    pub value_len: usize,
}

impl HttpHeader {
    /// Create new header
    pub fn new(name: &str, value: &str) -> Self {
        let mut header = Self {
            name: [0; 64],
            name_len: 0,
            value: [0; 256],
            value_len: 0,
        };

        let name_bytes = name.as_bytes();
        let value_bytes = value.as_bytes();

        header.name_len = name_bytes.len().min(64);
        header.name[..header.name_len].copy_from_slice(&name_bytes[..header.name_len]);

        header.value_len = value_bytes.len().min(256);
        header.value[..header.value_len].copy_from_slice(&value_bytes[..header.value_len]);

        header
    }

    /// Get name as str
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }

    /// Get value as str
    pub fn value_str(&self) -> &str {
        core::str::from_utf8(&self.value[..self.value_len]).unwrap_or("")
    }
}

/// HTTP request
pub struct HttpRequest {
    /// Method
    pub method: HttpMethod,
    /// URL
    pub url: [u8; 512],
    pub url_len: usize,
    /// Headers
    pub headers: [HttpHeader; 16],
    pub header_count: usize,
    /// Body
    pub body: Option<([u8; 4096], usize)>,
}

impl HttpRequest {
    /// Create GET request
    pub fn get(url: &str) -> Self {
        let mut req = Self {
            method: HttpMethod::Get,
            url: [0; 512],
            url_len: 0,
            headers: core::array::from_fn(|_| HttpHeader::new("", "")),
            header_count: 0,
            body: None,
        };

        let bytes = url.as_bytes();
        req.url_len = bytes.len().min(512);
        req.url[..req.url_len].copy_from_slice(&bytes[..req.url_len]);

        req
    }

    /// Create HEAD request
    pub fn head(url: &str) -> Self {
        let mut req = Self::get(url);
        req.method = HttpMethod::Head;
        req
    }

    /// Add header
    pub fn add_header(&mut self, name: &str, value: &str) -> &mut Self {
        if self.header_count < 16 {
            self.headers[self.header_count] = HttpHeader::new(name, value);
            self.header_count += 1;
        }
        self
    }

    /// Get URL as str
    pub fn url_str(&self) -> &str {
        core::str::from_utf8(&self.url[..self.url_len]).unwrap_or("")
    }
}

/// HTTP response
pub struct HttpResponse {
    /// Status
    pub status: HttpStatus,
    /// Headers
    pub headers: [HttpHeader; 16],
    pub header_count: usize,
    /// Content length
    pub content_length: Option<u64>,
    /// Content type
    pub content_type: [u8; 64],
    pub content_type_len: usize,
}

impl HttpResponse {
    /// Create new response
    pub fn new(status: HttpStatus) -> Self {
        Self {
            status,
            headers: core::array::from_fn(|_| HttpHeader::new("", "")),
            header_count: 0,
            content_length: None,
            content_type: [0; 64],
            content_type_len: 0,
        }
    }

    /// Get header value
    pub fn get_header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_ascii_lowercase();
        for i in 0..self.header_count {
            let header = &self.headers[i];
            // Case-insensitive comparison
            if header.name_str().to_ascii_lowercase() == name_lower {
                return Some(header.value_str());
            }
        }
        None
    }

    /// Get content type as str
    pub fn content_type_str(&self) -> &str {
        core::str::from_utf8(&self.content_type[..self.content_type_len]).unwrap_or("")
    }
}

// Trait extension for to_ascii_lowercase on str
trait _AsciiLowercaseExt {
    fn _to_ascii_lowercase(&self) -> _SmallString;
}

impl _AsciiLowercaseExt for str {
    fn _to_ascii_lowercase(&self) -> _SmallString {
        let mut s = _SmallString::_new();
        for c in self.chars() {
            s._push(c.to_ascii_lowercase());
        }
        s
    }
}

/// Small string buffer
struct _SmallString {
    buf: [u8; 64],
    len: usize,
}

impl _SmallString {
    fn _new() -> Self {
        Self {
            buf: [0; 64],
            len: 0,
        }
    }

    fn _push(&mut self, c: char) {
        if self.len < 64 {
            self.buf[self.len] = c as u8;
            self.len += 1;
        }
    }
}

impl PartialEq<_SmallString> for _SmallString {
    fn eq(&self, other: &_SmallString) -> bool {
        self.len == other.len && self.buf[..self.len] == other.buf[..other.len]
    }
}

/// HTTP boot client
pub struct HttpBootClient {
    /// Server address
    _server: IpAddress,
    /// Server port
    _port: u16,
    /// DNS server
    dns: Ipv4Address,
    /// Connected
    connected: bool,
}

impl HttpBootClient {
    /// Create new HTTP boot client
    pub fn new(server: IpAddress, port: u16) -> Self {
        Self {
            _server: server,
            _port: port,
            dns: Ipv4Address::ZERO,
            connected: false,
        }
    }

    /// Set DNS server
    pub fn set_dns(&mut self, dns: Ipv4Address) {
        self.dns = dns;
    }

    /// Connect to server
    pub fn connect(&mut self) -> Result<(), NetworkError> {
        // Establish TCP connection
        self.connected = true;
        Ok(())
    }

    /// Disconnect
    pub fn disconnect(&mut self) -> Result<(), NetworkError> {
        self.connected = false;
        Ok(())
    }

    /// Send request
    pub fn request(&mut self, _req: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        if !self.connected {
            return Err(NetworkError::NotConnected);
        }

        // Build and send HTTP request
        // Receive and parse response

        Ok(HttpResponse::new(HttpStatus::OK))
    }

    /// Download file
    pub fn download(&mut self, url: &str, _buffer: &mut [u8]) -> Result<usize, NetworkError> {
        let req = HttpRequest::get(url);
        let _response = self.request(&req)?;

        // Receive body into buffer

        Ok(0)
    }

    /// Get file size (HEAD request)
    pub fn get_file_size(&mut self, url: &str) -> Result<u64, NetworkError> {
        let req = HttpRequest::head(url);
        let response = self.request(&req)?;

        response.content_length.ok_or(NetworkError::InvalidResponse)
    }
}

// =============================================================================
// SIMPLE NETWORK PROTOCOL
// =============================================================================

/// EFI Simple Network Protocol GUID
pub const EFI_SIMPLE_NETWORK_PROTOCOL_GUID: [u8; 16] = [
    0xCE, 0x34, 0x5B, 0xA3, 0xD5, 0xBE, 0xD2, 0x11, 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
];

/// Network interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    Stopped,
    Started,
    Initialized,
}

/// Network interface statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStatistics {
    /// Rx total frames
    pub rx_total_frames: u64,
    /// Rx good frames
    pub rx_good_frames: u64,
    /// Rx undersize frames
    pub rx_undersize_frames: u64,
    /// Rx oversize frames
    pub rx_oversize_frames: u64,
    /// Rx dropped frames
    pub rx_dropped_frames: u64,
    /// Rx unicast frames
    pub rx_unicast_frames: u64,
    /// Rx broadcast frames
    pub rx_broadcast_frames: u64,
    /// Rx multicast frames
    pub rx_multicast_frames: u64,
    /// Rx CRC error frames
    pub rx_crc_error_frames: u64,
    /// Rx total bytes
    pub rx_total_bytes: u64,
    /// Tx total frames
    pub tx_total_frames: u64,
    /// Tx good frames
    pub tx_good_frames: u64,
    /// Tx undersize frames
    pub tx_undersize_frames: u64,
    /// Tx oversize frames
    pub tx_oversize_frames: u64,
    /// Tx dropped frames
    pub tx_dropped_frames: u64,
    /// Tx unicast frames
    pub tx_unicast_frames: u64,
    /// Tx broadcast frames
    pub tx_broadcast_frames: u64,
    /// Tx multicast frames
    pub tx_multicast_frames: u64,
    /// Tx CRC error frames
    pub tx_crc_error_frames: u64,
    /// Tx total bytes
    pub tx_total_bytes: u64,
    /// Collisions
    pub collisions: u64,
}

/// Network interface
pub struct NetworkInterface {
    /// Protocol handle
    _handle: usize,
    /// MAC address
    mac: MacAddress,
    /// State
    state: NetworkState,
    /// MTU
    mtu: u32,
    /// Media present
    media_present: bool,
    /// Statistics
    statistics: NetworkStatistics,
}

impl NetworkInterface {
    /// Create new network interface
    pub fn new(handle: usize, mac: MacAddress) -> Self {
        Self {
            _handle: handle,
            mac,
            state: NetworkState::Stopped,
            mtu: 1500,
            media_present: false,
            statistics: NetworkStatistics::default(),
        }
    }

    /// Start interface
    pub fn start(&mut self) -> Result<(), NetworkError> {
        if self.state != NetworkState::Stopped {
            return Ok(());
        }

        // Call EFI_SIMPLE_NETWORK_PROTOCOL.Start()
        self.state = NetworkState::Started;
        Ok(())
    }

    /// Initialize interface
    pub fn initialize(&mut self, _extra_rx: usize, _extra_tx: usize) -> Result<(), NetworkError> {
        if self.state != NetworkState::Started {
            return Err(NetworkError::NotStarted);
        }

        // Call EFI_SIMPLE_NETWORK_PROTOCOL.Initialize()
        self.state = NetworkState::Initialized;
        Ok(())
    }

    /// Shutdown interface
    pub fn shutdown(&mut self) -> Result<(), NetworkError> {
        if self.state != NetworkState::Initialized {
            return Ok(());
        }

        // Call EFI_SIMPLE_NETWORK_PROTOCOL.Shutdown()
        self.state = NetworkState::Started;
        Ok(())
    }

    /// Stop interface
    pub fn stop(&mut self) -> Result<(), NetworkError> {
        if self.state == NetworkState::Stopped {
            return Ok(());
        }

        if self.state == NetworkState::Initialized {
            self.shutdown()?;
        }

        // Call EFI_SIMPLE_NETWORK_PROTOCOL.Stop()
        self.state = NetworkState::Stopped;
        Ok(())
    }

    /// Transmit packet
    pub fn transmit(
        &mut self,
        _dest_mac: &MacAddress,
        _protocol: u16,
        data: &[u8],
    ) -> Result<(), NetworkError> {
        if self.state != NetworkState::Initialized {
            return Err(NetworkError::NotStarted);
        }

        if !self.media_present {
            return Err(NetworkError::NoMedia);
        }

        // Call EFI_SIMPLE_NETWORK_PROTOCOL.Transmit()
        self.statistics.tx_total_frames += 1;
        self.statistics.tx_total_bytes += data.len() as u64;

        Ok(())
    }

    /// Receive packet
    pub fn receive(&mut self, _buffer: &mut [u8]) -> Result<usize, NetworkError> {
        if self.state != NetworkState::Initialized {
            return Err(NetworkError::NotStarted);
        }

        // Call EFI_SIMPLE_NETWORK_PROTOCOL.Receive()

        Ok(0)
    }

    /// Get MAC address
    pub fn mac_address(&self) -> MacAddress {
        self.mac
    }

    /// Get MTU
    pub fn mtu(&self) -> u32 {
        self.mtu
    }

    /// Get state
    pub fn state(&self) -> NetworkState {
        self.state
    }

    /// Check media presence
    pub fn is_media_present(&self) -> bool {
        self.media_present
    }

    /// Get statistics
    pub fn statistics(&self) -> &NetworkStatistics {
        &self.statistics
    }

    /// Reset statistics
    pub fn reset_statistics(&mut self) {
        self.statistics = NetworkStatistics::default();
    }
}

// =============================================================================
// URL PARSING
// =============================================================================

/// URL scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlScheme {
    Http,
    Https,
    Tftp,
    Ftp,
    Unknown,
}

impl UrlScheme {
    /// Default port
    pub fn default_port(&self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
            Self::Tftp => 69,
            Self::Ftp => 21,
            Self::Unknown => 0,
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "http" => Self::Http,
            "https" => Self::Https,
            "tftp" => Self::Tftp,
            "ftp" => Self::Ftp,
            _ => Self::Unknown,
        }
    }
}

impl _SmallString {
    fn _as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// Parsed URL
#[derive(Debug)]
pub struct Url {
    /// Scheme
    pub scheme: UrlScheme,
    /// Host
    pub host: [u8; 256],
    pub host_len: usize,
    /// Port
    pub port: u16,
    /// Path
    pub path: [u8; 512],
    pub path_len: usize,
    /// Query string
    pub query: [u8; 256],
    pub query_len: usize,
}

impl Url {
    /// Parse URL
    pub fn parse(url: &str) -> Result<Self, NetworkError> {
        let mut result = Self {
            scheme: UrlScheme::Unknown,
            host: [0; 256],
            host_len: 0,
            port: 0,
            path: [0; 512],
            path_len: 0,
            query: [0; 256],
            query_len: 0,
        };

        let url = url.as_bytes();
        let mut pos = 0;

        // Parse scheme
        if let Some(scheme_end) = find_bytes(url, b"://") {
            let scheme_str = core::str::from_utf8(&url[..scheme_end]).unwrap_or("");
            result.scheme = UrlScheme::from_str(scheme_str);
            result.port = result.scheme.default_port();
            pos = scheme_end + 3;
        }

        // Find end of host
        let host_end = find_byte(&url[pos..], b'/')
            .map(|i| pos + i)
            .unwrap_or(url.len());

        // Check for port
        let host_part = &url[pos..host_end];
        if let Some(port_sep) = find_byte(host_part, b':') {
            result.host_len = port_sep;
            result.host[..result.host_len].copy_from_slice(&host_part[..result.host_len]);

            // Parse port
            if let Ok(port_str) = core::str::from_utf8(&host_part[port_sep + 1..]) {
                result.port = parse_u16(port_str).unwrap_or(result.port);
            }
        } else {
            result.host_len = host_part.len().min(256);
            result.host[..result.host_len].copy_from_slice(&host_part[..result.host_len]);
        }

        pos = host_end;

        // Parse path
        if pos < url.len() {
            let path_end = find_byte(&url[pos..], b'?')
                .map(|i| pos + i)
                .unwrap_or(url.len());

            result.path_len = (path_end - pos).min(512);
            result.path[..result.path_len].copy_from_slice(&url[pos..pos + result.path_len]);

            pos = path_end;

            // Parse query
            if pos < url.len() && url[pos] == b'?' {
                pos += 1;
                result.query_len = (url.len() - pos).min(256);
                result.query[..result.query_len].copy_from_slice(&url[pos..pos + result.query_len]);
            }
        } else {
            result.path[0] = b'/';
            result.path_len = 1;
        }

        Ok(result)
    }

    /// Get host as string
    pub fn host_str(&self) -> &str {
        core::str::from_utf8(&self.host[..self.host_len]).unwrap_or("")
    }

    /// Get path as string
    pub fn path_str(&self) -> &str {
        core::str::from_utf8(&self.path[..self.path_len]).unwrap_or("/")
    }

    /// Get query as string
    pub fn query_str(&self) -> Option<&str> {
        if self.query_len > 0 {
            core::str::from_utf8(&self.query[..self.query_len]).ok()
        } else {
            None
        }
    }
}

/// Find byte sequence in byte slice
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }

    for i in 0..=(haystack.len() - needle.len()) {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }

    None
}

/// Find byte in byte slice
fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    for (i, &b) in haystack.iter().enumerate() {
        if b == needle {
            return Some(i);
        }
    }
    None
}

/// Parse u16 from string
fn parse_u16(s: &str) -> Option<u16> {
    let mut result: u16 = 0;
    for c in s.chars() {
        if let Some(digit) = c.to_digit(10) {
            result = result.checked_mul(10)?.checked_add(digit as u16)?;
        } else {
            return None;
        }
    }
    Some(result)
}

// =============================================================================
// NETWORK ERROR
// =============================================================================

/// Network error
#[derive(Debug, Clone)]
pub enum NetworkError {
    /// Not started
    NotStarted,
    /// Not connected
    NotConnected,
    /// No media
    NoMedia,
    /// Timeout
    Timeout,
    /// Connection refused
    ConnectionRefused,
    /// Host unreachable
    HostUnreachable,
    /// Network unreachable
    NetworkUnreachable,
    /// DNS resolution failed
    DnsError,
    /// Invalid packet
    InvalidPacket,
    /// Invalid response
    InvalidResponse,
    /// Buffer too small
    BufferTooSmall,
    /// TFTP error
    TftpError(u16),
    /// HTTP error
    HttpError(u16),
    /// Protocol error
    ProtocolError,
    /// Unsupported
    Unsupported,
    /// Out of resources
    OutOfResources,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "network not started"),
            Self::NotConnected => write!(f, "not connected"),
            Self::NoMedia => write!(f, "no media"),
            Self::Timeout => write!(f, "timeout"),
            Self::ConnectionRefused => write!(f, "connection refused"),
            Self::HostUnreachable => write!(f, "host unreachable"),
            Self::NetworkUnreachable => write!(f, "network unreachable"),
            Self::DnsError => write!(f, "DNS resolution failed"),
            Self::InvalidPacket => write!(f, "invalid packet"),
            Self::InvalidResponse => write!(f, "invalid response"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::TftpError(code) => write!(f, "TFTP error: {}", code),
            Self::HttpError(code) => write!(f, "HTTP error: {}", code),
            Self::ProtocolError => write!(f, "protocol error"),
            Self::Unsupported => write!(f, "unsupported"),
            Self::OutOfResources => write!(f, "out of resources"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_mac_address() {
        let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(mac.is_unicast());
        assert!(!mac.is_broadcast());
        assert!(!mac.is_multicast());
    }

    #[test]
    fn test_ipv4_address() {
        let ip = Ipv4Address::new(192, 168, 1, 1);
        assert!(ip.is_private());
        assert!(!ip.is_loopback());

        let loopback = Ipv4Address::LOOPBACK;
        assert!(loopback.is_loopback());
    }

    #[test]
    fn test_http_status() {
        assert!(HttpStatus::OK.is_success());
        assert!(HttpStatus::NOT_FOUND.is_error());
        assert!(HttpStatus::MOVED_PERMANENTLY.is_redirect());
    }

    #[test]
    fn test_url_parse() {
        let url = Url::parse("http://example.com:8080/path?query=value").unwrap();
        assert_eq!(url.scheme, UrlScheme::Http);
        assert_eq!(url.host_str(), "example.com");
        assert_eq!(url.port, 8080);
        assert_eq!(url.path_str(), "/path");
    }
}
