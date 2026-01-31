//! Filesystem fragmentation analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::Inode;

// ============================================================================
// DEFRAG PRIORITY
// ============================================================================

/// Defragmentation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefragPriority {
    /// No need to defrag
    None     = 0,
    /// Low priority
    Low      = 1,
    /// Medium priority
    Medium   = 2,
    /// High priority
    High     = 3,
    /// Critical priority
    Critical = 4,
}

// ============================================================================
// FRAGMENTATION SCORE
// ============================================================================

/// Fragmentation score for a file
#[derive(Debug, Clone)]
pub struct FragmentationScore {
    /// Inode
    pub inode: Inode,
    /// Number of extents/fragments
    pub fragment_count: u32,
    /// File size
    pub size: u64,
    /// Fragmentation score (0.0 = contiguous, 1.0 = highly fragmented)
    pub score: f64,
    /// Defragmentation priority
    pub priority: DefragPriority,
}

// ============================================================================
// FRAGMENTATION ANALYZER
// ============================================================================

/// Analyzes filesystem fragmentation
pub struct FragmentationAnalyzer {
    /// File fragmentation scores
    file_scores: BTreeMap<Inode, FragmentationScore>,
    /// Free space fragmentation
    free_space_fragmentation: f64,
    /// Total fragments
    total_fragments: u64,
    /// Total files analyzed
    total_files: u64,
}

impl FragmentationAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            file_scores: BTreeMap::new(),
            free_space_fragmentation: 0.0,
            total_fragments: 0,
            total_files: 0,
        }
    }

    /// Analyze file fragmentation
    pub fn analyze_file(&mut self, inode: Inode, size: u64, fragment_count: u32, is_hot: bool) {
        let expected_fragments = ((size + 4095) / 4096 / 1024).max(1) as u32;
        let score = if expected_fragments > 0 {
            ((fragment_count as f64 / expected_fragments as f64) - 1.0).max(0.0) / 10.0
        } else {
            0.0
        };
        let score = score.min(1.0);

        let priority = match (score, is_hot) {
            (s, _) if s < 0.1 => DefragPriority::None,
            (s, true) if s > 0.5 => DefragPriority::Critical,
            (s, true) if s > 0.3 => DefragPriority::High,
            (s, _) if s > 0.5 => DefragPriority::Medium,
            _ => DefragPriority::Low,
        };

        self.file_scores.insert(inode, FragmentationScore {
            inode,
            fragment_count,
            size,
            score,
            priority,
        });

        self.total_fragments += fragment_count as u64;
        self.total_files += 1;
    }

    /// Set free space fragmentation
    pub fn set_free_space_fragmentation(&mut self, score: f64) {
        self.free_space_fragmentation = score;
    }

    /// Get file fragmentation score
    pub fn get_score(&self, inode: Inode) -> Option<&FragmentationScore> {
        self.file_scores.get(&inode)
    }

    /// Get files needing defragmentation
    pub fn files_needing_defrag(&self, min_priority: DefragPriority) -> Vec<&FragmentationScore> {
        self.file_scores
            .values()
            .filter(|s| s.priority >= min_priority)
            .collect()
    }

    /// Get most fragmented files
    pub fn most_fragmented(&self, n: usize) -> Vec<&FragmentationScore> {
        let mut files: Vec<_> = self.file_scores.values().collect();
        files.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        files.truncate(n);
        files
    }

    /// Get average fragmentation
    pub fn average_fragmentation(&self) -> f64 {
        if self.file_scores.is_empty() {
            0.0
        } else {
            let sum: f64 = self.file_scores.values().map(|s| s.score).sum();
            sum / self.file_scores.len() as f64
        }
    }

    /// Get overall filesystem health
    pub fn filesystem_health(&self) -> f64 {
        let avg_frag = self.average_fragmentation();
        let free_frag = self.free_space_fragmentation;

        1.0 - (avg_frag * 0.7 + free_frag * 0.3)
    }
}

impl Default for FragmentationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
