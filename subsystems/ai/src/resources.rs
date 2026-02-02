//! # Resource Orchestration Oracle
//!
//! The Resource Oracle intelligently manages and orchestrates computational
//! resources across CPU, GPU, NPU, and other accelerators for optimal performance.
//!
//! ## Capabilities
//!
//! - **Workload Distribution**: Balance load across available compute units
//! - **Device Selection**: Choose optimal device for each task
//! - **Power Management**: Balance performance with power consumption
//! - **Memory Management**: Intelligent memory allocation across devices
//! - **Predictive Allocation**: Pre-allocate resources for anticipated needs
//! - **Thermal Management**: Prevent overheating through load balancing
//!
//! ## Architecture
//!
//! ```text
//!   Task Queue ───────►┌─────────────────────────────────────┐
//!                      │       Resource Oracle               │
//!   Device Status ────►│                                     │
//!                      │  ┌─────────────────────────────┐    │
//!   Power Budget ─────►│  │    Device Manager           │    │
//!                      │  │  - CPU Cores               │    │
//!                      │  │  - GPU Compute Units       │    │
//!                      │  │  - NPU Engines             │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Workload Analyzer        │    │
//!                      │  │  - Task Classification     │    │
//!                      │  │  - Resource Requirements   │    │
//!                      │  │  - Dependency Graph        │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Placement Optimizer      │    │
//!                      │  │  - Affinity Rules          │    │
//!                      │  │  - Load Balancing          │    │
//!                      │  │  - Thermal Constraints     │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Scheduler                │───────► Resource Assignments
//!                      │  └─────────────────────────────┘    │
//!                      │                                     │
//!                      └─────────────────────────────────────┘
//! ```

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use crate::core::{
    AiAction, AiDecision, AiEvent, AiPriority, Confidence, DecisionContext, DecisionId,
    PowerProfile, ResourceType,
};

// =============================================================================
// Compute Devices
// =============================================================================

/// Type of compute device
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceType {
    /// CPU cores
    Cpu,
    /// GPU compute units
    Gpu,
    /// Neural Processing Unit
    Npu,
    /// Digital Signal Processor
    Dsp,
    /// FPGA accelerator
    Fpga,
    /// Custom accelerator
    Custom(u32),
}

/// A compute device in the system
#[derive(Debug, Clone)]
pub struct ComputeDevice {
    /// Unique device ID
    pub id: u64,
    /// Device type
    pub device_type: DeviceType,
    /// Device name
    pub name: String,
    /// Number of compute units
    pub compute_units: u32,
    /// Total memory (bytes)
    pub memory_bytes: u64,
    /// Current utilization (0-100)
    pub utilization: u8,
    /// Current temperature (Celsius)
    pub temperature_c: u8,
    /// Maximum temperature before throttling
    pub max_temp_c: u8,
    /// Current power consumption (mW)
    pub power_mw: u32,
    /// Maximum power draw (mW)
    pub max_power_mw: u32,
    /// Available memory (bytes)
    pub available_memory: u64,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    /// Device status
    pub status: DeviceStatus,
}

/// Device capabilities
#[derive(Debug, Clone, Default)]
pub struct DeviceCapabilities {
    /// Supports FP32 operations
    pub fp32: bool,
    /// Supports FP16 operations
    pub fp16: bool,
    /// Supports INT8 operations
    pub int8: bool,
    /// Supports tensor operations
    pub tensor_cores: bool,
    /// Supports ray tracing
    pub ray_tracing: bool,
    /// Supports video encode/decode
    pub video_codec: bool,
    /// Maximum concurrent kernels
    pub max_concurrent_kernels: u32,
    /// Memory bandwidth (GB/s)
    pub memory_bandwidth_gbps: f32,
    /// Compute capability version
    pub compute_version: String,
}

/// Device status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Device is available
    Available,
    /// Device is busy
    Busy,
    /// Device is throttled (thermal)
    Throttled,
    /// Device is in power saving mode
    PowerSaving,
    /// Device is unavailable
    Unavailable,
    /// Device has an error
    Error,
}

// =============================================================================
// Workload Profiling
// =============================================================================

