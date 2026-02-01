//! # Selection Strategies
//!
//! Year 3 EVOLUTION - Advanced selection mechanisms for genetic algorithms

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{Fitness, IndividualId};

// ============================================================================
// SELECTION TRAIT
// ============================================================================

/// Selection strategy trait
pub trait SelectionStrategy: Send + Sync {
    /// Select individuals for reproduction
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate>;

    /// Strategy name
    fn name(&self) -> &str;

    /// Get parameters
    fn params(&self) -> SelectionParams;
}

/// Selection candidate
#[derive(Debug, Clone)]
pub struct SelectionCandidate {
    /// ID
    pub id: IndividualId,
    /// Fitness
    pub fitness: Fitness,
    /// Rank (for rank-based selection)
    pub rank: Option<u32>,
    /// Crowding distance (for NSGA-II)
    pub crowding_distance: Option<f64>,
    /// Species ID
    pub species: Option<u64>,
}

/// Selection parameters
#[derive(Debug, Clone, Default)]
pub struct SelectionParams {
    /// Pressure (for tournament, etc.)
    pub pressure: f64,
    /// Elite count
    pub elite_count: usize,
    /// Tournament size
    pub tournament_size: usize,
    /// Sigma (for sigma scaling)
    pub sigma: f64,
}

// ============================================================================
// TOURNAMENT SELECTION
// ============================================================================

/// Tournament selection
pub struct TournamentSelection {
    /// Tournament size
    tournament_size: usize,
    /// Random state
    random_state: AtomicU64,
}

impl TournamentSelection {
    pub fn new(tournament_size: usize) -> Self {
        Self {
            tournament_size,
            random_state: AtomicU64::new(0xdeadbeef),
        }
    }

    fn random(&self, max: usize) -> usize {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as usize) % max
    }
}

impl SelectionStrategy for TournamentSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        let mut selected = Vec::with_capacity(count);

        for _ in 0..count {
            let mut best: Option<&SelectionCandidate> = None;

            for _ in 0..self.tournament_size {
                let idx = self.random(population.len());
                let candidate = &population[idx];

                match best {
                    None => best = Some(candidate),
                    Some(b) if candidate.fitness.score > b.fitness.score => best = Some(candidate),
                    _ => {},
                }
            }

            if let Some(winner) = best {
                selected.push(winner);
            }
        }

        selected
    }

    fn name(&self) -> &str {
        "Tournament"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams {
            tournament_size: self.tournament_size,
            ..Default::default()
        }
    }
}

// ============================================================================
// ROULETTE WHEEL SELECTION
// ============================================================================

/// Roulette wheel (fitness proportionate) selection
pub struct RouletteWheelSelection {
    random_state: AtomicU64,
}

impl RouletteWheelSelection {
    pub fn new() -> Self {
        Self {
            random_state: AtomicU64::new(0xcafebabe),
        }
    }

    fn random_f64(&self) -> f64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as f64) / (u64::MAX as f64)
    }
}

impl Default for RouletteWheelSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStrategy for RouletteWheelSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() {
            return Vec::new();
        }

        // Calculate total fitness (shift to positive if needed)
        let min_fitness = population
            .iter()
            .map(|c| c.fitness.score)
            .fold(f64::INFINITY, f64::min);

        let shift = if min_fitness < 0.0 {
            -min_fitness + 1.0
        } else {
            0.0
        };

        let total: f64 = population.iter().map(|c| c.fitness.score + shift).sum();

        if total == 0.0 {
            // All equal, random selection
            let mut selected = Vec::with_capacity(count);
            for i in 0..count {
                selected.push(&population[i % population.len()]);
            }
            return selected;
        }

        // Build cumulative probabilities
        let mut cumulative = Vec::with_capacity(population.len());
        let mut sum = 0.0;
        for c in population {
            sum += (c.fitness.score + shift) / total;
            cumulative.push(sum);
        }

        // Select
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let r = self.random_f64();

            for (i, &cum) in cumulative.iter().enumerate() {
                if r <= cum {
                    selected.push(&population[i]);
                    break;
                }
            }
        }

        selected
    }

    fn name(&self) -> &str {
        "RouletteWheel"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams::default()
    }
}

// ============================================================================
// RANK SELECTION
// ============================================================================

/// Rank-based selection
pub struct RankSelection {
    /// Selection pressure (1.0 to 2.0)
    pressure: f64,
    random_state: AtomicU64,
}

