//! # Cooperative Auction Mechanism
//!
//! Auction-based resource allocation between processes:
//! - Sealed-bid auctions
//! - Vickrey (second-price) auctions
//! - Combinatorial auctions
//! - Budget management
//! - Revenue recycling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// AUCTION TYPES
// ============================================================================

/// Auction type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionType {
    /// First-price sealed bid
    FirstPrice,
    /// Second-price (Vickrey)
    SecondPrice,
    /// Ascending (English)
    Ascending,
    /// Descending (Dutch)
    Descending,
}

/// Auction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionState {
    /// Open for bids
    Open,
    /// Bidding closed, computing winner
    Closed,
    /// Winner determined
    Settled,
    /// Cancelled
    Cancelled,
}

/// Auctioned resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuctionResource {
    /// CPU time slice
    CpuSlice,
    /// Memory pages
    MemoryPages,
    /// I/O bandwidth
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// Cache partition
    CachePartition,
    /// Priority boost
    PriorityBoost,
}

// ============================================================================
// BID
// ============================================================================

/// A bid in an auction
#[derive(Debug, Clone)]
pub struct Bid {
    /// Bidder process
    pub bidder: u64,
    /// Bid amount (in credits)
    pub amount: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Maximum willingness to pay
    pub max_price: u64,
}

impl Bid {
    pub fn new(bidder: u64, amount: u64, max_price: u64, now: u64) -> Self {
        Self {
            bidder,
            amount,
            timestamp: now,
            max_price,
        }
    }
}

// ============================================================================
// AUCTION
// ============================================================================

/// An auction instance
#[derive(Debug)]
pub struct Auction {
    /// Auction id
    pub id: u64,
    /// Auction type
    pub auction_type: AuctionType,
    /// Resource being auctioned
    pub resource: AuctionResource,
    /// Quantity
    pub quantity: u64,
    /// State
    pub state: AuctionState,
    /// Reserve price (minimum)
    pub reserve_price: u64,
    /// Bids
    bids: Vec<Bid>,
    /// Created at
    pub created_at: u64,
    /// Deadline
    pub deadline: u64,
    /// Winner
    pub winner: Option<u64>,
    /// Winning price
    pub winning_price: Option<u64>,
}

impl Auction {
    pub fn new(
        id: u64,
        auction_type: AuctionType,
        resource: AuctionResource,
        quantity: u64,
        reserve: u64,
        deadline: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            auction_type,
            resource,
            quantity,
            state: AuctionState::Open,
            reserve_price: reserve,
            bids: Vec::new(),
            created_at: now,
            deadline,
            winner: None,
            winning_price: None,
        }
    }

    /// Place bid
    pub fn place_bid(&mut self, bid: Bid) -> bool {
        if self.state != AuctionState::Open {
            return false;
        }
        if bid.amount < self.reserve_price {
            return false;
        }
        // For ascending auctions, must beat current high bid
        if self.auction_type == AuctionType::Ascending {
            if let Some(high) = self.highest_bid() {
                if bid.amount <= high.amount {
                    return false;
                }
            }
        }
        self.bids.push(bid);
        true
    }

    /// Highest bid
    pub fn highest_bid(&self) -> Option<&Bid> {
        self.bids.iter().max_by_key(|b| b.amount)
    }

    /// Second highest bid amount
    fn second_highest_amount(&self) -> u64 {
        if self.bids.len() < 2 {
            return self.reserve_price;
        }
        let mut amounts: Vec<u64> = self.bids.iter().map(|b| b.amount).collect();
        amounts.sort_unstable();
        amounts[amounts.len() - 2]
    }

    /// Close and determine winner
    pub fn settle(&mut self) -> Option<(u64, u64)> {
        if self.bids.is_empty() {
            self.state = AuctionState::Cancelled;
            return None;
        }

        self.state = AuctionState::Closed;

        // Find highest bidder
        let mut best_idx = 0;
        let mut best_amount = 0u64;
        for (i, bid) in self.bids.iter().enumerate() {
            if bid.amount > best_amount {
                best_amount = bid.amount;
                best_idx = i;
            }
        }

        let winner = self.bids[best_idx].bidder;
        let price = match self.auction_type {
            AuctionType::FirstPrice | AuctionType::Ascending => best_amount,
            AuctionType::SecondPrice => self.second_highest_amount(),
            AuctionType::Descending => best_amount,
        };

        self.winner = Some(winner);
        self.winning_price = Some(price);
        self.state = AuctionState::Settled;
        Some((winner, price))
    }

    /// Check deadline
    pub fn check_deadline(&mut self, now: u64) -> bool {
        if now >= self.deadline && self.state == AuctionState::Open {
            return true;
        }
        false
    }

    /// Bid count
    pub fn bid_count(&self) -> usize {
        self.bids.len()
    }

    /// Unique bidders
    pub fn unique_bidders(&self) -> usize {
        let mut seen = Vec::new();
        for bid in &self.bids {
            if !seen.contains(&bid.bidder) {
                seen.push(bid.bidder);
            }
        }
        seen.len()
    }
}

// ============================================================================
// PROCESS BUDGET
// ============================================================================

