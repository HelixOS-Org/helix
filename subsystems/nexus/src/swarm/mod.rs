//! # Emergent Swarm Intelligence
//!
//! Revolutionary decentralized intelligence system inspired by biological swarms.
//! Enables collective behavior from simple local interactions, creating emergent
//! global intelligence without central coordination.
//!
//! ## Module Structure
//!
//! - [`types`] - Core types: Position, Velocity, Particle, Ant, Boid, PheromoneMatrix
//! - [`pso`] - Particle Swarm Optimization: PsoOptimizer, AdaptivePso, MultiSwarmPso
//! - [`aco`] - Ant Colony Optimization: AcoOptimizer, MaxMinAntSystem, AntColonySystem
//! - [`boids`] - Flocking simulation: BoidFlock, PredatorPreyFlock
//! - [`stigmergy`] - Indirect coordination: MultiChannelStigmergy, ForagingSimulation
//!
//! ## Legacy Submodules
//!
//! - [`agent`] - Generic swarm agents
//! - [`bees`] - Bee colony algorithms
//! - [`consensus`] - Distributed consensus
//! - [`emergence`] - Emergent behavior patterns
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_nexus::swarm::{
//!     types::{Position, PsoConfig},
//!     pso::PsoOptimizer,
//!     aco::AcoOptimizer,
//!     boids::BoidFlock,
//! };
//!
//! // Optimize with PSO
//! let config = PsoConfig::default();
//! let mut pso = PsoOptimizer::new(30, 10, -5.0, 5.0, config, 42);
//! pso.optimize(|pos| pos.iter().map(|x| x * x).sum(), 100);
//!
//! // Simulate flocking
//! let mut flock = BoidFlock::new(100, 100.0, 100.0, 42);
//! flock.step(0.1);
//! ```

#![allow(dead_code)]

extern crate alloc;

// Production-quality submodules
pub mod aco;
pub mod boids;
pub mod pso;
pub mod stigmergy;
pub mod types;

// Re-export main types
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

pub use aco::{AcoOptimizer, AcoResult, AntColonySystem, DistanceMatrix, MaxMinAntSystem};
pub use boids::{BoidFlock, Predator, PredatorPreyFlock};
pub use pso::{AdaptivePso, MultiSwarmPso, PsoOptimizer, PsoResult};
pub use stigmergy::{
    ConstructionSimulation, ForagingSimulation, MultiChannelStigmergy, SignalType, StigmergicAgent,
};
pub use types::{
    AcoConfig, Ant, BoidsConfig, Particle, PheromoneMatrix, Position, PsoConfig, StigmergyGrid,
    Velocity,
};

/// 2D Vector for positions and velocities
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn magnitude(&self) -> f64 {
        libm::sqrt(self.x * self.x + self.y * self.y)
    }

    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag > 1e-8 {
            Self {
                x: self.x / mag,
                y: self.y / mag,
            }
        } else {
            Self::zero()
        }
    }

    pub fn distance(&self, other: &Vec2) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        libm::sqrt(dx * dx + dy * dy)
    }

    pub fn add(&self, other: &Vec2) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    pub fn sub(&self, other: &Vec2) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }

    pub fn scale(&self, s: f64) -> Self {
        Self {
            x: self.x * s,
            y: self.y * s,
        }
    }

    pub fn limit(&self, max: f64) -> Self {
        let mag = self.magnitude();
        if mag > max {
            self.normalize().scale(max)
        } else {
            *self
        }
    }
}

/// Simple agent identifier
pub type AgentId = u64;

/// Basic swarm agent
#[derive(Debug, Clone)]
pub struct Agent {
    /// Agent ID
    pub id: AgentId,
    /// Position
    pub position: Vec2,
    /// Velocity
    pub velocity: Vec2,
    /// Best known position
    pub best_position: Vec2,
    /// Best known fitness
    pub best_fitness: f64,
    /// Current fitness
    pub fitness: f64,
    /// Agent state
    pub state: AgentState,
}

/// Agent states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Exploring,
    Exploiting,
    Resting,
    Communicating,
    Returning,
}

impl Agent {
    /// Create a new agent at position
    pub fn new(id: AgentId, position: Vec2) -> Self {
        Self {
            id,
            position,
            velocity: Vec2::zero(),
            best_position: position,
            best_fitness: f64::NEG_INFINITY,
            fitness: 0.0,
            state: AgentState::Exploring,
        }
    }

    /// Update best if current is better
    pub fn update_best(&mut self) {
        if self.fitness > self.best_fitness {
            self.best_fitness = self.fitness;
            self.best_position = self.position;
        }
    }

    /// Move agent by velocity
    pub fn step(&mut self, dt: f64) {
        self.position = self.position.add(&self.velocity.scale(dt));
    }
}

/// Pheromone types for ACO
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PheromoneType {
    /// Positive attraction
    Attraction,
    /// Negative repulsion
    Repulsion,
    /// Food/resource marker
    Resource,
    /// Danger marker
    Danger,
    /// Path marker
    Trail,
}