/// Profile of a workload for resource allocation
#[derive(Debug, Clone)]
pub struct WorkloadProfile {
    /// Unique workload ID
    pub id: u64,
    /// Workload name/type
    pub name: String,
    /// Required compute units
    pub compute_requirement: ComputeRequirement,
    /// Memory requirements
    pub memory_requirement: MemoryRequirement,
    /// Preferred device type
    pub preferred_device: Option<DeviceType>,
    /// Fallback device types (in order of preference)
    pub fallback_devices: Vec<DeviceType>,
    /// Priority
    pub priority: TaskPriority,
    /// Deadline (optional, microseconds from now)
    pub deadline_us: Option<u64>,
    /// Energy preference
    pub energy_preference: EnergyPreference,
    /// Whether workload can be split across devices
    pub splittable: bool,
    /// Dependencies on other workloads
    pub dependencies: Vec<u64>,
}

/// Compute requirements
#[derive(Debug, Clone)]
pub struct ComputeRequirement {
    /// Minimum FLOPS needed
    pub min_flops: u64,
    /// Optimal FLOPS for best performance
    pub optimal_flops: u64,
    /// Required compute precision
    pub precision: ComputePrecision,
    /// Estimated duration (microseconds)
    pub estimated_duration_us: u64,
    /// Parallelism level
    pub parallelism: Parallelism,
}

/// Memory requirements
#[derive(Debug, Clone)]
pub struct MemoryRequirement {
    /// Minimum memory needed (bytes)
    pub min_bytes: u64,
    /// Optimal memory for best performance
    pub optimal_bytes: u64,
    /// Bandwidth requirement (GB/s)
    pub bandwidth_gbps: f32,
    /// Whether memory must be contiguous
    pub contiguous: bool,
    /// Memory access pattern
    pub access_pattern: MemoryAccessPattern,
}

/// Compute precision levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputePrecision {
    FP64,
    FP32,
    FP16,
    BF16,
    INT32,
    INT16,
    INT8,
    Mixed,
}

/// Parallelism characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parallelism {
    /// Completely serial
    Serial,
    /// SIMD vectorizable
    Simd,
    /// Embarrassingly parallel
    Parallel,
    /// Data parallel (same operation on different data)
    DataParallel,
    /// Task parallel (different operations)
    TaskParallel,
}

/// Memory access patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessPattern {
    Sequential,
    Random,
    Strided,
    Streaming,
    Mixed,
}

/// Task priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Background = 0,
    Low        = 1,
    Normal     = 2,
    High       = 3,
    RealTime   = 4,
    Critical   = 5,
}

/// Energy preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyPreference {
    /// Minimize power consumption
    MinPower,
    /// Balance power and performance
    Balanced,
    /// Maximum performance
    MaxPerformance,
    /// Meet deadline with minimum power
    Deadline,
}

// =============================================================================
// Resource Allocation
// =============================================================================

/// Allocated resources for a workload
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Workload ID
    pub workload_id: u64,
    /// Allocated devices
    pub devices: Vec<AllocatedDevice>,
    /// Total allocated compute
    pub total_compute_flops: u64,
    /// Total allocated memory
    pub total_memory_bytes: u64,
    /// Expected power consumption
    pub expected_power_mw: u32,
    /// Allocation confidence
    pub confidence: Confidence,
    /// Allocation timestamp
    pub allocated_at: u64,
    /// Expected completion time
    pub expected_completion_us: u64,
}

/// A single device allocation
#[derive(Debug, Clone)]
pub struct AllocatedDevice {
    /// Device ID
    pub device_id: u64,
    /// Allocated compute percentage (0-100)
    pub compute_percent: u8,
    /// Allocated memory (bytes)
    pub memory_bytes: u64,
    /// Priority on this device
    pub priority: u8,
}

// =============================================================================
// Resource Oracle Engine
// =============================================================================

/// The Resource Orchestration Oracle
pub struct ResourceOracle {
    /// GPU acceleration enabled
    gpu_enabled: bool,

    /// NPU acceleration enabled
    npu_enabled: bool,

    /// Registered compute devices
    devices: RwLock<Vec<ComputeDevice>>,

    /// Pending workloads
    pending_workloads: Mutex<VecDeque<WorkloadProfile>>,

    /// Active allocations
    active_allocations: RwLock<Vec<ResourceAllocation>>,

