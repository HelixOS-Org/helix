//! # Cooperative Exchange
//!
//! Resource exchange marketplace for cooperative processes:
//! - Bid/ask matching
//! - Resource pricing
//! - Exchange order book
//! - Trade settlement
//! - Market-based allocation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// EXCHANGE TYPES
// ============================================================================

/// Exchangeable resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExchangeResource {
    /// CPU time (ns)
    CpuTime,
    /// Memory (pages)
    MemoryPages,
    /// I/O bandwidth slots
    IoBandwidth,
    /// Network bandwidth slots
    NetBandwidth,
    /// Priority points
    PriorityPoints,
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    /// Offering resource (sell/ask)
    Offer,
    /// Requesting resource (buy/bid)
    Request,
}

/// Order state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderState {
    /// Open
    Open,
    /// Partially filled
    PartiallyFilled,
    /// Filled
    Filled,
    /// Cancelled
    Cancelled,
    /// Expired
    Expired,
}

// ============================================================================
// ORDER
// ============================================================================

/// Exchange order
#[derive(Debug, Clone)]
pub struct ExchangeOrder {
    /// Order ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Resource
    pub resource: ExchangeResource,
    /// Side
    pub side: OrderSide,
    /// Quantity
    pub quantity: u64,
    /// Filled quantity
    pub filled: u64,
    /// Price (priority points per unit)
    pub price: u32,
    /// State
    pub state: OrderState,
    /// Created at
    pub created_at: u64,
    /// Expiry
    pub expiry: u64,
}

impl ExchangeOrder {
    pub fn new(
        id: u64,
        pid: u64,
        resource: ExchangeResource,
        side: OrderSide,
        quantity: u64,
        price: u32,
        now: u64,
        duration_ns: u64,
    ) -> Self {
        Self {
            id,
            pid,
            resource,
            side,
            quantity,
            filled: 0,
            price,
            state: OrderState::Open,
            created_at: now,
            expiry: now + duration_ns,
        }
    }

    /// Remaining quantity
    pub fn remaining(&self) -> u64 {
        self.quantity.saturating_sub(self.filled)
    }

    /// Fill partially
    pub fn fill(&mut self, amount: u64) {
        self.filled += amount;
        if self.filled >= self.quantity {
            self.state = OrderState::Filled;
        } else {
            self.state = OrderState::PartiallyFilled;
        }
    }

    /// Is open
    pub fn is_open(&self) -> bool {
        matches!(self.state, OrderState::Open | OrderState::PartiallyFilled)
    }

    /// Check expiry
    pub fn check_expiry(&mut self, now: u64) {
        if now >= self.expiry && self.is_open() {
            self.state = OrderState::Expired;
        }
    }
}

// ============================================================================
// TRADE
// ============================================================================

/// Completed trade
#[derive(Debug, Clone)]
pub struct Trade {
    /// Trade ID
    pub id: u64,
    /// Offer order
    pub offer_order: u64,
    /// Request order
    pub request_order: u64,
    /// Offer PID (seller)
    pub offer_pid: u64,
    /// Request PID (buyer)
    pub request_pid: u64,
    /// Resource
    pub resource: ExchangeResource,
    /// Quantity
    pub quantity: u64,
    /// Price
    pub price: u32,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// ORDER BOOK
// ============================================================================

/// Order book for a single resource
#[derive(Debug, Clone)]
pub struct OrderBook {
    /// Resource
    pub resource: ExchangeResource,
    /// Offers (sorted by price ascending — cheapest first)
    offers: Vec<u64>,
    /// Requests (sorted by price descending — highest first)
    requests: Vec<u64>,
}

impl OrderBook {
    pub fn new(resource: ExchangeResource) -> Self {
        Self {
            resource,
            offers: Vec::new(),
            requests: Vec::new(),
        }
    }

    /// Add order
    pub fn add(&mut self, id: u64, side: OrderSide) {
        match side {
            OrderSide::Offer => self.offers.push(id),
            OrderSide::Request => self.requests.push(id),
        }
    }

