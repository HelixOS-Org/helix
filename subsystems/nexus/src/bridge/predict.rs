//! # Syscall Prediction Engine
//!
//! Predicts the next syscall(s) an application will make, enabling
//! prefetching, pre-computation, and zero-latency responses.
//!
//! Uses n-gram sequence analysis and Markov chain models to learn
//! per-process syscall patterns.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// CONFIDENCE TYPE
// ============================================================================

/// Confidence level for a prediction
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SyscallConfidence(f64);

impl SyscallConfidence {
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn value(&self) -> f64 {
        self.0
    }

    pub fn is_high(&self) -> bool {
        self.0 >= 0.8
    }

    pub fn is_medium(&self) -> bool {
        self.0 >= 0.5 && self.0 < 0.8
    }

    pub fn is_low(&self) -> bool {
        self.0 < 0.5
    }
}

// ============================================================================
// PREDICTION TYPES
// ============================================================================

/// A predicted future syscall
#[derive(Debug, Clone)]
pub struct PredictedSyscall {
    /// The predicted syscall type
    pub syscall_type: SyscallType,
    /// Confidence in this prediction
    pub confidence: SyscallConfidence,
    /// Estimated data size (for I/O predictions)
    pub estimated_data_size: usize,
    /// Steps ahead this prediction is for (1 = next, 2 = two ahead, etc.)
    pub steps_ahead: usize,
    /// Basis for the prediction
    pub basis: PredictionBasis,
}

/// What the prediction is based on
#[derive(Debug, Clone)]
pub enum PredictionBasis {
    /// N-gram pattern match
    NGram { n: usize, occurrences: u64 },
    /// Markov chain transition probability
    MarkovChain { order: usize },
    /// Application class default pattern
    AppClassDefault,
    /// Temporal pattern (time-of-day, periodic)
    TemporalPattern,
}

/// A sequence of syscalls (for pattern matching)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyscallSequence {
    types: Vec<u64>,
}

impl SyscallSequence {
    pub fn new() -> Self {
        Self { types: Vec::new() }
    }