/// Pheromone deposit
#[derive(Debug, Clone)]
pub struct Pheromone {
    /// Type
    pub ptype: PheromoneType,
    /// Intensity
    pub intensity: f64,
    /// Position
    pub position: Vec2,
    /// Age (for decay)
    pub age: f64,
}

impl Pheromone {
    /// Create new pheromone
    pub fn new(ptype: PheromoneType, intensity: f64, position: Vec2) -> Self {
        Self {
            ptype,
            intensity,
            position,
            age: 0.0,
        }
    }

    /// Decay over time
    pub fn decay(&mut self, rate: f64, dt: f64) {
        self.age += dt;
        self.intensity *= 1.0 - rate * dt;
        self.intensity = self.intensity.max(0.0);
    }

    /// Is effectively gone?
    pub fn is_expired(&self) -> bool {
        self.intensity < 0.01
    }
}

/// Ant Colony Optimization for graph problems
#[derive(Debug)]
pub struct AntColony {
    /// Number of nodes
    num_nodes: usize,
    /// Distance matrix
    distances: Vec<Vec<f64>>,
    /// Pheromone matrix
    pheromones: Vec<Vec<f64>>,
    /// Heuristic information (1/distance)
    heuristics: Vec<Vec<f64>>,
    /// Alpha (pheromone importance)
    alpha: f64,
    /// Beta (heuristic importance)
    beta: f64,
    /// Evaporation rate
    evaporation: f64,
    /// Q (pheromone deposit constant)
    q: f64,
    /// Number of ants
    num_ants: usize,
    /// Best tour found
    best_tour: Vec<usize>,
    /// Best tour length
    best_length: f64,
}

impl AntColony {
    /// Create new ACO instance
    pub fn new(distances: Vec<Vec<f64>>, num_ants: usize) -> Self {
        let num_nodes = distances.len();

        // Initialize pheromones uniformly
        let initial_pheromone = 1.0 / num_nodes as f64;
        let pheromones = alloc::vec![alloc::vec![initial_pheromone; num_nodes]; num_nodes];

        // Compute heuristics
        let heuristics: Vec<Vec<f64>> = distances
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&d| if d > 0.0 { 1.0 / d } else { 0.0 })
                    .collect()
            })
            .collect();

        Self {
            num_nodes,
            distances,
            pheromones,
            heuristics,
            alpha: 1.0,
            beta: 2.0,
            evaporation: 0.1,
            q: 100.0,
            num_ants,
            best_tour: Vec::new(),
            best_length: f64::INFINITY,
        }
    }

    /// Run one iteration
    pub fn iterate(&mut self, rng: &mut u64) {
        let mut all_tours = Vec::new();
        let mut all_lengths = Vec::new();

        // Each ant constructs a tour
        for _ in 0..self.num_ants {
            let (tour, length) = self.construct_tour(rng);
            all_tours.push(tour);
            all_lengths.push(length);
        }

        // Update pheromones
        self.update_pheromones(&all_tours, &all_lengths);

        // Update best
        for (tour, &length) in all_tours.iter().zip(&all_lengths) {
            if length < self.best_length {
                self.best_length = length;
                self.best_tour = tour.clone();
            }
        }
    }

    /// Construct tour for one ant
    fn construct_tour(&self, rng: &mut u64) -> (Vec<usize>, f64) {
        let mut tour = Vec::with_capacity(self.num_nodes);
        let mut visited = BTreeSet::new();

        // Start from random node
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let start = (*rng as usize) % self.num_nodes;

        tour.push(start);
        visited.insert(start);

        while tour.len() < self.num_nodes {
            let current = *tour.last().unwrap();
            let next = self.select_next(current, &visited, rng);
            tour.push(next);
            visited.insert(next);
        }

        let length = self.tour_length(&tour);
        (tour, length)
    }

    /// Select next node using probabilistic rule
    fn select_next(&self, current: usize, visited: &BTreeSet<usize>, rng: &mut u64) -> usize {
        let mut probabilities = Vec::new();
        let mut total = 0.0;

        for j in 0..self.num_nodes {
            if visited.contains(&j) {
                probabilities.push(0.0);
            } else {
                let tau = self.pheromones[current][j];
                let eta = self.heuristics[current][j];
                let prob = libm::pow(tau, self.alpha) * libm::pow(eta, self.beta);
                probabilities.push(prob);
                total += prob;
            }
        }

        // Roulette wheel selection
        *rng ^= *rng << 13;
        *rng ^= *rng >> 7;
        *rng ^= *rng << 17;
        let r = (*rng as f64 / u64::MAX as f64) * total;

        let mut cumulative = 0.0;
        for (j, &prob) in probabilities.iter().enumerate() {
            cumulative += prob;
            if cumulative >= r {
                return j;
            }
        }

        // Fallback: first unvisited
        for j in 0..self.num_nodes {
            if !visited.contains(&j) {
                return j;
            }
        }
        0
    }

    /// Calculate tour length
    fn tour_length(&self, tour: &[usize]) -> f64 {
        let mut length = 0.0;
        for i in 0..tour.len() {
            let from = tour[i];
            let to = tour[(i + 1) % tour.len()];
            length += self.distances[from][to];
        }
        length
    }

    /// Update pheromones
    fn update_pheromones(&mut self, tours: &[Vec<usize>], lengths: &[f64]) {
        // Evaporation
        for row in &mut self.pheromones {
            for p in row {
                *p *= 1.0 - self.evaporation;
            }
        }

        // Deposit
        for (tour, &length) in tours.iter().zip(lengths) {
            let deposit = self.q / length;
            for i in 0..tour.len() {
                let from = tour[i];
                let to = tour[(i + 1) % tour.len()];
                self.pheromones[from][to] += deposit;
                self.pheromones[to][from] += deposit;
            }
        }
    }

    /// Get best tour
    pub fn best(&self) -> (&[usize], f64) {
        (&self.best_tour, self.best_length)
    }
}

