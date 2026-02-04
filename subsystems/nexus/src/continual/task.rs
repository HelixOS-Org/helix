//! Task representation for continual learning.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// A task in the continual learning setting
#[derive(Debug, Clone)]
pub struct Task {
    /// Task identifier
    pub id: u64,
    /// Task name
    pub name: String,
    /// Number of training samples
    pub num_samples: usize,
    /// Number of epochs trained
    pub epochs_trained: u32,
    /// Final accuracy on this task
    pub accuracy: f64,
    /// Is this task currently active?
    pub is_active: bool,
    /// Task-specific metadata
    pub metadata: BTreeMap<String, f64>,
}

impl Task {
    /// Create a new task
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            num_samples: 0,
            epochs_trained: 0,
            accuracy: 0.0,
            is_active: true,
            metadata: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(0, alloc::string::String::from("test_task"));
        assert_eq!(task.id, 0);
        assert!(task.is_active);
    }
}