    pub fn push(&mut self, st: SyscallType) {
        self.types.push(st.from_number_reverse_pub());
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

/// A detected syscall pattern
#[derive(Debug, Clone)]
pub struct SyscallPattern {
    /// The sequence that triggers this pattern
    pub trigger: SyscallSequence,
    /// What typically follows
    pub following: SyscallType,
    /// How many times this pattern has been observed
    pub occurrences: u64,
    /// Total times the trigger appeared (for computing probability)
    pub trigger_count: u64,
}

impl SyscallPattern {
    /// Probability of the following syscall given the trigger
    pub fn probability(&self) -> f64 {
        if self.trigger_count == 0 {
            return 0.0;
        }
        self.occurrences as f64 / self.trigger_count as f64
    }
}

// ============================================================================
// SYSCALL PREDICTOR
// ============================================================================

/// The prediction engine — learns per-process syscall patterns and predicts
/// future syscalls using n-gram analysis.
pub struct SyscallPredictor {
    /// Recent syscall history (ring buffer per process)
    history: Vec<SyscallType>,
    /// History capacity
    capacity: usize,
    /// N-gram order (e.g., 3 = trigrams)
    ngram_order: usize,
    /// N-gram frequency table: sequence_hash -> (following_type -> count)
    ngram_table: BTreeMap<u64, BTreeMap<u64, u64>>,
    /// Total transitions from each sequence
    ngram_totals: BTreeMap<u64, u64>,
    /// Minimum occurrences before using a pattern
    min_occurrences: u64,
    /// Total observations
    total_observations: u64,
}

impl SyscallPredictor {
    /// Create a new predictor with given history capacity and n-gram order
    pub fn new(capacity: usize, ngram_order: usize) -> Self {
        Self {
            history: Vec::with_capacity(capacity),
            capacity,
            ngram_order: ngram_order.max(1),
            ngram_table: BTreeMap::new(),
            ngram_totals: BTreeMap::new(),
            min_occurrences: 2,
            total_observations: 0,
        }
    }

    /// Observe a syscall — updates the model
    pub fn observe(&mut self, syscall_type: SyscallType) {
        // If we have enough history, record the n-gram transition
        if self.history.len() >= self.ngram_order {
            let key = self.compute_ngram_key();
            let type_key = syscall_type.from_number_reverse_pub();

            *self.ngram_table.entry(key).or_default().entry(type_key).or_insert(0) += 1;
            *self.ngram_totals.entry(key).or_insert(0) += 1;
        }

        // Add to history
        if self.history.len() >= self.capacity {
            self.history.remove(0);
        }
        self.history.push(syscall_type);
        self.total_observations += 1;
    }

    /// Predict the next syscall
    pub fn predict_next(&self) -> Option<PredictedSyscall> {
        if self.history.len() < self.ngram_order {
            return None;
        }

        let key = self.compute_ngram_key();
        let transitions = self.ngram_table.get(&key)?;
        let total = *self.ngram_totals.get(&key)?;

        if total < self.min_occurrences {
            return None;
        }

        // Find the most likely next syscall
        let (best_type_key, best_count) = transitions
            .iter()
            .max_by_key(|(_, count)| **count)?;

        let probability = *best_count as f64 / total as f64;

        Some(PredictedSyscall {
            syscall_type: SyscallType::from_number(*best_type_key),
            confidence: SyscallConfidence::new(probability),
            estimated_data_size: 0,
            steps_ahead: 1,
            basis: PredictionBasis::NGram {
                n: self.ngram_order,
                occurrences: *best_count,
            },
        })
    }

    /// Predict the next N syscalls
    pub fn predict_sequence(&self, max_steps: usize) -> Vec<PredictedSyscall> {
        let mut predictions = Vec::new();
        let mut simulated_history = self.history.clone();

        for step in 0..max_steps {
            if simulated_history.len() < self.ngram_order {
                break;
            }

            let key = Self::compute_key_from_slice(
                &simulated_history[simulated_history.len() - self.ngram_order..],
            );

            if let Some(transitions) = self.ngram_table.get(&key) {
                if let Some(total) = self.ngram_totals.get(&key) {
                    if let Some((best_type_key, best_count)) =
                        transitions.iter().max_by_key(|(_, c)| **c)
                    {
                        let probability = *best_count as f64 / *total as f64;

                        // Confidence degrades with steps ahead
                        let degraded_conf = probability * (0.9_f64).powi(step as i32);

                        if degraded_conf < 0.3 {
                            break;
                        }

                        let predicted_type = SyscallType::from_number(*best_type_key);

                        predictions.push(PredictedSyscall {
                            syscall_type: predicted_type,
                            confidence: SyscallConfidence::new(degraded_conf),
                            estimated_data_size: 0,
                            steps_ahead: step + 1,
                            basis: PredictionBasis::NGram {
                                n: self.ngram_order,
                                occurrences: *best_count,
                            },
                        });

                        // Advance simulated history
                        if simulated_history.len() >= self.capacity {
                            simulated_history.remove(0);
                        }
                        simulated_history.push(predicted_type);
                    }
                }
            } else {
                break;
            }
        }

        predictions
    }

    /// Get the top K most likely next syscalls
    pub fn predict_top_k(&self, k: usize) -> Vec<PredictedSyscall> {
        if self.history.len() < self.ngram_order {
            return Vec::new();
        }

        let key = self.compute_ngram_key();
        let transitions = match self.ngram_table.get(&key) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let total = match self.ngram_totals.get(&key) {
            Some(t) => *t,
            None => return Vec::new(),
        };

        let mut entries: Vec<_> = transitions.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        entries
            .into_iter()
            .take(k)
            .map(|(type_key, count)| PredictedSyscall {
                syscall_type: SyscallType::from_number(*type_key),
                confidence: SyscallConfidence::new(*count as f64 / total as f64),
                estimated_data_size: 0,
                steps_ahead: 1,
                basis: PredictionBasis::NGram {
                    n: self.ngram_order,
                    occurrences: *count,
                },
            })
            .collect()
    }

    /// Total observations
    pub fn total_observations(&self) -> u64 {
        self.total_observations
    }

    /// Number of unique patterns learned
    pub fn patterns_learned(&self) -> usize {
        self.ngram_table.len()
    }

    /// Compute n-gram key from current history tail
    fn compute_ngram_key(&self) -> u64 {
        let start = self.history.len() - self.ngram_order;
        Self::compute_key_from_slice(&self.history[start..])
    }

    /// Compute n-gram key from a slice
    fn compute_key_from_slice(slice: &[SyscallType]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
        for st in slice {
            let v = st.from_number_reverse_pub();
            hash ^= v;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }
        hash
    }
}

impl SyscallType {
    /// Public version of number reverse (for hashing)
    pub fn from_number_reverse_pub(&self) -> u64 {
        match self {
            Self::Read => 0,
            Self::Write => 1,
            Self::Open => 2,
            Self::Close => 3,
            Self::Seek => 8,
            Self::Stat => 4,
            Self::Mmap => 9,
            Self::Munmap => 11,
            Self::Mprotect => 10,
            Self::Brk => 12,
            Self::Fork => 56,
            Self::Exec => 59,
            Self::Exit => 60,
            Self::Wait => 61,
            Self::Kill => 62,
            Self::Socket => 41,
            Self::Bind => 49,
            Self::Listen => 50,
            Self::Accept => 43,
            Self::Connect => 42,
            Self::Send => 44,
            Self::Recv => 45,
            Self::Futex => 202,
            Self::ClockGettime => 228,
            Self::Nanosleep => 35,
            Self::Ioctl => 16,
            Self::Fsync => 74,
            Self::Readdir => 78,
            Self::SemWait => 230,
            Self::SemPost => 231,
            Self::Unknown(n) => *n,
        }
    }
}
