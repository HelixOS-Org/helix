//! Advanced SMBIOS Table Parser
//!
//! Comprehensive SMBIOS parsing for system hardware enumeration.

use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr;

// =============================================================================
// SMBIOS PARSER
// =============================================================================

/// Advanced SMBIOS parser
pub struct SmbiosParser {
    /// Entry point version
    version: super::SmbiosVersion,
    /// Table address
    table_address: PhysicalAddress,
    /// Table length
    table_length: usize,
    /// Number of structures
    structure_count: usize,
    /// Parsed BIOS info
    bios_info: Option<BiosInformation>,
    /// Parsed system info
    system_info: Option<SystemInformation>,
    /// Parsed baseboard info
    baseboard_info: Option<BaseboardInformation>,
    /// Parsed chassis info
    chassis_info: Option<ChassisInformation>,
    /// Parsed processor info list
    processor_info: Vec<ProcessorInformation>,
    /// Parsed cache info list
    cache_info: Vec<CacheInformation>,
    /// Parsed memory device info list
    memory_devices: Vec<MemoryDeviceInformation>,
    /// Parsed memory array info list
    memory_arrays: Vec<PhysicalMemoryArrayInformation>,
    /// System slots
    system_slots: Vec<SystemSlotInformation>,
    /// Port connectors
    port_connectors: Vec<PortConnectorInformation>,
    /// On-board devices
    onboard_devices: Vec<OnboardDeviceInformation>,
    /// OEM strings
    oem_strings: Vec<String>,
    /// System configuration options
    config_options: Vec<String>,
    /// Boot status
    boot_info: Option<SystemBootInformation>,
}

impl SmbiosParser {
    /// Create new SMBIOS parser
    pub fn new() -> Self {
        Self {
            version: super::SmbiosVersion::new(0, 0),
            table_address: PhysicalAddress(0),
            table_length: 0,
            structure_count: 0,
            bios_info: None,
            system_info: None,
            baseboard_info: None,
            chassis_info: None,
            processor_info: Vec::new(),
            cache_info: Vec::new(),
            memory_devices: Vec::new(),
            memory_arrays: Vec::new(),
            system_slots: Vec::new(),
            port_connectors: Vec::new(),
            onboard_devices: Vec::new(),
            oem_strings: Vec::new(),
            config_options: Vec::new(),
            boot_info: None,
        }
    }

    /// Initialize from SMBIOS 2.x entry point
    pub unsafe fn init_from_entry_point(&mut self, entry: PhysicalAddress) -> Result<()> {
        let ep = &*(entry.0 as *const SmbiosEntryPoint);

        // Validate anchor string
        if &ep.anchor_string != b"_SM_" {
            return Err(Error::InvalidParameter);
        }

        // Validate intermediate anchor
        if &ep.intermediate_anchor != b"_DMI_" {
            return Err(Error::InvalidParameter);
        }

        self.version = super::SmbiosVersion::new(ep.major_version, ep.minor_version);
        self.table_address = PhysicalAddress(ep.structure_table_address as u64);
        self.table_length = ep.structure_table_length as usize;
        self.structure_count = ep.number_of_structures as usize;

        self.parse_all()?;
        Ok(())
    }

    /// Initialize from SMBIOS 3.x entry point
    pub unsafe fn init_from_entry_point3(&mut self, entry: PhysicalAddress) -> Result<()> {
        let ep = &*(entry.0 as *const SmbiosEntryPoint3);

        // Validate anchor string
        if &ep.anchor_string != b"_SM3_" {
            return Err(Error::InvalidParameter);
        }

        self.version = super::SmbiosVersion::new(ep.major_version, ep.minor_version);
        self.table_address = PhysicalAddress(ep.structure_table_address);
        self.table_length = ep.structure_table_max_size as usize;
        self.structure_count = 0; // Unknown for SMBIOS 3.x

        self.parse_all()?;
        Ok(())
    }

    /// Parse all structures
    unsafe fn parse_all(&mut self) -> Result<()> {
        let mut ptr = self.table_address.0 as *const u8;
        let end = ptr.add(self.table_length);

        while ptr < end {
            let header = &*(ptr as *const SmbiosHeader);

            // End of table marker
            if header.structure_type == 127 {
                break;
            }

            // Parse based on type
            match header.structure_type {
                0 => self.parse_bios_info(ptr)?,
                1 => self.parse_system_info(ptr)?,
                2 => self.parse_baseboard_info(ptr)?,
                3 => self.parse_chassis_info(ptr)?,
                4 => self.parse_processor_info(ptr)?,
                7 => self.parse_cache_info(ptr)?,
                8 => self.parse_port_connector(ptr)?,
                9 => self.parse_system_slot(ptr)?,
                10 => self.parse_onboard_device(ptr)?,
                11 => self.parse_oem_strings(ptr)?,
                12 => self.parse_config_options(ptr)?,
                16 => self.parse_memory_array(ptr)?,
                17 => self.parse_memory_device(ptr)?,
                32 => self.parse_boot_info(ptr)?,
                _ => {},
            }

            // Move to next structure
            ptr = self.find_next_structure(ptr, header.length)?;
        }

        Ok(())
    }

    /// Find next structure (skip strings section)
    unsafe fn find_next_structure(&self, ptr: *const u8, header_len: u8) -> Result<*const u8> {
        let mut string_ptr = ptr.add(header_len as usize);

        // Strings are null-terminated, section ends with double null
        loop {
            if *string_ptr == 0 {
                string_ptr = string_ptr.add(1);
                if *string_ptr == 0 {
                    return Ok(string_ptr.add(1));
                }
            } else {
                string_ptr = string_ptr.add(1);
            }

            // Safety check
            if string_ptr >= ptr.add(self.table_length) {
                return Err(Error::BufferTooSmall);
            }
        }
    }

    /// Get string by index from structure
    unsafe fn get_string(&self, ptr: *const u8, header_len: u8, index: u8) -> String {
        if index == 0 {
            return String::new();
        }

        let mut string_ptr = ptr.add(header_len as usize);
        let mut current_index = 1u8;

        while current_index < index {
            while *string_ptr != 0 {
                string_ptr = string_ptr.add(1);
            }
            string_ptr = string_ptr.add(1);
            if *string_ptr == 0 {
                return String::new();
            }
            current_index += 1;
        }

        // Read string
        let mut len = 0;
        while *string_ptr.add(len) != 0 {
            len += 1;
        }

        let slice = core::slice::from_raw_parts(string_ptr, len);
        String::from_utf8_lossy(slice).into_owned()
    }

    /// Parse BIOS information
    unsafe fn parse_bios_info(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let vendor_index = *data;
        let version_index = *data.add(1);
        let starting_segment = ptr::read_unaligned(data.add(2) as *const u16);
        let release_date_index = *data.add(4);
        let rom_size = *data.add(5);
        let characteristics = ptr::read_unaligned(data.add(6) as *const u64);

        let mut ext_characteristics = Vec::new();
        if header.length > 18 {
            for i in 0..(header.length - 18) {
                ext_characteristics.push(*data.add(14 + i as usize));
            }
        }

        let bios_version = if header.length >= 22 {
            Some(BiosVersion {
                major: *data.add(18),
                minor: *data.add(19),
            })
        } else {
            None
        };

        let ec_version = if header.length >= 24 {
            Some(EcVersion {
                major: *data.add(20),
                minor: *data.add(21),
            })
        } else {
            None
        };

        let extended_rom_size = if header.length >= 26 {
            Some(ptr::read_unaligned(data.add(22) as *const u16))
        } else {
            None
        };

        self.bios_info = Some(BiosInformation {
            vendor: self.get_string(ptr, header.length, vendor_index),
            version: self.get_string(ptr, header.length, version_index),
            starting_address_segment: starting_segment,
            release_date: self.get_string(ptr, header.length, release_date_index),
            rom_size,
            characteristics,
            characteristics_extension: ext_characteristics,
            bios_version,
            ec_version,
            extended_rom_size,
        });

        Ok(())
    }

    /// Parse system information
    unsafe fn parse_system_info(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let manufacturer_index = *data;
        let product_index = *data.add(1);
        let version_index = *data.add(2);
        let serial_index = *data.add(3);

        let uuid = if header.length >= 24 {
            let uuid_bytes = core::slice::from_raw_parts(data.add(4), 16);
            let mut uuid = [0u8; 16];
            uuid.copy_from_slice(uuid_bytes);
            Some(uuid)
        } else {
            None
        };

        let wake_up_type = if header.length >= 25 {
            Some(WakeUpType::from(*data.add(20)))
        } else {
            None
        };

        let sku_index = if header.length >= 26 {
            *data.add(21)
        } else {
            0
        };
        let family_index = if header.length >= 27 {
            *data.add(22)
        } else {
            0
        };

        self.system_info = Some(SystemInformation {
            manufacturer: self.get_string(ptr, header.length, manufacturer_index),
            product_name: self.get_string(ptr, header.length, product_index),
            version: self.get_string(ptr, header.length, version_index),
            serial_number: self.get_string(ptr, header.length, serial_index),
            uuid,
            wake_up_type,
            sku_number: self.get_string(ptr, header.length, sku_index),
            family: self.get_string(ptr, header.length, family_index),
        });

        Ok(())
    }

