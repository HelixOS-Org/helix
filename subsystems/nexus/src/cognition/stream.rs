//! # Cognitive Stream Processing
//!
//! Real-time stream processing for cognitive data.
//! Supports windowing, aggregation, and pattern detection.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// STREAM TYPES
// ============================================================================

/// A data stream element
#[derive(Debug, Clone)]
pub struct StreamElement<T> {
    /// Element ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Element key (for partitioning)
    pub key: Option<String>,
    /// Data
    pub data: T,
}

/// Stream window type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    /// Fixed-size tumbling window
    Tumbling,
    /// Sliding window with overlap
    Sliding,
    /// Session window (based on inactivity)
    Session,
    /// Count-based window
    Count,
}

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window type
    pub window_type: WindowType,
    /// Window size (time in ns or count)
    pub size: u64,
    /// Slide interval (for sliding windows)
    pub slide: Option<u64>,
    /// Session gap (for session windows)
    pub session_gap: Option<u64>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            window_type: WindowType::Tumbling,
            size: 1_000_000_000, // 1 second
            slide: None,
            session_gap: None,
        }
    }
}

/// A window of elements
#[derive(Debug, Clone)]
pub struct Window<T> {
    /// Window ID
    pub id: u64,
    /// Start time
    pub start: Timestamp,
    /// End time
    pub end: Option<Timestamp>,
    /// Elements in window
    pub elements: Vec<StreamElement<T>>,
    /// Is complete
    pub complete: bool,
}

/// Aggregation function type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationType {
    /// Count elements
    Count,
    /// Sum values
    Sum,
    /// Average
    Average,
    /// Minimum
    Min,
    /// Maximum
    Max,
    /// First value
    First,
    /// Last value
    Last,
    /// Distinct count
    DistinctCount,
}

/// Aggregation result
#[derive(Debug, Clone)]
pub struct AggregationResult {
    /// Window ID
    pub window_id: u64,
    /// Start time
    pub start: Timestamp,
    /// End time
    pub end: Timestamp,
    /// Aggregation type
    pub agg_type: AggregationType,
    /// Result value
    pub value: f64,
    /// Element count
    pub count: u64,
}

// ============================================================================
// STREAM PROCESSOR
// ============================================================================

/// Processes a data stream
pub struct StreamProcessor<T: Clone> {
    /// Stream ID
    id: u64,
    /// Stream name
    name: String,
    /// Window configuration
    window_config: WindowConfig,
    /// Current windows
    windows: BTreeMap<u64, Window<T>>,
    /// Next window ID
    next_window_id: AtomicU64,
    /// Next element ID
    next_element_id: AtomicU64,
    /// Pending elements (not yet windowed)
    pending: Vec<StreamElement<T>>,
    /// Completed aggregations
    aggregations: Vec<AggregationResult>,
    /// Statistics
    stats: StreamStats,
}

/// Stream statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct StreamStats {
    /// Total elements received
    pub total_elements: u64,
    /// Total windows created
    pub total_windows: u64,
    /// Total windows completed
    pub completed_windows: u64,
    /// Total aggregations
    pub total_aggregations: u64,
    /// Elements per second
    pub elements_per_second: f32,
    /// Average window size
    pub avg_window_size: f32,
}

impl<T: Clone> StreamProcessor<T> {
    /// Create a new stream processor
    pub fn new(id: u64, name: &str, window_config: WindowConfig) -> Self {
        Self {
            id,
            name: name.into(),
            window_config,
            windows: BTreeMap::new(),
            next_window_id: AtomicU64::new(1),
            next_element_id: AtomicU64::new(1),
            pending: Vec::new(),
            aggregations: Vec::new(),
            stats: StreamStats::default(),
        }
    }

    /// Push an element into the stream
    pub fn push(&mut self, source: DomainId, data: T, key: Option<String>) -> u64 {
        let id = self.next_element_id.fetch_add(1, Ordering::Relaxed);

        let element = StreamElement {
            id,
            source,
            timestamp: Timestamp::now(),
            key,
            data,
        };

        self.stats.total_elements += 1;

        // Assign to window
        match self.window_config.window_type {
            WindowType::Tumbling | WindowType::Sliding => {
                self.assign_time_window(element);
            },
            WindowType::Count => {
                self.assign_count_window(element);
            },
            WindowType::Session => {
                self.assign_session_window(element);
            },
        }

        id
    }

