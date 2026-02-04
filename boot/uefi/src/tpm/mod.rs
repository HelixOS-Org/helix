//! TPM (Trusted Platform Module) Support for Helix UEFI Bootloader
//!
//! This module provides comprehensive TPM 1.2 and TPM 2.0 support for measured
//! boot, attestation, and secure storage operations in the UEFI environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         TPM Protocol Stack                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Applications   │  Attestation  │  Sealing  │  Key Management          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  TPM Commands   │  PCR Extend  │  Hash  │  NVRAM  │  Random            │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  TPM Interface  │  TIS (TPM Interface Spec)  │  CRB (Command Response)  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Hardware       │  Discrete TPM  │  Firmware TPM (fTPM)                │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - TPM 1.2 and TPM 2.0 support
//! - PCR (Platform Configuration Register) operations
//! - Measured boot event logging
//! - Cryptographic operations (hash, HMAC, RSA, ECC)
//! - Secure storage (NVRAM, sealing/unsealing)
//! - Random number generation
//! - Key management
//! - Attestation support

#![no_std]

use core::fmt;

// =============================================================================
// TPM CONSTANTS
// =============================================================================

/// TPM TIS base address (memory-mapped)
pub const TPM_TIS_BASE: u64 = 0xFED40000;

/// TPM TIS register size
pub const TPM_TIS_SIZE: usize = 0x5000;

/// Maximum TPM command/response size
pub const TPM_MAX_COMMAND_SIZE: usize = 4096;

/// Maximum number of PCRs
pub const TPM_MAX_PCRS: usize = 24;

/// SHA-1 digest size
pub const SHA1_DIGEST_SIZE: usize = 20;

/// SHA-256 digest size
pub const SHA256_DIGEST_SIZE: usize = 32;

/// SHA-384 digest size
pub const SHA384_DIGEST_SIZE: usize = 48;

/// SHA-512 digest size
pub const SHA512_DIGEST_SIZE: usize = 64;

/// SM3-256 digest size
pub const SM3_256_DIGEST_SIZE: usize = 32;

// =============================================================================
// TPM VERSION
// =============================================================================

/// TPM version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmVersion {
    /// TPM 1.2
    Tpm12,
    /// TPM 2.0
    Tpm20,
    /// Unknown version
    Unknown,
}

impl TpmVersion {
    /// Get version string
    pub const fn as_str(&self) -> &'static str {
        match self {
            TpmVersion::Tpm12 => "TPM 1.2",
            TpmVersion::Tpm20 => "TPM 2.0",
            TpmVersion::Unknown => "Unknown",
        }
    }
}

impl fmt::Display for TpmVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// =============================================================================
// TPM INTERFACE TYPE
// =============================================================================

/// TPM interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmInterface {
    /// TPM Interface Specification (TIS)
    Tis,
    /// Command Response Buffer (CRB)
    Crb,
    /// FIFO interface
    Fifo,
}

// =============================================================================
// TPM TIS REGISTERS
// =============================================================================

/// TPM TIS register offsets
pub mod tis_reg {
    /// Access register
    pub const ACCESS: usize = 0x0000;
    /// Interrupt enable register
    pub const INT_ENABLE: usize = 0x0008;
    /// Interrupt vector register
    pub const INT_VECTOR: usize = 0x000C;
    /// Interrupt status register
    pub const INT_STATUS: usize = 0x0010;
    /// Interface capability register
    pub const INTF_CAPS: usize = 0x0014;
    /// Status register
    pub const STS: usize = 0x0018;
    /// Data FIFO
    pub const DATA_FIFO: usize = 0x0024;
    /// Interface ID register
    pub const INTF_ID: usize = 0x0030;
    /// xData FIFO (TPM 2.0)
    pub const XDATA_FIFO: usize = 0x0080;
    /// Device ID register
    pub const DID_VID: usize = 0x0F00;
    /// Revision ID register
    pub const RID: usize = 0x0F04;
}

/// TPM TIS access register bits
pub mod tis_access {
    /// TPM establishment
    pub const ESTABLISHMENT: u8 = 1 << 0;
    /// Request use
    pub const REQUEST_USE: u8 = 1 << 1;
    /// Pending request
    pub const PENDING_REQUEST: u8 = 1 << 2;
    /// Seize
    pub const SEIZE: u8 = 1 << 3;
    /// Been seized
    pub const BEEN_SEIZED: u8 = 1 << 4;
    /// Active locality
    pub const ACTIVE_LOCALITY: u8 = 1 << 5;
    /// TPM register valid
    pub const TPM_REG_VALID: u8 = 1 << 7;
}

/// TPM TIS status register bits
pub mod tis_sts {
    /// Response retry
    pub const RESPONSE_RETRY: u32 = 1 << 1;
    /// Self test done
    pub const SELF_TEST_DONE: u32 = 1 << 2;
    /// Expect
    pub const EXPECT: u32 = 1 << 3;
    /// Data available
    pub const DATA_AVAIL: u32 = 1 << 4;
    /// TPM go
    pub const TPM_GO: u32 = 1 << 5;
    /// Command ready
    pub const COMMAND_READY: u32 = 1 << 6;
    /// Status valid
    pub const STS_VALID: u32 = 1 << 7;
    /// Burst count (bits 8-23)
    pub const BURST_COUNT_SHIFT: u32 = 8;
    pub const BURST_COUNT_MASK: u32 = 0xFFFF;
    /// Command cancel
    pub const COMMAND_CANCEL: u32 = 1 << 24;
    /// Reset establishment
    pub const RESET_ESTABLISHMENT: u32 = 1 << 25;
    /// Family (bits 26-27)
    pub const FAMILY_SHIFT: u32 = 26;
    pub const FAMILY_MASK: u32 = 0x03;
}

// =============================================================================
// TPM CRB REGISTERS
// =============================================================================

/// TPM CRB register offsets
pub mod crb_reg {
    /// Locality state
    pub const LOC_STATE: usize = 0x0000;
    /// Locality control
    pub const LOC_CTRL: usize = 0x0008;
    /// Locality status
    pub const LOC_STS: usize = 0x000C;
    /// Interface ID
    pub const INTF_ID: usize = 0x0030;
    /// Control extension
    pub const CTRL_EXT: usize = 0x0038;
    /// Control request
    pub const CTRL_REQ: usize = 0x0040;
    /// Control status
    pub const CTRL_STS: usize = 0x0044;
    /// Control cancel
    pub const CTRL_CANCEL: usize = 0x0048;
    /// Control start
    pub const CTRL_START: usize = 0x004C;
    /// Interrupt enable
    pub const INT_ENABLE: usize = 0x0050;
    /// Interrupt status
    pub const INT_STS: usize = 0x0054;
    /// Command size
    pub const CTRL_CMD_SIZE: usize = 0x0058;
    /// Command address low
    pub const CTRL_CMD_LADDR: usize = 0x005C;
    /// Command address high
    pub const CTRL_CMD_HADDR: usize = 0x0060;
    /// Response size
    pub const CTRL_RSP_SIZE: usize = 0x0064;
    /// Response address
    pub const CTRL_RSP_ADDR: usize = 0x0068;
    /// Data buffer
    pub const DATA_BUFFER: usize = 0x0080;
}

// =============================================================================
// TPM 2.0 ALGORITHM IDS
// =============================================================================