    /// Parse baseboard information
    unsafe fn parse_baseboard_info(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let manufacturer_index = *data;
        let product_index = *data.add(1);
        let version_index = *data.add(2);
        let serial_index = *data.add(3);
        let asset_tag_index = if header.length >= 9 { *data.add(4) } else { 0 };
        let feature_flags = if header.length >= 10 { *data.add(5) } else { 0 };
        let location_index = if header.length >= 11 { *data.add(6) } else { 0 };
        let chassis_handle = if header.length >= 13 {
            ptr::read_unaligned(data.add(7) as *const u16)
        } else {
            0
        };
        let board_type = if header.length >= 14 {
            BoardType::from(*data.add(9))
        } else {
            BoardType::Unknown
        };

        self.baseboard_info = Some(BaseboardInformation {
            manufacturer: self.get_string(ptr, header.length, manufacturer_index),
            product: self.get_string(ptr, header.length, product_index),
            version: self.get_string(ptr, header.length, version_index),
            serial_number: self.get_string(ptr, header.length, serial_index),
            asset_tag: self.get_string(ptr, header.length, asset_tag_index),
            feature_flags,
            location_in_chassis: self.get_string(ptr, header.length, location_index),
            chassis_handle,
            board_type,
        });

        Ok(())
    }

    /// Parse chassis information
    unsafe fn parse_chassis_info(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let manufacturer_index = *data;
        let chassis_type = *data.add(1) & 0x7F;
        let version_index = *data.add(2);
        let serial_index = *data.add(3);
        let asset_tag_index = if header.length >= 9 { *data.add(4) } else { 0 };

        let boot_up_state = if header.length >= 10 {
            ChassisState::from(*data.add(5))
        } else {
            ChassisState::Unknown
        };

        let power_supply_state = if header.length >= 11 {
            ChassisState::from(*data.add(6))
        } else {
            ChassisState::Unknown
        };

        let thermal_state = if header.length >= 12 {
            ChassisState::from(*data.add(7))
        } else {
            ChassisState::Unknown
        };

        let security_status = if header.length >= 13 {
            SecurityStatus::from(*data.add(8))
        } else {
            SecurityStatus::Unknown
        };

        let oem_defined = if header.length >= 17 {
            ptr::read_unaligned(data.add(9) as *const u32)
        } else {
            0
        };

        let height = if header.length >= 18 {
            *data.add(13)
        } else {
            0
        };
        let power_cords = if header.length >= 19 {
            *data.add(14)
        } else {
            0
        };

        let sku_index = if header.length >= 22 {
            // After contained elements
            let n = *data.add(15) as usize;
            let m = *data.add(16) as usize;
            let offset = 17 + n * m;
            if header.length as usize > offset {
                *data.add(offset)
            } else {
                0
            }
        } else {
            0
        };

        self.chassis_info = Some(ChassisInformation {
            manufacturer: self.get_string(ptr, header.length, manufacturer_index),
            chassis_type: ChassisType::from(chassis_type),
            version: self.get_string(ptr, header.length, version_index),
            serial_number: self.get_string(ptr, header.length, serial_index),
            asset_tag: self.get_string(ptr, header.length, asset_tag_index),
            boot_up_state,
            power_supply_state,
            thermal_state,
            security_status,
            oem_defined,
            height,
            number_of_power_cords: power_cords,
            sku_number: self.get_string(ptr, header.length, sku_index),
        });

        Ok(())
    }

    /// Parse processor information
    unsafe fn parse_processor_info(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let socket_index = *data;
        let processor_type = ProcessorType::from(*data.add(1));
        let processor_family = *data.add(2);
        let manufacturer_index = *data.add(3);
        let processor_id = ptr::read_unaligned(data.add(4) as *const u64);
        let version_index = *data.add(12);
        let voltage = *data.add(13);
        let external_clock = ptr::read_unaligned(data.add(14) as *const u16);
        let max_speed = ptr::read_unaligned(data.add(16) as *const u16);
        let current_speed = ptr::read_unaligned(data.add(18) as *const u16);
        let status = *data.add(20);
        let processor_upgrade = ProcessorUpgrade::from(*data.add(21));

        let l1_cache_handle = if header.length >= 28 {
            ptr::read_unaligned(data.add(22) as *const u16)
        } else {
            0xFFFF
        };
        let l2_cache_handle = if header.length >= 30 {
            ptr::read_unaligned(data.add(24) as *const u16)
        } else {
            0xFFFF
        };
        let l3_cache_handle = if header.length >= 32 {
            ptr::read_unaligned(data.add(26) as *const u16)
        } else {
            0xFFFF
        };

        let serial_index = if header.length >= 33 {
            *data.add(28)
        } else {
            0
        };
        let asset_tag_index = if header.length >= 34 {
            *data.add(29)
        } else {
            0
        };
        let part_number_index = if header.length >= 35 {
            *data.add(30)
        } else {
            0
        };

        let core_count = if header.length >= 36 {
            *data.add(31)
        } else {
            0
        };
        let core_enabled = if header.length >= 37 {
            *data.add(32)
        } else {
            0
        };
        let thread_count = if header.length >= 38 {
            *data.add(33)
        } else {
            0
        };

        let characteristics = if header.length >= 40 {
            ptr::read_unaligned(data.add(34) as *const u16)
        } else {
            0
        };

        let processor_family2 = if header.length >= 42 {
            ptr::read_unaligned(data.add(36) as *const u16)
        } else {
            processor_family as u16
        };

        let core_count2 = if header.length >= 44 {
            ptr::read_unaligned(data.add(38) as *const u16)
        } else {
            core_count as u16
        };
        let core_enabled2 = if header.length >= 46 {
            ptr::read_unaligned(data.add(40) as *const u16)
        } else {
            core_enabled as u16
        };
        let thread_count2 = if header.length >= 48 {
            ptr::read_unaligned(data.add(42) as *const u16)
        } else {
            thread_count as u16
        };

        self.processor_info.push(ProcessorInformation {
            socket_designation: self.get_string(ptr, header.length, socket_index),
            processor_type,
            processor_family: processor_family2,
            manufacturer: self.get_string(ptr, header.length, manufacturer_index),
            processor_id,
            version: self.get_string(ptr, header.length, version_index),
            voltage,
            external_clock,
            max_speed,
            current_speed,
            status,
            processor_upgrade,
            l1_cache_handle,
            l2_cache_handle,
            l3_cache_handle,
            serial_number: self.get_string(ptr, header.length, serial_index),
            asset_tag: self.get_string(ptr, header.length, asset_tag_index),
            part_number: self.get_string(ptr, header.length, part_number_index),
            core_count: core_count2,
            core_enabled: core_enabled2,
            thread_count: thread_count2,
            characteristics,
        });

        Ok(())
    }

    /// Parse cache information
    unsafe fn parse_cache_info(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let socket_index = *data;
        let cache_config = ptr::read_unaligned(data.add(1) as *const u16);
        let max_size = ptr::read_unaligned(data.add(3) as *const u16);
        let installed_size = ptr::read_unaligned(data.add(5) as *const u16);
        let supported_sram_type = ptr::read_unaligned(data.add(7) as *const u16);
        let current_sram_type = ptr::read_unaligned(data.add(9) as *const u16);

        let cache_speed = if header.length >= 16 {
            *data.add(11)
        } else {
            0
        };
        let error_correction_type = if header.length >= 17 {
            CacheErrorCorrection::from(*data.add(12))
        } else {
            CacheErrorCorrection::Unknown
        };
        let system_cache_type = if header.length >= 18 {
            CacheType::from(*data.add(13))
        } else {
            CacheType::Unknown
        };
        let associativity = if header.length >= 19 {
            CacheAssociativity::from(*data.add(14))
        } else {
            CacheAssociativity::Unknown
        };

        let max_size2 = if header.length >= 23 {
            ptr::read_unaligned(data.add(15) as *const u32)
        } else {
            max_size as u32
        };
        let installed_size2 = if header.length >= 27 {
            ptr::read_unaligned(data.add(19) as *const u32)
        } else {
            installed_size as u32
        };

        let level = ((cache_config & 0x07) + 1) as u8;
        let enabled = (cache_config & 0x80) != 0;
        let location = CacheLocation::from(((cache_config >> 5) & 0x03) as u8);
        let mode = CacheOperationalMode::from(((cache_config >> 8) & 0x03) as u8);

        self.cache_info.push(CacheInformation {
            socket_designation: self.get_string(ptr, header.length, socket_index),
            level,
            enabled,
            location,
            mode,
            max_size_kb: max_size2,
            installed_size_kb: installed_size2,
            supported_sram_type,
            current_sram_type,
            speed_ns: cache_speed,
            error_correction_type,
            system_cache_type,
            associativity,
        });

        Ok(())
    }

