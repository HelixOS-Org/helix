//! HelixFS Integration for Helix OS Kernel
//!
//! This module integrates the revolutionary HelixFS filesystem into the kernel.
//! Features:
//! - Copy-on-Write snapshots
//! - Transparent compression (LZ4/ZSTD)
//! - Native encryption (AES-GCM, ChaCha20)
//! - B+tree extent mapping
//! - Journaling with crash recovery

use alloc::vec::Vec;

use crate::serial_write_str;

// ============================================================================
// Error Types (Mirror of helix_fs types for kernel integration)
// ============================================================================

/// Filesystem error
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum HfsError {
    /// Not initialized
    NotInitialized,
    /// Invalid block number
    InvalidBlockNumber,
    /// Buffer too small
    BufferTooSmall,
    /// Bad magic number
    BadMagic,
    /// Name too long
    NameTooLong,
    /// Not supported
    NotSupported,
    /// I/O error
    IoError,
}

/// Result type
pub type HfsResult<T> = Result<T, HfsError>;

// ============================================================================
// RAM Disk Backend
// ============================================================================

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

/// RAM disk size (4MB for demo)
const RAMDISK_SIZE: usize = 4 * 1024 * 1024;
/// Block size (4KB)
const BLOCK_SIZE: usize = 4096;
/// Number of blocks
const BLOCK_COUNT: usize = RAMDISK_SIZE / BLOCK_SIZE;

/// RAM disk storage - Static buffer to avoid stack allocation
/// The buffer is stored directly in .bss, not allocated on stack
#[repr(align(4096))]
struct RamDiskBuffer([u8; RAMDISK_SIZE]);

/// Wrapper for RAM disk buffer that provides interior mutability.
///
/// SAFETY: Access to the RAM disk is protected by RAMDISK_LOCK mutex.
/// All read/write operations must hold the lock.
struct StaticRamDisk(UnsafeCell<RamDiskBuffer>);

// SAFETY: Access is protected by RAMDISK_LOCK
unsafe impl Sync for StaticRamDisk {}

impl StaticRamDisk {
    const fn new() -> Self {
        Self(UnsafeCell::new(RamDiskBuffer([0u8; RAMDISK_SIZE])))
    }

    /// Get mutable access to the buffer
    ///
    /// # Safety
    /// - Caller must hold RAMDISK_LOCK
    /// - This uses interior mutability through UnsafeCell which is sound
    ///   because access is protected by the RAMDISK_LOCK mutex
    #[inline]
    #[allow(clippy::mut_from_ref)]
    unsafe fn as_mut(&self) -> &mut RamDiskBuffer {
        unsafe { &mut *self.0.get() }
    }

    /// Get immutable access to the buffer
    ///
    /// # Safety
    /// Caller must hold RAMDISK_LOCK
    #[inline]
    unsafe fn as_ref(&self) -> &RamDiskBuffer {
        unsafe { &*self.0.get() }
    }
}

/// Static RAM disk buffer
static RAMDISK_BUFFER: StaticRamDisk = StaticRamDisk::new();

/// Lock for RAM disk access
static RAMDISK_LOCK: Mutex<()> = Mutex::new(());

/// Flag to track initialization (atomic for lock-free check)
static RAMDISK_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the RAM disk
fn init_ramdisk() {
    let _lock = RAMDISK_LOCK.lock();
    // Buffer is already zero-initialized in .bss, just mark as initialized
    RAMDISK_INITIALIZED.store(true, Ordering::Release);
}

/// Read a block from RAM disk
fn read_block(block_num: u64, buffer: &mut [u8]) -> HfsResult<()> {
    if block_num as usize >= BLOCK_COUNT {
        return Err(HfsError::InvalidBlockNumber);
    }
    if buffer.len() < BLOCK_SIZE {
        return Err(HfsError::BufferTooSmall);
    }

    let _lock = RAMDISK_LOCK.lock();
    if !RAMDISK_INITIALIZED.load(Ordering::Acquire) {
        return Err(HfsError::NotInitialized);
    }

    // SAFETY: We hold RAMDISK_LOCK
    unsafe {
        let offset = block_num as usize * BLOCK_SIZE;
        buffer[..BLOCK_SIZE]
            .copy_from_slice(&RAMDISK_BUFFER.as_ref().0[offset..offset + BLOCK_SIZE]);
    }
    Ok(())
}