/// TPM 2.0 algorithm identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TpmAlgorithm {
    /// Error
    Error        = 0x0000,
    /// RSA
    Rsa          = 0x0001,
    /// Triple DES
    TripleDes    = 0x0003,
    /// SHA-1
    Sha1         = 0x0004,
    /// HMAC
    Hmac         = 0x0005,
    /// AES
    Aes          = 0x0006,
    /// MGF1
    Mgf1         = 0x0007,
    /// Key derivation function 1
    KeyedHash    = 0x0008,
    /// XOR
    Xor          = 0x000A,
    /// SHA-256
    Sha256       = 0x000B,
    /// SHA-384
    Sha384       = 0x000C,
    /// SHA-512
    Sha512       = 0x000D,
    /// Null algorithm
    Null         = 0x0010,
    /// SM3-256
    Sm3_256      = 0x0012,
    /// SM4
    Sm4          = 0x0013,
    /// RSA-SSA
    RsaSsa       = 0x0014,
    /// RSA-ES
    RsaEs        = 0x0015,
    /// RSA-PSS
    RsaPss       = 0x0016,
    /// RSA-OAEP
    RsaOaep      = 0x0017,
    /// ECDSA
    Ecdsa        = 0x0018,
    /// ECDH
    Ecdh         = 0x0019,
    /// ECDAA
    Ecdaa        = 0x001A,
    /// SM2
    Sm2          = 0x001B,
    /// EC Schnorr
    EcSchnorr    = 0x001C,
    /// ECMQV
    Ecmqv        = 0x001D,
    /// KDF1 SP800-56A
    Kdf1Sp80056a = 0x0020,
    /// KDF2
    Kdf2         = 0x0021,
    /// KDF1 SP800-108
    Kdf1Sp800108 = 0x0022,
    /// ECC
    Ecc          = 0x0023,
    /// Symmetric cipher
    SymCipher    = 0x0025,
    /// Camellia
    Camellia     = 0x0026,
    /// CTR mode
    Ctr          = 0x0040,
    /// OFB mode
    Ofb          = 0x0041,
    /// CBC mode
    Cbc          = 0x0042,
    /// CFB mode
    Cfb          = 0x0043,
    /// ECB mode
    Ecb          = 0x0044,
}

impl TpmAlgorithm {
    /// Get digest size for hash algorithms
    pub const fn digest_size(&self) -> Option<usize> {
        match self {
            TpmAlgorithm::Sha1 => Some(SHA1_DIGEST_SIZE),
            TpmAlgorithm::Sha256 | TpmAlgorithm::Sm3_256 => Some(SHA256_DIGEST_SIZE),
            TpmAlgorithm::Sha384 => Some(SHA384_DIGEST_SIZE),
            TpmAlgorithm::Sha512 => Some(SHA512_DIGEST_SIZE),
            _ => None,
        }
    }

    /// Check if this is a hash algorithm
    pub const fn is_hash(&self) -> bool {
        matches!(
            self,
            TpmAlgorithm::Sha1
                | TpmAlgorithm::Sha256
                | TpmAlgorithm::Sha384
                | TpmAlgorithm::Sha512
                | TpmAlgorithm::Sm3_256
        )
    }

    /// Check if this is an asymmetric algorithm
    pub const fn is_asymmetric(&self) -> bool {
        matches!(self, TpmAlgorithm::Rsa | TpmAlgorithm::Ecc)
    }

    /// Check if this is a symmetric algorithm
    pub const fn is_symmetric(&self) -> bool {
        matches!(
            self,
            TpmAlgorithm::Aes
                | TpmAlgorithm::TripleDes
                | TpmAlgorithm::Sm4
                | TpmAlgorithm::Camellia
        )
    }

    /// Get algorithm name
    pub const fn name(&self) -> &'static str {
        match self {
            TpmAlgorithm::Error => "Error",
            TpmAlgorithm::Rsa => "RSA",
            TpmAlgorithm::TripleDes => "3DES",
            TpmAlgorithm::Sha1 => "SHA-1",
            TpmAlgorithm::Hmac => "HMAC",
            TpmAlgorithm::Aes => "AES",
            TpmAlgorithm::Mgf1 => "MGF1",
            TpmAlgorithm::KeyedHash => "Keyed Hash",
            TpmAlgorithm::Xor => "XOR",
            TpmAlgorithm::Sha256 => "SHA-256",
            TpmAlgorithm::Sha384 => "SHA-384",
            TpmAlgorithm::Sha512 => "SHA-512",
            TpmAlgorithm::Null => "Null",
            TpmAlgorithm::Sm3_256 => "SM3-256",
            TpmAlgorithm::Sm4 => "SM4",
            TpmAlgorithm::RsaSsa => "RSA-SSA",
            TpmAlgorithm::RsaEs => "RSA-ES",
            TpmAlgorithm::RsaPss => "RSA-PSS",
            TpmAlgorithm::RsaOaep => "RSA-OAEP",
            TpmAlgorithm::Ecdsa => "ECDSA",
            TpmAlgorithm::Ecdh => "ECDH",
            TpmAlgorithm::Ecdaa => "ECDAA",
            TpmAlgorithm::Sm2 => "SM2",
            TpmAlgorithm::EcSchnorr => "EC-Schnorr",
            TpmAlgorithm::Ecmqv => "ECMQV",
            TpmAlgorithm::Kdf1Sp80056a => "KDF1-SP800-56A",
            TpmAlgorithm::Kdf2 => "KDF2",
            TpmAlgorithm::Kdf1Sp800108 => "KDF1-SP800-108",
            TpmAlgorithm::Ecc => "ECC",
            TpmAlgorithm::SymCipher => "Symmetric Cipher",
            TpmAlgorithm::Camellia => "Camellia",
            TpmAlgorithm::Ctr => "CTR",
            TpmAlgorithm::Ofb => "OFB",
            TpmAlgorithm::Cbc => "CBC",
            TpmAlgorithm::Cfb => "CFB",
            TpmAlgorithm::Ecb => "ECB",
        }
    }
}

// =============================================================================
// TPM 2.0 ECC CURVES
// =============================================================================

/// TPM 2.0 ECC curve identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TpmEccCurve {
    /// None
    None     = 0x0000,
    /// NIST P-192
    NistP192 = 0x0001,
    /// NIST P-224
    NistP224 = 0x0002,
    /// NIST P-256
    NistP256 = 0x0003,
    /// NIST P-384
    NistP384 = 0x0004,
    /// NIST P-521
    NistP521 = 0x0005,
    /// Barreto-Naehrig 256-bit
    Bn256    = 0x0010,
    /// Barreto-Naehrig 638-bit
    Bn638    = 0x0011,
    /// SM2 P-256
    Sm2P256  = 0x0020,
}

impl TpmEccCurve {
    /// Get key size in bits
    pub const fn key_bits(&self) -> u16 {
        match self {
            TpmEccCurve::None => 0,
            TpmEccCurve::NistP192 => 192,
            TpmEccCurve::NistP224 => 224,
            TpmEccCurve::NistP256 | TpmEccCurve::Bn256 | TpmEccCurve::Sm2P256 => 256,
            TpmEccCurve::NistP384 => 384,
            TpmEccCurve::NistP521 => 521,
            TpmEccCurve::Bn638 => 638,
        }
    }

    /// Get curve name
    pub const fn name(&self) -> &'static str {
        match self {
            TpmEccCurve::None => "None",
            TpmEccCurve::NistP192 => "NIST P-192",
            TpmEccCurve::NistP224 => "NIST P-224",
            TpmEccCurve::NistP256 => "NIST P-256",
            TpmEccCurve::NistP384 => "NIST P-384",
            TpmEccCurve::NistP521 => "NIST P-521",
            TpmEccCurve::Bn256 => "BN-256",
            TpmEccCurve::Bn638 => "BN-638",
            TpmEccCurve::Sm2P256 => "SM2 P-256",
        }
    }
}

// =============================================================================
// TPM 2.0 COMMAND CODES
// =============================================================================

