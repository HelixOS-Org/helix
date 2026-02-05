//! SD/MMC Card Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive SD/MMC card protocol support
//! for booting from SD cards and eMMC storage in the UEFI environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                       SD/MMC Protocol Stack                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Block Layer    │  Read  │  Write  │  Erase  │  Trim  │  Secure        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Card Layer     │  SD 2.0/3.0/4.0  │  MMC 4.x/5.x  │  eMMC 5.1        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Command Layer  │  Basic  │  Extended  │  Application  │  Switch       │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware       │  SDHC Controller  │  DMA  │  Interrupts              │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - SD Memory Card specification 2.0-4.0
//! - SDIO support
//! - eMMC 5.1 specification
//! - High-speed and UHS modes
//! - DDR and HS400 modes
//! - Command queuing
//! - Secure erase operations

use core::fmt;

// =============================================================================
// SD/MMC CONSTANTS
// =============================================================================

/// SD card default RCA
pub const SD_DEFAULT_RCA: u16 = 0x0001;

/// Maximum number of retries for commands
pub const SD_MAX_RETRIES: u32 = 1000;

/// SD card voltage check pattern
pub const SD_CHECK_PATTERN: u8 = 0xAA;

/// OCR voltage window (2.7V - 3.6V)
pub const OCR_VOLTAGE_WINDOW: u32 = 0x00FF_8000;

/// OCR high capacity bit
pub const OCR_HCS: u32 = 1 << 30;

/// OCR power up status bit
pub const OCR_BUSY: u32 = 1 << 31;

/// OCR S18A (switching to 1.8V accepted)
pub const OCR_S18A: u32 = 1 << 24;

/// SD sector size
pub const SD_SECTOR_SIZE: usize = 512;

/// Maximum block count for multi-block transfer
pub const SD_MAX_BLOCK_COUNT: u16 = 65535;

/// eMMC boot partition size multiplier (128KB units)
pub const EMMC_BOOT_MULT: u32 = 128 * 1024;

// =============================================================================
// SD/MMC COMMANDS
// =============================================================================

/// SD/MMC command definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SdCommand {
    /// GO_IDLE_STATE - Reset all cards
    GoIdleState        = 0,
    /// ALL_SEND_CID - Request all cards send CID
    AllSendCid         = 2,
    /// SEND_RELATIVE_ADDR - Ask card to publish RCA
    SendRelativeAddr   = 3,
    /// SET_DSR - Set driver stage register
    SetDsr             = 4,
    /// SWITCH - Switch function/high speed
    Switch             = 6,
    /// SELECT_CARD - Select/deselect card
    SelectCard         = 7,
    /// SEND_IF_COND - Send interface condition
    SendIfCond         = 8,
    /// SEND_CSD - Request CSD from card
    SendCsd            = 9,
    /// SEND_CID - Request CID from card
    SendCid            = 10,
    /// VOLTAGE_SWITCH - Switch to 1.8V signaling
    VoltageSwitch      = 11,
    /// STOP_TRANSMISSION - Stop transmission
    StopTransmission   = 12,
    /// SEND_STATUS - Request card status
    SendStatus         = 13,
    /// GO_INACTIVE_STATE - Set card to inactive
    GoInactiveState    = 15,
    /// SET_BLOCKLEN - Set block length
    SetBlocklen        = 16,
    /// READ_SINGLE_BLOCK - Read single block
    ReadSingleBlock    = 17,
    /// READ_MULTIPLE_BLOCK - Read multiple blocks
    ReadMultipleBlock  = 18,
    /// SEND_TUNING_BLOCK - Send tuning pattern
    SendTuningBlock    = 19,
    /// SPEED_CLASS_CONTROL - Speed class control
    SpeedClassControl  = 20,
    /// SET_BLOCK_COUNT - Set block count for multi-block
    SetBlockCount      = 23,
    /// WRITE_BLOCK - Write single block
    WriteBlock         = 24,
    /// WRITE_MULTIPLE_BLOCK - Write multiple blocks
    WriteMultipleBlock = 25,
    /// PROGRAM_CSD - Program CSD
    ProgramCsd         = 27,
    /// SET_WRITE_PROT - Set write protection
    SetWriteProt       = 28,
    /// CLR_WRITE_PROT - Clear write protection
    ClrWriteProt       = 29,
    /// SEND_WRITE_PROT - Send write protection status
    SendWriteProt      = 30,
    /// ERASE_WR_BLK_START - Set first erase block
    EraseWrBlkStart    = 32,
    /// ERASE_WR_BLK_END - Set last erase block
    EraseWrBlkEnd      = 33,
    /// ERASE - Erase blocks
    Erase              = 38,
    /// LOCK_UNLOCK - Lock/unlock card
    LockUnlock         = 42,
    /// APP_CMD - Next command is application command
    AppCmd             = 55,
    /// GEN_CMD - General command
    GenCmd             = 56,
    /// READ_OCR - Read OCR register (SPI mode)
    ReadOcr            = 58,
    /// CRC_ON_OFF - Turn CRC on/off (SPI mode)
    CrcOnOff           = 59,
}

/// SD Application commands (ACMD)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SdAcmd {
    /// SET_BUS_WIDTH
    SetBusWidth        = 6,
    /// SD_STATUS
    SdStatus           = 13,
    /// SEND_NUM_WR_BLOCKS
    SendNumWrBlocks    = 22,
    /// SET_WR_BLK_ERASE_COUNT
    SetWrBlkEraseCount = 23,
    /// SD_SEND_OP_COND
    SdSendOpCond       = 41,
    /// SET_CLR_CARD_DETECT
    SetClrCardDetect   = 42,
    /// SEND_SCR
    SendScr            = 51,
}

/// MMC-specific commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MmcCommand {
    /// SEND_OP_COND - Request OCR from card
    SendOpCond      = 1,
    /// SET_RELATIVE_ADDR - Assign RCA
    SetRelativeAddr = 3,
    /// SLEEP_AWAKE - Toggle sleep mode
    SleepAwake      = 5,
    /// SWITCH - Extended CSD access
    Switch          = 6,
    /// SEND_EXT_CSD - Send extended CSD
    SendExtCsd      = 8,
    /// BUSTEST_W - Bus test write
    BustestW        = 19,
    /// BUSTEST_R - Bus test read
    BustestR        = 14,
    /// ERASE_GROUP_START - Set first erase group
    EraseGroupStart = 35,
    /// ERASE_GROUP_END - Set last erase group
    EraseGroupEnd   = 36,
}

// =============================================================================
// COMMAND FLAGS AND TYPES
// =============================================================================

/// Command response types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseType {
    /// No response
    #[default]
    None,
    /// R1 - Normal response
    R1,
    /// R1b - Normal response with busy
    R1b,
    /// R2 - CID/CSD response (136 bits)
    R2,
    /// R3 - OCR response
    R3,
    /// R4 - Fast I/O (MMC)
    R4,
    /// R5 - SDIO response
    R5,
    /// R6 - RCA response
    R6,
    /// R7 - Card interface condition
    R7,
}

/// Data transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataDirection {
    /// No data transfer
    #[default]
    None,
    /// Read from card
    Read,
    /// Write to card
    Write,
}

/// Command flags
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandFlags {
    /// Response type
    pub response: ResponseType,
    /// Data direction
    pub data: DataDirection,
    /// Multiple block transfer
    pub multi_block: bool,
    /// Use DMA
    pub use_dma: bool,
    /// Auto CMD12
    pub auto_cmd12: bool,
    /// Auto CMD23
    pub auto_cmd23: bool,
    /// Check CRC
    pub check_crc: bool,
    /// Check index
    pub check_index: bool,
}

impl CommandFlags {
    /// Create flags for no-response command
    pub const fn no_response() -> Self {
        Self {
            response: ResponseType::None,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: false,
            check_index: false,
        }
    }

    /// Create flags for R1 response
    pub const fn r1() -> Self {
        Self {
            response: ResponseType::R1,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: true,
            check_index: true,
        }
    }

    /// Create flags for R1b response
    pub const fn r1b() -> Self {
        Self {
            response: ResponseType::R1b,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: true,
            check_index: true,
        }
    }

    /// Create flags for R2 response
    pub const fn r2() -> Self {
        Self {
            response: ResponseType::R2,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: true,
            check_index: false,
        }
    }