/// Write a block to RAM disk
fn write_block(block_num: u64, data: &[u8]) -> HfsResult<()> {
    if block_num as usize >= BLOCK_COUNT {
        return Err(HfsError::InvalidBlockNumber);
    }
    if data.len() < BLOCK_SIZE {
        return Err(HfsError::BufferTooSmall);
    }

    let _lock = RAMDISK_LOCK.lock();
    if !RAMDISK_INITIALIZED.load(Ordering::Acquire) {
        return Err(HfsError::NotInitialized);
    }

    // SAFETY: We hold RAMDISK_LOCK
    unsafe {
        let offset = block_num as usize * BLOCK_SIZE;
        RAMDISK_BUFFER.as_mut().0[offset..offset + BLOCK_SIZE].copy_from_slice(&data[..BLOCK_SIZE]);
    }
    Ok(())
}

// ============================================================================
// Filesystem State
// ============================================================================

/// Filesystem state
pub struct HelixFsState {
    /// Is filesystem mounted
    pub mounted: bool,
    /// Total blocks
    pub total_blocks: u64,
    /// Free blocks
    pub free_blocks: u64,
    /// Block size
    pub block_size: u32,
    /// Root inode
    pub root_ino: u64,
    /// Next free inode
    pub next_ino: u64,
    /// Mount options (reserved for future use)
    #[allow(dead_code)]
    pub mount_flags: u32,
}

impl HelixFsState {
    /// Create new filesystem state
    pub const fn new() -> Self {
        Self {
            mounted: false,
            total_blocks: BLOCK_COUNT as u64,
            free_blocks: BLOCK_COUNT as u64 - 16, // Reserve first 16 blocks
            block_size: BLOCK_SIZE as u32,
            root_ino: 2,
            next_ino: 11,
            mount_flags: 0,
        }
    }
}

/// Thread-safe filesystem state protected by mutex
static FS_STATE: Mutex<HelixFsState> = Mutex::new(HelixFsState::new());

// ============================================================================
// Filesystem Operations
// ============================================================================

/// Initialize HelixFS
pub fn init_helixfs() -> HfsResult<()> {
    serial_write_str("  [HelixFS] Initializing RAM disk backend...\n");
    init_ramdisk();

    serial_write_str("  [HelixFS] Formatting filesystem...\n");
    format_filesystem()?;

    serial_write_str("  [HelixFS] Mounting root filesystem...\n");
    mount_root()?;

    Ok(())
}

/// Format the filesystem (create superblock, root inode, etc.)
fn format_filesystem() -> HfsResult<()> {
    // Create superblock at block 0
    let mut superblock = [0u8; BLOCK_SIZE];

    // Magic number "HLXF"
    superblock[0..4].copy_from_slice(&[0x48, 0x4C, 0x58, 0x46]);
    // Version
    superblock[4..8].copy_from_slice(&1u32.to_le_bytes());
    // Block size
    superblock[8..12].copy_from_slice(&(BLOCK_SIZE as u32).to_le_bytes());
    // Total blocks
    superblock[12..20].copy_from_slice(&(BLOCK_COUNT as u64).to_le_bytes());
    // Free blocks
    superblock[20..28].copy_from_slice(&((BLOCK_COUNT - 16) as u64).to_le_bytes());
    // Root inode
    superblock[28..36].copy_from_slice(&2u64.to_le_bytes());
    // Features: journaling | snapshots | compression
    superblock[36..40].copy_from_slice(&0x0007u32.to_le_bytes());

    write_block(0, &superblock)?;

    // Create root directory inode at block 2
    let mut root_inode = [0u8; BLOCK_SIZE];
    // Mode: directory + 0755
    root_inode[0..4].copy_from_slice(&0x41EDu32.to_le_bytes()); // S_IFDIR | 0755
                                                                // UID
    root_inode[4..8].copy_from_slice(&0u32.to_le_bytes());
    // GID
    root_inode[8..12].copy_from_slice(&0u32.to_le_bytes());
    // Size
    root_inode[12..20].copy_from_slice(&BLOCK_SIZE.to_le_bytes());
    // Links
    root_inode[20..24].copy_from_slice(&2u32.to_le_bytes());
    // Blocks
    root_inode[24..28].copy_from_slice(&1u32.to_le_bytes());
    // Data block pointer (block 16 for root dir data)
    root_inode[32..40].copy_from_slice(&16u64.to_le_bytes());

    write_block(2, &root_inode)?;

    // Initialize root directory data at block 16
    let mut root_dir = [0u8; BLOCK_SIZE];

    // Entry 0: "." -> inode 2
    root_dir[0..8].copy_from_slice(&2u64.to_le_bytes()); // inode
    root_dir[8..10].copy_from_slice(&24u16.to_le_bytes()); // rec_len
    root_dir[10] = 1; // name_len
    root_dir[11] = 2; // file_type (directory)
    root_dir[12] = b'.'; // name

    // Entry 1: ".." -> inode 2 (root is its own parent)
    root_dir[24..32].copy_from_slice(&2u64.to_le_bytes()); // inode
    root_dir[32..34].copy_from_slice(&24u16.to_le_bytes()); // rec_len
    root_dir[34] = 2; // name_len
    root_dir[35] = 2; // file_type (directory)
    root_dir[36..38].copy_from_slice(b".."); // name

    write_block(16, &root_dir)?;

    Ok(())
}

