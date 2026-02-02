//! # Swarm Intelligence Types
//!
//! Core types for swarm intelligence algorithms.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// POSITION & VELOCITY
// ============================================================================

/// N-dimensional position vector
#[derive(Debug, Clone)]
pub struct Position {
    /// Coordinate values
    pub values: Vec<f64>,
}

impl Position {
    /// Create new position
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }

    /// Create zero position
    pub fn zeros(dim: usize) -> Self {
        Self {
            values: alloc::vec![0.0; dim],
        }
    }

    /// Create from bounds
    pub fn random_in_bounds(bounds: &[(f64, f64)], seed: u64) -> Self {
        let mut rng = seed;
        let mut values = Vec::with_capacity(bounds.len());

        for &(lo, hi) in bounds {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let t = (rng as f64) / (u64::MAX as f64);
            values.push(lo + t * (hi - lo));
        }

        Self { values }
    }

    /// Dimension
    pub fn dim(&self) -> usize {
        self.values.len()
    }

    /// Get value at index
    pub fn get(&self, i: usize) -> f64 {
        self.values.get(i).copied().unwrap_or(0.0)
    }

    /// Set value at index
    pub fn set(&mut self, i: usize, v: f64) {
        if i < self.values.len() {
            self.values[i] = v;
        }
    }

    /// Add another position
    pub fn add(&self, other: &Position) -> Position {
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(&a, &b)| a + b)
            .collect();
        Position { values }
    }

    /// Subtract another position
    pub fn sub(&self, other: &Position) -> Position {
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(&a, &b)| a - b)
            .collect();
        Position { values }
    }

    /// Scale by factor
    pub fn scale(&self, factor: f64) -> Position {
        Position {
            values: self.values.iter().map(|&v| v * factor).collect(),
        }
    }

    /// Distance to another position
    pub fn distance(&self, other: &Position) -> f64 {
        let sq_sum: f64 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(&a, &b)| (a - b) * (a - b))
            .sum();
        libm::sqrt(sq_sum)
    }

    /// Euclidean norm
    pub fn norm(&self) -> f64 {
        let sq_sum: f64 = self.values.iter().map(|&v| v * v).sum();
        libm::sqrt(sq_sum)
    }

    /// Clamp to bounds
    pub fn clamp(&mut self, bounds: &[(f64, f64)]) {
        for (i, &(lo, hi)) in bounds.iter().enumerate() {
            if i < self.values.len() {
                if self.values[i] < lo {
                    self.values[i] = lo;
                }
                if self.values[i] > hi {
                    self.values[i] = hi;
                }
            }
        }
    }
}

/// Velocity vector
#[derive(Debug, Clone)]
pub struct Velocity {
    /// Velocity components
    pub values: Vec<f64>,
}

impl Velocity {
    /// Create new velocity
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }

    /// Create zero velocity
    pub fn zeros(dim: usize) -> Self {
        Self {
            values: alloc::vec![0.0; dim],
        }
    }

    /// Dimension
    pub fn dim(&self) -> usize {
        self.values.len()
    }

    /// Add another velocity
    pub fn add(&self, other: &Velocity) -> Velocity {
        let values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(&a, &b)| a + b)
            .collect();
        Velocity { values }
    }

    /// Scale by factor
    pub fn scale(&self, factor: f64) -> Velocity {
        Velocity {
            values: self.values.iter().map(|&v| v * factor).collect(),
        }
    }

    /// Clamp magnitude
    pub fn clamp_magnitude(&mut self, max_speed: f64) {
        let speed: f64 = self.values.iter().map(|&v| v * v).sum();
        let speed = libm::sqrt(speed);

        if speed > max_speed && speed > 1e-10 {
            let factor = max_speed / speed;
            for v in &mut self.values {
                *v *= factor;
            }
        }
    }
}

// ============================================================================
// PARTICLE
// ============================================================================

/// Particle in swarm
#[derive(Debug, Clone)]
pub struct Particle {
    /// Current position
    pub position: Position,
    /// Current velocity
    pub velocity: Velocity,
    /// Personal best position
    pub best_position: Position,
    /// Personal best fitness
    pub best_fitness: f64,
    /// Current fitness
    pub fitness: f64,
}

