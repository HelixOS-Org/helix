//! # Particle Swarm Optimization (PSO)
//!
//! PSO algorithm for continuous optimization.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::types::{Particle, Position, PsoConfig};

// ============================================================================
// PSO OPTIMIZER
// ============================================================================

/// Particle Swarm Optimizer
pub struct PsoOptimizer {
    /// Configuration
    pub config: PsoConfig,
    /// Particles
    pub particles: Vec<Particle>,
    /// Global best position
    pub global_best: Position,
    /// Global best fitness
    pub global_best_fitness: f64,
    /// Search bounds
    pub bounds: Vec<(f64, f64)>,
    /// Current iteration
    pub iteration: usize,
    /// RNG state
    rng_state: u64,
}

impl PsoOptimizer {
    /// Create new optimizer
    pub fn new(bounds: Vec<(f64, f64)>, config: PsoConfig) -> Self {
        let dim = bounds.len();
        let mut particles = Vec::with_capacity(config.n_particles);
        let mut rng = 12345u64;

        for _ in 0..config.n_particles {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            particles.push(Particle::random(&bounds, rng));
        }

        Self {
            config,
            particles,
            global_best: Position::zeros(dim),
            global_best_fitness: f64::MAX,
            bounds,
            iteration: 0,
            rng_state: rng,
        }
    }

    /// Random float [0, 1)
    fn rand(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }

    /// Initialize with custom positions
    pub fn with_initial_positions(mut self, positions: Vec<Position>) -> Self {
        for (i, pos) in positions.into_iter().enumerate() {
            if i < self.particles.len() {
                self.particles[i].position = pos.clone();
                self.particles[i].best_position = pos;
            }
        }
        self
    }

    /// Evaluate fitness for all particles
    pub fn evaluate<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&Position) -> f64,
    {
        for particle in &mut self.particles {
            particle.fitness = fitness_fn(&particle.position);
            particle.update_best();

            if particle.fitness < self.global_best_fitness {
                self.global_best_fitness = particle.fitness;
                self.global_best = particle.position.clone();
            }
        }
    }

    /// Update velocities and positions
    pub fn update(&mut self) {
        let w = self.config.inertia;
        let c1 = self.config.c1;
        let c2 = self.config.c2;

        // Pre-generate random values to avoid borrowing self inside the particle loop
        let max_dim = self
            .particles
            .iter()
            .map(|p| p.position.dim())
            .max()
            .unwrap_or(0);
        let n_particles = self.particles.len();
        let mut random_values: Vec<(f64, f64)> = Vec::with_capacity(n_particles * max_dim);
        for _ in 0..(n_particles * max_dim) {
            random_values.push((self.rand(), self.rand()));
        }

        // Clone global_best to avoid borrowing self inside the loop
        let global_best = self.global_best.clone();

        for (p_idx, particle) in self.particles.iter_mut().enumerate() {
            let dim = particle.position.dim();

            for d in 0..dim {
                let (r1, r2) = random_values[p_idx * max_dim + d];

                // Velocity update
                let cognitive =
                    c1 * r1 * (particle.best_position.get(d) - particle.position.get(d));
                let social = c2 * r2 * (global_best.get(d) - particle.position.get(d));

                let new_vel = w * particle.velocity.values[d] + cognitive + social;
                particle.velocity.values[d] = new_vel;
            }

            // Clamp velocity
            particle.velocity.clamp_magnitude(self.config.max_velocity);

            // Position update
            for d in 0..dim {
                particle.position.values[d] += particle.velocity.values[d];
            }

            // Clamp to bounds
            particle.position.clamp(&self.bounds);
        }

        self.iteration += 1;
    }

    /// Run optimization step
    pub fn step<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&Position) -> f64,
    {
        self.evaluate(&fitness_fn);
        self.update();
    }

    /// Run full optimization
    pub fn optimize<F>(&mut self, fitness_fn: F) -> PsoResult
    where
        F: Fn(&Position) -> f64,
    {
        let mut history = Vec::with_capacity(self.config.max_iterations);

        // Initial evaluation
        self.evaluate(&fitness_fn);
        history.push(self.global_best_fitness);

        for _ in 0..self.config.max_iterations {
            self.update();
            self.evaluate(&fitness_fn);
            history.push(self.global_best_fitness);
        }

        PsoResult {
            best_position: self.global_best.clone(),
            best_fitness: self.global_best_fitness,
            iterations: self.iteration,
            history,
        }
    }

    /// Get diversity measure
    pub fn diversity(&self) -> f64 {
        if self.particles.is_empty() {
            return 0.0;
        }

        let dim = self.bounds.len();
        let n = self.particles.len() as f64;

        // Compute centroid
        let mut centroid = alloc::vec![0.0; dim];
        for p in &self.particles {
            for (d, &v) in p.position.values.iter().enumerate() {
                centroid[d] += v / n;
            }
        }

        // Average distance to centroid
        let mut total_dist = 0.0;
        for p in &self.particles {
            let mut dist_sq = 0.0;
            for (d, &v) in p.position.values.iter().enumerate() {
                let diff = v - centroid[d];
                dist_sq += diff * diff;
            }
            total_dist += libm::sqrt(dist_sq);
        }

        total_dist / n
    }
}

