//! Mutation strategies and mutator

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::input::FuzzInput;

// ============================================================================
// MUTATION STRATEGY
// ============================================================================

/// Mutation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationStrategy {
    /// Bit flip
    BitFlip,
    /// Byte flip
    ByteFlip,
    /// Arithmetic (add/subtract small values)
    Arithmetic,
    /// Interesting values (0, 0xFF, 0x7F, etc.)
    Interesting,
    /// Random bytes
    Random,
    /// Block insertion
    BlockInsert,
    /// Block deletion
    BlockDelete,
    /// Block swap
    BlockSwap,
    /// Dictionary
    Dictionary,
    /// Splice (combine two inputs)
    Splice,
}

impl MutationStrategy {
    /// Get all strategies
    pub fn all() -> &'static [MutationStrategy] {
        &[
            Self::BitFlip,
            Self::ByteFlip,
            Self::Arithmetic,
            Self::Interesting,
            Self::Random,
            Self::BlockInsert,
            Self::BlockDelete,
            Self::BlockSwap,
            Self::Dictionary,
            Self::Splice,
        ]
    }

    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::BitFlip => "bit_flip",
            Self::ByteFlip => "byte_flip",
            Self::Arithmetic => "arithmetic",
            Self::Interesting => "interesting",
            Self::Random => "random",
            Self::BlockInsert => "block_insert",
            Self::BlockDelete => "block_delete",
            Self::BlockSwap => "block_swap",
            Self::Dictionary => "dictionary",
            Self::Splice => "splice",
        }
    }
}

// ============================================================================
// MUTATOR
// ============================================================================

/// Input mutator
pub struct Mutator {
    /// Random seed
    seed: u64,
    /// Dictionary of interesting tokens
    dictionary: Vec<Vec<u8>>,
    /// Maximum input size
    max_size: usize,
    /// Interesting 8-bit values
    interesting_8: Vec<u8>,
    /// Interesting 16-bit values
    interesting_16: Vec<u16>,
    /// Interesting 32-bit values
    interesting_32: Vec<u32>,
}

impl Mutator {
    /// Create a new mutator
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            dictionary: Vec::new(),
            max_size: 1024 * 1024, // 1 MB
            interesting_8: vec![0, 1, 0x7F, 0x80, 0xFF],
            interesting_16: vec![0, 1, 0x7FFF, 0x8000, 0xFFFF],
            interesting_32: vec![0, 1, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF],
        }
    }

    /// Add dictionary entry
    pub fn add_dict(&mut self, entry: Vec<u8>) {
        self.dictionary.push(entry);
    }

    /// Set max size
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Get next random value
    pub fn rand(&mut self) -> u64 {
        // Simple xorshift64
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    /// Get random in range
    pub fn rand_range(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.rand() as usize) % max
    }

    /// Mutate input
    pub fn mutate(&mut self, input: &FuzzInput) -> FuzzInput {
        let strategy = MutationStrategy::all()[self.rand_range(MutationStrategy::all().len())];
        self.mutate_with(input, strategy)
    }

    /// Mutate with specific strategy
    pub fn mutate_with(&mut self, input: &FuzzInput, strategy: MutationStrategy) -> FuzzInput {
        let mut data = input.data.clone();

        if data.is_empty() {
            // Can only insert
            data.push(self.rand() as u8);
            return input.mutate(data);
        }

        match strategy {
            MutationStrategy::BitFlip => {
                let idx = self.rand_range(data.len());
                let bit = self.rand_range(8);
                data[idx] ^= 1 << bit;
            },
            MutationStrategy::ByteFlip => {
                let idx = self.rand_range(data.len());
                data[idx] ^= 0xFF;
            },
            MutationStrategy::Arithmetic => {
                let idx = self.rand_range(data.len());
                let delta = (self.rand_range(35) as i8) - 17; // -17 to +17
                data[idx] = data[idx].wrapping_add(delta as u8);
            },
            MutationStrategy::Interesting => {
                let idx = self.rand_range(data.len());
                let interesting_len = self.interesting_8.len();
                let val_idx = self.rand_range(interesting_len);
                let val = self.interesting_8[val_idx];
                data[idx] = val;
            },
            MutationStrategy::Random => {
                let idx = self.rand_range(data.len());
                data[idx] = self.rand() as u8;
            },
            MutationStrategy::BlockInsert => {
                if data.len() < self.max_size {
                    let idx = self.rand_range(data.len() + 1);
                    let len = self.rand_range(32) + 1;
                    let block: Vec<u8> = (0..len).map(|_| self.rand() as u8).collect();
                    data.splice(idx..idx, block);
                    data.truncate(self.max_size);
                }
            },
            MutationStrategy::BlockDelete => {
                if data.len() > 1 {
                    let len = self.rand_range(data.len().min(32)) + 1;
                    let start = self.rand_range(data.len() - len + 1);
                    data.drain(start..start + len);
                }
            },
            MutationStrategy::BlockSwap => {
                if data.len() >= 2 {
                    let len = self.rand_range(data.len().min(32) / 2).max(1);
                    let pos1 = self.rand_range(data.len() - len);
                    let pos2 = self.rand_range(data.len() - len);
                    if pos1 != pos2 {
                        for i in 0..len {
                            data.swap(pos1 + i, pos2 + i);
                        }
                    }
                }
            },
            MutationStrategy::Dictionary => {
                if !self.dictionary.is_empty() && data.len() < self.max_size {
                    let dict_idx = self.rand_range(self.dictionary.len());
                    let entry = self.dictionary[dict_idx].clone();
                    let idx = self.rand_range(data.len() + 1);
                    data.splice(idx..idx, entry);
                    data.truncate(self.max_size);
                }
            },
            MutationStrategy::Splice => {
                // For splice, we'd need another input - just do random for now
                let idx = self.rand_range(data.len());
                data[idx] = self.rand() as u8;
            },
        }

        input.mutate(data)
    }
}

impl Default for Mutator {
    fn default() -> Self {
        Self::new(12345)
    }
}
