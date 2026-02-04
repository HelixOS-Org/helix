//! Basic type aliases and enums for Evolutionary Game Theory.

/// Player (agent) identifier
pub type PlayerId = u32;

/// Strategy identifier
pub type StrategyId = u32;

/// Utility value
pub type Utility = f64;

/// Types of games
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameType {
    /// Zero-sum game (one player's gain is another's loss)
    ZeroSum,
    /// Coordination game (players benefit from matching)
    Coordination,
    /// Prisoner's dilemma
    PrisonersDilemma,
    /// Resource allocation game
    ResourceAllocation,
    /// Congestion game
    Congestion,
    /// Potential game
    Potential,
    /// General sum game
    GeneralSum,
}
