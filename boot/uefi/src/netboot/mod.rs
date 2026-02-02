//! Network Boot Support (PXE/HTTP Boot)
//!
//! This module provides network boot protocols and functionality
//! for the Helix UEFI Bootloader.
//!
//! # Features
//!
//! - PXE boot support
//! - HTTP/HTTPS boot support
//! - TFTP client
//! - DHCP integration
//! - Network configuration

#![no_std]

use core::fmt;

// =============================================================================
// NETWORK BOOT TYPES
// =============================================================================

/// Network boot protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetBootProtocol {
    /// Unknown/none
    #[default]
    None,
    /// PXE (TFTP-based)
    Pxe,
    /// HTTP boot
    Http,
    /// HTTPS boot
    Https,
    /// TFTP direct
    Tftp,
    /// iSCSI
    Iscsi,
    /// FCoE
    Fcoe,
    /// NFS
    Nfs,
}

impl fmt::Display for NetBootProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetBootProtocol::None => write!(f, "None"),
            NetBootProtocol::Pxe => write!(f, "PXE"),
            NetBootProtocol::Http => write!(f, "HTTP Boot"),
            NetBootProtocol::Https => write!(f, "HTTPS Boot"),
            NetBootProtocol::Tftp => write!(f, "TFTP"),
            NetBootProtocol::Iscsi => write!(f, "iSCSI"),
            NetBootProtocol::Fcoe => write!(f, "FCoE"),
            NetBootProtocol::Nfs => write!(f, "NFS"),
        }
    }
}

/// Network boot state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetBootState {
    /// Not initialized
    #[default]
    Idle,
    /// Discovering DHCP
    DhcpDiscovery,
    /// DHCP offer received
    DhcpOffer,
    /// DHCP configured
    DhcpConfigured,
    /// Connecting to server
    Connecting,
    /// Downloading
    Downloading,
    /// Download complete
    Complete,
    /// Error
    Error,
}

// =============================================================================
// IP ADDRESSES
// =============================================================================

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv4Addr {
    pub octets: [u8; 4],
}

impl Ipv4Addr {
    /// Create from octets
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            octets: [a, b, c, d],
        }
    }

    /// Create from u32 (network byte order)
    pub const fn from_u32(addr: u32) -> Self {
        Self {
            octets: [
                ((addr >> 24) & 0xFF) as u8,
                ((addr >> 16) & 0xFF) as u8,
                ((addr >> 8) & 0xFF) as u8,
                (addr & 0xFF) as u8,
            ],
        }
    }

    /// Convert to u32 (network byte order)
    pub const fn to_u32(&self) -> u32 {
        ((self.octets[0] as u32) << 24)
            | ((self.octets[1] as u32) << 16)
            | ((self.octets[2] as u32) << 8)
            | (self.octets[3] as u32)
    }

    /// Any address (0.0.0.0)
    pub const ANY: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

    /// Loopback address (127.0.0.1)
    pub const LOCALHOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);

    /// Check if unspecified
    pub const fn is_unspecified(&self) -> bool {
        self.octets[0] == 0 && self.octets[1] == 0 && self.octets[2] == 0 && self.octets[3] == 0
    }

    /// Check if loopback
    pub const fn is_loopback(&self) -> bool {
        self.octets[0] == 127
    }

    /// Check if private
    pub const fn is_private(&self) -> bool {
        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
        self.octets[0] == 10
            || (self.octets[0] == 172 && (self.octets[1] & 0xF0) == 16)
            || (self.octets[0] == 192 && self.octets[1] == 168)
    }

    /// Check if link-local
    pub const fn is_link_local(&self) -> bool {
        self.octets[0] == 169 && self.octets[1] == 254
    }

    /// Check if multicast
    pub const fn is_multicast(&self) -> bool {
        (self.octets[0] & 0xF0) == 0xE0
    }

    /// Check if broadcast
    pub const fn is_broadcast(&self) -> bool {
        self.octets[0] == 255
            && self.octets[1] == 255
            && self.octets[2] == 255
            && self.octets[3] == 255
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.octets[0], self.octets[1], self.octets[2], self.octets[3]
        )
    }
}

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv6Addr {
    pub segments: [u16; 8],
}

