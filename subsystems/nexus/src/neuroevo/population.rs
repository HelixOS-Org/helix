//! NEAT population management.

use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::genome::NeatGenome;
use crate::neuroevo::innovation::InnovationTracker;
use crate::neuroevo::network::NeatNetwork;
use crate::neuroevo::species::Species;
use crate::neuroevo::utils::lcg_next;

/// Configuration for NEAT
#[derive(Debug, Clone)]
pub struct NeatConfig {
    /// Population size
    pub population_size: usize,
    /// Number of inputs
    pub num_inputs: usize,
    /// Number of outputs
    pub num_outputs: usize,
    /// Output activation function
    pub output_activation: ActivationFunction,
    /// Compatibility threshold for speciation
    pub compatibility_threshold: f64,
    /// Probability of adding a connection
    pub add_connection_prob: f64,
    /// Probability of adding a node
    pub add_node_prob: f64,
    /// Probability of mutating weights
    pub weight_mutation_prob: f64,
    /// Weight perturbation power
    pub weight_perturbation: f64,
    /// Elitism - number of best genomes to preserve
    pub elitism: usize,
    /// Survival threshold within species
    pub survival_threshold: f64,
    /// Stagnation limit before species is killed
    pub stagnation_limit: u32,
    /// Target number of species
    pub target_species: usize,
}

impl Default for NeatConfig {
    fn default() -> Self {
        Self {
            population_size: 150,
            num_inputs: 4,
            num_outputs: 2,
            output_activation: ActivationFunction::Tanh,
            compatibility_threshold: 3.0,
            add_connection_prob: 0.05,
            add_node_prob: 0.03,
            weight_mutation_prob: 0.8,
            weight_perturbation: 0.5,
            elitism: 2,
            survival_threshold: 0.2,
            stagnation_limit: 15,
            target_species: 10,
        }
    }
}

/// NEAT Population manager
pub struct NeatPopulation {
    /// Configuration
    pub config: NeatConfig,
    /// All genomes
    pub genomes: Vec<NeatGenome>,
    /// Species list
    pub species: Vec<Species>,
    /// Innovation tracker
    pub tracker: InnovationTracker,
    /// Current generation
    pub generation: u32,
    /// Best genome ever
    pub best_genome: Option<NeatGenome>,
    /// Best fitness ever
    pub best_fitness: f64,
    /// Random seed
    seed: u64,
}

impl NeatPopulation {
    /// Create a new NEAT population
    pub fn new(config: NeatConfig, seed: u64) -> Self {
        let mut tracker = InnovationTracker::new();
        let mut genomes = Vec::with_capacity(config.population_size);

        // Create initial population
        for _ in 0..config.population_size {
            let genome = NeatGenome::minimal(
                config.num_inputs,
                config.num_outputs,
                config.output_activation,
                &mut tracker,
            );
            genomes.push(genome);
        }

        Self {
            config,
            genomes,
            species: Vec::new(),
            tracker,
            generation: 0,
            best_genome: None,
            best_fitness: f64::NEG_INFINITY,
            seed,
        }
    }

    /// Speciate the population
    pub fn speciate(&mut self) {
        // Clear member lists
        for species in &mut self.species {
            species.clear_members();
        }

        // Assign genomes to species
        for (i, genome) in self.genomes.iter_mut().enumerate() {
            let mut found_species = false;

            for species in &mut self.species {
                if species.is_compatible(genome, self.config.compatibility_threshold) {
                    species.add_member(i);
                    genome.species_id = species.id;
                    found_species = true;
                    break;
                }
            }

            if !found_species {
                // Create new species
                let new_species_id = self.species.len();
                let mut new_species = Species::new(new_species_id, genome.clone());
                new_species.add_member(i);
                genome.species_id = new_species_id;
                self.species.push(new_species);
            }
        }

        // Remove empty species
        self.species.retain(|s| !s.members.is_empty());

        // Adjust compatibility threshold to target species count
        if self.species.len() < self.config.target_species {
            self.config.compatibility_threshold -= 0.3;
        } else if self.species.len() > self.config.target_species {
            self.config.compatibility_threshold += 0.3;
        }
        self.config.compatibility_threshold = self.config.compatibility_threshold.clamp(1.0, 10.0);
    }

    /// Calculate adjusted fitness for each genome
    pub fn calculate_adjusted_fitness(&mut self) {
        for species in &self.species {
            let species_size = species.members.len() as f64;
            for &idx in &species.members {
                self.genomes[idx].adjusted_fitness = self.genomes[idx].fitness / species_size;
            }
        }

        // Update species stats
        let genomes = &self.genomes;
        for species in &mut self.species {
            species.update_stats(genomes);
        }
    }

