// SPDX-License-Identifier: GPL-2.0
//! Bridge acpi_bridge — ACPI subsystem interface bridge for power and device management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// ACPI device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiDeviceState {
    D0,
    D1,
    D2,
    D3Hot,
    D3Cold,
    Unknown,
}

impl AcpiDeviceState {
    pub fn power_level(&self) -> u8 {
        match self {
            Self::D0 => 0,
            Self::D1 => 1,
            Self::D2 => 2,
            Self::D3Hot => 3,
            Self::D3Cold => 4,
            Self::Unknown => 255,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::D0)
    }
}

/// ACPI table type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AcpiTableType {
    Dsdt,
    Ssdt,
    Fadt,
    Madt,
    Srat,
    Slit,
    Mcfg,
    Hpet,
    Bert,
    Bgrt,
    Custom,
}

/// ACPI table descriptor
#[derive(Debug, Clone)]
pub struct AcpiTable {
    pub table_type: AcpiTableType,
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub oem_id: String,
    pub address: u64,
}

impl AcpiTable {
    pub fn signature_str(&self) -> String {
        String::from_utf8_lossy(&self.signature).into_owned()
    }
}

/// ACPI device info
#[derive(Debug, Clone)]
pub struct AcpiDevice {
    pub hid: String,
    pub uid: String,
    pub path: String,
    pub state: AcpiDeviceState,
    pub status: u32,
    pub address: u64,
    pub wake_capable: bool,
    pub wake_enabled: bool,
}

impl AcpiDevice {
    pub fn is_present(&self) -> bool {
        self.status & 0x01 != 0
    }

    pub fn is_enabled(&self) -> bool {
        self.status & 0x02 != 0
    }

    pub fn is_functioning(&self) -> bool {
        self.status & 0x08 != 0
    }
}

/// System sleep state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepState {
    S0,
    S1,
    S2,
    S3,
    S4,
    S5,
}

impl SleepState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::S0 => "Working",
            Self::S1 => "Standby",
            Self::S2 => "Standby Low",
            Self::S3 => "Suspend to RAM",
            Self::S4 => "Hibernate",
            Self::S5 => "Soft Off",
        }
    }
}

/// ACPI event
#[derive(Debug, Clone)]
pub struct AcpiEvent {
    pub device: String,
    pub event_type: AcpiEventType,
    pub data: u32,
    pub timestamp: u64,
}

/// ACPI event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpiEventType {
    PowerButton,
    SleepButton,
    LidSwitch,
    AcAdapter,
    Battery,
    Thermal,
    DeviceNotify,
    BusCheck,
    DeviceCheck,
    Eject,
}

/// GPE (General Purpose Event) info
#[derive(Debug, Clone)]
pub struct GpeInfo {
    pub number: u32,
    pub dispatch_count: u64,
    pub handler_address: u64,
    pub enabled: bool,
    pub wake: bool,
}

/// ACPI bridge stats
#[derive(Debug, Clone)]
pub struct AcpiBridgeStats {
    pub table_count: u32,
    pub device_count: u32,
    pub gpe_count: u32,
    pub event_count: u64,
    pub sleep_transitions: u64,
    pub wake_events: u64,
}

/// Main ACPI bridge
pub struct BridgeAcpi {
    tables: Vec<AcpiTable>,
    devices: BTreeMap<String, AcpiDevice>,
    gpes: BTreeMap<u32, GpeInfo>,
    events: Vec<AcpiEvent>,
    max_events: usize,
    sleep_states_supported: Vec<SleepState>,
    current_sleep: SleepState,
    stats: AcpiBridgeStats,
}

impl BridgeAcpi {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            devices: BTreeMap::new(),
            gpes: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            sleep_states_supported: Vec::new(),
            current_sleep: SleepState::S0,
            stats: AcpiBridgeStats {
                table_count: 0, device_count: 0, gpe_count: 0,
                event_count: 0, sleep_transitions: 0, wake_events: 0,
            },
        }
    }

    pub fn register_table(&mut self, table: AcpiTable) {
        self.stats.table_count += 1;
        self.tables.push(table);
    }

    pub fn register_device(&mut self, dev: AcpiDevice) {
        self.stats.device_count += 1;
        self.devices.insert(dev.path.clone(), dev);
    }

    pub fn register_gpe(&mut self, gpe: GpeInfo) {
        self.stats.gpe_count += 1;
        self.gpes.insert(gpe.number, gpe);
    }

    pub fn set_device_state(&mut self, path: &str, state: AcpiDeviceState) {
        if let Some(dev) = self.devices.get_mut(path) {
            dev.state = state;
        }
    }

    pub fn record_event(&mut self, event: AcpiEvent) {
        self.stats.event_count += 1;
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    pub fn transition_sleep(&mut self, state: SleepState) {
        self.stats.sleep_transitions += 1;
        self.current_sleep = state;
    }

    pub fn wake_devices(&self) -> Vec<&AcpiDevice> {
        self.devices.values().filter(|d| d.wake_capable && d.wake_enabled).collect()
    }

    pub fn devices_in_state(&self, state: AcpiDeviceState) -> Vec<&AcpiDevice> {
        self.devices.values().filter(|d| d.state == state).collect()
    }

    pub fn find_table(&self, table_type: AcpiTableType) -> Option<&AcpiTable> {
        self.tables.iter().find(|t| t.table_type == table_type)
    }

    pub fn hottest_gpes(&self, n: usize) -> Vec<(u32, u64)> {
        let mut v: Vec<_> = self.gpes.iter().map(|(&num, g)| (num, g.dispatch_count)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> &AcpiBridgeStats {
        &self.stats
    }
}
