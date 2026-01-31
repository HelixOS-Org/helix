//! # Fuzzing Framework
//!
//! Kernel fuzzing for discovering edge cases and vulnerabilities.
//!
//! ## Key Features
//!
//! - **Coverage-Guided Fuzzing**: Maximize code coverage
//! - **Mutation Strategies**: Smart input mutation
//! - **Crash Detection**: Automatic crash analysis
//! - **Corpus Management**: Efficient test case storage
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `input`: Fuzz input representation
//! - `mutation`: Mutation strategies and mutator
//! - `corpus`: Corpus management
//! - `result`: Fuzz execution results
//! - `stats`: Fuzzing statistics
//! - `fuzzer`: Main fuzzer implementation

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod input;
pub mod mutation;
pub mod corpus;
pub mod result;
pub mod stats;
pub mod fuzzer;

// Re-export input
pub use input::FuzzInput;

// Re-export mutation
pub use mutation::{MutationStrategy, Mutator};

// Re-export corpus
pub use corpus::Corpus;

// Re-export result
pub use result::FuzzResult;

// Re-export stats
pub use stats::FuzzStats;

// Re-export fuzzer
pub use fuzzer::Fuzzer;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_input() {
        let input = FuzzInput::new(vec![1, 2, 3]);
        assert_eq!(input.len(), 3);
        assert!(!input.is_empty());
        assert_eq!(input.generation, 0);

        let child = input.mutate(vec![4, 5, 6]);
        assert_eq!(child.generation, 1);
        assert_eq!(child.parent, Some(input.id));
    }

    #[test]
    fn test_mutator() {
        let mut mutator = Mutator::new(42);
        let input = FuzzInput::new(vec![0, 1, 2, 3, 4, 5]);

        let mutated = mutator.mutate(&input);
        assert!(!mutated.data.is_empty());
    }

    #[test]
    fn test_corpus() {
        let mut corpus = Corpus::new(100);

        let input1 = FuzzInput::new(vec![1, 2, 3]).with_coverage(111);
        let input2 = FuzzInput::new(vec![4, 5, 6]).with_coverage(222);
        let input3 = FuzzInput::new(vec![7, 8, 9]).with_coverage(111); // Duplicate coverage

        assert!(corpus.add(input1));
        assert!(corpus.add(input2));
        assert!(!corpus.add(input3)); // Should be rejected

        assert_eq!(corpus.len(), 2);
    }

    #[test]
    fn test_fuzzer() {
        let mut fuzzer = Fuzzer::new(|data| {
            // Simulate finding a crash on specific input
            if data.len() > 5 && data[0] == 0xFF {
                FuzzResult::Crash { message: "test crash".into() }
            } else {
                FuzzResult::Ok
            }
        });

        fuzzer.add_seed(vec![1, 2, 3, 4, 5]);
        let results = fuzzer.run(100);

        let stats = fuzzer.stats();
        assert_eq!(stats.executions, 100);
    }
}
