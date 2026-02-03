//! # Asset Importers
//!
//! Importers for various asset formats.

use alloc::{boxed::Box, string::String, vec::Vec};
use crate::{
    AssetResult, AssetError, AssetErrorKind, AssetId, AssetType, AssetMetadata,
    ImportSettings, ImportedAsset, ImportedData, ImportedTexture, ImportedMesh,
    ImportedMaterial, ImportedVertex, Submesh, MeshBounds, TextureFormat, Importer,
};

/// PNG/JPEG image importer
pub struct ImageImporter;

impl Importer for ImageImporter {
    fn import(&self, path: &str, settings: &ImportSettings) -> AssetResult<ImportedAsset> {
        // Would decode PNG/JPEG/etc
        let texture = ImportedTexture {
            width: 256,
            height: 256,
            depth: 1,
            format: if settings.texture_settings.srgb {
                TextureFormat::Rgba8Srgb
            } else {
                TextureFormat::Rgba8
            },
            mip_levels: vec![vec![255u8; 256 * 256 * 4]],
            array_layers: 1,
            is_cubemap: false,
        };
        
        Ok(ImportedAsset {
            asset_type: AssetType::Texture,
            metadata: AssetMetadata {
                id: AssetId::from_path(path),
                name: extract_name(path),
                asset_type: AssetType::Texture,
                source_path: Some(path.into()),
                import_time: 0,
                size_bytes: texture.mip_levels[0].len() as u64,
                gpu_size_bytes: texture.mip_levels[0].len() as u64,
                dependencies: Vec::new(),
                tags: Vec::new(),
                custom_data: alloc::collections::BTreeMap::new(),
            },
            data: ImportedData::Texture(texture),
        })
    }
    
    fn supported_extensions(&self) -> &[&str] {
        &["png", "jpg", "jpeg", "bmp", "tga", "hdr", "exr"]
    }
}

/// glTF model importer
pub struct GltfImporter;

impl Importer for GltfImporter {
    fn import(&self, path: &str, settings: &ImportSettings) -> AssetResult<ImportedAsset> {
        // Would parse glTF/GLB
        let mesh = ImportedMesh {
            vertices: vec![
                ImportedVertex {
                    position: [-1.0, -1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv0: [0.0, 0.0],
                    uv1: None,
                    color: None,
                    bone_indices: None,
                    bone_weights: None,
                },
                ImportedVertex {
                    position: [1.0, -1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv0: [1.0, 0.0],
                    uv1: None,
                    color: None,
                    bone_indices: None,
                    bone_weights: None,
                },
                ImportedVertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    tangent: [1.0, 0.0, 0.0, 1.0],
                    uv0: [0.5, 1.0],
                    uv1: None,
                    color: None,
                    bone_indices: None,
                    bone_weights: None,
                },
            ],
            indices: vec![0, 1, 2],
            submeshes: vec![Submesh {
                index_offset: 0,
                index_count: 3,
                material_index: 0,
            }],
            bounds: MeshBounds {
                min: [-1.0, -1.0, 0.0],
                max: [1.0, 1.0, 0.0],
                center: [0.0, 0.0, 0.0],
                radius: 1.414,
            },
            skeleton: None,
        };
        
        Ok(ImportedAsset {
            asset_type: AssetType::Mesh,
            metadata: AssetMetadata {
                id: AssetId::from_path(path),
                name: extract_name(path),
                asset_type: AssetType::Mesh,
                source_path: Some(path.into()),
                import_time: 0,
                size_bytes: 0,
                gpu_size_bytes: 0,
                dependencies: Vec::new(),
                tags: Vec::new(),
                custom_data: alloc::collections::BTreeMap::new(),
            },
            data: ImportedData::Mesh(mesh),
        })
    }
    
    fn supported_extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

/// FBX model importer
pub struct FbxImporter;

impl Importer for FbxImporter {
    fn import(&self, path: &str, _settings: &ImportSettings) -> AssetResult<ImportedAsset> {
        // Would parse FBX
        Err(AssetError::new(AssetErrorKind::ImportError, "FBX import not implemented"))
    }
    
    fn supported_extensions(&self) -> &[&str] {
        &["fbx"]
    }
}

/// OBJ model importer
pub struct ObjImporter;

impl Importer for ObjImporter {
    fn import(&self, path: &str, settings: &ImportSettings) -> AssetResult<ImportedAsset> {
        // Would parse OBJ/MTL
        GltfImporter.import(path, settings) // Fallback for demo
    }
    
    fn supported_extensions(&self) -> &[&str] {
        &["obj"]
    }
}

/// HLSL shader importer
pub struct HlslImporter;

impl Importer for HlslImporter {
    fn import(&self, path: &str, _settings: &ImportSettings) -> AssetResult<ImportedAsset> {
        use crate::{ImportedShader, ShaderStageSource, ShaderStageType};
        
        let shader = ImportedShader {
            name: extract_name(path),
            stages: vec![
                ShaderStageSource {
                    stage: ShaderStageType::Vertex,
                    source: String::from("// Vertex shader"),
                    entry_point: String::from("VSMain"),
                },
                ShaderStageSource {
                    stage: ShaderStageType::Fragment,
                    source: String::from("// Fragment shader"),
                    entry_point: String::from("PSMain"),
                },
            ],
            defines: alloc::collections::BTreeMap::new(),
            includes: Vec::new(),
        };
        
        Ok(ImportedAsset {
            asset_type: AssetType::Shader,
            metadata: AssetMetadata {
                id: AssetId::from_path(path),
                name: extract_name(path),
                asset_type: AssetType::Shader,
                source_path: Some(path.into()),
                import_time: 0,
                size_bytes: 0,
                gpu_size_bytes: 0,
                dependencies: Vec::new(),
                tags: Vec::new(),
                custom_data: alloc::collections::BTreeMap::new(),
            },
            data: ImportedData::Shader(shader),
        })
    }
    
    fn supported_extensions(&self) -> &[&str] {
        &["hlsl", "hlsli"]
    }
}

/// GLSL shader importer
pub struct GlslImporter;

impl Importer for GlslImporter {
    fn import(&self, path: &str, settings: &ImportSettings) -> AssetResult<ImportedAsset> {
        HlslImporter.import(path, settings)
    }
    
    fn supported_extensions(&self) -> &[&str] {
        &["glsl", "vert", "frag", "comp", "geom", "tesc", "tese"]
    }
}

fn extract_name(path: &str) -> String {
    path.rsplit('/').next()
        .and_then(|s| s.rsplit('.').last())
        .unwrap_or("unnamed")
        .into()
}
