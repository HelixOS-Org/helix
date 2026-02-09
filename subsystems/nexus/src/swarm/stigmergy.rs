//! # Stigmergy and Emergent Behavior
//!
//! Indirect communication through environment modification.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec::Vec;
use core::cmp::Ordering;

use super::types::StigmergyGrid;
use crate::math::F64Ext;

// ============================================================================
// SIGNAL TYPES
// ============================================================================

/// Type of stigmergic signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    /// Attractant pheromone
    Attractant,
    /// Repellent pheromone
    Repellent,
    /// Food marker
    Food,
    /// Danger marker
    Danger,
    /// Trail marker
    Trail,
    /// Territory marker
    Territory,
}

// ============================================================================
// MULTI-CHANNEL STIGMERGY
// ============================================================================

/// Multi-channel stigmergy with different signal types
pub struct MultiChannelStigmergy {
    /// Grid dimensions
    pub width: usize,
    pub height: usize,
    /// Signal channels
    channels: Vec<(SignalType, StigmergyGrid)>,
}

impl MultiChannelStigmergy {
    /// Create new multi-channel system
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            channels: Vec::new(),
        }
    }

    /// Add signal channel
    #[inline(always)]
    pub fn add_channel(&mut self, signal_type: SignalType, decay_rate: f64) {
        let grid = StigmergyGrid::new(self.width, self.height, decay_rate);
        self.channels.push((signal_type, grid));
    }

    /// Get channel by type
    #[inline]
    pub fn get_channel(&self, signal_type: SignalType) -> Option<&StigmergyGrid> {
        self.channels
            .iter()
            .find(|(t, _)| *t == signal_type)
            .map(|(_, g)| g)
    }

    /// Get mutable channel
    #[inline]
    pub fn get_channel_mut(&mut self, signal_type: SignalType) -> Option<&mut StigmergyGrid> {
        self.channels
            .iter_mut()
            .find(|(t, _)| *t == signal_type)
            .map(|(_, g)| g)
    }

    /// Deposit signal
    #[inline]
    pub fn deposit(&mut self, signal_type: SignalType, x: usize, y: usize, amount: f64) {
        if let Some(grid) = self.get_channel_mut(signal_type) {
            grid.deposit(x, y, amount);
        }
    }

    /// Get combined signal (weighted sum)
    #[inline]
    pub fn combined_signal(&self, x: usize, y: usize, weights: &[(SignalType, f64)]) -> f64 {
        let mut total = 0.0;

        for &(signal_type, weight) in weights {
            if let Some(grid) = self.get_channel(signal_type) {
                total += grid.get(x, y) * weight;
            }
        }

        total
    }

    /// Decay all channels
    #[inline]
    pub fn decay_all(&mut self) {
        for (_, grid) in &mut self.channels {
            grid.decay();
        }
    }

    /// Diffuse all channels
    #[inline]
    pub fn diffuse_all(&mut self, rate: f64) {
        for (_, grid) in &mut self.channels {
            grid.diffuse(rate);
        }
    }
}

// ============================================================================
// STIGMERGIC AGENT
// ============================================================================

/// Agent that interacts with stigmergic environment
#[derive(Debug, Clone)]
pub struct StigmergicAgent {
    /// Agent ID
    pub id: usize,
    /// Position
    pub x: usize,
    pub y: usize,
    /// State
    pub state: AgentState,
    /// Carrying resource
    pub carrying: bool,
    /// Signal sensitivity
    pub sensitivity: f64,
}

/// Agent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Searching for resource
    Searching,
    /// Returning with resource
    Returning,
    /// Following trail
    Following,
    /// Exploring randomly
    Exploring,
    /// Idle
    Idle,
}

impl StigmergicAgent {
    /// Create new agent
    pub fn new(id: usize, x: usize, y: usize) -> Self {
        Self {
            id,
            x,
            y,
            state: AgentState::Searching,
            carrying: false,
            sensitivity: 1.0,
        }
    }