    /// Remove order
    pub fn remove(&mut self, id: u64) {
        self.offers.retain(|&o| o != id);
        self.requests.retain(|&r| r != id);
    }

    /// Best offer price
    pub fn best_offer(&self) -> Option<u64> {
        self.offers.first().copied()
    }

    /// Best request price
    pub fn best_request(&self) -> Option<u64> {
        self.requests.first().copied()
    }

    /// Depth (open orders)
    pub fn depth(&self) -> (usize, usize) {
        (self.offers.len(), self.requests.len())
    }
}

// ============================================================================
// MARKET STATS
// ============================================================================

/// Per-resource market statistics
#[derive(Debug, Clone)]
pub struct MarketStats {
    /// Resource
    pub resource: ExchangeResource,
    /// Last trade price
    pub last_price: u32,
    /// Volume (units traded)
    pub volume: u64,
    /// Trade count
    pub trade_count: u64,
    /// Average price
    pub avg_price: f64,
    /// High price
    pub high_price: u32,
    /// Low price
    pub low_price: u32,
}

impl MarketStats {
    pub fn new(resource: ExchangeResource) -> Self {
        Self {
            resource,
            last_price: 0,
            volume: 0,
            trade_count: 0,
            avg_price: 0.0,
            high_price: 0,
            low_price: u32::MAX,
        }
    }

    /// Record trade
    pub fn record_trade(&mut self, price: u32, quantity: u64) {
        self.last_price = price;
        self.volume += quantity;
        self.trade_count += 1;
        if price > self.high_price {
            self.high_price = price;
        }
        if price < self.low_price {
            self.low_price = price;
        }
        // Running average
        self.avg_price = (self.avg_price * (self.trade_count - 1) as f64 + price as f64)
            / self.trade_count as f64;
    }
}

// ============================================================================
// EXCHANGE MANAGER
// ============================================================================

/// Exchange stats
#[derive(Debug, Clone, Default)]
pub struct CoopExchangeStats {
    /// Open orders
    pub open_orders: usize,
    /// Total orders
    pub total_orders: u64,
    /// Total trades
    pub total_trades: u64,
    /// Total volume
    pub total_volume: u64,
}

/// Cooperative exchange manager
pub struct CoopExchangeManager {
    /// Orders
    orders: BTreeMap<u64, ExchangeOrder>,
    /// Order books
    books: BTreeMap<u8, OrderBook>,
    /// Trades
    trades: Vec<Trade>,
    /// Market stats
    market_stats: BTreeMap<u8, MarketStats>,
    /// Process balances (priority points)
    balances: BTreeMap<u64, u64>,
    /// Next IDs
    next_order_id: u64,
    next_trade_id: u64,
    /// Stats
    stats: CoopExchangeStats,
}

impl CoopExchangeManager {
    pub fn new() -> Self {
        Self {
            orders: BTreeMap::new(),
            books: BTreeMap::new(),
            trades: Vec::new(),
            market_stats: BTreeMap::new(),
            balances: BTreeMap::new(),
            next_order_id: 1,
            next_trade_id: 1,
            stats: CoopExchangeStats::default(),
        }
    }

    /// Set process balance
    pub fn set_balance(&mut self, pid: u64, balance: u64) {
        self.balances.insert(pid, balance);
    }

    /// Get balance
    pub fn balance(&self, pid: u64) -> u64 {
        self.balances.get(&pid).copied().unwrap_or(0)
    }

    /// Place order
    pub fn place_order(
        &mut self,
        pid: u64,
        resource: ExchangeResource,
        side: OrderSide,
        quantity: u64,
        price: u32,
        now: u64,
        duration_ns: u64,
    ) -> u64 {
        let id = self.next_order_id;
        self.next_order_id += 1;

        let order = ExchangeOrder::new(id, pid, resource, side, quantity, price, now, duration_ns);
        self.orders.insert(id, order);

        let book = self
            .books
            .entry(resource as u8)
            .or_insert_with(|| OrderBook::new(resource));
        book.add(id, side);

        self.stats.total_orders += 1;
        self.update_open_count();
        id
    }