impl RankSelection {
    pub fn new(pressure: f64) -> Self {
        Self {
            pressure: pressure.clamp(1.0, 2.0),
            random_state: AtomicU64::new(0xfeedface),
        }
    }

    fn random_f64(&self) -> f64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as f64) / (u64::MAX as f64)
    }
}

impl SelectionStrategy for RankSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() {
            return Vec::new();
        }

        // Sort by fitness
        let mut sorted: Vec<_> = population.iter().enumerate().collect();
        sorted.sort_by(|(_, a), (_, b)| a.fitness.score.partial_cmp(&b.fitness.score).unwrap());

        let n = sorted.len() as f64;

        // Calculate rank-based probabilities
        let mut probabilities = Vec::with_capacity(population.len());
        let mut prob_sum = 0.0;

        for (rank, _) in sorted.iter().enumerate() {
            let r = rank as f64;
            let p = (2.0 - self.pressure) / n + 2.0 * r * (self.pressure - 1.0) / (n * (n - 1.0));
            prob_sum += p;
            probabilities.push(prob_sum);
        }

        // Select
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let r = self.random_f64() * prob_sum;

            for (i, &cum) in probabilities.iter().enumerate() {
                if r <= cum {
                    selected.push(sorted[i].1);
                    break;
                }
            }
        }

        selected
    }

    fn name(&self) -> &str {
        "Rank"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams {
            pressure: self.pressure,
            ..Default::default()
        }
    }
}

// ============================================================================
// STOCHASTIC UNIVERSAL SAMPLING
// ============================================================================

/// Stochastic Universal Sampling (SUS)
pub struct StochasticUniversalSampling {
    random_state: AtomicU64,
}

impl StochasticUniversalSampling {
    pub fn new() -> Self {
        Self {
            random_state: AtomicU64::new(0x12345678),
        }
    }

    fn random_f64(&self) -> f64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as f64) / (u64::MAX as f64)
    }
}

impl Default for StochasticUniversalSampling {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStrategy for StochasticUniversalSampling {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() || count == 0 {
            return Vec::new();
        }

        // Calculate total fitness
        let min_fitness = population
            .iter()
            .map(|c| c.fitness.score)
            .fold(f64::INFINITY, f64::min);

        let shift = if min_fitness < 0.0 {
            -min_fitness + 1.0
        } else {
            0.0
        };

        let total: f64 = population.iter().map(|c| c.fitness.score + shift).sum();

        if total == 0.0 {
            return population.iter().take(count).collect();
        }

        let step = total / count as f64;
        let start = self.random_f64() * step;

        let mut selected = Vec::with_capacity(count);
        let mut pointer = start;
        let mut sum = 0.0;
        let mut idx = 0;

        for _ in 0..count {
            while sum + (population[idx].fitness.score + shift) < pointer {
                sum += population[idx].fitness.score + shift;
                idx = (idx + 1) % population.len();
            }
            selected.push(&population[idx]);
            pointer += step;
        }

        selected
    }

    fn name(&self) -> &str {
        "SUS"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams::default()
    }
}

// ============================================================================
// TRUNCATION SELECTION
// ============================================================================

/// Truncation selection (select top N%)
pub struct TruncationSelection {
    /// Truncation threshold (0.0 to 1.0)
    threshold: f64,
}

impl TruncationSelection {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.1, 1.0),
        }
    }
}

impl SelectionStrategy for TruncationSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() {
            return Vec::new();
        }

        // Sort by fitness descending
        let mut sorted: Vec<_> = population.iter().collect();
        sorted.sort_by(|a, b| b.fitness.score.partial_cmp(&a.fitness.score).unwrap());

        // Select from top portion
        let cutoff = ((population.len() as f64) * self.threshold) as usize;
        let cutoff = cutoff.max(1);

        let mut selected = Vec::with_capacity(count);
        for i in 0..count {
            selected.push(sorted[i % cutoff]);
        }

        selected
    }

    fn name(&self) -> &str {
        "Truncation"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams {
            pressure: self.threshold,
            ..Default::default()
        }
    }
}

// ============================================================================
// BOLTZMANN SELECTION
// ============================================================================

/// Boltzmann (temperature-based) selection
pub struct BoltzmannSelection {
    /// Temperature (decreases over time)
    temperature: f64,
    /// Cooling rate
    cooling_rate: f64,
    random_state: AtomicU64,
}