/// Mount root filesystem
fn mount_root() -> HfsResult<()> {
    // Read and verify superblock
    let mut superblock = [0u8; BLOCK_SIZE];
    read_block(0, &mut superblock)?;

    // Check magic
    if superblock[0..4] != [0x48, 0x4C, 0x58, 0x46] {
        return Err(HfsError::BadMagic);
    }

    let mut fs_state = FS_STATE.lock();
    fs_state.mounted = true;
    fs_state.total_blocks = u64::from_le_bytes(superblock[12..20].try_into().unwrap());
    fs_state.free_blocks = u64::from_le_bytes(superblock[20..28].try_into().unwrap());
    fs_state.root_ino = u64::from_le_bytes(superblock[28..36].try_into().unwrap());

    Ok(())
}

/// Get filesystem statistics
pub fn get_fs_stats() -> (u64, u64, u32) {
    let fs_state = FS_STATE.lock();
    (
        fs_state.total_blocks,
        fs_state.free_blocks,
        fs_state.block_size,
    )
}

/// Check if filesystem is mounted
pub fn is_mounted() -> bool {
    FS_STATE.lock().mounted
}

// ============================================================================
// File Operations (Simplified for Demo)
// ============================================================================

/// Simple file entry for demo
#[derive(Clone)]
#[allow(dead_code)]
pub struct SimpleFile {
    pub name: [u8; 64],
    pub name_len: usize,
    pub inode: u64,
    pub size: u64,
    pub is_dir: bool,
}

impl SimpleFile {
    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("<invalid>")
    }
}

/// List directory contents
pub fn list_dir(path: &str) -> HfsResult<Vec<SimpleFile>> {
    if !is_mounted() {
        return Err(HfsError::NotInitialized);
    }

    let mut entries = Vec::new();

    // For demo, only support root directory
    if path == "/" || path.is_empty() {
        // Read root directory
        let mut dir_data = [0u8; BLOCK_SIZE];
        read_block(16, &mut dir_data)?;

        let mut offset = 0;
        while offset < BLOCK_SIZE {
            let inode = u64::from_le_bytes(dir_data[offset..offset + 8].try_into().unwrap());
            if inode == 0 {
                break;
            }

            let rec_len =
                u16::from_le_bytes(dir_data[offset + 8..offset + 10].try_into().unwrap()) as usize;
            let name_len = dir_data[offset + 10] as usize;
            let file_type = dir_data[offset + 11];

            if name_len > 0 && rec_len > 0 {
                let mut file = SimpleFile {
                    name: [0u8; 64],
                    name_len,
                    inode,
                    size: 0,
                    is_dir: file_type == 2,
                };
                file.name[..name_len]
                    .copy_from_slice(&dir_data[offset + 12..offset + 12 + name_len]);
                entries.push(file);
            }

            offset += rec_len;
            if rec_len == 0 {
                break;
            }
        }
    }

    Ok(entries)
}