/// Process auction budget
#[derive(Debug, Clone)]
pub struct AuctionBudget {
    /// Process id
    pub pid: u64,
    /// Available credits
    pub credits: u64,
    /// Total spent
    pub total_spent: u64,
    /// Wins
    pub wins: u64,
    /// Losses
    pub losses: u64,
    /// Credit refresh rate (per second)
    pub refresh_rate: u64,
    /// Last refresh
    pub last_refresh: u64,
}

impl AuctionBudget {
    pub fn new(pid: u64, initial_credits: u64) -> Self {
        Self {
            pid,
            credits: initial_credits,
            total_spent: 0,
            wins: 0,
            losses: 0,
            refresh_rate: 100,
            last_refresh: 0,
        }
    }

    /// Spend credits
    pub fn spend(&mut self, amount: u64) -> bool {
        if amount > self.credits {
            return false;
        }
        self.credits -= amount;
        self.total_spent += amount;
        true
    }

    /// Refresh credits based on time
    pub fn refresh(&mut self, now: u64) {
        if self.last_refresh == 0 {
            self.last_refresh = now;
            return;
        }
        let elapsed_ns = now.saturating_sub(self.last_refresh);
        let secs = elapsed_ns / 1_000_000_000;
        if secs > 0 {
            self.credits += self.refresh_rate * secs;
            self.last_refresh = now;
        }
    }

    /// Win rate
    pub fn win_rate(&self) -> f64 {
        let total = self.wins + self.losses;
        if total == 0 {
            return 0.0;
        }
        self.wins as f64 / total as f64
    }
}

// ============================================================================
// AUCTION MANAGER
// ============================================================================

/// Auction stats
#[derive(Debug, Clone, Default)]
pub struct CoopAuctionStats {
    /// Active auctions
    pub active: usize,
    /// Total settled
    pub settled: u64,
    /// Total revenue
    pub total_revenue: u64,
    /// Average winning price
    pub avg_price: f64,
}

/// Cooperative auction manager
pub struct CoopAuctionManager {
    /// Auctions
    auctions: BTreeMap<u64, Auction>,
    /// Budgets
    budgets: BTreeMap<u64, AuctionBudget>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopAuctionStats,
    /// Revenue sum
    revenue_sum: u64,
    /// Settled count
    settled_count: u64,
}

impl CoopAuctionManager {
    pub fn new() -> Self {
        Self {
            auctions: BTreeMap::new(),
            budgets: BTreeMap::new(),
            next_id: 1,
            stats: CoopAuctionStats::default(),
            revenue_sum: 0,
            settled_count: 0,
        }
    }

    /// Set budget for process
    pub fn set_budget(&mut self, pid: u64, credits: u64) {
        self.budgets
            .entry(pid)
            .or_insert_with(|| AuctionBudget::new(pid, 0))
            .credits = credits;
    }

    /// Create auction
    pub fn create_auction(
        &mut self,
        auction_type: AuctionType,
        resource: AuctionResource,
        quantity: u64,
        reserve: u64,
        deadline: u64,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let auction = Auction::new(id, auction_type, resource, quantity, reserve, deadline, now);
        self.auctions.insert(id, auction);
        self.update_stats();
        id
    }

    /// Place bid
    pub fn bid(&mut self, auction_id: u64, bidder: u64, amount: u64, now: u64) -> bool {
        // Check budget
        let has_budget = self
            .budgets
            .get(&bidder)
            .map(|b| b.credits >= amount)
            .unwrap_or(false);
        if !has_budget {
            return false;
        }

        if let Some(auction) = self.auctions.get_mut(&auction_id) {
            let bid = Bid::new(bidder, amount, amount, now);
            auction.place_bid(bid)
        } else {
            false
        }
    }

    /// Settle auction
    pub fn settle(&mut self, auction_id: u64) -> Option<(u64, u64)> {
        let result = if let Some(auction) = self.auctions.get_mut(&auction_id) {
            auction.settle()
        } else {
            None
        };

        if let Some((winner, price)) = result {
            // Charge winner
            if let Some(budget) = self.budgets.get_mut(&winner) {
                budget.spend(price);
                budget.wins += 1;
            }
            // Record losses for other bidders
            if let Some(auction) = self.auctions.get(&auction_id) {
                for bid in &auction.bids {
                    if bid.bidder != winner {
                        if let Some(budget) = self.budgets.get_mut(&bid.bidder) {
                            budget.losses += 1;
                        }
                    }
                }
            }
            self.revenue_sum += price;
            self.settled_count += 1;
            self.stats.total_revenue = self.revenue_sum;
            self.stats.settled = self.settled_count;
            if self.settled_count > 0 {
                self.stats.avg_price = self.revenue_sum as f64 / self.settled_count as f64;
            }
        }
        self.update_stats();
        result
    }

    /// Refresh budgets
    pub fn refresh_budgets(&mut self, now: u64) {
        for budget in self.budgets.values_mut() {
            budget.refresh(now);
        }
    }

    fn update_stats(&mut self) {
        self.stats.active = self
            .auctions
            .values()
            .filter(|a| a.state == AuctionState::Open)
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &CoopAuctionStats {
        &self.stats
    }
}