/// PSO result
#[derive(Debug, Clone)]
pub struct PsoResult {
    /// Best position found
    pub best_position: Position,
    /// Best fitness value
    pub best_fitness: f64,
    /// Number of iterations
    pub iterations: usize,
    /// Fitness history
    pub history: Vec<f64>,
}

// ============================================================================
// ADAPTIVE PSO VARIANTS
// ============================================================================

/// Adaptive PSO with dynamic parameters
pub struct AdaptivePso {
    /// Base optimizer
    inner: PsoOptimizer,
    /// Minimum inertia
    w_min: f64,
    /// Maximum inertia
    w_max: f64,
    /// Stagnation counter
    stagnation: usize,
    /// Previous best
    prev_best: f64,
}

impl AdaptivePso {
    /// Create adaptive PSO
    pub fn new(bounds: Vec<(f64, f64)>, config: PsoConfig) -> Self {
        Self {
            inner: PsoOptimizer::new(bounds, config),
            w_min: 0.4,
            w_max: 0.9,
            stagnation: 0,
            prev_best: f64::MAX,
        }
    }

    /// Adapt parameters based on progress
    fn adapt(&mut self) {
        // Check for stagnation
        if (self.inner.global_best_fitness - self.prev_best).abs() < 1e-10 {
            self.stagnation += 1;
        } else {
            self.stagnation = 0;
        }
        self.prev_best = self.inner.global_best_fitness;

        // Linearly decrease inertia
        let progress = self.inner.iteration as f64 / self.inner.config.max_iterations as f64;
        self.inner.config.inertia = self.w_max - (self.w_max - self.w_min) * progress;

        // Increase exploration if stagnating
        if self.stagnation > 5 {
            self.inner.config.inertia = self.w_max;
            self.inner.config.c1 = 2.5; // More cognitive
            self.inner.config.c2 = 0.5; // Less social
        }
    }

    /// Run optimization step
    pub fn step<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&Position) -> f64,
    {
        self.adapt();
        self.inner.step(fitness_fn);
    }

    /// Run full optimization
    pub fn optimize<F>(&mut self, fitness_fn: F) -> PsoResult
    where
        F: Fn(&Position) -> f64,
    {
        let mut history = Vec::with_capacity(self.inner.config.max_iterations);

        self.inner.evaluate(&fitness_fn);
        history.push(self.inner.global_best_fitness);

        for _ in 0..self.inner.config.max_iterations {
            self.adapt();
            self.inner.update();
            self.inner.evaluate(&fitness_fn);
            history.push(self.inner.global_best_fitness);
        }

        PsoResult {
            best_position: self.inner.global_best.clone(),
            best_fitness: self.inner.global_best_fitness,
            iterations: self.inner.iteration,
            history,
        }
    }

    /// Get global best
    pub fn best(&self) -> (&Position, f64) {
        (&self.inner.global_best, self.inner.global_best_fitness)
    }
}

// ============================================================================
// MULTI-SWARM PSO
// ============================================================================

/// Multi-swarm PSO with migration
pub struct MultiSwarmPso {
    /// Sub-swarms
    swarms: Vec<PsoOptimizer>,
    /// Migration interval
    migration_interval: usize,
    /// Current iteration
    iteration: usize,
}

