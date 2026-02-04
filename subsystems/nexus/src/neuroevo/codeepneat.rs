//! CoDeepNEAT: Coevolution of deep neural network modules.

use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::utils::lcg_next;

/// A module blueprint in CoDeepNEAT
#[derive(Debug, Clone)]
pub struct ModuleBlueprint {
    /// Module ID
    pub id: u64,
    /// Number of layers in this module
    pub num_layers: usize,
    /// Layer configurations: (neurons, activation)
    pub layers: Vec<(usize, ActivationFunction)>,
    /// Skip connections within module
    pub skip_connections: Vec<(usize, usize)>,
    /// Fitness contribution
    pub fitness: f64,
    /// Age (generations)
    pub age: u32,
}

impl ModuleBlueprint {
    /// Create a new module blueprint
    pub fn new(id: u64, seed: u64) -> Self {
        let num_layers = ((seed % 5) + 1) as usize; // 1-5 layers
        let mut layers = Vec::with_capacity(num_layers);
        let mut rng = seed;

        for _ in 0..num_layers {
            rng = lcg_next(rng);
            let neurons = ((rng % 64) + 4) as usize; // 4-67 neurons
            rng = lcg_next(rng);
            let activation = ActivationFunction::random(rng);
            layers.push((neurons, activation));
        }

        Self {
            id,
            num_layers,
            layers,
            skip_connections: Vec::new(),
            fitness: 0.0,
            age: 0,
        }
    }

    /// Mutate this blueprint
    pub fn mutate(&mut self, seed: u64) {
        let mut rng = seed;

        // Mutate layer sizes
        for (neurons, _) in &mut self.layers {
            rng = lcg_next(rng);
            if rng % 100 < 30 {
                rng = lcg_next(rng);
                let delta = (rng % 8) as i64 - 4; // -4 to +3
                *neurons = ((*neurons as i64 + delta).max(1)) as usize;
            }
        }

        // Mutate activations
        for (_, activation) in &mut self.layers {
            rng = lcg_next(rng);
            if rng % 100 < 10 {
                *activation = ActivationFunction::random(rng);
            }
        }

        // Add/remove layers
        rng = lcg_next(rng);
        if rng % 100 < 5 && self.layers.len() < 10 {
            rng = lcg_next(rng);
            let neurons = ((rng % 64) + 4) as usize;
            rng = lcg_next(rng);
            let activation = ActivationFunction::random(rng);
            self.layers.push((neurons, activation));
        }

        if rng % 100 < 5 && self.layers.len() > 1 {
            self.layers.pop();
        }
    }
}

/// A complete network blueprint
#[derive(Debug, Clone)]
pub struct NetworkBlueprint {
    /// Blueprint ID
    pub id: u64,
    /// Module IDs in order
    pub modules: Vec<u64>,
    /// Connections between modules
    pub connections: Vec<(usize, usize)>,
    /// Fitness
    pub fitness: f64,
    /// Age
    pub age: u32,
}

impl NetworkBlueprint {
    /// Create a new network blueprint
    pub fn new(id: u64, module_ids: Vec<u64>) -> Self {
        // Create sequential connections
        let mut connections = Vec::new();
        for i in 0..module_ids.len().saturating_sub(1) {
            connections.push((i, i + 1));
        }

        Self {
            id,
            modules: module_ids,
            connections,
            fitness: 0.0,
            age: 0,
        }
    }

    /// Mutate this blueprint
    pub fn mutate(&mut self, available_modules: &[u64], seed: u64) {
        let mut rng = seed;

        // Swap modules
        for module in &mut self.modules {
            rng = lcg_next(rng);
            if rng % 100 < 20 && !available_modules.is_empty() {
                rng = lcg_next(rng);
                *module = available_modules[rng as usize % available_modules.len()];
            }
        }

        // Add module
        rng = lcg_next(rng);
        if rng % 100 < 10 && !available_modules.is_empty() && self.modules.len() < 10 {
            rng = lcg_next(rng);
            let new_module = available_modules[rng as usize % available_modules.len()];
            let pos = rng as usize % (self.modules.len() + 1);
            self.modules.insert(pos, new_module);

            // Update connections
            self.connections.clear();
            for i in 0..self.modules.len().saturating_sub(1) {
                self.connections.push((i, i + 1));
            }
        }

        // Remove module
        if rng % 100 < 10 && self.modules.len() > 1 {
            rng = lcg_next(rng);
            let pos = rng as usize % self.modules.len();
            self.modules.remove(pos);

            self.connections.clear();
            for i in 0..self.modules.len().saturating_sub(1) {
                self.connections.push((i, i + 1));
            }
        }
    }
}