    /// Match orders for a resource
    pub fn match_orders(&mut self, resource: ExchangeResource, now: u64) -> Vec<u64> {
        let book_key = resource as u8;
        let book = match self.books.get(&book_key) {
            Some(b) => b.clone(),
            None => return Vec::new(),
        };

        let mut trade_ids = Vec::new();

        // Simple price-time priority matching
        for &offer_id in &book.offers {
            let offer = match self.orders.get(&offer_id) {
                Some(o) if o.is_open() => o.clone(),
                _ => continue,
            };

            for &request_id in &book.requests {
                let request = match self.orders.get(&request_id) {
                    Some(r) if r.is_open() => r.clone(),
                    _ => continue,
                };

                // Price match: request price >= offer price
                if request.price >= offer.price {
                    let match_qty = offer.remaining().min(request.remaining());
                    if match_qty == 0 {
                        continue;
                    }

                    let trade_price = (offer.price + request.price) / 2;

                    // Execute trade
                    let trade_id = self.next_trade_id;
                    self.next_trade_id += 1;

                    self.trades.push(Trade {
                        id: trade_id,
                        offer_order: offer_id,
                        request_order: request_id,
                        offer_pid: offer.pid,
                        request_pid: request.pid,
                        resource,
                        quantity: match_qty,
                        price: trade_price,
                        timestamp: now,
                    });

                    // Update orders
                    if let Some(o) = self.orders.get_mut(&offer_id) {
                        o.fill(match_qty);
                    }
                    if let Some(r) = self.orders.get_mut(&request_id) {
                        r.fill(match_qty);
                    }

                    // Update balances
                    let cost = match_qty * trade_price as u64;
                    if let Some(bal) = self.balances.get_mut(&request.pid) {
                        *bal = bal.saturating_sub(cost);
                    }
                    *self.balances.entry(offer.pid).or_insert(0) += cost;

                    // Update market stats
                    let ms = self
                        .market_stats
                        .entry(book_key)
                        .or_insert_with(|| MarketStats::new(resource));
                    ms.record_trade(trade_price, match_qty);

                    self.stats.total_trades += 1;
                    self.stats.total_volume += match_qty;
                    trade_ids.push(trade_id);
                }
            }
        }

        // Clean filled orders from books
        if let Some(book) = self.books.get_mut(&book_key) {
            let filled: Vec<u64> = self
                .orders
                .iter()
                .filter(|(_, o)| o.resource == resource && !o.is_open())
                .map(|(&id, _)| id)
                .collect();
            for id in filled {
                book.remove(id);
            }
        }

        self.update_open_count();
        trade_ids
    }

    /// Cancel order
    pub fn cancel_order(&mut self, order_id: u64) {
        if let Some(order) = self.orders.get_mut(&order_id) {
            order.state = OrderState::Cancelled;
            let key = order.resource as u8;
            if let Some(book) = self.books.get_mut(&key) {
                book.remove(order_id);
            }
        }
        self.update_open_count();
    }

    /// Expire old orders
    pub fn expire_orders(&mut self, now: u64) {
        let expired: Vec<u64> = self
            .orders
            .iter()
            .filter(|(_, o)| o.is_open() && now >= o.expiry)
            .map(|(&id, _)| id)
            .collect();

        for id in expired {
            if let Some(order) = self.orders.get_mut(&id) {
                order.state = OrderState::Expired;
                let key = order.resource as u8;
                if let Some(book) = self.books.get_mut(&key) {
                    book.remove(id);
                }
            }
        }
        self.update_open_count();
    }

    fn update_open_count(&mut self) {
        self.stats.open_orders = self.orders.values().filter(|o| o.is_open()).count();
    }

    /// Market stats for resource
    pub fn market_stats(&self, resource: ExchangeResource) -> Option<&MarketStats> {
        self.market_stats.get(&(resource as u8))
    }

    /// Stats
    pub fn stats(&self) -> &CoopExchangeStats {
        &self.stats
    }
}
