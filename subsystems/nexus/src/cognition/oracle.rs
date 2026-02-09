//! # Cognitive Oracle
//!
//! Oracle system for cognitive predictions and queries.
//! Provides probabilistic reasoning and future state prediction.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// ORACLE TYPES
// ============================================================================

/// A query to the oracle
#[derive(Debug, Clone)]
pub struct OracleQuery {
    /// Query ID
    pub id: u64,
    /// Query type
    pub query_type: QueryType,
    /// Subject (what we're asking about)
    pub subject: String,
    /// Context
    pub context: BTreeMap<String, OracleValue>,
    /// Requester domain
    pub requester: DomainId,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Query type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Predict future value
    Predict,
    /// Estimate probability
    Probability,
    /// Recommend action
    Recommend,
    /// Classify input
    Classify,
    /// Detect anomaly
    Anomaly,
    /// Infer missing data
    Infer,
    /// Optimize parameter
    Optimize,
}

/// Oracle value
#[derive(Debug, Clone)]
pub enum OracleValue {
    /// Null
    Null,
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i64),
    /// Float
    Float(f64),
    /// String
    String(String),
    /// Array
    Array(Vec<OracleValue>),
    /// Map
    Map(BTreeMap<String, OracleValue>),
    /// Probability distribution
    Distribution(Vec<(String, f64)>),
}

impl OracleValue {
    #[inline]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            _ => None,
        }
    }
}

/// Oracle response
#[derive(Debug, Clone)]
pub struct OracleResponse {
    /// Query ID (matches request)
    pub query_id: u64,
    /// Response type
    pub response_type: ResponseType,
    /// Result value
    pub result: OracleValue,
    /// Confidence (0-1)
    pub confidence: f64,
    /// Explanation
    pub explanation: String,
    /// Alternative results
    pub alternatives: Vec<(OracleValue, f64)>,
    /// Processing time (ns)
    pub processing_time_ns: u64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Response type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// Successful prediction
    Success,
    /// Low confidence
    LowConfidence,
    /// Insufficient data
    InsufficientData,
    /// Unknown subject
    Unknown,
    /// Error
    Error,
}

// ============================================================================
// PREDICTION MODELS
// ============================================================================

/// Time series predictor
#[derive(Debug, Clone)]
pub struct TimeSeriesPredictor {
    /// Name
    pub name: String,
    /// Historical data
    history: VecDeque<(u64, f64)>,
    /// Maximum history
    max_history: usize,
    /// Model type
    model: TimeSeriesModel,
    /// Trained coefficients
    coefficients: Vec<f64>,
}

/// Time series model type
#[derive(Debug, Clone, Copy)]
pub enum TimeSeriesModel {
    /// Moving average
    MovingAverage(usize),
    /// Exponential smoothing
    ExponentialSmoothing(f64),
    /// Linear regression
    LinearRegression,
    /// ARIMA-like (simplified)
    Arima,
}

impl TimeSeriesPredictor {
    /// Create a new predictor
    pub fn new(name: &str, model: TimeSeriesModel, max_history: usize) -> Self {
        Self {
            name: name.into(),
            history: VecDeque::new(),
            max_history,
            model,
            coefficients: Vec::new(),
        }
    }

    /// Add observation
    #[inline]
    pub fn observe(&mut self, timestamp: u64, value: f64) {
        self.history.push_back((timestamp, value));
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
        self.update_model();
    }

    /// Update model coefficients
    fn update_model(&mut self) {
        match self.model {
            TimeSeriesModel::LinearRegression => {
                self.fit_linear_regression();
            },
            _ => {}, // Other models don't need pre-fitting
        }
    }

    /// Fit linear regression
    fn fit_linear_regression(&mut self) {
        if self.history.len() < 2 {
            return;
        }

        let n = self.history.len() as f64;
        let x: Vec<f64> = (0..self.history.len()).map(|i| i as f64).collect();
        let y: Vec<f64> = self.history.iter().map(|(_, v)| *v).collect();

        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
        let sum_xx: f64 = x.iter().map(|xi| xi * xi).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        self.coefficients = vec![intercept, slope];
    }