    /// Parse port connector information
    unsafe fn parse_port_connector(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let internal_ref_index = *data;
        let internal_connector_type = PortConnectorType::from(*data.add(1));
        let external_ref_index = *data.add(2);
        let external_connector_type = PortConnectorType::from(*data.add(3));
        let port_type = PortType::from(*data.add(4));

        self.port_connectors.push(PortConnectorInformation {
            internal_reference_designator: self.get_string(ptr, header.length, internal_ref_index),
            internal_connector_type,
            external_reference_designator: self.get_string(ptr, header.length, external_ref_index),
            external_connector_type,
            port_type,
        });

        Ok(())
    }

    /// Parse system slot information
    unsafe fn parse_system_slot(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let designation_index = *data;
        let slot_type = SlotType::from(*data.add(1));
        let slot_data_bus_width = SlotDataBusWidth::from(*data.add(2));
        let current_usage = SlotUsage::from(*data.add(3));
        let slot_length = SlotLength::from(*data.add(4));
        let slot_id = ptr::read_unaligned(data.add(5) as *const u16);
        let characteristics1 = *data.add(7);
        let characteristics2 = if header.length >= 13 { *data.add(8) } else { 0 };

        let segment_group = if header.length >= 15 {
            ptr::read_unaligned(data.add(9) as *const u16)
        } else {
            0
        };
        let bus = if header.length >= 16 {
            *data.add(11)
        } else {
            0
        };
        let device_function = if header.length >= 17 {
            *data.add(12)
        } else {
            0
        };

        self.system_slots.push(SystemSlotInformation {
            slot_designation: self.get_string(ptr, header.length, designation_index),
            slot_type,
            slot_data_bus_width,
            current_usage,
            slot_length,
            slot_id,
            characteristics1,
            characteristics2,
            segment_group_number: segment_group,
            bus_number: bus,
            device_function_number: device_function,
        });

        Ok(())
    }

    /// Parse on-board device information
    unsafe fn parse_onboard_device(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let device_count = (header.length - 4) / 2;

        for i in 0..device_count {
            let type_byte = *data.add((i * 2) as usize);
            let description_index = *data.add((i * 2 + 1) as usize);

            let enabled = (type_byte & 0x80) != 0;
            let device_type = OnboardDeviceType::from(type_byte & 0x7F);

            self.onboard_devices.push(OnboardDeviceInformation {
                description: self.get_string(ptr, header.length, description_index),
                device_type,
                enabled,
            });
        }

        Ok(())
    }

    /// Parse OEM strings
    unsafe fn parse_oem_strings(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let count = *data;

        for i in 1..=count {
            let s = self.get_string(ptr, header.length, i);
            if !s.is_empty() {
                self.oem_strings.push(s);
            }
        }

        Ok(())
    }

    /// Parse system configuration options
    unsafe fn parse_config_options(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let count = *data;

        for i in 1..=count {
            let s = self.get_string(ptr, header.length, i);
            if !s.is_empty() {
                self.config_options.push(s);
            }
        }

        Ok(())
    }

    /// Parse physical memory array
    unsafe fn parse_memory_array(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let location = MemoryArrayLocation::from(*data);
        let use_type = MemoryArrayUse::from(*data.add(1));
        let error_correction = MemoryErrorCorrection::from(*data.add(2));
        let maximum_capacity = ptr::read_unaligned(data.add(3) as *const u32);
        let error_handle = ptr::read_unaligned(data.add(7) as *const u16);
        let number_of_devices = ptr::read_unaligned(data.add(9) as *const u16);

        let extended_capacity = if header.length >= 19 {
            ptr::read_unaligned(data.add(11) as *const u64)
        } else if maximum_capacity == 0x8000_0000 {
            0 // Should use extended field
        } else {
            maximum_capacity as u64 * 1024 // Convert to bytes
        };

        self.memory_arrays.push(PhysicalMemoryArrayInformation {
            location,
            use_type,
            error_correction,
            maximum_capacity_kb: if maximum_capacity == 0x8000_0000 {
                extended_capacity
            } else {
                maximum_capacity as u64
            },
            error_information_handle: error_handle,
            number_of_memory_devices: number_of_devices,
        });

        Ok(())
    }

    /// Parse memory device
    unsafe fn parse_memory_device(&mut self, ptr: *const u8) -> Result<()> {
        let header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        let array_handle = ptr::read_unaligned(data as *const u16);
        let error_handle = ptr::read_unaligned(data.add(2) as *const u16);
        let total_width = ptr::read_unaligned(data.add(4) as *const u16);
        let data_width = ptr::read_unaligned(data.add(6) as *const u16);
        let size = ptr::read_unaligned(data.add(8) as *const u16);
        let form_factor = MemoryFormFactor::from(*data.add(10));
        let device_set = *data.add(11);
        let device_locator_index = *data.add(12);
        let bank_locator_index = *data.add(13);
        let memory_type = MemoryType::from(*data.add(14));
        let type_detail = ptr::read_unaligned(data.add(15) as *const u16);

        let speed = if header.length >= 21 {
            ptr::read_unaligned(data.add(17) as *const u16)
        } else {
            0
        };

        let manufacturer_index = if header.length >= 24 {
            *data.add(19)
        } else {
            0
        };
        let serial_index = if header.length >= 25 {
            *data.add(20)
        } else {
            0
        };
        let asset_tag_index = if header.length >= 26 {
            *data.add(21)
        } else {
            0
        };
        let part_number_index = if header.length >= 27 {
            *data.add(22)
        } else {
            0
        };

        let attributes = if header.length >= 28 {
            *data.add(23)
        } else {
            0
        };

        let extended_size = if header.length >= 32 {
            ptr::read_unaligned(data.add(24) as *const u32)
        } else {
            0
        };

        let configured_speed = if header.length >= 36 {
            ptr::read_unaligned(data.add(28) as *const u16)
        } else {
            0
        };

        let minimum_voltage = if header.length >= 40 {
            ptr::read_unaligned(data.add(30) as *const u16)
        } else {
            0
        };
        let maximum_voltage = if header.length >= 42 {
            ptr::read_unaligned(data.add(32) as *const u16)
        } else {
            0
        };
        let configured_voltage = if header.length >= 44 {
            ptr::read_unaligned(data.add(34) as *const u16)
        } else {
            0
        };

        let memory_technology = if header.length >= 50 {
            MemoryTechnology::from(*data.add(46))
        } else {
            MemoryTechnology::Unknown
        };

        let memory_operating_mode_capability = if header.length >= 52 {
            ptr::read_unaligned(data.add(47) as *const u16)
        } else {
            0
        };

        // Calculate size in MB
        let size_mb = if size == 0xFFFF {
            0 // Unknown
        } else if size == 0x7FFF {
            extended_size as u64 // Extended size in MB
        } else if (size & 0x8000) != 0 {
            (size & 0x7FFF) as u64 / 1024 // Size in KB, convert to MB
        } else {
            size as u64 // Size in MB
        };

        self.memory_devices.push(MemoryDeviceInformation {
            physical_memory_array_handle: array_handle,
            memory_error_information_handle: error_handle,
            total_width,
            data_width,
            size_mb,
            form_factor,
            device_set,
            device_locator: self.get_string(ptr, header.length, device_locator_index),
            bank_locator: self.get_string(ptr, header.length, bank_locator_index),
            memory_type,
            type_detail,
            speed_mhz: speed,
            manufacturer: self.get_string(ptr, header.length, manufacturer_index),
            serial_number: self.get_string(ptr, header.length, serial_index),
            asset_tag: self.get_string(ptr, header.length, asset_tag_index),
            part_number: self.get_string(ptr, header.length, part_number_index),
            rank: attributes & 0x0F,
            configured_memory_speed_mhz: configured_speed,
            minimum_voltage_mv: minimum_voltage,
            maximum_voltage_mv: maximum_voltage,
            configured_voltage_mv: configured_voltage,
            memory_technology,
            memory_operating_mode_capability,
        });

        Ok(())
    }

    /// Parse system boot information
    unsafe fn parse_boot_info(&mut self, ptr: *const u8) -> Result<()> {
        let _header = &*(ptr as *const SmbiosHeader);
        let data = ptr.add(4);

        // Skip reserved bytes
        let status = BootStatus::from(*data.add(6));

        self.boot_info = Some(SystemBootInformation { status });

        Ok(())
    }

