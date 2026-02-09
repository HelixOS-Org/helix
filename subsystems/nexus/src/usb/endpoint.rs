//! USB Endpoint
//!
//! USB endpoint definitions and transfer tracking.

use core::sync::atomic::{AtomicU64, Ordering};

/// Endpoint direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointDirection {
    /// In (device to host)
    In,
    /// Out (host to device)
    Out,
}

impl EndpointDirection {
    /// Get direction name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::In => "in",
            Self::Out => "out",
        }
    }
}

/// Endpoint transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    /// Control
    Control,
    /// Isochronous
    Isochronous,
    /// Bulk
    Bulk,
    /// Interrupt
    Interrupt,
}

impl TransferType {
    /// Get type name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Isochronous => "isochronous",
            Self::Bulk => "bulk",
            Self::Interrupt => "interrupt",
        }
    }

    /// From attributes
    #[inline]
    pub fn from_attributes(attrs: u8) -> Self {
        match attrs & 0x03 {
            0 => Self::Control,
            1 => Self::Isochronous,
            2 => Self::Bulk,
            3 => Self::Interrupt,
            _ => Self::Control,
        }
    }
}

/// USB endpoint
#[derive(Debug)]
pub struct UsbEndpoint {
    /// Endpoint address
    pub address: u8,
    /// Direction
    pub direction: EndpointDirection,
    /// Transfer type
    pub transfer_type: TransferType,
    /// Max packet size
    pub max_packet_size: u16,
    /// Interval (for interrupt/iso)
    pub interval: u8,
    /// Bytes transferred
    pub bytes_transferred: AtomicU64,
    /// Transfer count
    pub transfer_count: AtomicU64,
    /// Error count
    pub error_count: AtomicU64,
}

impl Clone for UsbEndpoint {
    fn clone(&self) -> Self {
        Self {
            address: self.address,
            direction: self.direction,
            transfer_type: self.transfer_type,
            max_packet_size: self.max_packet_size,
            interval: self.interval,
            bytes_transferred: AtomicU64::new(self.bytes_transferred.load(Ordering::Relaxed)),
            transfer_count: AtomicU64::new(self.transfer_count.load(Ordering::Relaxed)),
            error_count: AtomicU64::new(self.error_count.load(Ordering::Relaxed)),
        }
    }
}

impl UsbEndpoint {
    /// Create new endpoint
    pub fn new(address: u8, direction: EndpointDirection, transfer_type: TransferType) -> Self {
        Self {
            address,
            direction,
            transfer_type,
            max_packet_size: 0,
            interval: 0,
            bytes_transferred: AtomicU64::new(0),
            transfer_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }

    /// Endpoint number
    #[inline(always)]
    pub fn number(&self) -> u8 {
        self.address & 0x0f
    }

    /// Record transfer
    #[inline]
    pub fn record_transfer(&self, bytes: u64, success: bool) {
        self.transfer_count.fetch_add(1, Ordering::Relaxed);
        if success {
            self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
        } else {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get bytes transferred
    #[inline(always)]
    pub fn bytes_transferred(&self) -> u64 {
        self.bytes_transferred.load(Ordering::Relaxed)
    }

    /// Get transfer count
    #[inline(always)]
    pub fn transfer_count(&self) -> u64 {
        self.transfer_count.load(Ordering::Relaxed)
    }

    /// Get error rate
    #[inline]
    pub fn error_rate(&self) -> f32 {
        let total = self.transfer_count.load(Ordering::Relaxed);
        let errors = self.error_count.load(Ordering::Relaxed);
        if total > 0 {
            errors as f32 / total as f32
        } else {
            0.0
        }
    }
}