/// TPM 2.0 command codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TpmCommand {
    /// NV UndefineSpaceSpecial
    NvUndefineSpaceSpecial = 0x0000011F,
    /// Evict Control
    EvictControl        = 0x00000120,
    /// Hierarchy Control
    HierarchyControl    = 0x00000121,
    /// NV UndefineSpace
    NvUndefineSpace     = 0x00000122,
    /// Change EPS
    ChangeEps           = 0x00000124,
    /// Change PPS
    ChangePps           = 0x00000125,
    /// Clear
    Clear               = 0x00000126,
    /// Clear Control
    ClearControl        = 0x00000127,
    /// Clock Set
    ClockSet            = 0x00000128,
    /// Hierarchy Change Auth
    HierarchyChangeAuth = 0x00000129,
    /// NV Define Space
    NvDefineSpace       = 0x0000012A,
    /// PCR Allocate
    PcrAllocate         = 0x0000012B,
    /// PCR Set Auth Policy
    PcrSetAuthPolicy    = 0x0000012C,
    /// PP Commands
    PpCommands          = 0x0000012D,
    /// Set Primary Policy
    SetPrimaryPolicy    = 0x0000012E,
    /// Field Upgrade Start
    FieldUpgradeStart   = 0x0000012F,
    /// Clock Rate Adjust
    ClockRateAdjust     = 0x00000130,
    /// Create Primary
    CreatePrimary       = 0x00000131,
    /// NV Global Write Lock
    NvGlobalWriteLock   = 0x00000132,
    /// Get Command Audit Digest
    GetCommandAuditDigest = 0x00000133,
    /// NV Increment
    NvIncrement         = 0x00000134,
    /// NV Set Bits
    NvSetBits           = 0x00000135,
    /// NV Extend
    NvExtend            = 0x00000136,
    /// NV Write
    NvWrite             = 0x00000137,
    /// NV Write Lock
    NvWriteLock         = 0x00000138,
    /// Dictionary Attack Lock Reset
    DictionaryAttackLockReset = 0x00000139,
    /// Dictionary Attack Parameters
    DictionaryAttackParameters = 0x0000013A,
    /// NV Change Auth
    NvChangeAuth        = 0x0000013B,
    /// PCR Event
    PcrEvent            = 0x0000013C,
    /// PCR Reset
    PcrReset            = 0x0000013D,
    /// Sequence Complete
    SequenceComplete    = 0x0000013E,
    /// Set Algorithm Set
    SetAlgorithmSet     = 0x0000013F,
    /// Set Command Code Audit Status
    SetCommandCodeAuditStatus = 0x00000140,
    /// Field Upgrade Data
    FieldUpgradeData    = 0x00000141,
    /// Incremental Self Test
    IncrementalSelfTest = 0x00000142,
    /// Self Test
    SelfTest            = 0x00000143,
    /// Startup
    Startup             = 0x00000144,
    /// Shutdown
    Shutdown            = 0x00000145,
    /// Stir Random
    StirRandom          = 0x00000146,
    /// Activate Credential
    ActivateCredential  = 0x00000147,
    /// Certify
    Certify             = 0x00000148,
    /// Policy NV
    PolicyNv            = 0x00000149,
    /// Certify Creation
    CertifyCreation     = 0x0000014A,
    /// Duplicate
    Duplicate           = 0x0000014B,
    /// Get Time
    GetTime             = 0x0000014C,
    /// Get Session Audit Digest
    GetSessionAuditDigest = 0x0000014D,
    /// NV Read
    NvRead              = 0x0000014E,
    /// NV Read Lock
    NvReadLock          = 0x0000014F,
    /// Object Change Auth
    ObjectChangeAuth    = 0x00000150,
    /// Policy Secret
    PolicySecret        = 0x00000151,
    /// Rewrap
    Rewrap              = 0x00000152,
    /// Create
    Create              = 0x00000153,
    /// ECDH Key Gen
    EcdhKeyGen          = 0x00000154,
    /// HMAC
    Hmac                = 0x00000155,
    /// Import
    Import              = 0x00000156,
    /// Load
    Load                = 0x00000157,
    /// Quote
    Quote               = 0x00000158,
    /// RSA Decrypt
    RsaDecrypt          = 0x00000159,
    /// HMAC Start
    HmacStart           = 0x0000015B,
    /// Sequence Update
    SequenceUpdate      = 0x0000015C,
    /// Sign
    Sign                = 0x0000015D,
    /// Unseal
    Unseal              = 0x0000015E,
    /// Policy Signed
    PolicySigned        = 0x00000160,
    /// Context Load
    ContextLoad         = 0x00000161,
    /// Context Save
    ContextSave         = 0x00000162,
    /// ECDH Z Gen
    EcdhZGen            = 0x00000163,
    /// Encrypt Decrypt
    EncryptDecrypt      = 0x00000164,
    /// Flush Context
    FlushContext        = 0x00000165,
    /// Load External
    LoadExternal        = 0x00000167,
    /// Make Credential
    MakeCredential      = 0x00000168,
    /// NV Read Public
    NvReadPublic        = 0x00000169,
    /// Policy Authorize
    PolicyAuthorize     = 0x0000016A,
    /// Policy Auth Value
    PolicyAuthValue     = 0x0000016B,
    /// Policy Command Code
    PolicyCommandCode   = 0x0000016C,
    /// Policy Counter Timer
    PolicyCounterTimer  = 0x0000016D,
    /// Policy CP Hash
    PolicyCpHash        = 0x0000016E,
    /// Policy Locality
    PolicyLocality      = 0x0000016F,
    /// Policy Name Hash
    PolicyNameHash      = 0x00000170,
    /// Policy OR
    PolicyOr            = 0x00000171,
    /// Policy Ticket
    PolicyTicket        = 0x00000172,
    /// Read Public
    ReadPublic          = 0x00000173,
    /// RSA Encrypt
    RsaEncrypt          = 0x00000174,
    /// Start Auth Session
    StartAuthSession    = 0x00000176,
    /// Verify Signature
    VerifySignature     = 0x00000177,
    /// ECC Parameters
    EccParameters       = 0x00000178,
    /// Firmware Read
    FirmwareRead        = 0x00000179,
    /// Get Capability
    GetCapability       = 0x0000017A,
    /// Get Random
    GetRandom           = 0x0000017B,
    /// Get Test Result
    GetTestResult       = 0x0000017C,
    /// Hash
    Hash                = 0x0000017D,
    /// PCR Read
    PcrRead             = 0x0000017E,
    /// Policy PCR
    PolicyPcr           = 0x0000017F,
    /// Policy Restart
    PolicyRestart       = 0x00000180,
    /// Read Clock
    ReadClock           = 0x00000181,
    /// PCR Extend
    PcrExtend           = 0x00000182,
    /// Policy Get Digest
    PolicyGetDigest     = 0x00000189,
    /// Test Parms
    TestParms           = 0x0000018A,
    /// Commit
    Commit              = 0x0000018B,
    /// Policy Password
    PolicyPassword      = 0x0000018C,
    /// Z Gen 2 Phase
    ZGen2Phase          = 0x0000018D,
    /// EC Ephemeral
    EcEphemeral         = 0x0000018E,
    /// Policy NV Written
    PolicyNvWritten     = 0x0000018F,
    /// Policy Template
    PolicyTemplate      = 0x00000190,
    /// Create Loaded
    CreateLoaded        = 0x00000191,
    /// Policy Authorize NV
    PolicyAuthorizeNv   = 0x00000192,
    /// Encrypt Decrypt 2
    EncryptDecrypt2     = 0x00000193,
}

// =============================================================================
// TPM 2.0 RESPONSE CODES
// =============================================================================

/// TPM response code format 0 (TPM 1.2 compatible)
pub const TPM_RC_SUCCESS: u32 = 0x00000000;
pub const TPM_RC_BAD_TAG: u32 = 0x0000001E;