/// Particle Swarm Optimization
#[derive(Debug)]
pub struct ParticleSwarm {
    /// Particles
    particles: Vec<Agent>,
    /// Global best position
    global_best_position: Vec2,
    /// Global best fitness
    global_best_fitness: f64,
    /// Inertia weight
    inertia: f64,
    /// Cognitive coefficient
    c1: f64,
    /// Social coefficient
    c2: f64,
    /// Maximum velocity
    max_velocity: f64,
    /// Search bounds
    bounds: (Vec2, Vec2),
}

impl ParticleSwarm {
    /// Create new PSO
    pub fn new(num_particles: usize, bounds: (Vec2, Vec2), rng: &mut u64) -> Self {
        let mut particles = Vec::with_capacity(num_particles);

        for i in 0..num_particles {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let rx = *rng as f64 / u64::MAX as f64;

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let ry = *rng as f64 / u64::MAX as f64;

            let x = bounds.0.x + rx * (bounds.1.x - bounds.0.x);
            let y = bounds.0.y + ry * (bounds.1.y - bounds.0.y);

            particles.push(Agent::new(i as AgentId, Vec2::new(x, y)));
        }

        Self {
            particles,
            global_best_position: Vec2::zero(),
            global_best_fitness: f64::NEG_INFINITY,
            inertia: 0.729,
            c1: 1.494,
            c2: 1.494,
            max_velocity: (bounds.1.x - bounds.0.x) * 0.1,
            bounds,
        }
    }

    /// Evaluate fitness for all particles
    pub fn evaluate<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&Vec2) -> f64,
    {
        for particle in &mut self.particles {
            particle.fitness = fitness_fn(&particle.position);
            particle.update_best();

            if particle.best_fitness > self.global_best_fitness {
                self.global_best_fitness = particle.best_fitness;
                self.global_best_position = particle.best_position;
            }
        }
    }

    /// Update particle velocities and positions
    pub fn update(&mut self, rng: &mut u64) {
        for particle in &mut self.particles {
            // Random factors
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let r1 = *rng as f64 / u64::MAX as f64;

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let r2 = *rng as f64 / u64::MAX as f64;

            // Cognitive component
            let cognitive = particle
                .best_position
                .sub(&particle.position)
                .scale(self.c1 * r1);

            // Social component
            let social = self
                .global_best_position
                .sub(&particle.position)
                .scale(self.c2 * r2);

            // Update velocity
            particle.velocity = particle
                .velocity
                .scale(self.inertia)
                .add(&cognitive)
                .add(&social)
                .limit(self.max_velocity);

            // Update position
            particle.position = particle.position.add(&particle.velocity);

            // Clamp to bounds
            particle.position.x = particle.position.x.clamp(self.bounds.0.x, self.bounds.1.x);
            particle.position.y = particle.position.y.clamp(self.bounds.0.y, self.bounds.1.y);
        }
    }

    /// Run optimization
    pub fn optimize<F>(&mut self, fitness_fn: F, iterations: usize, rng: &mut u64)
    where
        F: Fn(&Vec2) -> f64,
    {
        for _ in 0..iterations {
            self.evaluate(&fitness_fn);
            self.update(rng);
        }
    }

    /// Get best solution
    pub fn best(&self) -> (Vec2, f64) {
        (self.global_best_position, self.global_best_fitness)
    }
}

/// Bee colony algorithm
#[derive(Debug)]
pub struct BeeColony {
    /// Employed bees (solutions)
    employed: Vec<(Vec<f64>, f64)>, // (solution, fitness)
    /// Onlooker bees count
    num_onlookers: usize,
    /// Scout bees threshold
    scout_limit: usize,
    /// Trial counters (for abandonment)
    trials: Vec<usize>,
    /// Solution dimension
    dim: usize,
    /// Bounds
    bounds: (f64, f64),
    /// Best solution
    best_solution: Vec<f64>,
    /// Best fitness
    best_fitness: f64,
}