    /// Assign element to time-based window
    fn assign_time_window(&mut self, element: StreamElement<T>) {
        let window_size = self.window_config.size;
        let element_time = element.timestamp.raw();

        // Calculate window start
        let window_start = (element_time / window_size) * window_size;

        // Find or create window
        let window_id = self
            .windows
            .iter()
            .find(|(_, w)| w.start.raw() == window_start)
            .map(|(id, _)| *id);

        let window_id = match window_id {
            Some(id) => id,
            None => {
                let id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
                let window = Window {
                    id,
                    start: Timestamp::from_raw(window_start),
                    end: None,
                    elements: Vec::new(),
                    complete: false,
                };
                self.windows.insert(id, window);
                self.stats.total_windows += 1;
                id
            },
        };

        if let Some(window) = self.windows.get_mut(&window_id) {
            window.elements.push(element);
        }

        // For sliding windows, element might belong to multiple windows
        if let (WindowType::Sliding, Some(slide)) =
            (self.window_config.window_type, self.window_config.slide)
        {
            // Check previous windows that might still include this element
            let mut prev_start = window_start.saturating_sub(slide);
            while prev_start >= window_start.saturating_sub(window_size) {
                let prev_window_id = self
                    .windows
                    .iter()
                    .find(|(_, w)| w.start.raw() == prev_start)
                    .map(|(id, _)| *id);

                if let Some(id) = prev_window_id {
                    if let Some(window) = self.windows.get_mut(&id) {
                        // Clone element for this window
                        let element_clone = window.elements.last().cloned();
                        if let Some(e) = element_clone {
                            window.elements.push(e);
                        }
                    }
                }

                if prev_start == 0 || slide == 0 {
                    break;
                }
                prev_start = prev_start.saturating_sub(slide);
            }
        }
    }

    /// Assign element to count-based window
    fn assign_count_window(&mut self, element: StreamElement<T>) {
        // Find open window
        let open_window = self
            .windows
            .iter()
            .find(|(_, w)| !w.complete)
            .map(|(id, _)| *id);

        let window_id = match open_window {
            Some(id) => id,
            None => {
                let id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
                let window = Window {
                    id,
                    start: Timestamp::now(),
                    end: None,
                    elements: Vec::new(),
                    complete: false,
                };
                self.windows.insert(id, window);
                self.stats.total_windows += 1;
                id
            },
        };

        if let Some(window) = self.windows.get_mut(&window_id) {
            window.elements.push(element);

            // Check if window is full
            if window.elements.len() as u64 >= self.window_config.size {
                window.complete = true;
                window.end = Some(Timestamp::now());
                self.stats.completed_windows += 1;
            }
        }
    }

    /// Assign element to session window
    fn assign_session_window(&mut self, element: StreamElement<T>) {
        let session_gap = self.window_config.session_gap.unwrap_or(1_000_000_000);

        // Find active session for this key
        let key = element.key.clone();
        let element_time = element.timestamp.raw();

        let active_session = self
            .windows
            .iter()
            .find(|(_, w)| {
                !w.complete
                    && w.elements
                        .last()
                        .map(|last| element_time - last.timestamp.raw() <= session_gap)
                        .unwrap_or(false)
                    && w.elements
                        .first()
                        .map(|first| first.key == key)
                        .unwrap_or(true)
            })
            .map(|(id, _)| *id);

        match active_session {
            Some(id) => {
                if let Some(window) = self.windows.get_mut(&id) {
                    window.elements.push(element);
                }
            },
            None => {
                // Close any expired sessions for this key
                let expired: Vec<_> = self
                    .windows
                    .iter()
                    .filter(|(_, w)| {
                        !w.complete
                            && w.elements.first().map(|f| f.key == key).unwrap_or(false)
                            && w.elements
                                .last()
                                .map(|last| element_time - last.timestamp.raw() > session_gap)
                                .unwrap_or(true)
                    })
                    .map(|(id, _)| *id)
                    .collect();

                for id in expired {
                    if let Some(window) = self.windows.get_mut(&id) {
                        window.complete = true;
                        window.end = Some(Timestamp::now());
                        self.stats.completed_windows += 1;
                    }
                }

                // Create new session
                let id = self.next_window_id.fetch_add(1, Ordering::Relaxed);
                let window = Window {
                    id,
                    start: element.timestamp,
                    end: None,
                    elements: vec![element],
                    complete: false,
                };
                self.windows.insert(id, window);
                self.stats.total_windows += 1;
            },
        }
    }

    /// Process expired windows
    pub fn process_expired(&mut self) -> Vec<u64> {
        let now = Timestamp::now();
        let window_size = self.window_config.size;
        let mut completed = Vec::new();

        for (id, window) in &mut self.windows {
            if window.complete {
                continue;
            }

            let window_end = window.start.raw() + window_size;
            if now.raw() >= window_end {
                window.complete = true;
                window.end = Some(Timestamp::from_raw(window_end));
                self.stats.completed_windows += 1;
                completed.push(*id);
            }
        }

        completed
    }

