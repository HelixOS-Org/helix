//! Bitmap-based block allocator.
//!
//! Uses hierarchical bitmaps for fast free block finding.
//! Each bit represents one block (0 = free, 1 = used).

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::core::atomic::AtomicBitset;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Bits per word in bitmap
pub const BITS_PER_WORD: usize = 64;

/// Words per bitmap block (4096 bytes / 8 bytes per u64)
pub const WORDS_PER_BLOCK: usize = 512;

/// Bits per bitmap block
pub const BITS_PER_BLOCK: usize = WORDS_PER_BLOCK * BITS_PER_WORD;

/// Maximum bitmap size (supports up to 2^48 blocks)
pub const MAX_BITMAP_BLOCKS: usize = 262144; // 256K blocks = 8PB with 4K blocks

// ============================================================================
// Bitmap Word Operations
// ============================================================================

/// Find first zero bit in word.
#[inline]
pub fn find_first_zero(word: u64) -> Option<u32> {
    let inverted = !word;
    if inverted == 0 {
        None
    } else {
        Some(inverted.trailing_zeros())
    }
}

/// Find first set bit in word.
#[inline]
pub fn find_first_set(word: u64) -> Option<u32> {
    if word == 0 {
        None
    } else {
        Some(word.trailing_zeros())
    }
}

/// Count zero bits in word.
#[inline]
pub fn count_zeros(word: u64) -> u32 {
    (!word).count_ones()
}

/// Find contiguous zeros in word.
pub fn find_contiguous_zeros(word: u64, count: u32) -> Option<u32> {
    if count == 0 {
        return Some(0);
    }
    if count > 64 {
        return None;
    }
    
    // Create mask with 'count' ones
    let mask = if count == 64 {
        u64::MAX
    } else {
        (1u64 << count) - 1
    };
    
    // Slide mask across word
    for pos in 0..=(64 - count) {
        let shifted_mask = mask << pos;
        if (word & shifted_mask) == 0 {
            return Some(pos);
        }
    }
    
    None
}

/// Set bits in range [start, start+count)
#[inline]
pub fn set_bits_range(word: &mut u64, start: u32, count: u32) {
    debug_assert!(start < 64 && start + count <= 64);
    let mask = if count == 64 {
        u64::MAX
    } else {
        ((1u64 << count) - 1) << start
    };
    *word |= mask;
}

/// Clear bits in range [start, start+count)
#[inline]
pub fn clear_bits_range(word: &mut u64, start: u32, count: u32) {
    debug_assert!(start < 64 && start + count <= 64);
    let mask = if count == 64 {
        u64::MAX
    } else {
        ((1u64 << count) - 1) << start
    };
    *word &= !mask;
}

// ============================================================================
// Bitmap Block
// ============================================================================

/// Single bitmap block (covers BITS_PER_BLOCK blocks).
#[repr(C, align(4096))]
pub struct BitmapBlock {
    /// Bitmap words
    words: [AtomicU64; WORDS_PER_BLOCK],
}