impl BeeColony {
    /// Create new bee colony
    pub fn new(num_employed: usize, dim: usize, bounds: (f64, f64), rng: &mut u64) -> Self {
        let mut employed = Vec::with_capacity(num_employed);

        for _ in 0..num_employed {
            let solution: Vec<f64> = (0..dim)
                .map(|_| {
                    *rng ^= *rng << 13;
                    *rng ^= *rng >> 7;
                    *rng ^= *rng << 17;
                    let r = *rng as f64 / u64::MAX as f64;
                    bounds.0 + r * (bounds.1 - bounds.0)
                })
                .collect();

            employed.push((solution, f64::NEG_INFINITY));
        }

        Self {
            employed,
            num_onlookers: num_employed,
            scout_limit: 100,
            trials: alloc::vec![0; num_employed],
            dim,
            bounds,
            best_solution: alloc::vec![0.0; dim],
            best_fitness: f64::NEG_INFINITY,
        }
    }

    /// Evaluate all solutions
    pub fn evaluate<F>(&mut self, fitness_fn: F)
    where
        F: Fn(&[f64]) -> f64,
    {
        for (solution, fitness) in &mut self.employed {
            *fitness = fitness_fn(solution);

            if *fitness > self.best_fitness {
                self.best_fitness = *fitness;
                self.best_solution = solution.clone();
            }
        }
    }

    /// Employed bee phase
    fn employed_phase<F>(&mut self, fitness_fn: &F, rng: &mut u64)
    where
        F: Fn(&[f64]) -> f64,
    {
        for i in 0..self.employed.len() {
            // Pick random dimension and neighbor
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let dim = (*rng as usize) % self.dim;

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let mut k = (*rng as usize) % self.employed.len();
            while k == i {
                *rng ^= *rng << 13;
                k = (*rng as usize) % self.employed.len();
            }

            // Create new solution
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let phi = (*rng as f64 / u64::MAX as f64) * 2.0 - 1.0;

            let mut new_solution = self.employed[i].0.clone();
            new_solution[dim] =
                self.employed[i].0[dim] + phi * (self.employed[i].0[dim] - self.employed[k].0[dim]);
            new_solution[dim] = new_solution[dim].clamp(self.bounds.0, self.bounds.1);

            let new_fitness = fitness_fn(&new_solution);

            // Greedy selection
            if new_fitness > self.employed[i].1 {
                self.employed[i] = (new_solution, new_fitness);
                self.trials[i] = 0;

                if new_fitness > self.best_fitness {
                    self.best_fitness = new_fitness;
                    self.best_solution = self.employed[i].0.clone();
                }
            } else {
                self.trials[i] += 1;
            }
        }
    }

    /// Onlooker bee phase
    fn onlooker_phase<F>(&mut self, fitness_fn: &F, rng: &mut u64)
    where
        F: Fn(&[f64]) -> f64,
    {
        // Calculate selection probabilities
        let min_fitness = self
            .employed
            .iter()
            .map(|(_, f)| *f)
            .fold(f64::INFINITY, f64::min);

        let fitness_sum: f64 = self
            .employed
            .iter()
            .map(|(_, f)| f - min_fitness + 1.0)
            .sum();

        for _ in 0..self.num_onlookers {
            // Roulette wheel selection
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let r = (*rng as f64 / u64::MAX as f64) * fitness_sum;

            let mut cumulative = 0.0;
            let mut selected = 0;
            for (i, (_, fitness)) in self.employed.iter().enumerate() {
                cumulative += fitness - min_fitness + 1.0;
                if cumulative >= r {
                    selected = i;
                    break;
                }
            }

            // Similar to employed phase
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let dim = (*rng as usize) % self.dim;

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let mut k = (*rng as usize) % self.employed.len();
            while k == selected {
                *rng ^= *rng << 13;
                k = (*rng as usize) % self.employed.len();
            }

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let phi = (*rng as f64 / u64::MAX as f64) * 2.0 - 1.0;

            let mut new_solution = self.employed[selected].0.clone();
            new_solution[dim] = self.employed[selected].0[dim]
                + phi * (self.employed[selected].0[dim] - self.employed[k].0[dim]);
            new_solution[dim] = new_solution[dim].clamp(self.bounds.0, self.bounds.1);

            let new_fitness = fitness_fn(&new_solution);

            if new_fitness > self.employed[selected].1 {
                self.employed[selected] = (new_solution, new_fitness);
                self.trials[selected] = 0;

                if new_fitness > self.best_fitness {
                    self.best_fitness = new_fitness;
                    self.best_solution = self.employed[selected].0.clone();
                }
            } else {
                self.trials[selected] += 1;
            }
        }
    }

    /// Scout bee phase
    fn scout_phase(&mut self, rng: &mut u64) {
        for i in 0..self.employed.len() {
            if self.trials[i] > self.scout_limit {
                // Abandon and create new random solution
                let solution: Vec<f64> = (0..self.dim)
                    .map(|_| {
                        *rng ^= *rng << 13;
                        *rng ^= *rng >> 7;
                        *rng ^= *rng << 17;
                        let r = *rng as f64 / u64::MAX as f64;
                        self.bounds.0 + r * (self.bounds.1 - self.bounds.0)
                    })
                    .collect();

                self.employed[i] = (solution, f64::NEG_INFINITY);
                self.trials[i] = 0;
            }
        }
    }

