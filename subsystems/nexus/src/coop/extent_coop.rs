// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Extent (cooperative extent allocation)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Extent allocation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopExtentType {
    Data,
    Metadata,
    Prealloc,
    Unwritten,
    Hole,
}

/// Cooperative extent entry
#[derive(Debug, Clone)]
pub struct CoopExtentEntry {
    pub extent_id: u64,
    pub inode: u64,
    pub logical_block: u64,
    pub physical_block: u64,
    pub length: u32,
    pub extent_type: CoopExtentType,
    pub depth: u8,
}

/// Extent tree node
#[derive(Debug, Clone)]
pub struct CoopExtentNode {
    pub node_id: u64,
    pub entries: Vec<CoopExtentEntry>,
    pub depth: u8,
    pub parent: Option<u64>,
}

/// Stats for extent cooperation
#[derive(Debug, Clone)]
pub struct CoopExtentStats {
    pub total_extents: u64,
    pub allocations: u64,
    pub frees: u64,
    pub splits: u64,
    pub merges: u64,
    pub fragmentation_ratio: u64,
}

/// Manager for extent cooperative operations
pub struct CoopExtentManager {
    extents: BTreeMap<u64, CoopExtentEntry>,
    inode_extents: BTreeMap<u64, Vec<u64>>,
    next_id: u64,
    stats: CoopExtentStats,
}

impl CoopExtentManager {
    pub fn new() -> Self {
        Self {
            extents: BTreeMap::new(),
            inode_extents: BTreeMap::new(),
            next_id: 1,
            stats: CoopExtentStats {
                total_extents: 0,
                allocations: 0,
                frees: 0,
                splits: 0,
                merges: 0,
                fragmentation_ratio: 0,
            },
        }
    }

    pub fn allocate(&mut self, inode: u64, logical_block: u64, physical_block: u64, length: u32, extent_type: CoopExtentType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let entry = CoopExtentEntry {
            extent_id: id,
            inode,
            logical_block,
            physical_block,
            length,
            extent_type,
            depth: 0,
        };
        self.extents.insert(id, entry);
        self.inode_extents.entry(inode).or_insert_with(Vec::new).push(id);
        self.stats.total_extents += 1;
        self.stats.allocations += 1;
        id
    }

    pub fn free(&mut self, extent_id: u64) -> bool {
        if let Some(entry) = self.extents.remove(&extent_id) {
            if let Some(list) = self.inode_extents.get_mut(&entry.inode) {
                list.retain(|&id| id != extent_id);
            }
            self.stats.frees += 1;
            true
        } else {
            false
        }
    }

    pub fn lookup(&self, inode: u64, logical_block: u64) -> Option<&CoopExtentEntry> {
        if let Some(extent_ids) = self.inode_extents.get(&inode) {
            for &id in extent_ids {
                if let Some(ext) = self.extents.get(&id) {
                    let end = ext.logical_block + ext.length as u64;
                    if logical_block >= ext.logical_block && logical_block < end {
                        return Some(ext);
                    }
                }
            }
        }
        None
    }

    pub fn extents_for_inode(&self, inode: u64) -> usize {
        self.inode_extents.get(&inode).map(|v| v.len()).unwrap_or(0)
    }

    pub fn stats(&self) -> &CoopExtentStats {
        &self.stats
    }
}
