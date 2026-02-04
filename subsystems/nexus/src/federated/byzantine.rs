//! Byzantine fault tolerance for federated learning.

use alloc::vec;
use alloc::vec::Vec;

use crate::federated::model::FederatedModel;
use crate::federated::update::ModelUpdate;

/// Byzantine-robust aggregation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByzantineDefense {
    /// Trimmed mean
    TrimmedMean,
    /// Median
    Median,
    /// Krum
    Krum,
    /// Multi-Krum
    MultiKrum,
    /// Bulyan
    Bulyan,
}

/// Byzantine-robust aggregator
#[derive(Debug, Clone)]
pub struct ByzantineRobustAggregator {
    /// Global model
    pub global_model: FederatedModel,
    /// Defense mechanism
    pub defense: ByzantineDefense,
    /// Trim fraction (for trimmed mean)
    pub trim_fraction: f64,
    /// Pending updates
    pub pending_updates: Vec<ModelUpdate>,
    /// Suspected byzantine clients
    pub suspected_clients: Vec<u32>,
    /// Krum parameter (number of neighbors)
    pub krum_k: usize,
}

impl ByzantineRobustAggregator {
    /// Create a new Byzantine-robust aggregator
    pub fn new(model: FederatedModel, defense: ByzantineDefense) -> Self {
        Self {
            global_model: model,
            defense,
            trim_fraction: 0.1,
            pending_updates: Vec::new(),
            suspected_clients: Vec::new(),
            krum_k: 2,
        }
    }

    /// Submit an update
    pub fn submit_update(&mut self, update: ModelUpdate) {
        self.pending_updates.push(update);
    }

    /// Aggregate using defense mechanism
    pub fn aggregate(&mut self) -> bool {
        if self.pending_updates.len() < 2 {
            return false;
        }

        let aggregated = match self.defense {
            ByzantineDefense::TrimmedMean => self.trimmed_mean(),
            ByzantineDefense::Median => self.median(),
            ByzantineDefense::Krum => self.krum(1),
            ByzantineDefense::MultiKrum => self.krum(self.pending_updates.len() / 2),
            ByzantineDefense::Bulyan => self.bulyan(),
        };

        // Apply aggregated update
        for (p, &a) in self
            .global_model
            .parameters
            .iter_mut()
            .zip(aggregated.iter())
        {
            *p += a;
        }

        self.global_model.version += 1;
        self.pending_updates.clear();

        true
    }

    /// Trimmed mean aggregation
    fn trimmed_mean(&self) -> Vec<f64> {
        let n = self.pending_updates.len();
        let num_params = self.pending_updates[0].delta.len();
        let trim = (n as f64 * self.trim_fraction) as usize;

        let mut result = vec![0.0; num_params];

        for i in 0..num_params {
            // Collect values for this parameter
            let mut values: Vec<f64> = self.pending_updates.iter().map(|u| u.delta[i]).collect();

            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

            // Trim and average
            let trimmed = &values[trim..n - trim];
            if !trimmed.is_empty() {
                result[i] = trimmed.iter().sum::<f64>() / trimmed.len() as f64;
            }
        }

        result
    }

    /// Median aggregation
    fn median(&self) -> Vec<f64> {
        let num_params = self.pending_updates[0].delta.len();
        let mut result = vec![0.0; num_params];

        for i in 0..num_params {
            let mut values: Vec<f64> = self.pending_updates.iter().map(|u| u.delta[i]).collect();

            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

            result[i] = values[values.len() / 2];
        }

        result
    }

    /// Krum/Multi-Krum aggregation
    fn krum(&mut self, m: usize) -> Vec<f64> {
        let n = self.pending_updates.len();
        let m = m.min(n);

        // Compute pairwise distances
        let mut distances: Vec<(usize, f64)> = Vec::new();

        for (i, update_i) in self.pending_updates.iter().enumerate() {
            let mut sum_dist = 0.0;
            let mut dists: Vec<f64> = Vec::new();

            for (j, update_j) in self.pending_updates.iter().enumerate() {
                if i != j {
                    let dist: f64 = update_i
                        .delta
                        .iter()
                        .zip(update_j.delta.iter())
                        .map(|(&a, &b)| (a - b).powi(2))
                        .sum();
                    dists.push(dist);
                }
            }

            // Sum of k nearest distances
            dists.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
            for d in dists.iter().take(self.krum_k.min(dists.len())) {
                sum_dist += d;
            }

            distances.push((i, sum_dist));
        }

        // Sort by score (lower is better)
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        // Mark suspected clients (high scores)
        for (i, _) in distances.iter().skip(m) {
            let client_id = self.pending_updates[*i].client_id;
            if !self.suspected_clients.contains(&client_id) {
                self.suspected_clients.push(client_id);
            }
        }

        // Average top-m updates
        let num_params = self.pending_updates[0].delta.len();
        let mut result = vec![0.0; num_params];

        for (i, _) in distances.iter().take(m) {
            for (r, &d) in result.iter_mut().zip(self.pending_updates[*i].delta.iter()) {
                *r += d;
            }
        }

        for r in &mut result {
            *r /= m as f64;
        }

        result
    }

    /// Bulyan aggregation
    fn bulyan(&mut self) -> Vec<f64> {
        // First run Krum to select subset
        let n = self.pending_updates.len();
        let m = n.saturating_sub(2 * (n / 5)); // n - 2f where f = n/5

        // Select using multi-krum
        let _ = self.krum(m);

        // Then apply trimmed mean on selected
        self.trimmed_mean()
    }
}