    /// Predict future value
    #[inline]
    pub fn predict(&self, steps_ahead: usize) -> Option<(f64, f64)> {
        if self.history.is_empty() {
            return None;
        }

        let (value, confidence) = match self.model {
            TimeSeriesModel::MovingAverage(window) => {
                let window = window.min(self.history.len());
                let avg = self
                    .history
                    .iter()
                    .rev()
                    .take(window)
                    .map(|(_, v)| *v)
                    .sum::<f64>()
                    / window as f64;
                (avg, 0.7)
            },
            TimeSeriesModel::ExponentialSmoothing(alpha) => {
                let mut smoothed = self.history[0].1;
                for (_, v) in &self.history[1..] {
                    smoothed = alpha * v + (1.0 - alpha) * smoothed;
                }
                (smoothed, 0.75)
            },
            TimeSeriesModel::LinearRegression => {
                if self.coefficients.len() != 2 {
                    return None;
                }
                let x = (self.history.len() + steps_ahead) as f64;
                let value = self.coefficients[0] + self.coefficients[1] * x;

                // Confidence decreases with steps ahead
                let conf = (0.9 - 0.05 * steps_ahead as f64).max(0.5);
                (value, conf)
            },
            TimeSeriesModel::Arima => {
                // Simplified ARIMA: use last value plus trend
                if self.history.len() < 2 {
                    return Some((self.history.back()?.1, 0.5));
                }
                let trend = self.history.back()?.1 - self.history[self.history.len() - 2].1;
                let value = self.history.back()?.1 + trend * steps_ahead as f64;
                (value, 0.6)
            },
        };

        Some((value, confidence))
    }

    /// Get trend
    pub fn get_trend(&self) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }

        // Simple trend: slope of last few points
        let n = self.history.len().min(10);
        let recent: Vec<_> = self.history.iter().rev().take(n).collect();

        let first = recent.last()?.1;
        let last = recent.first()?.1;

        Some((last - first) / (n - 1) as f64)
    }
}

// ============================================================================
// ORACLE ENGINE
// ============================================================================

/// Oracle engine
pub struct OracleEngine {
    /// Predictors
    predictors: BTreeMap<String, TimeSeriesPredictor>,
    /// Classification models
    classifiers: BTreeMap<String, Classifier>,
    /// Query history
    query_history: VecDeque<(OracleQuery, OracleResponse)>,
    /// Next query ID
    next_query_id: AtomicU64,
    /// Configuration
    config: OracleConfig,
    /// Statistics
    stats: OracleStats,
}

/// Classifier
#[derive(Debug, Clone)]
pub struct Classifier {
    /// Name
    pub name: String,
    /// Classes
    pub classes: Vec<String>,
    /// Class statistics (mean, std for each feature)
    class_stats: BTreeMap<String, Vec<(f64, f64)>>,
    /// Prior probabilities
    priors: BTreeMap<String, f64>,
}

impl Classifier {
    /// Create a new classifier
    pub fn new(name: &str, classes: Vec<String>) -> Self {
        Self {
            name: name.into(),
            classes: classes.clone(),
            class_stats: classes.iter().map(|c| (c.clone(), Vec::new())).collect(),
            priors: classes
                .iter()
                .map(|c| (c.clone(), 1.0 / classes.len() as f64))
                .collect(),
        }
    }

    /// Train with example
    pub fn train(&mut self, class: &str, features: &[f64]) {
        // Simplified: just update running statistics
        if let Some(stats) = self.class_stats.get_mut(class) {
            for (i, &feat) in features.iter().enumerate() {
                if i >= stats.len() {
                    stats.push((feat, 0.0));
                } else {
                    // Update mean incrementally
                    let (mean, _) = stats[i];
                    let new_mean = (mean + feat) / 2.0;
                    stats[i] = (new_mean, 0.1); // Fixed variance for simplicity
                }
            }
        }
    }

