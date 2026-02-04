//! Regeneration manager for morphogenetic system.

extern crate alloc;

use alloc::vec::Vec;

use super::cell::{Cell, Tissue};
use super::types::Position;

/// Regeneration manager
#[derive(Debug, Clone)]
pub struct RegenerationManager {
    /// Stem cell pool
    pub(crate) stem_cells: Vec<Cell>,
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

impl Default for RegenerationManager {
    fn default() -> Self {
        Self::new()
    }
}
