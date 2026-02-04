//! # Morphogenetic Kernel Structures
//!
//! Revolutionary self-organizing kernel architecture inspired by biological
//! morphogenesis. The kernel develops and adapts its internal structure
//! dynamically based on environmental signals and workload patterns.
//!
//! ## Features
//!
//! - **Cellular Automata**: Local rules producing global organization
//! - **Reaction-Diffusion**: Pattern formation for resource distribution
//! - **Developmental Programs**: Growth and adaptation over time
//! - **Homeostasis**: Self-regulating equilibrium maintenance
//! - **Regeneration**: Automatic recovery from damage
//! - **Differentiation**: Specialized structures for different tasks
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    MORPHOGENETIC KERNEL SYSTEM                          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    MORPHOGEN FIELD                              │     │
//! │  │   Concentration gradients guiding resource allocation          │     │
//! │  │   ∂A/∂t = D_A∇²A + f(A,B) - k_A·A                             │     │
//! │  │   ∂B/∂t = D_B∇²B + g(A,B) - k_B·B                             │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    CELL DIFFERENTIATION                         │     │
//! │  │   Kernel components differentiate based on morphogen levels    │     │
//! │  │                                                                 │     │
//! │  │   [Stem] ──▶ [Memory] │ [Scheduler] │ [I/O] │ [Network]       │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────────┐     │
//! │  │                    TISSUE ORGANIZATION                          │     │
//! │  │   Related components form functional tissues                    │     │
//! │  │   Tissues coordinate for organ-level behavior                  │     │
//! │  └────────────────────────────────────────────────────────────────┘     │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

// Submodules
pub mod cell;
pub mod field;
pub mod homeostasis;
pub mod kernel;
pub mod organ;
pub mod regeneration;
pub mod turing;
pub mod types;

// Re-exports
pub use cell::{Cell, CellState, CellType, GeneId, Tissue, TissueState};
pub use field::MorphogenField;
pub use homeostasis::HomeostasisController;
pub use kernel::MorphogeneticKernel;
pub use organ::{Organ, OrganFunction, OrganType};
pub use regeneration::RegenerationManager;
pub use turing::{TuringParams, TuringPattern};
pub use types::{MorphogenType, Position};

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeMap;

    use super::*;

    extern crate alloc;

    #[test]
    fn test_morphogen_field() {
        let mut field = MorphogenField::new(10, MorphogenType::CpuDemand);

        // Add source
        field.add_source(5, 5, 5, 100.0);
        assert_eq!(field.get(5, 5, 5), 100.0);

        // Simulate diffusion
        for _ in 0..10 {
            field.step(0.1);
        }

        // Should have spread to neighbors
        assert!(field.get(4, 5, 5) > 0.0);
        assert!(field.get(6, 5, 5) > 0.0);
    }

    #[test]
    fn test_turing_pattern() {
        let mut turing = TuringPattern::new(8, TuringParams::default());
        let mut rng = 12345u64;

        turing.initialize_random(&mut rng);

        let initial = turing.activator.total() + turing.inhibitor.total();

        for _ in 0..100 {
            turing.step(0.01);
        }

        // Pattern should evolve
        let final_total = turing.activator.total() + turing.inhibitor.total();
        assert!((final_total - initial).abs() < initial * 0.5);
    }

    #[test]
    fn test_cell_differentiation() {
        let mut cell = Cell::new_stem(1, Position::new(0.0, 0.0, 0.0));

        let mut morphogens = BTreeMap::new();
        morphogens.insert(MorphogenType::CpuDemand, 1.0);
        morphogens.insert(MorphogenType::MemoryPressure, 0.3);

        cell.differentiate(&morphogens);

        assert_eq!(cell.cell_type, CellType::Scheduler);
    }

    #[test]
    fn test_morphogenetic_kernel() {
        let mut kernel = MorphogeneticKernel::new(8);
        kernel.initialize_organs();

        // Inject workload signal
        kernel.inject_signal(MorphogenType::CpuDemand, 4, 4, 4, 10.0);

        // Simulate
        for _ in 0..100 {
            kernel.step(0.1);
        }

        assert!(kernel.system_health() > 0.0);
        assert!(kernel.total_cells() > 0);
    }

    #[test]
    fn test_homeostasis() {
        let mut controller = HomeostasisController::new();
        controller.set_target(MorphogenType::CpuDemand, 0.5);

        // Below target - should return positive
        let signal = controller.control(MorphogenType::CpuDemand, 0.3, 0.1);
        assert!(signal > 0.0);

        // Above target - should return negative
        let signal = controller.control(MorphogenType::CpuDemand, 0.7, 0.1);
        assert!(signal < 0.0);
    }
}
