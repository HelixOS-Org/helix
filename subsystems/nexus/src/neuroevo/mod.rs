//! # Neuroevolution Engine for Helix OS Kernel
//!
//! Year 3 "EVOLUTION" - Revolutionary neuroevolutionary algorithms that evolve
//! both neural network topologies AND weights simultaneously for kernel-level
//! intelligent systems.
//!
//! ## Key Features
//!
//! - **NEAT (NeuroEvolution of Augmenting Topologies)**: Evolves both structure and weights
//! - **HyperNEAT**: Indirect encoding using compositional pattern-producing networks
//! - **ES-HyperNEAT**: Evolves substrate topology based on network geometry
//! - **Novelty Search**: Exploration-based search without explicit objectives
//! - **MAP-Elites**: Quality-Diversity algorithm for illuminating solution spaces
//! - **CoDeepNEAT**: Coevolution of deep neural network modules
//!
//! ## Kernel Applications
//!
//! - Evolving optimal scheduling policies
//! - Generating memory management heuristics
//! - Creating adaptive interrupt handlers
//! - Discovering novel kernel optimization strategies

#![no_std]

extern crate alloc;

// Module declarations
pub mod activation;
pub mod codeepneat;
pub mod connection;
pub mod genome;
pub mod hyperneat;
pub mod innovation;
pub mod kernel;
pub mod map_elites;
pub mod network;
pub mod node;
pub mod novelty;
pub mod population;
pub mod species;
pub mod types;
pub mod utils;

