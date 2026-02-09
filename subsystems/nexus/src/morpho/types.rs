//! Basic types for morphogenetic kernel structures.

extern crate alloc;

use alloc::vec::Vec;

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

    #[inline]
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