impl BitmapBlock {
    /// Create new empty bitmap (all free)
    pub const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            words: [ZERO; WORDS_PER_BLOCK],
        }
    }
    
    /// Create new full bitmap (all used)
    pub fn new_full() -> Self {
        const MAX: AtomicU64 = AtomicU64::new(u64::MAX);
        Self {
            words: [MAX; WORDS_PER_BLOCK],
        }
    }
    
    /// Check if bit is set
    #[inline]
    pub fn is_set(&self, bit: usize) -> bool {
        debug_assert!(bit < BITS_PER_BLOCK);
        let word_idx = bit / BITS_PER_WORD;
        let bit_idx = bit % BITS_PER_WORD;
        let word = self.words[word_idx].load(Ordering::Relaxed);
        (word & (1u64 << bit_idx)) != 0
    }
    
    /// Set bit (mark as used)
    #[inline]
    pub fn set(&self, bit: usize) {
        debug_assert!(bit < BITS_PER_BLOCK);
        let word_idx = bit / BITS_PER_WORD;
        let bit_idx = bit % BITS_PER_WORD;
        self.words[word_idx].fetch_or(1u64 << bit_idx, Ordering::Relaxed);
    }
    
    /// Clear bit (mark as free)
    #[inline]
    pub fn clear(&self, bit: usize) {
        debug_assert!(bit < BITS_PER_BLOCK);
        let word_idx = bit / BITS_PER_WORD;
        let bit_idx = bit % BITS_PER_WORD;
        self.words[word_idx].fetch_and(!(1u64 << bit_idx), Ordering::Relaxed);
    }
    
    /// Set range of bits atomically
    pub fn set_range(&self, start: usize, count: usize) {
        debug_assert!(start + count <= BITS_PER_BLOCK);
        
        let mut remaining = count;
        let mut bit = start;
        
        while remaining > 0 {
            let word_idx = bit / BITS_PER_WORD;
            let bit_in_word = bit % BITS_PER_WORD;
            let bits_this_word = (BITS_PER_WORD - bit_in_word).min(remaining);
            
            // Create mask
            let mask = if bits_this_word == 64 {
                u64::MAX
            } else {
                ((1u64 << bits_this_word) - 1) << bit_in_word
            };
            
            self.words[word_idx].fetch_or(mask, Ordering::Relaxed);
            
            bit += bits_this_word;
            remaining -= bits_this_word;
        }
    }
    
    /// Clear range of bits atomically
    pub fn clear_range(&self, start: usize, count: usize) {
        debug_assert!(start + count <= BITS_PER_BLOCK);
        
        let mut remaining = count;
        let mut bit = start;
        
        while remaining > 0 {
            let word_idx = bit / BITS_PER_WORD;
            let bit_in_word = bit % BITS_PER_WORD;
            let bits_this_word = (BITS_PER_WORD - bit_in_word).min(remaining);
            
            // Create mask
            let mask = if bits_this_word == 64 {
                u64::MAX
            } else {
                ((1u64 << bits_this_word) - 1) << bit_in_word
            };
            
            self.words[word_idx].fetch_and(!mask, Ordering::Relaxed);
            
            bit += bits_this_word;
            remaining -= bits_this_word;
        }
    }
    
    /// Find first free bit
    pub fn find_first_free(&self) -> Option<usize> {
        for (i, word) in self.words.iter().enumerate() {
            let w = word.load(Ordering::Relaxed);
            if w != u64::MAX {
                if let Some(bit) = find_first_zero(w) {
                    return Some(i * BITS_PER_WORD + bit as usize);
                }
            }
        }
        None
    }
    
    /// Find first free bit starting from position
    pub fn find_first_free_from(&self, start: usize) -> Option<usize> {
        if start >= BITS_PER_BLOCK {
            return None;
        }
        
        let start_word = start / BITS_PER_WORD;
        let start_bit = start % BITS_PER_WORD;
        
        // Check first word (might be partial)
        let first_word = self.words[start_word].load(Ordering::Relaxed);
        let masked = first_word | ((1u64 << start_bit) - 1); // Mask out bits before start
        if masked != u64::MAX {
            if let Some(bit) = find_first_zero(masked) {
                return Some(start_word * BITS_PER_WORD + bit as usize);
            }
        }
        
        // Check remaining words
        for i in (start_word + 1)..WORDS_PER_BLOCK {
            let w = self.words[i].load(Ordering::Relaxed);
            if w != u64::MAX {
                if let Some(bit) = find_first_zero(w) {
                    return Some(i * BITS_PER_WORD + bit as usize);
                }
            }
        }
        
        None
    }
    
    /// Find contiguous free blocks
    pub fn find_contiguous_free(&self, count: usize) -> Option<usize> {
        if count == 0 {
            return Some(0);
        }
        if count > BITS_PER_BLOCK {
            return None;
        }
        
        let mut run_start = 0;
        let mut run_length = 0;
        
        for (word_idx, word) in self.words.iter().enumerate() {
            let w = word.load(Ordering::Relaxed);
            
            if w == 0 {
                // Entire word is free
                if run_length == 0 {
                    run_start = word_idx * BITS_PER_WORD;
                }
                run_length += BITS_PER_WORD;
                if run_length >= count {
                    return Some(run_start);
                }
            } else if w == u64::MAX {
                // Entire word is used
                run_length = 0;
            } else {
                // Mixed word - check bit by bit at boundaries
                for bit in 0..BITS_PER_WORD {
                    if (w & (1u64 << bit)) == 0 {
                        if run_length == 0 {
                            run_start = word_idx * BITS_PER_WORD + bit;
                        }
                        run_length += 1;
                        if run_length >= count {
                            return Some(run_start);
                        }
                    } else {
                        run_length = 0;
                    }
                }
            }
        }
        
        None
    }
    
    /// Count free bits
    pub fn count_free(&self) -> usize {
        let mut count = 0;
        for word in &self.words {
            let w = word.load(Ordering::Relaxed);
            count += count_zeros(w) as usize;
        }
        count
    }
    
    /// Count used bits
    pub fn count_used(&self) -> usize {
        BITS_PER_BLOCK - self.count_free()
    }
    
    /// Check if all bits are set (full)
    pub fn is_full(&self) -> bool {
        self.words.iter().all(|w| w.load(Ordering::Relaxed) == u64::MAX)
    }
    
    /// Check if all bits are clear (empty)
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| w.load(Ordering::Relaxed) == 0)
    }
    
    /// Load from bytes
    pub fn from_bytes(bytes: &[u8; 4096]) -> Self {
        let block = Self::new();
        for (i, chunk) in bytes.chunks_exact(8).enumerate() {
            let value = u64::from_le_bytes(chunk.try_into().unwrap());
            block.words[i].store(value, Ordering::Relaxed);
        }
        block
    }
    
    /// Store to bytes
    pub fn to_bytes(&self, bytes: &mut [u8; 4096]) {
        for (i, word) in self.words.iter().enumerate() {
            let value = word.load(Ordering::Relaxed);
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&value.to_le_bytes());
        }
    }
}

