//! # Multi-Objective Optimization
//!
//! Year 3 EVOLUTION - Multi-objective genetic algorithms

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::operators::Rng;
use super::{Fitness, IndividualId};

// ============================================================================
// OBJECTIVE TYPES
// ============================================================================

/// Objective ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectiveId(pub u64);

static OBJECTIVE_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ObjectiveId {
    pub fn generate() -> Self {
        Self(OBJECTIVE_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Objective definition
#[derive(Debug, Clone)]
pub struct Objective {
    /// ID
    pub id: ObjectiveId,
    /// Name
    pub name: String,
    /// Direction
    pub direction: OptDirection,
    /// Weight (for weighted sum)
    pub weight: f64,
    /// Reference point (for hypervolume)
    pub reference: Option<f64>,
    /// Ideal point
    pub ideal: Option<f64>,
    /// Nadir point
    pub nadir: Option<f64>,
}

/// Optimization direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptDirection {
    Minimize,
    Maximize,
}

/// Solution in objective space
#[derive(Debug, Clone)]
pub struct Solution {
    /// ID
    pub id: IndividualId,
    /// Objective values
    pub objectives: Vec<f64>,
    /// Constraint violations
    pub constraints: Vec<f64>,
    /// Pareto rank
    pub rank: u32,
    /// Crowding distance
    pub crowding_distance: f64,
    /// Reference point contribution
    pub contribution: f64,
}

impl Solution {
    pub fn new(id: IndividualId, objectives: Vec<f64>) -> Self {
        Self {
            id,
            objectives,
            constraints: Vec::new(),
            rank: 0,
            crowding_distance: 0.0,
            contribution: 0.0,
        }
    }

    /// Check if feasible (all constraints satisfied)
    pub fn is_feasible(&self) -> bool {
        self.constraints.iter().all(|&c| c <= 0.0)
    }

    /// Sum of constraint violations
    pub fn total_violation(&self) -> f64 {
        self.constraints.iter().filter(|&&c| c > 0.0).sum()
    }
}

// ============================================================================
// DOMINANCE
// ============================================================================

/// Dominance relation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dominance {
    /// a dominates b
    Dominates,
    /// a is dominated by b
    Dominated,
    /// Neither dominates
    NonDominated,
}

/// Check dominance between two solutions
pub fn check_dominance(a: &Solution, b: &Solution, directions: &[OptDirection]) -> Dominance {
    if a.objectives.len() != b.objectives.len() || a.objectives.len() != directions.len() {
        return Dominance::NonDominated;
    }

    let mut a_better = false;
    let mut b_better = false;

    for i in 0..a.objectives.len() {
        let cmp = match directions[i] {
            OptDirection::Minimize => a.objectives[i].partial_cmp(&b.objectives[i]),
            OptDirection::Maximize => b.objectives[i].partial_cmp(&a.objectives[i]),
        };

        match cmp {
            Some(core::cmp::Ordering::Less) => a_better = true,
            Some(core::cmp::Ordering::Greater) => b_better = true,
            _ => {},
        }
    }

    if a_better && !b_better {
        Dominance::Dominates
    } else if b_better && !a_better {
        Dominance::Dominated
    } else {
        Dominance::NonDominated
    }
}

/// Check weak dominance (<=)
pub fn weakly_dominates(a: &Solution, b: &Solution, directions: &[OptDirection]) -> bool {
    for i in 0..a.objectives.len() {
        let better = match directions[i] {
            OptDirection::Minimize => a.objectives[i] <= b.objectives[i],
            OptDirection::Maximize => a.objectives[i] >= b.objectives[i],
        };
        if !better {
            return false;
        }
    }
    true
}

// ============================================================================
// NON-DOMINATED SORTING
// ============================================================================

/// Non-dominated sorting
pub struct NonDominatedSorting {
    directions: Vec<OptDirection>,
}

impl NonDominatedSorting {
    pub fn new(directions: Vec<OptDirection>) -> Self {
        Self { directions }
    }

    /// Perform non-dominated sorting
    pub fn sort(&self, solutions: &mut [Solution]) -> Vec<Vec<usize>> {
        let n = solutions.len();
        if n == 0 {
            return Vec::new();
        }

        // Domination counts and dominated sets
        let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut domination_count: Vec<usize> = vec![0; n];

        // Compare all pairs
        for i in 0..n {
            for j in (i + 1)..n {
                match check_dominance(&solutions[i], &solutions[j], &self.directions) {
                    Dominance::Dominates => {
                        dominated_by[i].push(j);
                        domination_count[j] += 1;
                    },
                    Dominance::Dominated => {
                        dominated_by[j].push(i);
                        domination_count[i] += 1;
                    },
                    Dominance::NonDominated => {},
                }
            }
        }

        // Build fronts
        let mut fronts: Vec<Vec<usize>> = Vec::new();
        let mut current_front: Vec<usize> = Vec::new();

        // First front (not dominated by anyone)
        for i in 0..n {
            if domination_count[i] == 0 {
                current_front.push(i);
                solutions[i].rank = 0;
            }
        }

        let mut front_idx = 0;
        while !current_front.is_empty() {
            fronts.push(current_front.clone());

            let mut next_front: Vec<usize> = Vec::new();

            for &i in &current_front {
                for &j in &dominated_by[i] {
                    domination_count[j] -= 1;
                    if domination_count[j] == 0 {
                        next_front.push(j);
                        solutions[j].rank = front_idx + 1;
                    }
                }
            }

            current_front = next_front;
            front_idx += 1;
        }

        fronts
    }

    /// Get Pareto front (first front)
    pub fn pareto_front(&self, solutions: &mut [Solution]) -> Vec<usize> {
        let fronts = self.sort(solutions);
        fronts.into_iter().next().unwrap_or_default()
    }
}

// ============================================================================
// CROWDING DISTANCE
// ============================================================================

/// Calculate crowding distances
pub fn calculate_crowding_distance(solutions: &mut [Solution], front: &[usize]) {
    let n = front.len();
    if n <= 2 {
        for &i in front {
            solutions[i].crowding_distance = f64::INFINITY;
        }
        return;
    }

    // Initialize
    for &i in front {
        solutions[i].crowding_distance = 0.0;
    }

    let num_objectives = solutions[front[0]].objectives.len();

    for m in 0..num_objectives {
        // Sort by objective m
        let mut sorted_front: Vec<usize> = front.to_vec();
        sorted_front.sort_by(|&a, &b| {
            solutions[a].objectives[m]
                .partial_cmp(&solutions[b].objectives[m])
                .unwrap()
        });

        // Boundary points get infinite distance
        let first = sorted_front[0];
        let last = sorted_front[n - 1];
        solutions[first].crowding_distance = f64::INFINITY;
        solutions[last].crowding_distance = f64::INFINITY;

        // Calculate range
        let range = solutions[last].objectives[m] - solutions[first].objectives[m];
        if range < 1e-10 {
            continue;
        }

        // Interior points
        for i in 1..(n - 1) {
            let prev = sorted_front[i - 1];
            let next = sorted_front[i + 1];
            let curr = sorted_front[i];

            let distance = (solutions[next].objectives[m] - solutions[prev].objectives[m]) / range;
            solutions[curr].crowding_distance += distance;
        }
    }
}

// ============================================================================
// HYPERVOLUME
// ============================================================================

/// Hypervolume calculator
pub struct HypervolumeCalculator {
    /// Reference point
    reference: Vec<f64>,
    /// Directions
    directions: Vec<OptDirection>,
}

impl HypervolumeCalculator {
    pub fn new(reference: Vec<f64>, directions: Vec<OptDirection>) -> Self {
        Self {
            reference,
            directions,
        }
    }

    /// Calculate hypervolume for a set of solutions
    pub fn calculate(&self, solutions: &[Solution]) -> f64 {
        if solutions.is_empty() {
            return 0.0;
        }

        let n_obj = self.reference.len();

        // Transform objectives (all to minimization, relative to reference)
        let transformed: Vec<Vec<f64>> = solutions
            .iter()
            .map(|s| {
                s.objectives
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| match self.directions[i] {
                        OptDirection::Minimize => self.reference[i] - v,
                        OptDirection::Maximize => v - self.reference[i],
                    })
                    .collect()
            })
            .collect();

        // Filter dominated and negative contribution points
        let valid: Vec<_> = transformed
            .iter()
            .filter(|p| p.iter().all(|&v| v > 0.0))
            .collect();

        if valid.is_empty() {
            return 0.0;
        }

        // Use inclusion-exclusion for 2D
        if n_obj == 2 {
            return self.hypervolume_2d(&valid);
        }

        // For higher dimensions, use approximation
        self.hypervolume_monte_carlo(&valid, 10000)
    }

    fn hypervolume_2d(&self, points: &[&Vec<f64>]) -> f64 {
        if points.is_empty() {
            return 0.0;
        }

        // Sort by first objective
        let mut sorted: Vec<_> = points.to_vec();
        sorted.sort_by(|a, b| b[0].partial_cmp(&a[0]).unwrap());

        let mut volume = 0.0;
        let mut prev_y = 0.0;

        for p in sorted {
            if p[1] > prev_y {
                volume += p[0] * (p[1] - prev_y);
                prev_y = p[1];
            }
        }

        volume
    }

    fn hypervolume_monte_carlo(&self, points: &[&Vec<f64>], samples: usize) -> f64 {
        if points.is_empty() {
            return 0.0;
        }

        let n_obj = points[0].len();
        let rng = Rng::default();

        // Find bounding box
        let mut max_point = vec![0.0f64; n_obj];
        for p in points {
            for (i, &v) in p.iter().enumerate() {
                max_point[i] = max_point[i].max(v);
            }
        }

        // Calculate bounding volume
        let bound_volume: f64 = max_point.iter().product();

        // Monte Carlo sampling
        let mut count = 0;
        for _ in 0..samples {
            // Random point in bounding box
            let sample: Vec<f64> = (0..n_obj).map(|i| rng.next_f64() * max_point[i]).collect();

            // Check if dominated by any point
            let dominated = points
                .iter()
                .any(|p| p.iter().zip(sample.iter()).all(|(&pi, &si)| pi >= si));

            if dominated {
                count += 1;
            }
        }

        bound_volume * (count as f64) / (samples as f64)
    }

    /// Calculate contribution of each solution
    pub fn calculate_contributions(&self, solutions: &mut [Solution]) {
        if solutions.is_empty() {
            return;
        }

        let total = self.calculate(solutions);

        for i in 0..solutions.len() {
            // Hypervolume without this solution
            let without: Vec<Solution> = solutions
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, s)| s.clone())
                .collect();

            let hv_without = self.calculate(&without);
            solutions[i].contribution = total - hv_without;
        }
    }
}