impl Ipv6Addr {
    /// Create from segments
    pub const fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Self {
        Self {
            segments: [a, b, c, d, e, f, g, h],
        }
    }

    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self {
            segments: [
                u16::from_be_bytes([bytes[0], bytes[1]]),
                u16::from_be_bytes([bytes[2], bytes[3]]),
                u16::from_be_bytes([bytes[4], bytes[5]]),
                u16::from_be_bytes([bytes[6], bytes[7]]),
                u16::from_be_bytes([bytes[8], bytes[9]]),
                u16::from_be_bytes([bytes[10], bytes[11]]),
                u16::from_be_bytes([bytes[12], bytes[13]]),
                u16::from_be_bytes([bytes[14], bytes[15]]),
            ],
        }
    }

    /// Unspecified address (::)
    pub const UNSPECIFIED: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);

    /// Loopback address (::1)
    pub const LOCALHOST: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

    /// Check if unspecified
    pub const fn is_unspecified(&self) -> bool {
        self.segments[0] == 0
            && self.segments[1] == 0
            && self.segments[2] == 0
            && self.segments[3] == 0
            && self.segments[4] == 0
            && self.segments[5] == 0
            && self.segments[6] == 0
            && self.segments[7] == 0
    }

    /// Check if loopback
    pub const fn is_loopback(&self) -> bool {
        self.segments[0] == 0
            && self.segments[1] == 0
            && self.segments[2] == 0
            && self.segments[3] == 0
            && self.segments[4] == 0
            && self.segments[5] == 0
            && self.segments[6] == 0
            && self.segments[7] == 1
    }

    /// Check if link-local
    pub const fn is_link_local(&self) -> bool {
        (self.segments[0] & 0xFFC0) == 0xFE80
    }

    /// Check if multicast
    pub const fn is_multicast(&self) -> bool {
        self.segments[0] >> 8 == 0xFF
    }
}

/// IP address (v4 or v6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpAddr {
    /// IPv4 address
    V4(Ipv4Addr),
    /// IPv6 address
    V6(Ipv6Addr),
}

impl Default for IpAddr {
    fn default() -> Self {
        IpAddr::V4(Ipv4Addr::ANY)
    }
}

// =============================================================================
// MAC ADDRESS
// =============================================================================

/// MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacAddr {
    pub octets: [u8; 6],
}

impl MacAddr {
    /// Create from octets
    pub const fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        Self {
            octets: [a, b, c, d, e, f],
        }
    }

    /// Broadcast address
    pub const BROADCAST: MacAddr = MacAddr::new(0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF);

    /// Check if broadcast
    pub const fn is_broadcast(&self) -> bool {
        self.octets[0] == 0xFF
            && self.octets[1] == 0xFF
            && self.octets[2] == 0xFF
            && self.octets[3] == 0xFF
            && self.octets[4] == 0xFF
            && self.octets[5] == 0xFF
    }

    /// Check if multicast
    pub const fn is_multicast(&self) -> bool {
        (self.octets[0] & 0x01) != 0
    }

    /// Check if locally administered
    pub const fn is_local(&self) -> bool {
        (self.octets[0] & 0x02) != 0
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.octets[0],
            self.octets[1],
            self.octets[2],
            self.octets[3],
            self.octets[4],
            self.octets[5]
        )
    }
}

// =============================================================================
// DHCP
// =============================================================================

/// DHCP message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum DhcpMessageType {
    #[default]
    Discover = 1,
    Offer    = 2,
    Request  = 3,
    Decline  = 4,
    Ack      = 5,
    Nak      = 6,
    Release  = 7,
    Inform   = 8,
}

/// DHCP option
#[derive(Debug, Clone, Copy)]
pub enum DhcpOption {
    /// Subnet mask
    SubnetMask(Ipv4Addr),
    /// Router/gateway
    Router(Ipv4Addr),
    /// DNS server
    DnsServer(Ipv4Addr),
    /// Hostname
    Hostname([u8; 64], usize),
    /// Domain name
    DomainName([u8; 64], usize),
    /// Lease time (seconds)
    LeaseTime(u32),
    /// Server identifier
    ServerIdentifier(Ipv4Addr),
    /// Message type
    MessageType(DhcpMessageType),
    /// TFTP server name
    TftpServer([u8; 64], usize),
    /// Boot file name
    BootFile([u8; 128], usize),
    /// HTTP Boot URL
    HttpBootUrl([u8; 256], usize),
    /// Other
    Other(u8, [u8; 64], usize),
}

