//! # Scene Streaming
//!
//! Dynamic scene loading and unloading.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::ecs::Entity;

/// Streaming manager
pub struct StreamingManager {
    cells: BTreeMap<CellId, StreamingCell>,
    loaded_cells: Vec<CellId>,
    loading_queue: Vec<CellId>,
    unload_queue: Vec<CellId>,
    config: StreamingConfig,
    camera_cell: Option<CellId>,
}

impl StreamingManager {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            cells: BTreeMap::new(),
            loaded_cells: Vec::new(),
            loading_queue: Vec::new(),
            unload_queue: Vec::new(),
            config,
            camera_cell: None,
        }
    }

    /// Register a streaming cell
    pub fn register_cell(&mut self, id: CellId, cell: StreamingCell) {
        self.cells.insert(id, cell);
    }

    /// Update streaming based on camera position
    pub fn update(&mut self, camera_pos: [f32; 3]) {
        // Find camera cell
        let camera_cell = self.get_cell_at(camera_pos);

        if self.camera_cell != camera_cell {
            self.camera_cell = camera_cell;
            self.update_streaming_state(camera_pos);
        }

        // Process loading queue
        self.process_loading();

        // Process unloading queue
        self.process_unloading();
    }

    fn get_cell_at(&self, pos: [f32; 3]) -> Option<CellId> {
        let cell_x = (pos[0] / self.config.cell_size).floor() as i32;
        let cell_z = (pos[2] / self.config.cell_size).floor() as i32;

        let id = CellId {
            x: cell_x,
            y: 0,
            z: cell_z,
        };

        if self.cells.contains_key(&id) {
            Some(id)
        } else {
            None
        }
    }

    fn update_streaming_state(&mut self, camera_pos: [f32; 3]) {
        let load_radius = self.config.load_distance / self.config.cell_size;
        let unload_radius = self.config.unload_distance / self.config.cell_size;

        // Find cells to load
        for (id, _cell) in &self.cells {
            let cell_center = self.cell_center(*id);
            let distance = Self::distance(camera_pos, cell_center);

            if distance < load_radius * self.config.cell_size {
                if !self.loaded_cells.contains(id) && !self.loading_queue.contains(id) {
                    self.loading_queue.push(*id);
                }
            } else if distance > unload_radius * self.config.cell_size {
                if self.loaded_cells.contains(id) && !self.unload_queue.contains(id) {
                    self.unload_queue.push(*id);
                }
            }
        }

        // Sort by distance for priority loading
        let cam_pos = camera_pos;
        self.loading_queue.sort_by(|a, b| {
            let dist_a = Self::distance(cam_pos, self.cell_center(*a));
            let dist_b = Self::distance(cam_pos, self.cell_center(*b));
            dist_a.partial_cmp(&dist_b).unwrap()
        });
    }

    fn cell_center(&self, id: CellId) -> [f32; 3] {
        [
            (id.x as f32 + 0.5) * self.config.cell_size,
            (id.y as f32 + 0.5) * self.config.cell_size,
            (id.z as f32 + 0.5) * self.config.cell_size,
        ]
    }

    fn process_loading(&mut self) {
        let max_loads = self.config.max_concurrent_loads;
        let mut loads_this_frame = 0;

        while loads_this_frame < max_loads && !self.loading_queue.is_empty() {
            let id = self.loading_queue.remove(0);

            if let Some(cell) = self.cells.get_mut(&id) {
                cell.state = CellState::Loading;
                // Would trigger async load
                cell.state = CellState::Loaded;
                self.loaded_cells.push(id);
            }

            loads_this_frame += 1;
        }
    }

    fn process_unloading(&mut self) {
        while !self.unload_queue.is_empty() {
            let id = self.unload_queue.remove(0);

            if let Some(cell) = self.cells.get_mut(&id) {
                cell.state = CellState::Unloaded;
            }

            self.loaded_cells.retain(|&c| c != id);
        }
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let dz = b[2] - a[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Get loaded cells
    pub fn loaded_cells(&self) -> &[CellId] {
        &self.loaded_cells
    }

    /// Check if cell is loaded
    pub fn is_loaded(&self, id: CellId) -> bool {
        self.loaded_cells.contains(&id)
    }
}

/// Streaming configuration
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub cell_size: f32,
    pub load_distance: f32,
    pub unload_distance: f32,
    pub max_concurrent_loads: u32,
    pub preload_neighbors: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            cell_size: 256.0,
            load_distance: 512.0,
            unload_distance: 768.0,
            max_concurrent_loads: 2,
            preload_neighbors: true,
        }
    }
}

