//! Complete morphogenetic kernel system.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::cell::{Cell, CellType, Tissue};
use super::field::MorphogenField;
use super::homeostasis::HomeostasisController;
use super::organ::{Organ, OrganType};
use super::regeneration::RegenerationManager;
use super::turing::{TuringParams, TuringPattern};
use super::types::{MorphogenType, Position};

/// Complete morphogenetic kernel system
pub struct MorphogeneticKernel {
    /// Morphogen fields
    fields: BTreeMap<MorphogenType, MorphogenField>,
    /// Turing pattern generator
    turing: TuringPattern,
    /// Organs
    organs: Vec<Organ>,
    /// Homeostasis controller
    homeostasis: HomeostasisController,
    /// Regeneration manager
    regeneration: RegenerationManager,
    /// Grid size
    grid_size: usize,
    /// Simulation time
    time: f64,
}

impl MorphogeneticKernel {
    /// Create a new morphogenetic kernel
    pub fn new(grid_size: usize) -> Self {
        let mut fields = BTreeMap::new();

        // Initialize morphogen fields
        for morph_type in [
            MorphogenType::CpuDemand,
            MorphogenType::MemoryPressure,
            MorphogenType::IoLoad,
            MorphogenType::NetworkActivity,
            MorphogenType::ThermalStress,
            MorphogenType::PowerBudget,
        ] {
            fields.insert(morph_type, MorphogenField::new(grid_size, morph_type));
        }

        // Initialize homeostasis targets
        let mut homeostasis = HomeostasisController::new();
        homeostasis.set_target(MorphogenType::CpuDemand, 0.7);
        homeostasis.set_target(MorphogenType::MemoryPressure, 0.6);
        homeostasis.set_target(MorphogenType::ThermalStress, 0.3);

        Self {
            fields,
            turing: TuringPattern::new(grid_size, TuringParams::default()),
            organs: Vec::new(),
            homeostasis,
            regeneration: RegenerationManager::new(),
            grid_size,
            time: 0.0,
        }
    }

    /// Inject morphogen signal (e.g., from workload)
    pub fn inject_signal(
        &mut self,
        morph_type: MorphogenType,
        x: usize,
        y: usize,
        z: usize,
        amount: f64,
    ) {
        if let Some(field) = self.fields.get_mut(&morph_type) {
            field.add_source(x, y, z, amount);
        }
    }

    /// Initialize organs
    pub fn initialize_organs(&mut self) {
        // Create processing core organ
        let mut processing = Organ::new(0, OrganType::ProcessingCore);
        let mut scheduler_tissue = Tissue::new(0, CellType::Scheduler);

        // Add some initial cells
        for i in 0..10 {
            let mut cell =
                Cell::new_stem(i as u64, Position::new((i % 5) as f64, (i / 5) as f64, 0.0));
            cell.cell_type = CellType::Scheduler;
            scheduler_tissue.add_cell(cell);
        }
        processing.add_tissue(scheduler_tissue);
        self.organs.push(processing);

        // Create memory pool organ
        let mut memory = Organ::new(1, OrganType::MemoryPool);
        let mut memory_tissue = Tissue::new(1, CellType::Memory);
        for i in 0..10 {
            let mut cell = Cell::new_stem(
                10 + i as u64,
                Position::new((i % 5) as f64 + 5.0, (i / 5) as f64, 0.0),
            );
            cell.cell_type = CellType::Memory;
            memory_tissue.add_cell(cell);
        }
        memory.add_tissue(memory_tissue);
        self.organs.push(memory);

        // Initialize regeneration
        self.regeneration.add_stem_cells(
            50,
            Position::new(
                (self.grid_size / 2) as f64,
                (self.grid_size / 2) as f64,
                (self.grid_size / 2) as f64,
            ),
        );
    }

    /// Simulate one time step
    pub fn step(&mut self, dt: f64) {
        self.time += dt;

        // Diffuse all morphogen fields
        for field in self.fields.values_mut() {
            field.step(dt);
        }

        // Step Turing pattern
        self.turing.step(dt);

        // Get current morphogen concentrations at organ locations
        // Pre-compute morphogen samples to avoid borrowing self in the loop
        let morphogen_samples: Vec<_> = self
            .organs
            .iter()
            .map(|organ| self.sample_morphogens(&organ.tissues[0].center))
            .collect();

        for (organ, morphogens) in self.organs.iter_mut().zip(morphogen_samples.into_iter()) {
            organ.update(&morphogens, dt);

            // Check if regeneration needed
            for tissue in &mut organ.tissues {
                if tissue.state.health < 0.5 && tissue.size() < tissue.state.capacity {
                    let needed = (tissue.state.capacity - tissue.size()).min(5);
                    self.regeneration.regenerate(tissue, needed);
                }
            }
        }

        // Homeostatic control
        for (morph_type, field) in &mut self.fields {
            let avg_concentration = field.total() / (self.grid_size.pow(3) as f64);
            let control = self.homeostasis.control(*morph_type, avg_concentration, dt);

            // Apply control signal (adjust sources)
            let center = self.grid_size / 2;
            if control > 0.0 {
                field.add_source(center, center, center, control * dt);
            }
        }

        // Replenish stem cells
        self.regeneration.replenish(Position::new(
            (self.grid_size / 2) as f64,
            (self.grid_size / 2) as f64,
            (self.grid_size / 2) as f64,
        ));
    }

    /// Sample morphogen concentrations at position
    fn sample_morphogens(&self, pos: &Position) -> BTreeMap<MorphogenType, f64> {
        let x = libm::floor(pos.x) as usize;
        let y = libm::floor(pos.y) as usize;
        let z = libm::floor(pos.z) as usize;

        self.fields
            .iter()
            .map(|(&t, f)| (t, f.get(x, y, z)))
            .collect()
    }

    /// Get overall system health
    pub fn system_health(&self) -> f64 {
        if self.organs.is_empty() {
            return 0.0;
        }

        self.organs.iter().map(|o| o.function.capacity).sum::<f64>() / self.organs.len() as f64
    }

    /// Get total cell count
    pub fn total_cells(&self) -> usize {
        self.organs.iter().map(|o| o.cell_count()).sum::<usize>()
            + self.regeneration.stem_cells.len()
    }
}