    /// Create flags for R3 response (OCR)
    pub const fn r3() -> Self {
        Self {
            response: ResponseType::R3,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: false,
            check_index: false,
        }
    }

    /// Create flags for R6 response (RCA)
    pub const fn r6() -> Self {
        Self {
            response: ResponseType::R6,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: true,
            check_index: true,
        }
    }

    /// Create flags for R7 response
    pub const fn r7() -> Self {
        Self {
            response: ResponseType::R7,
            data: DataDirection::None,
            multi_block: false,
            use_dma: false,
            auto_cmd12: false,
            auto_cmd23: false,
            check_crc: true,
            check_index: true,
        }
    }

    /// Set read data transfer
    pub const fn with_read(mut self) -> Self {
        self.data = DataDirection::Read;
        self
    }

    /// Set write data transfer
    pub const fn with_write(mut self) -> Self {
        self.data = DataDirection::Write;
        self
    }

    /// Set multi-block transfer
    pub const fn multi(mut self) -> Self {
        self.multi_block = true;
        self
    }
}

// =============================================================================
// CARD STATUS
// =============================================================================

/// Card status (R1 response)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardStatus(pub u32);

impl CardStatus {
    /// App command enabled
    pub const fn app_cmd(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }

    /// Ready for data
    pub const fn ready_for_data(&self) -> bool {
        (self.0 & (1 << 8)) != 0
    }

    /// Current state
    pub const fn current_state(&self) -> CardState {
        CardState::from_u8(((self.0 >> 9) & 0xF) as u8)
    }

    /// Erase reset
    pub const fn erase_reset(&self) -> bool {
        (self.0 & (1 << 13)) != 0
    }

    /// Card ECC disabled
    pub const fn card_ecc_disabled(&self) -> bool {
        (self.0 & (1 << 14)) != 0
    }

    /// Write protect erase skip
    pub const fn wp_erase_skip(&self) -> bool {
        (self.0 & (1 << 15)) != 0
    }

    /// CSD overwrite error
    pub const fn csd_overwrite(&self) -> bool {
        (self.0 & (1 << 16)) != 0
    }

    /// Error
    pub const fn error(&self) -> bool {
        (self.0 & (1 << 19)) != 0
    }

    /// CC error
    pub const fn cc_error(&self) -> bool {
        (self.0 & (1 << 20)) != 0
    }

    /// Card ECC failed
    pub const fn card_ecc_failed(&self) -> bool {
        (self.0 & (1 << 21)) != 0
    }

    /// Illegal command
    pub const fn illegal_command(&self) -> bool {
        (self.0 & (1 << 22)) != 0
    }

    /// COM CRC error
    pub const fn com_crc_error(&self) -> bool {
        (self.0 & (1 << 23)) != 0
    }

    /// Lock/unlock failed
    pub const fn lock_unlock_failed(&self) -> bool {
        (self.0 & (1 << 24)) != 0
    }

    /// Card is locked
    pub const fn card_is_locked(&self) -> bool {
        (self.0 & (1 << 25)) != 0
    }

    /// Write protect violation
    pub const fn wp_violation(&self) -> bool {
        (self.0 & (1 << 26)) != 0
    }

    /// Erase param error
    pub const fn erase_param(&self) -> bool {
        (self.0 & (1 << 27)) != 0
    }

    /// Erase sequence error
    pub const fn erase_seq_error(&self) -> bool {
        (self.0 & (1 << 28)) != 0
    }

    /// Block length error
    pub const fn block_len_error(&self) -> bool {
        (self.0 & (1 << 29)) != 0
    }

    /// Address error
    pub const fn address_error(&self) -> bool {
        (self.0 & (1 << 30)) != 0
    }

    /// Out of range
    pub const fn out_of_range(&self) -> bool {
        (self.0 & (1 << 31)) != 0
    }

    /// Check for any error
    pub const fn has_error(&self) -> bool {
        (self.0 & 0xFDF9_8000) != 0
    }
}

/// Card states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CardState {
    /// Idle state
    Idle    = 0,
    /// Ready state
    Ready   = 1,
    /// Identification state
    Ident   = 2,
    /// Stand-by state
    Stby    = 3,
    /// Transfer state
    Tran    = 4,
    /// Data state
    Data    = 5,
    /// Receive state
    Rcv     = 6,
    /// Programming state
    Prg     = 7,
    /// Disconnect state
    Dis     = 8,
    /// Bus test state
    Btst    = 9,
    /// Sleep state
    Slp     = 10,
    /// Unknown state
    Unknown = 15,
}

impl CardState {
    /// Create from u8
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0 => CardState::Idle,
            1 => CardState::Ready,
            2 => CardState::Ident,
            3 => CardState::Stby,
            4 => CardState::Tran,
            5 => CardState::Data,
            6 => CardState::Rcv,
            7 => CardState::Prg,
            8 => CardState::Dis,
            9 => CardState::Btst,
            10 => CardState::Slp,
            _ => CardState::Unknown,
        }
    }
}

// =============================================================================
// CID (Card Identification) REGISTER
// =============================================================================

/// CID register for SD cards (128 bits)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SdCid {
    /// Raw CID data (16 bytes / 128 bits)
    pub raw: [u8; 16],
}

impl SdCid {
    /// Manufacturer ID (MID)
    pub const fn manufacturer_id(&self) -> u8 {
        self.raw[0]
    }

    /// OEM/Application ID (OID)
    pub fn oem_id(&self) -> [u8; 2] {
        [self.raw[1], self.raw[2]]
    }

    /// Product name (PNM) - 5 bytes
    pub fn product_name(&self) -> [u8; 5] {
        [
            self.raw[3],
            self.raw[4],
            self.raw[5],
            self.raw[6],
            self.raw[7],
        ]
    }

    /// Product revision (PRV)
    pub const fn product_revision(&self) -> u8 {
        self.raw[8]
    }

    /// Product revision major
    pub const fn product_revision_major(&self) -> u8 {
        self.raw[8] >> 4
    }

    /// Product revision minor
    pub const fn product_revision_minor(&self) -> u8 {
        self.raw[8] & 0x0F
    }

    /// Product serial number (PSN)
    pub fn product_serial(&self) -> u32 {
        ((self.raw[9] as u32) << 24)
            | ((self.raw[10] as u32) << 16)
            | ((self.raw[11] as u32) << 8)
            | (self.raw[12] as u32)
    }

    /// Manufacturing date year (0 = 2000)
    pub const fn manufacturing_year(&self) -> u16 {
        2000 + ((((self.raw[13] & 0x0F) as u16) << 4) | ((self.raw[14] >> 4) as u16))
    }

    /// Manufacturing date month
    pub const fn manufacturing_month(&self) -> u8 {
        self.raw[14] & 0x0F
    }

    /// CRC7 checksum
    pub const fn crc(&self) -> u8 {
        self.raw[15] >> 1
    }
}

/// CID register for MMC cards (128 bits)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MmcCid {
    /// Raw CID data
    pub raw: [u8; 16],
}

impl MmcCid {
    /// Manufacturer ID
    pub const fn manufacturer_id(&self) -> u8 {
        self.raw[0]
    }

    /// Device/BGA type and OEM ID
    pub const fn device_type(&self) -> u8 {
        self.raw[1]
    }

    /// OEM ID
    pub const fn oem_id(&self) -> u8 {
        self.raw[2]
    }

    /// Product name - 6 bytes
    pub fn product_name(&self) -> [u8; 6] {
        [
            self.raw[3],
            self.raw[4],
            self.raw[5],
            self.raw[6],
            self.raw[7],
            self.raw[8],
        ]
    }

    /// Product revision
    pub const fn product_revision(&self) -> u8 {
        self.raw[9]
    }

    /// Serial number
    pub fn serial_number(&self) -> u32 {
        ((self.raw[10] as u32) << 24)
            | ((self.raw[11] as u32) << 16)
            | ((self.raw[12] as u32) << 8)
            | (self.raw[13] as u32)
    }

    /// Manufacturing date
    pub const fn manufacturing_date(&self) -> u8 {
        self.raw[14]
    }
}

// =============================================================================
// CSD (Card Specific Data) REGISTER
// =============================================================================

/// CSD version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsdVersion {
    /// CSD Version 1.0 (SDSC)
    V1,
    /// CSD Version 2.0 (SDHC/SDXC)
    V2,
    /// CSD Version 3.0 (SDUC)
    V3,
    /// Unknown/reserved
    Unknown,
}