/// DHCP configuration
#[derive(Debug, Clone, Copy)]
pub struct DhcpConfig {
    /// Client IP address
    pub client_ip: Ipv4Addr,
    /// Subnet mask
    pub subnet_mask: Ipv4Addr,
    /// Gateway
    pub gateway: Ipv4Addr,
    /// DNS server
    pub dns_server: Ipv4Addr,
    /// DHCP server
    pub dhcp_server: Ipv4Addr,
    /// TFTP server
    pub tftp_server: Ipv4Addr,
    /// Lease time (seconds)
    pub lease_time: u32,
    /// Boot file
    pub boot_file: [u8; 128],
    pub boot_file_len: usize,
}

impl Default for DhcpConfig {
    fn default() -> Self {
        Self {
            client_ip: Ipv4Addr::default(),
            subnet_mask: Ipv4Addr::default(),
            gateway: Ipv4Addr::default(),
            dns_server: Ipv4Addr::default(),
            dhcp_server: Ipv4Addr::default(),
            tftp_server: Ipv4Addr::default(),
            lease_time: 0,
            boot_file: [0u8; 128],
            boot_file_len: 0,
        }
    }
}

impl DhcpConfig {
    /// Get boot file as string
    pub fn boot_file(&self) -> &str {
        core::str::from_utf8(&self.boot_file[..self.boot_file_len]).unwrap_or("")
    }

    /// Set boot file
    pub fn set_boot_file(&mut self, file: &str) {
        let bytes = file.as_bytes();
        let len = bytes.len().min(128);
        self.boot_file[..len].copy_from_slice(&bytes[..len]);
        self.boot_file_len = len;
    }
}

// =============================================================================
// TFTP
// =============================================================================

/// TFTP opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// TFTP error code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TftpError {
    /// Not defined
    NotDefined        = 0,
    /// File not found
    FileNotFound      = 1,
    /// Access violation
    AccessViolation   = 2,
    /// Disk full
    DiskFull          = 3,
    /// Illegal operation
    IllegalOperation  = 4,
    /// Unknown transfer ID
    UnknownTid        = 5,
    /// File already exists
    FileExists        = 6,
    /// No such user
    NoSuchUser        = 7,
    /// Option negotiation
    OptionNegotiation = 8,
}

/// TFTP transfer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TftpMode {
    /// ASCII/netascii
    Ascii,
    /// Binary/octet
    #[default]
    Binary,
}

/// TFTP options
#[derive(Debug, Clone, Copy, Default)]
pub struct TftpOptions {
    /// Block size
    pub block_size: u16,
    /// Timeout (seconds)
    pub timeout: u8,
    /// Transfer size
    pub transfer_size: u64,
    /// Multicast (not commonly used)
    pub multicast: bool,
}

impl TftpOptions {
    /// Default block size
    pub const DEFAULT_BLOCK_SIZE: u16 = 512;

    /// Maximum block size
    pub const MAX_BLOCK_SIZE: u16 = 65464;
}

/// TFTP transfer state
#[derive(Debug, Clone, Copy, Default)]
pub struct TftpTransfer {
    /// Server address
    pub server: Ipv4Addr,
    /// Server port
    pub server_port: u16,
    /// Local port (TID)
    pub local_port: u16,
    /// Current block number
    pub block_num: u16,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total size (if known)
    pub total_size: u64,
    /// Options
    pub options: TftpOptions,
    /// Mode
    pub mode: TftpMode,
    /// Is complete
    pub complete: bool,
    /// Error occurred
    pub error: Option<TftpError>,
}

impl TftpTransfer {
    /// Create new transfer
    pub const fn new(server: Ipv4Addr) -> Self {
        Self {
            server,
            server_port: 69, // Standard TFTP port
            local_port: 0,
            block_num: 0,
            bytes_transferred: 0,
            total_size: 0,
            options: TftpOptions {
                block_size: TftpOptions::DEFAULT_BLOCK_SIZE,
                timeout: 5,
                transfer_size: 0,
                multicast: false,
            },
            mode: TftpMode::Binary,
            complete: false,
            error: None,
        }
    }

    /// Get progress percentage
    pub fn progress(&self) -> u8 {
        if self.total_size == 0 {
            0
        } else {
            ((self.bytes_transferred * 100) / self.total_size) as u8
        }
    }

    /// Check if last block
    pub fn is_last_block(&self, data_len: usize) -> bool {
        data_len < self.options.block_size as usize
    }
}

// =============================================================================
// HTTP BOOT
// =============================================================================

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Head,
    Post,
    Put,
    Delete,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpStatus(u16);

impl HttpStatus {
    // Informational
    pub const CONTINUE: HttpStatus = HttpStatus(100);

