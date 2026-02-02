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

extern crate alloc;

// TODO: Ces sous-modules doivent être créés
// pub mod automata;
// pub mod cell;
// pub mod development;
// pub mod gradient;
// pub mod homeostasis;
// pub mod morphogen;
// pub mod organ;
// pub mod pattern;
// pub mod regeneration;
// pub mod signaling;
// pub mod tissue;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::math::F64Ext;

/// Morphogen types (signaling molecules)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MorphogenType {
    /// Activator - promotes growth/activity
    Activator,
    /// Inhibitor - suppresses growth/activity
    Inhibitor,
    /// CPU demand signal
    CpuDemand,
    /// Memory pressure signal
    MemoryPressure,
    /// I/O load signal
    IoLoad,
    /// Network activity signal
    NetworkActivity,
    /// Thermal stress signal
    ThermalStress,
    /// Power budget signal
    PowerBudget,
    /// Latency sensitivity signal
    LatencySensitivity,
    /// Throughput demand signal
    ThroughputDemand,
}

/// 3D position in the morphogenetic field
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn distance(&self, other: &Position) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        libm::sqrt(dx * dx + dy * dy + dz * dz)
    }

    pub fn neighbors(&self, grid_size: usize) -> Vec<(usize, usize, usize)> {
        let mut result = Vec::new();
        let ix = self.x as i64;
        let iy = self.y as i64;
        let iz = self.z as i64;

        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let nx = ix + dx;
                    let ny = iy + dy;
                    let nz = iz + dz;
                    if nx >= 0
                        && ny >= 0
                        && nz >= 0
                        && (nx as usize) < grid_size
                        && (ny as usize) < grid_size
                        && (nz as usize) < grid_size
                    {
                        result.push((nx as usize, ny as usize, nz as usize));
                    }
                }
            }
        }
        result
    }
}

/// Morphogen concentration field
#[derive(Debug, Clone)]
pub struct MorphogenField {
    /// Grid size
    size: usize,
    /// Morphogen type
    morphogen_type: MorphogenType,
    /// Concentration values (3D grid)
    concentrations: Vec<f64>,
    /// Diffusion coefficient
    diffusion: f64,
    /// Decay rate
    decay: f64,
}

impl MorphogenField {
    /// Create a new morphogen field
    pub fn new(size: usize, morphogen_type: MorphogenType) -> Self {
        let total = size * size * size;
        Self {
            size,
            morphogen_type,
            concentrations: alloc::vec![0.0; total],
            diffusion: 0.1,
            decay: 0.01,
        }
    }

    /// Set diffusion coefficient
    pub fn with_diffusion(mut self, diffusion: f64) -> Self {
        self.diffusion = diffusion;
        self
    }

    /// Set decay rate
    pub fn with_decay(mut self, decay: f64) -> Self {
        self.decay = decay;
        self
    }