impl Particle {
    /// Create new particle
    pub fn new(position: Position, velocity: Velocity) -> Self {
        let best_position = position.clone();
        Self {
            position,
            velocity,
            best_position,
            best_fitness: f64::MAX,
            fitness: f64::MAX,
        }
    }

    /// Create random particle within bounds
    pub fn random(bounds: &[(f64, f64)], seed: u64) -> Self {
        let position = Position::random_in_bounds(bounds, seed);
        let velocity = Velocity::zeros(bounds.len());
        Self::new(position, velocity)
    }

    /// Update personal best if current is better
    pub fn update_best(&mut self) {
        if self.fitness < self.best_fitness {
            self.best_fitness = self.fitness;
            self.best_position = self.position.clone();
        }
    }
}

// ============================================================================
// ANT
// ============================================================================

/// Ant for ant colony optimization
#[derive(Debug, Clone)]
pub struct Ant {
    /// Current path (node indices)
    pub path: Vec<usize>,
    /// Path length/cost
    pub path_cost: f64,
    /// Visited nodes set (bit flags for small graphs)
    visited: u64,
}

impl Ant {
    /// Create new ant starting at node
    pub fn new(start: usize) -> Self {
        let mut path = Vec::with_capacity(32);
        path.push(start);
        Self {
            path,
            path_cost: 0.0,
            visited: 1u64 << start,
        }
    }

    /// Check if node is visited
    pub fn is_visited(&self, node: usize) -> bool {
        if node < 64 {
            (self.visited >> node) & 1 == 1
        } else {
            self.path.contains(&node)
        }
    }

    /// Visit a node
    pub fn visit(&mut self, node: usize, cost: f64) {
        self.path.push(node);
        self.path_cost += cost;
        if node < 64 {
            self.visited |= 1u64 << node;
        }
    }

    /// Get path length
    pub fn path_len(&self) -> usize {
        self.path.len()
    }

    /// Reset ant for new iteration
    pub fn reset(&mut self, start: usize) {
        self.path.clear();
        self.path.push(start);
        self.path_cost = 0.0;
        self.visited = 1u64 << start;
    }
}

// ============================================================================
// BOID
// ============================================================================

/// Boid for flocking simulation
#[derive(Debug, Clone)]
pub struct Boid {
    /// Position in 2D or 3D space
    pub position: Position,
    /// Velocity
    pub velocity: Velocity,
    /// Acceleration accumulator
    pub acceleration: Velocity,
    /// Boid identifier
    pub id: usize,
}

impl Boid {
    /// Create new boid
    pub fn new(id: usize, position: Position, velocity: Velocity) -> Self {
        let dim = velocity.dim();
        Self {
            position,
            velocity,
            acceleration: Velocity::zeros(dim),
            id,
        }
    }

    /// Apply force
    pub fn apply_force(&mut self, force: &Velocity) {
        for (i, &f) in force.values.iter().enumerate() {
            if i < self.acceleration.values.len() {
                self.acceleration.values[i] += f;
            }
        }
    }

    /// Update position and velocity
    pub fn update(&mut self, max_speed: f64) {
        // Update velocity
        for (i, &a) in self.acceleration.values.iter().enumerate() {
            if i < self.velocity.values.len() {
                self.velocity.values[i] += a;
            }
        }

        // Clamp speed
        self.velocity.clamp_magnitude(max_speed);

        // Update position
        for (i, &v) in self.velocity.values.iter().enumerate() {
            if i < self.position.values.len() {
                self.position.values[i] += v;
            }
        }

        // Reset acceleration
        for a in &mut self.acceleration.values {
            *a = 0.0;
        }
    }
}

// ============================================================================
// PHEROMONE MATRIX
// ============================================================================

/// Pheromone matrix for ACO
#[derive(Debug, Clone)]
pub struct PheromoneMatrix {
    /// Number of nodes
    pub n_nodes: usize,
    /// Pheromone values (flattened n×n matrix)
    pub values: Vec<f64>,
    /// Initial pheromone level
    pub initial: f64,
}

impl PheromoneMatrix {
    /// Create new pheromone matrix
    pub fn new(n_nodes: usize, initial: f64) -> Self {
        Self {
            n_nodes,
            values: alloc::vec![initial; n_nodes * n_nodes],
            initial,
        }
    }