    /// Run one iteration
    pub fn iterate<F>(&mut self, fitness_fn: &F, rng: &mut u64)
    where
        F: Fn(&[f64]) -> f64,
    {
        self.evaluate(fitness_fn);
        self.employed_phase(fitness_fn, rng);
        self.onlooker_phase(fitness_fn, rng);
        self.scout_phase(rng);
    }

    /// Get best solution
    pub fn best(&self) -> (&[f64], f64) {
        (&self.best_solution, self.best_fitness)
    }
}

/// Boid (bird-like agent for flocking)
#[derive(Debug, Clone)]
pub struct Boid {
    /// Position
    pub position: Vec2,
    /// Velocity
    pub velocity: Vec2,
    /// Acceleration
    pub acceleration: Vec2,
    /// Maximum speed
    pub max_speed: f64,
    /// Maximum force
    pub max_force: f64,
}

impl Boid {
    /// Create new boid
    pub fn new(position: Vec2, velocity: Vec2) -> Self {
        Self {
            position,
            velocity,
            acceleration: Vec2::zero(),
            max_speed: 4.0,
            max_force: 0.1,
        }
    }

    /// Apply steering force
    pub fn apply_force(&mut self, force: Vec2) {
        self.acceleration = self.acceleration.add(&force);
    }

    /// Update position and velocity
    pub fn update(&mut self) {
        self.velocity = self.velocity.add(&self.acceleration).limit(self.max_speed);
        self.position = self.position.add(&self.velocity);
        self.acceleration = Vec2::zero();
    }

    /// Seek target
    pub fn seek(&self, target: Vec2) -> Vec2 {
        let desired = target.sub(&self.position).normalize().scale(self.max_speed);
        desired.sub(&self.velocity).limit(self.max_force)
    }

    /// Flee from target
    pub fn flee(&self, target: Vec2) -> Vec2 {
        self.seek(target).scale(-1.0)
    }
}

/// Flock of boids
#[derive(Debug)]
pub struct Flock {
    /// Boids
    boids: Vec<Boid>,
    /// Separation weight
    separation_weight: f64,
    /// Alignment weight
    alignment_weight: f64,
    /// Cohesion weight
    cohesion_weight: f64,
    /// Perception radius
    perception_radius: f64,
}

impl Flock {
    /// Create new flock
    pub fn new(num_boids: usize, bounds: (Vec2, Vec2), rng: &mut u64) -> Self {
        let mut boids = Vec::with_capacity(num_boids);

        for _ in 0..num_boids {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let rx = *rng as f64 / u64::MAX as f64;

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let ry = *rng as f64 / u64::MAX as f64;

            let x = bounds.0.x + rx * (bounds.1.x - bounds.0.x);
            let y = bounds.0.y + ry * (bounds.1.y - bounds.0.y);

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let vx = (*rng as f64 / u64::MAX as f64) * 2.0 - 1.0;

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let vy = (*rng as f64 / u64::MAX as f64) * 2.0 - 1.0;

            boids.push(Boid::new(Vec2::new(x, y), Vec2::new(vx, vy)));
        }

        Self {
            boids,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            perception_radius: 50.0,
        }
    }

    /// Calculate separation (avoid crowding)
    fn separation(&self, boid_idx: usize) -> Vec2 {
        let mut steer = Vec2::zero();
        let mut count = 0;
        let boid = &self.boids[boid_idx];

        for (i, other) in self.boids.iter().enumerate() {
            if i == boid_idx {
                continue;
            }
            let d = boid.position.distance(&other.position);
            if d > 0.0 && d < self.perception_radius * 0.5 {
                let diff = boid
                    .position
                    .sub(&other.position)
                    .normalize()
                    .scale(1.0 / d);
                steer = steer.add(&diff);
                count += 1;
            }
        }

        if count > 0 {
            steer = steer.scale(1.0 / count as f64);
            steer = steer.normalize().scale(boid.max_speed);
            steer = steer.sub(&boid.velocity).limit(boid.max_force);
        }

        steer
    }

    /// Calculate alignment (steer towards average heading)
    fn alignment(&self, boid_idx: usize) -> Vec2 {
        let mut avg = Vec2::zero();
        let mut count = 0;
        let boid = &self.boids[boid_idx];

        for (i, other) in self.boids.iter().enumerate() {
            if i == boid_idx {
                continue;
            }
            let d = boid.position.distance(&other.position);
            if d > 0.0 && d < self.perception_radius {
                avg = avg.add(&other.velocity);
                count += 1;
            }
        }

        if count > 0 {
            avg = avg.scale(1.0 / count as f64);
            avg = avg.normalize().scale(boid.max_speed);
            avg.sub(&boid.velocity).limit(boid.max_force)
        } else {
            Vec2::zero()
        }
    }

    /// Calculate cohesion (steer towards center of mass)
    fn cohesion(&self, boid_idx: usize) -> Vec2 {
        let mut center = Vec2::zero();
        let mut count = 0;
        let boid = &self.boids[boid_idx];

        for (i, other) in self.boids.iter().enumerate() {
            if i == boid_idx {
                continue;
            }
            let d = boid.position.distance(&other.position);
            if d > 0.0 && d < self.perception_radius {
                center = center.add(&other.position);
                count += 1;
            }
        }

        if count > 0 {
            center = center.scale(1.0 / count as f64);
            boid.seek(center)
        } else {
            Vec2::zero()
        }
    }