    /// Classify features
    pub fn classify(&self, features: &[f64]) -> Vec<(String, f64)> {
        let mut scores: Vec<(String, f64)> = Vec::new();

        for class in &self.classes {
            let prior = self.priors.get(class).copied().unwrap_or(0.0);
            let stats = self.class_stats.get(class);

            let likelihood = match stats {
                Some(s) if !s.is_empty() => {
                    features
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &f)| {
                            s.get(i).map(|(mean, std)| {
                                // Gaussian likelihood
                                let std = std.max(&0.1);
                                let diff = (f - mean) / std;
                                (-0.5 * diff * diff).exp()
                            })
                        })
                        .product::<f64>()
                },
                _ => 0.1,
            };

            scores.push((class.clone(), prior * likelihood));
        }

        // Normalize
        let total: f64 = scores.iter().map(|(_, s)| s).sum();
        if total > 0.0 {
            for (_, score) in &mut scores {
                *score /= total;
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        scores
    }
}

/// Oracle configuration
#[derive(Debug, Clone)]
pub struct OracleConfig {
    /// Maximum history
    pub max_history: usize,
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Maximum predictors
    pub max_predictors: usize,
    /// Default prediction horizon
    pub default_horizon: usize,
}

impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            min_confidence: 0.5,
            max_predictors: 100,
            default_horizon: 1,
        }
    }
}

/// Oracle statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct OracleStats {
    /// Total queries
    pub total_queries: u64,
    /// Successful predictions
    pub successful: u64,
    /// Low confidence
    pub low_confidence: u64,
    /// Average confidence
    pub avg_confidence: f64,
    /// Average processing time (ns)
    pub avg_processing_ns: f64,
}

impl OracleEngine {
    /// Create a new oracle engine
    pub fn new(config: OracleConfig) -> Self {
        Self {
            predictors: BTreeMap::new(),
            classifiers: BTreeMap::new(),
            query_history: VecDeque::new(),
            next_query_id: AtomicU64::new(1),
            config,
            stats: OracleStats::default(),
        }
    }

    /// Create predictor
    #[inline(always)]
    pub fn create_predictor(&mut self, name: &str, model: TimeSeriesModel) {
        let predictor = TimeSeriesPredictor::new(name, model, self.config.max_history);
        self.predictors.insert(name.into(), predictor);
    }

    /// Create classifier
    #[inline(always)]
    pub fn create_classifier(&mut self, name: &str, classes: Vec<String>) {
        let classifier = Classifier::new(name, classes);
        self.classifiers.insert(name.into(), classifier);
    }

    /// Record observation
    #[inline]
    pub fn observe(&mut self, predictor_name: &str, timestamp: u64, value: f64) {
        if let Some(predictor) = self.predictors.get_mut(predictor_name) {
            predictor.observe(timestamp, value);
        }
    }

    /// Train classifier
    #[inline]
    pub fn train_classifier(&mut self, classifier_name: &str, class: &str, features: &[f64]) {
        if let Some(classifier) = self.classifiers.get_mut(classifier_name) {
            classifier.train(class, features);
        }
    }

    /// Process query
    pub fn query(&mut self, query: OracleQuery) -> OracleResponse {
        let start = Timestamp::now();
        self.stats.total_queries += 1;

        let response = match query.query_type {
            QueryType::Predict => self.handle_predict(&query),
            QueryType::Probability => self.handle_probability(&query),
            QueryType::Classify => self.handle_classify(&query),
            QueryType::Recommend => self.handle_recommend(&query),
            QueryType::Anomaly => self.handle_anomaly(&query),
            QueryType::Infer => self.handle_infer(&query),
            QueryType::Optimize => self.handle_optimize(&query),
        };

        let processing_time = Timestamp::now().elapsed_since(start);

        let response = OracleResponse {
            query_id: query.id,
            processing_time_ns: processing_time,
            timestamp: Timestamp::now(),
            ..response
        };

        // Update stats
        if response.response_type == ResponseType::Success {
            self.stats.successful += 1;
        }
        if response.confidence < self.config.min_confidence {
            self.stats.low_confidence += 1;
        }
        self.stats.avg_confidence = (self.stats.avg_confidence
            * (self.stats.total_queries - 1) as f64
            + response.confidence)
            / self.stats.total_queries as f64;
        self.stats.avg_processing_ns = (self.stats.avg_processing_ns
            * (self.stats.total_queries - 1) as f64
            + processing_time as f64)
            / self.stats.total_queries as f64;

        // Store in history
        if self.query_history.len() >= self.config.max_history {
            self.query_history.pop_front();
        }
        self.query_history.push_back((query, response.clone()));

        response
    }