/// TPM 2.0 response code format 1 (base values)
pub const TPM_RC_INITIALIZE: u32 = 0x00000100;
pub const TPM_RC_FAILURE: u32 = 0x00000101;
pub const TPM_RC_SEQUENCE: u32 = 0x00000103;
pub const TPM_RC_PRIVATE: u32 = 0x0000010B;
pub const TPM_RC_HMAC: u32 = 0x00000119;
pub const TPM_RC_DISABLED: u32 = 0x00000120;
pub const TPM_RC_EXCLUSIVE: u32 = 0x00000121;
pub const TPM_RC_AUTH_TYPE: u32 = 0x00000124;
pub const TPM_RC_AUTH_MISSING: u32 = 0x00000125;
pub const TPM_RC_POLICY: u32 = 0x00000126;
pub const TPM_RC_PCR: u32 = 0x00000127;
pub const TPM_RC_PCR_CHANGED: u32 = 0x00000128;
pub const TPM_RC_UPGRADE: u32 = 0x0000012D;
pub const TPM_RC_TOO_MANY_CONTEXTS: u32 = 0x0000012E;
pub const TPM_RC_AUTH_UNAVAILABLE: u32 = 0x0000012F;
pub const TPM_RC_REBOOT: u32 = 0x00000130;
pub const TPM_RC_UNBALANCED: u32 = 0x00000131;
pub const TPM_RC_COMMAND_SIZE: u32 = 0x00000142;
pub const TPM_RC_COMMAND_CODE: u32 = 0x00000143;
pub const TPM_RC_AUTHSIZE: u32 = 0x00000144;
pub const TPM_RC_AUTH_CONTEXT: u32 = 0x00000145;
pub const TPM_RC_NV_RANGE: u32 = 0x00000146;
pub const TPM_RC_NV_SIZE: u32 = 0x00000147;
pub const TPM_RC_NV_LOCKED: u32 = 0x00000148;
pub const TPM_RC_NV_AUTHORIZATION: u32 = 0x00000149;
pub const TPM_RC_NV_UNINITIALIZED: u32 = 0x0000014A;
pub const TPM_RC_NV_SPACE: u32 = 0x0000014B;
pub const TPM_RC_NV_DEFINED: u32 = 0x0000014C;
pub const TPM_RC_BAD_CONTEXT: u32 = 0x00000150;
pub const TPM_RC_CPHASH: u32 = 0x00000151;
pub const TPM_RC_PARENT: u32 = 0x00000152;
pub const TPM_RC_NEEDS_TEST: u32 = 0x00000153;
pub const TPM_RC_NO_RESULT: u32 = 0x00000154;
pub const TPM_RC_SENSITIVE: u32 = 0x00000155;

/// TPM response code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TpmResponseCode(pub u32);

impl TpmResponseCode {
    /// Check if response indicates success
    pub const fn is_success(&self) -> bool {
        self.0 == TPM_RC_SUCCESS
    }

    /// Get error message
    pub fn message(&self) -> &'static str {
        match self.0 {
            TPM_RC_SUCCESS => "Success",
            TPM_RC_BAD_TAG => "Bad tag",
            TPM_RC_INITIALIZE => "TPM not initialized",
            TPM_RC_FAILURE => "TPM failure",
            TPM_RC_SEQUENCE => "Sequence error",
            TPM_RC_DISABLED => "Command disabled",
            TPM_RC_EXCLUSIVE => "Exclusive session",
            TPM_RC_AUTH_MISSING => "Authorization missing",
            TPM_RC_POLICY => "Policy failure",
            TPM_RC_PCR => "PCR error",
            TPM_RC_COMMAND_CODE => "Unknown command",
            TPM_RC_NV_LOCKED => "NV locked",
            TPM_RC_NV_AUTHORIZATION => "NV authorization failed",
            TPM_RC_NV_UNINITIALIZED => "NV uninitialized",
            TPM_RC_NV_SPACE => "NV space not available",
            TPM_RC_NV_DEFINED => "NV already defined",
            _ => "Unknown error",
        }
    }
}

impl fmt::Display for TpmResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_success() {
            write!(f, "TPM_RC_SUCCESS")
        } else {
            write!(f, "TPM_RC_0x{:08X}: {}", self.0, self.message())
        }
    }
}

// =============================================================================
// TPM 2.0 HANDLES
// =============================================================================

/// TPM handle types
pub mod handle {
    /// PCR handles (0x00000000 - 0x0000001F)
    pub const PCR_FIRST: u32 = 0x00000000;
    pub const PCR_LAST: u32 = 0x0000001F;

    /// NV index handles (0x01000000 - 0x01FFFFFF)
    pub const NV_INDEX_FIRST: u32 = 0x01000000;
    pub const NV_INDEX_LAST: u32 = 0x01FFFFFF;

    /// HMAC/loaded session handles (0x02000000 - 0x02FFFFFF)
    pub const HMAC_SESSION_FIRST: u32 = 0x02000000;
    pub const HMAC_SESSION_LAST: u32 = 0x02FFFFFF;

    /// Loaded session handles (0x02000000 - 0x02FFFFFF)
    pub const LOADED_SESSION_FIRST: u32 = 0x02000000;
    pub const LOADED_SESSION_LAST: u32 = 0x02FFFFFF;

    /// Policy session handles (0x03000000 - 0x03FFFFFF)
    pub const POLICY_SESSION_FIRST: u32 = 0x03000000;
    pub const POLICY_SESSION_LAST: u32 = 0x03FFFFFF;

    /// Transient object handles (0x80000000 - 0x80FFFFFF)
    pub const TRANSIENT_FIRST: u32 = 0x80000000;
    pub const TRANSIENT_LAST: u32 = 0x80FFFFFF;

    /// Persistent object handles (0x81000000 - 0x81FFFFFF)
    pub const PERSISTENT_FIRST: u32 = 0x81000000;
    pub const PERSISTENT_LAST: u32 = 0x81FFFFFF;

    /// Permanent handles
    pub const PERMANENT_FIRST: u32 = 0x40000000;

    /// Owner hierarchy
    pub const RH_OWNER: u32 = 0x40000001;
    /// Null hierarchy
    pub const RH_NULL: u32 = 0x40000007;
    /// Lockout
    pub const RH_LOCKOUT: u32 = 0x4000000A;
    /// Endorsement hierarchy
    pub const RH_ENDORSEMENT: u32 = 0x4000000B;
    /// Platform hierarchy
    pub const RH_PLATFORM: u32 = 0x4000000C;
    /// Platform NV
    pub const RH_PLATFORM_NV: u32 = 0x4000000D;

    /// Password authorization
    pub const RS_PW: u32 = 0x40000009;
}

// =============================================================================
// PCR (Platform Configuration Register)
// =============================================================================

/// PCR bank selection
#[derive(Debug, Clone, Copy)]
pub struct PcrSelection {
    /// Hash algorithm
    pub algorithm: TpmAlgorithm,
    /// PCR bitmap (bit n = PCR n)
    pub pcr_select: [u8; 3],
}

impl PcrSelection {
    /// Create a new PCR selection
    pub const fn new(algorithm: TpmAlgorithm) -> Self {
        Self {
            algorithm,
            pcr_select: [0; 3],
        }
    }

    /// Select a PCR
    pub fn select_pcr(&mut self, pcr: u8) {
        if pcr < 24 {
            self.pcr_select[(pcr / 8) as usize] |= 1 << (pcr % 8);
        }
    }

    /// Deselect a PCR
    pub fn deselect_pcr(&mut self, pcr: u8) {
        if pcr < 24 {
            self.pcr_select[(pcr / 8) as usize] &= !(1 << (pcr % 8));
        }
    }

    /// Check if a PCR is selected
    pub const fn is_selected(&self, pcr: u8) -> bool {
        if pcr < 24 {
            (self.pcr_select[(pcr / 8) as usize] & (1 << (pcr % 8))) != 0
        } else {
            false
        }
    }

    /// Select PCRs 0-7 (BIOS/firmware measurements)
    pub fn select_firmware_pcrs(&mut self) {
        self.pcr_select[0] = 0xFF;
    }

    /// Select all PCRs
    pub fn select_all(&mut self) {
        self.pcr_select = [0xFF, 0xFF, 0xFF];
    }
}

/// PCR value with associated algorithm
#[derive(Clone)]
pub struct PcrValue {
    /// Hash algorithm
    pub algorithm: TpmAlgorithm,
    /// Digest value
    pub digest: [u8; 64],
    /// Actual digest length
    pub digest_len: usize,
}

impl PcrValue {
    /// Create a new empty PCR value
    pub const fn new(algorithm: TpmAlgorithm) -> Self {
        Self {
            algorithm,
            digest: [0; 64],
            digest_len: 0,
        }
    }