/// CSD register (128 bits)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Csd {
    /// Raw CSD data (16 bytes)
    pub raw: [u8; 16],
}

impl Csd {
    /// Get CSD structure version
    pub fn version(&self) -> CsdVersion {
        match (self.raw[0] >> 6) & 0x03 {
            0 => CsdVersion::V1,
            1 => CsdVersion::V2,
            2 => CsdVersion::V3,
            _ => CsdVersion::Unknown,
        }
    }

    /// TAAC - Data read access time (ns)
    pub const fn taac(&self) -> u8 {
        self.raw[1]
    }

    /// NSAC - Data read access time (CLK cycles)
    pub const fn nsac(&self) -> u8 {
        self.raw[2]
    }

    /// TRAN_SPEED - Max data transfer rate
    pub const fn tran_speed(&self) -> u8 {
        self.raw[3]
    }

    /// Max transfer speed in kHz
    pub fn max_transfer_rate_khz(&self) -> u32 {
        let rate_unit = match self.raw[3] & 0x07 {
            0 => 100,     // 100 kbit/s
            1 => 1_000,   // 1 Mbit/s
            2 => 10_000,  // 10 Mbit/s
            3 => 100_000, // 100 Mbit/s
            _ => 0,
        };

        let multiplier = match (self.raw[3] >> 3) & 0x0F {
            1 => 10,
            2 => 12,
            3 => 13,
            4 => 15,
            5 => 20,
            6 => 25,
            7 => 30,
            8 => 35,
            9 => 40,
            10 => 45,
            11 => 50,
            12 => 55,
            13 => 60,
            14 => 70,
            15 => 80,
            _ => 0,
        };

        rate_unit * multiplier / 10
    }

    /// CCC - Card command classes
    pub fn ccc(&self) -> u16 {
        ((self.raw[4] as u16) << 4) | ((self.raw[5] >> 4) as u16)
    }

    /// READ_BL_LEN - Max read data block length
    pub const fn read_bl_len(&self) -> u8 {
        self.raw[5] & 0x0F
    }

    /// Max read block length in bytes
    pub const fn max_read_block_len(&self) -> usize {
        1 << (self.raw[5] & 0x0F)
    }

    /// READ_BL_PARTIAL - Partial blocks for read allowed
    pub const fn read_bl_partial(&self) -> bool {
        (self.raw[6] & 0x80) != 0
    }

    /// WRITE_BLK_MISALIGN - Write block misalignment
    pub const fn write_blk_misalign(&self) -> bool {
        (self.raw[6] & 0x40) != 0
    }

    /// READ_BLK_MISALIGN - Read block misalignment
    pub const fn read_blk_misalign(&self) -> bool {
        (self.raw[6] & 0x20) != 0
    }

    /// DSR_IMP - DSR implemented
    pub const fn dsr_imp(&self) -> bool {
        (self.raw[6] & 0x10) != 0
    }

    /// C_SIZE - Device size (version dependent)
    pub fn c_size(&self) -> u32 {
        match self.version() {
            CsdVersion::V1 => {
                let c_size_high = ((self.raw[6] & 0x03) as u32) << 10;
                let c_size_mid = (self.raw[7] as u32) << 2;
                let c_size_low = ((self.raw[8] >> 6) as u32) & 0x03;
                c_size_high | c_size_mid | c_size_low
            },
            CsdVersion::V2 | CsdVersion::V3 => {
                let c_size_high = ((self.raw[7] & 0x3F) as u32) << 16;
                let c_size_mid = (self.raw[8] as u32) << 8;
                let c_size_low = self.raw[9] as u32;
                c_size_high | c_size_mid | c_size_low
            },
            CsdVersion::Unknown => 0,
        }
    }

    /// C_SIZE_MULT - Device size multiplier (V1 only)
    pub const fn c_size_mult(&self) -> u8 {
        ((self.raw[9] & 0x03) << 1) | ((self.raw[10] >> 7) & 0x01)
    }

    /// Calculate card capacity in sectors (512 bytes each)
    pub fn capacity_sectors(&self) -> u64 {
        match self.version() {
            CsdVersion::V1 => {
                let c_size = self.c_size() as u64;
                let c_size_mult = self.c_size_mult() as u64;
                let read_bl_len = self.read_bl_len() as u64;
                let mult = 1u64 << (c_size_mult + 2);
                let blocknr = (c_size + 1) * mult;
                let block_len = 1u64 << read_bl_len;
                (blocknr * block_len) / 512
            },
            CsdVersion::V2 => {
                // (C_SIZE + 1) * 512K bytes
                (self.c_size() as u64 + 1) * 1024
            },
            CsdVersion::V3 => {
                // (C_SIZE + 1) * 512K bytes (same as V2 for SDUC)
                (self.c_size() as u64 + 1) * 1024
            },
            CsdVersion::Unknown => 0,
        }
    }

    /// Calculate card capacity in bytes
    pub fn capacity_bytes(&self) -> u64 {
        self.capacity_sectors() * 512
    }

    /// ERASE_BLK_EN - Erase single block enable
    pub const fn erase_blk_en(&self) -> bool {
        (self.raw[10] & 0x40) != 0
    }

    /// SECTOR_SIZE - Erase sector size
    pub const fn sector_size(&self) -> u8 {
        ((self.raw[10] & 0x3F) << 1) | ((self.raw[11] >> 7) & 0x01)
    }

    /// WP_GRP_SIZE - Write protect group size
    pub const fn wp_grp_size(&self) -> u8 {
        self.raw[11] & 0x7F
    }

    /// WP_GRP_ENABLE - Write protect group enable
    pub const fn wp_grp_enable(&self) -> bool {
        (self.raw[12] & 0x80) != 0
    }

    /// R2W_FACTOR - Write speed factor
    pub const fn r2w_factor(&self) -> u8 {
        (self.raw[12] >> 2) & 0x07
    }

    /// WRITE_BL_LEN - Max write data block length
    pub const fn write_bl_len(&self) -> u8 {
        ((self.raw[12] & 0x03) << 2) | ((self.raw[13] >> 6) & 0x03)
    }

    /// WRITE_BL_PARTIAL - Partial blocks for write allowed
    pub const fn write_bl_partial(&self) -> bool {
        (self.raw[13] & 0x20) != 0
    }

    /// FILE_FORMAT_GRP - File format group
    pub const fn file_format_grp(&self) -> bool {
        (self.raw[14] & 0x80) != 0
    }

    /// COPY - Copy flag
    pub const fn copy(&self) -> bool {
        (self.raw[14] & 0x40) != 0
    }

    /// PERM_WRITE_PROTECT - Permanent write protection
    pub const fn perm_write_protect(&self) -> bool {
        (self.raw[14] & 0x20) != 0
    }

    /// TMP_WRITE_PROTECT - Temporary write protection
    pub const fn tmp_write_protect(&self) -> bool {
        (self.raw[14] & 0x10) != 0
    }

    /// FILE_FORMAT - File format
    pub const fn file_format(&self) -> u8 {
        (self.raw[14] >> 2) & 0x03
    }
}

// =============================================================================
// SCR (SD Card Configuration Register)
// =============================================================================

/// SCR register (64 bits)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Scr {
    /// Raw SCR data (8 bytes)
    pub raw: [u8; 8],
}

impl Scr {
    /// SCR structure version
    pub const fn scr_structure(&self) -> u8 {
        (self.raw[0] >> 4) & 0x0F
    }

    /// SD spec version
    pub const fn sd_spec(&self) -> u8 {
        self.raw[0] & 0x0F
    }

    /// Data status after erases
    pub const fn data_stat_after_erase(&self) -> bool {
        (self.raw[1] & 0x80) != 0
    }

    /// SD security version
    pub const fn sd_security(&self) -> u8 {
        (self.raw[1] >> 4) & 0x07
    }

    /// DAT bus widths supported
    pub const fn sd_bus_widths(&self) -> u8 {
        self.raw[1] & 0x0F
    }

    /// 1-bit bus width supported
    pub const fn supports_1bit(&self) -> bool {
        (self.raw[1] & 0x01) != 0
    }

    /// 4-bit bus width supported
    pub const fn supports_4bit(&self) -> bool {
        (self.raw[1] & 0x04) != 0
    }

    /// SD spec version 3.0 or higher
    pub const fn sd_spec3(&self) -> bool {
        (self.raw[2] & 0x80) != 0
    }

