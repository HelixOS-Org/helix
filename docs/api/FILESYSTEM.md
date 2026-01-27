# HelixFS Filesystem API Reference

<div align="center">

📁 **Complete Filesystem API Documentation**

*Modern Copy-on-Write Filesystem with B-Tree, Snapshots, and Compression*

</div>

---

## Table of Contents

1. [Overview](#1-overview)
2. [Core Types](#2-core-types)
3. [VFS Layer](#3-vfs-layer)
4. [File Operations](#4-file-operations)
5. [Directory Operations](#5-directory-operations)
6. [B-Tree Structure](#6-b-tree-structure)
7. [Journaling](#7-journaling)
8. [Snapshots](#8-snapshots)
9. [Compression](#9-compression)
10. [Disk Interface](#10-disk-interface)

---

## 1. Overview

### 1.1 HelixFS Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         HELIXFS ARCHITECTURE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Application Layer                                                          │
│  ═════════════════                                                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  open()  read()  write()  close()  stat()  mkdir()  unlink()  ...  │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  VFS Layer                                                                  │
│  ═════════                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Virtual File System                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │  Path Res.  │  │  Mount Mgr  │  │  File Cache │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  HelixFS Core                                                               │
│  ═══════════                                                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │   │
│  │  │   B-Tree      │  │   Journaling  │  │   Snapshots   │           │   │
│  │  │   Engine      │  │   System      │  │   Manager     │           │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘           │   │
│  │                                                                     │   │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │   │
│  │  │  Compression  │  │   Encryption  │  │  Allocator    │           │   │
│  │  │   Engine      │  │   Layer       │  │               │           │   │
│  │  └───────────────┘  └───────────────┘  └───────────────┘           │   │
│  │                                                                     │   │
│  └────────────────────────────────┬────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  Block Layer                                                                │
│  ═══════════                                                                │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │   │
│  │  │ Block Cache │  │  I/O Queue  │  │ Disk Driver │                 │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Crate Structure

```
helix-fs/
├── Cargo.toml
├── include/
│   └── helixfs/           # C API headers
└── src/
    ├── lib.rs             # Crate root
    ├── alloc/             # Block allocators
    │   ├── mod.rs
    │   ├── bitmap.rs
    │   └── extent.rs
    ├── api/               # Public API
    │   ├── mod.rs
    │   ├── file.rs
    │   └── directory.rs
    ├── cache/             # Caching layer
    │   ├── mod.rs
    │   ├── block_cache.rs
    │   └── inode_cache.rs
    ├── compress/          # Compression
    │   ├── mod.rs
    │   ├── lz4.rs
    │   └── zstd.rs
    ├── core/              # Core types
    │   ├── mod.rs
    │   ├── superblock.rs
    │   ├── inode.rs
    │   └── extent.rs
    ├── crypto/            # Encryption
    │   ├── mod.rs
    │   └── aes.rs
    ├── disk/              # Disk I/O
    │   ├── mod.rs
    │   └── block_device.rs
    ├── journal/           # Journaling
    │   ├── mod.rs
    │   └── transaction.rs
    ├── ops/               # Operations
    │   ├── mod.rs
    │   ├── read.rs
    │   └── write.rs
    ├── snapshot/          # Snapshots
    │   ├── mod.rs
    │   └── cow.rs
    ├── tree/              # B-Tree
    │   ├── mod.rs
    │   ├── node.rs
    │   └── operations.rs
    └── vfs/               # VFS layer
        ├── mod.rs
        ├── mount.rs
        └── path.rs
```

---

## 2. Core Types

### 2.1 Fundamental Types

```rust
/// Inode number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeNum(pub u64);

impl InodeNum {
    /// Root inode
    pub const ROOT: InodeNum = InodeNum(2);
    
    /// Invalid inode
    pub const INVALID: InodeNum = InodeNum(0);
}

/// Block number
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockNum(pub u64);

/// File handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileHandle(pub u64);

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Entry name
    pub name: String,
    /// Inode number
    pub inode: InodeNum,
    /// Entry type
    pub file_type: FileType,
}

/// File types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    BlockDevice,
    CharDevice,
    Fifo,
    Socket,
}

/// File mode (permissions)
#[derive(Debug, Clone, Copy)]
pub struct FileMode(pub u32);

impl FileMode {
    // Permission bits
    pub const S_IRWXU: u32 = 0o700;  // User RWX
    pub const S_IRUSR: u32 = 0o400;  // User read
    pub const S_IWUSR: u32 = 0o200;  // User write
    pub const S_IXUSR: u32 = 0o100;  // User execute
    pub const S_IRWXG: u32 = 0o070;  // Group RWX
    pub const S_IRWXO: u32 = 0o007;  // Other RWX
    
    // Type bits
    pub const S_IFREG: u32 = 0o100000;  // Regular file
    pub const S_IFDIR: u32 = 0o040000;  // Directory
    pub const S_IFLNK: u32 = 0o120000;  // Symlink
    
    /// Create mode from permissions
    pub const fn new(perms: u32) -> Self {
        Self(perms)
    }
    
    /// Get file type
    pub fn file_type(&self) -> FileType {
        match self.0 & 0o170000 {
            0o040000 => FileType::Directory,
            0o100000 => FileType::Regular,
            0o120000 => FileType::Symlink,
            _ => FileType::Regular,
        }
    }
    
    /// Check if readable by owner
    pub fn owner_read(&self) -> bool {
        self.0 & Self::S_IRUSR != 0
    }
    
    /// Check if writable by owner
    pub fn owner_write(&self) -> bool {
        self.0 & Self::S_IWUSR != 0
    }
    
    /// Check if executable by owner
    pub fn owner_execute(&self) -> bool {
        self.0 & Self::S_IXUSR != 0
    }
}

/// Timestamp
#[derive(Debug, Clone, Copy, Default)]
pub struct Timestamp {
    /// Seconds since epoch
    pub secs: i64,
    /// Nanoseconds
    pub nsecs: u32,
}

impl Timestamp {
    /// Create from seconds
    pub const fn from_secs(secs: i64) -> Self {
        Self { secs, nsecs: 0 }
    }
    
    /// Get current time (placeholder)
    pub fn now() -> Self {
        Self { secs: 0, nsecs: 0 }
    }
}

/// File statistics
#[derive(Debug, Clone)]
pub struct FileStat {
    /// Inode number
    pub inode: InodeNum,
    /// File mode
    pub mode: FileMode,
    /// Number of hard links
    pub nlink: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// File size in bytes
    pub size: u64,
    /// Block size
    pub blksize: u32,
    /// Blocks allocated
    pub blocks: u64,
    /// Access time
    pub atime: Timestamp,
    /// Modification time
    pub mtime: Timestamp,
    /// Change time
    pub ctime: Timestamp,
    /// Creation time
    pub crtime: Timestamp,
}
```

### 2.2 Inode Structure

```rust
/// On-disk inode structure
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Inode {
    /// Inode number
    pub ino: InodeNum,
    
    /// File mode and type
    pub mode: FileMode,
    
    /// Owner user ID
    pub uid: u32,
    
    /// Owner group ID
    pub gid: u32,
    
    /// File size in bytes
    pub size: u64,
    
    /// Link count
    pub nlink: u32,
    
    /// Flags
    pub flags: InodeFlags,
    
    /// Generation number (for NFS)
    pub generation: u32,
    
    /// Access time
    pub atime: Timestamp,
    
    /// Modification time
    pub mtime: Timestamp,
    
    /// Inode change time
    pub ctime: Timestamp,
    
    /// Creation time
    pub crtime: Timestamp,
    
    /// Data storage
    pub data: InodeData,
}

bitflags! {
    /// Inode flags
    pub struct InodeFlags: u32 {
        /// Compressed data
        const COMPRESSED = 1 << 0;
        /// Encrypted data
        const ENCRYPTED = 1 << 1;
        /// Copy-on-write
        const COW = 1 << 2;
        /// Immutable
        const IMMUTABLE = 1 << 3;
        /// Append only
        const APPEND = 1 << 4;
        /// No dump
        const NODUMP = 1 << 5;
        /// Inline data
        const INLINE = 1 << 6;
    }
}

/// Inode data storage
#[derive(Debug, Clone)]
pub enum InodeData {
    /// Inline data (small files)
    Inline(InlineData),
    
    /// Extent tree for larger files
    Extents(ExtentTree),
    
    /// Directory entries
    Directory(DirData),
    
    /// Symbolic link target
    Symlink(String),
}

/// Inline data for small files (< 60 bytes)
#[derive(Debug, Clone)]
pub struct InlineData {
    pub data: [u8; 60],
    pub len: u8,
}

impl InlineData {
    pub fn new(data: &[u8]) -> Option<Self> {
        if data.len() > 60 {
            return None;
        }
        
        let mut inline = Self {
            data: [0u8; 60],
            len: data.len() as u8,
        };
        inline.data[..data.len()].copy_from_slice(data);
        Some(inline)
    }
    
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

/// Extent for file data
#[derive(Debug, Clone, Copy)]
pub struct Extent {
    /// File offset (in blocks)
    pub file_block: u64,
    /// Start block on disk
    pub disk_block: BlockNum,
    /// Number of blocks
    pub length: u32,
    /// Extent flags
    pub flags: ExtentFlags,
}

bitflags! {
    pub struct ExtentFlags: u16 {
        /// Extent is unwritten (hole)
        const UNWRITTEN = 1 << 0;
        /// Extent is compressed
        const COMPRESSED = 1 << 1;
        /// Extent is encrypted
        const ENCRYPTED = 1 << 2;
    }
}

/// Extent tree root
#[derive(Debug, Clone)]
pub struct ExtentTree {
    /// Tree depth (0 = leaf extents in inode)
    pub depth: u8,
    /// Maximum entries
    pub max_entries: u16,
    /// Current entries
    pub entries: u16,
    /// Extents or index nodes
    pub nodes: Vec<ExtentNode>,
}

/// Extent tree node
#[derive(Debug, Clone)]
pub enum ExtentNode {
    /// Leaf node with extent
    Leaf(Extent),
    /// Index node pointing to block
    Index {
        file_block: u64,
        block: BlockNum,
    },
}
```

### 2.3 Superblock

```rust
/// Filesystem superblock
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Superblock {
    /// Magic number
    pub magic: u32,
    
    /// Filesystem version
    pub version: u32,
    
    /// Block size (bytes)
    pub block_size: u32,
    
    /// Total blocks
    pub total_blocks: u64,
    
    /// Free blocks
    pub free_blocks: u64,
    
    /// Total inodes
    pub total_inodes: u64,
    
    /// Free inodes
    pub free_inodes: u64,
    
    /// Root inode number
    pub root_inode: InodeNum,
    
    /// First data block
    pub first_data_block: BlockNum,
    
    /// Blocks per group
    pub blocks_per_group: u32,
    
    /// Inodes per group
    pub inodes_per_group: u32,
    
    /// Number of block groups
    pub group_count: u32,
    
    /// Filesystem state
    pub state: FsState,
    
    /// Features
    pub features: FsFeatures,
    
    /// UUID
    pub uuid: [u8; 16],
    
    /// Volume name
    pub volume_name: [u8; 64],
    
    /// Last mount time
    pub last_mount_time: Timestamp,
    
    /// Last write time
    pub last_write_time: Timestamp,
    
    /// Mount count
    pub mount_count: u16,
    
    /// Max mount count before check
    pub max_mount_count: u16,
    
    /// Journal inode
    pub journal_inode: InodeNum,
    
    /// Snapshot root inode
    pub snapshot_root: InodeNum,
}

impl Superblock {
    /// HelixFS magic number
    pub const MAGIC: u32 = 0x48454C58; // "HELX"
    
    /// Default block size
    pub const DEFAULT_BLOCK_SIZE: u32 = 4096;
    
    /// Validate superblock
    pub fn validate(&self) -> FsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(FsError::InvalidMagic);
        }
        
        if !self.block_size.is_power_of_two() {
            return Err(FsError::InvalidBlockSize);
        }
        
        if self.block_size < 512 || self.block_size > 65536 {
            return Err(FsError::InvalidBlockSize);
        }
        
        Ok(())
    }
}

/// Filesystem state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum FsState {
    /// Cleanly unmounted
    Clean = 0,
    /// Mounted or dirty
    Dirty = 1,
    /// Errors detected
    Errors = 2,
}

bitflags! {
    /// Filesystem features
    pub struct FsFeatures: u64 {
        /// Compression support
        const COMPRESSION = 1 << 0;
        /// Encryption support
        const ENCRYPTION = 1 << 1;
        /// Journaling
        const JOURNAL = 1 << 2;
        /// Snapshots
        const SNAPSHOTS = 1 << 3;
        /// Extents
        const EXTENTS = 1 << 4;
        /// Large files (> 2GB)
        const LARGE_FILES = 1 << 5;
        /// Sparse files
        const SPARSE = 1 << 6;
        /// B-Tree directories
        const BTREE_DIR = 1 << 7;
        /// Inline data
        const INLINE_DATA = 1 << 8;
    }
}
```

---

## 3. VFS Layer

### 3.1 VFS Operations

```rust
/// Virtual File System interface
pub trait VfsOps: Send + Sync {
    /// Mount the filesystem
    fn mount(&mut self, device: &dyn BlockDevice, options: MountOptions) -> FsResult<()>;
    
    /// Unmount the filesystem
    fn unmount(&mut self) -> FsResult<()>;
    
    /// Sync all pending writes
    fn sync(&self) -> FsResult<()>;
    
    /// Get filesystem statistics
    fn statfs(&self) -> FsResult<FsStats>;
    
    /// Lookup inode by path
    fn lookup(&self, path: &str) -> FsResult<InodeNum>;
    
    /// Get inode
    fn get_inode(&self, ino: InodeNum) -> FsResult<Inode>;
    
    /// Create file
    fn create(&mut self, parent: InodeNum, name: &str, mode: FileMode) -> FsResult<InodeNum>;
    
    /// Create directory
    fn mkdir(&mut self, parent: InodeNum, name: &str, mode: FileMode) -> FsResult<InodeNum>;
    
    /// Remove file
    fn unlink(&mut self, parent: InodeNum, name: &str) -> FsResult<()>;
    
    /// Remove directory
    fn rmdir(&mut self, parent: InodeNum, name: &str) -> FsResult<()>;
    
    /// Rename
    fn rename(&mut self, old_parent: InodeNum, old_name: &str, 
              new_parent: InodeNum, new_name: &str) -> FsResult<()>;
    
    /// Create hard link
    fn link(&mut self, inode: InodeNum, new_parent: InodeNum, new_name: &str) -> FsResult<()>;
    
    /// Create symbolic link
    fn symlink(&mut self, parent: InodeNum, name: &str, target: &str) -> FsResult<InodeNum>;
    
    /// Read symbolic link
    fn readlink(&self, inode: InodeNum) -> FsResult<String>;
    
    /// Read directory entries
    fn readdir(&self, inode: InodeNum) -> FsResult<Vec<DirEntry>>;
}

/// Mount options
#[derive(Debug, Clone, Default)]
pub struct MountOptions {
    /// Read-only mount
    pub read_only: bool,
    /// No access time updates
    pub noatime: bool,
    /// Synchronous writes
    pub sync: bool,
    /// Enable compression
    pub compress: bool,
    /// Compression algorithm
    pub compress_algo: CompressionAlgo,
    /// Enable encryption
    pub encrypt: bool,
    /// User ID for all files
    pub uid: Option<u32>,
    /// Group ID for all files
    pub gid: Option<u32>,
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FsStats {
    /// Block size
    pub block_size: u64,
    /// Total blocks
    pub total_blocks: u64,
    /// Free blocks
    pub free_blocks: u64,
    /// Available blocks (for non-root)
    pub available_blocks: u64,
    /// Total inodes
    pub total_inodes: u64,
    /// Free inodes
    pub free_inodes: u64,
    /// Maximum filename length
    pub max_name_len: u32,
}
```

### 3.2 Path Resolution

```rust
/// Path resolver
pub struct PathResolver<'a> {
    fs: &'a dyn VfsOps,
}

impl<'a> PathResolver<'a> {
    /// Create new resolver
    pub fn new(fs: &'a dyn VfsOps) -> Self {
        Self { fs }
    }
    
    /// Resolve path to inode
    pub fn resolve(&self, path: &str) -> FsResult<InodeNum> {
        if path.is_empty() {
            return Err(FsError::InvalidPath);
        }
        
        let components = self.parse_path(path)?;
        let mut current = InodeNum::ROOT;
        
        for component in components {
            match component {
                PathComponent::Root => current = InodeNum::ROOT,
                PathComponent::Current => continue,
                PathComponent::Parent => {
                    current = self.get_parent(current)?;
                }
                PathComponent::Name(name) => {
                    current = self.lookup_in_dir(current, name)?;
                }
            }
        }
        
        Ok(current)
    }
    
    /// Resolve parent directory and final component
    pub fn resolve_parent(&self, path: &str) -> FsResult<(InodeNum, String)> {
        let (parent_path, name) = self.split_path(path)?;
        let parent = self.resolve(parent_path)?;
        Ok((parent, name.to_string()))
    }
    
    fn parse_path(&self, path: &str) -> FsResult<Vec<PathComponent>> {
        let mut components = Vec::new();
        
        if path.starts_with('/') {
            components.push(PathComponent::Root);
        }
        
        for part in path.split('/') {
            match part {
                "" | "." => components.push(PathComponent::Current),
                ".." => components.push(PathComponent::Parent),
                name => components.push(PathComponent::Name(name)),
            }
        }
        
        Ok(components)
    }
    
    fn split_path<'b>(&self, path: &'b str) -> FsResult<(&'b str, &'b str)> {
        if let Some(pos) = path.rfind('/') {
            let parent = if pos == 0 { "/" } else { &path[..pos] };
            let name = &path[pos + 1..];
            if name.is_empty() {
                return Err(FsError::InvalidPath);
            }
            Ok((parent, name))
        } else {
            Ok((".", path))
        }
    }
    
    fn get_parent(&self, inode: InodeNum) -> FsResult<InodeNum> {
        self.lookup_in_dir(inode, "..")
    }
    
    fn lookup_in_dir(&self, dir: InodeNum, name: &str) -> FsResult<InodeNum> {
        let entries = self.fs.readdir(dir)?;
        
        for entry in entries {
            if entry.name == name {
                return Ok(entry.inode);
            }
        }
        
        Err(FsError::NotFound)
    }
}

/// Path component
enum PathComponent<'a> {
    Root,
    Current,
    Parent,
    Name(&'a str),
}
```

---

## 4. File Operations

### 4.1 File API

```rust
/// File operations
pub trait FileOps {
    /// Open a file
    fn open(&mut self, path: &str, flags: OpenFlags) -> FsResult<FileHandle>;
    
    /// Close a file
    fn close(&mut self, handle: FileHandle) -> FsResult<()>;
    
    /// Read from file
    fn read(&self, handle: FileHandle, buf: &mut [u8]) -> FsResult<usize>;
    
    /// Read at offset
    fn read_at(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> FsResult<usize>;
    
    /// Write to file
    fn write(&mut self, handle: FileHandle, buf: &[u8]) -> FsResult<usize>;
    
    /// Write at offset
    fn write_at(&mut self, handle: FileHandle, offset: u64, buf: &[u8]) -> FsResult<usize>;
    
    /// Seek in file
    fn seek(&mut self, handle: FileHandle, pos: SeekFrom) -> FsResult<u64>;
    
    /// Get file position
    fn tell(&self, handle: FileHandle) -> FsResult<u64>;
    
    /// Truncate file
    fn truncate(&mut self, handle: FileHandle, size: u64) -> FsResult<()>;
    
    /// Sync file to disk
    fn fsync(&self, handle: FileHandle) -> FsResult<()>;
    
    /// Get file status
    fn fstat(&self, handle: FileHandle) -> FsResult<FileStat>;
}

bitflags! {
    /// File open flags
    pub struct OpenFlags: u32 {
        /// Read-only
        const O_RDONLY = 0;
        /// Write-only
        const O_WRONLY = 1;
        /// Read-write
        const O_RDWR = 2;
        /// Create if not exists
        const O_CREAT = 1 << 6;
        /// Exclusive create
        const O_EXCL = 1 << 7;
        /// Truncate to zero
        const O_TRUNC = 1 << 9;
        /// Append mode
        const O_APPEND = 1 << 10;
        /// Non-blocking
        const O_NONBLOCK = 1 << 11;
        /// Sync writes
        const O_SYNC = 1 << 12;
        /// Directory
        const O_DIRECTORY = 1 << 16;
    }
}

/// Seek position
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    /// From beginning
    Start(u64),
    /// From end
    End(i64),
    /// From current position
    Current(i64),
}

/// Open file descriptor
pub struct OpenFile {
    /// File handle
    pub handle: FileHandle,
    /// Inode
    pub inode: InodeNum,
    /// Open flags
    pub flags: OpenFlags,
    /// Current position
    pub position: u64,
}

impl OpenFile {
    /// Check if readable
    pub fn can_read(&self) -> bool {
        !self.flags.contains(OpenFlags::O_WRONLY)
    }
    
    /// Check if writable
    pub fn can_write(&self) -> bool {
        self.flags.intersects(OpenFlags::O_WRONLY | OpenFlags::O_RDWR)
    }
}
```

### 4.2 File Implementation

```rust
/// File manager implementation
pub struct FileManager {
    /// Open file table
    open_files: HashMap<FileHandle, OpenFile>,
    
    /// Next handle
    next_handle: AtomicU64,
    
    /// Inode cache
    inode_cache: InodeCache,
    
    /// Block cache
    block_cache: BlockCache,
}

impl FileManager {
    /// Create new file manager
    pub fn new(inode_cache: InodeCache, block_cache: BlockCache) -> Self {
        Self {
            open_files: HashMap::new(),
            next_handle: AtomicU64::new(1),
            inode_cache,
            block_cache,
        }
    }
    
    /// Allocate new handle
    fn alloc_handle(&self) -> FileHandle {
        FileHandle(self.next_handle.fetch_add(1, Ordering::SeqCst))
    }
}

impl FileOps for FileManager {
    fn open(&mut self, path: &str, flags: OpenFlags) -> FsResult<FileHandle> {
        // Resolve path to inode
        let inode_num = if flags.contains(OpenFlags::O_CREAT) {
            // Create file if needed
            match self.lookup(path) {
                Ok(ino) => {
                    if flags.contains(OpenFlags::O_EXCL) {
                        return Err(FsError::AlreadyExists);
                    }
                    ino
                }
                Err(FsError::NotFound) => {
                    self.create_file(path, FileMode::new(0o644))?
                }
                Err(e) => return Err(e),
            }
        } else {
            self.lookup(path)?
        };
        
        // Get inode
        let inode = self.inode_cache.get(inode_num)?;
        
        // Validate flags
        if flags.contains(OpenFlags::O_DIRECTORY) && 
           inode.mode.file_type() != FileType::Directory {
            return Err(FsError::NotDirectory);
        }
        
        // Handle truncate
        if flags.contains(OpenFlags::O_TRUNC) && self.can_write_flags(flags) {
            self.do_truncate(inode_num, 0)?;
        }
        
        // Create file descriptor
        let handle = self.alloc_handle();
        let open_file = OpenFile {
            handle,
            inode: inode_num,
            flags,
            position: 0,
        };
        
        self.open_files.insert(handle, open_file);
        
        Ok(handle)
    }
    
    fn close(&mut self, handle: FileHandle) -> FsResult<()> {
        self.open_files.remove(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        Ok(())
    }
    
    fn read(&self, handle: FileHandle, buf: &mut [u8]) -> FsResult<usize> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        if !file.can_read() {
            return Err(FsError::PermissionDenied);
        }
        
        let bytes_read = self.do_read(file.inode, file.position, buf)?;
        
        // Update position (need mutable)
        // file.position += bytes_read as u64;
        
        Ok(bytes_read)
    }
    
    fn read_at(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> FsResult<usize> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        if !file.can_read() {
            return Err(FsError::PermissionDenied);
        }
        
        self.do_read(file.inode, offset, buf)
    }
    
    fn write(&mut self, handle: FileHandle, buf: &[u8]) -> FsResult<usize> {
        let file = self.open_files.get_mut(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        if !file.can_write() {
            return Err(FsError::PermissionDenied);
        }
        
        let offset = if file.flags.contains(OpenFlags::O_APPEND) {
            let inode = self.inode_cache.get(file.inode)?;
            inode.size
        } else {
            file.position
        };
        
        let bytes_written = self.do_write(file.inode, offset, buf)?;
        file.position = offset + bytes_written as u64;
        
        Ok(bytes_written)
    }
    
    fn write_at(&mut self, handle: FileHandle, offset: u64, buf: &[u8]) -> FsResult<usize> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        if !file.can_write() {
            return Err(FsError::PermissionDenied);
        }
        
        self.do_write(file.inode, offset, buf)
    }
    
    fn seek(&mut self, handle: FileHandle, pos: SeekFrom) -> FsResult<u64> {
        let file = self.open_files.get_mut(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        let inode = self.inode_cache.get(file.inode)?;
        
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset < 0 {
                    inode.size.checked_sub((-offset) as u64)
                        .ok_or(FsError::InvalidSeek)?
                } else {
                    inode.size + offset as u64
                }
            }
            SeekFrom::Current(offset) => {
                if offset < 0 {
                    file.position.checked_sub((-offset) as u64)
                        .ok_or(FsError::InvalidSeek)?
                } else {
                    file.position + offset as u64
                }
            }
        };
        
        file.position = new_pos;
        Ok(new_pos)
    }
    
    fn tell(&self, handle: FileHandle) -> FsResult<u64> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        Ok(file.position)
    }
    
    fn truncate(&mut self, handle: FileHandle, size: u64) -> FsResult<()> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        if !file.can_write() {
            return Err(FsError::PermissionDenied);
        }
        
        self.do_truncate(file.inode, size)
    }
    
    fn fsync(&self, handle: FileHandle) -> FsResult<()> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        // Flush inode and data blocks
        self.inode_cache.flush(file.inode)?;
        self.block_cache.flush()?;
        
        Ok(())
    }
    
    fn fstat(&self, handle: FileHandle) -> FsResult<FileStat> {
        let file = self.open_files.get(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        let inode = self.inode_cache.get(file.inode)?;
        
        Ok(FileStat {
            inode: file.inode,
            mode: inode.mode,
            nlink: inode.nlink,
            uid: inode.uid,
            gid: inode.gid,
            size: inode.size,
            blksize: 4096,
            blocks: (inode.size + 511) / 512,
            atime: inode.atime,
            mtime: inode.mtime,
            ctime: inode.ctime,
            crtime: inode.crtime,
        })
    }
}
```

---

## 5. Directory Operations

### 5.1 Directory API

```rust
/// Directory operations
pub trait DirectoryOps {
    /// Open directory
    fn opendir(&mut self, path: &str) -> FsResult<DirHandle>;
    
    /// Close directory
    fn closedir(&mut self, handle: DirHandle) -> FsResult<()>;
    
    /// Read next entry
    fn readdir_next(&mut self, handle: DirHandle) -> FsResult<Option<DirEntry>>;
    
    /// Rewind directory
    fn rewinddir(&mut self, handle: DirHandle) -> FsResult<()>;
    
    /// Create directory
    fn mkdir(&mut self, path: &str, mode: FileMode) -> FsResult<()>;
    
    /// Remove directory
    fn rmdir(&mut self, path: &str) -> FsResult<()>;
}

/// Directory handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DirHandle(pub u64);

/// Open directory state
pub struct OpenDirectory {
    /// Handle
    pub handle: DirHandle,
    /// Inode number
    pub inode: InodeNum,
    /// Current position
    pub position: usize,
    /// Cached entries
    pub entries: Vec<DirEntry>,
}

/// Directory implementation
pub struct DirectoryManager {
    /// Open directories
    open_dirs: HashMap<DirHandle, OpenDirectory>,
    
    /// Next handle
    next_handle: AtomicU64,
    
    /// Filesystem reference
    fs: Arc<dyn VfsOps>,
}

impl DirectoryOps for DirectoryManager {
    fn opendir(&mut self, path: &str) -> FsResult<DirHandle> {
        // Resolve path
        let inode_num = self.fs.lookup(path)?;
        
        // Verify it's a directory
        let inode = self.fs.get_inode(inode_num)?;
        if inode.mode.file_type() != FileType::Directory {
            return Err(FsError::NotDirectory);
        }
        
        // Read entries
        let entries = self.fs.readdir(inode_num)?;
        
        // Create handle
        let handle = DirHandle(self.next_handle.fetch_add(1, Ordering::SeqCst));
        
        self.open_dirs.insert(handle, OpenDirectory {
            handle,
            inode: inode_num,
            position: 0,
            entries,
        });
        
        Ok(handle)
    }
    
    fn closedir(&mut self, handle: DirHandle) -> FsResult<()> {
        self.open_dirs.remove(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        Ok(())
    }
    
    fn readdir_next(&mut self, handle: DirHandle) -> FsResult<Option<DirEntry>> {
        let dir = self.open_dirs.get_mut(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        if dir.position >= dir.entries.len() {
            return Ok(None);
        }
        
        let entry = dir.entries[dir.position].clone();
        dir.position += 1;
        
        Ok(Some(entry))
    }
    
    fn rewinddir(&mut self, handle: DirHandle) -> FsResult<()> {
        let dir = self.open_dirs.get_mut(&handle)
            .ok_or(FsError::BadFileDescriptor)?;
        
        dir.position = 0;
        Ok(())
    }
    
    fn mkdir(&mut self, path: &str, mode: FileMode) -> FsResult<()> {
        // Split path
        let (parent_path, name) = self.split_path(path)?;
        
        // Resolve parent
        let parent_inode = self.fs.lookup(parent_path)?;
        
        // Create directory
        self.fs.mkdir(parent_inode, name, mode)?;
        
        Ok(())
    }
    
    fn rmdir(&mut self, path: &str) -> FsResult<()> {
        // Split path
        let (parent_path, name) = self.split_path(path)?;
        
        // Resolve parent
        let parent_inode = self.fs.lookup(parent_path)?;
        
        // Remove directory
        self.fs.rmdir(parent_inode, name)?;
        
        Ok(())
    }
}
```

---

## 6. B-Tree Structure

### 6.1 B-Tree Node

```rust
/// B-Tree order (maximum children per node)
const BTREE_ORDER: usize = 256;

/// B-Tree node
#[derive(Debug, Clone)]
pub struct BTreeNode<K, V> {
    /// Keys
    pub keys: Vec<K>,
    
    /// Values (for leaf nodes)
    pub values: Vec<V>,
    
    /// Child pointers (for internal nodes)
    pub children: Vec<BlockNum>,
    
    /// Is this a leaf node?
    pub is_leaf: bool,
    
    /// Number of keys
    pub n_keys: usize,
}

impl<K: Ord + Clone, V: Clone> BTreeNode<K, V> {
    /// Create new leaf node
    pub fn new_leaf() -> Self {
        Self {
            keys: Vec::with_capacity(BTREE_ORDER - 1),
            values: Vec::with_capacity(BTREE_ORDER - 1),
            children: Vec::new(),
            is_leaf: true,
            n_keys: 0,
        }
    }
    
    /// Create new internal node
    pub fn new_internal() -> Self {
        Self {
            keys: Vec::with_capacity(BTREE_ORDER - 1),
            values: Vec::new(),
            children: Vec::with_capacity(BTREE_ORDER),
            is_leaf: false,
            n_keys: 0,
        }
    }
    
    /// Check if node is full
    pub fn is_full(&self) -> bool {
        self.n_keys >= BTREE_ORDER - 1
    }
    
    /// Search for key in node
    pub fn search(&self, key: &K) -> Result<usize, usize> {
        self.keys[..self.n_keys].binary_search(key)
    }
    
    /// Insert key-value pair in leaf
    pub fn insert_leaf(&mut self, key: K, value: V) {
        let pos = match self.search(&key) {
            Ok(i) => {
                // Key exists, update value
                self.values[i] = value;
                return;
            }
            Err(i) => i,
        };
        
        self.keys.insert(pos, key);
        self.values.insert(pos, value);
        self.n_keys += 1;
    }
    
    /// Split node
    pub fn split(&mut self) -> (K, Self) {
        let mid = self.n_keys / 2;
        let median_key = self.keys[mid].clone();
        
        let mut right = if self.is_leaf {
            let mut node = Self::new_leaf();
            node.keys = self.keys.split_off(mid + 1);
            node.values = self.values.split_off(mid + 1);
            node.n_keys = node.keys.len();
            self.n_keys = mid + 1;
            node
        } else {
            let mut node = Self::new_internal();
            node.keys = self.keys.split_off(mid + 1);
            node.children = self.children.split_off(mid + 1);
            node.n_keys = node.keys.len();
            self.keys.pop(); // Remove median
            self.n_keys = mid;
            node
        };
        
        (median_key, right)
    }
}
```

### 6.2 B-Tree Implementation

```rust
/// B-Tree for file system metadata
pub struct BTree<K, V> {
    /// Root block
    root: BlockNum,
    
    /// Tree height
    height: u32,
    
    /// Block device
    device: Arc<dyn BlockDevice>,
    
    /// Block cache
    cache: Arc<BlockCache>,
}

impl<K: Ord + Clone + Serialize, V: Clone + Serialize> BTree<K, V> {
    /// Create new empty B-Tree
    pub fn new(device: Arc<dyn BlockDevice>, cache: Arc<BlockCache>) -> FsResult<Self> {
        let root_block = Self::allocate_block(&device)?;
        
        // Create root as empty leaf
        let root_node: BTreeNode<K, V> = BTreeNode::new_leaf();
        Self::write_node(&cache, root_block, &root_node)?;
        
        Ok(Self {
            root: root_block,
            height: 1,
            device,
            cache,
        })
    }
    
    /// Search for key
    pub fn get(&self, key: &K) -> FsResult<Option<V>> {
        let mut current = self.root;
        
        loop {
            let node = self.read_node(current)?;
            
            match node.search(key) {
                Ok(i) => {
                    if node.is_leaf {
                        return Ok(Some(node.values[i].clone()));
                    } else {
                        current = node.children[i + 1];
                    }
                }
                Err(i) => {
                    if node.is_leaf {
                        return Ok(None);
                    } else {
                        current = node.children[i];
                    }
                }
            }
        }
    }
    
    /// Insert key-value pair
    pub fn insert(&mut self, key: K, value: V) -> FsResult<()> {
        // Check if root needs split
        let root_node = self.read_node(self.root)?;
        
        if root_node.is_full() {
            // Create new root
            let new_root_block = Self::allocate_block(&self.device)?;
            let mut new_root: BTreeNode<K, V> = BTreeNode::new_internal();
            new_root.children.push(self.root);
            
            // Split old root
            self.split_child(&mut new_root, 0)?;
            
            // Write new root
            Self::write_node(&self.cache, new_root_block, &new_root)?;
            self.root = new_root_block;
            self.height += 1;
        }
        
        // Insert into non-full tree
        self.insert_non_full(self.root, key, value)
    }
    
    /// Insert into non-full node
    fn insert_non_full(&self, block: BlockNum, key: K, value: V) -> FsResult<()> {
        let mut node = self.read_node(block)?;
        
        if node.is_leaf {
            node.insert_leaf(key, value);
            Self::write_node(&self.cache, block, &node)?;
        } else {
            // Find child to descend
            let mut i = match node.search(&key) {
                Ok(i) | Err(i) => i,
            };
            
            // Check if child is full
            let child_block = node.children[i];
            let child = self.read_node(child_block)?;
            
            if child.is_full() {
                self.split_child_at(&mut node, i)?;
                Self::write_node(&self.cache, block, &node)?;
                
                // Determine which child to use after split
                if key > node.keys[i] {
                    i += 1;
                }
            }
            
            self.insert_non_full(node.children[i], key, value)?;
        }
        
        Ok(())
    }
    
    /// Delete key
    pub fn delete(&mut self, key: &K) -> FsResult<bool> {
        self.delete_recursive(self.root, key)
    }
    
    /// Range query
    pub fn range(&self, start: &K, end: &K) -> FsResult<Vec<(K, V)>> {
        let mut results = Vec::new();
        self.range_recursive(self.root, start, end, &mut results)?;
        Ok(results)
    }
    
    fn range_recursive(&self, block: BlockNum, start: &K, end: &K, 
                       results: &mut Vec<(K, V)>) -> FsResult<()> {
        let node = self.read_node(block)?;
        
        if node.is_leaf {
            for i in 0..node.n_keys {
                if &node.keys[i] >= start && &node.keys[i] <= end {
                    results.push((node.keys[i].clone(), node.values[i].clone()));
                }
            }
        } else {
            for i in 0..node.n_keys {
                if &node.keys[i] >= start {
                    self.range_recursive(node.children[i], start, end, results)?;
                }
                if &node.keys[i] >= start && &node.keys[i] <= end {
                    // Internal node doesn't store values, skip
                }
            }
            // Check last child
            if node.n_keys > 0 && &node.keys[node.n_keys - 1] <= end {
                self.range_recursive(node.children[node.n_keys], start, end, results)?;
            }
        }
        
        Ok(())
    }
    
    // Helper methods
    fn read_node(&self, block: BlockNum) -> FsResult<BTreeNode<K, V>> {
        let data = self.cache.read(block)?;
        deserialize(&data)
    }
    
    fn write_node(cache: &BlockCache, block: BlockNum, node: &BTreeNode<K, V>) -> FsResult<()> {
        let data = serialize(node)?;
        cache.write(block, &data)
    }
    
    fn allocate_block(device: &Arc<dyn BlockDevice>) -> FsResult<BlockNum> {
        // Allocate from block allocator
        Ok(BlockNum(0)) // Placeholder
    }
}
```

---

## 7. Journaling

### 7.1 Journal Structure

```rust
/// Journal for crash recovery
pub struct Journal {
    /// Journal start block
    start_block: BlockNum,
    
    /// Journal size in blocks
    size: u32,
    
    /// Current write position
    write_pos: u32,
    
    /// Active transaction
    active_transaction: Option<Transaction>,
    
    /// Block device
    device: Arc<dyn BlockDevice>,
}

/// Journal transaction
pub struct Transaction {
    /// Transaction ID
    id: u64,
    
    /// Transaction state
    state: TransactionState,
    
    /// Logged blocks
    blocks: Vec<JournalBlock>,
    
    /// Start time
    start_time: Timestamp,
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Transaction is open
    Open,
    /// Transaction is committing
    Committing,
    /// Transaction is committed
    Committed,
    /// Transaction is complete (checkpointed)
    Complete,
    /// Transaction aborted
    Aborted,
}

/// Journaled block
struct JournalBlock {
    /// Original block number
    original: BlockNum,
    /// New data
    data: Vec<u8>,
}

/// Journal record header
#[repr(C)]
struct JournalHeader {
    /// Magic number
    magic: u32,
    /// Record type
    record_type: JournalRecordType,
    /// Transaction ID
    transaction_id: u64,
    /// Sequence number
    sequence: u64,
    /// Number of blocks in this record
    block_count: u32,
    /// Checksum
    checksum: u32,
}

/// Journal record types
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum JournalRecordType {
    /// Transaction begin
    Begin = 1,
    /// Block data
    Data = 2,
    /// Transaction commit
    Commit = 3,
    /// Checkpoint
    Checkpoint = 4,
}

impl Journal {
    /// Journal magic number
    const MAGIC: u32 = 0x4A4F5552; // "JOUR"
    
    /// Create new journal
    pub fn new(device: Arc<dyn BlockDevice>, start: BlockNum, size: u32) -> Self {
        Self {
            start_block: start,
            size,
            write_pos: 0,
            active_transaction: None,
            device,
        }
    }
    
    /// Begin a transaction
    pub fn begin_transaction(&mut self) -> FsResult<&mut Transaction> {
        if self.active_transaction.is_some() {
            return Err(FsError::TransactionActive);
        }
        
        let txn = Transaction {
            id: self.generate_txn_id(),
            state: TransactionState::Open,
            blocks: Vec::new(),
            start_time: Timestamp::now(),
        };
        
        self.active_transaction = Some(txn);
        
        // Write begin record
        self.write_begin_record()?;
        
        Ok(self.active_transaction.as_mut().unwrap())
    }
    
    /// Log a block in the current transaction
    pub fn log_block(&mut self, block: BlockNum, data: &[u8]) -> FsResult<()> {
        let txn = self.active_transaction.as_mut()
            .ok_or(FsError::NoActiveTransaction)?;
        
        if txn.state != TransactionState::Open {
            return Err(FsError::TransactionNotOpen);
        }
        
        txn.blocks.push(JournalBlock {
            original: block,
            data: data.to_vec(),
        });
        
        Ok(())
    }
    
    /// Commit the current transaction
    pub fn commit(&mut self) -> FsResult<()> {
        let txn = self.active_transaction.as_mut()
            .ok_or(FsError::NoActiveTransaction)?;
        
        txn.state = TransactionState::Committing;
        
        // Write all logged blocks
        for block in &txn.blocks {
            self.write_data_record(block)?;
        }
        
        // Write commit record
        self.write_commit_record()?;
        
        txn.state = TransactionState::Committed;
        
        // Now write to actual locations
        for block in &txn.blocks {
            self.device.write_block(block.original, &block.data)?;
        }
        
        txn.state = TransactionState::Complete;
        self.active_transaction = None;
        
        Ok(())
    }
    
    /// Abort the current transaction
    pub fn abort(&mut self) -> FsResult<()> {
        let txn = self.active_transaction.as_mut()
            .ok_or(FsError::NoActiveTransaction)?;
        
        txn.state = TransactionState::Aborted;
        self.active_transaction = None;
        
        Ok(())
    }
    
    /// Recover from crash
    pub fn recover(&mut self) -> FsResult<()> {
        serial_println!("[JOURNAL] Starting recovery");
        
        // Scan journal for committed transactions
        let mut pos = 0;
        
        while pos < self.size {
            let header = self.read_header(pos)?;
            
            if header.magic != Self::MAGIC {
                break;
            }
            
            match header.record_type {
                JournalRecordType::Begin => {
                    // Start tracking transaction
                }
                JournalRecordType::Data => {
                    // Record block data
                }
                JournalRecordType::Commit => {
                    // Replay this transaction
                    self.replay_transaction(header.transaction_id)?;
                }
                JournalRecordType::Checkpoint => {
                    // Can skip everything before this
                }
            }
            
            pos += 1 + header.block_count;
        }
        
        serial_println!("[JOURNAL] Recovery complete");
        
        Ok(())
    }
    
    fn write_begin_record(&mut self) -> FsResult<()> {
        let txn = self.active_transaction.as_ref().unwrap();
        
        let header = JournalHeader {
            magic: Self::MAGIC,
            record_type: JournalRecordType::Begin,
            transaction_id: txn.id,
            sequence: 0,
            block_count: 0,
            checksum: 0,
        };
        
        self.write_record(&header, &[])?;
        
        Ok(())
    }
    
    fn write_data_record(&mut self, block: &JournalBlock) -> FsResult<()> {
        let txn = self.active_transaction.as_ref().unwrap();
        
        let header = JournalHeader {
            magic: Self::MAGIC,
            record_type: JournalRecordType::Data,
            transaction_id: txn.id,
            sequence: block.original.0,
            block_count: 1,
            checksum: self.compute_checksum(&block.data),
        };
        
        self.write_record(&header, &block.data)?;
        
        Ok(())
    }
    
    fn write_commit_record(&mut self) -> FsResult<()> {
        let txn = self.active_transaction.as_ref().unwrap();
        
        let header = JournalHeader {
            magic: Self::MAGIC,
            record_type: JournalRecordType::Commit,
            transaction_id: txn.id,
            sequence: 0,
            block_count: 0,
            checksum: 0,
        };
        
        self.write_record(&header, &[])?;
        
        // Sync to ensure durability
        self.device.sync()?;
        
        Ok(())
    }
    
    fn write_record(&mut self, header: &JournalHeader, data: &[u8]) -> FsResult<()> {
        // Write to journal area
        let block = BlockNum(self.start_block.0 + self.write_pos as u64);
        
        let mut record = Vec::new();
        // Serialize header + data
        // ...
        
        self.device.write_block(block, &record)?;
        
        self.write_pos = (self.write_pos + 1 + header.block_count) % self.size;
        
        Ok(())
    }
    
    fn compute_checksum(&self, data: &[u8]) -> u32 {
        // CRC32 or similar
        let mut crc = 0u32;
        for &byte in data {
            crc = crc.wrapping_add(byte as u32);
        }
        crc
    }
}
```

---

## 8. Snapshots

### 8.1 Snapshot Manager

```rust
/// Copy-on-write snapshot manager
pub struct SnapshotManager {
    /// Active snapshots
    snapshots: HashMap<SnapshotId, Snapshot>,
    
    /// COW reference counts
    refcounts: HashMap<BlockNum, u32>,
    
    /// Block allocator
    allocator: Arc<BlockAllocator>,
}

/// Snapshot identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapshotId(pub u64);

/// Snapshot metadata
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// Snapshot ID
    pub id: SnapshotId,
    
    /// Snapshot name
    pub name: String,
    
    /// Creation time
    pub created: Timestamp,
    
    /// Root inode at snapshot time
    pub root: InodeNum,
    
    /// Is read-only
    pub readonly: bool,
    
    /// Parent snapshot (if incremental)
    pub parent: Option<SnapshotId>,
}

impl SnapshotManager {
    /// Create new snapshot manager
    pub fn new(allocator: Arc<BlockAllocator>) -> Self {
        Self {
            snapshots: HashMap::new(),
            refcounts: HashMap::new(),
            allocator,
        }
    }
    
    /// Create a snapshot
    pub fn create_snapshot(&mut self, name: &str, root: InodeNum) -> FsResult<SnapshotId> {
        let id = SnapshotId(self.generate_id());
        
        let snapshot = Snapshot {
            id,
            name: name.to_string(),
            created: Timestamp::now(),
            root,
            readonly: true,
            parent: None,
        };
        
        // Increment refcounts for all blocks in snapshot
        self.increment_all_refs(root)?;
        
        self.snapshots.insert(id, snapshot);
        
        serial_println!("[SNAPSHOT] Created snapshot '{}' (ID: {:?})", name, id);
        
        Ok(id)
    }
    
    /// Delete a snapshot
    pub fn delete_snapshot(&mut self, id: SnapshotId) -> FsResult<()> {
        let snapshot = self.snapshots.remove(&id)
            .ok_or(FsError::NotFound)?;
        
        // Decrement refcounts
        self.decrement_all_refs(snapshot.root)?;
        
        serial_println!("[SNAPSHOT] Deleted snapshot '{}'", snapshot.name);
        
        Ok(())
    }
    
    /// Restore from snapshot
    pub fn restore_snapshot(&self, id: SnapshotId) -> FsResult<InodeNum> {
        let snapshot = self.snapshots.get(&id)
            .ok_or(FsError::NotFound)?;
        
        // Return root inode from snapshot
        Ok(snapshot.root)
    }
    
    /// Handle COW for write
    pub fn cow_write(&mut self, block: BlockNum) -> FsResult<BlockNum> {
        let refcount = self.refcounts.get(&block).copied().unwrap_or(1);
        
        if refcount > 1 {
            // Block is shared, need to copy
            let new_block = self.allocator.allocate()?;
            
            // Copy data (caller must do this)
            
            // Update refcounts
            self.decrement_ref(block)?;
            self.refcounts.insert(new_block, 1);
            
            serial_println!("[COW] Block {:?} -> {:?}", block, new_block);
            
            Ok(new_block)
        } else {
            // Block is not shared, can modify in place
            Ok(block)
        }
    }
    
    /// Increment reference count
    pub fn increment_ref(&mut self, block: BlockNum) {
        *self.refcounts.entry(block).or_insert(0) += 1;
    }
    
    /// Decrement reference count
    pub fn decrement_ref(&mut self, block: BlockNum) -> FsResult<()> {
        if let Some(count) = self.refcounts.get_mut(&block) {
            *count -= 1;
            if *count == 0 {
                self.refcounts.remove(&block);
                self.allocator.free(block)?;
            }
        }
        Ok(())
    }
    
    /// List all snapshots
    pub fn list_snapshots(&self) -> Vec<&Snapshot> {
        self.snapshots.values().collect()
    }
    
    /// Get snapshot by name
    pub fn get_by_name(&self, name: &str) -> Option<&Snapshot> {
        self.snapshots.values().find(|s| s.name == name)
    }
    
    fn generate_id(&self) -> u64 {
        // Would use proper ID generation
        self.snapshots.len() as u64 + 1
    }
    
    fn increment_all_refs(&mut self, root: InodeNum) -> FsResult<()> {
        // Walk tree and increment all block refs
        // (simplified)
        Ok(())
    }
    
    fn decrement_all_refs(&mut self, root: InodeNum) -> FsResult<()> {
        // Walk tree and decrement all block refs
        // (simplified)
        Ok(())
    }
}
```

---

## 9. Compression

### 9.1 Compression Engine

```rust
/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgo {
    /// No compression
    None,
    /// LZ4 fast compression
    Lz4,
    /// Zstandard compression
    Zstd,
}

/// Compression level
#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(pub i32);

impl CompressionLevel {
    pub const FAST: CompressionLevel = CompressionLevel(1);
    pub const DEFAULT: CompressionLevel = CompressionLevel(3);
    pub const BEST: CompressionLevel = CompressionLevel(9);
}

/// Compression engine
pub struct CompressionEngine {
    /// Default algorithm
    algorithm: CompressionAlgo,
    
    /// Compression level
    level: CompressionLevel,
    
    /// Minimum size to compress
    min_size: usize,
}

impl CompressionEngine {
    /// Create new compression engine
    pub fn new(algorithm: CompressionAlgo, level: CompressionLevel) -> Self {
        Self {
            algorithm,
            level,
            min_size: 64, // Don't compress small data
        }
    }
    
    /// Compress data
    pub fn compress(&self, data: &[u8]) -> FsResult<CompressedData> {
        if data.len() < self.min_size {
            return Ok(CompressedData::uncompressed(data));
        }
        
        let compressed = match self.algorithm {
            CompressionAlgo::None => return Ok(CompressedData::uncompressed(data)),
            CompressionAlgo::Lz4 => self.compress_lz4(data)?,
            CompressionAlgo::Zstd => self.compress_zstd(data)?,
        };
        
        // Only use compression if it actually reduces size
        if compressed.len() >= data.len() {
            return Ok(CompressedData::uncompressed(data));
        }
        
        Ok(CompressedData {
            algorithm: self.algorithm,
            original_size: data.len(),
            data: compressed,
        })
    }
    
    /// Decompress data
    pub fn decompress(&self, compressed: &CompressedData) -> FsResult<Vec<u8>> {
        match compressed.algorithm {
            CompressionAlgo::None => Ok(compressed.data.clone()),
            CompressionAlgo::Lz4 => self.decompress_lz4(compressed),
            CompressionAlgo::Zstd => self.decompress_zstd(compressed),
        }
    }
    
    fn compress_lz4(&self, data: &[u8]) -> FsResult<Vec<u8>> {
        // LZ4 compression implementation
        // (would use lz4 crate in real implementation)
        Ok(data.to_vec()) // Placeholder
    }
    
    fn decompress_lz4(&self, compressed: &CompressedData) -> FsResult<Vec<u8>> {
        // LZ4 decompression
        Ok(compressed.data.clone()) // Placeholder
    }
    
    fn compress_zstd(&self, data: &[u8]) -> FsResult<Vec<u8>> {
        // Zstandard compression implementation
        Ok(data.to_vec()) // Placeholder
    }
    
    fn decompress_zstd(&self, compressed: &CompressedData) -> FsResult<Vec<u8>> {
        // Zstandard decompression
        Ok(compressed.data.clone()) // Placeholder
    }
}

/// Compressed data container
#[derive(Debug, Clone)]
pub struct CompressedData {
    /// Algorithm used
    pub algorithm: CompressionAlgo,
    /// Original uncompressed size
    pub original_size: usize,
    /// Compressed data
    pub data: Vec<u8>,
}

impl CompressedData {
    /// Create uncompressed container
    fn uncompressed(data: &[u8]) -> Self {
        Self {
            algorithm: CompressionAlgo::None,
            original_size: data.len(),
            data: data.to_vec(),
        }
    }
    
    /// Check if actually compressed
    pub fn is_compressed(&self) -> bool {
        self.algorithm != CompressionAlgo::None && self.data.len() < self.original_size
    }
    
    /// Get compression ratio
    pub fn ratio(&self) -> f32 {
        if self.original_size == 0 {
            1.0
        } else {
            self.data.len() as f32 / self.original_size as f32
        }
    }
}
```

---

## 10. Disk Interface

### 10.1 Block Device Trait

```rust
/// Block device interface
pub trait BlockDevice: Send + Sync {
    /// Get block size
    fn block_size(&self) -> u32;
    
    /// Get total blocks
    fn total_blocks(&self) -> u64;
    
    /// Read a block
    fn read_block(&self, block: BlockNum, buf: &mut [u8]) -> FsResult<()>;
    
    /// Write a block
    fn write_block(&self, block: BlockNum, data: &[u8]) -> FsResult<()>;
    
    /// Read multiple blocks
    fn read_blocks(&self, start: BlockNum, count: u32, buf: &mut [u8]) -> FsResult<()> {
        let block_size = self.block_size() as usize;
        for i in 0..count {
            let offset = i as usize * block_size;
            let block = BlockNum(start.0 + i as u64);
            self.read_block(block, &mut buf[offset..offset + block_size])?;
        }
        Ok(())
    }
    
    /// Write multiple blocks
    fn write_blocks(&self, start: BlockNum, data: &[u8]) -> FsResult<()> {
        let block_size = self.block_size() as usize;
        let count = data.len() / block_size;
        for i in 0..count {
            let offset = i * block_size;
            let block = BlockNum(start.0 + i as u64);
            self.write_block(block, &data[offset..offset + block_size])?;
        }
        Ok(())
    }
    
    /// Sync all pending writes
    fn sync(&self) -> FsResult<()>;
    
    /// Get device info
    fn info(&self) -> DeviceInfo;
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device name
    pub name: String,
    /// Block size
    pub block_size: u32,
    /// Total blocks
    pub total_blocks: u64,
    /// Total size in bytes
    pub total_bytes: u64,
    /// Is read-only
    pub read_only: bool,
    /// Is removable
    pub removable: bool,
}

/// RAM disk for testing
pub struct RamDisk {
    /// Block size
    block_size: u32,
    /// Data storage
    data: Mutex<Vec<u8>>,
}

impl RamDisk {
    /// Create new RAM disk
    pub fn new(block_size: u32, total_blocks: u64) -> Self {
        let size = (block_size as u64 * total_blocks) as usize;
        Self {
            block_size,
            data: Mutex::new(vec![0u8; size]),
        }
    }
}

impl BlockDevice for RamDisk {
    fn block_size(&self) -> u32 {
        self.block_size
    }
    
    fn total_blocks(&self) -> u64 {
        let data = self.data.lock();
        (data.len() / self.block_size as usize) as u64
    }
    
    fn read_block(&self, block: BlockNum, buf: &mut [u8]) -> FsResult<()> {
        let data = self.data.lock();
        let offset = block.0 as usize * self.block_size as usize;
        let end = offset + self.block_size as usize;
        
        if end > data.len() {
            return Err(FsError::IoError);
        }
        
        buf[..self.block_size as usize].copy_from_slice(&data[offset..end]);
        Ok(())
    }
    
    fn write_block(&self, block: BlockNum, data: &[u8]) -> FsResult<()> {
        let mut storage = self.data.lock();
        let offset = block.0 as usize * self.block_size as usize;
        let end = offset + self.block_size as usize;
        
        if end > storage.len() {
            return Err(FsError::IoError);
        }
        
        storage[offset..end].copy_from_slice(&data[..self.block_size as usize]);
        Ok(())
    }
    
    fn sync(&self) -> FsResult<()> {
        // RAM disk is always synced
        Ok(())
    }
    
    fn info(&self) -> DeviceInfo {
        let data = self.data.lock();
        DeviceInfo {
            name: "ramdisk".to_string(),
            block_size: self.block_size,
            total_blocks: self.total_blocks(),
            total_bytes: data.len() as u64,
            read_only: false,
            removable: false,
        }
    }
}
```

---

## Summary

HelixFS provides:

1. **Core Types**: Inodes, extents, superblock
2. **VFS Layer**: Unified filesystem interface
3. **File Operations**: Open, read, write, seek, close
4. **Directory Operations**: Create, list, remove
5. **B-Tree Structure**: Efficient metadata storage
6. **Journaling**: Crash recovery
7. **Snapshots**: Copy-on-write snapshots
8. **Compression**: LZ4, Zstd support
9. **Disk Interface**: Block device abstraction

For implementation details, see [fs/src/](../../fs/src/).

---

<div align="center">

📁 *A modern filesystem for a modern kernel* 📁

</div>