impl MultiSwarmPso {
    /// Create multi-swarm optimizer
    pub fn new(
        bounds: Vec<(f64, f64)>,
        n_swarms: usize,
        particles_per_swarm: usize,
        migration_interval: usize,
    ) -> Self {
        let mut swarms = Vec::with_capacity(n_swarms);

        for i in 0..n_swarms {
            let config = PsoConfig {
                n_particles: particles_per_swarm,
                ..PsoConfig::default()
            };

            let mut swarm = PsoOptimizer::new(bounds.clone(), config);
            swarm.rng_state = 12345u64 + i as u64 * 1000;
            swarms.push(swarm);
        }

        Self {
            swarms,
            migration_interval,
            iteration: 0,
        }
    }

    /// Migrate best particles between swarms
    fn migrate(&mut self) {
        let n = self.swarms.len();
        if n < 2 {
            return;
        }

        // Collect best positions
        let best_positions: Vec<Position> =
            self.swarms.iter().map(|s| s.global_best.clone()).collect();

        // Ring migration: send best to next swarm
        for (i, best_pos) in best_positions.iter().enumerate() {
            let next = (i + 1) % n;
            let migrant = best_pos.clone();

            // Replace worst particle in next swarm
            if let Some(worst_idx) = self.swarms[next]
                .particles
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.fitness.partial_cmp(&b.fitness).unwrap())
                .map(|(idx, _)| idx)
            {
                self.swarms[next].particles[worst_idx].position = migrant.clone();
                self.swarms[next].particles[worst_idx].best_position = migrant;
            }
        }
    }

    /// Run optimization step
    pub fn step<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&Position) -> f64 + Clone,
    {
        for swarm in &mut self.swarms {
            swarm.step(fitness_fn.clone());
        }

        self.iteration += 1;

        if self.iteration % self.migration_interval == 0 {
            self.migrate();
        }
    }

    /// Get global best across all swarms
    pub fn global_best(&self) -> (Position, f64) {
        self.swarms
            .iter()
            .map(|s| (s.global_best.clone(), s.global_best_fitness))
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap_or((Position::zeros(1), f64::MAX))
    }

    /// Run full optimization
    pub fn optimize<F>(&mut self, fitness_fn: F, max_iterations: usize) -> PsoResult
    where
        F: Fn(&Position) -> f64 + Clone,
    {
        let mut history = Vec::with_capacity(max_iterations);

        for _ in 0..max_iterations {
            self.step(fitness_fn.clone());
            let (_, best_fitness) = self.global_best();
            history.push(best_fitness);
        }

        let (best_pos, best_fit) = self.global_best();

        PsoResult {
            best_position: best_pos,
            best_fitness: best_fit,
            iterations: self.iteration,
            history,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere(pos: &Position) -> f64 {
        pos.values.iter().map(|&x| x * x).sum()
    }

    fn rastrigin(pos: &Position) -> f64 {
        let a = 10.0;
        let n = pos.dim() as f64;
        let mut sum = a * n;
        for &x in &pos.values {
            sum += x * x - a * libm::cos(2.0 * core::f64::consts::PI * x);
        }
        sum
    }

    #[test]
    fn test_pso_sphere() {
        let bounds = alloc::vec![(-5.0, 5.0); 2];
        let config = PsoConfig {
            n_particles: 20,
            max_iterations: 50,
            ..Default::default()
        };

        let mut pso = PsoOptimizer::new(bounds, config);
        let result = pso.optimize(sphere);

        // Should find near-zero minimum
        assert!(result.best_fitness < 1.0);
    }

    #[test]
    fn test_adaptive_pso() {
        let bounds = alloc::vec![(-5.0, 5.0); 3];
        let config = PsoConfig {
            n_particles: 30,
            max_iterations: 100,
            ..Default::default()
        };

        let mut apso = AdaptivePso::new(bounds, config);
        let result = apso.optimize(sphere);

        assert!(result.best_fitness < 0.1);
    }

    #[test]
    fn test_multi_swarm_pso() {
        let bounds = alloc::vec![(-5.0, 5.0); 2];
        let mut mpso = MultiSwarmPso::new(bounds, 3, 10, 5);

        let result = mpso.optimize(sphere, 50);

        assert!(result.best_fitness < 1.0);
    }

    #[test]
    fn test_pso_diversity() {
        let bounds = alloc::vec![(-5.0, 5.0); 2];
        let config = PsoConfig::default();

        let pso = PsoOptimizer::new(bounds, config);
        let diversity = pso.diversity();

        // Initial diversity should be > 0
        assert!(diversity > 0.0);
    }
}