    /// Get digest slice
    pub fn digest(&self) -> &[u8] {
        &self.digest[..self.digest_len]
    }

    /// Check if PCR is in initial state (all zeros)
    pub fn is_initial(&self) -> bool {
        self.digest[..self.digest_len].iter().all(|&b| b == 0)
    }

    /// Check if PCR is in final state (all 0xFF)
    pub fn is_final(&self) -> bool {
        self.digest[..self.digest_len].iter().all(|&b| b == 0xFF)
    }
}

impl fmt::Debug for PcrValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PCR({:?}, ", self.algorithm)?;
        for byte in &self.digest[..self.digest_len] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

// =============================================================================
// TPM EVENT LOG
// =============================================================================

/// TCG event log event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EventType {
    /// Pre-boot cert
    PrebootCert          = 0x00000000,
    /// POST code
    PostCode             = 0x00000001,
    /// Unused
    Unused               = 0x00000002,
    /// No action
    NoAction             = 0x00000003,
    /// Separator
    Separator            = 0x00000004,
    /// Action
    Action               = 0x00000005,
    /// Event tag
    EventTag             = 0x00000006,
    /// S-CRTM contents
    SCrtmContents        = 0x00000007,
    /// S-CRTM version
    SCrtmVersion         = 0x00000008,
    /// CPU microcode
    CpuMicrocode         = 0x00000009,
    /// Platform config flags
    PlatformConfigFlags  = 0x0000000A,
    /// Table of devices
    TableOfDevices       = 0x0000000B,
    /// Compact hash
    CompactHash          = 0x0000000C,
    /// IPL
    Ipl                  = 0x0000000D,
    /// IPL partition data
    IplPartitionData     = 0x0000000E,
    /// Non-host code
    NonHostCode          = 0x0000000F,
    /// Non-host config
    NonHostConfig        = 0x00000010,
    /// Non-host info
    NonHostInfo          = 0x00000011,
    /// Omit boot device events
    OmitBootDeviceEvents = 0x00000012,
    /// EFI event base
    EfiEventBase         = 0x80000000,
    /// EFI variable driver config
    EfiVariableDriverConfig = 0x80000001,
    /// EFI variable boot
    EfiVariableBoot      = 0x80000002,
    /// EFI boot services application
    EfiBootServicesApplication = 0x80000003,
    /// EFI boot services driver
    EfiBootServicesDriver = 0x80000004,
    /// EFI runtime services driver
    EfiRuntimeServicesDriver = 0x80000005,
    /// EFI GPT event
    EfiGptEvent          = 0x80000006,
    /// EFI action
    EfiAction            = 0x80000007,
    /// EFI platform firmware blob
    EfiPlatformFirmwareBlob = 0x80000008,
    /// EFI handoff tables
    EfiHandoffTables     = 0x80000009,
    /// EFI platform firmware blob 2
    EfiPlatformFirmwareBlob2 = 0x8000000A,
    /// EFI handoff tables 2
    EfiHandoffTables2    = 0x8000000B,
    /// EFI variable boot 2
    EfiVariableBoot2     = 0x8000000C,
    /// EFI HCR event
    EfiHcrEvent          = 0x80000010,
    /// EFI variable authority
    EfiVariableAuthority = 0x800000E0,
    /// EFI SPDM firmware blob
    EfiSpdmFirmwareBlob  = 0x800000E1,
    /// EFI SPDM firmware config
    EfiSpdmFirmwareConfig = 0x800000E2,
}

/// TCG event log entry header (spec ID 1.0)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TcgEventHeader {
    /// PCR index
    pub pcr_index: u32,
    /// Event type
    pub event_type: u32,
    /// SHA-1 digest
    pub digest: [u8; 20],
    /// Event data size
    pub event_size: u32,
}

/// TCG event log entry (Crypto-Agile format for TPM 2.0)
#[derive(Debug, Clone)]
pub struct TcgEvent2 {
    /// PCR index
    pub pcr_index: u32,
    /// Event type
    pub event_type: EventType,
    /// Digests for all active banks
    pub digests: [PcrValue; 4],
    /// Number of digests
    pub digest_count: usize,
    /// Event data
    pub event_data: [u8; 256],
    /// Event data size
    pub event_size: usize,
}

impl TcgEvent2 {
    /// Create new event
    pub const fn new(pcr_index: u32, event_type: EventType) -> Self {
        Self {
            pcr_index,
            event_type,
            digests: [
                PcrValue::new(TpmAlgorithm::Sha256),
                PcrValue::new(TpmAlgorithm::Sha384),
                PcrValue::new(TpmAlgorithm::Sha512),
                PcrValue::new(TpmAlgorithm::Sha1),
            ],
            digest_count: 0,
            event_data: [0; 256],
            event_size: 0,
        }
    }
}

// =============================================================================
// TPM NVRAM
// =============================================================================

/// TPM NV index attributes
#[derive(Debug, Clone, Copy)]
pub struct NvAttributes(pub u32);

impl NvAttributes {
    /// Platform create
    pub const PPWRITE: u32 = 1 << 0;
    /// Owner write
    pub const OWNERWRITE: u32 = 1 << 1;
    /// Auth write
    pub const AUTHWRITE: u32 = 1 << 2;
    /// Policy write
    pub const POLICYWRITE: u32 = 1 << 3;
    /// Counter
    pub const COUNTER: u32 = 1 << 4;
    /// Bits
    pub const BITS: u32 = 1 << 5;
    /// Extend
    pub const EXTEND: u32 = 1 << 6;
    /// Policy delete
    pub const POLICY_DELETE: u32 = 1 << 10;
    /// Written
    pub const WRITTEN: u32 = 1 << 11;
    /// Write all
    pub const WRITEALL: u32 = 1 << 12;
    /// Write define
    pub const WRITEDEFINE: u32 = 1 << 13;
    /// Write stclear
    pub const WRITE_STCLEAR: u32 = 1 << 14;
    /// Global lock
    pub const GLOBALLOCK: u32 = 1 << 15;
    /// Platform read
    pub const PPREAD: u32 = 1 << 16;
    /// Owner read
    pub const OWNERREAD: u32 = 1 << 17;
    /// Auth read
    pub const AUTHREAD: u32 = 1 << 18;
    /// Policy read
    pub const POLICYREAD: u32 = 1 << 19;
    /// No DA
    pub const NO_DA: u32 = 1 << 25;
    /// Orderly
    pub const ORDERLY: u32 = 1 << 26;
    /// Clear stclear
    pub const CLEAR_STCLEAR: u32 = 1 << 27;
    /// Read stclear
    pub const READ_STCLEAR: u32 = 1 << 28;
    /// Read locked
    pub const READLOCKED: u32 = 1 << 30;
    /// Write locked
    pub const WRITELOCKED: u32 = 1 << 31;

    /// Create new NV attributes
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Check if platform write is allowed
    pub const fn ppwrite(&self) -> bool {
        (self.0 & Self::PPWRITE) != 0
    }

    /// Check if owner write is allowed
    pub const fn ownerwrite(&self) -> bool {
        (self.0 & Self::OWNERWRITE) != 0
    }

    /// Check if auth write is allowed
    pub const fn authwrite(&self) -> bool {
        (self.0 & Self::AUTHWRITE) != 0
    }

    /// Check if this is a counter
    pub const fn is_counter(&self) -> bool {
        (self.0 & Self::COUNTER) != 0
    }

    /// Check if this is a bit field
    pub const fn is_bits(&self) -> bool {
        (self.0 & Self::BITS) != 0
    }

    /// Check if this is an extend area
    pub const fn is_extend(&self) -> bool {
        (self.0 & Self::EXTEND) != 0
    }

    /// Check if written
    pub const fn is_written(&self) -> bool {
        (self.0 & Self::WRITTEN) != 0
    }

    /// Check if write locked
    pub const fn is_write_locked(&self) -> bool {
        (self.0 & Self::WRITELOCKED) != 0
    }