    /// Get index from coordinates
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.size + z * self.size * self.size
    }

    /// Get concentration at position
    pub fn get(&self, x: usize, y: usize, z: usize) -> f64 {
        if x < self.size && y < self.size && z < self.size {
            self.concentrations[self.index(x, y, z)]
        } else {
            0.0
        }
    }

    /// Set concentration at position
    pub fn set(&mut self, x: usize, y: usize, z: usize, value: f64) {
        if x < self.size && y < self.size && z < self.size {
            let idx = self.index(x, y, z);
            self.concentrations[idx] = value;
        }
    }

    /// Add concentration (source)
    pub fn add_source(&mut self, x: usize, y: usize, z: usize, amount: f64) {
        if x < self.size && y < self.size && z < self.size {
            let idx = self.index(x, y, z);
            self.concentrations[idx] += amount;
        }
    }

    /// Simulate one time step (diffusion + decay)
    pub fn step(&mut self, dt: f64) {
        let mut new_concentrations = self.concentrations.clone();

        for z in 0..self.size {
            for y in 0..self.size {
                for x in 0..self.size {
                    let idx = self.index(x, y, z);
                    let current = self.concentrations[idx];

                    // Laplacian (discrete)
                    let mut laplacian = -6.0 * current;

                    if x > 0 {
                        laplacian += self.get(x - 1, y, z);
                    }
                    if x < self.size - 1 {
                        laplacian += self.get(x + 1, y, z);
                    }
                    if y > 0 {
                        laplacian += self.get(x, y - 1, z);
                    }
                    if y < self.size - 1 {
                        laplacian += self.get(x, y + 1, z);
                    }
                    if z > 0 {
                        laplacian += self.get(x, y, z - 1);
                    }
                    if z < self.size - 1 {
                        laplacian += self.get(x, y, z + 1);
                    }

                    // Diffusion + decay
                    let change = self.diffusion * laplacian - self.decay * current;
                    new_concentrations[idx] = (current + change * dt).max(0.0);
                }
            }
        }

        self.concentrations = new_concentrations;
    }

    /// Get gradient at position
    pub fn gradient(&self, x: usize, y: usize, z: usize) -> (f64, f64, f64) {
        let dx = if x > 0 && x < self.size - 1 {
            (self.get(x + 1, y, z) - self.get(x - 1, y, z)) / 2.0
        } else {
            0.0
        };

        let dy = if y > 0 && y < self.size - 1 {
            (self.get(x, y + 1, z) - self.get(x, y - 1, z)) / 2.0
        } else {
            0.0
        };

        let dz = if z > 0 && z < self.size - 1 {
            (self.get(x, y, z + 1) - self.get(x, y, z - 1)) / 2.0
        } else {
            0.0
        };

        (dx, dy, dz)
    }

    /// Get total concentration
    pub fn total(&self) -> f64 {
        self.concentrations.iter().sum()
    }
}

/// Turing pattern generator (reaction-diffusion)
#[derive(Debug, Clone)]
pub struct TuringPattern {
    /// Activator field
    activator: MorphogenField,
    /// Inhibitor field
    inhibitor: MorphogenField,
    /// Reaction parameters
    params: TuringParams,
}

/// Turing pattern parameters
#[derive(Debug, Clone, Copy)]
pub struct TuringParams {
    /// Activator production rate
    pub alpha: f64,
    /// Inhibitor production rate
    pub beta: f64,
    /// Activator self-activation
    pub gamma: f64,
    /// Inhibitor effect on activator
    pub delta: f64,
    /// Activator effect on inhibitor
    pub epsilon: f64,
    /// Inhibitor self-decay
    pub zeta: f64,
}

impl Default for TuringParams {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            beta: 0.5,
            gamma: 0.1,
            delta: 1.0,
            epsilon: 1.0,
            zeta: 0.1,
        }
    }
}

impl TuringPattern {
    /// Create a new Turing pattern generator
    pub fn new(size: usize, params: TuringParams) -> Self {
        Self {
            activator: MorphogenField::new(size, MorphogenType::Activator).with_diffusion(0.01),
            inhibitor: MorphogenField::new(size, MorphogenType::Inhibitor).with_diffusion(0.05), // Inhibitor diffuses faster
            params,
        }
    }

    /// Initialize with random perturbation
    pub fn initialize_random(&mut self, rng: &mut u64) {
        for i in 0..self.activator.concentrations.len() {
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;

            self.activator.concentrations[i] = 1.0 + 0.1 * ((*rng as f64 / u64::MAX as f64) - 0.5);

            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;

            self.inhibitor.concentrations[i] = 1.0 + 0.1 * ((*rng as f64 / u64::MAX as f64) - 0.5);
        }
    }

