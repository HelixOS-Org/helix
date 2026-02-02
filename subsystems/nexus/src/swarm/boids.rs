//! # Boids Flocking Simulation
//!
//! Emergent behavior simulation using Reynolds' boids model.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::types::{Boid, BoidsConfig, Position, Velocity};

// ============================================================================
// BOID FLOCK
// ============================================================================

/// Flock of boids for emergent behavior
pub struct BoidFlock {
    /// Configuration
    pub config: BoidsConfig,
    /// All boids
    pub boids: Vec<Boid>,
    /// World bounds (min, max for each dimension)
    pub bounds: Vec<(f64, f64)>,
}

impl BoidFlock {
    /// Create new flock
    pub fn new(n_boids: usize, dims: usize, config: BoidsConfig) -> Self {
        let mut boids = Vec::with_capacity(n_boids);
        let mut rng = 12345u64;

        for id in 0..n_boids {
            // Random initial position and velocity
            let mut pos_vals = Vec::with_capacity(dims);
            let mut vel_vals = Vec::with_capacity(dims);

            for _ in 0..dims {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let p = (rng as f64 / u64::MAX as f64) * 100.0;

                rng ^= rng << 13;
                let v = (rng as f64 / u64::MAX as f64 - 0.5) * 2.0;

                pos_vals.push(p);
                vel_vals.push(v);
            }

            let position = Position::new(pos_vals);
            let velocity = Velocity::new(vel_vals);
            boids.push(Boid::new(id, position, velocity));
        }

        // Default bounds
        let bounds = (0..dims).map(|_| (0.0, 100.0)).collect();

        Self {
            config,
            boids,
            bounds,
        }
    }

    /// Set world bounds
    pub fn with_bounds(mut self, bounds: Vec<(f64, f64)>) -> Self {
        self.bounds = bounds;
        self
    }

    /// Get neighbors within perception radius
    fn get_neighbors(&self, boid_idx: usize) -> Vec<usize> {
        let boid = &self.boids[boid_idx];
        let mut neighbors = Vec::new();

        for (i, other) in self.boids.iter().enumerate() {
            if i == boid_idx {
                continue;
            }

            let dist = boid.position.distance(&other.position);
            if dist < self.config.perception_radius {
                neighbors.push(i);
            }
        }

        neighbors
    }

    /// Separation: steer away from nearby boids
    fn separation(&self, boid_idx: usize, neighbors: &[usize]) -> Velocity {
        let boid = &self.boids[boid_idx];
        let dim = boid.velocity.dim();
        let mut steer = Velocity::zeros(dim);

        if neighbors.is_empty() {
            return steer;
        }

        for &i in neighbors {
            let other = &self.boids[i];
            let dist = boid.position.distance(&other.position);

            if dist > 1e-10 {
                // Vector pointing away from neighbor
                let diff = boid.position.sub(&other.position);
                // Weight by inverse distance
                let weight = 1.0 / dist;

                for (d, &v) in diff.values.iter().enumerate() {
                    if d < dim {
                        steer.values[d] += v * weight;
                    }
                }
            }
        }

        // Normalize and scale
        let mag: f64 = steer.values.iter().map(|&v| v * v).sum();
        let mag = libm::sqrt(mag);

        if mag > 1e-10 {
            for v in &mut steer.values {
                *v = (*v / mag) * self.config.max_force * self.config.separation_weight;
            }
        }

        steer
    }

    /// Alignment: steer towards average heading of neighbors
    fn alignment(&self, boid_idx: usize, neighbors: &[usize]) -> Velocity {
        let boid = &self.boids[boid_idx];
        let dim = boid.velocity.dim();
        let mut avg = Velocity::zeros(dim);

        if neighbors.is_empty() {
            return avg;
        }

        for &i in neighbors {
            let other = &self.boids[i];
            for (d, &v) in other.velocity.values.iter().enumerate() {
                if d < dim {
                    avg.values[d] += v;
                }
            }
        }

        let n = neighbors.len() as f64;
        for v in &mut avg.values {
            *v /= n;
        }

        // Steer towards average velocity
        let mut steer = Velocity::zeros(dim);
        for d in 0..dim {
            steer.values[d] =
                (avg.values[d] - boid.velocity.values[d]) * self.config.alignment_weight;
        }

        steer.clamp_magnitude(self.config.max_force);
        steer
    }

    /// Cohesion: steer towards center of mass of neighbors
    fn cohesion(&self, boid_idx: usize, neighbors: &[usize]) -> Velocity {
        let boid = &self.boids[boid_idx];
        let dim = boid.position.dim();
        let mut center = Position::zeros(dim);

        if neighbors.is_empty() {
            return Velocity::zeros(dim);
        }

        for &i in neighbors {
            let other = &self.boids[i];
            for (d, &v) in other.position.values.iter().enumerate() {
                if d < dim {
                    center.values[d] += v;
                }
            }
        }

        let n = neighbors.len() as f64;
        for v in &mut center.values {
            *v /= n;
        }

        // Direction to center
        let mut steer = Velocity::zeros(dim);
        for d in 0..dim {
            steer.values[d] =
                (center.values[d] - boid.position.values[d]) * self.config.cohesion_weight * 0.01; // Scale down
        }

        steer.clamp_magnitude(self.config.max_force);
        steer
    }

