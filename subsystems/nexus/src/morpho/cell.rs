//! Kernel cell types and tissue organization.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::{MorphogenType, Position};

/// Cell type (differentiated kernel component)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellType {
    /// Undifferentiated stem cell
    Stem,
    /// Memory management cell
    Memory,
    /// Scheduler cell
    Scheduler,
    /// I/O handling cell
    IO,
    /// Network processing cell
    Network,
    /// Security monitoring cell
    Security,
    /// Power management cell
    Power,
    /// Cache management cell
    Cache,
    /// Interrupt handling cell
    Interrupt,
    /// Timer cell
    Timer,
}

/// Cell internal state
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CellState {
    /// Activity level (0-1)
    pub activity: f64,
    /// Energy level
    pub energy: f64,
    /// Stress level
    pub stress: f64,
    /// Health (0-1)
    pub health: f64,
}

/// Gene identifier
pub type GeneId = u32;

/// Cell state
#[derive(Debug, Clone)]
pub struct Cell {
    /// Cell identifier
    pub id: u64,
    /// Cell type
    pub cell_type: CellType,
    /// Position in the field
    pub position: Position,
    /// Internal state
    pub state: CellState,
    /// Gene expression levels
    pub expression: BTreeMap<GeneId, f64>,
    /// Receptors for morphogens
    pub receptors: BTreeMap<MorphogenType, f64>,
    /// Age (in simulation steps)
    pub age: u64,
}

impl Cell {
    /// Create a new stem cell
    pub fn new_stem(id: u64, position: Position) -> Self {
        Self {
            id,
            cell_type: CellType::Stem,
            position,
            state: CellState {
                activity: 0.5,
                energy: 1.0,
                stress: 0.0,
                health: 1.0,
            },
            expression: BTreeMap::new(),
            receptors: BTreeMap::new(),
            age: 0,
        }
    }

    /// Differentiate based on morphogen concentrations
    pub fn differentiate(&mut self, morphogens: &BTreeMap<MorphogenType, f64>) {
        if self.cell_type != CellType::Stem {
            return; // Already differentiated
        }

        // Determine cell type based on morphogen levels
        let cpu = morphogens
            .get(&MorphogenType::CpuDemand)
            .copied()
            .unwrap_or(0.0);
        let memory = morphogens
            .get(&MorphogenType::MemoryPressure)
            .copied()
            .unwrap_or(0.0);
        let io = morphogens
            .get(&MorphogenType::IoLoad)
            .copied()
            .unwrap_or(0.0);
        let network = morphogens
            .get(&MorphogenType::NetworkActivity)
            .copied()
            .unwrap_or(0.0);

        let max_signal = cpu.max(memory).max(io).max(network);

        if max_signal < 0.1 {
            return; // Not enough signal to differentiate
        }

        self.cell_type = if cpu == max_signal {
            CellType::Scheduler
        } else if memory == max_signal {
            CellType::Memory
        } else if io == max_signal {
            CellType::IO
        } else {
            CellType::Network
        };

        // Initialize specialized receptors
        match self.cell_type {
            CellType::Scheduler => {
                self.receptors.insert(MorphogenType::CpuDemand, 1.0);
                self.receptors
                    .insert(MorphogenType::LatencySensitivity, 0.8);
            },
            CellType::Memory => {
                self.receptors.insert(MorphogenType::MemoryPressure, 1.0);
            },
            CellType::IO => {
                self.receptors.insert(MorphogenType::IoLoad, 1.0);
                self.receptors.insert(MorphogenType::ThroughputDemand, 0.7);
            },
            CellType::Network => {
                self.receptors.insert(MorphogenType::NetworkActivity, 1.0);
            },
            _ => {},
        }
    }

    /// Update cell based on environment
    pub fn update(&mut self, morphogens: &BTreeMap<MorphogenType, f64>, dt: f64) {
        self.age += 1;

        // Update activity based on relevant morphogens
        let mut input = 0.0;
        for (morph_type, &sensitivity) in &self.receptors {
            if let Some(&concentration) = morphogens.get(morph_type) {
                input += concentration * sensitivity;
            }
        }

        // Sigmoid activation
        self.state.activity = 1.0 / (1.0 + libm::exp(-input + 2.0));

        // Energy consumption
        self.state.energy -= self.state.activity * 0.01 * dt;
        self.state.energy = self.state.energy.max(0.0);

        // Stress accumulation
        if self.state.activity > 0.8 {
            self.state.stress += 0.1 * dt;
        } else {
            self.state.stress -= 0.05 * dt;
        }
        self.state.stress = self.state.stress.clamp(0.0, 1.0);

        // Health depends on energy and stress
        self.state.health = (self.state.energy * (1.0 - self.state.stress * 0.5)).clamp(0.0, 1.0);
    }

    /// Check if cell should die (apoptosis)
    #[inline(always)]
    pub fn should_die(&self) -> bool {
        self.state.health < 0.1 || (self.age > 10000 && self.state.activity < 0.1)
    }

    /// Check if cell can divide
    #[inline(always)]
    pub fn can_divide(&self) -> bool {
        self.state.energy > 0.8 && self.state.health > 0.8 && self.cell_type == CellType::Stem
    }
}

/// Tissue state
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TissueState {
    /// Overall health
    pub health: f64,
    /// Capacity (number of cells)
    pub capacity: usize,
    /// Throughput capability
    pub throughput: f64,
}

/// Tissue (collection of similar cells)
#[derive(Debug, Clone)]
pub struct Tissue {
    /// Tissue identifier
    pub id: u64,
    /// Tissue type (based on majority cell type)
    pub tissue_type: CellType,
    /// Cells in this tissue
    pub cells: Vec<Cell>,
    /// Center of mass
    pub center: Position,
    /// Tissue state
    pub state: TissueState,
}

impl Tissue {
    /// Create a new tissue
    pub fn new(id: u64, tissue_type: CellType) -> Self {
        Self {
            id,
            tissue_type,
            cells: Vec::new(),
            center: Position::default(),
            state: TissueState {
                health: 1.0,
                capacity: 100,
                throughput: 0.0,
            },
        }
    }

    /// Add cell to tissue
    #[inline]
    pub fn add_cell(&mut self, cell: Cell) {
        if cell.cell_type == self.tissue_type || cell.cell_type == CellType::Stem {
            self.cells.push(cell);
            self.update_center();
        }
    }

    /// Update center of mass
    fn update_center(&mut self) {
        if self.cells.is_empty() {
            return;
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;

        for cell in &self.cells {
            sum_x += cell.position.x;
            sum_y += cell.position.y;
            sum_z += cell.position.z;
        }

        let n = self.cells.len() as f64;
        self.center = Position::new(sum_x / n, sum_y / n, sum_z / n);
    }

    /// Update tissue state
    pub fn update(&mut self, morphogens: &BTreeMap<MorphogenType, f64>, dt: f64) {
        // Update all cells
        for cell in &mut self.cells {
            cell.update(morphogens, dt);
        }

        // Remove dead cells
        self.cells.retain(|c| !c.should_die());

        // Calculate tissue health
        if !self.cells.is_empty() {
            self.state.health =
                self.cells.iter().map(|c| c.state.health).sum::<f64>() / self.cells.len() as f64;

            self.state.throughput = self.cells.iter().map(|c| c.state.activity).sum::<f64>();
        } else {
            self.state.health = 0.0;
            self.state.throughput = 0.0;
        }

        self.update_center();
    }

    /// Get tissue size
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.cells.len()
    }
}
