//! Main fuzzer implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::corpus::Corpus;
use super::input::FuzzInput;
use super::mutation::Mutator;
use super::result::FuzzResult;
use super::stats::FuzzStats;
use crate::core::NexusTimestamp;

// ============================================================================
// FUZZER
// ============================================================================

/// The fuzzer
pub struct Fuzzer {
    /// Target function
    target: Box<dyn Fn(&[u8]) -> FuzzResult + Send + Sync>,
    /// Corpus
    corpus: Corpus,
    /// Mutator
    mutator: Mutator,
    /// Crash inputs
    crashes: Vec<FuzzInput>,
    /// Max crashes to keep
    max_crashes: usize,
    /// Statistics
    stats: FuzzStats,
    /// Running
    running: AtomicBool,
}

impl Fuzzer {
    /// Create a new fuzzer
    pub fn new(target: impl Fn(&[u8]) -> FuzzResult + Send + Sync + 'static) -> Self {
        Self {
            target: Box::new(target),
            corpus: Corpus::default(),
            mutator: Mutator::default(),
            crashes: Vec::new(),
            max_crashes: 1000,
            stats: FuzzStats::default(),
            running: AtomicBool::new(false),
        }
    }

    /// Set corpus
    #[inline(always)]
    pub fn with_corpus(mut self, corpus: Corpus) -> Self {
        self.corpus = corpus;
        self
    }

    /// Set mutator
    #[inline(always)]
    pub fn with_mutator(mut self, mutator: Mutator) -> Self {
        self.mutator = mutator;
        self
    }

    /// Add seed input
    #[inline(always)]
    pub fn add_seed(&mut self, data: Vec<u8>) {
        let input = FuzzInput::new(data);
        self.corpus.add(input);
    }

    /// Run one iteration
    pub fn fuzz_one(&mut self) -> FuzzResult {
        self.stats.executions += 1;

        // Get or generate input
        let input = if self.corpus.is_empty() {
            // Generate random initial input
            let len = self.mutator.rand_range(256) + 1;
            let data: Vec<u8> = (0..len).map(|_| self.mutator.rand() as u8).collect();
            FuzzInput::new(data)
        } else {
            // Mutate existing input
            let mut seed = self.stats.executions;
            let base = self.corpus.random(&mut seed).unwrap().clone();
            self.mutator.mutate(&base)
        };

        // Execute
        let result = (self.target)(&input.data);

        // Process result
        match &result {
            FuzzResult::Crash { .. } => {
                self.stats.crashes += 1;
                if self.crashes.len() < self.max_crashes {
                    self.crashes.push(input.clone());
                }
            },
            FuzzResult::Timeout => {
                self.stats.timeouts += 1;
            },
            FuzzResult::Hang => {
                self.stats.hangs += 1;
            },
            FuzzResult::NewCoverage { coverage_hash } => {
                self.stats.new_coverage += 1;
                let input = input.with_coverage(*coverage_hash).with_score(1.0);
                self.corpus.add(input);
            },
            FuzzResult::Ok => {
                // Still might be worth keeping if random
                if self.corpus.len() < 100 {
                    self.corpus.add(input);
                }
            },
        }

        self.stats.corpus_size = self.corpus.len();
        result
    }

    /// Run fuzzing loop
    pub fn run(&mut self, iterations: u64) -> Vec<FuzzResult> {
        self.running.store(true, Ordering::SeqCst);
        self.stats.start_time = NexusTimestamp::now();

        let mut results = Vec::new();

        for _ in 0..iterations {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            let result = self.fuzz_one();
            if result.is_interesting() {
                results.push(result);
            }
        }

        self.stats.last_update = NexusTimestamp::now();
        let duration = self.stats.last_update.duration_since(self.stats.start_time);
        if duration > 0 {
            self.stats.exec_per_sec =
                (self.stats.executions as f64) / (duration as f64 / 1_000_000_000.0);
        }

        self.running.store(false, Ordering::SeqCst);
        results
    }

    /// Stop fuzzing
    #[inline(always)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &FuzzStats {
        &self.stats
    }

    /// Get crashes
    #[inline(always)]
    pub fn crashes(&self) -> &[FuzzInput] {
        &self.crashes
    }

    /// Get corpus
    #[inline(always)]
    pub fn corpus(&self) -> &Corpus {
        &self.corpus
    }

    /// Get mutable corpus
    #[inline(always)]
    pub fn corpus_mut(&mut self) -> &mut Corpus {
        &mut self.corpus
    }
}