    /// Aggregate a window
    pub fn aggregate<F>(
        &mut self,
        window_id: u64,
        agg_type: AggregationType,
        value_fn: F,
    ) -> Option<AggregationResult>
    where
        F: Fn(&T) -> f64,
    {
        let window = self.windows.get(&window_id)?;

        if window.elements.is_empty() {
            return None;
        }

        let values: Vec<f64> = window.elements.iter().map(|e| value_fn(&e.data)).collect();

        let result_value = match agg_type {
            AggregationType::Count => values.len() as f64,
            AggregationType::Sum => values.iter().sum(),
            AggregationType::Average => values.iter().sum::<f64>() / values.len() as f64,
            AggregationType::Min => values.iter().cloned().fold(f64::MAX, f64::min),
            AggregationType::Max => values.iter().cloned().fold(f64::MIN, f64::max),
            AggregationType::First => *values.first()?,
            AggregationType::Last => *values.last()?,
            AggregationType::DistinctCount => {
                let mut unique = values.clone();
                unique.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
                unique.dedup();
                unique.len() as f64
            },
        };

        let result = AggregationResult {
            window_id,
            start: window.start,
            end: window.end.unwrap_or_else(Timestamp::now),
            agg_type,
            value: result_value,
            count: window.elements.len() as u64,
        };

        self.aggregations.push(result.clone());
        self.stats.total_aggregations += 1;
        self.stats.avg_window_size = (self.stats.avg_window_size
            * (self.stats.total_aggregations - 1) as f32
            + window.elements.len() as f32)
            / self.stats.total_aggregations as f32;

        Some(result)
    }

    /// Get completed windows
    #[inline(always)]
    pub fn completed_windows(&self) -> Vec<&Window<T>> {
        self.windows.values().filter(|w| w.complete).collect()
    }

    /// Get active windows
    #[inline(always)]
    pub fn active_windows(&self) -> Vec<&Window<T>> {
        self.windows.values().filter(|w| !w.complete).collect()
    }

    /// Get window by ID
    #[inline(always)]
    pub fn get_window(&self, id: u64) -> Option<&Window<T>> {
        self.windows.get(&id)
    }

    /// Clear completed windows
    #[inline(always)]
    pub fn clear_completed(&mut self) {
        self.windows.retain(|_, w| !w.complete);
    }

    /// Get stream ID
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get stream name
    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &StreamStats {
        &self.stats
    }

    /// Get aggregations
    #[inline(always)]
    pub fn aggregations(&self) -> &[AggregationResult] {
        &self.aggregations
    }
}

// ============================================================================
// STREAM MANAGER
// ============================================================================

/// Manages multiple streams
pub struct StreamManager {
    /// Streams by ID
    streams: BTreeMap<u64, String>, // Store name, type-erased
    /// Next stream ID
    next_id: AtomicU64,
}

impl StreamManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            streams: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a stream ID
    #[inline]
    pub fn create_stream(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.streams.insert(id, name.into());
        id
    }

    /// Get stream name
    #[inline(always)]
    pub fn get_name(&self, id: u64) -> Option<&String> {
        self.streams.get(&id)
    }

    /// Remove stream
    #[inline(always)]
    pub fn remove_stream(&mut self, id: u64) {
        self.streams.remove(&id);
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_window() {
        let config = WindowConfig {
            window_type: WindowType::Count,
            size: 3,
            slide: None,
            session_gap: None,
        };

        let mut processor: StreamProcessor<i32> = StreamProcessor::new(1, "test", config);

        let domain = DomainId::new(1);
        processor.push(domain, 1, None);
        processor.push(domain, 2, None);
        processor.push(domain, 3, None);

        let completed = processor.completed_windows();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].elements.len(), 3);
    }

    #[test]
    fn test_aggregation() {
        let config = WindowConfig {
            window_type: WindowType::Count,
            size: 5,
            slide: None,
            session_gap: None,
        };

        let mut processor: StreamProcessor<i32> = StreamProcessor::new(1, "test", config);

        let domain = DomainId::new(1);
        for i in 1..=5 {
            processor.push(domain, i, None);
        }

        let completed = processor.completed_windows();
        assert_eq!(completed.len(), 1);

        let window_id = completed[0].id;

        let sum = processor.aggregate(window_id, AggregationType::Sum, |&v| v as f64);
        assert!(sum.is_some());
        assert_eq!(sum.unwrap().value, 15.0);

        let avg = processor.aggregate(window_id, AggregationType::Average, |&v| v as f64);
        assert!(avg.is_some());
        assert_eq!(avg.unwrap().value, 3.0);
    }

    #[test]
    fn test_stream_manager() {
        let mut manager = StreamManager::new();

        let id = manager.create_stream("signals");
        assert!(id > 0);
        assert_eq!(manager.get_name(id).unwrap(), "signals");
    }
}