    /// Simulate one time step
    pub fn step(&mut self, dt: f64) {
        let _size = self.activator.size;
        let p = &self.params;

        // Store old concentrations
        let old_a = self.activator.concentrations.clone();
        let old_b = self.inhibitor.concentrations.clone();

        for i in 0..old_a.len() {
            let a = old_a[i];
            let b = old_b[i];

            // Reaction terms (Gray-Scott like)
            let da = p.alpha * a * a / (1.0 + p.delta * b) - p.gamma * a;
            let db = p.epsilon * a * a - p.zeta * b;

            self.activator.concentrations[i] = (a + da * dt).max(0.0);
            self.inhibitor.concentrations[i] = (b + db * dt).max(0.0);
        }

        // Diffusion
        self.activator.step(dt);
        self.inhibitor.step(dt);
    }

    /// Get activator concentration
    pub fn get_activator(&self, x: usize, y: usize, z: usize) -> f64 {
        self.activator.get(x, y, z)
    }

    /// Get inhibitor concentration
    pub fn get_inhibitor(&self, x: usize, y: usize, z: usize) -> f64 {
        self.inhibitor.get(x, y, z)
    }

    /// Check if pattern has stabilized
    pub fn is_stable(&self, previous_total: f64, threshold: f64) -> bool {
        let current = self.activator.total() + self.inhibitor.total();
        (current - previous_total).abs() < threshold
    }
}

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

/// Cell internal state
#[derive(Debug, Clone, Default)]
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
    pub fn should_die(&self) -> bool {
        self.state.health < 0.1 || (self.age > 10000 && self.state.activity < 0.1)
    }

    /// Check if cell can divide
    pub fn can_divide(&self) -> bool {
        self.state.energy > 0.8 && self.state.health > 0.8 && self.cell_type == CellType::Stem
    }
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

/// Tissue state
#[derive(Debug, Clone, Default)]
pub struct TissueState {
    /// Overall health
    pub health: f64,
    /// Capacity (number of cells)
    pub capacity: usize,
    /// Throughput capability
    pub throughput: f64,
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
    pub fn size(&self) -> usize {
        self.cells.len()
    }
}

/// Organ (functional unit composed of tissues)
#[derive(Debug, Clone)]
pub struct Organ {
    /// Organ identifier
    pub id: u64,
    /// Organ type
    pub organ_type: OrganType,
    /// Tissues in this organ
    pub tissues: Vec<Tissue>,
    /// Organ function
    pub function: OrganFunction,
}

/// Organ types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganType {
    /// CPU management organ
    ProcessingCore,
    /// Memory management organ
    MemoryPool,
    /// I/O subsystem organ
    IoSubsystem,
    /// Network stack organ
    NetworkStack,
    /// Security monitoring organ
    SecurityMonitor,
}

/// Organ function status
#[derive(Debug, Clone, Default)]
pub struct OrganFunction {
    /// Current capacity
    pub capacity: f64,
    /// Current load
    pub load: f64,
    /// Efficiency
    pub efficiency: f64,
    /// Fault rate
    pub fault_rate: f64,
}

impl Organ {
    /// Create a new organ
    pub fn new(id: u64, organ_type: OrganType) -> Self {
        Self {
            id,
            organ_type,
            tissues: Vec::new(),
            function: OrganFunction::default(),
        }
    }

    /// Add tissue to organ
    pub fn add_tissue(&mut self, tissue: Tissue) {
        self.tissues.push(tissue);
    }

    /// Update organ
    pub fn update(&mut self, morphogens: &BTreeMap<MorphogenType, f64>, dt: f64) {
        for tissue in &mut self.tissues {
            tissue.update(morphogens, dt);
        }

        // Calculate organ function
        let total_health: f64 = self.tissues.iter().map(|t| t.state.health).sum();
        let total_throughput: f64 = self.tissues.iter().map(|t| t.state.throughput).sum();
        let num_tissues = self.tissues.len() as f64;

        if num_tissues > 0.0 {
            self.function.capacity = total_health / num_tissues;
            self.function.load = total_throughput / (total_health + 1.0);
            self.function.efficiency = (1.0 - self.function.load / 2.0).max(0.1);
            self.function.fault_rate = (1.0 - total_health / num_tissues) * 0.1;
        }
    }

