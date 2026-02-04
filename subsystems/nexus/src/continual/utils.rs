//! Utility functions for continual learning.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

/// Linear congruential generator
pub fn lcg_next(state: u64) -> u64 {
    state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

/// Average a list of gradients
pub fn average_gradients(gradients: &[Vec<f64>]) -> Option<Vec<f64>> {
    if gradients.is_empty() {
        return None;
    }

    let n = gradients.len() as f64;
    let dim = gradients[0].len();
    let mut avg = vec![0.0; dim];

    for grad in gradients {
        for (i, &g) in grad.iter().enumerate() {
            if i < avg.len() {
                avg[i] += g;
            }
        }
    }

    for v in &mut avg {
        *v /= n;
    }

    Some(avg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average_gradients() {
        let grads = vec![vec![1.0, 2.0, 3.0], vec![3.0, 2.0, 1.0]];

        let avg = average_gradients(&grads).unwrap();
        assert_eq!(avg, vec![2.0, 2.0, 2.0]);
    }
}