    // Accessor methods
    pub fn version(&self) -> super::SmbiosVersion {
        self.version
    }
    pub fn bios_info(&self) -> Option<&BiosInformation> {
        self.bios_info.as_ref()
    }
    pub fn system_info(&self) -> Option<&SystemInformation> {
        self.system_info.as_ref()
    }
    pub fn baseboard_info(&self) -> Option<&BaseboardInformation> {
        self.baseboard_info.as_ref()
    }
    pub fn chassis_info(&self) -> Option<&ChassisInformation> {
        self.chassis_info.as_ref()
    }
    pub fn processor_info(&self) -> &[ProcessorInformation] {
        &self.processor_info
    }
    pub fn cache_info(&self) -> &[CacheInformation] {
        &self.cache_info
    }
    pub fn memory_devices(&self) -> &[MemoryDeviceInformation] {
        &self.memory_devices
    }
    pub fn memory_arrays(&self) -> &[PhysicalMemoryArrayInformation] {
        &self.memory_arrays
    }
    pub fn system_slots(&self) -> &[SystemSlotInformation] {
        &self.system_slots
    }
    pub fn port_connectors(&self) -> &[PortConnectorInformation] {
        &self.port_connectors
    }
    pub fn onboard_devices(&self) -> &[OnboardDeviceInformation] {
        &self.onboard_devices
    }
    pub fn oem_strings(&self) -> &[String] {
        &self.oem_strings
    }
    pub fn config_options(&self) -> &[String] {
        &self.config_options
    }
    pub fn boot_info(&self) -> Option<&SystemBootInformation> {
        self.boot_info.as_ref()
    }

    /// Get total installed memory in MB
    pub fn total_memory_mb(&self) -> u64 {
        self.memory_devices.iter().map(|d| d.size_mb).sum()
    }

    /// Get total CPU count
    pub fn cpu_count(&self) -> usize {
        self.processor_info.len()
    }

    /// Get total core count
    pub fn total_cores(&self) -> u32 {
        self.processor_info
            .iter()
            .map(|p| p.core_count as u32)
            .sum()
    }

    /// Get total thread count
    pub fn total_threads(&self) -> u32 {
        self.processor_info
            .iter()
            .map(|p| p.thread_count as u32)
            .sum()
    }
}

impl Default for SmbiosParser {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// RAW STRUCTURES
// =============================================================================

/// SMBIOS Entry Point (2.x)
#[repr(C, packed)]
struct SmbiosEntryPoint {
    anchor_string: [u8; 4], // "_SM_"
    entry_point_checksum: u8,
    entry_point_length: u8,
    major_version: u8,
    minor_version: u8,
    max_structure_size: u16,
    entry_point_revision: u8,
    formatted_area: [u8; 5],
    intermediate_anchor: [u8; 5], // "_DMI_"
    intermediate_checksum: u8,
    structure_table_length: u16,
    structure_table_address: u32,
    number_of_structures: u16,
    bcd_revision: u8,
}

/// SMBIOS Entry Point (3.x)
#[repr(C, packed)]
struct SmbiosEntryPoint3 {
    anchor_string: [u8; 5], // "_SM3_"
    entry_point_checksum: u8,
    entry_point_length: u8,
    major_version: u8,
    minor_version: u8,
    docrev: u8,
    entry_point_revision: u8,
    reserved: u8,
    structure_table_max_size: u32,
    structure_table_address: u64,
}

/// SMBIOS Structure Header
#[repr(C, packed)]
struct SmbiosHeader {
    structure_type: u8,
    length: u8,
    handle: u16,
}

// =============================================================================
// PARSED STRUCTURES
// =============================================================================

/// BIOS Version
#[derive(Debug, Clone, Copy)]
pub struct BiosVersion {
    pub major: u8,
    pub minor: u8,
}

/// EC Version
#[derive(Debug, Clone, Copy)]
pub struct EcVersion {
    pub major: u8,
    pub minor: u8,
}

/// BIOS Information
#[derive(Debug, Clone)]
pub struct BiosInformation {
    pub vendor: String,
    pub version: String,
    pub starting_address_segment: u16,
    pub release_date: String,
    pub rom_size: u8,
    pub characteristics: u64,
    pub characteristics_extension: Vec<u8>,
    pub bios_version: Option<BiosVersion>,
    pub ec_version: Option<EcVersion>,
    pub extended_rom_size: Option<u16>,
}

impl BiosInformation {
    /// Get ROM size in KB
    pub fn rom_size_kb(&self) -> usize {
        if let Some(ext) = self.extended_rom_size {
            let unit = (ext >> 14) & 0x03;
            let size = (ext & 0x3FFF) as usize;
            match unit {
                0 => size * 1024 * 1024,        // MB to KB
                1 => size * 1024 * 1024 * 1024, // GB to KB
                _ => (self.rom_size as usize + 1) * 64,
            }
        } else {
            (self.rom_size as usize + 1) * 64
        }
    }

    pub fn supports_isa(&self) -> bool {
        (self.characteristics & (1 << 4)) != 0
    }
    pub fn supports_pci(&self) -> bool {
        (self.characteristics & (1 << 7)) != 0
    }
    pub fn supports_pnp(&self) -> bool {
        (self.characteristics & (1 << 9)) != 0
    }
    pub fn supports_apm(&self) -> bool {
        (self.characteristics & (1 << 10)) != 0
    }
    pub fn upgradeable(&self) -> bool {
        (self.characteristics & (1 << 11)) != 0
    }
    pub fn shadowing(&self) -> bool {
        (self.characteristics & (1 << 12)) != 0
    }
    pub fn escd_support(&self) -> bool {
        (self.characteristics & (1 << 14)) != 0
    }
    pub fn boot_from_cd(&self) -> bool {
        (self.characteristics & (1 << 15)) != 0
    }
    pub fn selectable_boot(&self) -> bool {
        (self.characteristics & (1 << 16)) != 0
    }
    pub fn uefi(&self) -> bool {
        !self.characteristics_extension.is_empty()
            && (self.characteristics_extension[0] & (1 << 3)) != 0
    }
}

/// System Information
#[derive(Debug, Clone)]
pub struct SystemInformation {
    pub manufacturer: String,
    pub product_name: String,
    pub version: String,
    pub serial_number: String,
    pub uuid: Option<[u8; 16]>,
    pub wake_up_type: Option<WakeUpType>,
    pub sku_number: String,
    pub family: String,
}

impl SystemInformation {
    /// Get UUID as string
    pub fn uuid_string(&self) -> Option<String> {
        self.uuid.map(|u| {
            alloc::format!(
                "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                u[3], u[2], u[1], u[0], u[5], u[4], u[7], u[6],
                u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
            )
        })
    }
}

/// Wake-up type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeUpType {
    Reserved,
    Other,
    Unknown,
    ApmTimer,
    ModemRing,
    LanRemote,
    PowerSwitch,
    PciPme,
    AcPowerRestored,
}

impl From<u8> for WakeUpType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::ApmTimer,
            0x04 => Self::ModemRing,
            0x05 => Self::LanRemote,
            0x06 => Self::PowerSwitch,
            0x07 => Self::PciPme,
            0x08 => Self::AcPowerRestored,
            _ => Self::Reserved,
        }
    }
}

/// Baseboard Information
#[derive(Debug, Clone)]
pub struct BaseboardInformation {
    pub manufacturer: String,
    pub product: String,
    pub version: String,
    pub serial_number: String,
    pub asset_tag: String,
    pub feature_flags: u8,
    pub location_in_chassis: String,
    pub chassis_handle: u16,
    pub board_type: BoardType,
}

impl BaseboardInformation {
    pub fn is_hosting_board(&self) -> bool {
        (self.feature_flags & 0x01) != 0
    }
    pub fn requires_daughter_board(&self) -> bool {
        (self.feature_flags & 0x02) != 0
    }
    pub fn is_removable(&self) -> bool {
        (self.feature_flags & 0x04) != 0
    }
    pub fn is_replaceable(&self) -> bool {
        (self.feature_flags & 0x08) != 0
    }
    pub fn is_hot_swappable(&self) -> bool {
        (self.feature_flags & 0x10) != 0
    }
}

/// Board type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardType {
    Unknown,
    Other,
    ServerBlade,
    ConnectivitySwitch,
    SystemManagement,
    ProcessorModule,
    IOModule,
    MemoryModule,
    DaughterBoard,
    Motherboard,
    ProcessorMemoryModule,
    ProcessorIOModule,
    InterconnectBoard,
}

impl From<u8> for BoardType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Unknown,
            0x02 => Self::Other,
            0x03 => Self::ServerBlade,
            0x04 => Self::ConnectivitySwitch,
            0x05 => Self::SystemManagement,
            0x06 => Self::ProcessorModule,
            0x07 => Self::IOModule,
            0x08 => Self::MemoryModule,
            0x09 => Self::DaughterBoard,
            0x0A => Self::Motherboard,
            0x0B => Self::ProcessorMemoryModule,
            0x0C => Self::ProcessorIOModule,
            0x0D => Self::InterconnectBoard,
            _ => Self::Unknown,
        }
    }
}