    /// Get total cell count
    pub fn cell_count(&self) -> usize {
        self.tissues.iter().map(|t| t.size()).sum()
    }
}

/// Homeostasis controller
#[derive(Debug, Clone)]
pub struct HomeostasisController {
    /// Target setpoints for morphogens
    setpoints: BTreeMap<MorphogenType, f64>,
    /// PID gains
    kp: f64,
    ki: f64,
    kd: f64,
    /// Integral error accumulator
    integral: BTreeMap<MorphogenType, f64>,
    /// Previous error
    prev_error: BTreeMap<MorphogenType, f64>,
}

impl HomeostasisController {
    /// Create a new homeostasis controller
    pub fn new() -> Self {
        Self {
            setpoints: BTreeMap::new(),
            kp: 0.5,
            ki: 0.1,
            kd: 0.2,
            integral: BTreeMap::new(),
            prev_error: BTreeMap::new(),
        }
    }

    /// Set target setpoint
    pub fn set_target(&mut self, morph_type: MorphogenType, target: f64) {
        self.setpoints.insert(morph_type, target);
    }

    /// Calculate control signal
    pub fn control(&mut self, morph_type: MorphogenType, current: f64, dt: f64) -> f64 {
        let target = self.setpoints.get(&morph_type).copied().unwrap_or(1.0);
        let error = target - current;

        // Update integral
        let prev_integral = self.integral.get(&morph_type).copied().unwrap_or(0.0);
        let new_integral = prev_integral + error * dt;
        self.integral.insert(morph_type, new_integral);

        // Calculate derivative
        let prev_error = self.prev_error.get(&morph_type).copied().unwrap_or(error);
        let derivative = (error - prev_error) / dt;
        self.prev_error.insert(morph_type, error);

        // PID control
        self.kp * error + self.ki * new_integral + self.kd * derivative
    }
}

/// Regeneration manager
#[derive(Debug, Clone)]
pub struct RegenerationManager {
    /// Stem cell pool
    stem_cells: Vec<Cell>,
    /// Regeneration rate
    regen_rate: f64,
    /// Maximum stem cells
    max_stem_cells: usize,
    /// Next cell ID
    next_id: u64,
}

impl RegenerationManager {
    /// Create a new regeneration manager
    pub fn new() -> Self {
        Self {
            stem_cells: Vec::new(),
            regen_rate: 0.1,
            max_stem_cells: 100,
            next_id: 1,
        }
    }

    /// Add stem cells
    pub fn add_stem_cells(&mut self, count: usize, center: Position) {
        for i in 0..count {
            if self.stem_cells.len() >= self.max_stem_cells {
                break;
            }

            let position = Position::new(
                center.x + (i as f64 * 0.1) % 1.0,
                center.y + (i as f64 * 0.07) % 1.0,
                center.z + (i as f64 * 0.13) % 1.0,
            );

            self.stem_cells.push(Cell::new_stem(self.next_id, position));
            self.next_id += 1;
        }
    }

    /// Request regeneration for a tissue
    pub fn regenerate(&mut self, tissue: &mut Tissue, count: usize) {
        let to_add = count.min(self.stem_cells.len());

        for _ in 0..to_add {
            if let Some(mut stem) = self.stem_cells.pop() {
                stem.cell_type = tissue.tissue_type;
                tissue.add_cell(stem);
            }
        }
    }

    /// Replenish stem cell pool
    pub fn replenish(&mut self, center: Position) {
        if self.stem_cells.len() < self.max_stem_cells / 2 {
            self.add_stem_cells(10, center);
        }
    }
}

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
        let x = pos.x.floor() as usize;
        let y = pos.y.floor() as usize;
        let z = pos.z.floor() as usize;

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

#[cfg(test)]
mod tests {
    use super::*;

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
