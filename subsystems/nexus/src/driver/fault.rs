//! Driver fault prediction.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::DriverId;
use crate::core::NexusTimestamp;
use crate::math;

// ============================================================================
// FAULT PREDICTOR
// ============================================================================

/// Predicts driver faults
pub struct DriverFaultPredictor {
    /// Fault history
    history: BTreeMap<DriverId, Vec<FaultRecord>>,
    /// Predictions
    predictions: BTreeMap<DriverId, FaultPrediction>,
    /// Model parameters
    mtbf_estimates: BTreeMap<DriverId, f64>,
}

/// Fault record
#[derive(Debug, Clone)]
struct FaultRecord {
    /// Timestamp
    timestamp: u64,
    /// Fault type
    fault_type: FaultType,
    /// Was recovered
    #[allow(dead_code)]
    recovered: bool,
}

/// Fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    /// Timeout
    Timeout,
    /// Device error
    DeviceError,
    /// DMA error
    DmaError,
    /// Interrupt error
    InterruptError,
    /// Memory error
    MemoryError,
    /// Protocol error
    ProtocolError,
    /// Unknown error
    Unknown,
}

/// Fault prediction
#[derive(Debug, Clone)]
pub struct FaultPrediction {
    /// Driver ID
    pub driver_id: DriverId,
    /// Probability of fault in next hour
    pub probability: f64,
    /// Estimated time to failure (seconds)
    pub estimated_ttf: Option<u64>,
    /// Confidence
    pub confidence: f64,
    /// Likely fault type
    pub likely_type: Option<FaultType>,
}

impl DriverFaultPredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            history: BTreeMap::new(),
            predictions: BTreeMap::new(),
            mtbf_estimates: BTreeMap::new(),
        }
    }

    /// Record fault
    pub fn record_fault(&mut self, driver_id: DriverId, fault_type: FaultType, recovered: bool) {
        let record = FaultRecord {
            timestamp: NexusTimestamp::now().raw(),
            fault_type,
            recovered,
        };

        let history = self.history.entry(driver_id).or_default();
        history.push(record);

        // Update MTBF estimate
        self.update_mtbf(driver_id);

        // Update prediction
        self.update_prediction(driver_id);
    }

    /// Update MTBF estimate
    fn update_mtbf(&mut self, driver_id: DriverId) {
        let history = match self.history.get(&driver_id) {
            Some(h) if h.len() >= 2 => h,
            _ => return,
        };

        let mut intervals = Vec::new();
        for i in 1..history.len() {
            let interval = history[i]
                .timestamp
                .saturating_sub(history[i - 1].timestamp);
            intervals.push(interval as f64);
        }

        if !intervals.is_empty() {
            let mtbf = intervals.iter().sum::<f64>() / intervals.len() as f64;
            self.mtbf_estimates.insert(driver_id, mtbf);
        }
    }

    /// Update prediction
    fn update_prediction(&mut self, driver_id: DriverId) {
        let history = match self.history.get(&driver_id) {
            Some(h) if !h.is_empty() => h,
            _ => return,
        };

        let mtbf = self.mtbf_estimates.get(&driver_id).copied();

        // Calculate probability based on time since last fault
        let last_fault = history.last().map(|f| f.timestamp).unwrap_or(0);
        let now = NexusTimestamp::now().raw();
        let time_since_last = now.saturating_sub(last_fault) as f64;

        let probability = if let Some(mtbf) = mtbf {
            // Exponential distribution CDF
            1.0 - math::exp(-time_since_last / mtbf)
        } else {
            0.1 // Default low probability
        };

        // Find most common fault type
        let mut type_counts: BTreeMap<u8, u32> = BTreeMap::new();
        for record in history {
            let key = match record.fault_type {
                FaultType::Timeout => 0,
                FaultType::DeviceError => 1,
                FaultType::DmaError => 2,
                FaultType::InterruptError => 3,
                FaultType::MemoryError => 4,
                FaultType::ProtocolError => 5,
                FaultType::Unknown => 6,
            };
            *type_counts.entry(key).or_insert(0) += 1;
        }

        let likely_type = type_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(&key, _)| match key {
                0 => FaultType::Timeout,
                1 => FaultType::DeviceError,
                2 => FaultType::DmaError,
                3 => FaultType::InterruptError,
                4 => FaultType::MemoryError,
                5 => FaultType::ProtocolError,
                _ => FaultType::Unknown,
            });

        let prediction = FaultPrediction {
            driver_id,
            probability: probability.min(1.0),
            estimated_ttf: mtbf.map(|m| (m / 1_000_000_000.0) as u64),
            confidence: (history.len() as f64 / 10.0).min(1.0),
            likely_type,
        };

        self.predictions.insert(driver_id, prediction);
    }

    /// Get prediction
    #[inline(always)]
    pub fn get_prediction(&self, driver_id: DriverId) -> Option<&FaultPrediction> {
        self.predictions.get(&driver_id)
    }

    /// Get high-risk drivers
    #[inline]
    pub fn high_risk_drivers(&self, threshold: f64) -> Vec<&FaultPrediction> {
        self.predictions
            .values()
            .filter(|p| p.probability >= threshold)
            .collect()
    }

    /// Get MTBF estimate
    #[inline(always)]
    pub fn get_mtbf(&self, driver_id: DriverId) -> Option<f64> {
        self.mtbf_estimates.get(&driver_id).copied()
    }
}

impl Default for DriverFaultPredictor {
    fn default() -> Self {
        Self::new()
    }
}
