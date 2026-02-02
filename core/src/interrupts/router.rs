//! # Interrupt Router
//!
//! Routes interrupts to appropriate handlers based on configuration.

use alloc::collections::BTreeMap;

use helix_hal::interrupts::InterruptVector;
use spin::RwLock;

/// Interrupt routing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingMode {
    /// Route to a specific CPU
    Cpu(usize),
    /// Round-robin across CPUs
    RoundRobin,
    /// Route to the lowest-load CPU
    LowestLoad,
    /// Route to the local CPU
    Local,
}

/// Interrupt router
pub struct InterruptRouter {
    /// Routing configuration per vector
    routes: RwLock<BTreeMap<InterruptVector, RoutingMode>>,
    /// Round-robin counter
    rr_counter: core::sync::atomic::AtomicUsize,
}

impl InterruptRouter {
    /// Create a new router
    pub const fn new() -> Self {
        Self {
            routes: RwLock::new(BTreeMap::new()),
            rr_counter: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Set routing for an interrupt vector
    pub fn set_route(&self, vector: InterruptVector, mode: RoutingMode) {
        self.routes.write().insert(vector, mode);
    }

    /// Get routing for an interrupt vector
    pub fn get_route(&self, vector: InterruptVector) -> RoutingMode {
        self.routes
            .read()
            .get(&vector)
            .copied()
            .unwrap_or(RoutingMode::Local)
    }

    /// Determine which CPU should handle an interrupt
    pub fn route_to_cpu(&self, vector: InterruptVector, cpu_count: usize) -> usize {
        match self.get_route(vector) {
            RoutingMode::Cpu(cpu) => cpu % cpu_count,
            RoutingMode::RoundRobin => {
                let counter = self
                    .rr_counter
                    .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                counter % cpu_count
            },
            RoutingMode::LowestLoad => {
                // TODO: Implement actual load balancing
                0
            },
            RoutingMode::Local => {
                // TODO: Get current CPU
                0
            },
        }
    }
}

/// Global interrupt router
static ROUTER: InterruptRouter = InterruptRouter::new();

/// Get the interrupt router
pub fn router() -> &'static InterruptRouter {
    &ROUTER
}