// ============================================================================
// REFERENCE POINT METHODS
// ============================================================================

/// Reference point based methods (NSGA-III style)
pub struct ReferencePoints {
    /// Reference points
    points: Vec<Vec<f64>>,
    /// Number of objectives
    n_objectives: usize,
    /// Divisions per objective
    divisions: usize,
}

impl ReferencePoints {
    /// Generate uniformly distributed reference points
    pub fn uniform(n_objectives: usize, divisions: usize) -> Self {
        let mut points = Vec::new();
        Self::generate_recursive(
            n_objectives,
            divisions,
            divisions as f64,
            &mut vec![0.0; n_objectives],
            0,
            &mut points,
        );

        Self {
            points,
            n_objectives,
            divisions,
        }
    }

    fn generate_recursive(
        n_obj: usize,
        divisions: usize,
        remaining: f64,
        current: &mut Vec<f64>,
        idx: usize,
        result: &mut Vec<Vec<f64>>,
    ) {
        if idx == n_obj - 1 {
            current[idx] = remaining / divisions as f64;
            result.push(current.clone());
            return;
        }

        for i in 0..=(remaining as usize) {
            current[idx] = i as f64 / divisions as f64;
            Self::generate_recursive(
                n_obj,
                divisions,
                remaining - i as f64,
                current,
                idx + 1,
                result,
            );
        }
    }

