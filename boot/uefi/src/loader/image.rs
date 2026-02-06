//! Image Abstraction
//!
//! Unified interface for different executable formats.

use crate::error::{Error, Result};
use crate::loader::elf::ElfLoader;
use crate::loader::pe::PeLoader;
use crate::loader::{
    ImageFlags, ImageFormat, ImageSection, LoadedImage, MachineType, SectionFlags,
};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// IMAGE READER
// =============================================================================

/// Unified image reader
pub struct ImageReader {
    /// Format
    format: ImageFormat,
    /// ELF loader (if ELF)
    elf_loader: Option<ElfLoader>,
    /// PE loader (if PE)
    pe_loader: Option<PeLoader>,
    /// Loaded image
    image: Option<LoadedImage>,
}

impl ImageReader {
    /// Create new image reader
    pub fn new() -> Self {
        Self {
            format: ImageFormat::Unknown,
            elf_loader: None,
            pe_loader: None,
            image: None,
        }
    }

    /// Load image from buffer
    pub fn load(&mut self, data: &[u8]) -> Result<&LoadedImage> {
        // Detect format
        self.format = ImageFormat::detect(data)?;

        // Load based on format
        match self.format {
            ImageFormat::Elf64 | ImageFormat::Elf32 => {
                let mut loader = ElfLoader::new();
                let image = loader.load(data)?;
                self.image = Some(image);
                self.elf_loader = Some(loader);
            },
            ImageFormat::Pe32Plus | ImageFormat::Pe32 => {
                let mut loader = PeLoader::new();
                let image = loader.load(data)?;
                self.image = Some(image);
                self.pe_loader = Some(loader);
            },
            _ => return Err(Error::UnsupportedFormat),
        }

        Ok(self.image.as_ref().unwrap())
    }

    /// Get loaded image
    pub fn image(&self) -> Option<&LoadedImage> {
        self.image.as_ref()
    }

    /// Get format
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Get ELF loader
    pub fn elf(&self) -> Option<&ElfLoader> {
        self.elf_loader.as_ref()
    }

    /// Get PE loader
    pub fn pe(&self) -> Option<&PeLoader> {
        self.pe_loader.as_ref()
    }

    /// Get entry point
    pub fn entry_point(&self) -> Option<VirtualAddress> {
        self.image.as_ref().map(|i| i.entry_point)
    }

    /// Get machine type
    pub fn machine(&self) -> Option<MachineType> {
        self.image.as_ref().map(|i| i.machine)
    }

    /// Get sections
    pub fn sections(&self) -> &[ImageSection] {
        self.image
            .as_ref()
            .map(|i| i.sections.as_slice())
            .unwrap_or(&[])
    }
}

impl Default for ImageReader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// IMAGE INFO
// =============================================================================

/// Extracted image information
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Format
    pub format: ImageFormat,
    /// Entry point
    pub entry_point: VirtualAddress,
    /// Load address
    pub load_address: VirtualAddress,
    /// Image size
    pub image_size: u64,
    /// Machine type
    pub machine: MachineType,
    /// Section count
    pub section_count: usize,
    /// Is PIE
    pub is_pie: bool,
    /// Is stripped
    pub is_stripped: bool,
    /// Has debug info
    pub has_debug: bool,
}

impl ImageInfo {
    /// Extract from loaded image
    pub fn from_image(image: &LoadedImage) -> Self {
        Self {
            format: image.format,
            entry_point: image.entry_point,
            load_address: image.load_address,
            image_size: image.image_size,
            machine: image.machine,
            section_count: image.sections.len(),
            is_pie: image.flags.pie,
            is_stripped: image.flags.stripped,
            has_debug: image.flags.has_symbols,
        }
    }
}

// =============================================================================
// SECTION ITERATOR
// =============================================================================

/// Section iterator with filtering
pub struct SectionIterator<'a> {
    sections: &'a [ImageSection],
    index: usize,
    filter: SectionFilter,
}

impl<'a> SectionIterator<'a> {
    /// Create new iterator
    pub fn new(sections: &'a [ImageSection]) -> Self {
        Self {
            sections,
            index: 0,
            filter: SectionFilter::All,
        }
    }

    /// Filter by type
    pub fn filter(mut self, filter: SectionFilter) -> Self {
        self.filter = filter;
        self
    }
}

impl<'a> Iterator for SectionIterator<'a> {
    type Item = &'a ImageSection;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.sections.len() {
            let section = &self.sections[self.index];
            self.index += 1;

            let matches = match self.filter {
                SectionFilter::All => true,
                SectionFilter::Code => section.flags.executable,
                SectionFilter::Data => section.flags.writable && !section.flags.bss,
                SectionFilter::ReadOnly => !section.flags.writable && !section.flags.executable,
                SectionFilter::Bss => section.flags.bss,
                SectionFilter::Allocated => section.flags.allocated,
            };