impl BoltzmannSelection {
    pub fn new(initial_temp: f64, cooling_rate: f64) -> Self {
        Self {
            temperature: initial_temp,
            cooling_rate: cooling_rate.clamp(0.9, 0.9999),
            random_state: AtomicU64::new(0xabcdef01),
        }
    }

    fn random_f64(&self) -> f64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as f64) / (u64::MAX as f64)
    }

    /// Cool down the temperature
    pub fn cool(&mut self) {
        self.temperature *= self.cooling_rate;
    }

    /// Set temperature
    pub fn set_temperature(&mut self, temp: f64) {
        self.temperature = temp;
    }
}

impl SelectionStrategy for BoltzmannSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() {
            return Vec::new();
        }

        // Calculate Boltzmann probabilities
        let exp_values: Vec<f64> = population
            .iter()
            .map(|c| (c.fitness.score / self.temperature).exp())
            .collect();

        let total: f64 = exp_values.iter().sum();

        if total == 0.0 || !total.is_finite() {
            return population.iter().take(count).collect();
        }

        // Build cumulative probabilities
        let mut cumulative = Vec::with_capacity(population.len());
        let mut sum = 0.0;
        for exp_val in &exp_values {
            sum += exp_val / total;
            cumulative.push(sum);
        }

        // Select
        let mut selected = Vec::with_capacity(count);
        for _ in 0..count {
            let r = self.random_f64();

            for (i, &cum) in cumulative.iter().enumerate() {
                if r <= cum {
                    selected.push(&population[i]);
                    break;
                }
            }
        }

        selected
    }

    fn name(&self) -> &str {
        "Boltzmann"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams {
            pressure: self.temperature,
            ..Default::default()
        }
    }
}

// ============================================================================
// NSGA-II SELECTION
// ============================================================================

/// NSGA-II selection (non-dominated sorting with crowding distance)
pub struct NSGA2Selection {
    random_state: AtomicU64,
}

impl NSGA2Selection {
    pub fn new() -> Self {
        Self {
            random_state: AtomicU64::new(0x87654321),
        }
    }

    fn random(&self, max: usize) -> usize {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as usize) % max
    }

    /// Compare by rank then crowding distance
    fn dominates(a: &SelectionCandidate, b: &SelectionCandidate) -> bool {
        match (a.rank, b.rank) {
            (Some(ra), Some(rb)) if ra < rb => true,
            (Some(ra), Some(rb)) if ra == rb => match (a.crowding_distance, b.crowding_distance) {
                (Some(da), Some(db)) => da > db,
                _ => false,
            },
            _ => false,
        }
    }
}

impl Default for NSGA2Selection {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStrategy for NSGA2Selection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        let mut selected = Vec::with_capacity(count);

        // Binary tournament with NSGA-II criteria
        for _ in 0..count {
            let i = self.random(population.len());
            let j = self.random(population.len());

            let winner = if Self::dominates(&population[i], &population[j]) {
                &population[i]
            } else if Self::dominates(&population[j], &population[i]) {
                &population[j]
            } else {
                // Equal, random choice
                if self.random(2) == 0 {
                    &population[i]
                } else {
                    &population[j]
                }
            };

            selected.push(winner);
        }

        selected
    }

    fn name(&self) -> &str {
        "NSGA-II"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams {
            tournament_size: 2,
            ..Default::default()
        }
    }
}

// ============================================================================
// LEXICASE SELECTION
// ============================================================================

/// Lexicase selection (for multi-case problems)
pub struct LexicaseSelection {
    random_state: AtomicU64,
}

impl LexicaseSelection {
    pub fn new() -> Self {
        Self {
            random_state: AtomicU64::new(0x11223344),
        }
    }

    fn random(&self, max: usize) -> usize {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as usize) % max
    }

    fn shuffle(&self, indices: &mut Vec<usize>) {
        for i in (1..indices.len()).rev() {
            let j = self.random(i + 1);
            indices.swap(i, j);
        }
    }
}

impl Default for LexicaseSelection {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionStrategy for LexicaseSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() {
            return Vec::new();
        }

        let mut selected = Vec::with_capacity(count);