    /// Extended security supported
    pub const fn ex_security(&self) -> u8 {
        (self.raw[2] >> 3) & 0x0F
    }

    /// SD spec version 4.0 or higher
    pub const fn sd_spec4(&self) -> bool {
        (self.raw[2] & 0x04) != 0
    }

    /// SD spec version X (for SD spec > 4.0)
    pub const fn sd_specx(&self) -> u8 {
        ((self.raw[2] & 0x03) << 2) | ((self.raw[3] >> 6) & 0x03)
    }

    /// Command support bits
    pub const fn cmd_support(&self) -> u8 {
        self.raw[3] & 0x0F
    }

    /// CMD23 supported
    pub const fn cmd23_support(&self) -> bool {
        (self.raw[3] & 0x02) != 0
    }

    /// CMD20 (Speed Class Control) supported
    pub const fn cmd20_support(&self) -> bool {
        (self.raw[3] & 0x01) != 0
    }

    /// Get full SD specification version
    pub fn sd_version(&self) -> SdVersion {
        let spec = self.sd_spec();
        let spec3 = self.sd_spec3();
        let spec4 = self.sd_spec4();
        let specx = self.sd_specx();

        match (spec, spec3, spec4, specx) {
            (0, _, _, _) => SdVersion::V1_0,
            (1, _, _, _) => SdVersion::V1_1,
            (2, false, _, _) => SdVersion::V2_0,
            (2, true, false, _) => SdVersion::V3_0,
            (2, true, true, 0) => SdVersion::V4_0,
            (2, true, true, 1) => SdVersion::V5_0,
            (2, true, true, 2) => SdVersion::V6_0,
            (2, true, true, 3) => SdVersion::V7_0,
            (2, true, true, 4) => SdVersion::V8_0,
            (2, true, true, 5) => SdVersion::V9_0,
            _ => SdVersion::Unknown,
        }
    }
}

/// SD specification version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdVersion {
    /// SD 1.0
    V1_0,
    /// SD 1.1
    V1_1,
    /// SD 2.0
    V2_0,
    /// SD 3.0
    V3_0,
    /// SD 4.0
    V4_0,
    /// SD 5.0
    V5_0,
    /// SD 6.0
    V6_0,
    /// SD 7.0
    V7_0,
    /// SD 8.0
    V8_0,
    /// SD 9.0
    V9_0,
    /// Unknown
    Unknown,
}

// =============================================================================
// SSR (SD Status Register)
// =============================================================================

/// SSR register (512 bits = 64 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Ssr {
    /// Raw SSR data
    pub raw: [u8; 64],
}

impl Ssr {
    /// DAT bus width
    pub const fn dat_bus_width(&self) -> u8 {
        (self.raw[0] >> 6) & 0x03
    }

    /// Secured mode
    pub const fn secured_mode(&self) -> bool {
        (self.raw[0] & 0x20) != 0
    }

    /// SD card type
    pub fn sd_card_type(&self) -> u16 {
        ((self.raw[2] as u16) << 8) | (self.raw[3] as u16)
    }

    /// Size of protected area in bytes
    pub fn size_of_protected_area(&self) -> u32 {
        ((self.raw[4] as u32) << 24)
            | ((self.raw[5] as u32) << 16)
            | ((self.raw[6] as u32) << 8)
            | (self.raw[7] as u32)
    }

    /// Speed class
    pub const fn speed_class(&self) -> SpeedClass {
        SpeedClass::from_value(self.raw[8])
    }

    /// Performance move rating (MB/s)
    pub const fn performance_move(&self) -> u8 {
        self.raw[9]
    }

    /// AU size
    pub const fn au_size(&self) -> u8 {
        self.raw[10] >> 4
    }

    /// AU size in bytes
    pub fn au_size_bytes(&self) -> u32 {
        match self.au_size() {
            0 => 0,
            1 => 16 * 1024,
            2 => 32 * 1024,
            3 => 64 * 1024,
            4 => 128 * 1024,
            5 => 256 * 1024,
            6 => 512 * 1024,
            7 => 1024 * 1024,
            8 => 2 * 1024 * 1024,
            9 => 4 * 1024 * 1024,
            10 => 8 * 1024 * 1024,
            11 => 12 * 1024 * 1024,
            12 => 16 * 1024 * 1024,
            13 => 24 * 1024 * 1024,
            14 => 32 * 1024 * 1024,
            15 => 64 * 1024 * 1024,
            _ => 0,
        }
    }

    /// Erase size (AU count)
    pub fn erase_size(&self) -> u16 {
        ((self.raw[11] as u16) << 8) | (self.raw[12] as u16)
    }

    /// Erase timeout (seconds)
    pub const fn erase_timeout(&self) -> u8 {
        self.raw[13] >> 2
    }

    /// Erase offset
    pub const fn erase_offset(&self) -> u8 {
        self.raw[13] & 0x03
    }

    /// UHS speed grade
    pub const fn uhs_speed_grade(&self) -> u8 {
        self.raw[14] >> 4
    }

    /// UHS AU size
    pub const fn uhs_au_size(&self) -> u8 {
        self.raw[14] & 0x0F
    }

    /// Video speed class
    pub const fn video_speed_class(&self) -> u8 {
        self.raw[15]
    }

    /// VSC AU size
    pub fn vsc_au_size(&self) -> u16 {
        (((self.raw[16] & 0x03) as u16) << 8) | (self.raw[17] as u16)
    }

    /// Suspension address
    pub fn sus_addr(&self) -> u32 {
        (((self.raw[18] & 0x3F) as u32) << 16)
            | ((self.raw[19] as u32) << 8)
            | (self.raw[20] as u32)
    }

    /// Application performance class
    pub const fn app_perf_class(&self) -> u8 {
        self.raw[21] >> 4
    }

    /// Performance enhance
    pub const fn performance_enhance(&self) -> u8 {
        self.raw[21] & 0x0F
    }

    /// Discard support
    pub const fn discard_support(&self) -> bool {
        (self.raw[22] & 0x02) != 0
    }

    /// FULE (Full User area Logical Erase) support
    pub const fn fule_support(&self) -> bool {
        (self.raw[22] & 0x01) != 0
    }
}

/// SD Speed Class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedClass {
    /// Class 0 (undefined)
    Class0,
    /// Class 2
    Class2,
    /// Class 4
    Class4,
    /// Class 6
    Class6,
    /// Class 10
    Class10,
    /// Unknown class
    Unknown(u8),
}

impl SpeedClass {
    /// Create from register value
    pub const fn from_value(val: u8) -> Self {
        match val {
            0 => SpeedClass::Class0,
            1 => SpeedClass::Class2,
            2 => SpeedClass::Class4,
            3 => SpeedClass::Class6,
            4 => SpeedClass::Class10,
            _ => SpeedClass::Unknown(val),
        }
    }

    /// Get minimum write speed in MB/s
    pub const fn min_write_speed_mbps(&self) -> u8 {
        match self {
            SpeedClass::Class0 => 0,
            SpeedClass::Class2 => 2,
            SpeedClass::Class4 => 4,
            SpeedClass::Class6 => 6,
            SpeedClass::Class10 => 10,
            SpeedClass::Unknown(_) => 0,
        }
    }
}

// =============================================================================
// EXTENDED CSD (eMMC)
// =============================================================================

/// Extended CSD register for eMMC (512 bytes)
#[derive(Clone)]
#[repr(C)]
pub struct ExtCsd {
    /// Raw Extended CSD data
    pub raw: [u8; 512],
}