/// Cell identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellId {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Streaming cell
#[derive(Debug, Clone)]
pub struct StreamingCell {
    pub bounds: [f32; 6], // AABB
    pub state: CellState,
    pub entities: Vec<Entity>,
    pub lod_levels: u8,
    pub asset_path: String,
}

/// Cell state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Unloaded,
    Loading,
    Loaded,
    Unloading,
}

/// World streamer for open world
pub struct WorldStreamer {
    tiles: BTreeMap<TileCoord, WorldTile>,
    loaded_tiles: Vec<TileCoord>,
    tile_size: f32,
    view_distance: f32,
}

impl WorldStreamer {
    pub fn new(tile_size: f32, view_distance: f32) -> Self {
        Self {
            tiles: BTreeMap::new(),
            loaded_tiles: Vec::new(),
            tile_size,
            view_distance,
        }
    }

    /// Update loaded tiles
    pub fn update(&mut self, camera_pos: [f32; 3]) {
        let cam_tile_x = (camera_pos[0] / self.tile_size).floor() as i32;
        let cam_tile_z = (camera_pos[2] / self.tile_size).floor() as i32;

        let tile_radius = (self.view_distance / self.tile_size).ceil() as i32;

        // Collect tiles to load
        let mut tiles_to_load = Vec::new();
        for z in (cam_tile_z - tile_radius)..=(cam_tile_z + tile_radius) {
            for x in (cam_tile_x - tile_radius)..=(cam_tile_x + tile_radius) {
                let coord = TileCoord { x, z };
                if !self.loaded_tiles.contains(&coord) {
                    tiles_to_load.push(coord);
                }
            }
        }

        // Collect tiles to unload
        let tiles_to_unload: Vec<_> = self
            .loaded_tiles
            .iter()
            .filter(|coord| {
                (coord.x - cam_tile_x).abs() > tile_radius + 1
                    || (coord.z - cam_tile_z).abs() > tile_radius + 1
            })
            .copied()
            .collect();

        // Process
        for coord in tiles_to_load {
            self.load_tile(coord);
        }

        for coord in tiles_to_unload {
            self.unload_tile(coord);
        }
    }

    fn load_tile(&mut self, coord: TileCoord) {
        if let Some(tile) = self.tiles.get_mut(&coord) {
            tile.state = TileState::Loaded;
            self.loaded_tiles.push(coord);
        }
    }

    fn unload_tile(&mut self, coord: TileCoord) {
        if let Some(tile) = self.tiles.get_mut(&coord) {
            tile.state = TileState::Unloaded;
        }
        self.loaded_tiles.retain(|&c| c != coord);
    }

    /// Register a world tile
    pub fn register_tile(&mut self, coord: TileCoord, tile: WorldTile) {
        self.tiles.insert(coord, tile);
    }
}

/// Tile coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileCoord {
    pub x: i32,
    pub z: i32,
}

/// World tile
#[derive(Debug, Clone)]
pub struct WorldTile {
    pub state: TileState,
    pub heightmap: Option<u64>,
    pub detail_layers: Vec<DetailLayer>,
}

/// Tile state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileState {
    Unloaded,
    Loading,
    Loaded,
}

/// Detail layer (grass, rocks, etc.)
#[derive(Debug, Clone)]
pub struct DetailLayer {
    pub mesh_id: u64,
    pub density: f32,
    pub min_scale: f32,
    pub max_scale: f32,
}