    /// Check if read locked
    pub const fn is_read_locked(&self) -> bool {
        (self.0 & Self::READLOCKED) != 0
    }
}

/// NV index public area
#[derive(Debug, Clone)]
pub struct NvPublic {
    /// NV index handle
    pub nv_index: u32,
    /// Name algorithm
    pub name_alg: TpmAlgorithm,
    /// Attributes
    pub attributes: NvAttributes,
    /// Auth policy
    pub auth_policy: [u8; 64],
    /// Auth policy size
    pub auth_policy_size: usize,
    /// Data size
    pub data_size: u16,
}

impl NvPublic {
    /// Create a new NV public area
    pub const fn new(nv_index: u32, data_size: u16) -> Self {
        Self {
            nv_index,
            name_alg: TpmAlgorithm::Sha256,
            attributes: NvAttributes(
                NvAttributes::OWNERWRITE
                    | NvAttributes::OWNERREAD
                    | NvAttributes::AUTHREAD
                    | NvAttributes::AUTHWRITE,
            ),
            auth_policy: [0; 64],
            auth_policy_size: 0,
            data_size,
        }
    }
}

// =============================================================================
// TPM CAPABILITY
// =============================================================================

/// TPM capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TpmCapability {
    /// Algorithms
    Algorithms    = 0x00000000,
    /// Handles
    Handles       = 0x00000001,
    /// Commands
    Commands      = 0x00000002,
    /// PP Commands
    PpCommands    = 0x00000003,
    /// Audit Commands
    AuditCommands = 0x00000004,
    /// Assigned PCRs
    Pcrs          = 0x00000005,
    /// TPM Properties
    TpmProperties = 0x00000006,
    /// PCR Properties
    PcrProperties = 0x00000007,
    /// ECC Curves
    EccCurves     = 0x00000008,
    /// Auth Policies
    AuthPolicies  = 0x00000009,
}

/// TPM property tags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TpmProperty {
    /// TPM family indicator
    FamilyIndicator   = 0x100,
    /// Level
    Level             = 0x101,
    /// Revision
    Revision          = 0x102,
    /// Day of year
    DayOfYear         = 0x103,
    /// Year
    Year              = 0x104,
    /// Manufacturer
    Manufacturer      = 0x105,
    /// Vendor string 1
    VendorString1     = 0x106,
    /// Vendor string 2
    VendorString2     = 0x107,
    /// Vendor string 3
    VendorString3     = 0x108,
    /// Vendor string 4
    VendorString4     = 0x109,
    /// Vendor TPM type
    VendorTpmType     = 0x10A,
    /// Firmware version 1
    FirmwareVersion1  = 0x10B,
    /// Firmware version 2
    FirmwareVersion2  = 0x10C,
    /// Input buffer size
    InputBuffer       = 0x10D,
    /// Max response size
    MaxResponseSize   = 0x10E,
    /// Max digest
    MaxDigest         = 0x10F,
    /// Max object context size
    MaxObjectContext  = 0x110,
    /// Max session context size
    MaxSessionContext = 0x111,
    /// PS family indicator
    PsFamilyIndicator = 0x112,
    /// PS level
    PsLevel           = 0x113,
    /// PS revision
    PsRevision        = 0x114,
    /// PS day of year
    PsDayOfYear       = 0x115,
    /// PS year
    PsYear            = 0x116,
    /// Split max
    SplitMax          = 0x117,
    /// Total commands
    TotalCommands     = 0x118,
    /// Library commands
    LibraryCommands   = 0x119,
    /// Vendor commands
    VendorCommands    = 0x11A,
    /// NV buffer max
    NvBufferMax       = 0x11B,
    /// Modes
    Modes             = 0x11C,
    /// Max cap buffer
    MaxCapBuffer      = 0x11D,
}

/// TPM startup type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TpmStartupType {
    /// Clear
    Clear = 0x0000,
    /// State
    State = 0x0001,
}

/// TPM shutdown type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TpmShutdownType {
    /// Clear
    Clear = 0x0000,
    /// State
    State = 0x0001,
}

// =============================================================================
// TPM COMMAND HEADER
// =============================================================================

/// TPM command header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TpmCommandHeader {
    /// Tag
    pub tag: u16,
    /// Command size
    pub command_size: u32,
    /// Command code
    pub command_code: u32,
}

impl TpmCommandHeader {
    /// TPM 2.0 command without sessions
    pub const TPM_ST_NO_SESSIONS: u16 = 0x8001;
    /// TPM 2.0 command with sessions
    pub const TPM_ST_SESSIONS: u16 = 0x8002;

    /// Create a new command header
    pub const fn new(command_code: TpmCommand, size: u32, with_sessions: bool) -> Self {
        Self {
            tag: if with_sessions {
                Self::TPM_ST_SESSIONS.swap_bytes()
            } else {
                Self::TPM_ST_NO_SESSIONS.swap_bytes()
            },
            command_size: size.swap_bytes(),
            command_code: (command_code as u32).swap_bytes(),
        }
    }
}

/// TPM response header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TpmResponseHeader {
    /// Tag
    pub tag: u16,
    /// Response size
    pub response_size: u32,
    /// Response code
    pub response_code: u32,
}

impl TpmResponseHeader {
    /// Get response code
    pub const fn response_code(&self) -> TpmResponseCode {
        TpmResponseCode(u32::from_be(self.response_code))
    }

    /// Get response size
    pub const fn size(&self) -> u32 {
        u32::from_be(self.response_size)
    }

    /// Check if response is success
    pub const fn is_success(&self) -> bool {
        self.response_code().is_success()
    }
}

// =============================================================================
// TPM COMMAND BUILDERS
// =============================================================================

/// TPM command buffer
pub struct TpmCommandBuffer {
    /// Buffer data
    pub data: [u8; TPM_MAX_COMMAND_SIZE],
    /// Current position
    pub pos: usize,
}

impl TpmCommandBuffer {
    /// Create a new command buffer
    pub const fn new() -> Self {
        Self {
            data: [0; TPM_MAX_COMMAND_SIZE],
            pos: 0,
        }
    }

    /// Reset buffer
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    /// Write u8
    pub fn write_u8(&mut self, value: u8) {
        if self.pos < TPM_MAX_COMMAND_SIZE {
            self.data[self.pos] = value;
            self.pos += 1;
        }
    }

    /// Write u16 (big-endian)
    pub fn write_u16(&mut self, value: u16) {
        let bytes = value.to_be_bytes();
        self.write_bytes(&bytes);
    }

    /// Write u32 (big-endian)
    pub fn write_u32(&mut self, value: u32) {
        let bytes = value.to_be_bytes();
        self.write_bytes(&bytes);
    }