    /// Allocation history
    allocation_history: Mutex<VecDeque<AllocationRecord>>,

    /// Current power budget (mW)
    power_budget_mw: RwLock<u32>,

    /// Current power profile
    power_profile: RwLock<PowerProfile>,

    /// Statistics
    stats: ResourceStats,
}

/// Record of an allocation for history
#[derive(Debug, Clone)]
struct AllocationRecord {
    allocation: ResourceAllocation,
    outcome: AllocationOutcome,
    actual_duration_us: u64,
    actual_power_mw: u32,
}

/// Outcome of an allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocationOutcome {
    Completed,
    TimedOut,
    Cancelled,
    Failed,
    Migrated,
}

struct ResourceStats {
    allocations_made: AtomicU64,
    allocations_successful: AtomicU64,
    migrations_performed: AtomicU64,
    power_savings_mw: AtomicU64,
    gpu_offloads: AtomicU64,
    npu_offloads: AtomicU64,
    preemptions: AtomicU64,
}

impl Default for ResourceStats {
    fn default() -> Self {
        Self {
            allocations_made: AtomicU64::new(0),
            allocations_successful: AtomicU64::new(0),
            migrations_performed: AtomicU64::new(0),
            power_savings_mw: AtomicU64::new(0),
            gpu_offloads: AtomicU64::new(0),
            npu_offloads: AtomicU64::new(0),
            preemptions: AtomicU64::new(0),
        }
    }
}

impl ResourceOracle {
    /// Maximum allocation history size
    const MAX_HISTORY: usize = 1000;

    /// Create a new Resource Oracle
    pub fn new(gpu_enabled: bool, npu_enabled: bool) -> Self {
        Self {
            gpu_enabled,
            npu_enabled,
            devices: RwLock::new(Self::discover_devices(gpu_enabled, npu_enabled)),
            pending_workloads: Mutex::new(VecDeque::new()),
            active_allocations: RwLock::new(Vec::new()),
            allocation_history: Mutex::new(VecDeque::with_capacity(Self::MAX_HISTORY)),
            power_budget_mw: RwLock::new(u32::MAX), // Unlimited by default
            power_profile: RwLock::new(PowerProfile::Balanced),
            stats: ResourceStats::default(),
        }
    }

    /// Discover available compute devices
    fn discover_devices(gpu_enabled: bool, npu_enabled: bool) -> Vec<ComputeDevice> {
        let mut devices = Vec::new();

        // Always add CPU
        devices.push(ComputeDevice {
            id: 0,
            device_type: DeviceType::Cpu,
            name: "System CPU".to_string(),
            compute_units: 8,                      // Would be detected
            memory_bytes: 16 * 1024 * 1024 * 1024, // 16 GB
            utilization: 0,
            temperature_c: 45,
            max_temp_c: 100,
            power_mw: 5000,
            max_power_mw: 65000,
            available_memory: 12 * 1024 * 1024 * 1024,
            capabilities: DeviceCapabilities {
                fp32: true,
                fp16: true,
                int8: true,
                tensor_cores: false,
                ray_tracing: false,
                video_codec: false,
                max_concurrent_kernels: 8,
                memory_bandwidth_gbps: 50.0,
                compute_version: "x86_64".to_string(),
            },
            status: DeviceStatus::Available,
        });

        // Add GPU if enabled
        if gpu_enabled {
            devices.push(ComputeDevice {
                id: 1,
                device_type: DeviceType::Gpu,
                name: "Discrete GPU".to_string(),
                compute_units: 60,                    // Streaming multiprocessors
                memory_bytes: 8 * 1024 * 1024 * 1024, // 8 GB VRAM
                utilization: 0,
                temperature_c: 40,
                max_temp_c: 83,
                power_mw: 10000,
                max_power_mw: 250000,
                available_memory: 7 * 1024 * 1024 * 1024,
                capabilities: DeviceCapabilities {
                    fp32: true,
                    fp16: true,
                    int8: true,
                    tensor_cores: true,
                    ray_tracing: true,
                    video_codec: true,
                    max_concurrent_kernels: 128,
                    memory_bandwidth_gbps: 320.0,
                    compute_version: "8.6".to_string(),
                },
                status: DeviceStatus::Available,
            });
        }

        // Add NPU if enabled
        if npu_enabled {
            devices.push(ComputeDevice {
                id: 2,
                device_type: DeviceType::Npu,
                name: "Neural Processing Unit".to_string(),
                compute_units: 16,
                memory_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
                utilization: 0,
                temperature_c: 35,
                max_temp_c: 80,
                power_mw: 2000,
                max_power_mw: 15000,
                available_memory: 2 * 1024 * 1024 * 1024,
                capabilities: DeviceCapabilities {
                    fp32: true,
                    fp16: true,
                    int8: true,
                    tensor_cores: true,
                    ray_tracing: false,
                    video_codec: false,
                    max_concurrent_kernels: 4,
                    memory_bandwidth_gbps: 64.0,
                    compute_version: "npu-1.0".to_string(),
                },
                status: DeviceStatus::Available,
            });
        }

        devices
    }