    /// Create next generation through selection and reproduction
    pub fn evolve(&mut self) {
        self.generation += 1;
        self.tracker.new_generation();

        // Calculate offspring allocation for each species
        let total_adjusted: f64 = self.genomes.iter().map(|g| g.adjusted_fitness).sum();

        if total_adjusted <= 0.0 {
            return;
        }

        for species in &mut self.species {
            let species_adjusted: f64 = species
                .members
                .iter()
                .map(|&i| self.genomes[i].adjusted_fitness)
                .sum();
            species.offspring = ((species_adjusted / total_adjusted)
                * self.config.population_size as f64)
                .round() as usize;
        }

        // Remove stagnant species (keep at least 2)
        if self.species.len() > 2 {
            self.species
                .retain(|s| s.stagnation < self.config.stagnation_limit);
        }

        // Create offspring
        let mut new_genomes = Vec::with_capacity(self.config.population_size);

        // Elitism: keep best genome overall
        if let Some(ref best) = self.best_genome {
            new_genomes.push(best.clone());
        }

        for species in &self.species {
            if species.members.is_empty() {
                continue;
            }

            // Sort members by fitness
            let mut sorted_members = species.members.clone();
            sorted_members.sort_by(|&a, &b| {
                self.genomes[b]
                    .fitness
                    .partial_cmp(&self.genomes[a].fitness)
                    .unwrap()
            });

            // Keep only top survivors
            let survivors =
                (sorted_members.len() as f64 * self.config.survival_threshold).ceil() as usize;
            let survivors = survivors.max(1);
            sorted_members.truncate(survivors);

            // Elitism within species
            for i in 0..self.config.elitism.min(sorted_members.len()) {
                if new_genomes.len() < self.config.population_size {
                    new_genomes.push(self.genomes[sorted_members[i]].clone());
                }
            }

            // Create offspring for this species
            for _ in 0..species.offspring {
                if new_genomes.len() >= self.config.population_size {
                    break;
                }

                self.seed = lcg_next(self.seed);
                let parent1_idx = sorted_members[self.seed as usize % sorted_members.len()];

                self.seed = lcg_next(self.seed);
                let offspring = if self.seed % 100 < 75 && sorted_members.len() > 1 {
                    // Crossover
                    self.seed = lcg_next(self.seed);
                    let parent2_idx = sorted_members[self.seed as usize % sorted_members.len()];
                    NeatGenome::crossover(
                        &self.genomes[parent1_idx],
                        &self.genomes[parent2_idx],
                        self.seed,
                    )
                } else {
                    // Asexual reproduction
                    self.genomes[parent1_idx].clone()
                };

                new_genomes.push(offspring);
            }
        }

        // Fill remaining slots with random offspring
        while new_genomes.len() < self.config.population_size {
            self.seed = lcg_next(self.seed);
            let idx = self.seed as usize % self.genomes.len();
            new_genomes.push(self.genomes[idx].clone());
        }

        // Mutate offspring (skip elites)
        let elite_count = self.config.elitism * self.species.len().max(1);
        for i in elite_count..new_genomes.len() {
            self.seed = lcg_next(self.seed);
            self.mutate_genome(&mut new_genomes[i]);
        }

        self.genomes = new_genomes;
    }

    /// Apply mutations to a genome
    fn mutate_genome(&mut self, genome: &mut NeatGenome) {
        // Weight mutation
        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < self.config.weight_mutation_prob {
            genome.mutate_weights(0.8, self.config.weight_perturbation, self.seed);
        }

        // Add connection
        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < self.config.add_connection_prob {
            genome.mutate_add_connection(&mut self.tracker, self.seed);
        }

        // Add node
        self.seed = lcg_next(self.seed);
        if (self.seed as f64 / u64::MAX as f64) < self.config.add_node_prob {
            genome.mutate_add_node(&mut self.tracker, self.seed);
        }
    }

    /// Update best genome tracking
    pub fn update_best(&mut self) {
        for genome in &self.genomes {
            if genome.fitness > self.best_fitness {
                self.best_fitness = genome.fitness;
                self.best_genome = Some(genome.clone());
            }
        }
    }

    /// Get the best network
    pub fn get_best_network(&self) -> Option<NeatNetwork> {
        self.best_genome.as_ref().map(NeatNetwork::from_genome)
    }
}