    /// Write bytes
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u8(byte);
        }
    }

    /// Start command
    pub fn start_command(&mut self, command: TpmCommand, with_sessions: bool) {
        self.reset();
        // Tag
        self.write_u16(if with_sessions {
            TpmCommandHeader::TPM_ST_SESSIONS
        } else {
            TpmCommandHeader::TPM_ST_NO_SESSIONS
        });
        // Size placeholder
        self.write_u32(0);
        // Command code
        self.write_u32(command as u32);
    }

    /// Finalize command (update size)
    pub fn finalize(&mut self) {
        let size = self.pos as u32;
        let bytes = size.to_be_bytes();
        self.data[2..6].copy_from_slice(&bytes);
    }

    /// Get command data
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.pos]
    }

    /// Build TPM2_Startup command
    pub fn build_startup(&mut self, startup_type: TpmStartupType) {
        self.start_command(TpmCommand::Startup, false);
        self.write_u16(startup_type as u16);
        self.finalize();
    }

    /// Build TPM2_Shutdown command
    pub fn build_shutdown(&mut self, shutdown_type: TpmShutdownType) {
        self.start_command(TpmCommand::Shutdown, false);
        self.write_u16(shutdown_type as u16);
        self.finalize();
    }

    /// Build TPM2_SelfTest command
    pub fn build_self_test(&mut self, full_test: bool) {
        self.start_command(TpmCommand::SelfTest, false);
        self.write_u8(if full_test { 1 } else { 0 });
        self.finalize();
    }

    /// Build TPM2_GetRandom command
    pub fn build_get_random(&mut self, bytes_requested: u16) {
        self.start_command(TpmCommand::GetRandom, false);
        self.write_u16(bytes_requested);
        self.finalize();
    }

    /// Build TPM2_PCR_Read command
    pub fn build_pcr_read(&mut self, selection: &PcrSelection) {
        self.start_command(TpmCommand::PcrRead, false);
        // PCR selection count
        self.write_u32(1);
        // Algorithm
        self.write_u16(selection.algorithm as u16);
        // Size of select
        self.write_u8(3);
        // PCR select bitmap
        self.write_bytes(&selection.pcr_select);
        self.finalize();
    }

    /// Build TPM2_PCR_Extend command
    pub fn build_pcr_extend(&mut self, pcr_handle: u32, algorithm: TpmAlgorithm, digest: &[u8]) {
        self.start_command(TpmCommand::PcrExtend, true);
        // PCR handle
        self.write_u32(pcr_handle);
        // Authorization area (password session)
        self.write_u32(9); // Auth size
        self.write_u32(handle::RS_PW); // Password session
        self.write_u16(0); // Nonce size
        self.write_u8(0); // Session attributes
        self.write_u16(0); // Auth value size
                           // Digest count
        self.write_u32(1);
        // Algorithm
        self.write_u16(algorithm as u16);
        // Digest
        self.write_u16(digest.len() as u16);
        self.write_bytes(digest);
        self.finalize();
    }

    /// Build TPM2_Hash command
    pub fn build_hash(&mut self, data: &[u8], algorithm: TpmAlgorithm) {
        self.start_command(TpmCommand::Hash, false);
        // Data
        self.write_u16(data.len() as u16);
        self.write_bytes(data);
        // Algorithm
        self.write_u16(algorithm as u16);
        // Hierarchy
        self.write_u32(handle::RH_NULL);
        self.finalize();
    }

    /// Build TPM2_GetCapability command
    pub fn build_get_capability(
        &mut self,
        capability: TpmCapability,
        property: u32,
        property_count: u32,
    ) {
        self.start_command(TpmCommand::GetCapability, false);
        self.write_u32(capability as u32);
        self.write_u32(property);
        self.write_u32(property_count);
        self.finalize();
    }

    /// Build TPM2_NV_Read command
    pub fn build_nv_read(&mut self, nv_index: u32, size: u16, offset: u16) {
        self.start_command(TpmCommand::NvRead, true);
        // Auth handle
        self.write_u32(nv_index);
        // NV index
        self.write_u32(nv_index);
        // Authorization area
        self.write_u32(9);
        self.write_u32(handle::RS_PW);
        self.write_u16(0);
        self.write_u8(0);
        self.write_u16(0);
        // Size
        self.write_u16(size);
        // Offset
        self.write_u16(offset);
        self.finalize();
    }

    /// Build TPM2_NV_Write command
    pub fn build_nv_write(&mut self, nv_index: u32, data: &[u8], offset: u16) {
        self.start_command(TpmCommand::NvWrite, true);
        // Auth handle
        self.write_u32(nv_index);
        // NV index
        self.write_u32(nv_index);
        // Authorization area
        self.write_u32(9);
        self.write_u32(handle::RS_PW);
        self.write_u16(0);
        self.write_u8(0);
        self.write_u16(0);
        // Data
        self.write_u16(data.len() as u16);
        self.write_bytes(data);
        // Offset
        self.write_u16(offset);
        self.finalize();
    }
}

impl Default for TpmCommandBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TPM RESPONSE PARSER
// =============================================================================

/// TPM response buffer
pub struct TpmResponseBuffer<'a> {
    /// Response data
    pub data: &'a [u8],
    /// Current position
    pub pos: usize,
}

impl<'a> TpmResponseBuffer<'a> {
    /// Create a new response buffer
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Read u8
    pub fn read_u8(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let value = self.data[self.pos];
            self.pos += 1;
            Some(value)
        } else {
            None
        }
    }

    /// Read u16 (big-endian)
    pub fn read_u16(&mut self) -> Option<u16> {
        if self.pos + 2 <= self.data.len() {
            let value = u16::from_be_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(value)
        } else {
            None
        }
    }

    /// Read u32 (big-endian)
    pub fn read_u32(&mut self) -> Option<u32> {
        if self.pos + 4 <= self.data.len() {
            let value = u32::from_be_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(value)
        } else {
            None
        }
    }

    /// Read bytes
    pub fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.pos + len <= self.data.len() {
            let slice = &self.data[self.pos..self.pos + len];
            self.pos += len;
            Some(slice)
        } else {
            None
        }
    }

    /// Read response header
    pub fn read_header(&mut self) -> Option<TpmResponseHeader> {
        let tag = self.read_u16()?;
        let size = self.read_u32()?;
        let code = self.read_u32()?;

        Some(TpmResponseHeader {
            tag: tag.swap_bytes(),
            response_size: size.swap_bytes(),
            response_code: code.swap_bytes(),
        })
    }

    /// Skip bytes
    pub fn skip(&mut self, len: usize) -> bool {
        if self.pos + len <= self.data.len() {
            self.pos += len;
            true
        } else {
            false
        }
    }

    /// Remaining bytes
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }
}

// =============================================================================
// TPM INFO STRUCTURE
// =============================================================================

/// TPM information
#[derive(Debug, Clone)]
pub struct TpmInfo {
    /// TPM version
    pub version: TpmVersion,
    /// Interface type
    pub interface: TpmInterface,
    /// Manufacturer ID
    pub manufacturer: u32,
    /// Vendor string
    pub vendor: [u8; 16],
    /// Firmware version
    pub firmware_version: u64,
    /// Is TPM enabled
    pub enabled: bool,
    /// Is TPM activated
    pub activated: bool,
    /// Is owner present
    pub owner_present: bool,
    /// Physical presence required
    pub pp_required: bool,
    /// Lockout active
    pub lockout: bool,
    /// Maximum PCRs
    pub max_pcrs: u8,
    /// Supported algorithms
    pub algorithms: u16,
}

impl TpmInfo {
    /// Create new TPM info
    pub const fn new() -> Self {
        Self {
            version: TpmVersion::Unknown,
            interface: TpmInterface::Tis,
            manufacturer: 0,
            vendor: [0; 16],
            firmware_version: 0,
            enabled: false,
            activated: false,
            owner_present: false,
            pp_required: false,
            lockout: false,
            max_pcrs: 24,
            algorithms: 0,
        }
    }

    /// Get manufacturer name
    pub fn manufacturer_name(&self) -> &'static str {
        match self.manufacturer {
            0x414D4400 => "AMD",
            0x41544D4C => "Atmel",
            0x4252434D => "Broadcom",
            0x48504500 => "HPE",
            0x49424D00 => "IBM",
            0x49465800 => "Infineon",
            0x494E5443 => "Intel",
            0x4C454E00 => "Lenovo",
            0x4D534654 => "Microsoft",
            0x4E534D20 => "National Semiconductor",
            0x4E545A00 => "Nationz",
            0x4E544300 => "Nuvoton",
            0x51434F4D => "Qualcomm",
            0x534D5343 => "SMSC",
            0x53544D20 => "ST Microelectronics",
            0x534D534E => "Samsung",
            0x534E5300 => "Sinosun",
            0x54584E00 => "Texas Instruments",
            0x57454300 => "Winbond",
            _ => "Unknown",
        }
    }
}