    /// Analyze an event for resource management
    pub fn analyze(
        &self,
        event: &AiEvent,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        match event {
            AiEvent::CpuThreshold {
                usage_percent,
                cpu_id,
            } => self.handle_cpu_threshold(*usage_percent, *cpu_id, context),
            AiEvent::MemoryPressure { available_percent } => {
                self.handle_memory_pressure(*available_percent, context)
            },
            AiEvent::ProcessResourceSpike { pid, resource } => {
                self.handle_resource_spike(*pid, resource, context)
            },
            _ => Ok(None),
        }
    }

    /// Handle high CPU usage
    fn handle_cpu_threshold(
        &self,
        usage: u8,
        cpu_id: u32,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if usage < 80 {
            return Ok(None);
        }

        // Check if GPU is available for offloading
        if self.gpu_enabled && usage > 90 {
            if let Some(gpu) = self.find_available_device(DeviceType::Gpu) {
                if gpu.utilization < 50 {
                    self.stats.gpu_offloads.fetch_add(1, Ordering::Relaxed);
                    return Ok(Some((
                        AiAction::OffloadToGpu {
                            task_id: 0, // Would be actual task
                            kernel_name: "parallel_compute".to_string(),
                        },
                        Confidence::new(0.75),
                        format!(
                            "CPU {} at {}%, offloading to GPU ({}% utilized)",
                            cpu_id, usage, gpu.utilization
                        ),
                    )));
                }
            }
        }

        // Check if NPU is available for AI workloads
        if self.npu_enabled && usage > 85 {
            if let Some(npu) = self.find_available_device(DeviceType::Npu) {
                if npu.utilization < 70 {
                    self.stats.npu_offloads.fetch_add(1, Ordering::Relaxed);
                    return Ok(Some((
                        AiAction::OffloadToNpu {
                            task_id: 0,
                            model_id: 0,
                        },
                        Confidence::new(0.7),
                        format!("CPU {} at {}%, offloading to NPU", cpu_id, usage),
                    )));
                }
            }
        }

        // Adjust power profile for more performance
        if usage > 95 {
            return Ok(Some((
                AiAction::SetPowerProfile {
                    profile: PowerProfile::Performance,
                },
                Confidence::new(0.8),
                format!(
                    "CPU critically high ({}%), enabling performance mode",
                    usage
                ),
            )));
        }

        Ok(None)
    }

    /// Handle memory pressure
    fn handle_memory_pressure(
        &self,
        available: u8,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if available > 20 {
            return Ok(None);
        }

        // Check if GPU has available memory to offload to
        if self.gpu_enabled && available < 15 {
            if let Some(gpu) = self.find_available_device(DeviceType::Gpu) {
                let gpu_available =
                    (gpu.available_memory as f64 / gpu.memory_bytes as f64 * 100.0) as u8;
                if gpu_available > 50 {
                    return Ok(Some((
                        AiAction::PreallocateResources {
                            resource: ResourceType::Gpu,
                            amount: 256 * 1024 * 1024, // 256 MB
                        },
                        Confidence::new(0.7),
                        format!("System memory low ({}% free), using GPU memory", available),
                    )));
                }
            }
        }

        Ok(None)
    }