    /// Get reference points
    pub fn points(&self) -> &[Vec<f64>] {
        &self.points
    }

    /// Associate solutions with reference points
    pub fn associate(&self, solutions: &[Solution]) -> Vec<(usize, usize, f64)> {
        let mut associations = Vec::with_capacity(solutions.len());

        for (sol_idx, solution) in solutions.iter().enumerate() {
            // Normalize objectives
            let normalized = self.normalize(&solution.objectives);

            // Find closest reference point
            let mut min_dist = f64::INFINITY;
            let mut closest = 0;

            for (ref_idx, ref_point) in self.points.iter().enumerate() {
                let dist = self.perpendicular_distance(&normalized, ref_point);
                if dist < min_dist {
                    min_dist = dist;
                    closest = ref_idx;
                }
            }

            associations.push((sol_idx, closest, min_dist));
        }

        associations
    }

    fn normalize(&self, objectives: &[f64]) -> Vec<f64> {
        // Simple normalization (would use ideal/nadir in practice)
        let sum: f64 = objectives.iter().sum();
        if sum > 0.0 {
            objectives.iter().map(|&v| v / sum).collect()
        } else {
            vec![1.0 / self.n_objectives as f64; self.n_objectives]
        }
    }

    fn perpendicular_distance(&self, point: &[f64], reference: &[f64]) -> f64 {
        // Project point onto reference direction
        let dot: f64 = point.iter().zip(reference.iter()).map(|(p, r)| p * r).sum();
        let ref_norm: f64 = reference.iter().map(|r| r * r).sum::<f64>().sqrt();

        if ref_norm < 1e-10 {
            return f64::INFINITY;
        }

        let projection: Vec<f64> = reference
            .iter()
            .map(|r| (dot / (ref_norm * ref_norm)) * r)
            .collect();

        // Distance from point to projection
        point
            .iter()
            .zip(projection.iter())
            .map(|(p, proj)| (p - proj).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Niching (for NSGA-III selection)
    pub fn niche(&self, solutions: &mut [Solution], to_select: usize) -> Vec<usize> {
        let associations = self.associate(solutions);

        // Count per reference point
        let mut niche_counts: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
        for (sol_idx, ref_idx, dist) in associations {
            niche_counts
                .entry(ref_idx)
                .or_default()
                .push((sol_idx, dist));
        }

        let mut selected = Vec::new();
        let mut rng = Rng::default();

        while selected.len() < to_select.min(solutions.len()) {
            // Find reference point with minimum count
            let min_count = niche_counts
                .values()
                .filter(|v| !v.is_empty())
                .map(|v| v.len())
                .min();

            if min_count.is_none() {
                break;
            }

            // Get all reference points with minimum count
            let candidates: Vec<usize> = niche_counts
                .iter()
                .filter(|(_, v)| !v.is_empty() && v.len() == min_count.unwrap())
                .map(|(&k, _)| k)
                .collect();

            if candidates.is_empty() {
                break;
            }

            // Random reference point
            let ref_idx = candidates[rng.next_usize(candidates.len())];

            // Get solution with minimum distance to this reference
            if let Some(members) = niche_counts.get_mut(&ref_idx) {
                if !members.is_empty() {
                    members.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    let (sol_idx, _) = members.remove(0);
                    selected.push(sol_idx);
                }
            }
        }

        selected
    }
}

// ============================================================================
// INDICATOR-BASED SELECTION
// ============================================================================

/// Indicator-based selection (using hypervolume or epsilon indicator)
pub struct IndicatorSelection {
    /// Indicator type
    indicator: IndicatorType,
    /// Reference point
    reference: Vec<f64>,
    /// Directions
    directions: Vec<OptDirection>,
}

/// Indicator type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorType {
    Hypervolume,
    Epsilon,
    R2,
}

impl IndicatorSelection {
    pub fn new(
        indicator: IndicatorType,
        reference: Vec<f64>,
        directions: Vec<OptDirection>,
    ) -> Self {
        Self {
            indicator,
            reference,
            directions,
        }
    }

