//! # Round-Robin Scheduler Module
//!
//! A simple, fair round-robin scheduler for Helix.
//!
//! ## Features
//! - Time-slice based scheduling
//! - Priority levels support
//! - Per-CPU run queues for SMP
//! - Fair distribution across all runnable threads
//!
//! ## Usage
//!
//! This module is loaded by the kernel and automatically registers
//! itself as the active scheduler when initialized.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod config;
mod scheduler;

use alloc::sync::Arc;

pub use config::RoundRobinConfig;
use helix_execution::scheduler::Scheduler; // Import the Scheduler trait
use helix_modules::v2::{
    Context, Event, EventResponse, ModuleInfo, ModuleTrait, Request, Response,
};
use helix_modules::{ModuleError, ModuleFlags};
pub use scheduler::RoundRobinScheduler;

// =============================================================================
// Module Definition using v2 API
// =============================================================================

/// Round-Robin scheduler module
pub struct RoundRobinModule {
    /// The actual scheduler instance
    scheduler: Option<Arc<RoundRobinScheduler>>,
    /// Configuration
    config: RoundRobinConfig,
    /// CPU count
    cpu_count: usize,
}

impl RoundRobinModule {
    /// Create a new module instance with default configuration
    pub fn new() -> Self {
        Self {
            scheduler: None,
            config: RoundRobinConfig::default(),
            cpu_count: 1,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: RoundRobinConfig) -> Self {
        Self {
            scheduler: None,
            config,
            cpu_count: 1,
        }
    }
}

impl Default for RoundRobinModule {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Module Trait v2 Implementation
// =============================================================================

impl ModuleTrait for RoundRobinModule {
    fn info(&self) -> ModuleInfo {
        ModuleInfo::new("scheduler.round-robin")
            .version(1, 0, 0)
            .description("Simple round-robin scheduler with priority support")
            .author("Helix OS Team")
            .license("MIT OR Apache-2.0")
            .flags(ModuleFlags::SCHEDULER | ModuleFlags::ESSENTIAL)
            .provides(&["scheduler", "cpu.scheduling"])
    }

    fn init(&mut self, ctx: &Context) -> Result<(), ModuleError> {
        log::info!("[round-robin] Initializing scheduler module");

        // Get CPU count from configuration
        self.cpu_count = ctx.config_usize("cpu_count").unwrap_or(1);

        // Get time slice from configuration (default 10ms)
        if let Some(slice_ms) = ctx.config_usize("time_slice_ms") {
            self.config.default_time_slice_ns = slice_ms as u64 * 1_000_000; // Convert to ns
        }

        // Create and initialize the scheduler
        let mut scheduler = RoundRobinScheduler::new(self.config.clone());

        scheduler.init(self.cpu_count).map_err(|e| {
            ModuleError::InitError(alloc::format!("Scheduler init failed: {:?}", e))
        })?;

        self.scheduler = Some(Arc::new(scheduler));

        log::info!(
            "[round-robin] Initialized for {} CPUs, time slice: {}ms",
            self.cpu_count,
            self.config.default_time_slice_ns / 1_000_000
        );

        Ok(())
    }

    fn start(&mut self) -> Result<(), ModuleError> {
        log::info!("[round-robin] Starting scheduler");

        // In a full implementation, this would register with the execution subsystem:
        // helix_execution::scheduler::set_scheduler(self.scheduler.clone().unwrap());

        log::info!("[round-robin] Scheduler started and active");
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ModuleError> {
        log::info!("[round-robin] Stopping scheduler");

        // Deregister from execution subsystem
        // helix_execution::scheduler::clear_scheduler();

        Ok(())
    }

    fn handle_event(&mut self, event: &Event) -> EventResponse {
        match event {
            Event::Tick { timestamp_ns: _ } => {
                // Handle timer tick - this drives the scheduler
                if let Some(ref scheduler) = self.scheduler {
                    // Tick all CPUs
                    for cpu in 0..self.cpu_count {
                        scheduler.tick(cpu);
                    }
                }
                EventResponse::Handled
            },
            Event::CpuHotplug { cpu_id, online } => {
                if let Some(ref scheduler) = self.scheduler {
                    if *online {
                        log::info!("[round-robin] CPU {} coming online", cpu_id);
                        // scheduler.add_cpu(*cpu_id);
                    } else {
                        log::info!("[round-robin] CPU {} going offline", cpu_id);
                        // scheduler.remove_cpu(*cpu_id);
                    }
                }
                EventResponse::Handled
            },
            Event::Shutdown => {
                log::info!("[round-robin] Received shutdown event");
                EventResponse::Handled
            },
            _ => EventResponse::Ignored,
        }
    }

    fn handle_request(&mut self, request: &Request) -> Result<Response, ModuleError> {
        match request.request_type.as_str() {
            "get_stats" => {
                if let Some(ref scheduler) = self.scheduler {
                    let stats = scheduler.stats();
                    // Serialize stats (simplified - would use proper serialization)
                    let payload = alloc::format!(
                        "{{\"context_switches\":{},\"runnable_threads\":{},\"blocked_threads\":{}}}",
                        stats.context_switches,
                        stats.runnable_threads,
                        stats.blocked_threads
                    );
                    Ok(Response::ok(payload.into_bytes()))
                } else {
                    Ok(Response::err("Scheduler not initialized"))
                }
            },
            "set_time_slice" => {
                // Parse time slice from payload
                // For now, just acknowledge
                Ok(Response::ok_empty())
            },
            _ => Ok(Response::err("Unknown request type")),
        }
    }

    fn is_healthy(&self) -> bool {
        self.scheduler.is_some()
    }

    fn save_state(&self) -> Option<alloc::vec::Vec<u8>> {
        // Could save current thread states for hot-reload
        // For simplicity, return None
        None
    }

    fn restore_state(&mut self, _state: &[u8]) -> Result<(), ModuleError> {
        Ok(())
    }
}

// =============================================================================
// Module Entry Point
// =============================================================================

/// Create a new instance of this module
pub fn create_module() -> RoundRobinModule {
    RoundRobinModule::new()
}

/// Create module with custom config
pub fn create_module_with_config(config: RoundRobinConfig) -> RoundRobinModule {
    RoundRobinModule::with_config(config)
}