    /// Handle resource spike
    fn handle_resource_spike(
        &self,
        pid: u64,
        resource: &ResourceType,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        match resource {
            ResourceType::Cpu => {
                // Consider migrating to less loaded CPU
                Ok(Some((
                    AiAction::MigrateProcess {
                        pid,
                        from_cpu: 0,
                        to_cpu: self.find_least_loaded_cpu(),
                    },
                    Confidence::new(0.65),
                    format!(
                        "Migrating CPU-intensive process {} to less loaded core",
                        pid
                    ),
                )))
            },
            ResourceType::Gpu => {
                // May need to preempt lower priority GPU tasks
                self.stats.preemptions.fetch_add(1, Ordering::Relaxed);
                Ok(Some((
                    AiAction::AdjustProcessPriority {
                        pid,
                        old_priority: 0,
                        new_priority: -5, // Higher priority
                    },
                    Confidence::new(0.6),
                    format!("Boosting GPU process {} priority", pid),
                )))
            },
            _ => Ok(None),
        }
    }

    /// Proactive resource check
    pub fn proactive_check(&self, context: &DecisionContext) -> Result<Option<AiDecision>, ()> {
        // Check for thermal throttling risk
        if let Some(action) = self.check_thermal_status() {
            return Ok(Some(AiDecision {
                id: DecisionId::new(),
                timestamp: 0,
                action,
                confidence: Confidence::new(0.8),
                priority: AiPriority::High,
                reasoning: vec!["Preventing thermal throttling".to_string()],
                expected_outcome: "Maintain performance stability".to_string(),
                rollback: None,
                context: context.clone(),
            }));
        }

        // Check power budget
        if let Some(action) = self.check_power_budget() {
            return Ok(Some(AiDecision {
                id: DecisionId::new(),
                timestamp: 0,
                action,
                confidence: Confidence::new(0.7),
                priority: AiPriority::Normal,
                reasoning: vec!["Optimizing power consumption".to_string()],
                expected_outcome: "Reduce power usage while maintaining performance".to_string(),
                rollback: None,
                context: context.clone(),
            }));
        }

        // Look for optimization opportunities
        if let Some(action) = self.find_optimization_opportunity() {
            return Ok(Some(AiDecision {
                id: DecisionId::new(),
                timestamp: 0,
                action,
                confidence: Confidence::new(0.6),
                priority: AiPriority::Low,
                reasoning: vec!["Proactive resource optimization".to_string()],
                expected_outcome: "Improved resource utilization".to_string(),
                rollback: None,
                context: context.clone(),
            }));
        }

        Ok(None)
    }

    /// Check thermal status of devices
    fn check_thermal_status(&self) -> Option<AiAction> {
        let devices = self.devices.read();

        for device in devices.iter() {
            if device.temperature_c > device.max_temp_c - 10 {
                // Close to thermal limit
                return Some(match device.device_type {
                    DeviceType::Cpu => AiAction::SetPowerProfile {
                        profile: PowerProfile::Balanced,
                    },
                    DeviceType::Gpu => AiAction::SuspendIdleProcesses {
                        threshold_seconds: 30,
                    },
                    _ => continue,
                });
            }
        }

        None
    }

    /// Check power budget
    fn check_power_budget(&self) -> Option<AiAction> {
        let devices = self.devices.read();
        let budget = *self.power_budget_mw.read();

        if budget == u32::MAX {
            return None; // Unlimited
        }

        let total_power: u32 = devices.iter().map(|d| d.power_mw).sum();

        if total_power > budget * 90 / 100 {
            // Over 90% of budget
            return Some(AiAction::SetPowerProfile {
                profile: PowerProfile::PowerSaver,
            });
        }

        None
    }

    /// Find optimization opportunities
    fn find_optimization_opportunity(&self) -> Option<AiAction> {
        let devices = self.devices.read();

        // Check for underutilized accelerators
        for device in devices.iter() {
            if device.device_type == DeviceType::Gpu && device.utilization < 10 {
                // GPU is nearly idle, might want to power down
                return Some(AiAction::SetPowerProfile {
                    profile: PowerProfile::Balanced,
                });
            }
        }

        None
    }

    /// Find an available device of a specific type
    fn find_available_device(&self, device_type: DeviceType) -> Option<ComputeDevice> {
        self.devices
            .read()
            .iter()
            .find(|d| d.device_type == device_type && d.status == DeviceStatus::Available)
            .cloned()
    }