impl Default for BitmapBlock {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hierarchical Bitmap
// ============================================================================

/// Summary level for hierarchical bitmap.
///
/// Each bit in summary represents whether corresponding bitmap block
/// has any free blocks.
pub struct BitmapSummary {
    /// Summary words (1 bit per bitmap block)
    /// Bit = 0 means bitmap block has free space
    /// Bit = 1 means bitmap block is full
    words: [AtomicU64; 4096], // Supports up to 262144 bitmap blocks
}

impl Default for BitmapSummary {
    fn default() -> Self {
        Self::new()
    }
}

impl BitmapSummary {
    /// Create new summary (all blocks have free space)
    pub const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            words: [ZERO; 4096],
        }
    }
    
    /// Mark bitmap block as full
    pub fn mark_full(&self, block_idx: usize) {
        let word_idx = block_idx / 64;
        let bit_idx = block_idx % 64;
        self.words[word_idx].fetch_or(1u64 << bit_idx, Ordering::Relaxed);
    }
    
    /// Mark bitmap block as having free space
    pub fn mark_has_free(&self, block_idx: usize) {
        let word_idx = block_idx / 64;
        let bit_idx = block_idx % 64;
        self.words[word_idx].fetch_and(!(1u64 << bit_idx), Ordering::Relaxed);
    }
    
    /// Check if bitmap block has free space
    pub fn has_free(&self, block_idx: usize) -> bool {
        let word_idx = block_idx / 64;
        let bit_idx = block_idx % 64;
        let word = self.words[word_idx].load(Ordering::Relaxed);
        (word & (1u64 << bit_idx)) == 0
    }
    
    /// Find first bitmap block with free space
    pub fn find_first_with_free(&self, num_blocks: usize) -> Option<usize> {
        let num_words = (num_blocks + 63) / 64;
        
        for (i, word) in self.words[..num_words].iter().enumerate() {
            let w = word.load(Ordering::Relaxed);
            if w != u64::MAX {
                // This word has at least one bit clear
                if let Some(bit) = find_first_zero(w) {
                    let block_idx = i * 64 + bit as usize;
                    if block_idx < num_blocks {
                        return Some(block_idx);
                    }
                }
            }
        }
        None
    }
    
    /// Find first bitmap block with free space, starting from position
    pub fn find_first_with_free_from(&self, start: usize, num_blocks: usize) -> Option<usize> {
        let start_word = start / 64;
        let start_bit = start % 64;
        let num_words = (num_blocks + 63) / 64;
        
        // Check first word
        if start_word < num_words {
            let first = self.words[start_word].load(Ordering::Relaxed);
            let masked = first | ((1u64 << start_bit) - 1);
            if masked != u64::MAX {
                if let Some(bit) = find_first_zero(masked) {
                    let block_idx = start_word * 64 + bit as usize;
                    if block_idx < num_blocks {
                        return Some(block_idx);
                    }
                }
            }
        }
        
        // Check remaining words
        for i in (start_word + 1)..num_words {
            let w = self.words[i].load(Ordering::Relaxed);
            if w != u64::MAX {
                if let Some(bit) = find_first_zero(w) {
                    let block_idx = i * 64 + bit as usize;
                    if block_idx < num_blocks {
                        return Some(block_idx);
                    }
                }
            }
        }
        
        None
    }
}

