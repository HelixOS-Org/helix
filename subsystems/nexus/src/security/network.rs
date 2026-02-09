//! Network security monitoring and traffic analysis.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::{format, vec};

use super::types::{Threat, ThreatSeverity, ThreatType};
use crate::core::NexusTimestamp;

// ============================================================================
// NETWORK SECURITY MONITOR
// ============================================================================

/// Network security monitoring
pub struct NetworkSecurityMonitor {
    /// Connection tracking
    connections: BTreeMap<u64, ConnectionInfo>,
    /// Blocked destinations
    blocked_destinations: Vec<u64>, // IP hashes
    /// Allowed ports
    allowed_ports: Vec<u16>,
    /// Traffic analysis
    traffic_stats: BTreeMap<u64, TrafficStats>, // process_id -> stats
    /// Anomaly thresholds
    thresholds: NetworkThresholds,
}

/// Connection information
#[derive(Debug, Clone)]
struct ConnectionInfo {
    /// Process ID
    process_id: u64,
    /// Local port
    local_port: u16,
    /// Remote address hash
    remote_hash: u64,
    /// Remote port
    remote_port: u16,
    /// Bytes sent
    bytes_sent: u64,
    /// Bytes received
    bytes_received: u64,
    /// Start time
    start_time: NexusTimestamp,
    /// Is encrypted
    encrypted: bool,
}

/// Per-process traffic statistics
#[derive(Debug, Clone, Default)]
struct TrafficStats {
    /// Total bytes out
    bytes_out: u64,
    /// Total bytes in
    bytes_in: u64,
    /// Unique destinations
    unique_destinations: u32,
    /// Failed connections
    failed_connections: u32,
    /// Packets per second (out)
    pps_out: f64,
    /// Last update
    last_update: u64,
}

/// Network anomaly thresholds
#[derive(Debug, Clone)]
pub struct NetworkThresholds {
    /// Max bytes out per process
    pub max_bytes_out: u64,
    /// Max unique destinations
    pub max_destinations: u32,
    /// Max packets per second
    pub max_pps: f64,
    /// Max failed connections
    pub max_failed: u32,
}

impl Default for NetworkThresholds {
    fn default() -> Self {
        Self {
            max_bytes_out: 100 * 1024 * 1024, // 100MB
            max_destinations: 100,
            max_pps: 10000.0,
            max_failed: 50,
        }
    }
}

impl NetworkSecurityMonitor {
    /// Create new network security monitor
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
            blocked_destinations: Vec::new(),
            allowed_ports: vec![80, 443, 22, 53], // Default allowed
            traffic_stats: BTreeMap::new(),
            thresholds: NetworkThresholds::default(),
        }
    }

    /// Record new connection
    pub fn record_connection(
        &mut self,
        conn_id: u64,
        process_id: u64,
        local_port: u16,
        remote_hash: u64,
        remote_port: u16,
    ) -> bool {
        // Check if blocked
        if self.blocked_destinations.contains(&remote_hash) {
            return false;
        }

        // Check port allowlist (for outgoing connections)
        // This is simplified - real implementation would be more nuanced

        self.connections.insert(conn_id, ConnectionInfo {
            process_id,
            local_port,
            remote_hash,
            remote_port,
            bytes_sent: 0,
            bytes_received: 0,
            start_time: NexusTimestamp::now(),
            encrypted: remote_port == 443,
        });

        // Update stats
        let stats = self.traffic_stats.entry(process_id).or_default();
        stats.unique_destinations += 1;

        true
    }

    /// Record traffic
    pub fn record_traffic(&mut self, conn_id: u64, bytes_out: u64, bytes_in: u64) {
        if let Some(conn) = self.connections.get_mut(&conn_id) {
            conn.bytes_sent += bytes_out;
            conn.bytes_received += bytes_in;

            // Update process stats
            if let Some(stats) = self.traffic_stats.get_mut(&conn.process_id) {
                stats.bytes_out += bytes_out;
                stats.bytes_in += bytes_in;
            }
        }
    }

    /// Check for data exfiltration
    pub fn check_exfiltration(&self, process_id: u64) -> Option<Threat> {
        let stats = self.traffic_stats.get(&process_id)?;

        // Check for excessive outbound traffic
        if stats.bytes_out > self.thresholds.max_bytes_out {
            let mut threat = Threat::new(ThreatType::DataExfiltration, process_id)
                .with_severity(ThreatSeverity::High)
                .with_description(format!(
                    "Excessive outbound traffic: {} bytes",
                    stats.bytes_out
                ));
            threat
                .context
                .insert("bytes_out".into(), format!("{}", stats.bytes_out));
            return Some(threat);
        }

        // Check for connection to many destinations
        if stats.unique_destinations > self.thresholds.max_destinations {
            let threat = Threat::new(ThreatType::DataExfiltration, process_id)
                .with_severity(ThreatSeverity::Medium)
                .with_description(format!(
                    "Connected to {} unique destinations",
                    stats.unique_destinations
                ));
            return Some(threat);
        }

        None
    }

    /// Check for port scanning
    pub fn check_port_scan(&self, process_id: u64) -> Option<Threat> {
        let stats = self.traffic_stats.get(&process_id)?;

        if stats.failed_connections > self.thresholds.max_failed {
            let threat = Threat::new(ThreatType::NetworkAnomaly, process_id)
                .with_severity(ThreatSeverity::Medium)
                .with_description("Potential port scanning detected");
            return Some(threat);
        }

        None
    }

    /// Block destination
    #[inline]
    pub fn block_destination(&mut self, hash: u64) {
        if !self.blocked_destinations.contains(&hash) {
            self.blocked_destinations.push(hash);
        }
    }

    /// Set thresholds
    #[inline(always)]
    pub fn set_thresholds(&mut self, thresholds: NetworkThresholds) {
        self.thresholds = thresholds;
    }
}

impl Default for NetworkSecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}