    /// Select solutions using indicator
    pub fn select(&self, solutions: &mut [Solution], count: usize) -> Vec<usize> {
        match self.indicator {
            IndicatorType::Hypervolume => self.select_hypervolume(solutions, count),
            IndicatorType::Epsilon => self.select_epsilon(solutions, count),
            IndicatorType::R2 => self.select_r2(solutions, count),
        }
    }

    fn select_hypervolume(&self, solutions: &mut [Solution], count: usize) -> Vec<usize> {
        let calc = HypervolumeCalculator::new(self.reference.clone(), self.directions.clone());

        let mut remaining: Vec<usize> = (0..solutions.len()).collect();
        let mut selected = Vec::new();

        // Iteratively remove solution with smallest contribution
        while remaining.len() > count {
            // Get solutions in remaining
            let subset: Vec<Solution> = remaining.iter().map(|&i| solutions[i].clone()).collect();

            // Calculate contributions
            let mut subset_mut = subset;
            calc.calculate_contributions(&mut subset_mut);

            // Find minimum contribution
            let min_idx = subset_mut
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.contribution.partial_cmp(&b.1.contribution).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);

            remaining.remove(min_idx);
        }

        remaining
    }

    fn select_epsilon(&self, solutions: &mut [Solution], count: usize) -> Vec<usize> {
        // Epsilon indicator selection
        let mut remaining: Vec<usize> = (0..solutions.len()).collect();

        while remaining.len() > count {
            // Calculate epsilon indicator for each solution
            let mut min_indicator = f64::INFINITY;
            let mut min_idx = 0;

            for (i, &sol_i) in remaining.iter().enumerate() {
                let mut max_eps = f64::NEG_INFINITY;

                for &sol_j in &remaining {
                    if sol_i == sol_j {
                        continue;
                    }

                    // Epsilon to dominate sol_j
                    let eps = self.epsilon_indicator(&solutions[sol_i], &solutions[sol_j]);
                    max_eps = max_eps.max(eps);
                }

                if max_eps < min_indicator {
                    min_indicator = max_eps;
                    min_idx = i;
                }
            }

            remaining.remove(min_idx);
        }

        remaining
    }