/// Chassis Information
#[derive(Debug, Clone)]
pub struct ChassisInformation {
    pub manufacturer: String,
    pub chassis_type: ChassisType,
    pub version: String,
    pub serial_number: String,
    pub asset_tag: String,
    pub boot_up_state: ChassisState,
    pub power_supply_state: ChassisState,
    pub thermal_state: ChassisState,
    pub security_status: SecurityStatus,
    pub oem_defined: u32,
    pub height: u8,
    pub number_of_power_cords: u8,
    pub sku_number: String,
}

/// Chassis type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisType {
    Other,
    Unknown,
    Desktop,
    LowProfileDesktop,
    PizzaBox,
    MiniTower,
    Tower,
    Portable,
    Laptop,
    Notebook,
    HandHeld,
    DockingStation,
    AllInOne,
    SubNotebook,
    SpaceSaving,
    LunchBox,
    MainServerChassis,
    ExpansionChassis,
    SubChassis,
    BusExpansionChassis,
    PeripheralChassis,
    RAIDChassis,
    RackMountChassis,
    SealedCasePC,
    MultiSystemChassis,
    CompactPCI,
    AdvancedTCA,
    Blade,
    BladeEnclosure,
    Tablet,
    Convertible,
    Detachable,
    IoTGateway,
    EmbeddedPC,
    MiniPC,
    StickPC,
}

impl From<u8> for ChassisType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Desktop,
            0x04 => Self::LowProfileDesktop,
            0x05 => Self::PizzaBox,
            0x06 => Self::MiniTower,
            0x07 => Self::Tower,
            0x08 => Self::Portable,
            0x09 => Self::Laptop,
            0x0A => Self::Notebook,
            0x0B => Self::HandHeld,
            0x0C => Self::DockingStation,
            0x0D => Self::AllInOne,
            0x0E => Self::SubNotebook,
            0x0F => Self::SpaceSaving,
            0x10 => Self::LunchBox,
            0x11 => Self::MainServerChassis,
            0x12 => Self::ExpansionChassis,
            0x13 => Self::SubChassis,
            0x14 => Self::BusExpansionChassis,
            0x15 => Self::PeripheralChassis,
            0x16 => Self::RAIDChassis,
            0x17 => Self::RackMountChassis,
            0x18 => Self::SealedCasePC,
            0x19 => Self::MultiSystemChassis,
            0x1A => Self::CompactPCI,
            0x1B => Self::AdvancedTCA,
            0x1C => Self::Blade,
            0x1D => Self::BladeEnclosure,
            0x1E => Self::Tablet,
            0x1F => Self::Convertible,
            0x20 => Self::Detachable,
            0x21 => Self::IoTGateway,
            0x22 => Self::EmbeddedPC,
            0x23 => Self::MiniPC,
            0x24 => Self::StickPC,
            _ => Self::Unknown,
        }
    }
}

/// Chassis state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisState {
    Other,
    Unknown,
    Safe,
    Warning,
    Critical,
    NonRecoverable,
}

impl From<u8> for ChassisState {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Safe,
            0x04 => Self::Warning,
            0x05 => Self::Critical,
            0x06 => Self::NonRecoverable,
            _ => Self::Unknown,
        }
    }
}

/// Security status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityStatus {
    Other,
    Unknown,
    None,
    ExternalInterfaceLockedOut,
    ExternalInterfaceEnabled,
}

impl From<u8> for SecurityStatus {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::None,
            0x04 => Self::ExternalInterfaceLockedOut,
            0x05 => Self::ExternalInterfaceEnabled,
            _ => Self::Unknown,
        }
    }
}

/// Processor Information
#[derive(Debug, Clone)]
pub struct ProcessorInformation {
    pub socket_designation: String,
    pub processor_type: ProcessorType,
    pub processor_family: u16,
    pub manufacturer: String,
    pub processor_id: u64,
    pub version: String,
    pub voltage: u8,
    pub external_clock: u16,
    pub max_speed: u16,
    pub current_speed: u16,
    pub status: u8,
    pub processor_upgrade: ProcessorUpgrade,
    pub l1_cache_handle: u16,
    pub l2_cache_handle: u16,
    pub l3_cache_handle: u16,
    pub serial_number: String,
    pub asset_tag: String,
    pub part_number: String,
    pub core_count: u16,
    pub core_enabled: u16,
    pub thread_count: u16,
    pub characteristics: u16,
}

impl ProcessorInformation {
    pub fn is_populated(&self) -> bool {
        (self.status & 0x40) != 0
    }
    pub fn cpu_status(&self) -> CpuStatus {
        match self.status & 0x07 {
            0 => CpuStatus::Unknown,
            1 => CpuStatus::Enabled,
            2 => CpuStatus::DisabledByUser,
            3 => CpuStatus::DisabledByBios,
            4 => CpuStatus::Idle,
            7 => CpuStatus::Other,
            _ => CpuStatus::Unknown,
        }
    }

    pub fn supports_64bit(&self) -> bool {
        (self.characteristics & 0x04) != 0
    }
    pub fn supports_multicore(&self) -> bool {
        (self.characteristics & 0x08) != 0
    }
    pub fn supports_ht(&self) -> bool {
        (self.characteristics & 0x10) != 0
    }
    pub fn supports_execute_protection(&self) -> bool {
        (self.characteristics & 0x20) != 0
    }
    pub fn supports_enhanced_virtualization(&self) -> bool {
        (self.characteristics & 0x40) != 0
    }
    pub fn supports_power_perf_control(&self) -> bool {
        (self.characteristics & 0x80) != 0
    }

    /// Get voltage in volts
    pub fn voltage_v(&self) -> f32 {
        if (self.voltage & 0x80) != 0 {
            (self.voltage & 0x7F) as f32 / 10.0
        } else {
            match self.voltage & 0x0F {
                0x01 => 5.0,
                0x02 => 3.3,
                0x04 => 2.9,
                _ => 0.0,
            }
        }
    }
}

/// CPU status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuStatus {
    Unknown,
    Enabled,
    DisabledByUser,
    DisabledByBios,
    Idle,
    Other,
}

/// Processor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorType {
    Other,
    Unknown,
    CentralProcessor,
    MathProcessor,
    DspProcessor,
    VideoProcessor,
}

impl From<u8> for ProcessorType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::CentralProcessor,
            0x04 => Self::MathProcessor,
            0x05 => Self::DspProcessor,
            0x06 => Self::VideoProcessor,
            _ => Self::Unknown,
        }
    }
}

/// Processor upgrade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessorUpgrade {
    Other,
    Unknown,
    DaughterBoard,
    ZifSocket,
    ReplaceablePiggyBack,
    None,
    LifSocket,
    Slot1,
    Slot2,
    Socket370,
    SlotA,
    SlotM,
    Socket423,
    SocketA,
    Socket478,
    Socket754,
    Socket940,
    Socket939,
    SocketMpga604,
    SocketLga771,
    SocketLga775,
    SocketS1,
    SocketAm2,
    SocketF,
    SocketLga1366,
    SocketG34,
    SocketAm3,
    SocketC32,
    SocketLga1156,
    SocketLga1567,
    SocketPga988A,
    SocketBga1288,
    SocketRpga988B,
    SocketBga1023,
    SocketBga1224,
    SocketLga1155,
    SocketLga1356,
    SocketLga2011,
    SocketFs1,
    SocketFs2,
    SocketFm1,
    SocketFm2,
    SocketLga20113,
    SocketLga13563,
    SocketLga1150,
    SocketBga1168,
    SocketBga1234,
    SocketBga1364,
    SocketAm4,
    SocketLga1151,
    SocketBga1356,
    SocketBga1440,
    SocketBga1515,
    SocketLga36471,
    SocketSp3,
    SocketSp3r2,
    SocketLga2066,
    SocketBga1392,
    SocketBga1510,
    SocketBga1528,
    SocketLga4189,
    SocketLga1200,
    SocketLga4677,
    SocketLga1700,
    SocketBga1744,
    SocketBga1781,
    SocketBga1211,
    SocketBga2422,
    SocketLga1211,
    SocketLga2422,
    SocketLga5773,
    SocketBga5773,
    SocketAm5,
    SocketSp5,
    SocketSp6,
    SocketBga883,
    SocketBga1190,
    SocketBga4129,
    SocketLga4710,
    SocketLga7529,
}