/// CoDeepNEAT: Coevolution of Deep Network Modules
pub struct CoDeepNeat {
    /// Module population
    pub modules: Vec<ModuleBlueprint>,
    /// Network population
    pub networks: Vec<NetworkBlueprint>,
    /// Module population size
    pub module_pop_size: usize,
    /// Network population size
    pub network_pop_size: usize,
    /// Generation counter
    pub generation: u32,
    /// Next module ID
    next_module_id: u64,
    /// Next network ID
    next_network_id: u64,
    /// Random seed
    seed: u64,
}

impl CoDeepNeat {
    /// Create a new CoDeepNEAT instance
    pub fn new(module_pop_size: usize, network_pop_size: usize, seed: u64) -> Self {
        let mut rng = seed;
        let mut modules = Vec::with_capacity(module_pop_size);

        for i in 0..module_pop_size {
            rng = lcg_next(rng);
            modules.push(ModuleBlueprint::new(i as u64, rng));
        }

        let module_ids: Vec<u64> = modules.iter().map(|m| m.id).collect();
        let mut networks = Vec::with_capacity(network_pop_size);

        for i in 0..network_pop_size {
            rng = lcg_next(rng);
            let num_modules = ((rng % 3) + 1) as usize;
            let mut net_modules = Vec::with_capacity(num_modules);
            for _ in 0..num_modules {
                rng = lcg_next(rng);
                net_modules.push(module_ids[rng as usize % module_ids.len()]);
            }
            networks.push(NetworkBlueprint::new(i as u64, net_modules));
        }

        Self {
            modules,
            networks,
            module_pop_size,
            network_pop_size,
            generation: 0,
            next_module_id: module_pop_size as u64,
            next_network_id: network_pop_size as u64,
            seed: rng,
        }
    }

    /// Evolve both populations
    pub fn evolve(&mut self) {
        self.generation += 1;

        // Age all individuals
        for module in &mut self.modules {
            module.age += 1;
        }
        for network in &mut self.networks {
            network.age += 1;
        }

        // Sort by fitness
        self.modules
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        self.networks
            .sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

        // Evolve modules
        let module_survivors = self.module_pop_size / 2;
        self.modules.truncate(module_survivors);

        while self.modules.len() < self.module_pop_size {
            self.seed = lcg_next(self.seed);
            let parent_idx = self.seed as usize % module_survivors;
            let mut offspring = self.modules[parent_idx].clone();
            offspring.id = self.next_module_id;
            self.next_module_id += 1;
            offspring.age = 0;
            self.seed = lcg_next(self.seed);
            offspring.mutate(self.seed);
            self.modules.push(offspring);
        }

        // Evolve networks
        let network_survivors = self.network_pop_size / 2;
        self.networks.truncate(network_survivors);

        let module_ids: Vec<u64> = self.modules.iter().map(|m| m.id).collect();
        while self.networks.len() < self.network_pop_size {
            self.seed = lcg_next(self.seed);
            let parent_idx = self.seed as usize % network_survivors;
            let mut offspring = self.networks[parent_idx].clone();
            offspring.id = self.next_network_id;
            self.next_network_id += 1;
            offspring.age = 0;
            self.seed = lcg_next(self.seed);
            offspring.mutate(&module_ids, self.seed);
            self.networks.push(offspring);
        }
    }

    /// Get the best network
    pub fn get_best(&self) -> Option<&NetworkBlueprint> {
        self.networks
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
    }
}
