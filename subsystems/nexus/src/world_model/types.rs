//! Constants and type definitions for the world model.

/// Maximum latent dimension
pub const MAX_LATENT_DIM: usize = 256;

/// Maximum action dimension
pub const MAX_ACTION_DIM: usize = 64;

/// Maximum observation dimension
pub const MAX_OBS_DIM: usize = 512;

/// Default imagination horizon
pub const DEFAULT_HORIZON: usize = 50;

/// Ensemble size for uncertainty
pub const ENSEMBLE_SIZE: usize = 5;