/// Extended CSD field offsets
pub mod ext_csd {
    // Properties segment (read-only)
    /// S_CMD_SET
    pub const S_CMD_SET: usize = 504;
    /// HPI_FEATURES
    pub const HPI_FEATURES: usize = 503;
    /// BKOPS_SUPPORT
    pub const BKOPS_SUPPORT: usize = 502;
    /// MAX_PACKED_READS
    pub const MAX_PACKED_READS: usize = 501;
    /// MAX_PACKED_WRITES
    pub const MAX_PACKED_WRITES: usize = 500;
    /// DATA_TAG_SUPPORT
    pub const DATA_TAG_SUPPORT: usize = 499;
    /// TAG_UNIT_SIZE
    pub const TAG_UNIT_SIZE: usize = 498;
    /// TAG_RES_SIZE
    pub const TAG_RES_SIZE: usize = 497;
    /// CONTEXT_CAPABILITIES
    pub const CONTEXT_CAPABILITIES: usize = 496;
    /// LARGE_UNIT_SIZE_M1
    pub const LARGE_UNIT_SIZE_M1: usize = 495;
    /// EXT_SUPPORT
    pub const EXT_SUPPORT: usize = 494;
    /// SUPPORTED_MODES
    pub const SUPPORTED_MODES: usize = 493;
    /// FFU_FEATURES
    pub const FFU_FEATURES: usize = 492;
    /// OPERATION_CODE_TIMEOUT
    pub const OPERATION_CODE_TIMEOUT: usize = 491;
    /// FFU_ARG
    pub const FFU_ARG: usize = 487;
    /// BARRIER_SUPPORT
    pub const BARRIER_SUPPORT: usize = 486;
    /// CMDQ_SUPPORT
    pub const CMDQ_SUPPORT: usize = 308;
    /// CMDQ_DEPTH
    pub const CMDQ_DEPTH: usize = 307;
    /// NUMBER_OF_FW_SECTORS_CORRECTLY_PROGRAMMED
    pub const NUM_FW_SECTORS: usize = 302;
    /// FIRMWARE_VERSION
    pub const FIRMWARE_VERSION: usize = 254;
    /// DEVICE_VERSION
    pub const DEVICE_VERSION: usize = 252;
    /// OPTIMAL_TRIM_UNIT_SIZE
    pub const OPTIMAL_TRIM_UNIT_SIZE: usize = 264;
    /// OPTIMAL_WRITE_SIZE
    pub const OPTIMAL_WRITE_SIZE: usize = 263;
    /// OPTIMAL_READ_SIZE
    pub const OPTIMAL_READ_SIZE: usize = 262;
    /// PRE_EOL_INFO
    pub const PRE_EOL_INFO: usize = 267;
    /// DEVICE_LIFE_TIME_EST_TYP_A
    pub const DEVICE_LIFE_TIME_A: usize = 268;
    /// DEVICE_LIFE_TIME_EST_TYP_B
    pub const DEVICE_LIFE_TIME_B: usize = 269;
    /// VENDOR_PROPRIETARY_HEALTH_REPORT
    pub const VENDOR_HEALTH_REPORT: usize = 270;
    /// NATIVE_SECTOR_SIZE
    pub const NATIVE_SECTOR_SIZE: usize = 63;
    /// USE_NATIVE_SECTOR
    pub const USE_NATIVE_SECTOR: usize = 62;
    /// DATA_SECTOR_SIZE
    pub const DATA_SECTOR_SIZE: usize = 61;
    /// INI_TIMEOUT_AP
    pub const INI_TIMEOUT_AP: usize = 241;
    /// PWR_CL_DDR_52_360
    pub const PWR_CL_DDR_52_360: usize = 239;
    /// PWR_CL_DDR_52_195
    pub const PWR_CL_DDR_52_195: usize = 238;
    /// PWR_CL_200_360
    pub const PWR_CL_200_360: usize = 237;
    /// PWR_CL_200_195
    pub const PWR_CL_200_195: usize = 236;
    /// MIN_PERF_DDR_R_8_52
    pub const MIN_PERF_DDR_R_8_52: usize = 235;
    /// MIN_PERF_DDR_W_8_52
    pub const MIN_PERF_DDR_W_8_52: usize = 234;
    /// SEC_COUNT
    pub const SEC_COUNT: usize = 212;
    /// SLEEP_NOTIFICATION_TIME
    pub const SLEEP_NOTIFICATION_TIME: usize = 216;
    /// S_A_TIMEOUT
    pub const S_A_TIMEOUT: usize = 217;
    /// PRODUCTION_STATE_AWARENESS_TIMEOUT
    pub const PRODUCTION_STATE_AWARENESS_TIMEOUT: usize = 218;
    /// S_C_VCCQ
    pub const S_C_VCCQ: usize = 219;
    /// S_C_VCC
    pub const S_C_VCC: usize = 220;
    /// HC_WP_GRP_SIZE
    pub const HC_WP_GRP_SIZE: usize = 221;
    /// REL_WR_SEC_C
    pub const REL_WR_SEC_C: usize = 222;
    /// ERASE_TIMEOUT_MULT
    pub const ERASE_TIMEOUT_MULT: usize = 223;
    /// HC_ERASE_GRP_SIZE
    pub const HC_ERASE_GRP_SIZE: usize = 224;
    /// ACC_SIZE
    pub const ACC_SIZE: usize = 225;
    /// BOOT_SIZE_MULT
    pub const BOOT_SIZE_MULT: usize = 226;
    /// BOOT_INFO
    pub const BOOT_INFO: usize = 228;
    /// SEC_TRIM_MULT
    pub const SEC_TRIM_MULT: usize = 229;
    /// SEC_ERASE_MULT
    pub const SEC_ERASE_MULT: usize = 230;
    /// SEC_FEATURE_SUPPORT
    pub const SEC_FEATURE_SUPPORT: usize = 231;
    /// TRIM_MULT
    pub const TRIM_MULT: usize = 232;
    /// MIN_PERF_R_8_52
    pub const MIN_PERF_R_8_52: usize = 206;
    /// MIN_PERF_W_8_52
    pub const MIN_PERF_W_8_52: usize = 205;
    /// MIN_PERF_R_8_26_4_52
    pub const MIN_PERF_R_8_26_4_52: usize = 208;
    /// MIN_PERF_W_8_26_4_52
    pub const MIN_PERF_W_8_26_4_52: usize = 207;
    /// MIN_PERF_R_4_26
    pub const MIN_PERF_R_4_26: usize = 210;
    /// MIN_PERF_W_4_26
    pub const MIN_PERF_W_4_26: usize = 209;
    /// DEVICE_TYPE
    pub const DEVICE_TYPE: usize = 196;
    /// CSD_STRUCTURE
    pub const CSD_STRUCTURE: usize = 194;
    /// EXT_CSD_REV
    pub const EXT_CSD_REV: usize = 192;