// ============================================================================
// Bitmap Allocator State
// ============================================================================

/// State for bitmap-based allocator.
#[derive(Default)]
pub struct BitmapAllocState {
    /// Number of bitmap blocks
    num_blocks: u32,
    /// Total blocks managed
    total_blocks: u64,
    /// Free blocks
    free_blocks: AtomicU64,
    /// Last allocation position (for next-fit)
    last_alloc_pos: AtomicU64,
    /// Summary bitmap
    summary: BitmapSummary,
}

impl BitmapAllocState {
    /// Create new state
    pub fn new(total_blocks: u64) -> Self {
        let num_bitmap_blocks = ((total_blocks as usize + BITS_PER_BLOCK - 1) / BITS_PER_BLOCK) as u32;
        
        Self {
            num_blocks: num_bitmap_blocks,
            total_blocks,
            free_blocks: AtomicU64::new(total_blocks),
            last_alloc_pos: AtomicU64::new(0),
            summary: BitmapSummary::new(),
        }
    }
    
    /// Get number of bitmap blocks
    #[inline]
    pub fn num_bitmap_blocks(&self) -> u32 {
        self.num_blocks
    }
    
    /// Get total blocks
    #[inline]
    pub fn total_blocks(&self) -> u64 {
        self.total_blocks
    }
    
    /// Get free blocks
    #[inline]
    pub fn free_blocks(&self) -> u64 {
        self.free_blocks.load(Ordering::Relaxed)
    }
    
    /// Decrease free count
    pub fn consume(&self, count: u64) -> bool {
        let mut current = self.free_blocks.load(Ordering::Relaxed);
        loop {
            if current < count {
                return false;
            }
            match self.free_blocks.compare_exchange_weak(
                current,
                current - count,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(c) => current = c,
            }
        }
    }
    
    /// Increase free count
    pub fn release(&self, count: u64) {
        self.free_blocks.fetch_add(count, Ordering::Relaxed);
    }
    
    /// Update last allocation position
    pub fn update_last_pos(&self, pos: u64) {
        self.last_alloc_pos.store(pos, Ordering::Relaxed);
    }
    
    /// Get last allocation position
    pub fn last_pos(&self) -> u64 {
        self.last_alloc_pos.load(Ordering::Relaxed)
    }
    
    /// Get summary
    #[inline]
    pub fn summary(&self) -> &BitmapSummary {
        &self.summary
    }
    