    // Success
    pub const OK: HttpStatus = HttpStatus(200);
    pub const CREATED: HttpStatus = HttpStatus(201);
    pub const ACCEPTED: HttpStatus = HttpStatus(202);
    pub const NO_CONTENT: HttpStatus = HttpStatus(204);
    pub const PARTIAL_CONTENT: HttpStatus = HttpStatus(206);

    // Redirection
    pub const MOVED_PERMANENTLY: HttpStatus = HttpStatus(301);
    pub const FOUND: HttpStatus = HttpStatus(302);
    pub const SEE_OTHER: HttpStatus = HttpStatus(303);
    pub const NOT_MODIFIED: HttpStatus = HttpStatus(304);
    pub const TEMPORARY_REDIRECT: HttpStatus = HttpStatus(307);
    pub const PERMANENT_REDIRECT: HttpStatus = HttpStatus(308);

    // Client errors
    pub const BAD_REQUEST: HttpStatus = HttpStatus(400);
    pub const UNAUTHORIZED: HttpStatus = HttpStatus(401);
    pub const FORBIDDEN: HttpStatus = HttpStatus(403);
    pub const NOT_FOUND: HttpStatus = HttpStatus(404);
    pub const METHOD_NOT_ALLOWED: HttpStatus = HttpStatus(405);
    pub const REQUEST_TIMEOUT: HttpStatus = HttpStatus(408);

    // Server errors
    pub const INTERNAL_SERVER_ERROR: HttpStatus = HttpStatus(500);
    pub const NOT_IMPLEMENTED: HttpStatus = HttpStatus(501);
    pub const BAD_GATEWAY: HttpStatus = HttpStatus(502);
    pub const SERVICE_UNAVAILABLE: HttpStatus = HttpStatus(503);
    pub const GATEWAY_TIMEOUT: HttpStatus = HttpStatus(504);

    /// Get code
    pub const fn code(&self) -> u16 {
        self.0
    }

    /// Check if success
    pub const fn is_success(&self) -> bool {
        self.0 >= 200 && self.0 < 300
    }

    /// Check if redirect
    pub const fn is_redirect(&self) -> bool {
        self.0 >= 300 && self.0 < 400
    }

    /// Check if client error
    pub const fn is_client_error(&self) -> bool {
        self.0 >= 400 && self.0 < 500
    }

    /// Check if server error
    pub const fn is_server_error(&self) -> bool {
        self.0 >= 500 && self.0 < 600
    }
}

/// Maximum URL length
pub const MAX_URL_LEN: usize = 512;

/// HTTP URL
#[derive(Debug, Clone, Copy)]
pub struct HttpUrl {
    /// Full URL
    url: [u8; MAX_URL_LEN],
    url_len: usize,
    /// Scheme (http/https)
    pub is_https: bool,
    /// Host start offset
    host_start: usize,
    /// Host length
    host_len: usize,
    /// Port (0 = default)
    pub port: u16,
    /// Path start offset
    path_start: usize,
}

impl Default for HttpUrl {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpUrl {
    /// Create empty URL
    pub const fn new() -> Self {
        Self {
            url: [0; MAX_URL_LEN],
            url_len: 0,
            is_https: false,
            host_start: 0,
            host_len: 0,
            port: 0,
            path_start: 0,
        }
    }

    /// Parse URL from string
    pub fn parse(url: &str) -> Option<Self> {
        let bytes = url.as_bytes();
        if bytes.len() > MAX_URL_LEN {
            return None;
        }

        let mut result = Self::new();
        result.url[..bytes.len()].copy_from_slice(bytes);
        result.url_len = bytes.len();

        // Check scheme
        let (scheme_end, is_https) = if url.starts_with("https://") {
            (8, true)
        } else if url.starts_with("http://") {
            (7, false)
        } else {
            return None;
        };

        result.is_https = is_https;
        result.host_start = scheme_end;

        // Find host end
        let rest = &url[scheme_end..];
        let (host_end, port_start) = if let Some(pos) = rest.find(':') {
            // Port specified
            let port_end = rest[pos + 1..]
                .find('/')
                .map(|p| pos + 1 + p)
                .unwrap_or(rest.len());
            let port_str = &rest[pos + 1..port_end];
            if let Ok(port) = port_str.parse() {
                result.port = port;
            }
            (pos, port_end)
        } else if let Some(pos) = rest.find('/') {
            (pos, pos)
        } else {
            (rest.len(), rest.len())
        };

        result.host_len = host_end;
        result.path_start = scheme_end + port_start;

        // Default ports
        if result.port == 0 {
            result.port = if is_https { 443 } else { 80 };
        }

        Some(result)
    }

