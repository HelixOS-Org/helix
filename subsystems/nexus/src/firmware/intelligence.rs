//! Firmware Intelligence
//!
//! Comprehensive firmware analysis and management.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    AcpiParseResult, AcpiParser, AcpiSignature, FirmwareType, FirmwareUpdateManager, SmbiosInfo,
    SmbiosParser, UefiRuntimeServices,
};

/// Firmware analysis result
#[derive(Debug, Clone)]
pub struct FirmwareAnalysis {
    /// Firmware type
    pub firmware_type: FirmwareType,
    /// Health score (0-100)
    pub health_score: f32,
    /// Issues detected
    pub issues: Vec<FirmwareIssue>,
    /// Recommendations
    pub recommendations: Vec<FirmwareRecommendation>,
    /// ACPI info
    pub acpi_info: Option<AcpiParseResult>,
    /// SMBIOS info
    pub smbios_info: Option<SmbiosInfo>,
}

/// Firmware issue
#[derive(Debug, Clone)]
pub struct FirmwareIssue {
    /// Issue type
    pub issue_type: FirmwareIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
}

/// Firmware issue types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareIssueType {
    /// Missing ACPI table
    MissingAcpiTable,
    /// Invalid ACPI checksum
    InvalidAcpiChecksum,
    /// Outdated firmware
    OutdatedFirmware,
    /// Missing runtime services
    MissingRuntimeServices,
    /// Invalid SMBIOS data
    InvalidSmbiosData,
    /// Security vulnerability
    SecurityVulnerability,
}

/// Firmware recommendation
#[derive(Debug, Clone)]
pub struct FirmwareRecommendation {
    /// Action
    pub action: FirmwareAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Firmware actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareAction {
    /// Update firmware
    UpdateFirmware,
    /// Enable feature
    EnableFeature,
    /// Disable feature
    DisableFeature,
    /// Configure setting
    ConfigureSetting,
}

/// Firmware Intelligence - comprehensive firmware analysis and management
pub struct FirmwareIntelligence {
    /// Detected firmware type
    firmware_type: FirmwareType,
    /// ACPI parser
    acpi_parser: AcpiParser,
    /// UEFI runtime services
    uefi_runtime: UefiRuntimeServices,
    /// SMBIOS parser
    smbios_parser: SmbiosParser,
    /// Update manager
    update_manager: FirmwareUpdateManager,
    /// Boot timestamp
    boot_timestamp: u64,
    /// Initialization complete
    initialized: bool,
}

impl FirmwareIntelligence {
    /// Create new firmware intelligence
    pub fn new() -> Self {
        Self {
            firmware_type: FirmwareType::Unknown,
            acpi_parser: AcpiParser::new(),
            uefi_runtime: UefiRuntimeServices::new(),
            smbios_parser: SmbiosParser::new(),
            update_manager: FirmwareUpdateManager::new(),
            boot_timestamp: 0,
            initialized: false,
        }
    }

    /// Initialize with firmware type
    pub fn initialize(&mut self, firmware_type: FirmwareType, boot_timestamp: u64) {
        self.firmware_type = firmware_type;
        self.boot_timestamp = boot_timestamp;

        if firmware_type == FirmwareType::Uefi {
            self.uefi_runtime.initialize();
        }

        self.initialized = true;
    }

    /// Get firmware type
    pub fn firmware_type(&self) -> FirmwareType {
        self.firmware_type
    }

    /// Get ACPI parser
    pub fn acpi_parser(&self) -> &AcpiParser {
        &self.acpi_parser
    }

    /// Get ACPI parser mutably
    pub fn acpi_parser_mut(&mut self) -> &mut AcpiParser {
        &mut self.acpi_parser
    }

    /// Get UEFI runtime services
    pub fn uefi_runtime(&self) -> &UefiRuntimeServices {
        &self.uefi_runtime
    }

    /// Get UEFI runtime services mutably
    pub fn uefi_runtime_mut(&mut self) -> &mut UefiRuntimeServices {
        &mut self.uefi_runtime
    }

    /// Get SMBIOS parser
    pub fn smbios_parser(&self) -> &SmbiosParser {
        &self.smbios_parser
    }

    /// Get SMBIOS parser mutably
    pub fn smbios_parser_mut(&mut self) -> &mut SmbiosParser {
        &mut self.smbios_parser
    }

    /// Get update manager
    pub fn update_manager(&self) -> &FirmwareUpdateManager {
        &self.update_manager
    }

    /// Get update manager mutably
    pub fn update_manager_mut(&mut self) -> &mut FirmwareUpdateManager {
        &mut self.update_manager
    }

    /// Analyze firmware
    pub fn analyze(&mut self) -> FirmwareAnalysis {
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Parse ACPI if available
        let acpi_info = if self.firmware_type.supports_acpi() {
            let result = self.acpi_parser.parse();

            // Check for missing critical tables
            if !self.acpi_parser.has_table(AcpiSignature::FADT) {
                health_score -= 20.0;
                issues.push(FirmwareIssue {
                    issue_type: FirmwareIssueType::MissingAcpiTable,
                    severity: 8,
                    description: String::from("Missing FADT table"),
                });
            }

            if !self.acpi_parser.has_table(AcpiSignature::MADT) {
                health_score -= 15.0;
                issues.push(FirmwareIssue {
                    issue_type: FirmwareIssueType::MissingAcpiTable,
                    severity: 7,
                    description: String::from("Missing MADT table"),
                });
            }

            // Check for invalid checksums
            for table in &result.tables {
                if !table.checksum_valid {
                    health_score -= 5.0;
                    issues.push(FirmwareIssue {
                        issue_type: FirmwareIssueType::InvalidAcpiChecksum,
                        severity: 4,
                        description: format!("Invalid checksum for {} table", table.signature.as_str()),
                    });
                }
            }

            Some(result)
        } else {
            None
        };

        // Parse SMBIOS
        let smbios_info = {
            let info = self.smbios_parser.parse().clone();

            // Check for missing critical data
            if info.bios_vendor.is_empty() {
                health_score -= 5.0;
                issues.push(FirmwareIssue {
                    issue_type: FirmwareIssueType::InvalidSmbiosData,
                    severity: 3,
                    description: String::from("Missing BIOS vendor information"),
                });
            }

            Some(info)
        };

        // Check UEFI runtime services
        if self.firmware_type == FirmwareType::Uefi && !self.uefi_runtime.is_available() {
            health_score -= 25.0;
            issues.push(FirmwareIssue {
                issue_type: FirmwareIssueType::MissingRuntimeServices,
                severity: 8,
                description: String::from("UEFI runtime services not available"),
            });
        }

        // Generate recommendations
        if !self.update_manager.pending_updates().is_empty() {
            recommendations.push(FirmwareRecommendation {
                action: FirmwareAction::UpdateFirmware,
                expected_improvement: 15.0,
                reason: String::from("Firmware updates available"),
            });
        }

        health_score = health_score.max(0.0);

        FirmwareAnalysis {
            firmware_type: self.firmware_type,
            health_score,
            issues,
            recommendations,
            acpi_info,
            smbios_info,
        }
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get boot timestamp
    pub fn boot_timestamp(&self) -> u64 {
        self.boot_timestamp
    }
}

impl Default for FirmwareIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