    /// Find the least loaded CPU core
    fn find_least_loaded_cpu(&self) -> u32 {
        // In real implementation, would query actual CPU stats
        0
    }

    /// Submit a workload for allocation
    pub fn submit_workload(&self, workload: WorkloadProfile) {
        self.pending_workloads.lock().push_back(workload);
    }

    /// Allocate resources for a workload
    pub fn allocate(&self, workload: &WorkloadProfile) -> Option<ResourceAllocation> {
        let devices = self.devices.read();

        // Find best device for this workload
        let device = self.select_device(workload, &devices)?;

        let allocation = ResourceAllocation {
            workload_id: workload.id,
            devices: vec![AllocatedDevice {
                device_id: device.id,
                compute_percent: 100,
                memory_bytes: workload.memory_requirement.min_bytes,
                priority: workload.priority as u8,
            }],
            total_compute_flops: workload.compute_requirement.optimal_flops,
            total_memory_bytes: workload.memory_requirement.min_bytes,
            expected_power_mw: device.power_mw,
            confidence: Confidence::new(0.8),
            allocated_at: 0,
            expected_completion_us: workload.compute_requirement.estimated_duration_us,
        };

        self.stats.allocations_made.fetch_add(1, Ordering::Relaxed);
        self.active_allocations.write().push(allocation.clone());

        Some(allocation)
    }

