//! # Asset Exporters
//!
//! Export assets to optimized GPU formats.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{
    AssetError, AssetErrorKind, AssetResult, AssetType, ExportedAsset, Exporter, ImportedAsset,
    ImportedData, TextureFormat,
};

/// Texture exporter
pub struct TextureExporter {
    version: u32,
}

impl TextureExporter {
    pub fn new() -> Self {
        Self { version: 1 }
    }
}

impl Default for TextureExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter for TextureExporter {
    fn export(&self, asset: &ImportedAsset) -> AssetResult<ExportedAsset> {
        let texture = match &asset.data {
            ImportedData::Texture(t) => t,
            _ => {
                return Err(AssetError::new(
                    AssetErrorKind::InvalidFormat,
                    "Not a texture",
                ))
            },
        };

        let mut data = Vec::new();

        // Magic number "LTEX"
        data.extend_from_slice(b"LTEX");

        // Version
        data.extend_from_slice(&self.version.to_le_bytes());

        // Dimensions
        data.extend_from_slice(&texture.width.to_le_bytes());
        data.extend_from_slice(&texture.height.to_le_bytes());
        data.extend_from_slice(&texture.depth.to_le_bytes());

        // Format
        data.extend_from_slice(&(texture.format as u32).to_le_bytes());

        // Mip count
        data.extend_from_slice(&(texture.mip_levels.len() as u32).to_le_bytes());

        // Array layers
        data.extend_from_slice(&texture.array_layers.to_le_bytes());

        // Flags
        let flags = if texture.is_cubemap { 1u32 } else { 0u32 };
        data.extend_from_slice(&flags.to_le_bytes());

        // Mip level data
        for mip in &texture.mip_levels {
            data.extend_from_slice(&(mip.len() as u32).to_le_bytes());
            data.extend_from_slice(mip);
        }

        Ok(ExportedAsset {
            data,
            format_version: self.version,
        })
    }
}

/// Mesh exporter
pub struct MeshExporter {
    version: u32,
}

impl MeshExporter {
    pub fn new() -> Self {
        Self { version: 1 }
    }
}

impl Default for MeshExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter for MeshExporter {
    fn export(&self, asset: &ImportedAsset) -> AssetResult<ExportedAsset> {
        let mesh = match &asset.data {
            ImportedData::Mesh(m) => m,
            _ => return Err(AssetError::new(AssetErrorKind::InvalidFormat, "Not a mesh")),
        };

        let mut data = Vec::new();

        // Magic number "LMSH"
        data.extend_from_slice(b"LMSH");

        // Version
        data.extend_from_slice(&self.version.to_le_bytes());

        // Vertex count
        data.extend_from_slice(&(mesh.vertices.len() as u32).to_le_bytes());

        // Index count
        data.extend_from_slice(&(mesh.indices.len() as u32).to_le_bytes());

        // Submesh count
        data.extend_from_slice(&(mesh.submeshes.len() as u32).to_le_bytes());

        // Bounds
        for v in &mesh.bounds.min {
            data.extend_from_slice(&v.to_le_bytes());
        }
        for v in &mesh.bounds.max {
            data.extend_from_slice(&v.to_le_bytes());
        }
        for v in &mesh.bounds.center {
            data.extend_from_slice(&v.to_le_bytes());
        }
        data.extend_from_slice(&mesh.bounds.radius.to_le_bytes());

        // Vertex data
        for vertex in &mesh.vertices {
            for v in &vertex.position {
                data.extend_from_slice(&v.to_le_bytes());
            }
            for v in &vertex.normal {
                data.extend_from_slice(&v.to_le_bytes());
            }
            for v in &vertex.tangent {
                data.extend_from_slice(&v.to_le_bytes());
            }
            for v in &vertex.uv0 {
                data.extend_from_slice(&v.to_le_bytes());
            }
        }

        // Index data
        for idx in &mesh.indices {
            data.extend_from_slice(&idx.to_le_bytes());
        }

        // Submesh data
        for submesh in &mesh.submeshes {
            data.extend_from_slice(&submesh.index_offset.to_le_bytes());
            data.extend_from_slice(&submesh.index_count.to_le_bytes());
            data.extend_from_slice(&submesh.material_index.to_le_bytes());
        }

        Ok(ExportedAsset {
            data,
            format_version: self.version,
        })
    }
}