            if matches {
                return Some(section);
            }
        }
        None
    }
}

/// Section filter type
#[derive(Debug, Clone, Copy)]
pub enum SectionFilter {
    /// All sections
    All,
    /// Code sections
    Code,
    /// Data sections (writable, not BSS)
    Data,
    /// Read-only sections
    ReadOnly,
    /// BSS sections
    Bss,
    /// Allocated sections
    Allocated,
}

// =============================================================================
// IMAGE LAYOUT
// =============================================================================

/// Image memory layout
#[derive(Debug, Clone)]
pub struct ImageLayout {
    /// Base virtual address
    pub base: VirtualAddress,
    /// Total size
    pub size: u64,
    /// Code region
    pub code: Option<LayoutRegion>,
    /// Data region
    pub data: Option<LayoutRegion>,
    /// Read-only data region
    pub rodata: Option<LayoutRegion>,
    /// BSS region
    pub bss: Option<LayoutRegion>,
}

impl ImageLayout {
    /// Compute from image
    pub fn from_image(image: &LoadedImage) -> Self {
        let mut layout = Self {
            base: image.load_address,
            size: image.image_size,
            code: None,
            data: None,
            rodata: None,
            bss: None,
        };

        // Find code section bounds
        let code_sections: Vec<_> = image
            .sections
            .iter()
            .filter(|s| s.flags.executable)
            .collect();

        if !code_sections.is_empty() {
            let min = code_sections
                .iter()
                .map(|s| s.virtual_address)
                .min()
                .unwrap();
            let max = code_sections
                .iter()
                .map(|s| s.virtual_address + s.size)
                .max()
                .unwrap();
            layout.code = Some(LayoutRegion {
                base: min,
                size: max - min,
            });
        }

        // Find data section bounds
        let data_sections: Vec<_> = image
            .sections
            .iter()
            .filter(|s| s.flags.writable && !s.flags.bss)
            .collect();

        if !data_sections.is_empty() {
            let min = data_sections
                .iter()
                .map(|s| s.virtual_address)
                .min()
                .unwrap();
            let max = data_sections
                .iter()
                .map(|s| s.virtual_address + s.size)
                .max()
                .unwrap();
            layout.data = Some(LayoutRegion {
                base: min,
                size: max - min,
            });
        }

        // Find rodata section bounds
        let rodata_sections: Vec<_> = image
            .sections
            .iter()
            .filter(|s| !s.flags.writable && !s.flags.executable && !s.flags.bss)
            .collect();

        if !rodata_sections.is_empty() {
            let min = rodata_sections
                .iter()
                .map(|s| s.virtual_address)
                .min()
                .unwrap();
            let max = rodata_sections
                .iter()
                .map(|s| s.virtual_address + s.size)
                .max()
                .unwrap();
            layout.rodata = Some(LayoutRegion {
                base: min,
                size: max - min,
            });
        }

        // Find BSS section bounds
        let bss_sections: Vec<_> = image.sections.iter().filter(|s| s.flags.bss).collect();

        if !bss_sections.is_empty() {
            let min = bss_sections
                .iter()
                .map(|s| s.virtual_address)
                .min()
                .unwrap();
            let max = bss_sections
                .iter()
                .map(|s| s.virtual_address + s.size)
                .max()
                .unwrap();
            layout.bss = Some(LayoutRegion {
                base: min,
                size: max - min,
            });
        }

        layout
    }

    /// Get end address
    pub fn end(&self) -> VirtualAddress {
        self.base + self.size
    }
}

/// Layout region
#[derive(Debug, Clone, Copy)]
pub struct LayoutRegion {
    /// Base address
    pub base: VirtualAddress,
    /// Size
    pub size: u64,
}

impl LayoutRegion {
    /// Get end address
    pub fn end(&self) -> VirtualAddress {
        self.base + self.size
    }
}

// =============================================================================
// IMAGE BUILDER
// =============================================================================

/// Build custom loaded image
pub struct ImageBuilder {
    format: ImageFormat,
    entry_point: VirtualAddress,
    load_address: VirtualAddress,
    machine: MachineType,
    sections: Vec<ImageSection>,
    name: String,
    flags: ImageFlags,
}