    /// Get pheromone level between nodes
    pub fn get(&self, from: usize, to: usize) -> f64 {
        if from < self.n_nodes && to < self.n_nodes {
            self.values[from * self.n_nodes + to]
        } else {
            self.initial
        }
    }

    /// Set pheromone level
    pub fn set(&mut self, from: usize, to: usize, value: f64) {
        if from < self.n_nodes && to < self.n_nodes {
            self.values[from * self.n_nodes + to] = value;
        }
    }

    /// Deposit pheromone
    pub fn deposit(&mut self, from: usize, to: usize, amount: f64) {
        if from < self.n_nodes && to < self.n_nodes {
            let idx = from * self.n_nodes + to;
            self.values[idx] += amount;
        }
    }

    /// Evaporate pheromones
    pub fn evaporate(&mut self, rate: f64) {
        let retention = 1.0 - rate;
        for v in &mut self.values {
            *v *= retention;
            // Ensure minimum pheromone
            if *v < self.initial * 0.01 {
                *v = self.initial * 0.01;
            }
        }
    }

    /// Reset all pheromones
    pub fn reset(&mut self) {
        for v in &mut self.values {
            *v = self.initial;
        }
    }
}

// ============================================================================
// STIGMERGY GRID
// ============================================================================

/// Stigmergy grid for emergent behavior
#[derive(Debug, Clone)]
pub struct StigmergyGrid {
    /// Grid width
    pub width: usize,
    /// Grid height
    pub height: usize,
    /// Signal values per cell
    pub signals: Vec<f64>,
    /// Decay rate
    pub decay_rate: f64,
}

impl StigmergyGrid {
    /// Create new grid
    pub fn new(width: usize, height: usize, decay_rate: f64) -> Self {
        Self {
            width,
            height,
            signals: alloc::vec![0.0; width * height],
            decay_rate,
        }
    }

    /// Get signal at position
    pub fn get(&self, x: usize, y: usize) -> f64 {
        if x < self.width && y < self.height {
            self.signals[y * self.width + x]
        } else {
            0.0
        }
    }

    /// Set signal at position
    pub fn set(&mut self, x: usize, y: usize, value: f64) {
        if x < self.width && y < self.height {
            self.signals[y * self.width + x] = value;
        }
    }

    /// Deposit signal at position
    pub fn deposit(&mut self, x: usize, y: usize, amount: f64) {
        if x < self.width && y < self.height {
            self.signals[y * self.width + x] += amount;
        }
    }

    /// Apply decay
    pub fn decay(&mut self) {
        let retention = 1.0 - self.decay_rate;
        for s in &mut self.signals {
            *s *= retention;
        }
    }

    /// Diffuse signals
    pub fn diffuse(&mut self, rate: f64) {
        let mut new_signals = self.signals.clone();

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let current = self.signals[idx];

                let mut neighbor_sum = 0.0;
                let mut count = 0;

                // Neighbors
                if x > 0 {
                    neighbor_sum += self.signals[idx - 1];
                    count += 1;
                }
                if x < self.width - 1 {
                    neighbor_sum += self.signals[idx + 1];
                    count += 1;
                }
                if y > 0 {
                    neighbor_sum += self.signals[idx - self.width];
                    count += 1;
                }
                if y < self.height - 1 {
                    neighbor_sum += self.signals[idx + self.width];
                    count += 1;
                }

                if count > 0 {
                    let avg = neighbor_sum / count as f64;
                    new_signals[idx] = current * (1.0 - rate) + avg * rate;
                }
            }
        }

        self.signals = new_signals;
    }

    /// Get gradient at position (direction of highest signal)
    pub fn gradient(&self, x: usize, y: usize) -> (f64, f64) {
        let mut gx = 0.0;
        let mut gy = 0.0;

        if x > 0 && x < self.width - 1 {
            let left = self.get(x - 1, y);
            let right = self.get(x + 1, y);
            gx = right - left;
        }

        if y > 0 && y < self.height - 1 {
            let up = self.get(x, y - 1);
            let down = self.get(x, y + 1);
            gy = down - up;
        }

        // Normalize
        let mag = libm::sqrt(gx * gx + gy * gy);
        if mag > 1e-10 {
            gx /= mag;
            gy /= mag;
        }

        (gx, gy)
    }
}

// ============================================================================
// SWARM CONFIG
// ============================================================================