        for _ in 0..count {
            // Get candidates (initially all)
            let mut candidates: Vec<usize> = (0..population.len()).collect();

            // Get cases in random order
            let num_cases = population
                .first()
                .and_then(|c| {
                    // Use objectives as cases
                    Some(c.fitness.objectives.len())
                })
                .unwrap_or(1);

            let mut cases: Vec<usize> = (0..num_cases).collect();
            self.shuffle(&mut cases);

            // Filter by each case
            for case_idx in cases {
                if candidates.len() <= 1 {
                    break;
                }

                // Find best on this case
                let best_on_case = candidates
                    .iter()
                    .map(|&i| {
                        let obj = population[i]
                            .fitness
                            .objectives
                            .get(case_idx)
                            .copied()
                            .unwrap_or(0.0);
                        (i, obj)
                    })
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(i, _)| i);

                if let Some(best_idx) = best_on_case {
                    let best_val = population[best_idx]
                        .fitness
                        .objectives
                        .get(case_idx)
                        .copied()
                        .unwrap_or(0.0);

                    // Keep only those with best value on this case
                    candidates.retain(|&i| {
                        let val = population[i]
                            .fitness
                            .objectives
                            .get(case_idx)
                            .copied()
                            .unwrap_or(0.0);
                        (val - best_val).abs() < 1e-9
                    });
                }
            }

            // Select randomly from remaining candidates
            let winner_idx = candidates[self.random(candidates.len())];
            selected.push(&population[winner_idx]);
        }

        selected
    }

    fn name(&self) -> &str {
        "Lexicase"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams::default()
    }
}

// ============================================================================
// SELECTION COMBINATOR
// ============================================================================

/// Combined selection (elite + main selection)
pub struct ElitistSelection {
    /// Elite count
    elite_count: usize,
    /// Main selection strategy
    main_strategy: Box<dyn SelectionStrategy>,
}

impl ElitistSelection {
    pub fn new(elite_count: usize, main: Box<dyn SelectionStrategy>) -> Self {
        Self {
            elite_count,
            main_strategy: main,
        }
    }
}

impl SelectionStrategy for ElitistSelection {
    fn select<'a>(
        &self,
        population: &'a [SelectionCandidate],
        count: usize,
    ) -> Vec<&'a SelectionCandidate> {
        if population.is_empty() {
            return Vec::new();
        }

        // Sort by fitness descending
        let mut sorted: Vec<_> = population.iter().collect();
        sorted.sort_by(|a, b| b.fitness.score.partial_cmp(&a.fitness.score).unwrap());

        let mut selected = Vec::with_capacity(count);

        // Add elites
        let elite_count = self.elite_count.min(count).min(sorted.len());
        for &elite in sorted.iter().take(elite_count) {
            selected.push(elite);
        }

        // Fill rest with main strategy
        if selected.len() < count {
            let remaining = count - selected.len();
            let more = self.main_strategy.select(population, remaining);
            selected.extend(more);
        }

        selected
    }

    fn name(&self) -> &str {
        "Elitist"
    }

    fn params(&self) -> SelectionParams {
        SelectionParams {
            elite_count: self.elite_count,
            ..self.main_strategy.params()
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_population() -> Vec<SelectionCandidate> {
        vec![
            SelectionCandidate {
                id: IndividualId(1),
                fitness: Fitness {
                    score: 10.0,
                    objectives: vec![10.0],
                    constraints: vec![],
                    components: BTreeMap::new(),
                    generation: 0,
                },
                rank: Some(1),
                crowding_distance: Some(1.0),
                species: None,
            },
            SelectionCandidate {
                id: IndividualId(2),
                fitness: Fitness {
                    score: 20.0,
                    objectives: vec![20.0],
                    constraints: vec![],
                    components: BTreeMap::new(),
                    generation: 0,
                },
                rank: Some(1),
                crowding_distance: Some(2.0),
                species: None,
            },
            SelectionCandidate {
                id: IndividualId(3),
                fitness: Fitness {
                    score: 30.0,
                    objectives: vec![30.0],
                    constraints: vec![],
                    components: BTreeMap::new(),
                    generation: 0,
                },
                rank: Some(2),
                crowding_distance: Some(0.5),
                species: None,
            },
        ]
    }

    #[test]
    fn test_tournament() {
        let selection = TournamentSelection::new(2);
        let pop = create_population();
        let selected = selection.select(&pop, 2);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_roulette() {
        let selection = RouletteWheelSelection::new();
        let pop = create_population();
        let selected = selection.select(&pop, 2);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn test_truncation() {
        let selection = TruncationSelection::new(0.5);
        let pop = create_population();
        let selected = selection.select(&pop, 3);
        assert_eq!(selected.len(), 3);
        // Should select from top 50%
    }

    #[test]
    fn test_nsga2() {
        let selection = NSGA2Selection::new();
        let pop = create_population();
        let selected = selection.select(&pop, 2);
        assert_eq!(selected.len(), 2);
    }
}