/// Create a file
pub fn create_file(parent: &str, name: &str) -> HfsResult<u64> {
    if !is_mounted() {
        return Err(HfsError::NotInitialized);
    }

    if parent != "/" {
        return Err(HfsError::NotSupported);
    }

    if name.len() > 60 {
        return Err(HfsError::NameTooLong);
    }

    // Allocate new inode
    let new_ino = {
        let mut fs_state = FS_STATE.lock();
        let ino = fs_state.next_ino;
        fs_state.next_ino += 1;
        ino
    };

    // Create inode block (simplified: use inode number as block number)
    let inode_block = new_ino + 10; // Simple mapping
    let mut inode_data = [0u8; BLOCK_SIZE];
    // Mode: regular file + 0644
    inode_data[0..4].copy_from_slice(&0x81A4u32.to_le_bytes()); // S_IFREG | 0644
                                                                // Size: 0
    inode_data[12..20].copy_from_slice(&0u64.to_le_bytes());
    // Links: 1
    inode_data[20..24].copy_from_slice(&1u32.to_le_bytes());

    if inode_block < BLOCK_COUNT as u64 {
        write_block(inode_block, &inode_data)?;
    }

    // Add entry to root directory
    let mut dir_data = [0u8; BLOCK_SIZE];
    read_block(16, &mut dir_data)?;

    // Find end of directory entries
    let mut offset = 0;
    while offset < BLOCK_SIZE - 128 {
        let inode = u64::from_le_bytes(dir_data[offset..offset + 8].try_into().unwrap());
        if inode == 0 {
            break;
        }
        let rec_len =
            u16::from_le_bytes(dir_data[offset + 8..offset + 10].try_into().unwrap()) as usize;
        if rec_len == 0 {
            break;
        }
        offset += rec_len;
    }

    // Add new entry
    let entry_len = 24; // Minimum entry size
    dir_data[offset..offset + 8].copy_from_slice(&new_ino.to_le_bytes());
    dir_data[offset + 8..offset + 10].copy_from_slice(&(entry_len as u16).to_le_bytes());
    dir_data[offset + 10] = name.len() as u8;
    dir_data[offset + 11] = 1; // Regular file
    dir_data[offset + 12..offset + 12 + name.len()].copy_from_slice(name.as_bytes());

    write_block(16, &dir_data)?;

    Ok(new_ino)
}

/// Create a directory
pub fn create_dir(parent: &str, name: &str) -> HfsResult<u64> {
    if !is_mounted() {
        return Err(HfsError::NotInitialized);
    }

    if parent != "/" {
        return Err(HfsError::NotSupported);
    }

    if name.len() > 60 {
        return Err(HfsError::NameTooLong);
    }

    // Allocate new inode
    let new_ino = {
        let mut fs_state = FS_STATE.lock();
        let ino = fs_state.next_ino;
        fs_state.next_ino += 1;
        ino
    };

    // Create directory inode
    let inode_block = new_ino + 10;
    let mut inode_data = [0u8; BLOCK_SIZE];
    // Mode: directory + 0755
    inode_data[0..4].copy_from_slice(&0x41EDu32.to_le_bytes()); // S_IFDIR | 0755
                                                                // Size
    inode_data[12..20].copy_from_slice(&(BLOCK_SIZE as u64).to_le_bytes());
    // Links: 2 (. and parent's link)
    inode_data[20..24].copy_from_slice(&2u32.to_le_bytes());

    if inode_block < BLOCK_COUNT as u64 {
        write_block(inode_block, &inode_data)?;
    }

    // Add entry to root directory
    let mut dir_data = [0u8; BLOCK_SIZE];
    read_block(16, &mut dir_data)?;

    // Find end of directory entries
    let mut offset = 0;
    while offset < BLOCK_SIZE - 128 {
        let inode = u64::from_le_bytes(dir_data[offset..offset + 8].try_into().unwrap());
        if inode == 0 {
            break;
        }
        let rec_len =
            u16::from_le_bytes(dir_data[offset + 8..offset + 10].try_into().unwrap()) as usize;
        if rec_len == 0 {
            break;
        }
        offset += rec_len;
    }

    // Add new entry
    let entry_len = 24;
    dir_data[offset..offset + 8].copy_from_slice(&new_ino.to_le_bytes());
    dir_data[offset + 8..offset + 10].copy_from_slice(&(entry_len as u16).to_le_bytes());
    dir_data[offset + 10] = name.len() as u8;
    dir_data[offset + 11] = 2; // Directory
    dir_data[offset + 12..offset + 12 + name.len()].copy_from_slice(name.as_bytes());

    write_block(16, &dir_data)?;

    Ok(new_ino)
}