/// PSO configuration
#[derive(Debug, Clone)]
pub struct PsoConfig {
    /// Number of particles
    pub n_particles: usize,
    /// Inertia weight
    pub inertia: f64,
    /// Cognitive coefficient (personal best attraction)
    pub c1: f64,
    /// Social coefficient (global best attraction)
    pub c2: f64,
    /// Maximum velocity
    pub max_velocity: f64,
    /// Maximum iterations
    pub max_iterations: usize,
}

impl Default for PsoConfig {
    fn default() -> Self {
        Self {
            n_particles: 30,
            inertia: 0.729,
            c1: 1.49445,
            c2: 1.49445,
            max_velocity: 1.0,
            max_iterations: 100,
        }
    }
}

/// ACO configuration
#[derive(Debug, Clone)]
pub struct AcoConfig {
    /// Number of ants
    pub n_ants: usize,
    /// Pheromone importance (α)
    pub alpha: f64,
    /// Heuristic importance (β)
    pub beta: f64,
    /// Evaporation rate (ρ)
    pub evaporation_rate: f64,
    /// Initial pheromone
    pub initial_pheromone: f64,
    /// Maximum iterations
    pub max_iterations: usize,
}

impl Default for AcoConfig {
    fn default() -> Self {
        Self {
            n_ants: 20,
            alpha: 1.0,
            beta: 2.0,
            evaporation_rate: 0.1,
            initial_pheromone: 0.1,
            max_iterations: 100,
        }
    }
}

/// Boids configuration
#[derive(Debug, Clone)]
pub struct BoidsConfig {
    /// Separation weight
    pub separation_weight: f64,
    /// Alignment weight
    pub alignment_weight: f64,
    /// Cohesion weight
    pub cohesion_weight: f64,
    /// Perception radius
    pub perception_radius: f64,
    /// Maximum speed
    pub max_speed: f64,
    /// Maximum force
    pub max_force: f64,
}

impl Default for BoidsConfig {
    fn default() -> Self {
        Self {
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            perception_radius: 50.0,
            max_speed: 4.0,
            max_force: 0.1,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position() {
        let p1 = Position::new(alloc::vec![1.0, 2.0, 3.0]);
        let p2 = Position::new(alloc::vec![4.0, 5.0, 6.0]);

        let sum = p1.add(&p2);
        assert_eq!(sum.values, [5.0, 7.0, 9.0]);

        let diff = p2.sub(&p1);
        assert_eq!(diff.values, [3.0, 3.0, 3.0]);

        let dist = p1.distance(&p2);
        assert!((dist - libm::sqrt(27.0)).abs() < 1e-10);
    }

    #[test]
    fn test_velocity_clamp() {
        let mut v = Velocity::new(alloc::vec![3.0, 4.0]);
        v.clamp_magnitude(2.5);

        let speed: f64 = v.values.iter().map(|&x| x * x).sum();
        let speed = libm::sqrt(speed);

        assert!((speed - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_particle() {
        let bounds = [(0.0, 10.0), (0.0, 10.0)];
        let p = Particle::random(&bounds, 12345);

        assert!(p.position.values[0] >= 0.0 && p.position.values[0] <= 10.0);
        assert!(p.position.values[1] >= 0.0 && p.position.values[1] <= 10.0);
    }

    #[test]
    fn test_ant() {
        let mut ant = Ant::new(0);

        assert!(ant.is_visited(0));
        assert!(!ant.is_visited(1));

        ant.visit(1, 5.0);
        assert!(ant.is_visited(1));
        assert_eq!(ant.path_len(), 2);
        assert!((ant.path_cost - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_pheromone_matrix() {
        let mut pm = PheromoneMatrix::new(5, 1.0);

        pm.deposit(0, 1, 0.5);
        assert!((pm.get(0, 1) - 1.5).abs() < 1e-10);

        pm.evaporate(0.1);
        assert!((pm.get(0, 1) - 1.35).abs() < 1e-10);
    }

    #[test]
    fn test_stigmergy_grid() {
        let mut grid = StigmergyGrid::new(10, 10, 0.1);

        grid.deposit(5, 5, 1.0);
        assert!((grid.get(5, 5) - 1.0).abs() < 1e-10);

        grid.decay();
        assert!((grid.get(5, 5) - 0.9).abs() < 1e-10);
    }
}