// Re-exports for public API
pub use activation::ActivationFunction;
pub use codeepneat::{CoDeepNeat, ModuleBlueprint, NetworkBlueprint};
pub use connection::ConnectionGene;
pub use genome::NeatGenome;
pub use hyperneat::{GeneratedNetwork, HyperNeat, SubstrateConfig, SubstratePoint};
pub use innovation::InnovationTracker;
pub use kernel::{EvolvedComponent, KernelEvolutionTarget, KernelNeuroevoManager, NeuroevoStats, NeuroevoSummary};
pub use map_elites::{EliteCell, MapElites, MapElitesConfig};
pub use network::NeatNetwork;
pub use node::{NodeGene, NodeType};
pub use novelty::{BehaviorVector, NoveltySearch};
pub use population::{NeatConfig, NeatPopulation};
pub use species::Species;
pub use types::{InnovationNumber, NodeId};
pub use utils::{lcg_next, random_weight};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    #[test]
    fn test_activation_functions() {
        assert!((ActivationFunction::Sigmoid.apply(0.0) - 0.5).abs() < 1e-10);
        assert!((ActivationFunction::Tanh.apply(0.0)).abs() < 1e-10);
        assert_eq!(ActivationFunction::ReLU.apply(-1.0), 0.0);
        assert_eq!(ActivationFunction::ReLU.apply(1.0), 1.0);
        assert_eq!(ActivationFunction::Step.apply(0.5), 1.0);
        assert_eq!(ActivationFunction::Step.apply(-0.5), 0.0);
    }

    #[test]
    fn test_node_gene_creation() {
        let input = NodeGene::input(1);
        assert_eq!(input.node_type, NodeType::Input);
        assert_eq!(input.layer, -1);

        let hidden = NodeGene::hidden(2, ActivationFunction::Tanh);
        assert_eq!(hidden.node_type, NodeType::Hidden);
        assert_eq!(hidden.activation, ActivationFunction::Tanh);

        let output = NodeGene::output(3, ActivationFunction::Sigmoid);
        assert_eq!(output.node_type, NodeType::Output);
    }

    #[test]
    fn test_innovation_tracker() {
        let mut tracker = InnovationTracker::new();

        let innov1 = tracker.get_or_create(1, 2);
        let innov2 = tracker.get_or_create(1, 3);
        let innov1_again = tracker.get_or_create(1, 2);

        assert_eq!(innov1, 1);
        assert_eq!(innov2, 2);
        assert_eq!(innov1_again, innov1); // Should return same innovation

        let node1 = tracker.new_node_id();
        let node2 = tracker.new_node_id();
        assert_ne!(node1, node2);
    }

    #[test]
    fn test_minimal_genome() {
        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        assert_eq!(genome.num_inputs, 2);
        assert_eq!(genome.num_outputs, 1);
        // Should have: 1 bias + 2 inputs + 1 output = 4 nodes
        assert_eq!(genome.nodes.len(), 4);
        // Should have connections from each input + bias to output = 3 connections
        assert_eq!(genome.connections.len(), 3);
    }

    #[test]
    fn test_network_activation() {
        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let mut network = NeatNetwork::from_genome(&genome);
        let output = network.activate(&[0.5, 0.5]);

        assert_eq!(output.len(), 1);
        // Output should be in [-1, 1] due to tanh
        assert!(output[0] >= -1.0 && output[0] <= 1.0);
    }

    #[test]
    fn test_weight_mutation() {
        let mut tracker = InnovationTracker::new();
        let mut genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let original_weights: Vec<f64> = genome.connections.iter().map(|c| c.weight).collect();
        genome.mutate_weights(1.0, 0.5, 12345); // 100% mutation rate

        let mutated_weights: Vec<f64> = genome.connections.iter().map(|c| c.weight).collect();

        // At least some weights should have changed
        let changed = original_weights
            .iter()
            .zip(mutated_weights.iter())
            .any(|(a, b)| (a - b).abs() > 1e-10);
        assert!(changed);
    }

    #[test]
    fn test_add_node_mutation() {
        let mut tracker = InnovationTracker::new();
        let mut genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let original_nodes = genome.nodes.len();
        let original_connections = genome.connections.len();

        genome.mutate_add_node(&mut tracker, 12345);

        // Should have added 1 hidden node
        assert_eq!(genome.nodes.len(), original_nodes + 1);
        // Should have added 2 new connections (disabled one old)
        assert_eq!(genome.connections.len(), original_connections + 2);

        // One connection should be disabled
        let disabled = genome.connections.iter().filter(|c| !c.enabled).count();
        assert_eq!(disabled, 1);
    }

    #[test]
    fn test_compatibility_distance() {
        let mut tracker = InnovationTracker::new();
        let genome1 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);
        let genome2 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let distance = genome1.compatibility_distance(&genome2, 1.0, 1.0, 0.4);

        // Similar genomes should have low distance
        assert!(distance < 5.0);
    }

    #[test]
    fn test_crossover() {
        let mut tracker = InnovationTracker::new();
        let mut parent1 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);
        let mut parent2 = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        parent1.fitness = 0.8;
        parent2.fitness = 0.6;

        let child = NeatGenome::crossover(&parent1, &parent2, 12345);

        assert_eq!(child.num_inputs, 2);
        assert_eq!(child.num_outputs, 1);
        assert_eq!(child.generation, 1);
    }

    #[test]
    fn test_species() {
        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);
        let species = Species::new(0, genome.clone());

        assert!(species.is_compatible(&genome, 5.0));
    }

    #[test]
    fn test_neat_population() {
        let config = NeatConfig {
            population_size: 10,
            num_inputs: 2,
            num_outputs: 1,
            ..Default::default()
        };

        let mut population = NeatPopulation::new(config, 12345);

        assert_eq!(population.genomes.len(), 10);

        // Set random fitness and evolve
        for (i, genome) in population.genomes.iter_mut().enumerate() {
            genome.fitness = i as f64 / 10.0;
        }

        population.speciate();
        population.calculate_adjusted_fitness();
        population.evolve();

        assert_eq!(population.genomes.len(), 10);
        assert_eq!(population.generation, 1);
    }

    #[test]
    fn test_behavior_vector() {
        let b1 = BehaviorVector::new(vec![0.0, 0.0, 0.0]);
        let b2 = BehaviorVector::new(vec![1.0, 0.0, 0.0]);
        let b3 = BehaviorVector::new(vec![1.0, 1.0, 1.0]);

        assert!((b1.distance(&b2) - 1.0).abs() < 1e-10);
        assert!((b1.distance(&b3) - libm::sqrt(3.0)).abs() < 1e-10);
        assert!((b1.distance(&b1)).abs() < 1e-10);
    }

    #[test]
    fn test_novelty_search() {
        let mut novelty = NoveltySearch::new(0.5, 5, 100);

        let b1 = BehaviorVector::new(vec![0.0, 0.0]);
        let score1 = novelty.novelty_score(&b1);
        assert_eq!(score1, f64::INFINITY); // First behavior is maximally novel

        novelty.add_to_archive(b1);

        let b2 = BehaviorVector::new(vec![0.0, 0.0]);
        let score2 = novelty.novelty_score(&b2);
        assert!(score2 < f64::INFINITY); // Now there's something to compare to
    }

    #[test]
    fn test_map_elites() {
        let config = MapElitesConfig {
            behavior_dims: 2,
            bins_per_dim: 10,
            behavior_bounds: vec![(0.0, 1.0), (0.0, 1.0)],
            batch_size: 10,
        };

        let mut map_elites = MapElites::new(config, 12345);

        let mut tracker = InnovationTracker::new();
        let genome = NeatGenome::minimal(2, 1, ActivationFunction::Tanh, &mut tracker);

        let added = map_elites.try_add(genome.clone(), vec![0.5, 0.5], 1.0);
        assert!(added);
        assert_eq!(map_elites.coverage, 1);

        // Same cell, lower fitness - should not add
        let not_added = map_elites.try_add(genome.clone(), vec![0.5, 0.5], 0.5);
        assert!(!not_added);

        // Same cell, higher fitness - should add
        let replaced = map_elites.try_add(genome, vec![0.5, 0.5], 2.0);
        assert!(replaced);
        assert_eq!(map_elites.coverage, 1); // Still 1 cell
    }

    #[test]
    fn test_substrate_point() {
        let p1 = SubstratePoint::new(0.0, 0.0);
        let p2 = SubstratePoint::new(3.0, 4.0);

        assert!((p1.distance(&p2) - 5.0).abs() < 1e-10); // 3-4-5 triangle
    }

    #[test]
    fn test_substrate_config() {
        let substrate = SubstrateConfig::grid_2d(4, 8, 2);

        assert_eq!(substrate.inputs.len(), 4);
        assert_eq!(substrate.hidden.len(), 8);
        assert_eq!(substrate.outputs.len(), 2);
    }

    #[test]
    fn test_module_blueprint() {
        let mut module = ModuleBlueprint::new(1, 12345);

        assert!(module.num_layers >= 1 && module.num_layers <= 5);
        assert!(!module.layers.is_empty());

        module.mutate(67890);
        // Mutation should not break the module
        assert!(!module.layers.is_empty());
    }

    #[test]
    fn test_network_blueprint() {
        let mut network = NetworkBlueprint::new(1, vec![0, 1, 2]);

        assert_eq!(network.modules.len(), 3);
        assert_eq!(network.connections.len(), 2); // Sequential connections

        network.mutate(&[0, 1, 2, 3, 4], 12345);
        // Should still have valid structure
        assert!(!network.modules.is_empty());
    }

    #[test]
    fn test_codeepneat() {
        let mut codeepneat = CoDeepNeat::new(5, 5, 12345);

        assert_eq!(codeepneat.modules.len(), 5);
        assert_eq!(codeepneat.networks.len(), 5);

        // Set fitness
        for module in &mut codeepneat.modules {
            module.fitness = 1.0;
        }
        for network in &mut codeepneat.networks {
            network.fitness = 1.0;
        }

        codeepneat.evolve();

        assert_eq!(codeepneat.modules.len(), 5);
        assert_eq!(codeepneat.networks.len(), 5);
        assert_eq!(codeepneat.generation, 1);
    }

    #[test]
    fn test_hyperneat() {
        let substrate = SubstrateConfig::grid_2d(4, 8, 2);
        let hyperneat = HyperNeat::new(substrate, 12345);

        assert_eq!(hyperneat.cppn_population.genomes.len(), 100);

        // Generate a network
        let network = hyperneat.generate_network(0);
        assert_eq!(network.num_inputs, 4);
        assert_eq!(network.num_hidden, 8);
        assert_eq!(network.num_outputs, 2);
    }

    #[test]
    fn test_kernel_neuroevo_manager() {
        let mut manager = KernelNeuroevoManager::new(12345);

        manager.initialize_target(KernelEvolutionTarget::Scheduler, 8, 4, 12345);

        // Set fitness for genomes
        if let Some(pop) = manager
            .populations
            .get_mut(&KernelEvolutionTarget::Scheduler)
        {
            for (i, genome) in pop.genomes.iter_mut().enumerate() {
                genome.fitness = i as f64 / 100.0;
            }
        }

        manager.evolve_target(KernelEvolutionTarget::Scheduler);

        let summary = manager.get_summary();
        assert_eq!(summary.total_generations, 1);
        assert!(summary.genomes_evaluated > 0);
    }

    #[test]
    fn test_generated_network() {
        let mut network = GeneratedNetwork::new(2, 4, 1);

        // Add some connections manually
        network.add_connection(0, 0, 2, 0.5);
        network.add_connection(0, 1, 3, 0.5);
        network.add_connection(1, 2, 6, 0.5);
        network.add_connection(1, 3, 6, 0.5);

        let output = network.activate(&[1.0, 1.0]);
        assert_eq!(output.len(), 1);
    }
}