    /// Boundary avoidance
    fn boundary_force(&self, boid_idx: usize) -> Velocity {
        let boid = &self.boids[boid_idx];
        let dim = boid.position.dim();
        let mut force = Velocity::zeros(dim);

        let margin = self.config.perception_radius * 0.5;

        for d in 0..dim {
            if d < self.bounds.len() {
                let (lo, hi) = self.bounds[d];
                let pos = boid.position.values[d];

                if pos < lo + margin {
                    force.values[d] = self.config.max_force;
                } else if pos > hi - margin {
                    force.values[d] = -self.config.max_force;
                }
            }
        }

        force
    }

    /// Update single boid
    fn update_boid(&mut self, boid_idx: usize) {
        let neighbors = self.get_neighbors(boid_idx);

        let sep = self.separation(boid_idx, &neighbors);
        let ali = self.alignment(boid_idx, &neighbors);
        let coh = self.cohesion(boid_idx, &neighbors);
        let boundary = self.boundary_force(boid_idx);

        // Apply forces
        self.boids[boid_idx].apply_force(&sep);
        self.boids[boid_idx].apply_force(&ali);
        self.boids[boid_idx].apply_force(&coh);
        self.boids[boid_idx].apply_force(&boundary);

        // Update position
        self.boids[boid_idx].update(self.config.max_speed);
    }

    /// Step simulation
    pub fn step(&mut self) {
        // Compute forces for all boids
        let forces: Vec<_> = (0..self.boids.len())
            .map(|i| {
                let neighbors = self.get_neighbors(i);
                let sep = self.separation(i, &neighbors);
                let ali = self.alignment(i, &neighbors);
                let coh = self.cohesion(i, &neighbors);
                let boundary = self.boundary_force(i);
                (sep, ali, coh, boundary)
            })
            .collect();

        // Apply forces and update
        for (i, (sep, ali, coh, boundary)) in forces.into_iter().enumerate() {
            self.boids[i].apply_force(&sep);
            self.boids[i].apply_force(&ali);
            self.boids[i].apply_force(&coh);
            self.boids[i].apply_force(&boundary);
            self.boids[i].update(self.config.max_speed);
        }
    }

    /// Get flock center of mass
    pub fn center_of_mass(&self) -> Position {
        if self.boids.is_empty() {
            return Position::zeros(2);
        }

        let dim = self.boids[0].position.dim();
        let mut center = Position::zeros(dim);
        let n = self.boids.len() as f64;

        for boid in &self.boids {
            for (d, &v) in boid.position.values.iter().enumerate() {
                if d < dim {
                    center.values[d] += v / n;
                }
            }
        }

        center
    }

    /// Get flock velocity variance (measure of order)
    pub fn velocity_variance(&self) -> f64 {
        if self.boids.is_empty() {
            return 0.0;
        }

        let dim = self.boids[0].velocity.dim();
        let n = self.boids.len() as f64;

        // Mean velocity
        let mut mean = alloc::vec![0.0; dim];
        for boid in &self.boids {
            for (d, &v) in boid.velocity.values.iter().enumerate() {
                mean[d] += v / n;
            }
        }

        // Variance
        let mut variance = 0.0;
        for boid in &self.boids {
            for (d, &v) in boid.velocity.values.iter().enumerate() {
                let diff = v - mean[d];
                variance += diff * diff;
            }
        }

        variance / n
    }

    /// Get order parameter (0 = chaos, 1 = aligned)
    pub fn order_parameter(&self) -> f64 {
        if self.boids.is_empty() {
            return 0.0;
        }

        let dim = self.boids[0].velocity.dim();
        let mut avg_vel = alloc::vec![0.0; dim];
        let mut avg_speed = 0.0;

        for boid in &self.boids {
            let speed: f64 = boid.velocity.values.iter().map(|&v| v * v).sum();
            let speed = libm::sqrt(speed);
            avg_speed += speed;

            for (d, &v) in boid.velocity.values.iter().enumerate() {
                avg_vel[d] += v;
            }
        }

        let n = self.boids.len() as f64;
        avg_speed /= n;

        if avg_speed < 1e-10 {
            return 0.0;
        }

        let avg_vel_mag: f64 = avg_vel.iter().map(|&v| (v / n) * (v / n)).sum();
        let avg_vel_mag = libm::sqrt(avg_vel_mag);

        avg_vel_mag / avg_speed
    }
}

// ============================================================================
// PREDATOR-PREY EXTENSION
// ============================================================================

/// Predator that chases the flock
pub struct Predator {
    /// Position
    pub position: Position,
    /// Velocity
    pub velocity: Velocity,
    /// Speed
    pub speed: f64,
    /// Influence radius
    pub influence_radius: f64,
}