impl ImageBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            format: ImageFormat::Binary,
            entry_point: VirtualAddress(0),
            load_address: VirtualAddress(0),
            machine: MachineType::Unknown,
            sections: Vec::new(),
            name: String::new(),
            flags: ImageFlags::default(),
        }
    }

    /// Set format
    pub fn format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    /// Set entry point
    pub fn entry_point(mut self, entry: VirtualAddress) -> Self {
        self.entry_point = entry;
        self
    }

    /// Set load address
    pub fn load_address(mut self, addr: VirtualAddress) -> Self {
        self.load_address = addr;
        self
    }

    /// Set machine type
    pub fn machine(mut self, machine: MachineType) -> Self {
        self.machine = machine;
        self
    }

    /// Set name
    pub fn name(mut self, name: &str) -> Self {
        self.name = String::from(name);
        self
    }

    /// Add section
    pub fn add_section(mut self, section: ImageSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Add code section
    pub fn add_code_section(
        mut self,
        name: &str,
        virtual_address: VirtualAddress,
        size: u64,
    ) -> Self {
        self.sections.push(ImageSection {
            name: String::from(name),
            virtual_address,
            size,
            file_offset: 0,
            file_size: size,
            alignment: 4096,
            flags: SectionFlags {
                readable: true,
                executable: true,
                allocated: true,
                code: true,
                ..Default::default()
            },
        });
        self
    }

    /// Add data section
    pub fn add_data_section(
        mut self,
        name: &str,
        virtual_address: VirtualAddress,
        size: u64,
    ) -> Self {
        self.sections.push(ImageSection {
            name: String::from(name),
            virtual_address,
            size,
            file_offset: 0,
            file_size: size,
            alignment: 4096,
            flags: SectionFlags {
                readable: true,
                writable: true,
                allocated: true,
                data: true,
                ..Default::default()
            },
        });
        self
    }

    /// Add rodata section
    pub fn add_rodata_section(
        mut self,
        name: &str,
        virtual_address: VirtualAddress,
        size: u64,
    ) -> Self {
        self.sections.push(ImageSection {
            name: String::from(name),
            virtual_address,
            size,
            file_offset: 0,
            file_size: size,
            alignment: 4096,
            flags: SectionFlags {
                readable: true,
                allocated: true,
                data: true,
                ..Default::default()
            },
        });
        self
    }

    /// Add BSS section
    pub fn add_bss_section(
        mut self,
        name: &str,
        virtual_address: VirtualAddress,
        size: u64,
    ) -> Self {
        self.sections.push(ImageSection {
            name: String::from(name),
            virtual_address,
            size,
            file_offset: 0,
            file_size: 0,
            alignment: 4096,
            flags: SectionFlags {
                readable: true,
                writable: true,
                allocated: true,
                bss: true,
                ..Default::default()
            },
        });
        self
    }

    /// Set flags
    pub fn flags(mut self, flags: ImageFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Build image
    pub fn build(self) -> LoadedImage {
        let image_size = if self.sections.is_empty() {
            0
        } else {
            let max_end = self
                .sections
                .iter()
                .map(|s| s.virtual_address + s.size)
                .max()
                .unwrap_or(VirtualAddress(0));
            max_end.0.saturating_sub(self.load_address.0)
        };

        let bss_section = self.sections.iter().find(|s| s.flags.bss);
        let (bss_start, bss_size) = bss_section
            .map_or((None, 0), |s| (Some(s.virtual_address), s.size));

        LoadedImage {
            format: self.format,
            entry_point: self.entry_point,
            load_address: self.load_address,
            image_size,
            sections: self.sections,
            stack_top: None,
            bss_start,
            bss_size,
            name: self.name,
            machine: self.machine,
            flags: self.flags,
        }
    }
}

impl Default for ImageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_image_builder() {
        let image = ImageBuilder::new()
            .format(ImageFormat::Elf64)
            .entry_point(0x1000)
            .load_address(0x1000)
            .machine(MachineType::X86_64)
            .name("test")
            .add_code_section(".text", 0x1000, 0x1000)
            .add_data_section(".data", 0x2000, 0x1000)
            .build();

        assert_eq!(image.format, ImageFormat::Elf64);
        assert_eq!(image.entry_point, 0x1000);
        assert_eq!(image.sections.len(), 2);
    }

    #[test]
    fn test_section_filter() {
        let sections = vec![
            ImageSection {
                name: String::from(".text"),
                virtual_address: 0x1000,
                size: 0x1000,
                file_offset: 0,
                file_size: 0x1000,
                alignment: 4096,
                flags: SectionFlags {
                    executable: true,
                    readable: true,
                    allocated: true,
                    ..Default::default()
                },
            },
            ImageSection {
                name: String::from(".data"),
                virtual_address: 0x2000,
                size: 0x1000,
                file_offset: 0,
                file_size: 0x1000,
                alignment: 4096,
                flags: SectionFlags {
                    writable: true,
                    readable: true,
                    allocated: true,
                    ..Default::default()
                },
            },
        ];

        let code: Vec<_> = SectionIterator::new(&sections)
            .filter(SectionFilter::Code)
            .collect();

        assert_eq!(code.len(), 1);
        assert_eq!(code[0].name, ".text");
    }
}