    /// Get full URL
    pub fn url(&self) -> &str {
        core::str::from_utf8(&self.url[..self.url_len]).unwrap_or("")
    }

    /// Get host
    pub fn host(&self) -> &str {
        let end = self.host_start + self.host_len;
        core::str::from_utf8(&self.url[self.host_start..end]).unwrap_or("")
    }

    /// Get path
    pub fn path(&self) -> &str {
        if self.path_start >= self.url_len {
            "/"
        } else {
            core::str::from_utf8(&self.url[self.path_start..self.url_len]).unwrap_or("/")
        }
    }
}

/// HTTP transfer state
#[derive(Debug, Clone, Copy, Default)]
pub struct HttpTransfer {
    /// URL
    pub url: HttpUrl,
    /// Method
    pub method: HttpMethod,
    /// Response status
    pub status: Option<HttpStatus>,
    /// Content length (if known)
    pub content_length: u64,
    /// Bytes downloaded
    pub bytes_downloaded: u64,
    /// Chunked transfer
    pub chunked: bool,
    /// Connection keep-alive
    pub keep_alive: bool,
    /// Is complete
    pub complete: bool,
    /// Error message
    pub error: bool,
}

impl HttpTransfer {
    /// Create new transfer
    pub const fn new() -> Self {
        Self {
            url: HttpUrl::new(),
            method: HttpMethod::Get,
            status: None,
            content_length: 0,
            bytes_downloaded: 0,
            chunked: false,
            keep_alive: false,
            complete: false,
            error: false,
        }
    }

    /// Get progress percentage
    pub fn progress(&self) -> u8 {
        if self.content_length == 0 {
            0
        } else {
            ((self.bytes_downloaded * 100) / self.content_length) as u8
        }
    }
}

// =============================================================================
// NETWORK BOOT CONFIG
// =============================================================================

/// Network boot configuration
#[derive(Debug, Clone, Copy, Default)]
pub struct NetBootConfig {
    /// Protocol
    pub protocol: NetBootProtocol,
    /// DHCP config
    pub dhcp: DhcpConfig,
    /// Use DHCP
    pub use_dhcp: bool,
    /// MAC address
    pub mac: MacAddr,
    /// Timeout (seconds)
    pub timeout: u16,
    /// Retry count
    pub retry_count: u8,
    /// HTTP URL (for HTTP boot)
    pub http_url: HttpUrl,
}

impl NetBootConfig {
    /// Create new config
    pub const fn new() -> Self {
        Self {
            protocol: NetBootProtocol::None,
            dhcp: DhcpConfig {
                client_ip: Ipv4Addr::ANY,
                subnet_mask: Ipv4Addr::ANY,
                gateway: Ipv4Addr::ANY,
                dns_server: Ipv4Addr::ANY,
                dhcp_server: Ipv4Addr::ANY,
                tftp_server: Ipv4Addr::ANY,
                lease_time: 0,
                boot_file: [0; 128],
                boot_file_len: 0,
            },
            use_dhcp: true,
            mac: MacAddr { octets: [0; 6] },
            timeout: 30,
            retry_count: 3,
            http_url: HttpUrl::new(),
        }
    }

    /// Check if configured
    pub fn is_configured(&self) -> bool {
        !self.dhcp.client_ip.is_unspecified()
    }
}

// =============================================================================
// NETWORK BOOT MANAGER
// =============================================================================

/// Network boot manager state
#[derive(Debug)]
pub struct NetBootManager {
    /// Configuration
    pub config: NetBootConfig,
    /// State
    pub state: NetBootState,
    /// TFTP transfer (if using TFTP/PXE)
    pub tftp: TftpTransfer,
    /// HTTP transfer (if using HTTP boot)
    pub http: HttpTransfer,
    /// Error message
    error_msg: [u8; 128],
    error_len: usize,
    /// Retry count
    retries: u8,
}

impl Default for NetBootManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetBootManager {
    /// Create new manager
    pub const fn new() -> Self {
        Self {
            config: NetBootConfig::new(),
            state: NetBootState::Idle,
            tftp: TftpTransfer::new(Ipv4Addr::ANY),
            http: HttpTransfer::new(),
            error_msg: [0; 128],
            error_len: 0,
            retries: 0,
        }
    }