impl From<u8> for ProcessorUpgrade {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::DaughterBoard,
            0x04 => Self::ZifSocket,
            0x05 => Self::ReplaceablePiggyBack,
            0x06 => Self::None,
            0x07 => Self::LifSocket,
            0x08 => Self::Slot1,
            0x09 => Self::Slot2,
            0x0A => Self::Socket370,
            0x0B => Self::SlotA,
            0x0C => Self::SlotM,
            0x0D => Self::Socket423,
            0x0E => Self::SocketA,
            0x0F => Self::Socket478,
            0x10 => Self::Socket754,
            0x11 => Self::Socket940,
            0x12 => Self::Socket939,
            0x13 => Self::SocketMpga604,
            0x14 => Self::SocketLga771,
            0x15 => Self::SocketLga775,
            0x16 => Self::SocketS1,
            0x17 => Self::SocketAm2,
            0x18 => Self::SocketF,
            0x19 => Self::SocketLga1366,
            0x1A => Self::SocketG34,
            0x1B => Self::SocketAm3,
            0x1C => Self::SocketC32,
            0x1D => Self::SocketLga1156,
            0x1E => Self::SocketLga1567,
            0x1F => Self::SocketPga988A,
            0x20 => Self::SocketBga1288,
            0x21 => Self::SocketRpga988B,
            0x22 => Self::SocketBga1023,
            0x23 => Self::SocketBga1224,
            0x24 => Self::SocketLga1155,
            0x25 => Self::SocketLga1356,
            0x26 => Self::SocketLga2011,
            0x27 => Self::SocketFs1,
            0x28 => Self::SocketFs2,
            0x29 => Self::SocketFm1,
            0x2A => Self::SocketFm2,
            0x2B => Self::SocketLga20113,
            0x2C => Self::SocketLga13563,
            0x2D => Self::SocketLga1150,
            0x2E => Self::SocketBga1168,
            0x2F => Self::SocketBga1234,
            0x30 => Self::SocketBga1364,
            0x31 => Self::SocketAm4,
            0x32 => Self::SocketLga1151,
            0x33 => Self::SocketBga1356,
            0x34 => Self::SocketBga1440,
            0x35 => Self::SocketBga1515,
            0x36 => Self::SocketLga36471,
            0x37 => Self::SocketSp3,
            0x38 => Self::SocketSp3r2,
            0x39 => Self::SocketLga2066,
            0x3A => Self::SocketBga1392,
            0x3B => Self::SocketBga1510,
            0x3C => Self::SocketBga1528,
            0x3D => Self::SocketLga4189,
            0x3E => Self::SocketLga1200,
            0x3F => Self::SocketLga4677,
            0x40 => Self::SocketLga1700,
            0x41 => Self::SocketBga1744,
            0x42 => Self::SocketBga1781,
            0x43 => Self::SocketBga1211,
            0x44 => Self::SocketBga2422,
            0x45 => Self::SocketLga1211,
            0x46 => Self::SocketLga2422,
            0x47 => Self::SocketLga5773,
            0x48 => Self::SocketBga5773,
            0x49 => Self::SocketAm5,
            0x4A => Self::SocketSp5,
            0x4B => Self::SocketSp6,
            0x4C => Self::SocketBga883,
            0x4D => Self::SocketBga1190,
            0x4E => Self::SocketBga4129,
            0x4F => Self::SocketLga4710,
            0x50 => Self::SocketLga7529,
            _ => Self::Unknown,
        }
    }
}

/// Cache Information
#[derive(Debug, Clone)]
pub struct CacheInformation {
    pub socket_designation: String,
    pub level: u8,
    pub enabled: bool,
    pub location: CacheLocation,
    pub mode: CacheOperationalMode,
    pub max_size_kb: u32,
    pub installed_size_kb: u32,
    pub supported_sram_type: u16,
    pub current_sram_type: u16,
    pub speed_ns: u8,
    pub error_correction_type: CacheErrorCorrection,
    pub system_cache_type: CacheType,
    pub associativity: CacheAssociativity,
}

/// Cache location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLocation {
    Internal,
    External,
    Reserved,
    Unknown,
}

impl From<u8> for CacheLocation {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Internal,
            1 => Self::External,
            2 => Self::Reserved,
            _ => Self::Unknown,
        }
    }
}

/// Cache operational mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheOperationalMode {
    WriteThrough,
    WriteBack,
    VariesWithMemoryAddress,
    Unknown,
}

impl From<u8> for CacheOperationalMode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::WriteThrough,
            1 => Self::WriteBack,
            2 => Self::VariesWithMemoryAddress,
            _ => Self::Unknown,
        }
    }
}

/// Cache error correction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheErrorCorrection {
    Other,
    Unknown,
    None,
    Parity,
    SingleBitEcc,
    MultiBitEcc,
}

impl From<u8> for CacheErrorCorrection {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::None,
            0x04 => Self::Parity,
            0x05 => Self::SingleBitEcc,
            0x06 => Self::MultiBitEcc,
            _ => Self::Unknown,
        }
    }
}

/// Cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    Other,
    Unknown,
    Instruction,
    Data,
    Unified,
}

impl From<u8> for CacheType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Instruction,
            0x04 => Self::Data,
            0x05 => Self::Unified,
            _ => Self::Unknown,
        }
    }
}

/// Cache associativity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheAssociativity {
    Other,
    Unknown,
    DirectMapped,
    TwoWay,
    FourWay,
    FullyAssociative,
    EightWay,
    SixteenWay,
    TwelveWay,
    TwentyFourWay,
    ThirtyTwoWay,
    FortyEightWay,
    SixtyFourWay,
    TwentyWay,
}

impl From<u8> for CacheAssociativity {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::DirectMapped,
            0x04 => Self::TwoWay,
            0x05 => Self::FourWay,
            0x06 => Self::FullyAssociative,
            0x07 => Self::EightWay,
            0x08 => Self::SixteenWay,
            0x09 => Self::TwelveWay,
            0x0A => Self::TwentyFourWay,
            0x0B => Self::ThirtyTwoWay,
            0x0C => Self::FortyEightWay,
            0x0D => Self::SixtyFourWay,
            0x0E => Self::TwentyWay,
            _ => Self::Unknown,
        }
    }
}

/// Port connector information
#[derive(Debug, Clone)]
pub struct PortConnectorInformation {
    pub internal_reference_designator: String,
    pub internal_connector_type: PortConnectorType,
    pub external_reference_designator: String,
    pub external_connector_type: PortConnectorType,
    pub port_type: PortType,
}

/// Port connector type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortConnectorType {
    None,
    Centronics,
    MiniCentronics,
    Proprietary,
    Db25PinMale,
    Db25PinFemale,
    Db15PinMale,
    Db15PinFemale,
    Db9PinMale,
    Db9PinFemale,
    Rj11,
    Rj45,
    MiniScsi50Pin,
    MiniDin,
    MicroDin,
    Ps2,
    Infrared,
    HpHil,
    AccessBusUsb,
    SsaScsi,
    CircularDin8Male,
    CircularDin8Female,
    OnBoardIde,
    OnBoardFloppy,
    DualInline9Pin,
    DualInline25Pin,
    DualInline50Pin,
    DualInline68Pin,
    OnBoardSoundInputCdRom,
    MiniCentronics14,
    MiniCentronics26,
    MiniJackHeadphones,
    Bnc,
    Ieee1394,
    SasSata,
    UsbTypeC,
    Pc98,
    Pc98Hireso,
    PcH98,
    Pc98Note,
    Pc98Full,
    Other,
}

impl From<u8> for PortConnectorType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::None,
            0x01 => Self::Centronics,
            0x02 => Self::MiniCentronics,
            0x03 => Self::Proprietary,
            0x04 => Self::Db25PinMale,
            0x05 => Self::Db25PinFemale,
            0x06 => Self::Db15PinMale,
            0x07 => Self::Db15PinFemale,
            0x08 => Self::Db9PinMale,
            0x09 => Self::Db9PinFemale,
            0x0A => Self::Rj11,
            0x0B => Self::Rj45,
            0x0C => Self::MiniScsi50Pin,
            0x0D => Self::MiniDin,
            0x0E => Self::MicroDin,
            0x0F => Self::Ps2,
            0x10 => Self::Infrared,
            0x11 => Self::HpHil,
            0x12 => Self::AccessBusUsb,
            0x13 => Self::SsaScsi,
            0x14 => Self::CircularDin8Male,
            0x15 => Self::CircularDin8Female,
            0x16 => Self::OnBoardIde,
            0x17 => Self::OnBoardFloppy,
            0x18 => Self::DualInline9Pin,
            0x19 => Self::DualInline25Pin,
            0x1A => Self::DualInline50Pin,
            0x1B => Self::DualInline68Pin,
            0x1C => Self::OnBoardSoundInputCdRom,
            0x1D => Self::MiniCentronics14,
            0x1E => Self::MiniCentronics26,
            0x1F => Self::MiniJackHeadphones,
            0x20 => Self::Bnc,
            0x21 => Self::Ieee1394,
            0x22 => Self::SasSata,
            0x23 => Self::UsbTypeC,
            0xA0 => Self::Pc98,
            0xA1 => Self::Pc98Hireso,
            0xA2 => Self::PcH98,
            0xA3 => Self::Pc98Note,
            0xA4 => Self::Pc98Full,
            0xFF => Self::Other,
            _ => Self::Other,
        }
    }
}

