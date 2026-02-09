//! Sysfs Manager
//!
//! Managing sysfs directory entries and attributes.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::KobjectId;

/// Sysfs attribute
#[derive(Debug, Clone)]
pub struct SysfsAttribute {
    /// Attribute name
    pub name: String,
    /// File mode
    pub mode: u16,
    /// Readable
    pub readable: bool,
    /// Writable
    pub writable: bool,
    /// Binary attribute
    pub is_binary: bool,
    /// Size (for binary)
    pub size: usize,
}

impl SysfsAttribute {
    /// Create new attribute
    pub fn new(name: String, mode: u16) -> Self {
        Self {
            name,
            mode,
            readable: mode & 0o444 != 0,
            writable: mode & 0o222 != 0,
            is_binary: false,
            size: 0,
        }
    }
}

/// Sysfs directory entry
#[derive(Debug, Clone)]
pub struct SysfsDirEntry {
    /// Entry name
    pub name: String,
    /// Full path
    pub path: String,
    /// Is directory
    pub is_dir: bool,
    /// Is symlink
    pub is_link: bool,
    /// Link target (if symlink)
    pub link_target: Option<String>,
    /// Kobject (if directory)
    pub kobject: Option<KobjectId>,
    /// Attributes (if directory)
    pub attributes: Vec<SysfsAttribute>,
}

impl SysfsDirEntry {
    /// Create new directory entry
    #[inline]
    pub fn new_dir(name: String, path: String, kobject: KobjectId) -> Self {
        Self {
            name,
            path,
            is_dir: true,
            is_link: false,
            link_target: None,
            kobject: Some(kobject),
            attributes: Vec::new(),
        }
    }

    /// Create new symlink
    #[inline]
    pub fn new_link(name: String, path: String, target: String) -> Self {
        Self {
            name,
            path,
            is_dir: false,
            is_link: true,
            link_target: Some(target),
            kobject: None,
            attributes: Vec::new(),
        }
    }
}

/// Sysfs manager
pub struct SysfsManager {
    /// Root entries
    entries: BTreeMap<String, SysfsDirEntry>,
    /// Path to kobject mapping
    path_to_kobject: BTreeMap<String, KobjectId>,
    /// Kobject to path mapping
    kobject_to_path: BTreeMap<KobjectId, String>,
    /// Total directories
    total_dirs: AtomicU64,
    /// Total symlinks
    total_links: AtomicU64,
    /// Total attributes
    total_attrs: AtomicU64,
}

impl SysfsManager {
    /// Create new sysfs manager
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            path_to_kobject: BTreeMap::new(),
            kobject_to_path: BTreeMap::new(),
            total_dirs: AtomicU64::new(0),
            total_links: AtomicU64::new(0),
            total_attrs: AtomicU64::new(0),
        }
    }

    /// Add directory
    #[inline]
    pub fn add_directory(&mut self, path: String, kobject: KobjectId) {
        let name = path.rsplit('/').next().unwrap_or("").to_string();
        let entry = SysfsDirEntry::new_dir(name, path.clone(), kobject);

        self.entries.insert(path.clone(), entry);
        self.path_to_kobject.insert(path.clone(), kobject);
        self.kobject_to_path.insert(kobject, path);
        self.total_dirs.fetch_add(1, Ordering::Relaxed);
    }

    /// Add symlink
    #[inline]
    pub fn add_symlink(&mut self, path: String, target: String) {
        let name = path.rsplit('/').next().unwrap_or("").to_string();
        let entry = SysfsDirEntry::new_link(name, path.clone(), target);

        self.entries.insert(path, entry);
        self.total_links.fetch_add(1, Ordering::Relaxed);
    }

    /// Add attribute to directory
    #[inline]
    pub fn add_attribute(&mut self, dir_path: &str, attr: SysfsAttribute) {
        if let Some(entry) = self.entries.get_mut(dir_path) {
            entry.attributes.push(attr);
            self.total_attrs.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Remove entry
    #[inline]
    pub fn remove_entry(&mut self, path: &str) {
        if let Some(entry) = self.entries.remove(path) {
            if let Some(kobject) = entry.kobject {
                self.path_to_kobject.remove(path);
                self.kobject_to_path.remove(&kobject);
            }
        }
    }

    /// Get entry by path
    #[inline(always)]
    pub fn get_entry(&self, path: &str) -> Option<&SysfsDirEntry> {
        self.entries.get(path)
    }

    /// Get path for kobject
    #[inline(always)]
    pub fn get_path(&self, kobject: KobjectId) -> Option<&str> {
        self.kobject_to_path.get(&kobject).map(|s| s.as_str())
    }

    /// Get kobject for path
    #[inline(always)]
    pub fn get_kobject(&self, path: &str) -> Option<KobjectId> {
        self.path_to_kobject.get(path).copied()
    }

    /// List directory contents
    pub fn list_directory(&self, path: &str) -> Vec<&SysfsDirEntry> {
        let prefix = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };

        self.entries
            .values()
            .filter(|e| {
                e.path.starts_with(&prefix) && e.path[prefix.len()..].chars().all(|c| c != '/')
            })
            .collect()
    }

    /// Get total directory count
    #[inline(always)]
    pub fn dir_count(&self) -> u64 {
        self.total_dirs.load(Ordering::Relaxed)
    }

    /// Get total symlink count
    #[inline(always)]
    pub fn link_count(&self) -> u64 {
        self.total_links.load(Ordering::Relaxed)
    }

    /// Get total attribute count
    #[inline(always)]
    pub fn attr_count(&self) -> u64 {
        self.total_attrs.load(Ordering::Relaxed)
    }
}

impl Default for SysfsManager {
    fn default() -> Self {
        Self::new()
    }
}