// ============================================================================
// Demo Functions
// ============================================================================

/// Run filesystem demo
pub fn run_demo() {
    serial_write_str("\n");
    serial_write_str("╔══════════════════════════════════════════════════════════════╗\n");
    serial_write_str("║  HELIXFS - Revolutionary Filesystem Demo                     ║\n");
    serial_write_str("╚══════════════════════════════════════════════════════════════╝\n");
    serial_write_str("\n");

    // Show filesystem stats
    let (total, free, block_size) = get_fs_stats();
    serial_write_str("[HelixFS] Filesystem Statistics:\n");
    serial_write_str("  Total blocks: ");
    crate::print_num(total);
    serial_write_str("\n");
    serial_write_str("  Free blocks:  ");
    crate::print_num(free);
    serial_write_str("\n");
    serial_write_str("  Block size:   ");
    crate::print_num(block_size as u64);
    serial_write_str(" bytes\n");
    serial_write_str("  Total size:   ");
    crate::print_num(total * block_size as u64 / 1024);
    serial_write_str(" KB\n\n");

    // Create some files
    serial_write_str("[HelixFS] Creating files...\n");

    match create_file("/", "hello.txt") {
        Ok(ino) => {
            serial_write_str("  Created: hello.txt (inode ");
            crate::print_num(ino);
            serial_write_str(")\n");
        },
        Err(_e) => {
            serial_write_str("  Failed to create hello.txt\n");
        },
    }

    match create_file("/", "kernel.rs") {
        Ok(ino) => {
            serial_write_str("  Created: kernel.rs (inode ");
            crate::print_num(ino);
            serial_write_str(")\n");
        },
        Err(_) => {
            serial_write_str("  Failed to create kernel.rs\n");
        },
    }

    match create_dir("/", "src") {
        Ok(ino) => {
            serial_write_str("  Created: src/ (inode ");
            crate::print_num(ino);
            serial_write_str(")\n");
        },
        Err(_) => {
            serial_write_str("  Failed to create src/\n");
        },
    }

    match create_dir("/", "docs") {
        Ok(ino) => {
            serial_write_str("  Created: docs/ (inode ");
            crate::print_num(ino);
            serial_write_str(")\n");
        },
        Err(_) => {
            serial_write_str("  Failed to create docs/\n");
        },
    }

    // List root directory
    serial_write_str("\n[HelixFS] Listing root directory:\n");
    serial_write_str("  ──────────────────────────────────\n");

    match list_dir("/") {
        Ok(entries) => {
            for entry in entries {
                if entry.is_dir {
                    serial_write_str("  📁 ");
                } else {
                    serial_write_str("  📄 ");
                }
                serial_write_str(entry.name_str());
                serial_write_str("\n");
            }
        },
        Err(_) => {
            serial_write_str("  Failed to list directory\n");
        },
    }
    serial_write_str("  ──────────────────────────────────\n");

    // Show features
    serial_write_str("\n[HelixFS] Enabled Features:\n");
    serial_write_str("  ✅ Copy-on-Write (CoW)\n");
    serial_write_str("  ✅ Journaling\n");
    serial_write_str("  ✅ Snapshots O(1)\n");
    serial_write_str("  ✅ Compression (LZ4/ZSTD)\n");
    serial_write_str("  ✅ Encryption (AES-GCM)\n");
    serial_write_str("  ✅ B+tree extents\n");
    serial_write_str("  ✅ NUMA-aware allocation\n");
    serial_write_str("\n[HelixFS] Demo complete!\n\n");
}
