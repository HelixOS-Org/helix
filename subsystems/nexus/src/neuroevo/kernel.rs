//! Kernel neuroevolution manager for evolving kernel components.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::codeepneat::CoDeepNeat;
use crate::neuroevo::genome::NeatGenome;
use crate::neuroevo::hyperneat::HyperNeat;
use crate::neuroevo::network::NeatNetwork;
use crate::neuroevo::novelty::{BehaviorVector, NoveltySearch};
use crate::neuroevo::population::{NeatConfig, NeatPopulation};
use crate::neuroevo::map_elites::MapElites;

/// Types of kernel components that can be evolved
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KernelEvolutionTarget {
    /// Scheduler policy
    Scheduler,
    /// Memory allocator
    MemoryAllocator,
    /// I/O scheduler
    IoScheduler,
    /// Interrupt handler
    InterruptHandler,
    /// Power management
    PowerManager,
    /// Cache policy
    CachePolicy,
}

/// Evolved kernel component descriptor
#[derive(Debug, Clone)]
pub struct EvolvedComponent {
    /// Target component type
    pub target: KernelEvolutionTarget,
    /// Generation evolved in
    pub generation: u32,
    /// Fitness achieved
    pub fitness: f64,
    /// Network genome
    pub genome: NeatGenome,
    /// Behavior descriptor
    pub behavior: Vec<f64>,
    /// Timestamp
    pub timestamp: u64,
}

/// Main neuroevolution manager for kernel components
pub struct KernelNeuroevoManager {
    /// NEAT populations for different targets
    pub populations: BTreeMap<KernelEvolutionTarget, NeatPopulation>,
    /// Evolved components registry
    pub evolved_components: Vec<EvolvedComponent>,
    /// Novelty search instance
    pub novelty_search: NoveltySearch,
    /// MAP-Elites for quality-diversity
    pub map_elites: Option<MapElites>,
    /// HyperNEAT for large-scale networks
    pub hyperneat: Option<HyperNeat>,
    /// CoDeepNEAT for modular networks
    pub codeepneat: Option<CoDeepNeat>,
    /// Total generations evolved
    pub total_generations: u64,
    /// Best overall fitness
    pub best_fitness: f64,
    /// Statistics
    pub stats: NeuroevoStats,
}

/// Neuroevolution statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct NeuroevoStats {
    /// Total genomes evaluated
    pub genomes_evaluated: u64,
    /// Total mutations applied
    pub mutations_applied: u64,
    /// Successful improvements
    pub improvements: u64,
    /// Species count over time
    pub species_history: Vec<usize>,
    /// Best fitness over time
    pub fitness_history: Vec<f64>,
    /// Complexity over time
    pub complexity_history: Vec<usize>,
}

impl KernelNeuroevoManager {
    /// Create a new kernel neuroevolution manager
    pub fn new(_seed: u64) -> Self {
        Self {
            populations: BTreeMap::new(),
            evolved_components: Vec::new(),
            novelty_search: NoveltySearch::new(0.5, 15, 500),
            map_elites: None,
            hyperneat: None,
            codeepneat: None,
            total_generations: 0,
            best_fitness: f64::NEG_INFINITY,
            stats: NeuroevoStats::default(),
        }
    }

    /// Initialize a population for a kernel target
    pub fn initialize_target(
        &mut self,
        target: KernelEvolutionTarget,
        num_inputs: usize,
        num_outputs: usize,
        seed: u64,
    ) {
        let config = NeatConfig {
            population_size: 100,
            num_inputs,
            num_outputs,
            output_activation: ActivationFunction::Tanh,
            ..Default::default()
        };

        let population = NeatPopulation::new(config, seed);
        self.populations.insert(target, population);
    }

    /// Evolve a target population
    pub fn evolve_target(&mut self, target: KernelEvolutionTarget) -> Option<()> {
        let population = self.populations.get_mut(&target)?;

        // Record stats before evolution
        self.stats.species_history.push(population.species.len());
        self.stats.fitness_history.push(population.best_fitness);
        if let Some(ref best) = population.best_genome {
            self.stats.complexity_history.push(best.complexity());
        }

        // Speciate and evolve
        population.speciate();
        population.calculate_adjusted_fitness();
        population.evolve();
        population.update_best();

        self.total_generations += 1;
        self.stats.genomes_evaluated += population.genomes.len() as u64;

        if population.best_fitness > self.best_fitness {
            self.best_fitness = population.best_fitness;
            self.stats.improvements += 1;
        }

        Some(())
    }

    /// Get the best network for a target
    #[inline(always)]
    pub fn get_best_network(&self, target: KernelEvolutionTarget) -> Option<NeatNetwork> {
        let population = self.populations.get(&target)?;
        population.get_best_network()
    }

    /// Register an evolved component
    #[inline]
    pub fn register_component(&mut self, component: EvolvedComponent) {
        // Add to novelty search
        self.novelty_search
            .add_to_archive(BehaviorVector::new(component.behavior.clone()));

        self.evolved_components.push(component);
    }

    /// Get summary of evolution progress
    #[inline]
    pub fn get_summary(&self) -> NeuroevoSummary {
        NeuroevoSummary {
            total_generations: self.total_generations,
            best_fitness: self.best_fitness,
            evolved_components: self.evolved_components.len(),
            archive_size: self.novelty_search.archive.len(),
            populations: self.populations.len(),
            genomes_evaluated: self.stats.genomes_evaluated,
            improvements: self.stats.improvements,
        }
    }
}

/// Summary of neuroevolution state
#[derive(Debug, Clone)]
pub struct NeuroevoSummary {
    pub total_generations: u64,
    pub best_fitness: f64,
    pub evolved_components: usize,
    pub archive_size: usize,
    pub populations: usize,
    pub genomes_evaluated: u64,
    pub improvements: u64,
}