    /// Select best device for a workload
    fn select_device<'a>(
        &self,
        workload: &WorkloadProfile,
        devices: &'a [ComputeDevice],
    ) -> Option<&'a ComputeDevice> {
        // Check preferred device first
        if let Some(preferred) = workload.preferred_device {
            if let Some(device) = devices
                .iter()
                .find(|d| d.device_type == preferred && d.status == DeviceStatus::Available)
            {
                if device.available_memory >= workload.memory_requirement.min_bytes {
                    return Some(device);
                }
            }
        }

        // Try fallback devices
        for fallback in &workload.fallback_devices {
            if let Some(device) = devices
                .iter()
                .find(|d| d.device_type == *fallback && d.status == DeviceStatus::Available)
            {
                if device.available_memory >= workload.memory_requirement.min_bytes {
                    return Some(device);
                }
            }
        }

        // Default to CPU
        devices
            .iter()
            .find(|d| d.device_type == DeviceType::Cpu && d.status == DeviceStatus::Available)
    }

    /// Release an allocation
    pub fn release(&self, workload_id: u64) {
        let mut allocations = self.active_allocations.write();
        if let Some(pos) = allocations
            .iter()
            .position(|a| a.workload_id == workload_id)
        {
            let allocation = allocations.remove(pos);

            // Record in history
            let mut history = self.allocation_history.lock();
            while history.len() >= Self::MAX_HISTORY {
                history.pop_front();
            }
            history.push_back(AllocationRecord {
                allocation,
                outcome: AllocationOutcome::Completed,
                actual_duration_us: 0,
                actual_power_mw: 0,
            });

            self.stats
                .allocations_successful
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Update device status
    pub fn update_device(&self, device_id: u64, utilization: u8, temperature: u8, power: u32) {
        let mut devices = self.devices.write();
        if let Some(device) = devices.iter_mut().find(|d| d.id == device_id) {
            device.utilization = utilization;
            device.temperature_c = temperature;
            device.power_mw = power;

            // Update status based on metrics
            device.status = if temperature >= device.max_temp_c - 5 {
                DeviceStatus::Throttled
            } else if utilization > 95 {
                DeviceStatus::Busy
            } else {
                DeviceStatus::Available
            };
        }
    }

    /// Set power budget
    pub fn set_power_budget(&self, budget_mw: u32) {
        *self.power_budget_mw.write() = budget_mw;
    }

    /// Set power profile
    pub fn set_power_profile(&self, profile: PowerProfile) {
        *self.power_profile.write() = profile;
    }

    /// Get all devices
    pub fn devices(&self) -> Vec<ComputeDevice> {
        self.devices.read().clone()
    }

    /// Get active allocations
    pub fn active_allocations(&self) -> Vec<ResourceAllocation> {
        self.active_allocations.read().clone()
    }

    /// Get statistics
    pub fn statistics(&self) -> ResourceOracleStatistics {
        let devices = self.devices.read();
        let total_power: u32 = devices.iter().map(|d| d.power_mw).sum();

        ResourceOracleStatistics {
            gpu_enabled: self.gpu_enabled,
            npu_enabled: self.npu_enabled,
            devices_count: devices.len(),
            allocations_made: self.stats.allocations_made.load(Ordering::Relaxed),
            allocations_successful: self.stats.allocations_successful.load(Ordering::Relaxed),
            migrations_performed: self.stats.migrations_performed.load(Ordering::Relaxed),
            power_savings_mw: self.stats.power_savings_mw.load(Ordering::Relaxed),
            gpu_offloads: self.stats.gpu_offloads.load(Ordering::Relaxed),
            npu_offloads: self.stats.npu_offloads.load(Ordering::Relaxed),
            preemptions: self.stats.preemptions.load(Ordering::Relaxed),
            active_allocations: self.active_allocations.read().len(),
            current_power_mw: total_power,
            power_budget_mw: *self.power_budget_mw.read(),
        }
    }
}

/// Public statistics structure
#[derive(Debug, Clone)]
pub struct ResourceOracleStatistics {
    pub gpu_enabled: bool,
    pub npu_enabled: bool,
    pub devices_count: usize,
    pub allocations_made: u64,
    pub allocations_successful: u64,
    pub migrations_performed: u64,
    pub power_savings_mw: u64,
    pub gpu_offloads: u64,
    pub npu_offloads: u64,
    pub preemptions: u64,
    pub active_allocations: usize,
    pub current_power_mw: u32,
    pub power_budget_mw: u32,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_oracle_creation() {
        let oracle = ResourceOracle::new(true, true);
        let devices = oracle.devices();

        assert!(devices.len() >= 1); // At least CPU
        assert!(devices.iter().any(|d| d.device_type == DeviceType::Cpu));
        assert!(devices.iter().any(|d| d.device_type == DeviceType::Gpu));
        assert!(devices.iter().any(|d| d.device_type == DeviceType::Npu));
    }

    #[test]
    fn test_resource_oracle_no_accelerators() {
        let oracle = ResourceOracle::new(false, false);
        let devices = oracle.devices();

        assert_eq!(devices.len(), 1); // Only CPU
        assert_eq!(devices[0].device_type, DeviceType::Cpu);
    }

    #[test]
    fn test_workload_allocation() {
        let oracle = ResourceOracle::new(false, false);

        let workload = WorkloadProfile {
            id: 1,
            name: "test_workload".to_string(),
            compute_requirement: ComputeRequirement {
                min_flops: 1_000_000,
                optimal_flops: 10_000_000,
                precision: ComputePrecision::FP32,
                estimated_duration_us: 1000,
                parallelism: Parallelism::Parallel,
            },
            memory_requirement: MemoryRequirement {
                min_bytes: 1024 * 1024,
                optimal_bytes: 10 * 1024 * 1024,
                bandwidth_gbps: 1.0,
                contiguous: false,
                access_pattern: MemoryAccessPattern::Sequential,
            },
            preferred_device: Some(DeviceType::Cpu),
            fallback_devices: Vec::new(),
            priority: TaskPriority::Normal,
            deadline_us: None,
            energy_preference: EnergyPreference::Balanced,
            splittable: false,
            dependencies: Vec::new(),
        };

        let allocation = oracle.allocate(&workload);
        assert!(allocation.is_some());

        let alloc = allocation.unwrap();
        assert_eq!(alloc.workload_id, 1);
        assert!(!alloc.devices.is_empty());
    }

    #[test]
    fn test_device_update() {
        let oracle = ResourceOracle::new(true, false);

        // Update GPU device
        oracle.update_device(1, 80, 75, 150000);

        let devices = oracle.devices();
        let gpu = devices.iter().find(|d| d.id == 1).unwrap();

        assert_eq!(gpu.utilization, 80);
        assert_eq!(gpu.temperature_c, 75);
        assert_eq!(gpu.power_mw, 150000);
    }

    #[test]
    fn test_power_budget() {
        let oracle = ResourceOracle::new(true, true);

        oracle.set_power_budget(100000); // 100W
        let stats = oracle.statistics();
        assert_eq!(stats.power_budget_mw, 100000);
    }
}