/// Port type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    None,
    ParallelXtAt,
    ParallelPs2,
    ParallelEcp,
    ParallelEpp,
    ParallelEcpEpp,
    SerialXtAt,
    Serial16450,
    Serial16550,
    Serial16550A,
    Scsi,
    Midi,
    Joystick,
    Keyboard,
    Mouse,
    SsaScsi,
    Usb,
    FireWire,
    PcmciaTypeI,
    PcmciaTypeII,
    PcmciaTypeIII,
    Cardbus,
    AccessBus,
    Scsi2,
    ScsiWide,
    Pc98,
    Pc98Hireso,
    PcH98,
    Video,
    Audio,
    Modem,
    Network,
    Sata,
    Sas,
    Mfdp,
    Thunderbolt,
    Intel8251Compatible,
    Intel8251FifoCompatible,
    Other,
}

impl From<u8> for PortType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::None,
            0x01 => Self::ParallelXtAt,
            0x02 => Self::ParallelPs2,
            0x03 => Self::ParallelEcp,
            0x04 => Self::ParallelEpp,
            0x05 => Self::ParallelEcpEpp,
            0x06 => Self::SerialXtAt,
            0x07 => Self::Serial16450,
            0x08 => Self::Serial16550,
            0x09 => Self::Serial16550A,
            0x0A => Self::Scsi,
            0x0B => Self::Midi,
            0x0C => Self::Joystick,
            0x0D => Self::Keyboard,
            0x0E => Self::Mouse,
            0x0F => Self::SsaScsi,
            0x10 => Self::Usb,
            0x11 => Self::FireWire,
            0x12 => Self::PcmciaTypeI,
            0x13 => Self::PcmciaTypeII,
            0x14 => Self::PcmciaTypeIII,
            0x15 => Self::Cardbus,
            0x16 => Self::AccessBus,
            0x17 => Self::Scsi2,
            0x18 => Self::ScsiWide,
            0x19 => Self::Pc98,
            0x1A => Self::Pc98Hireso,
            0x1B => Self::PcH98,
            0x1C => Self::Video,
            0x1D => Self::Audio,
            0x1E => Self::Modem,
            0x1F => Self::Network,
            0x20 => Self::Sata,
            0x21 => Self::Sas,
            0x22 => Self::Mfdp,
            0x23 => Self::Thunderbolt,
            0xA0 => Self::Intel8251Compatible,
            0xA1 => Self::Intel8251FifoCompatible,
            0xFF => Self::Other,
            _ => Self::Other,
        }
    }
}

/// System slot information
#[derive(Debug, Clone)]
pub struct SystemSlotInformation {
    pub slot_designation: String,
    pub slot_type: SlotType,
    pub slot_data_bus_width: SlotDataBusWidth,
    pub current_usage: SlotUsage,
    pub slot_length: SlotLength,
    pub slot_id: u16,
    pub characteristics1: u8,
    pub characteristics2: u8,
    pub segment_group_number: u16,
    pub bus_number: u8,
    pub device_function_number: u8,
}

/// Slot type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotType {
    Other,
    Unknown,
    Isa,
    Mca,
    Eisa,
    Pci,
    Pcmcia,
    VlVesa,
    Proprietary,
    ProcessorCard,
    ProprietaryMemory,
    IoRiserCard,
    Nubus,
    Pci66Mhz,
    Agp,
    Agp2x,
    Agp4x,
    PciX,
    Agp8x,
    M2Socket1Dp,
    M2Socket1Sd,
    M2Socket2,
    M2Socket3,
    MxmTypeI,
    MxmTypeII,
    MxmTypeIIIStandard,
    MxmTypeIIIHe,
    MxmTypeIV,
    Mxm3TypeA,
    Mxm3TypeB,
    PcieGen2Sff8639,
    PcieGen3Sff8639,
    PcieGen4Sff8639,
    PcieGen5Sff8639,
    OcpNic3SmallFormFactor,
    OcpNic3LargeFormFactor,
    OcpNicPriorTo30,
    Cxl1,
    Cxl2,
    PciExpressGen2,
    PciExpressGen3,
    PciExpressGen4,
    PciExpressGen5,
    PciExpressMini52WithKeepouts,
    PciExpressMini52WithoutKeepouts,
    PciExpressMini76,
    PciExpressGen6,
    Pc98C20,
    Pc98C24,
    Pc98E,
    Pc98LocalBus,
    Pc98Card,
    PciExpress,
    PciExpressX1,
    PciExpressX2,
    PciExpressX4,
    PciExpressX8,
    PciExpressX16,
}

impl From<u8> for SlotType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Isa,
            0x04 => Self::Mca,
            0x05 => Self::Eisa,
            0x06 => Self::Pci,
            0x07 => Self::Pcmcia,
            0x08 => Self::VlVesa,
            0x09 => Self::Proprietary,
            0x0A => Self::ProcessorCard,
            0x0B => Self::ProprietaryMemory,
            0x0C => Self::IoRiserCard,
            0x0D => Self::Nubus,
            0x0E => Self::Pci66Mhz,
            0x0F => Self::Agp,
            0x10 => Self::Agp2x,
            0x11 => Self::Agp4x,
            0x12 => Self::PciX,
            0x13 => Self::Agp8x,
            0x14 => Self::M2Socket1Dp,
            0x15 => Self::M2Socket1Sd,
            0x16 => Self::M2Socket2,
            0x17 => Self::M2Socket3,
            0x18 => Self::MxmTypeI,
            0x19 => Self::MxmTypeII,
            0x1A => Self::MxmTypeIIIStandard,
            0x1B => Self::MxmTypeIIIHe,
            0x1C => Self::MxmTypeIV,
            0x1D => Self::Mxm3TypeA,
            0x1E => Self::Mxm3TypeB,
            0x1F => Self::PcieGen2Sff8639,
            0x20 => Self::PcieGen3Sff8639,
            0x21 => Self::PcieGen4Sff8639,
            0x22 => Self::PcieGen5Sff8639,
            0x23 => Self::OcpNic3SmallFormFactor,
            0x24 => Self::OcpNic3LargeFormFactor,
            0x25 => Self::OcpNicPriorTo30,
            0x26 => Self::Cxl1,
            0x27 => Self::Cxl2,
            0x30 => Self::PciExpressGen2,
            0x31 => Self::PciExpressGen3,
            0x32 => Self::PciExpressGen4,
            0x33 => Self::PciExpressGen5,
            0x34 => Self::PciExpressMini52WithKeepouts,
            0x35 => Self::PciExpressMini52WithoutKeepouts,
            0x36 => Self::PciExpressMini76,
            0x37 => Self::PciExpressGen6,
            0xA0 => Self::Pc98C20,
            0xA1 => Self::Pc98C24,
            0xA2 => Self::Pc98E,
            0xA3 => Self::Pc98LocalBus,
            0xA4 => Self::Pc98Card,
            0xA5 => Self::PciExpress,
            0xA6 => Self::PciExpressX1,
            0xA7 => Self::PciExpressX2,
            0xA8 => Self::PciExpressX4,
            0xA9 => Self::PciExpressX8,
            0xAA => Self::PciExpressX16,
            _ => Self::Unknown,
        }
    }
}

/// Slot data bus width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotDataBusWidth {
    Other,
    Unknown,
    Bit8,
    Bit16,
    Bit32,
    Bit64,
    Bit128,
    X1,
    X2,
    X4,
    X8,
    X12,
    X16,
    X32,
}

impl From<u8> for SlotDataBusWidth {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Bit8,
            0x04 => Self::Bit16,
            0x05 => Self::Bit32,
            0x06 => Self::Bit64,
            0x07 => Self::Bit128,
            0x08 => Self::X1,
            0x09 => Self::X2,
            0x0A => Self::X4,
            0x0B => Self::X8,
            0x0C => Self::X12,
            0x0D => Self::X16,
            0x0E => Self::X32,
            _ => Self::Unknown,
        }
    }
}

/// Slot usage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotUsage {
    Other,
    Unknown,
    Available,
    InUse,
    Unavailable,
}

impl From<u8> for SlotUsage {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Available,
            0x04 => Self::InUse,
            0x05 => Self::Unavailable,
            _ => Self::Unknown,
        }
    }
}

/// Slot length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotLength {
    Other,
    Unknown,
    Short,
    Long,
    DriveFormFactor25,
    DriveFormFactor35,
}

impl From<u8> for SlotLength {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Short,
            0x04 => Self::Long,
            0x05 => Self::DriveFormFactor25,
            0x06 => Self::DriveFormFactor35,
            _ => Self::Unknown,
        }
    }
}

/// On-board device information
#[derive(Debug, Clone)]
pub struct OnboardDeviceInformation {
    pub description: String,
    pub device_type: OnboardDeviceType,
    pub enabled: bool,
}

/// On-board device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardDeviceType {
    Other,
    Unknown,
    Video,
    ScsiController,
    Ethernet,
    TokenRing,
    Sound,
    PataController,
    SataController,
    SasController,
    WirelessLan,
    Bluetooth,
    Wwan,
    Emmc,
    NvmeController,
    UfsController,
}