    /// Update flock
    pub fn update(&mut self) {
        let mut forces: Vec<Vec2> = Vec::with_capacity(self.boids.len());

        for i in 0..self.boids.len() {
            let sep = self.separation(i).scale(self.separation_weight);
            let ali = self.alignment(i).scale(self.alignment_weight);
            let coh = self.cohesion(i).scale(self.cohesion_weight);
            forces.push(sep.add(&ali).add(&coh));
        }

        for (boid, force) in self.boids.iter_mut().zip(forces) {
            boid.apply_force(force);
            boid.update();
        }
    }

    /// Get flock center of mass
    pub fn center_of_mass(&self) -> Vec2 {
        let mut sum = Vec2::zero();
        for boid in &self.boids {
            sum = sum.add(&boid.position);
        }
        sum.scale(1.0 / self.boids.len() as f64)
    }

    /// Get flock average velocity
    pub fn average_velocity(&self) -> Vec2 {
        let mut sum = Vec2::zero();
        for boid in &self.boids {
            sum = sum.add(&boid.velocity);
        }
        sum.scale(1.0 / self.boids.len() as f64)
    }
}

/// Stigmergic environment for indirect coordination
#[derive(Debug)]
pub struct StigmergicEnvironment {
    /// Width
    width: usize,
    /// Height
    height: usize,
    /// Pheromone grids per type
    grids: BTreeMap<PheromoneType, Vec<f64>>,
    /// Decay rates per type
    decay_rates: BTreeMap<PheromoneType, f64>,
    /// Diffusion rates per type
    diffusion_rates: BTreeMap<PheromoneType, f64>,
}

impl StigmergicEnvironment {
    /// Create new environment
    pub fn new(width: usize, height: usize) -> Self {
        let mut env = Self {
            width,
            height,
            grids: BTreeMap::new(),
            decay_rates: BTreeMap::new(),
            diffusion_rates: BTreeMap::new(),
        };

        // Initialize default pheromone types
        for ptype in [
            PheromoneType::Attraction,
            PheromoneType::Resource,
            PheromoneType::Trail,
        ] {
            env.grids.insert(ptype, alloc::vec![0.0; width * height]);
            env.decay_rates.insert(ptype, 0.01);
            env.diffusion_rates.insert(ptype, 0.1);
        }

        env
    }

    /// Get index from coordinates
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Deposit pheromone
    pub fn deposit(&mut self, ptype: PheromoneType, x: usize, y: usize, amount: f64) {
        if x < self.width && y < self.height {
            let width = self.width;
            if let Some(grid) = self.grids.get_mut(&ptype) {
                let idx = y * width + x;
                grid[idx] = (grid[idx] + amount).min(100.0);
            }
        }
    }

    /// Read pheromone level
    pub fn read(&self, ptype: PheromoneType, x: usize, y: usize) -> f64 {
        if x < self.width && y < self.height {
            self.grids
                .get(&ptype)
                .map(|g| g[self.index(x, y)])
                .unwrap_or(0.0)
        } else {
            0.0
        }
    }

    /// Get gradient at position
    pub fn gradient(&self, ptype: PheromoneType, x: usize, y: usize) -> Vec2 {
        let left = if x > 0 {
            self.read(ptype, x - 1, y)
        } else {
            0.0
        };
        let right = if x < self.width - 1 {
            self.read(ptype, x + 1, y)
        } else {
            0.0
        };
        let up = if y > 0 {
            self.read(ptype, x, y - 1)
        } else {
            0.0
        };
        let down = if y < self.height - 1 {
            self.read(ptype, x, y + 1)
        } else {
            0.0
        };

        Vec2::new(right - left, down - up).normalize()
    }

    /// Update environment (decay and diffusion)
    pub fn update(&mut self, dt: f64) {
        let width = self.width;
        let height = self.height;

        // Collect pheromone types and their decay/diffusion rates before iterating
        let ptype_rates: alloc::vec::Vec<_> = self
            .grids
            .keys()
            .map(|&ptype| {
                let decay = self.decay_rates.get(&ptype).copied().unwrap_or(0.01);
                let diffusion = self.diffusion_rates.get(&ptype).copied().unwrap_or(0.1);
                (ptype, decay, diffusion)
            })
            .collect();

        for (ptype, decay, diffusion) in ptype_rates {
            if let Some(grid) = self.grids.get_mut(&ptype) {
                // Create copy for diffusion calculation
                let old_grid = grid.clone();

                for y in 0..height {
                    for x in 0..width {
                        let idx = y * width + x;

                        // Decay
                        grid[idx] *= 1.0 - decay * dt;

                        // Diffusion (Laplacian)
                        let mut laplacian = -4.0 * old_grid[idx];
                        if x > 0 {
                            laplacian += old_grid[y * width + (x - 1)];
                        }
                        if x < width - 1 {
                            laplacian += old_grid[y * width + (x + 1)];
                        }
                        if y > 0 {
                            laplacian += old_grid[(y - 1) * width + x];
                        }
                        if y < height - 1 {
                            laplacian += old_grid[(y + 1) * width + x];
                        }

                        grid[idx] += diffusion * laplacian * dt;
                        grid[idx] = grid[idx].max(0.0);
                    }
                }
            }
        }
    }