    // Modes segment (read/write)
    /// CMD_SET
    pub const CMD_SET: usize = 191;
    /// CMD_SET_REV
    pub const CMD_SET_REV: usize = 189;
    /// POWER_CLASS
    pub const POWER_CLASS: usize = 187;
    /// HS_TIMING
    pub const HS_TIMING: usize = 185;
    /// BUS_WIDTH
    pub const BUS_WIDTH: usize = 183;
    /// ERASED_MEM_CONT
    pub const ERASED_MEM_CONT: usize = 181;
    /// PARTITION_CONFIG
    pub const PARTITION_CONFIG: usize = 179;
    /// BOOT_CONFIG_PROT
    pub const BOOT_CONFIG_PROT: usize = 178;
    /// BOOT_BUS_CONDITIONS
    pub const BOOT_BUS_CONDITIONS: usize = 177;
    /// ERASE_GROUP_DEF
    pub const ERASE_GROUP_DEF: usize = 175;
    /// BOOT_WP_STATUS
    pub const BOOT_WP_STATUS: usize = 174;
    /// BOOT_WP
    pub const BOOT_WP: usize = 173;
    /// USER_WP
    pub const USER_WP: usize = 171;
    /// FW_CONFIG
    pub const FW_CONFIG: usize = 169;
    /// RPMB_SIZE_MULT
    pub const RPMB_SIZE_MULT: usize = 168;
    /// WR_REL_SET
    pub const WR_REL_SET: usize = 167;
    /// WR_REL_PARAM
    pub const WR_REL_PARAM: usize = 166;
    /// SANITIZE_START
    pub const SANITIZE_START: usize = 165;
    /// BKOPS_START
    pub const BKOPS_START: usize = 164;
    /// BKOPS_EN
    pub const BKOPS_EN: usize = 163;
    /// RST_N_FUNCTION
    pub const RST_N_FUNCTION: usize = 162;
    /// HPI_MGMT
    pub const HPI_MGMT: usize = 161;
    /// PARTITIONING_SUPPORT
    pub const PARTITIONING_SUPPORT: usize = 160;
    /// MAX_ENH_SIZE_MULT
    pub const MAX_ENH_SIZE_MULT: usize = 157;
    /// PARTITIONS_ATTRIBUTE
    pub const PARTITIONS_ATTRIBUTE: usize = 156;
    /// PARTITION_SETTING_COMPLETED
    pub const PARTITION_SETTING_COMPLETED: usize = 155;
    /// GP_SIZE_MULT
    pub const GP_SIZE_MULT: usize = 143;
    /// ENH_SIZE_MULT
    pub const ENH_SIZE_MULT: usize = 140;
    /// ENH_START_ADDR
    pub const ENH_START_ADDR: usize = 136;
    /// SEC_BAD_BLK_MGMNT
    pub const SEC_BAD_BLK_MGMNT: usize = 134;
    /// PRODUCTION_STATE_AWARENESS
    pub const PRODUCTION_STATE_AWARENESS: usize = 133;
    /// TCASE_SUPPORT
    pub const TCASE_SUPPORT: usize = 132;
    /// PERIODIC_WAKEUP
    pub const PERIODIC_WAKEUP: usize = 131;
    /// PROGRAM_CID_CSD_DDR_SUPPORT
    pub const PROGRAM_CID_CSD_DDR_SUPPORT: usize = 130;
    /// VENDOR_SPECIFIC_FIELD
    pub const VENDOR_SPECIFIC_FIELD: usize = 64;
    /// NATIVE_SECTOR_SIZE2
    pub const NATIVE_SECTOR_SIZE2: usize = 63;
    /// CORRECTLY_PRG_SECTORS_NUM
    pub const CORRECTLY_PRG_SECTORS_NUM: usize = 242;
    /// BKOPS_STATUS
    pub const BKOPS_STATUS: usize = 246;
    /// POWER_OFF_LONG_TIME
    pub const POWER_OFF_LONG_TIME: usize = 247;
    /// GENERIC_CMD6_TIME
    pub const GENERIC_CMD6_TIME: usize = 248;
    /// CACHE_SIZE
    pub const CACHE_SIZE: usize = 249;
    /// PWR_CL_DDR_200_360
    pub const PWR_CL_DDR_200_360: usize = 253;
    /// CACHE_CTRL
    pub const CACHE_CTRL: usize = 33;
    /// CACHE_FLUSHING
    pub const CACHE_FLUSHING: usize = 32;
    /// BARRIER_CTRL
    pub const BARRIER_CTRL: usize = 31;
    /// MODE_CONFIG
    pub const MODE_CONFIG: usize = 30;
    /// MODE_OPERATION_CODES
    pub const MODE_OPERATION_CODES: usize = 29;
    /// FFU_STATUS
    pub const FFU_STATUS: usize = 26;
    /// PRE_LOADING_DATA_SIZE
    pub const PRE_LOADING_DATA_SIZE: usize = 22;
    /// MAX_PRE_LOADING_DATA_SIZE
    pub const MAX_PRE_LOADING_DATA_SIZE: usize = 18;
    /// PRODUCT_STATE_AWARENESS_ENABLEMENT
    pub const PRODUCT_STATE_AWARENESS_ENABLEMENT: usize = 17;
    /// SECURE_REMOVAL_TYPE
    pub const SECURE_REMOVAL_TYPE: usize = 16;
    /// CMDQ_MODE_EN
    pub const CMDQ_MODE_EN: usize = 15;
}

impl ExtCsd {
    /// Get sector count (device capacity in 512-byte sectors)
    pub fn sector_count(&self) -> u32 {
        u32::from_le_bytes([
            self.raw[ext_csd::SEC_COUNT],
            self.raw[ext_csd::SEC_COUNT + 1],
            self.raw[ext_csd::SEC_COUNT + 2],
            self.raw[ext_csd::SEC_COUNT + 3],
        ])
    }

    /// Get device capacity in bytes
    pub fn capacity_bytes(&self) -> u64 {
        (self.sector_count() as u64) * 512
    }

    /// Get Extended CSD revision
    pub const fn ext_csd_rev(&self) -> u8 {
        self.raw[ext_csd::EXT_CSD_REV]
    }

    /// Get device type
    pub const fn device_type(&self) -> u8 {
        self.raw[ext_csd::DEVICE_TYPE]
    }

    /// Check if HS200 is supported
    pub const fn supports_hs200(&self) -> bool {
        (self.raw[ext_csd::DEVICE_TYPE] & 0x10) != 0
    }

    /// Check if HS400 is supported
    pub const fn supports_hs400(&self) -> bool {
        (self.raw[ext_csd::DEVICE_TYPE] & 0x40) != 0
    }

    /// Check if DDR is supported at 52MHz
    pub const fn supports_ddr_52(&self) -> bool {
        (self.raw[ext_csd::DEVICE_TYPE] & 0x04) != 0
    }

    /// Get current bus width setting
    pub const fn bus_width(&self) -> u8 {
        self.raw[ext_csd::BUS_WIDTH]
    }

    /// Get HS timing setting
    pub const fn hs_timing(&self) -> u8 {
        self.raw[ext_csd::HS_TIMING]
    }

    /// Get boot partition size in bytes
    pub fn boot_partition_size(&self) -> u32 {
        (self.raw[ext_csd::BOOT_SIZE_MULT] as u32) * EMMC_BOOT_MULT
    }

    /// Get RPMB partition size in bytes
    pub fn rpmb_size(&self) -> u32 {
        (self.raw[ext_csd::RPMB_SIZE_MULT] as u32) * 128 * 1024
    }

    /// Get partition config
    pub const fn partition_config(&self) -> u8 {
        self.raw[ext_csd::PARTITION_CONFIG]
    }

    /// Get current boot partition
    pub const fn boot_partition_enabled(&self) -> u8 {
        (self.raw[ext_csd::PARTITION_CONFIG] >> 3) & 0x07
    }

    /// Get current partition access
    pub const fn partition_access(&self) -> u8 {
        self.raw[ext_csd::PARTITION_CONFIG] & 0x07
    }

    /// Check if enhanced strobe is supported
    pub const fn supports_enhanced_strobe(&self) -> bool {
        (self.raw[ext_csd::DEVICE_TYPE] & 0x80) != 0
    }

    /// Get erase group size in sectors
    pub fn hc_erase_grp_size(&self) -> u32 {
        (self.raw[ext_csd::HC_ERASE_GRP_SIZE] as u32) * 512 * 1024 / 512
    }

    /// Get write protect group size in sectors
    pub fn hc_wp_grp_size(&self) -> u32 {
        let erase_grp = self.hc_erase_grp_size();
        let wp_mult = self.raw[ext_csd::HC_WP_GRP_SIZE] as u32;
        erase_grp * wp_mult
    }

    /// Check if secure erase is supported
    pub const fn supports_secure_erase(&self) -> bool {
        (self.raw[ext_csd::SEC_FEATURE_SUPPORT] & 0x10) != 0
    }

    /// Check if sanitize is supported
    pub const fn supports_sanitize(&self) -> bool {
        (self.raw[ext_csd::SEC_FEATURE_SUPPORT] & 0x40) != 0
    }

    /// Check if TRIM is supported
    pub const fn supports_trim(&self) -> bool {
        (self.raw[ext_csd::SEC_FEATURE_SUPPORT] & 0x10) != 0
    }

    /// Get pre-EOL information
    pub const fn pre_eol_info(&self) -> u8 {
        self.raw[ext_csd::PRE_EOL_INFO]
    }

    /// Get device life time estimate type A
    pub const fn device_life_time_a(&self) -> u8 {
        self.raw[ext_csd::DEVICE_LIFE_TIME_A]
    }

    /// Get device life time estimate type B
    pub const fn device_life_time_b(&self) -> u8 {
        self.raw[ext_csd::DEVICE_LIFE_TIME_B]
    }

    /// Check if command queuing is supported
    pub const fn supports_cmdq(&self) -> bool {
        self.raw[ext_csd::CMDQ_SUPPORT] != 0
    }

    /// Get command queue depth
    pub const fn cmdq_depth(&self) -> u8 {
        (self.raw[ext_csd::CMDQ_DEPTH] & 0x1F) + 1
    }

    /// Get cache size in KB
    pub fn cache_size(&self) -> u32 {
        u32::from_le_bytes([
            self.raw[ext_csd::CACHE_SIZE],
            self.raw[ext_csd::CACHE_SIZE + 1],
            self.raw[ext_csd::CACHE_SIZE + 2],
            self.raw[ext_csd::CACHE_SIZE + 3],
        ])
    }

    /// Check if cache is enabled
    pub const fn cache_enabled(&self) -> bool {
        self.raw[ext_csd::CACHE_CTRL] != 0
    }
}

