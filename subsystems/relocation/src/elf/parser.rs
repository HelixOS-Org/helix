//! # ELF Parser
//!
//! High-level ELF parsing utilities.

use super::*;
use crate::{PhysAddr, RelocError, RelocResult};

/// ELF parser for kernel images
pub struct ElfParser<'a> {
    /// Raw data
    data: &'a [u8],
    /// Parsed header
    header: &'a Elf64Header,
}

impl<'a> ElfParser<'a> {
    /// Create parser from raw bytes
    pub fn new(data: &'a [u8]) -> RelocResult<Self> {
        if data.len() < core::mem::size_of::<Elf64Header>() {
            return Err(RelocError::InvalidElfMagic);
        }

        let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
        header.validate().map_err(|_| RelocError::InvalidElfMagic)?;

        Ok(Self { data, header })
    }

    /// Create parser from physical address
    ///
    /// # Safety
    /// Address must point to valid ELF data
    pub unsafe fn from_addr(addr: PhysAddr, size: usize) -> RelocResult<Self> {
        let data = unsafe { core::slice::from_raw_parts(addr.as_u64() as *const u8, size) };
        Self::new(data)
    }

    /// Get ELF header
    pub fn header(&self) -> &Elf64Header {
        self.header
    }

    /// Get program headers iterator
    pub fn program_headers(&self) -> impl Iterator<Item = &Elf64Phdr> {
        let phdr_offset = self.header.e_phoff as usize;
        let phdr_size = self.header.e_phentsize as usize;
        let phdr_count = self.header.e_phnum as usize;

        (0..phdr_count).map(move |i| {
            let offset = phdr_offset + i * phdr_size;
            unsafe { &*(self.data.as_ptr().add(offset) as *const Elf64Phdr) }
        })
    }

    /// Get section headers iterator
    pub fn section_headers(&self) -> impl Iterator<Item = &Elf64Shdr> {
        let shdr_offset = self.header.e_shoff as usize;
        let shdr_size = self.header.e_shentsize as usize;
        let shdr_count = self.header.e_shnum as usize;

        (0..shdr_count).map(move |i| {
            let offset = shdr_offset + i * shdr_size;
            unsafe { &*(self.data.as_ptr().add(offset) as *const Elf64Shdr) }
        })
    }

    /// Find dynamic segment
    pub fn find_dynamic(&self) -> Option<&Elf64Phdr> {
        self.program_headers().find(|p| p.is_dynamic())
    }

    /// Get dynamic entries iterator
    pub fn dynamic_entries(&self) -> impl Iterator<Item = &Elf64Dyn> {
        let dynamic = match self.find_dynamic() {
            Some(d) => d,
            None => return DynIter::empty(),
        };

        let offset = dynamic.p_offset as usize;
        let size = dynamic.p_filesz as usize;
        let count = size / core::mem::size_of::<Elf64Dyn>();

        DynIter {
            data: self.data,
            offset,
            count,
            index: 0,
        }
    }

    /// Get RELA entries
    pub fn rela_entries(&self) -> RelocResult<RelaIter<'a>> {
        let mut rela_addr = None;
        let mut rela_size = 0usize;

        for dyn_entry in self.dynamic_entries() {
            match dyn_entry.d_tag {
                DT_RELA => rela_addr = Some(dyn_entry.d_val),
                DT_RELASZ => rela_size = dyn_entry.d_val as usize,
                _ => {},
            }
        }

        let vaddr = rela_addr.ok_or(RelocError::SectionNotFound(".rela.dyn"))?;

        // Convert virtual address to file offset
        let offset = self.vaddr_to_offset(vaddr)?;
        let count = rela_size / core::mem::size_of::<Elf64Rela>();

        Ok(RelaIter {
            data: self.data,
            offset,
            count,
            index: 0,
        })
    }

    /// Convert virtual address to file offset
    pub fn vaddr_to_offset(&self, vaddr: u64) -> RelocResult<usize> {
        for phdr in self.program_headers() {
            if phdr.is_loadable() && vaddr >= phdr.p_vaddr && vaddr < phdr.p_vaddr + phdr.p_filesz {
                return Ok((vaddr - phdr.p_vaddr + phdr.p_offset) as usize);
            }
        }
        Err(RelocError::OutOfBounds(vaddr))
    }

    /// Get base virtual address (lowest load address)
    pub fn base_vaddr(&self) -> u64 {
        self.program_headers()
            .filter(|p| p.is_loadable())
            .map(|p| p.p_vaddr)
            .min()
            .unwrap_or(0)
    }

    /// Parse into ElfInfo
    pub fn into_elf_info(self) -> RelocResult<ElfInfo> {
        unsafe { ElfInfo::from_header(self.data.as_ptr()) }.map_err(|_| RelocError::InvalidElfMagic)
    }
}

/// Dynamic entry iterator
struct DynIter<'a> {
    data: &'a [u8],
    offset: usize,
    count: usize,
    index: usize,
}

impl DynIter<'_> {
    fn empty() -> Self {
        Self {
            data: &[],
            offset: 0,
            count: 0,
            index: 0,
        }
    }
}

impl<'a> Iterator for DynIter<'a> {
    type Item = &'a Elf64Dyn;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }

        let entry_size = core::mem::size_of::<Elf64Dyn>();
        let offset = self.offset + self.index * entry_size;

        if offset + entry_size > self.data.len() {
            return None;
        }

        let entry = unsafe { &*(self.data.as_ptr().add(offset) as *const Elf64Dyn) };

        if entry.d_tag == DT_NULL {
            return None;
        }

        self.index += 1;
        Some(entry)
    }
}

/// RELA entry iterator
pub struct RelaIter<'a> {
    data: &'a [u8],
    offset: usize,
    count: usize,
    index: usize,
}

impl<'a> Iterator for RelaIter<'a> {
    type Item = &'a Elf64Rela;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }

        let entry_size = core::mem::size_of::<Elf64Rela>();
        let offset = self.offset + self.index * entry_size;

        if offset + entry_size > self.data.len() {
            return None;
        }

        let entry = unsafe { &*(self.data.as_ptr().add(offset) as *const Elf64Rela) };
        self.index += 1;
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for RelaIter<'_> {}
