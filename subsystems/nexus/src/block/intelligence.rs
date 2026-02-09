//! Block Intelligence
//!
//! Central coordinator for block device analysis.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    BlockDevice, BlockDeviceId, BlockDeviceState, BlockDeviceType, BlockManager, IoRequest,
    IoScheduler,
};

/// Block analysis
#[derive(Debug, Clone)]
pub struct BlockAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Performance score (0-100)
    pub performance_score: f32,
    /// Issues
    pub issues: Vec<BlockIssue>,
    /// Recommendations
    pub recommendations: Vec<BlockRecommendation>,
}

/// Block issue
#[derive(Debug, Clone)]
pub struct BlockIssue {
    /// Issue type
    pub issue_type: BlockIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Device
    pub device: Option<BlockDeviceId>,
}

/// Block issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockIssueType {
    /// High latency
    HighLatency,
    /// Queue full
    QueueFull,
    /// High utilization
    HighUtilization,
    /// Wrong scheduler
    WrongScheduler,
    /// Trim not enabled
    TrimDisabled,
    /// Write cache disabled
    WriteCacheDisabled,
    /// Device error
    DeviceError,
}

/// Block recommendation
#[derive(Debug, Clone)]
pub struct BlockRecommendation {
    /// Action
    pub action: BlockAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Block action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockAction {
    /// Change scheduler
    ChangeScheduler,
    /// Enable trim
    EnableTrim,
    /// Enable write cache
    EnableWriteCache,
    /// Increase queue depth
    IncreaseQueueDepth,
    /// Check health
    CheckHealth,
}

/// Block Intelligence
pub struct BlockIntelligence {
    /// Manager
    manager: BlockManager,
}

impl BlockIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: BlockManager::new(),
        }
    }

    /// Register device
    #[inline(always)]
    pub fn register_device(&mut self, device: BlockDevice) {
        self.manager.register_device(device);
    }

    /// Submit I/O
    #[inline(always)]
    pub fn submit_io(&mut self, device_id: BlockDeviceId, request: IoRequest) {
        self.manager.submit_io(device_id, request);
    }

    /// Complete I/O
    #[inline(always)]
    pub fn complete_io(
        &mut self,
        device_id: BlockDeviceId,
        request_id: u64,
        end_time: u64,
    ) -> Option<IoRequest> {
        self.manager.complete_io(device_id, request_id, end_time)
    }

    /// Analyze block subsystem
    pub fn analyze(&self) -> BlockAnalysis {
        let mut health_score = 100.0f32;
        let mut performance_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        for device in self.manager.devices() {
            // Check device state
            if matches!(device.state, BlockDeviceState::Error) {
                health_score -= 30.0;
                issues.push(BlockIssue {
                    issue_type: BlockIssueType::DeviceError,
                    severity: 10,
                    description: format!("Device {} is in error state", device.name),
                    device: Some(device.id),
                });
                recommendations.push(BlockRecommendation {
                    action: BlockAction::CheckHealth,
                    expected_improvement: 20.0,
                    reason: String::from("Check device health and replace if necessary"),
                });
            }

            // Check scheduler appropriateness
            if device.device_type.is_solid_state()
                && !matches!(device.scheduler, IoScheduler::None | IoScheduler::Kyber)
            {
                performance_score -= 5.0;
                issues.push(BlockIssue {
                    issue_type: BlockIssueType::WrongScheduler,
                    severity: 4,
                    description: format!(
                        "SSD {} using scheduler {}",
                        device.name,
                        device.scheduler.name()
                    ),
                    device: Some(device.id),
                });
                recommendations.push(BlockRecommendation {
                    action: BlockAction::ChangeScheduler,
                    expected_improvement: 5.0,
                    reason: String::from("Use 'none' or 'kyber' scheduler for SSDs"),
                });
            }

            // Check trim support
            if device.device_type.supports_trim() && !device.supports_trim {
                health_score -= 5.0;
                issues.push(BlockIssue {
                    issue_type: BlockIssueType::TrimDisabled,
                    severity: 5,
                    description: format!("TRIM not enabled on {}", device.name),
                    device: Some(device.id),
                });
                recommendations.push(BlockRecommendation {
                    action: BlockAction::EnableTrim,
                    expected_improvement: 5.0,
                    reason: String::from("Enable TRIM for better SSD performance and longevity"),
                });
            }

            // Check queue utilization
            if let Some(queue) = device.request_queue() {
                if queue.depth.utilization() > 0.9 {
                    performance_score -= 10.0;
                    issues.push(BlockIssue {
                        issue_type: BlockIssueType::QueueFull,
                        severity: 6,
                        description: format!(
                            "Device {} queue is {}% full",
                            device.name,
                            (queue.depth.utilization() * 100.0) as u32
                        ),
                        device: Some(device.id),
                    });
                    recommendations.push(BlockRecommendation {
                        action: BlockAction::IncreaseQueueDepth,
                        expected_improvement: 8.0,
                        reason: String::from("Increase queue depth to reduce contention"),
                    });
                }

                // Check latency
                let avg_latency_ms = queue.avg_read_latency() / 1_000_000;
                let high_latency = match device.device_type {
                    BlockDeviceType::Nvme => avg_latency_ms > 1,
                    BlockDeviceType::Ssd => avg_latency_ms > 5,
                    BlockDeviceType::Hdd => avg_latency_ms > 20,
                    _ => avg_latency_ms > 10,
                };

                if high_latency {
                    performance_score -= 15.0;
                    issues.push(BlockIssue {
                        issue_type: BlockIssueType::HighLatency,
                        severity: 7,
                        description: format!(
                            "Device {} has high latency: {}ms",
                            device.name, avg_latency_ms
                        ),
                        device: Some(device.id),
                    });
                }
            }
        }

        health_score = health_score.max(0.0);
        performance_score = performance_score.max(0.0);

        BlockAnalysis {
            health_score,
            performance_score,
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &BlockManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut BlockManager {
        &mut self.manager
    }
}

impl Default for BlockIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