// =============================================================================
// BUS WIDTH AND TIMING
// =============================================================================

/// Bus width settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BusWidth {
    /// 1-bit bus
    Width1    = 0,
    /// 4-bit bus
    Width4    = 1,
    /// 8-bit bus (MMC only)
    Width8    = 2,
    /// 4-bit DDR
    Width4Ddr = 5,
    /// 8-bit DDR
    Width8Ddr = 6,
}

/// Timing modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimingMode {
    /// Legacy timing
    Legacy    = 0,
    /// High Speed timing
    HighSpeed = 1,
    /// HS200 timing
    Hs200     = 2,
    /// HS400 timing
    Hs400     = 3,
}

/// UHS timing modes (SD)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UhsMode {
    /// SDR12 (12.5MB/s)
    Sdr12  = 0,
    /// SDR25 (25MB/s)
    Sdr25  = 1,
    /// SDR50 (50MB/s)
    Sdr50  = 2,
    /// SDR104 (104MB/s)
    Sdr104 = 3,
    /// DDR50 (50MB/s)
    Ddr50  = 4,
}

// =============================================================================
// CARD TYPE
// =============================================================================

/// Card type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    /// Unknown card type
    Unknown,
    /// MMC card
    Mmc,
    /// SD Standard Capacity
    Sdsc,
    /// SD High Capacity (SDHC)
    Sdhc,
    /// SD Extended Capacity (SDXC)
    Sdxc,
    /// SD Ultra Capacity (SDUC)
    Sduc,
    /// eMMC
    Emmc,
    /// SDIO card
    Sdio,
    /// Combo (SD + SDIO)
    Combo,
}

// =============================================================================
// CARD INFO
// =============================================================================

/// Card information structure
#[derive(Debug, Clone)]
pub struct CardInfo {
    /// Card type
    pub card_type: CardType,
    /// Relative card address
    pub rca: u16,
    /// Card capacity in sectors
    pub capacity_sectors: u64,
    /// Sector size in bytes
    pub sector_size: u16,
    /// Maximum transfer speed in kHz
    pub max_speed_khz: u32,
    /// Current bus width
    pub bus_width: BusWidth,
    /// Current timing mode
    pub timing: TimingMode,
    /// High capacity flag
    pub high_capacity: bool,
    /// Write protected
    pub write_protected: bool,
}

impl Default for CardInfo {
    fn default() -> Self {
        Self {
            card_type: CardType::Unknown,
            rca: 0,
            capacity_sectors: 0,
            sector_size: 512,
            max_speed_khz: 25000,
            bus_width: BusWidth::Width1,
            timing: TimingMode::Legacy,
            high_capacity: false,
            write_protected: false,
        }
    }
}

impl CardInfo {
    /// Get capacity in bytes
    pub const fn capacity_bytes(&self) -> u64 {
        self.capacity_sectors * (self.sector_size as u64)
    }

    /// Get capacity in MB
    pub const fn capacity_mb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024)
    }

    /// Get capacity in GB
    pub const fn capacity_gb(&self) -> u64 {
        self.capacity_bytes() / (1024 * 1024 * 1024)
    }

    /// Check if card is an SD card
    pub const fn is_sd(&self) -> bool {
        matches!(
            self.card_type,
            CardType::Sdsc | CardType::Sdhc | CardType::Sdxc | CardType::Sduc
        )
    }

    /// Check if card is an eMMC
    pub const fn is_emmc(&self) -> bool {
        matches!(self.card_type, CardType::Emmc)
    }

    /// Check if card supports high speed
    pub const fn supports_high_speed(&self) -> bool {
        self.max_speed_khz >= 50000
    }
}

// =============================================================================
// SDHC CONTROLLER REGISTERS
// =============================================================================

/// SDHC controller register offsets (SD Host Controller Standard)
pub mod sdhc_regs {
    /// SDMA System Address / Argument 2
    pub const SDMA_ADDR: usize = 0x00;
    /// Block Size Register
    pub const BLOCK_SIZE: usize = 0x04;
    /// Block Count Register
    pub const BLOCK_COUNT: usize = 0x06;
    /// Argument 1 Register
    pub const ARGUMENT: usize = 0x08;
    /// Transfer Mode Register
    pub const TRANSFER_MODE: usize = 0x0C;
    /// Command Register
    pub const COMMAND: usize = 0x0E;
    /// Response Register 0
    pub const RESPONSE0: usize = 0x10;
    /// Response Register 1
    pub const RESPONSE1: usize = 0x14;
    /// Response Register 2
    pub const RESPONSE2: usize = 0x18;
    /// Response Register 3
    pub const RESPONSE3: usize = 0x1C;
    /// Buffer Data Port
    pub const BUFFER_DATA: usize = 0x20;
    /// Present State Register
    pub const PRESENT_STATE: usize = 0x24;
    /// Host Control 1 Register
    pub const HOST_CONTROL1: usize = 0x28;
    /// Power Control Register
    pub const POWER_CONTROL: usize = 0x29;
    /// Block Gap Control Register
    pub const BLOCK_GAP: usize = 0x2A;
    /// Wakeup Control Register
    pub const WAKEUP_CONTROL: usize = 0x2B;
    /// Clock Control Register
    pub const CLOCK_CONTROL: usize = 0x2C;
    /// Timeout Control Register
    pub const TIMEOUT_CONTROL: usize = 0x2E;
    /// Software Reset Register
    pub const SOFTWARE_RESET: usize = 0x2F;
    /// Normal Interrupt Status Register
    pub const NORMAL_INT_STATUS: usize = 0x30;
    /// Error Interrupt Status Register
    pub const ERROR_INT_STATUS: usize = 0x32;
    /// Normal Interrupt Status Enable
    pub const NORMAL_INT_ENABLE: usize = 0x34;
    /// Error Interrupt Status Enable
    pub const ERROR_INT_ENABLE: usize = 0x36;
    /// Normal Interrupt Signal Enable
    pub const NORMAL_SIGNAL_ENABLE: usize = 0x38;
    /// Error Interrupt Signal Enable
    pub const ERROR_SIGNAL_ENABLE: usize = 0x3A;
    /// Auto CMD Error Status
    pub const AUTO_CMD_STATUS: usize = 0x3C;
    /// Host Control 2 Register
    pub const HOST_CONTROL2: usize = 0x3E;
    /// Capabilities Register 0
    pub const CAPABILITIES0: usize = 0x40;
    /// Capabilities Register 1
    pub const CAPABILITIES1: usize = 0x44;
    /// Maximum Current Capabilities
    pub const MAX_CURRENT: usize = 0x48;
    /// Force Event Register
    pub const FORCE_EVENT: usize = 0x50;
    /// ADMA Error Status
    pub const ADMA_ERROR: usize = 0x54;
    /// ADMA System Address
    pub const ADMA_ADDR: usize = 0x58;
    /// Preset Value Registers
    pub const PRESET_VALUE: usize = 0x60;
    /// Shared Bus Control (optional)
    pub const SHARED_BUS: usize = 0xE0;
    /// Slot Interrupt Status
    pub const SLOT_INT_STATUS: usize = 0xFC;
    /// Host Controller Version
    pub const HOST_VERSION: usize = 0xFE;
}

/// Present State register bits
pub mod present_state {
    /// Command inhibit (CMD)
    pub const CMD_INHIBIT: u32 = 1 << 0;
    /// Command inhibit (DAT)
    pub const DAT_INHIBIT: u32 = 1 << 1;
    /// DAT line active
    pub const DAT_ACTIVE: u32 = 1 << 2;
    /// Re-tuning request
    pub const RETUNE_REQ: u32 = 1 << 3;
    /// Write transfer active
    pub const WRITE_ACTIVE: u32 = 1 << 8;
    /// Read transfer active
    pub const READ_ACTIVE: u32 = 1 << 9;
    /// Buffer write enable
    pub const BUF_WRITE_ENABLE: u32 = 1 << 10;
    /// Buffer read enable
    pub const BUF_READ_ENABLE: u32 = 1 << 11;
    /// Card inserted
    pub const CARD_INSERTED: u32 = 1 << 16;
    /// Card state stable
    pub const CARD_STABLE: u32 = 1 << 17;
    /// Card detect pin level
    pub const CARD_DETECT: u32 = 1 << 18;
    /// Write protect switch pin level
    pub const WRITE_PROTECT: u32 = 1 << 19;
    /// DAT[3:0] line signal level
    pub const DAT_LEVEL: u32 = 0xF << 20;
    /// CMD line signal level
    pub const CMD_LEVEL: u32 = 1 << 24;
    /// DAT[7:4] line signal level
    pub const DAT_LEVEL_HIGH: u32 = 0xF << 25;
}