impl Default for TpmInfo {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TPM ERROR TYPES
// =============================================================================

/// TPM error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpmError {
    /// TPM not found
    NotFound,
    /// TPM not initialized
    NotInitialized,
    /// Communication error
    CommunicationError,
    /// Invalid response
    InvalidResponse,
    /// TPM error
    TpmError(TpmResponseCode),
    /// Timeout
    Timeout,
    /// Access denied
    AccessDenied,
    /// Invalid parameter
    InvalidParameter,
    /// Buffer too small
    BufferTooSmall,
    /// NV space not defined
    NvNotDefined,
    /// NV locked
    NvLocked,
    /// PCR error
    PcrError,
    /// Authorization failed
    AuthorizationFailed,
    /// Unsupported
    Unsupported,
}

impl fmt::Display for TpmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TpmError::NotFound => write!(f, "TPM not found"),
            TpmError::NotInitialized => write!(f, "TPM not initialized"),
            TpmError::CommunicationError => write!(f, "Communication error"),
            TpmError::InvalidResponse => write!(f, "Invalid response"),
            TpmError::TpmError(code) => write!(f, "TPM error: {}", code),
            TpmError::Timeout => write!(f, "Timeout"),
            TpmError::AccessDenied => write!(f, "Access denied"),
            TpmError::InvalidParameter => write!(f, "Invalid parameter"),
            TpmError::BufferTooSmall => write!(f, "Buffer too small"),
            TpmError::NvNotDefined => write!(f, "NV space not defined"),
            TpmError::NvLocked => write!(f, "NV locked"),
            TpmError::PcrError => write!(f, "PCR error"),
            TpmError::AuthorizationFailed => write!(f, "Authorization failed"),
            TpmError::Unsupported => write!(f, "Unsupported operation"),
        }
    }
}

// =============================================================================
// ATTESTATION STRUCTURES
// =============================================================================

/// TPM attestation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AttestationType {
    /// Certification
    Certify      = 0x8017,
    /// Quote
    Quote        = 0x8018,
    /// Session audit
    SessionAudit = 0x8019,
    /// Command audit
    CommandAudit = 0x801A,
    /// Time
    Time         = 0x801B,
    /// Creation
    Creation     = 0x801C,
    /// NV
    Nv           = 0x8014,
}

/// Quote info structure
#[derive(Debug, Clone)]
pub struct QuoteInfo {
    /// PCR selection
    pub pcr_select: [PcrSelection; 4],
    /// Number of PCR selections
    pub pcr_select_count: usize,
    /// PCR digest
    pub pcr_digest: [u8; 64],
    /// PCR digest size
    pub pcr_digest_size: usize,
}

impl QuoteInfo {
    /// Create new quote info
    pub const fn new() -> Self {
        Self {
            pcr_select: [
                PcrSelection::new(TpmAlgorithm::Sha256),
                PcrSelection::new(TpmAlgorithm::Sha256),
                PcrSelection::new(TpmAlgorithm::Sha256),
                PcrSelection::new(TpmAlgorithm::Sha256),
            ],
            pcr_select_count: 0,
            pcr_digest: [0; 64],
            pcr_digest_size: 0,
        }
    }
}

impl Default for QuoteInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Clock info from attestation
#[derive(Debug, Clone, Copy)]
pub struct ClockInfo {
    /// Clock value in milliseconds
    pub clock: u64,
    /// Reset count
    pub reset_count: u32,
    /// Restart count
    pub restart_count: u32,
    /// Safe flag
    pub safe: bool,
}

// =============================================================================
// MEASURED BOOT HELPERS
// =============================================================================

/// Standard PCR assignments
pub mod pcr_index {
    /// SRTM/CRTM, BIOS, firmware
    pub const SRTM: u32 = 0;
    /// Host platform configuration
    pub const PLATFORM_CONFIG: u32 = 1;
    /// Option ROM code
    pub const OPTION_ROM_CODE: u32 = 2;
    /// Option ROM configuration and data
    pub const OPTION_ROM_CONFIG: u32 = 3;
    /// IPL code (MBR, partition table)
    pub const IPL_CODE: u32 = 4;
    /// IPL configuration and data
    pub const IPL_CONFIG: u32 = 5;
    /// State transitions and wake events
    pub const STATE_TRANSITIONS: u32 = 6;
    /// Host platform manufacturer control
    pub const MANUFACTURER_CONTROL: u32 = 7;
    /// UEFI/Secure Boot (used by OS loaders)
    pub const SECURE_BOOT: u32 = 7;
    /// OS/Application (typically kernel)
    pub const OS_KERNEL: u32 = 8;
    /// Application/kernel configuration
    pub const OS_CONFIG: u32 = 9;
    /// Application/user data
    pub const USER_DATA: u32 = 10;
    /// Microsoft BitLocker
    pub const BITLOCKER: u32 = 11;
    /// Reserved for future use
    pub const RESERVED_12: u32 = 12;
    /// Reserved for future use
    pub const RESERVED_13: u32 = 13;
    /// Reserved for OS
    pub const OS_RESERVED: u32 = 14;
    /// Reserved for OS
    pub const OS_RESERVED_2: u32 = 15;
    /// Debug PCR
    pub const DEBUG: u32 = 16;
    /// Locality 4 (trusted OS)
    pub const DRTM: u32 = 17;
    /// Trusted OS (DRTM)
    pub const TRUSTED_OS: u32 = 18;
    /// Trusted OS (DRTM)
    pub const TRUSTED_OS_2: u32 = 19;
    /// Trusted OS (DRTM)
    pub const TRUSTED_OS_3: u32 = 20;
    /// Trusted OS (DRTM)
    pub const TRUSTED_OS_4: u32 = 21;
    /// Trusted OS (DRTM)
    pub const TRUSTED_OS_5: u32 = 22;
    /// Application support
    pub const APPLICATION: u32 = 23;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_digest_size() {
        assert_eq!(TpmAlgorithm::Sha1.digest_size(), Some(20));
        assert_eq!(TpmAlgorithm::Sha256.digest_size(), Some(32));
        assert_eq!(TpmAlgorithm::Sha384.digest_size(), Some(48));
        assert_eq!(TpmAlgorithm::Sha512.digest_size(), Some(64));
        assert_eq!(TpmAlgorithm::Rsa.digest_size(), None);
    }

    #[test]
    fn test_algorithm_classification() {
        assert!(TpmAlgorithm::Sha256.is_hash());
        assert!(!TpmAlgorithm::Rsa.is_hash());
        assert!(TpmAlgorithm::Rsa.is_asymmetric());
        assert!(TpmAlgorithm::Aes.is_symmetric());
    }

    #[test]
    fn test_pcr_selection() {
        let mut sel = PcrSelection::new(TpmAlgorithm::Sha256);
        sel.select_pcr(0);
        sel.select_pcr(7);
        sel.select_pcr(23);

        assert!(sel.is_selected(0));
        assert!(sel.is_selected(7));
        assert!(sel.is_selected(23));
        assert!(!sel.is_selected(1));
    }

    #[test]
    fn test_command_buffer() {
        let mut cmd = TpmCommandBuffer::new();
        cmd.build_startup(TpmStartupType::Clear);

        let slice = cmd.as_slice();
        assert!(slice.len() > 10);
        // Tag should be TPM_ST_NO_SESSIONS in big-endian
        assert_eq!(slice[0], 0x80);
        assert_eq!(slice[1], 0x01);
    }

    #[test]
    fn test_response_code() {
        let success = TpmResponseCode(TPM_RC_SUCCESS);
        assert!(success.is_success());

        let failure = TpmResponseCode(TPM_RC_FAILURE);
        assert!(!failure.is_success());
    }

    #[test]
    fn test_nv_attributes() {
        let attrs = NvAttributes::new(NvAttributes::OWNERWRITE | NvAttributes::OWNERREAD);
        assert!(attrs.ownerwrite());
        assert!(!attrs.ppwrite());
    }

    #[test]
    fn test_ecc_curves() {
        assert_eq!(TpmEccCurve::NistP256.key_bits(), 256);
        assert_eq!(TpmEccCurve::NistP384.key_bits(), 384);
        assert_eq!(TpmEccCurve::NistP521.key_bits(), 521);
    }
}