/// Material exporter
pub struct MaterialExporter {
    version: u32,
}

impl MaterialExporter {
    pub fn new() -> Self {
        Self { version: 1 }
    }
}

impl Default for MaterialExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter for MaterialExporter {
    fn export(&self, asset: &ImportedAsset) -> AssetResult<ExportedAsset> {
        let material = match &asset.data {
            ImportedData::Material(m) => m,
            _ => {
                return Err(AssetError::new(
                    AssetErrorKind::InvalidFormat,
                    "Not a material",
                ))
            },
        };

        let mut data = Vec::new();

        // Magic number "LMAT"
        data.extend_from_slice(b"LMAT");

        // Version
        data.extend_from_slice(&self.version.to_le_bytes());

        // Name
        write_string(&mut data, &material.name);

        // Shader name
        write_string(&mut data, &material.shader);

        // Property count
        data.extend_from_slice(&(material.properties.len() as u32).to_le_bytes());

        // Properties
        for (name, value) in &material.properties {
            write_string(&mut data, name);
            write_property(&mut data, value);
        }

        // Texture count
        data.extend_from_slice(&(material.textures.len() as u32).to_le_bytes());

        // Textures
        for (name, path) in &material.textures {
            write_string(&mut data, name);
            write_string(&mut data, path);
        }

        Ok(ExportedAsset {
            data,
            format_version: self.version,
        })
    }
}

/// Shader exporter
pub struct ShaderExporter {
    version: u32,
}

impl ShaderExporter {
    pub fn new() -> Self {
        Self { version: 1 }
    }
}

impl Default for ShaderExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter for ShaderExporter {
    fn export(&self, asset: &ImportedAsset) -> AssetResult<ExportedAsset> {
        let shader = match &asset.data {
            ImportedData::Shader(s) => s,
            _ => {
                return Err(AssetError::new(
                    AssetErrorKind::InvalidFormat,
                    "Not a shader",
                ))
            },
        };

        let mut data = Vec::new();

        // Magic number "LSHD"
        data.extend_from_slice(b"LSHD");

        // Version
        data.extend_from_slice(&self.version.to_le_bytes());

        // Name
        write_string(&mut data, &shader.name);

        // Stage count
        data.extend_from_slice(&(shader.stages.len() as u32).to_le_bytes());

        // Stages
        for stage in &shader.stages {
            data.push(stage.stage as u8);
            write_string(&mut data, &stage.entry_point);
            write_string(&mut data, &stage.source);
        }

        Ok(ExportedAsset {
            data,
            format_version: self.version,
        })
    }
}

fn write_string(data: &mut Vec<u8>, s: &str) {
    data.extend_from_slice(&(s.len() as u32).to_le_bytes());
    data.extend_from_slice(s.as_bytes());
}

fn write_property(data: &mut Vec<u8>, value: &crate::MaterialProperty) {
    use crate::MaterialProperty;

    match value {
        MaterialProperty::Float(v) => {
            data.push(0);
            data.extend_from_slice(&v.to_le_bytes());
        },
        MaterialProperty::Float2(v) => {
            data.push(1);
            for f in v {
                data.extend_from_slice(&f.to_le_bytes());
            }
        },
        MaterialProperty::Float3(v) => {
            data.push(2);
            for f in v {
                data.extend_from_slice(&f.to_le_bytes());
            }
        },
        MaterialProperty::Float4(v) => {
            data.push(3);
            for f in v {
                data.extend_from_slice(&f.to_le_bytes());
            }
        },
        MaterialProperty::Int(v) => {
            data.push(4);
            data.extend_from_slice(&v.to_le_bytes());
        },
        MaterialProperty::Bool(v) => {
            data.push(5);
            data.push(if *v { 1 } else { 0 });
        },
        MaterialProperty::Texture(path) => {
            data.push(6);
            write_string(data, path);
        },
    }
}