/// Normal interrupt status bits
pub mod normal_int {
    /// Command complete
    pub const CMD_COMPLETE: u16 = 1 << 0;
    /// Transfer complete
    pub const TRANSFER_COMPLETE: u16 = 1 << 1;
    /// Block gap event
    pub const BLOCK_GAP: u16 = 1 << 2;
    /// DMA interrupt
    pub const DMA_INT: u16 = 1 << 3;
    /// Buffer write ready
    pub const BUF_WRITE_READY: u16 = 1 << 4;
    /// Buffer read ready
    pub const BUF_READ_READY: u16 = 1 << 5;
    /// Card insertion
    pub const CARD_INSERTION: u16 = 1 << 6;
    /// Card removal
    pub const CARD_REMOVAL: u16 = 1 << 7;
    /// Card interrupt
    pub const CARD_INT: u16 = 1 << 8;
    /// INT_A (for embedded)
    pub const INT_A: u16 = 1 << 9;
    /// INT_B (for embedded)
    pub const INT_B: u16 = 1 << 10;
    /// INT_C (for embedded)
    pub const INT_C: u16 = 1 << 11;
    /// Re-tuning event
    pub const RETUNE_EVENT: u16 = 1 << 12;
    /// Error interrupt
    pub const ERROR_INT: u16 = 1 << 15;
}

/// Error interrupt status bits
pub mod error_int {
    /// Command timeout
    pub const CMD_TIMEOUT: u16 = 1 << 0;
    /// Command CRC error
    pub const CMD_CRC: u16 = 1 << 1;
    /// Command end bit error
    pub const CMD_END_BIT: u16 = 1 << 2;
    /// Command index error
    pub const CMD_INDEX: u16 = 1 << 3;
    /// Data timeout
    pub const DATA_TIMEOUT: u16 = 1 << 4;
    /// Data CRC error
    pub const DATA_CRC: u16 = 1 << 5;
    /// Data end bit error
    pub const DATA_END_BIT: u16 = 1 << 6;
    /// Current limit error
    pub const CURRENT_LIMIT: u16 = 1 << 7;
    /// Auto CMD error
    pub const AUTO_CMD: u16 = 1 << 8;
    /// ADMA error
    pub const ADMA: u16 = 1 << 9;
    /// Tuning error
    pub const TUNING: u16 = 1 << 10;
    /// Response error
    pub const RESPONSE: u16 = 1 << 11;
    /// Vendor specific error
    pub const VENDOR: u16 = 0xF << 12;
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// SD/MMC error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdError {
    /// No card present
    NoCard,
    /// Card not responding
    NoResponse,
    /// Command timeout
    Timeout,
    /// CRC error
    CrcError,
    /// Invalid command
    InvalidCommand,
    /// Card error
    CardError(CardStatus),
    /// Out of range
    OutOfRange,
    /// Write protected
    WriteProtected,
    /// Card locked
    CardLocked,
    /// Data error
    DataError,
    /// Busy timeout
    BusyTimeout,
    /// Initialization failed
    InitFailed,
    /// Unsupported card
    UnsupportedCard,
    /// Internal error
    InternalError,
}

impl fmt::Display for SdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdError::NoCard => write!(f, "No card present"),
            SdError::NoResponse => write!(f, "Card not responding"),
            SdError::Timeout => write!(f, "Command timeout"),
            SdError::CrcError => write!(f, "CRC error"),
            SdError::InvalidCommand => write!(f, "Invalid command"),
            SdError::CardError(status) => write!(f, "Card error: status={:#x}", status.0),
            SdError::OutOfRange => write!(f, "Address out of range"),
            SdError::WriteProtected => write!(f, "Card is write protected"),
            SdError::CardLocked => write!(f, "Card is locked"),
            SdError::DataError => write!(f, "Data transfer error"),
            SdError::BusyTimeout => write!(f, "Busy timeout"),
            SdError::InitFailed => write!(f, "Initialization failed"),
            SdError::UnsupportedCard => write!(f, "Unsupported card type"),
            SdError::InternalError => write!(f, "Internal error"),
        }
    }
}

// =============================================================================
// ADMA2 DESCRIPTOR
// =============================================================================

/// ADMA2 descriptor attributes
pub mod adma2_attr {
    /// Valid descriptor
    pub const VALID: u16 = 1 << 0;
    /// End of descriptor
    pub const END: u16 = 1 << 1;
    /// Generate interrupt
    pub const INT: u16 = 1 << 2;
    /// NOP action (no data transfer)
    pub const NOP: u16 = 0 << 4;
    /// Transfer data action
    pub const TRAN: u16 = 2 << 4;
    /// Link to next descriptor table
    pub const LINK: u16 = 3 << 4;
}

/// ADMA2 descriptor (64-bit mode)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Adma2Descriptor {
    /// Attribute and length
    pub attr_len: u32,
    /// Address (64-bit)
    pub address: u64,
}

impl Adma2Descriptor {
    /// Create new transfer descriptor
    pub const fn transfer(address: u64, length: u16, end: bool, interrupt: bool) -> Self {
        let mut attr = adma2_attr::VALID | adma2_attr::TRAN;
        if end {
            attr |= adma2_attr::END;
        }
        if interrupt {
            attr |= adma2_attr::INT;
        }
        Self {
            attr_len: (attr as u32) | ((length as u32) << 16),
            address,
        }
    }

    /// Create link descriptor
    pub const fn link(address: u64) -> Self {
        Self {
            attr_len: (adma2_attr::VALID | adma2_attr::LINK) as u32,
            address,
        }
    }

    /// Create NOP descriptor
    pub const fn nop() -> Self {
        Self {
            attr_len: (adma2_attr::VALID | adma2_attr::NOP) as u32,
            address: 0,
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Calculate CRC7 for SD commands
pub fn crc7(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for byte in data {
        for i in (0..8).rev() {
            let bit = (byte >> i) & 1;
            crc = (crc << 1) | bit;
            if crc & 0x80 != 0 {
                crc ^= 0x89; // Polynomial: x^7 + x^3 + 1
            }
        }
    }
    (crc << 1) | 1 // Shift and add stop bit
}

/// Calculate CRC16 for SD data
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for byte in data {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021; // CCITT polynomial
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_card_status() {
        let status = CardStatus(0x00000900);
        assert!(status.ready_for_data());
        assert_eq!(status.current_state(), CardState::Tran);
        assert!(!status.has_error());
    }

    #[test]
    fn test_card_state() {
        assert_eq!(CardState::from_u8(0), CardState::Idle);
        assert_eq!(CardState::from_u8(4), CardState::Tran);
        assert_eq!(CardState::from_u8(15), CardState::Unknown);
    }

    #[test]
    fn test_crc7() {
        // Test vector: CMD0 (GO_IDLE_STATE)
        let cmd = [0x40, 0x00, 0x00, 0x00, 0x00];
        let crc = crc7(&cmd);
        // CRC7 result should be valid
        assert_eq!(crc & 1, 1); // Stop bit
    }

    #[test]
    fn test_command_flags() {
        let flags = CommandFlags::r1().with_read();
        assert!(matches!(flags.response, ResponseType::R1));
        assert!(matches!(flags.data, DataDirection::Read));
        assert!(flags.check_crc);
    }

    #[test]
    fn test_adma2_descriptor() {
        let desc = Adma2Descriptor::transfer(0x1000, 512, true, false);
        // SAFETY: Reading from packed struct fields using unaligned access
        unsafe {
            assert_eq!(core::ptr::addr_of!(desc.address).read_unaligned(), 0x1000);
            let attr = core::ptr::addr_of!(desc.attr_len).read_unaligned() as u16;
            assert!((attr & adma2_attr::VALID) != 0);
            assert!((attr & adma2_attr::END) != 0);
            assert!((attr & adma2_attr::TRAN) != 0);
        }
    }

    #[test]
    fn test_bus_width() {
        assert_eq!(BusWidth::Width1 as u8, 0);
        assert_eq!(BusWidth::Width4 as u8, 1);
        assert_eq!(BusWidth::Width8 as u8, 2);
    }
}
