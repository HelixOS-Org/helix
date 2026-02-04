//! Prediction-specific error types

use alloc::string::String;
use core::fmt;

/// Errors specific to prediction
#[derive(Debug, Clone)]
pub enum PredictionError {
    /// Insufficient data for prediction
    InsufficientData { required: usize, available: usize },
    /// Model not trained
    ModelNotTrained,
    /// Feature extraction failed
    FeatureExtractionFailed(String),
    /// Prediction confidence too low
    LowConfidence { confidence: f32, minimum: f32 },
    /// Invalid feature
    InvalidFeature { name: String, reason: String },
}

impl fmt::Display for PredictionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient data: need {}, have {}",
                    required, available
                )
            },
            Self::ModelNotTrained => write!(f, "Model not trained"),
            Self::FeatureExtractionFailed(msg) => {
                write!(f, "Feature extraction failed: {}", msg)
            },
            Self::LowConfidence {
                confidence,
                minimum,
            } => {
                write!(f, "Confidence {} below minimum {}", confidence, minimum)
            },
            Self::InvalidFeature { name, reason } => {
                write!(f, "Invalid feature '{}': {}", name, reason)
            },
        }
    }
}