impl From<u8> for OnboardDeviceType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Video,
            0x04 => Self::ScsiController,
            0x05 => Self::Ethernet,
            0x06 => Self::TokenRing,
            0x07 => Self::Sound,
            0x08 => Self::PataController,
            0x09 => Self::SataController,
            0x0A => Self::SasController,
            0x0B => Self::WirelessLan,
            0x0C => Self::Bluetooth,
            0x0D => Self::Wwan,
            0x0E => Self::Emmc,
            0x0F => Self::NvmeController,
            0x10 => Self::UfsController,
            _ => Self::Unknown,
        }
    }
}

/// Physical memory array information
#[derive(Debug, Clone)]
pub struct PhysicalMemoryArrayInformation {
    pub location: MemoryArrayLocation,
    pub use_type: MemoryArrayUse,
    pub error_correction: MemoryErrorCorrection,
    pub maximum_capacity_kb: u64,
    pub error_information_handle: u16,
    pub number_of_memory_devices: u16,
}

/// Memory array location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryArrayLocation {
    Other,
    Unknown,
    SystemBoard,
    IsaAddon,
    EisaAddon,
    PciAddon,
    McaAddon,
    PcmciaAddon,
    ProprietaryAddon,
    NuBus,
    Pc98C20Addon,
    Pc98C24Addon,
    Pc98EAddon,
    Pc98LocalBusAddon,
    CxlFlexbus10Addon,
}

impl From<u8> for MemoryArrayLocation {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::SystemBoard,
            0x04 => Self::IsaAddon,
            0x05 => Self::EisaAddon,
            0x06 => Self::PciAddon,
            0x07 => Self::McaAddon,
            0x08 => Self::PcmciaAddon,
            0x09 => Self::ProprietaryAddon,
            0x0A => Self::NuBus,
            0xA0 => Self::Pc98C20Addon,
            0xA1 => Self::Pc98C24Addon,
            0xA2 => Self::Pc98EAddon,
            0xA3 => Self::Pc98LocalBusAddon,
            0xA4 => Self::CxlFlexbus10Addon,
            _ => Self::Unknown,
        }
    }
}

/// Memory array use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryArrayUse {
    Other,
    Unknown,
    SystemMemory,
    VideoMemory,
    FlashMemory,
    NonVolatileRam,
    CacheMemory,
}

impl From<u8> for MemoryArrayUse {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::SystemMemory,
            0x04 => Self::VideoMemory,
            0x05 => Self::FlashMemory,
            0x06 => Self::NonVolatileRam,
            0x07 => Self::CacheMemory,
            _ => Self::Unknown,
        }
    }
}

/// Memory error correction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryErrorCorrection {
    Other,
    Unknown,
    None,
    Parity,
    SingleBitEcc,
    MultiBitEcc,
    Crc,
}

impl From<u8> for MemoryErrorCorrection {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::None,
            0x04 => Self::Parity,
            0x05 => Self::SingleBitEcc,
            0x06 => Self::MultiBitEcc,
            0x07 => Self::Crc,
            _ => Self::Unknown,
        }
    }
}

/// Memory device information
#[derive(Debug, Clone)]
pub struct MemoryDeviceInformation {
    pub physical_memory_array_handle: u16,
    pub memory_error_information_handle: u16,
    pub total_width: u16,
    pub data_width: u16,
    pub size_mb: u64,
    pub form_factor: MemoryFormFactor,
    pub device_set: u8,
    pub device_locator: String,
    pub bank_locator: String,
    pub memory_type: MemoryType,
    pub type_detail: u16,
    pub speed_mhz: u16,
    pub manufacturer: String,
    pub serial_number: String,
    pub asset_tag: String,
    pub part_number: String,
    pub rank: u8,
    pub configured_memory_speed_mhz: u16,
    pub minimum_voltage_mv: u16,
    pub maximum_voltage_mv: u16,
    pub configured_voltage_mv: u16,
    pub memory_technology: MemoryTechnology,
    pub memory_operating_mode_capability: u16,
}

/// Memory form factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryFormFactor {
    Other,
    Unknown,
    Simm,
    Sip,
    Chip,
    Dip,
    Zip,
    ProprietaryCard,
    Dimm,
    Tsop,
    RowOfChips,
    Rimm,
    Sodimm,
    Srimm,
    FbDimm,
    Die,
}

impl From<u8> for MemoryFormFactor {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Simm,
            0x04 => Self::Sip,
            0x05 => Self::Chip,
            0x06 => Self::Dip,
            0x07 => Self::Zip,
            0x08 => Self::ProprietaryCard,
            0x09 => Self::Dimm,
            0x0A => Self::Tsop,
            0x0B => Self::RowOfChips,
            0x0C => Self::Rimm,
            0x0D => Self::Sodimm,
            0x0E => Self::Srimm,
            0x0F => Self::FbDimm,
            0x10 => Self::Die,
            _ => Self::Unknown,
        }
    }
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Other,
    Unknown,
    Dram,
    Edram,
    Vram,
    Sram,
    Ram,
    Rom,
    Flash,
    Eeprom,
    Feprom,
    Eprom,
    Cdram,
    Dram3D,
    Sdram,
    Sgram,
    Rdram,
    Ddr,
    Ddr2,
    Ddr2FbDimm,
    Reserved1,
    Reserved2,
    Reserved3,
    Ddr3,
    Fbd2,
    Ddr4,
    Lpddr,
    Lpddr2,
    Lpddr3,
    Lpddr4,
    LogicalNonVolatile,
    Hbm,
    Hbm2,
    Ddr5,
    Lpddr5,
    Hbm3,
}

impl From<u8> for MemoryType {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Dram,
            0x04 => Self::Edram,
            0x05 => Self::Vram,
            0x06 => Self::Sram,
            0x07 => Self::Ram,
            0x08 => Self::Rom,
            0x09 => Self::Flash,
            0x0A => Self::Eeprom,
            0x0B => Self::Feprom,
            0x0C => Self::Eprom,
            0x0D => Self::Cdram,
            0x0E => Self::Dram3D,
            0x0F => Self::Sdram,
            0x10 => Self::Sgram,
            0x11 => Self::Rdram,
            0x12 => Self::Ddr,
            0x13 => Self::Ddr2,
            0x14 => Self::Ddr2FbDimm,
            0x15 => Self::Reserved1,
            0x16 => Self::Reserved2,
            0x17 => Self::Reserved3,
            0x18 => Self::Ddr3,
            0x19 => Self::Fbd2,
            0x1A => Self::Ddr4,
            0x1B => Self::Lpddr,
            0x1C => Self::Lpddr2,
            0x1D => Self::Lpddr3,
            0x1E => Self::Lpddr4,
            0x1F => Self::LogicalNonVolatile,
            0x20 => Self::Hbm,
            0x21 => Self::Hbm2,
            0x22 => Self::Ddr5,
            0x23 => Self::Lpddr5,
            0x24 => Self::Hbm3,
            _ => Self::Unknown,
        }
    }
}

/// Memory technology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTechnology {
    Other,
    Unknown,
    Dram,
    NvdimmN,
    NvdimmF,
    NvdimmP,
    IntelOptane,
}

impl From<u8> for MemoryTechnology {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::Other,
            0x02 => Self::Unknown,
            0x03 => Self::Dram,
            0x04 => Self::NvdimmN,
            0x05 => Self::NvdimmF,
            0x06 => Self::NvdimmP,
            0x07 => Self::IntelOptane,
            _ => Self::Unknown,
        }
    }
}

/// System boot information
#[derive(Debug, Clone)]
pub struct SystemBootInformation {
    pub status: BootStatus,
}

/// Boot status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootStatus {
    NoErrorsDetected,
    NoBootableMedia,
    NormalOsNotLoaded,
    FirmwareDetectedFailure,
    OsDetectedFailure,
    UserRequestedBoot,
    SystemSecurityViolation,
    PreviouslyRequestedImage,
    SystemWatchdogTimer,
    Reserved,
    Other,
}

impl From<u8> for BootStatus {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::NoErrorsDetected,
            1 => Self::NoBootableMedia,
            2 => Self::NormalOsNotLoaded,
            3 => Self::FirmwareDetectedFailure,
            4 => Self::OsDetectedFailure,
            5 => Self::UserRequestedBoot,
            6 => Self::SystemSecurityViolation,
            7 => Self::PreviouslyRequestedImage,
            8 => Self::SystemWatchdogTimer,
            9..=127 => Self::Reserved,
            _ => Self::Other,
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type() {
        assert_eq!(MemoryType::from(0x22), MemoryType::Ddr5);
        assert_eq!(MemoryType::from(0x1A), MemoryType::Ddr4);
    }

    #[test]
    fn test_chassis_type() {
        assert_eq!(ChassisType::from(0x03), ChassisType::Desktop);
        assert_eq!(ChassisType::from(0x09), ChassisType::Laptop);
    }

    #[test]
    fn test_processor_upgrade() {
        assert_eq!(ProcessorUpgrade::from(0x49), ProcessorUpgrade::SocketAm5);
    }
}