    /// Convert block number to bitmap position
    #[inline]
    pub fn block_to_bitmap(&self, block: u64) -> (usize, usize) {
        let bitmap_block = (block / BITS_PER_BLOCK as u64) as usize;
        let bit_in_block = (block % BITS_PER_BLOCK as u64) as usize;
        (bitmap_block, bit_in_block)
    }
    
    /// Convert bitmap position to block number
    #[inline]
    pub fn bitmap_to_block(&self, bitmap_block: usize, bit: usize) -> u64 {
        bitmap_block as u64 * BITS_PER_BLOCK as u64 + bit as u64
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_find_first_zero() {
        assert_eq!(find_first_zero(0), Some(0));
        assert_eq!(find_first_zero(1), Some(1));
        assert_eq!(find_first_zero(0b111), Some(3));
        assert_eq!(find_first_zero(u64::MAX), None);
    }
    
    #[test]
    fn test_find_contiguous_zeros() {
        assert_eq!(find_contiguous_zeros(0, 64), Some(0));
        assert_eq!(find_contiguous_zeros(0b1111, 4), Some(4));
        assert_eq!(find_contiguous_zeros(0b11110000, 4), Some(8));
        assert_eq!(find_contiguous_zeros(u64::MAX, 1), None);
    }
    
    #[test]
    fn test_bitmap_block_basic() {
        let block = BitmapBlock::new();
        
        assert!(!block.is_set(0));
        assert!(!block.is_set(100));
        
        block.set(100);
        assert!(block.is_set(100));
        assert!(!block.is_set(99));
        assert!(!block.is_set(101));
        
        block.clear(100);
        assert!(!block.is_set(100));
    }
    
    #[test]
    fn test_bitmap_block_range() {
        let block = BitmapBlock::new();
        
        block.set_range(10, 20);
        
        assert!(!block.is_set(9));
        assert!(block.is_set(10));
        assert!(block.is_set(29));
        assert!(!block.is_set(30));
        
        block.clear_range(15, 5);
        assert!(block.is_set(14));
        assert!(!block.is_set(15));
        assert!(!block.is_set(19));
        assert!(block.is_set(20));
    }
    
    #[test]
    fn test_bitmap_find_free() {
        let block = BitmapBlock::new();
        
        assert_eq!(block.find_first_free(), Some(0));
        
        block.set_range(0, 100);
        assert_eq!(block.find_first_free(), Some(100));
        
        block.set_range(100, BITS_PER_BLOCK - 100);
        assert_eq!(block.find_first_free(), None);
    }
    
    #[test]
    fn test_bitmap_find_contiguous() {
        let block = BitmapBlock::new();
        
        // Set some blocks used
        block.set_range(0, 10);
        block.set_range(20, 5);
        block.set_range(100, 50);
        
        // Find 8 contiguous free
        assert_eq!(block.find_contiguous_free(8), Some(10));
        
        // Find 10 contiguous free
        assert_eq!(block.find_contiguous_free(10), Some(25));
    }
    
    #[test]
    fn test_bitmap_summary() {
        let summary = BitmapSummary::new();
        
        assert!(summary.has_free(0));
        assert!(summary.has_free(100));
        
        summary.mark_full(50);
        assert!(!summary.has_free(50));
        assert!(summary.has_free(49));
        assert!(summary.has_free(51));
        
        summary.mark_has_free(50);
        assert!(summary.has_free(50));
    }
    
    #[test]
    fn test_bitmap_alloc_state() {
        let state = BitmapAllocState::new(1_000_000);
        
        assert_eq!(state.total_blocks(), 1_000_000);
        assert_eq!(state.free_blocks(), 1_000_000);
        
        // Calculate expected bitmap blocks
        let expected = (1_000_000 + BITS_PER_BLOCK - 1) / BITS_PER_BLOCK;
        assert_eq!(state.num_bitmap_blocks() as usize, expected);
        
        // Consume blocks
        assert!(state.consume(100));
        assert_eq!(state.free_blocks(), 999_900);
        
        // Release blocks
        state.release(50);
        assert_eq!(state.free_blocks(), 999_950);
    }
}
