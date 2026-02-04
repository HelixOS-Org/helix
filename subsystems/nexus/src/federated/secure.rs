//! Secure aggregation protocol for federated learning.

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::federated::types::lcg_next;

/// Secret share for secure aggregation
#[derive(Debug, Clone)]
pub struct SecretShare {
    /// Share ID
    pub id: u32,
    /// Share values
    pub values: Vec<f64>,
    /// Source client
    pub source: u32,
    /// Target client
    pub target: u32,
}

/// Secure aggregation protocol
#[derive(Debug, Clone)]
pub struct SecureAggregation {
    /// Number of parties
    pub num_parties: usize,
    /// Threshold for reconstruction
    pub threshold: usize,
    /// Current round
    pub round: u32,
    /// Collected shares
    pub shares: BTreeMap<(u32, u32), SecretShare>,
    /// RNG state
    rng_state: u64,
}

impl SecureAggregation {
    /// Create a new secure aggregation
    pub fn new(num_parties: usize, threshold: usize) -> Self {
        Self {
            num_parties,
            threshold: threshold.min(num_parties),
            round: 0,
            shares: BTreeMap::new(),
            rng_state: 12345,
        }
    }

    /// Create secret shares for a value
    pub fn create_shares(&mut self, values: &[f64], client_id: u32) -> Vec<SecretShare> {
        let mut shares = Vec::new();
        let n = self.num_parties;

        // Create additive shares
        let mut remaining = values.to_vec();

        for target in 0..n as u32 {
            if target == client_id {
                continue;
            }

            // Random share
            let share_values: Vec<f64> = remaining
                .iter()
                .map(|_| {
                    self.rng_state = lcg_next(self.rng_state);
                    ((self.rng_state as f64 / u64::MAX as f64) - 0.5) * 2.0
                })
                .collect();

            // Subtract from remaining
            for (r, &s) in remaining.iter_mut().zip(share_values.iter()) {
                *r -= s;
            }

            shares.push(SecretShare {
                id: self.round,
                values: share_values,
                source: client_id,
                target,
            });
        }

        // Last share is the remaining
        shares.push(SecretShare {
            id: self.round,
            values: remaining,
            source: client_id,
            target: client_id,
        });

        shares
    }

    /// Submit a share
    pub fn submit_share(&mut self, share: SecretShare) {
        self.shares.insert((share.source, share.target), share);
    }

    /// Reconstruct aggregated value
    pub fn reconstruct(&self, client_id: u32) -> Option<Vec<f64>> {
        // Collect all shares intended for this client
        let client_shares: Vec<&SecretShare> = self
            .shares
            .values()
            .filter(|s| s.target == client_id)
            .collect();

        if client_shares.is_empty() {
            return None;
        }

        let dim = client_shares[0].values.len();
        let mut result = vec![0.0; dim];

        for share in client_shares {
            for (r, &s) in result.iter_mut().zip(share.values.iter()) {
                *r += s;
            }
        }

        Some(result)
    }

    /// Reset for new round
    pub fn new_round(&mut self) {
        self.round += 1;
        self.shares.clear();
    }
}
