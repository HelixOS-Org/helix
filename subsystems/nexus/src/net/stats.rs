//! Network Statistics
//!
//! Interface, ring buffer, and queue statistics.

// ============================================================================
// INTERFACE STATISTICS
// ============================================================================

/// Interface statistics
#[derive(Debug, Clone, Default)]
pub struct InterfaceStats {
    // RX stats
    /// Received bytes
    pub rx_bytes: u64,
    /// Received packets
    pub rx_packets: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Receive drops
    pub rx_dropped: u64,
    /// Receive FIFO errors
    pub rx_fifo_errors: u64,
    /// Receive frame errors
    pub rx_frame_errors: u64,
    /// Receive compressed
    pub rx_compressed: u64,
    /// Receive multicast
    pub multicast: u64,

    // TX stats
    /// Transmitted bytes
    pub tx_bytes: u64,
    /// Transmitted packets
    pub tx_packets: u64,
    /// Transmit errors
    pub tx_errors: u64,
    /// Transmit drops
    pub tx_dropped: u64,
    /// Transmit FIFO errors
    pub tx_fifo_errors: u64,
    /// Collisions
    pub collisions: u64,
    /// Carrier errors
    pub tx_carrier_errors: u64,
    /// Transmit compressed
    pub tx_compressed: u64,
}

impl InterfaceStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Total bytes
    pub fn total_bytes(&self) -> u64 {
        self.rx_bytes + self.tx_bytes
    }

    /// Total packets
    pub fn total_packets(&self) -> u64 {
        self.rx_packets + self.tx_packets
    }

    /// Total errors
    pub fn total_errors(&self) -> u64 {
        self.rx_errors + self.tx_errors
    }

    /// Error rate
    pub fn error_rate(&self) -> f32 {
        let total = self.total_packets();
        if total > 0 {
            self.total_errors() as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Drop rate
    pub fn drop_rate(&self) -> f32 {
        let total = self.total_packets();
        if total > 0 {
            (self.rx_dropped + self.tx_dropped) as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Average packet size
    pub fn avg_packet_size(&self) -> u64 {
        let total_packets = self.total_packets();
        if total_packets > 0 {
            self.total_bytes() / total_packets
        } else {
            0
        }
    }
}

// ============================================================================
// RING BUFFER STATISTICS
// ============================================================================

/// Ring buffer statistics
#[derive(Debug, Clone, Default)]
pub struct RingStats {
    /// RX ring size
    pub rx_pending: u32,
    /// RX max size
    pub rx_max: u32,
    /// TX ring size
    pub tx_pending: u32,
    /// TX max size
    pub tx_max: u32,
}

impl RingStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// RX utilization
    pub fn rx_utilization(&self) -> f32 {
        if self.rx_max > 0 {
            self.rx_pending as f32 / self.rx_max as f32
        } else {
            0.0
        }
    }

    /// TX utilization
    pub fn tx_utilization(&self) -> f32 {
        if self.tx_max > 0 {
            self.tx_pending as f32 / self.tx_max as f32
        } else {
            0.0
        }
    }
}

// ============================================================================
// QUEUE STATISTICS
// ============================================================================

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Bytes
    pub bytes: u64,
    /// Packets
    pub packets: u64,
    /// Drops
    pub drops: u64,
    /// Overlimits
    pub overlimits: u64,
    /// Requeues
    pub requeues: u64,
    /// Backlog bytes
    pub backlog: u64,
    /// Backlog packets
    pub qlen: u32,
}

impl QueueStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }
}