    /// Handle prediction query
    fn handle_predict(&self, query: &OracleQuery) -> OracleResponse {
        let predictor = match self.predictors.get(&query.subject) {
            Some(p) => p,
            None => return self.unknown_response(),
        };

        let horizon = query
            .context
            .get("horizon")
            .and_then(|v| v.as_f64())
            .map(|v| v as usize)
            .unwrap_or(self.config.default_horizon);

        match predictor.predict(horizon) {
            Some((value, confidence)) => {
                let trend = predictor.get_trend();

                OracleResponse {
                    query_id: query.id,
                    response_type: if confidence >= self.config.min_confidence {
                        ResponseType::Success
                    } else {
                        ResponseType::LowConfidence
                    },
                    result: OracleValue::Float(value),
                    confidence,
                    explanation: format!(
                        "Predicted value {:.2} with {:.1}% confidence, trend: {:+.4}",
                        value,
                        confidence * 100.0,
                        trend.unwrap_or(0.0)
                    ),
                    alternatives: Vec::new(),
                    processing_time_ns: 0,
                    timestamp: Timestamp::now(),
                }
            },
            None => self.insufficient_data_response(),
        }
    }

    /// Handle probability query
    fn handle_probability(&self, query: &OracleQuery) -> OracleResponse {
        // Use classifier to get probability distribution
        if let Some(classifier) = self.classifiers.get(&query.subject) {
            let features = query
                .context
                .get("features")
                .and_then(|v| match v {
                    OracleValue::Array(arr) => {
                        Some(arr.iter().filter_map(|v| v.as_f64()).collect::<Vec<_>>())
                    },
                    _ => None,
                })
                .unwrap_or_default();

            let probs = classifier.classify(&features);
            let top = probs.first().cloned().unwrap_or(("unknown".into(), 0.0));

            return OracleResponse {
                query_id: query.id,
                response_type: ResponseType::Success,
                result: OracleValue::Distribution(probs.clone()),
                confidence: top.1,
                explanation: format!("Most likely class: {} ({:.1}%)", top.0, top.1 * 100.0),
                alternatives: probs
                    .iter()
                    .skip(1)
                    .map(|(c, p)| (OracleValue::String(c.clone()), *p))
                    .collect(),
                processing_time_ns: 0,
                timestamp: Timestamp::now(),
            };
        }

        self.unknown_response()
    }

    /// Handle classification query
    fn handle_classify(&self, query: &OracleQuery) -> OracleResponse {
        self.handle_probability(query)
    }

    /// Handle recommendation query
    fn handle_recommend(&self, _query: &OracleQuery) -> OracleResponse {
        // Placeholder: return generic recommendation
        OracleResponse {
            query_id: 0,
            response_type: ResponseType::Success,
            result: OracleValue::String("maintain_current_state".into()),
            confidence: 0.6,
            explanation: "Recommendation based on current trends".into(),
            alternatives: vec![
                (OracleValue::String("increase_capacity".into()), 0.25),
                (OracleValue::String("reduce_load".into()), 0.15),
            ],
            processing_time_ns: 0,
            timestamp: Timestamp::now(),
        }
    }