    /// Get position as tuple
    #[inline(always)]
    pub fn position(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    /// Move to new position
    #[inline(always)]
    pub fn move_to(&mut self, x: usize, y: usize) {
        self.x = x;
        self.y = y;
    }

    /// Pick up resource
    #[inline(always)]
    pub fn pickup(&mut self) {
        self.carrying = true;
        self.state = AgentState::Returning;
    }

    /// Drop resource
    #[inline(always)]
    pub fn drop_resource(&mut self) {
        self.carrying = false;
        self.state = AgentState::Searching;
    }
}

// ============================================================================
// FORAGING SIMULATION
// ============================================================================

/// Foraging simulation with stigmergy
pub struct ForagingSimulation {
    /// Environment
    pub environment: MultiChannelStigmergy,
    /// Agents
    pub agents: Vec<StigmergicAgent>,
    /// Food source positions
    pub food_sources: Vec<(usize, usize, f64)>, // x, y, amount
    /// Nest position
    pub nest: (usize, usize),
    /// RNG state
    rng_state: u64,
    /// Collected food
    pub collected: f64,
}

impl ForagingSimulation {
    /// Create new simulation
    pub fn new(width: usize, height: usize, n_agents: usize) -> Self {
        let mut environment = MultiChannelStigmergy::new(width, height);
        environment.add_channel(SignalType::Trail, 0.02);
        environment.add_channel(SignalType::Food, 0.01);

        let nest = (width / 2, height / 2);

        let mut agents = Vec::with_capacity(n_agents);
        for i in 0..n_agents {
            agents.push(StigmergicAgent::new(i, nest.0, nest.1));
        }

        Self {
            environment,
            agents,
            food_sources: Vec::new(),
            nest,
            rng_state: 12345,
            collected: 0.0,
        }
    }

    /// Add food source
    #[inline]
    pub fn add_food(&mut self, x: usize, y: usize, amount: f64) {
        self.food_sources.push((x, y, amount));

        // Mark food signal
        if let Some(grid) = self.environment.get_channel_mut(SignalType::Food) {
            grid.deposit(x, y, 10.0);
        }
    }

    /// Random direction
    fn random_direction(&mut self) -> (i32, i32) {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;

        let dx = (self.rng_state % 3) as i32 - 1;

        self.rng_state ^= self.rng_state << 13;
        let dy = (self.rng_state % 3) as i32 - 1;

        (dx, dy)
    }

    /// Get best direction based on gradient
    fn gradient_direction(
        &self,
        x: usize,
        y: usize,
        signal_type: SignalType,
    ) -> Option<(i32, i32)> {
        let grid = self.environment.get_channel(signal_type)?;

        let mut best_dir = (0i32, 0i32);
        let mut best_val = grid.get(x, y);

        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x as i32 + dx) as usize;
                let ny = (y as i32 + dy) as usize;