    /// Find strongest pheromone location
    pub fn find_peak(&self, ptype: PheromoneType) -> Option<(usize, usize, f64)> {
        self.grids.get(&ptype).and_then(|grid| {
            let mut max_val = 0.0;
            let mut max_pos = None;

            for (idx, &val) in grid.iter().enumerate() {
                if val > max_val {
                    max_val = val;
                    let x = idx % self.width;
                    let y = idx / self.width;
                    max_pos = Some((x, y, val));
                }
            }

            max_pos
        })
    }
}

/// Distributed consensus using swarm
#[derive(Debug)]
pub struct SwarmConsensus {
    /// Agent opinions (continuous value)
    opinions: Vec<f64>,
    /// Influence matrix
    influence: Vec<Vec<f64>>,
    /// Convergence threshold
    threshold: f64,
}

impl SwarmConsensus {
    /// Create new consensus instance
    pub fn new(num_agents: usize, initial_opinions: Vec<f64>) -> Self {
        let mut influence = alloc::vec![alloc::vec![0.0; num_agents]; num_agents];

        // Initialize with uniform influence
        for i in 0..num_agents {
            for j in 0..num_agents {
                influence[i][j] = 1.0 / num_agents as f64;
            }
        }

        Self {
            opinions: initial_opinions,
            influence,
            threshold: 0.01,
        }
    }

    /// Update opinions (average with neighbors)
    pub fn update(&mut self) {
        let old_opinions = self.opinions.clone();

        for i in 0..self.opinions.len() {
            let mut new_opinion = 0.0;
            for j in 0..self.opinions.len() {
                new_opinion += self.influence[i][j] * old_opinions[j];
            }
            self.opinions[i] = new_opinion;
        }
    }

    /// Check if consensus reached
    pub fn has_consensus(&self) -> bool {
        if self.opinions.is_empty() {
            return true;
        }

        let mean = self.opinions.iter().sum::<f64>() / self.opinions.len() as f64;
        self.opinions
            .iter()
            .all(|&o| (o - mean).abs() < self.threshold)
    }

    /// Get current consensus value
    pub fn consensus_value(&self) -> f64 {
        if self.opinions.is_empty() {
            0.0
        } else {
            self.opinions.iter().sum::<f64>() / self.opinions.len() as f64
        }
    }

    /// Run until consensus or max iterations
    pub fn run(&mut self, max_iterations: usize) -> (bool, usize) {
        for i in 0..max_iterations {
            self.update();
            if self.has_consensus() {
                return (true, i + 1);
            }
        }
        (false, max_iterations)
    }
}

/// Kernel swarm manager
pub struct KernelSwarmManager {
    /// Task assignment swarm (ACO)
    task_aco: Option<AntColony>,
    /// Resource optimization swarm (PSO)
    resource_pso: Option<ParticleSwarm>,
    /// Load balancing swarm (Bees)
    load_bees: Option<BeeColony>,
    /// Coordination flock (Boids)
    coord_flock: Option<Flock>,
    /// Environment for stigmergy
    environment: StigmergicEnvironment,
    /// Consensus mechanism
    consensus: Option<SwarmConsensus>,
    /// RNG state
    rng: u64,
}

impl KernelSwarmManager {
    /// Create new swarm manager
    pub fn new() -> Self {
        Self {
            task_aco: None,
            resource_pso: None,
            load_bees: None,
            coord_flock: None,
            environment: StigmergicEnvironment::new(100, 100),
            consensus: None,
            rng: 12345,
        }
    }

    /// Initialize task assignment with ACO
    pub fn init_task_aco(&mut self, costs: Vec<Vec<f64>>, num_ants: usize) {
        self.task_aco = Some(AntColony::new(costs, num_ants));
    }

    /// Initialize resource optimization with PSO
    pub fn init_resource_pso(&mut self, num_particles: usize, bounds: (Vec2, Vec2)) {
        self.resource_pso = Some(ParticleSwarm::new(num_particles, bounds, &mut self.rng));
    }

    /// Initialize load balancing with Bees
    pub fn init_load_bees(&mut self, num_bees: usize, dim: usize, bounds: (f64, f64)) {
        self.load_bees = Some(BeeColony::new(num_bees, dim, bounds, &mut self.rng));
    }

    /// Initialize coordination flock
    pub fn init_coordination(&mut self, num_agents: usize, bounds: (Vec2, Vec2)) {
        self.coord_flock = Some(Flock::new(num_agents, bounds, &mut self.rng));
    }

    /// Initialize consensus
    pub fn init_consensus(&mut self, initial_opinions: Vec<f64>) {
        self.consensus = Some(SwarmConsensus::new(
            initial_opinions.len(),
            initial_opinions,
        ));
    }

