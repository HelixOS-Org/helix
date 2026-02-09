//! Connection Predictor
//!
//! Predicts connection patterns.

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

/// Connection pattern for a source
#[derive(Debug, Clone)]
struct ConnectionPattern {
    /// Common destinations
    common_destinations: ArrayMap<u32, 32>,
    /// Common ports
    common_ports: BTreeMap<u16, u32>,
    /// Average connection rate
    avg_rate: f64,
    /// Total connections
    total: u64,
}

/// Connection record
#[derive(Debug, Clone, Copy)]
struct ConnectionRecord {
    /// Source IP
    src_ip: u32,
    /// Destination IP
    dst_ip: u32,
    /// Destination port
    dst_port: u16,
    /// Timestamp
    timestamp: u64,
}

/// Predicts connection patterns
pub struct ConnectionPredictor {
    /// Connection patterns by source
    patterns: BTreeMap<u32, ConnectionPattern>,
    /// Recent connections
    recent_connections: VecDeque<ConnectionRecord>,
    /// Max history
    max_history: usize,
}

impl ConnectionPredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            recent_connections: VecDeque::new(),
            max_history: 10000,
        }
    }

    /// Record new connection
    pub fn record_connection(&mut self, src_ip: u32, dst_ip: u32, dst_port: u16) {
        let record = ConnectionRecord {
            src_ip,
            dst_ip,
            dst_port,
            timestamp: NexusTimestamp::now().raw(),
        };

        self.recent_connections.push_back(record);
        if self.recent_connections.len() > self.max_history {
            self.recent_connections.pop_front();
        }

        // Update pattern
        let pattern = self
            .patterns
            .entry(src_ip)
            .or_insert_with(|| ConnectionPattern {
                common_destinations: ArrayMap::new(0),
                common_ports: BTreeMap::new(),
                avg_rate: 0.0,
                total: 0,
            });

        *pattern.common_destinations.entry(dst_ip).or_insert(0) += 1;
        *pattern.common_ports.entry(dst_port).or_insert(0) += 1;
        pattern.total += 1;
    }

    /// Predict likely next connections for source
    pub fn predict_connections(&self, src_ip: u32, n: usize) -> Vec<(u32, u16)> {
        let pattern = match self.patterns.get(&src_ip) {
            Some(p) => p,
            None => return Vec::new(),
        };

        // Get top destinations
        let mut destinations: Vec<_> = pattern.common_destinations.iter().collect();
        destinations.sort_by(|a, b| b.1.cmp(a.1));

        // Get top ports
        let mut ports: Vec<_> = pattern.common_ports.iter().collect();
        ports.sort_by(|a, b| b.1.cmp(a.1));

        // Combine top destinations with top ports
        let mut predictions = Vec::new();
        for &(&dst, _) in destinations.iter().take(n) {
            for &(&port, _) in ports.iter().take(2) {
                predictions.push((dst, port));
                if predictions.len() >= n {
                    break;
                }
            }
            if predictions.len() >= n {
                break;
            }
        }

        predictions
    }

    /// Get connection rate for source
    pub fn connection_rate(&self, src_ip: u32) -> f64 {
        let records: Vec<_> = self
            .recent_connections
            .iter()
            .filter(|r| r.src_ip == src_ip)
            .collect();

        if records.len() < 2 {
            return 0.0;
        }

        let first = records[0].timestamp;
        let last = records[records.len() - 1].timestamp;
        let duration = last.saturating_sub(first);

        if duration == 0 {
            return 0.0;
        }

        records.len() as f64 * 1_000_000_000.0 / duration as f64
    }

    /// Is connection anomalous?
    pub fn is_anomalous(&self, src_ip: u32, dst_ip: u32, dst_port: u16) -> bool {
        let pattern = match self.patterns.get(&src_ip) {
            Some(p) if p.total >= 10 => p,
            _ => return false, // Not enough data
        };

        // Check if destination is common
        let dst_count = pattern.common_destinations.get(&dst_ip).unwrap_or(&0);
        let dst_ratio = *dst_count as f64 / pattern.total as f64;

        // Check if port is common
        let port_count = pattern.common_ports.get(&dst_port).unwrap_or(&0);
        let port_ratio = *port_count as f64 / pattern.total as f64;

        // Both destination and port are unusual
        dst_ratio < 0.01 && port_ratio < 0.01
    }

    /// Clear all data
    #[inline(always)]
    pub fn clear(&mut self) {
        self.patterns.clear();
        self.recent_connections.clear();
    }
}

impl Default for ConnectionPredictor {
    fn default() -> Self {
        Self::new()
    }
}