    /// Handle anomaly query
    fn handle_anomaly(&self, query: &OracleQuery) -> OracleResponse {
        let predictor = match self.predictors.get(&query.subject) {
            Some(p) => p,
            None => return self.unknown_response(),
        };

        let current = query
            .context
            .get("current")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Check if current value is anomalous
        if let Some((predicted, confidence)) = predictor.predict(0) {
            let deviation = (current - predicted).abs() / predicted.abs().max(0.001);
            let is_anomaly = deviation > 0.5;

            return OracleResponse {
                query_id: query.id,
                response_type: ResponseType::Success,
                result: OracleValue::Bool(is_anomaly),
                confidence,
                explanation: format!(
                    "Deviation: {:.1}%, anomaly: {}",
                    deviation * 100.0,
                    is_anomaly
                ),
                alternatives: Vec::new(),
                processing_time_ns: 0,
                timestamp: Timestamp::now(),
            };
        }

        self.insufficient_data_response()
    }

    /// Handle inference query
    fn handle_infer(&self, _query: &OracleQuery) -> OracleResponse {
        self.insufficient_data_response()
    }

    /// Handle optimization query
    fn handle_optimize(&self, _query: &OracleQuery) -> OracleResponse {
        self.insufficient_data_response()
    }

    fn unknown_response(&self) -> OracleResponse {
        OracleResponse {
            query_id: 0,
            response_type: ResponseType::Unknown,
            result: OracleValue::Null,
            confidence: 0.0,
            explanation: "Unknown subject".into(),
            alternatives: Vec::new(),
            processing_time_ns: 0,
            timestamp: Timestamp::now(),
        }
    }

    fn insufficient_data_response(&self) -> OracleResponse {
        OracleResponse {
            query_id: 0,
            response_type: ResponseType::InsufficientData,
            result: OracleValue::Null,
            confidence: 0.0,
            explanation: "Insufficient data for prediction".into(),
            alternatives: Vec::new(),
            processing_time_ns: 0,
            timestamp: Timestamp::now(),
        }
    }

    /// Create a query
    #[inline]
    pub fn create_query(
        &self,
        query_type: QueryType,
        subject: &str,
        context: BTreeMap<String, OracleValue>,
        requester: DomainId,
    ) -> OracleQuery {
        OracleQuery {
            id: self.next_query_id.fetch_add(1, Ordering::Relaxed),
            query_type,
            subject: subject.into(),
            context,
            requester,
            timestamp: Timestamp::now(),
        }
    }

    /// Get predictor
    #[inline(always)]
    pub fn get_predictor(&self, name: &str) -> Option<&TimeSeriesPredictor> {
        self.predictors.get(name)
    }

    /// Get classifier
    #[inline(always)]
    pub fn get_classifier(&self, name: &str) -> Option<&Classifier> {
        self.classifiers.get(name)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &OracleStats {
        &self.stats
    }
}

impl Default for OracleEngine {
    fn default() -> Self {
        Self::new(OracleConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series_prediction() {
        let mut predictor =
            TimeSeriesPredictor::new("test", TimeSeriesModel::LinearRegression, 100);

        // Add linear data
        for i in 0..10 {
            predictor.observe(i as u64, i as f64 * 2.0 + 10.0);
        }

        let (value, confidence) = predictor.predict(1).unwrap();

        // Should predict ~30 (next value in sequence)
        assert!(value > 25.0 && value < 35.0);
        assert!(confidence > 0.5);
    }

    #[test]
    fn test_classifier() {
        let mut classifier = Classifier::new("test", vec!["a".into(), "b".into()]);

        // Train with some examples
        classifier.train("a", &[1.0, 2.0]);
        classifier.train("a", &[1.5, 2.5]);
        classifier.train("b", &[5.0, 6.0]);
        classifier.train("b", &[5.5, 6.5]);

        // Classify
        let results = classifier.classify(&[1.2, 2.2]);
        assert_eq!(results[0].0, "a");
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn test_oracle_predict() {
        let mut oracle = OracleEngine::default();
        let domain = DomainId::new(1);

        oracle.create_predictor("metric", TimeSeriesModel::MovingAverage(5));

        // Add data
        for i in 0..10 {
            oracle.observe("metric", i as u64, 100.0 + i as f64);
        }

        let query = oracle.create_query(QueryType::Predict, "metric", BTreeMap::new(), domain);

        let response = oracle.query(query);
        assert_eq!(response.response_type, ResponseType::Success);
    }
}