impl Predator {
    /// Create predator
    pub fn new(position: Position, speed: f64) -> Self {
        let dim = position.dim();
        Self {
            position,
            velocity: Velocity::zeros(dim),
            speed,
            influence_radius: 100.0,
        }
    }

    /// Chase center of flock
    pub fn chase(&mut self, flock_center: &Position) {
        let dim = self.position.dim();
        let mut direction = Velocity::zeros(dim);

        for d in 0..dim {
            direction.values[d] = flock_center.get(d) - self.position.get(d);
        }

        // Normalize
        let mag: f64 = direction.values.iter().map(|&v| v * v).sum();
        let mag = libm::sqrt(mag);

        if mag > 1e-10 {
            for d in 0..dim {
                self.velocity.values[d] = (direction.values[d] / mag) * self.speed;
            }
        }

        // Update position
        for d in 0..dim {
            self.position.values[d] += self.velocity.values[d];
        }
    }
}

/// Flock with predator avoidance
pub struct PredatorPreyFlock {
    /// Base flock
    pub flock: BoidFlock,
    /// Predators
    pub predators: Vec<Predator>,
    /// Predator avoidance weight
    pub avoidance_weight: f64,
}

impl PredatorPreyFlock {
    /// Create new system
    pub fn new(flock: BoidFlock) -> Self {
        Self {
            flock,
            predators: Vec::new(),
            avoidance_weight: 5.0,
        }
    }

    /// Add predator
    pub fn add_predator(&mut self, predator: Predator) {
        self.predators.push(predator);
    }

    /// Compute predator avoidance force for boid
    fn avoidance_force(&self, boid_idx: usize) -> Velocity {
        let boid = &self.flock.boids[boid_idx];
        let dim = boid.position.dim();
        let mut force = Velocity::zeros(dim);

        for pred in &self.predators {
            let dist = boid.position.distance(&pred.position);

            if dist < pred.influence_radius && dist > 1e-10 {
                // Flee force
                let weight = (pred.influence_radius - dist) / pred.influence_radius;

                for d in 0..dim {
                    let diff = boid.position.get(d) - pred.position.get(d);
                    force.values[d] += (diff / dist) * weight * self.avoidance_weight;
                }
            }
        }

        force.clamp_magnitude(self.flock.config.max_force * 2.0);
        force
    }

    /// Step simulation
    pub fn step(&mut self) {
        // Update predators
        let center = self.flock.center_of_mass();
        for pred in &mut self.predators {
            pred.chase(&center);
        }

        // Compute all forces
        let forces: Vec<_> = (0..self.flock.boids.len())
            .map(|i| {
                let neighbors = self.flock.get_neighbors(i);
                let sep = self.flock.separation(i, &neighbors);
                let ali = self.flock.alignment(i, &neighbors);
                let coh = self.flock.cohesion(i, &neighbors);
                let boundary = self.flock.boundary_force(i);
                let avoid = self.avoidance_force(i);
                (sep, ali, coh, boundary, avoid)
            })
            .collect();

        // Apply and update
        for (i, (sep, ali, coh, boundary, avoid)) in forces.into_iter().enumerate() {
            self.flock.boids[i].apply_force(&sep);
            self.flock.boids[i].apply_force(&ali);
            self.flock.boids[i].apply_force(&coh);
            self.flock.boids[i].apply_force(&boundary);
            self.flock.boids[i].apply_force(&avoid);
            self.flock.boids[i].update(self.flock.config.max_speed);
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
    fn test_boid_flock_creation() {
        let config = BoidsConfig::default();
        let flock = BoidFlock::new(20, 2, config);

        assert_eq!(flock.boids.len(), 20);
    }

    #[test]
    fn test_boid_flock_step() {
        let config = BoidsConfig::default();
        let mut flock = BoidFlock::new(10, 2, config);

        let initial_center = flock.center_of_mass();

        for _ in 0..10 {
            flock.step();
        }

        let final_center = flock.center_of_mass();

        // Center should have moved (boids are not static)
        let moved = initial_center.distance(&final_center) > 0.0;
        assert!(
            moved
                || flock
                    .boids
                    .iter()
                    .all(|b| b.velocity.values.iter().all(|&v| v.abs() < 1e-10))
        );
    }

    #[test]
    fn test_order_parameter() {
        let config = BoidsConfig::default();
        let mut flock = BoidFlock::new(20, 2, config);

        // Run simulation to reach some order
        for _ in 0..50 {
            flock.step();
        }

        let order = flock.order_parameter();

        // Order should be between 0 and 1
        assert!(order >= 0.0 && order <= 1.0);
    }

    #[test]
    fn test_predator_prey() {
        let config = BoidsConfig::default();
        let flock = BoidFlock::new(20, 2, config);

        let mut system = PredatorPreyFlock::new(flock);

        let predator = Predator::new(Position::new(alloc::vec![50.0, 50.0]), 3.0);
        system.add_predator(predator);

        for _ in 0..20 {
            system.step();
        }

        // System should still be running
        assert!(!system.flock.boids.is_empty());
    }
}