    /// Run task assignment iteration
    pub fn assign_tasks(&mut self) -> Option<(Vec<usize>, f64)> {
        if let Some(aco) = &mut self.task_aco {
            aco.iterate(&mut self.rng);
            let (tour, length) = aco.best();
            Some((tour.to_vec(), length))
        } else {
            None
        }
    }

    /// Run resource optimization
    pub fn optimize_resources<F>(&mut self, fitness_fn: F, iterations: usize) -> Option<(Vec2, f64)>
    where
        F: Fn(&Vec2) -> f64,
    {
        if let Some(pso) = &mut self.resource_pso {
            pso.optimize(fitness_fn, iterations, &mut self.rng);
            Some(pso.best())
        } else {
            None
        }
    }

    /// Signal resource at location
    pub fn signal_resource(&mut self, x: usize, y: usize, strength: f64) {
        self.environment
            .deposit(PheromoneType::Resource, x, y, strength);
    }

    /// Find best resource location
    pub fn find_best_resource(&self) -> Option<(usize, usize)> {
        self.environment
            .find_peak(PheromoneType::Resource)
            .map(|(x, y, _)| (x, y))
    }

    /// Update environment
    pub fn update_environment(&mut self, dt: f64) {
        self.environment.update(dt);
    }

    /// Reach consensus on value
    pub fn reach_consensus(&mut self, max_iterations: usize) -> Option<(bool, f64)> {
        if let Some(cons) = &mut self.consensus {
            let (reached, _) = cons.run(max_iterations);
            Some((reached, cons.consensus_value()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2() {
        let a = Vec2::new(3.0, 4.0);
        assert!((a.magnitude() - 5.0).abs() < 0.001);

        let b = Vec2::new(1.0, 2.0);
        let sum = a.add(&b);
        assert!((sum.x - 4.0).abs() < 0.001);
        assert!((sum.y - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_ant_colony() {
        // Small TSP instance
        let distances = vec![
            vec![0.0, 10.0, 15.0, 20.0],
            vec![10.0, 0.0, 35.0, 25.0],
            vec![15.0, 35.0, 0.0, 30.0],
            vec![20.0, 25.0, 30.0, 0.0],
        ];

        let mut aco = AntColony::new(distances, 10);
        let mut rng = 12345u64;

        for _ in 0..100 {
            aco.iterate(&mut rng);
        }

        let (tour, length) = aco.best();
        assert_eq!(tour.len(), 4);
        assert!(length < f64::INFINITY);
    }

    #[test]
    fn test_particle_swarm() {
        let mut rng = 12345u64;
        let bounds = (Vec2::new(-10.0, -10.0), Vec2::new(10.0, 10.0));
        let mut pso = ParticleSwarm::new(20, bounds, &mut rng);

        // Sphere function: minimum at origin
        let sphere = |p: &Vec2| -(p.x * p.x + p.y * p.y);

        pso.optimize(sphere, 100, &mut rng);

        let (best_pos, _) = pso.best();
        // Should be close to origin
        assert!(best_pos.magnitude() < 1.0);
    }

    #[test]
    fn test_bee_colony() {
        let mut rng = 12345u64;
        let mut bees = BeeColony::new(20, 2, (-10.0, 10.0), &mut rng);

        // Sphere function
        let sphere = |x: &[f64]| -(x[0] * x[0] + x[1] * x[1]);

        for _ in 0..100 {
            bees.iterate(&sphere, &mut rng);
        }

        let (best, fitness) = bees.best();
        assert!(fitness > -1.0); // Close to optimum
    }

    #[test]
    fn test_flock() {
        let mut rng = 12345u64;
        let bounds = (Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        let mut flock = Flock::new(10, bounds, &mut rng);

        for _ in 0..50 {
            flock.update();
        }

        // Flock should have some coherent motion
        let avg_vel = flock.average_velocity();
        assert!(avg_vel.magnitude() > 0.0);
    }

    #[test]
    fn test_stigmergic_environment() {
        let mut env = StigmergicEnvironment::new(10, 10);

        env.deposit(PheromoneType::Resource, 5, 5, 10.0);
        assert!(env.read(PheromoneType::Resource, 5, 5) > 0.0);

        // Update should diffuse
        for _ in 0..10 {
            env.update(0.1);
        }

        assert!(env.read(PheromoneType::Resource, 4, 5) > 0.0);
    }

    #[test]
    fn test_swarm_consensus() {
        let opinions = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut consensus = SwarmConsensus::new(5, opinions);

        let (reached, _) = consensus.run(1000);

        assert!(reached);
        assert!((consensus.consensus_value() - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_kernel_swarm_manager() {
        let mut manager = KernelSwarmManager::new();

        manager.signal_resource(50, 50, 100.0);
        manager.update_environment(0.1);

        let peak = manager.find_best_resource();
        assert!(peak.is_some());
        let (x, y) = peak.unwrap();
        assert_eq!(x, 50);
        assert_eq!(y, 50);
    }
}