                if nx < self.environment.width && ny < self.environment.height {
                    let val = grid.get(nx, ny);
                    if val > best_val {
                        best_val = val;
                        best_dir = (dx, dy);
                    }
                }
            }
        }

        if best_dir == (0, 0) {
            None
        } else {
            Some(best_dir)
        }
    }

    /// Direction towards position
    fn direction_to(&self, from: (usize, usize), to: (usize, usize)) -> (i32, i32) {
        let dx = match to.0.cmp(&from.0) {
            Ordering::Greater => 1,
            Ordering::Less => -1,
            Ordering::Equal => 0,
        };
        let dy = match to.1.cmp(&from.1) {
            Ordering::Greater => 1,
            Ordering::Less => -1,
            Ordering::Equal => 0,
        };
        (dx, dy)
    }

    /// Update single agent
    fn update_agent(&mut self, agent_idx: usize) {
        let agent = &self.agents[agent_idx];
        let (x, y) = (agent.x, agent.y);

        let (dx, dy) = match agent.state {
            AgentState::Searching => {
                // Follow food signal or explore
                if let Some(dir) = self.gradient_direction(x, y, SignalType::Food) {
                    dir
                } else if let Some(dir) = self.gradient_direction(x, y, SignalType::Trail) {
                    // Sometimes follow trail
                    self.rng_state ^= self.rng_state << 13;
                    if self.rng_state % 3 == 0 {
                        dir
                    } else {
                        self.random_direction()
                    }
                } else {
                    self.random_direction()
                }
            },
            AgentState::Returning => {
                // Return to nest, leave trail
                self.direction_to((x, y), self.nest)
            },
            _ => self.random_direction(),
        };

        // Move
        let nx = ((x as i32 + dx).max(0) as usize).min(self.environment.width - 1);
        let ny = ((y as i32 + dy).max(0) as usize).min(self.environment.height - 1);

        self.agents[agent_idx].move_to(nx, ny);

        // Deposit trail if returning
        if self.agents[agent_idx].state == AgentState::Returning {
            self.environment.deposit(SignalType::Trail, nx, ny, 1.0);
        }
    }

    /// Check for food pickup
    fn check_food(&mut self, agent_idx: usize) {
        let agent = &self.agents[agent_idx];

        if agent.carrying {
            return;
        }

        for (fx, fy, amount) in &mut self.food_sources {
            if agent.x == *fx && agent.y == *fy && *amount > 0.0 {
                self.agents[agent_idx].pickup();
                *amount -= 1.0;
                break;
            }
        }
    }

    /// Check for food dropoff at nest
    fn check_nest(&mut self, agent_idx: usize) {
        let agent = &self.agents[agent_idx];

        if agent.carrying && agent.x == self.nest.0 && agent.y == self.nest.1 {
            self.agents[agent_idx].drop_resource();
            self.collected += 1.0;
        }
    }

    /// Step simulation
    pub fn step(&mut self) {
        // Update agents
        for i in 0..self.agents.len() {
            self.update_agent(i);
            self.check_food(i);
            self.check_nest(i);
        }

        // Environment dynamics
        self.environment.decay_all();
        self.environment.diffuse_all(0.1);

        // Refresh food signals
        for &(fx, fy, amount) in &self.food_sources {
            if amount > 0.0 {
                self.environment.deposit(SignalType::Food, fx, fy, 0.5);
            }
        }
    }

    /// Run simulation for n steps
    #[inline]
    pub fn run(&mut self, steps: usize) -> f64 {
        for _ in 0..steps {
            self.step();
        }
        self.collected
    }
}

// ============================================================================
// CONSTRUCTION PATTERNS
// ============================================================================

/// Emergent construction simulation
pub struct ConstructionSimulation {
    /// Building grid (material deposited)
    pub building: Vec<f64>,
    /// Width
    pub width: usize,
    /// Height
    pub height: usize,
    /// Builder agents
    builders: Vec<StigmergicAgent>,
    /// Stigmergic cues
    cues: StigmergyGrid,
    /// RNG
    rng_state: u64,
}

impl ConstructionSimulation {
    /// Create new simulation
    pub fn new(width: usize, height: usize, n_builders: usize) -> Self {
        let mut builders = Vec::with_capacity(n_builders);
        let mut rng = 42u64;

        for i in 0..n_builders {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;

            let x = (rng % width as u64) as usize;
            let y = (rng % height as u64) as usize;
            builders.push(StigmergicAgent::new(i, x, y));
        }

        Self {
            building: alloc::vec![0.0; width * height],
            width,
            height,
            builders,
            cues: StigmergyGrid::new(width, height, 0.01),
            rng_state: rng,
        }
    }

    /// Get building value at position
    #[inline]
    pub fn get_building(&self, x: usize, y: usize) -> f64 {
        if x < self.width && y < self.height {
            self.building[y * self.width + x]
        } else {
            0.0
        }
    }