    fn epsilon_indicator(&self, a: &Solution, b: &Solution) -> f64 {
        a.objectives
            .iter()
            .zip(b.objectives.iter())
            .zip(self.directions.iter())
            .map(|((&ai, &bi), &dir)| match dir {
                OptDirection::Minimize => bi - ai,
                OptDirection::Maximize => ai - bi,
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn select_r2(&self, solutions: &mut [Solution], count: usize) -> Vec<usize> {
        // Simplified R2 selection
        let mut remaining: Vec<usize> = (0..solutions.len()).collect();

        // Sort by sum of normalized objectives
        remaining.sort_by(|&a, &b| {
            let sum_a: f64 = solutions[a].objectives.iter().sum();
            let sum_b: f64 = solutions[b].objectives.iter().sum();
            sum_a.partial_cmp(&sum_b).unwrap()
        });

        remaining.truncate(count);
        remaining
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dominance() {
        let a = Solution::new(IndividualId(1), vec![1.0, 2.0]);
        let b = Solution::new(IndividualId(2), vec![2.0, 3.0]);
        let dirs = vec![OptDirection::Minimize, OptDirection::Minimize];

        assert_eq!(check_dominance(&a, &b, &dirs), Dominance::Dominates);
    }

    #[test]
    fn test_non_dominated_sorting() {
        let directions = vec![OptDirection::Minimize, OptDirection::Minimize];
        let sorter = NonDominatedSorting::new(directions);

        let mut solutions = vec![
            Solution::new(IndividualId(1), vec![1.0, 4.0]),
            Solution::new(IndividualId(2), vec![2.0, 3.0]),
            Solution::new(IndividualId(3), vec![3.0, 2.0]),
            Solution::new(IndividualId(4), vec![4.0, 1.0]),
            Solution::new(IndividualId(5), vec![2.5, 2.5]),
        ];

        let fronts = sorter.sort(&mut solutions);

        // First front should have the non-dominated solutions
        assert!(!fronts.is_empty());
        assert_eq!(fronts[0].len(), 4); // All except (2.5, 2.5) are Pareto optimal
    }

    #[test]
    fn test_crowding_distance() {
        let mut solutions = vec![
            Solution::new(IndividualId(1), vec![1.0, 4.0]),
            Solution::new(IndividualId(2), vec![2.0, 3.0]),
            Solution::new(IndividualId(3), vec![3.0, 2.0]),
            Solution::new(IndividualId(4), vec![4.0, 1.0]),
        ];

        let front: Vec<usize> = (0..4).collect();
        calculate_crowding_distance(&mut solutions, &front);

        // Boundary points should have infinite distance
        assert!(solutions[0].crowding_distance.is_infinite());
        assert!(solutions[3].crowding_distance.is_infinite());
    }

    #[test]
    fn test_reference_points() {
        let ref_points = ReferencePoints::uniform(3, 4);

        // Number of points should be C(n+p-1, p) = C(6, 4) = 15
        assert_eq!(ref_points.points().len(), 15);
    }

    #[test]
    fn test_hypervolume_2d() {
        let reference = vec![4.0, 4.0];
        let directions = vec![OptDirection::Minimize, OptDirection::Minimize];
        let calc = HypervolumeCalculator::new(reference, directions);

        let solutions = vec![
            Solution::new(IndividualId(1), vec![1.0, 3.0]),
            Solution::new(IndividualId(2), vec![2.0, 2.0]),
            Solution::new(IndividualId(3), vec![3.0, 1.0]),
        ];

        let hv = calc.calculate(&solutions);
        assert!(hv > 0.0);
    }
}