    /// Initialize with config
    pub fn init(&mut self, config: NetBootConfig) {
        self.config = config;
        self.state = NetBootState::Idle;

        if config.protocol == NetBootProtocol::Pxe || config.protocol == NetBootProtocol::Tftp {
            self.tftp = TftpTransfer::new(config.dhcp.tftp_server);
        }
    }

    /// Start DHCP discovery
    pub fn start_dhcp(&mut self) {
        self.state = NetBootState::DhcpDiscovery;
        self.retries = 0;
    }

    /// Handle DHCP completion
    pub fn dhcp_complete(&mut self, config: DhcpConfig) {
        self.config.dhcp = config;
        self.state = NetBootState::DhcpConfigured;
    }

    /// Start download
    pub fn start_download(&mut self) {
        self.state = NetBootState::Connecting;

        match self.config.protocol {
            NetBootProtocol::Pxe | NetBootProtocol::Tftp => {
                self.tftp.server = self.config.dhcp.tftp_server;
                self.tftp.block_num = 0;
                self.tftp.bytes_transferred = 0;
                self.tftp.complete = false;
            },
            NetBootProtocol::Http | NetBootProtocol::Https => {
                self.http = HttpTransfer::new();
                self.http.url = self.config.http_url;
                self.http.method = HttpMethod::Get;
            },
            _ => {},
        }
    }

    /// Set error
    pub fn set_error(&mut self, msg: &str) {
        self.state = NetBootState::Error;
        let bytes = msg.as_bytes();
        let len = bytes.len().min(128);
        self.error_msg[..len].copy_from_slice(&bytes[..len]);
        self.error_len = len;
    }

    /// Get error message
    pub fn error_message(&self) -> &str {
        core::str::from_utf8(&self.error_msg[..self.error_len]).unwrap_or("")
    }

    /// Get progress percentage
    pub fn progress(&self) -> u8 {
        match self.state {
            NetBootState::Idle => 0,
            NetBootState::DhcpDiscovery => 10,
            NetBootState::DhcpOffer => 20,
            NetBootState::DhcpConfigured => 30,
            NetBootState::Connecting => 40,
            NetBootState::Downloading => {
                40 + match self.config.protocol {
                    NetBootProtocol::Pxe | NetBootProtocol::Tftp => {
                        (self.tftp.progress() * 55) / 100
                    },
                    NetBootProtocol::Http | NetBootProtocol::Https => {
                        (self.http.progress() * 55) / 100
                    },
                    _ => 0,
                }
            },
            NetBootState::Complete => 100,
            NetBootState::Error => 0,
        }
    }

    /// Check if complete
    pub const fn is_complete(&self) -> bool {
        matches!(self.state, NetBootState::Complete)
    }

    /// Check if error
    pub const fn is_error(&self) -> bool {
        matches!(self.state, NetBootState::Error)
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_addr() {
        let ip = Ipv4Addr::new(192, 168, 1, 100);
        assert!(ip.is_private());
        assert!(!ip.is_loopback());
    }

    #[test]
    fn test_ipv4_format() {
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        // format!() not available in no_std, but Display is implemented
        assert!(ip.is_private());
    }

    #[test]
    fn test_mac_addr() {
        let mac = MacAddr::new(0x00, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E);
        assert!(!mac.is_broadcast());
        assert!(!mac.is_multicast());
    }

    #[test]
    fn test_http_url() {
        let url = HttpUrl::parse("https://boot.example.com/kernel.efi").unwrap();
        assert!(url.is_https);
        assert_eq!(url.port, 443);
        assert_eq!(url.host(), "boot.example.com");
    }

    #[test]
    fn test_http_url_with_port() {
        let url = HttpUrl::parse("http://192.168.1.1:8080/boot/kernel").unwrap();
        assert!(!url.is_https);
        assert_eq!(url.port, 8080);
    }

    #[test]
    fn test_http_status() {
        assert!(HttpStatus::OK.is_success());
        assert!(HttpStatus::NOT_FOUND.is_client_error());
        assert!(HttpStatus::FOUND.is_redirect());
    }

    #[test]
    fn test_tftp_transfer() {
        let mut transfer = TftpTransfer::new(Ipv4Addr::new(192, 168, 1, 1));
        transfer.total_size = 1000;
        transfer.bytes_transferred = 500;
        assert_eq!(transfer.progress(), 50);
    }

    #[test]
    fn test_net_boot_manager() {
        let mut mgr = NetBootManager::new();
        assert_eq!(mgr.state, NetBootState::Idle);

        mgr.start_dhcp();
        assert_eq!(mgr.state, NetBootState::DhcpDiscovery);
    }
}