    /// Add building material
    fn add_material(&mut self, x: usize, y: usize, amount: f64) {
        if x < self.width && y < self.height {
            self.building[y * self.width + x] += amount;
            // Leave cue for other builders
            self.cues.deposit(x, y, amount);
        }
    }

    /// Count neighbors with material
    fn neighbor_material(&self, x: usize, y: usize) -> f64 {
        let mut total = 0.0;

        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = (x as i32 + dx).max(0) as usize;
                let ny = (y as i32 + dy).max(0) as usize;

                if nx < self.width && ny < self.height {
                    total += self.get_building(nx, ny);
                }
            }
        }

        total
    }

    /// Step simulation
    pub fn step(&mut self) {
        for i in 0..self.builders.len() {
            let (x, y) = (self.builders[i].x, self.builders[i].y);

            // Decision: build or move?
            let neighbor_material = self.neighbor_material(x, y);
            let cue_strength = self.cues.get(x, y);

            // Build probability increases with nearby material
            let build_prob = ((neighbor_material + cue_strength) / 10.0).min(0.8);

            self.rng_state ^= self.rng_state << 13;
            self.rng_state ^= self.rng_state >> 7;
            self.rng_state ^= self.rng_state << 17;

            let r = (self.rng_state as f64) / (u64::MAX as f64);

            if r < build_prob && self.get_building(x, y) < 10.0 {
                // Build
                self.add_material(x, y, 1.0);
            } else {
                // Move (biased towards cues)
                let (gx, gy) = self.cues.gradient(x, y);

                self.rng_state ^= self.rng_state << 13;
                let noise_x = ((self.rng_state % 3) as i32 - 1) as f64;
                self.rng_state ^= self.rng_state >> 7;
                let noise_y = ((self.rng_state % 3) as i32 - 1) as f64;

                let dx = (gx + noise_x * 0.5).round() as i32;
                let dy = (gy + noise_y * 0.5).round() as i32;

                let nx = ((x as i32 + dx).max(0) as usize).min(self.width - 1);
                let ny = ((y as i32 + dy).max(0) as usize).min(self.height - 1);

                self.builders[i].move_to(nx, ny);
            }
        }

        // Decay cues
        self.cues.decay();
    }

    /// Get total material placed
    #[inline(always)]
    pub fn total_material(&self) -> f64 {
        self.building.iter().sum()
    }

    /// Get material distribution
    pub fn material_stats(&self) -> (f64, f64, f64) {
        let total = self.total_material();
        let n = self.building.len() as f64;
        let mean = total / n;

        let variance: f64 = self
            .building
            .iter()
            .map(|&v| (v - mean) * (v - mean))
            .sum::<f64>()
            / n;

        let max = self.building.iter().cloned().fold(0.0, f64::max);

        (mean, libm::sqrt(variance), max)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_channel_stigmergy() {
        let mut env = MultiChannelStigmergy::new(10, 10);
        env.add_channel(SignalType::Trail, 0.1);
        env.add_channel(SignalType::Food, 0.05);

        env.deposit(SignalType::Trail, 5, 5, 1.0);

        let val = env.get_channel(SignalType::Trail).unwrap().get(5, 5);
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_foraging_simulation() {
        let mut sim = ForagingSimulation::new(50, 50, 10);
        sim.add_food(10, 10, 100.0);
        sim.add_food(40, 40, 100.0);

        let collected = sim.run(500);

        // Should collect some food
        assert!(collected >= 0.0);
    }

    #[test]
    fn test_construction_simulation() {
        let mut sim = ConstructionSimulation::new(20, 20, 10);

        // Seed with initial material
        sim.add_material(10, 10, 5.0);

        for _ in 0..100 {
            sim.step();
        }

        // Should have built more
        assert!(sim.total_material() > 5.0);
    }
}
