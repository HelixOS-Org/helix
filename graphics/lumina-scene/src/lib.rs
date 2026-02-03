//! # LUMINA Scene Graph
//!
//! High-performance scene management with ECS architecture.
//!
//! ## Features
//!
//! - **ECS**: Entity-Component-System architecture
//! - **Transforms**: Hierarchical transform system
//! - **Culling**: Frustum and occlusion culling
//! - **Instancing**: GPU instancing for repeated geometry
//! - **LOD**: Level of detail selection
//! - **Streaming**: Scene streaming and paging

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod culling;
pub mod ecs;
pub mod graph;
pub mod instancing;
pub mod lod;
pub mod streaming;
pub mod transform;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Scene manager
pub struct Scene {
    name: String,
    world: ecs::World,
    root: NodeId,
    nodes: BTreeMap<NodeId, SceneNode>,
    next_node_id: u64,
    transform_system: transform::TransformSystem,
    culling_system: culling::CullingSystem,
    instance_manager: instancing::InstanceManager,
    lod_system: lod::LodSystem,
}

impl Scene {
    /// Create a new scene
    pub fn new(name: &str) -> Self {
        let mut nodes = BTreeMap::new();
        let root_id = NodeId(0);
        nodes.insert(root_id, SceneNode {
            name: "Root".into(),
            parent: None,
            children: Vec::new(),
            entity: None,
            visible: true,
            static_object: false,
        });

        Self {
            name: name.into(),
            world: ecs::World::new(),
            root: root_id,
            nodes,
            next_node_id: 1,
            transform_system: transform::TransformSystem::new(),
            culling_system: culling::CullingSystem::new(),
            instance_manager: instancing::InstanceManager::new(),
            lod_system: lod::LodSystem::new(),
        }
    }

    /// Create a new entity
    pub fn create_entity(&mut self) -> ecs::Entity {
        self.world.create_entity()
    }

    /// Create a node and attach to parent
    pub fn create_node(&mut self, name: &str, parent: Option<NodeId>) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;

        let parent_id = parent.unwrap_or(self.root);

        self.nodes.insert(id, SceneNode {
            name: name.into(),
            parent: Some(parent_id),
            children: Vec::new(),
            entity: None,
            visible: true,
            static_object: false,
        });

        if let Some(parent_node) = self.nodes.get_mut(&parent_id) {
            parent_node.children.push(id);
        }

        id
    }

    /// Attach an entity to a node
    pub fn attach_entity(&mut self, node: NodeId, entity: ecs::Entity) {
        if let Some(node) = self.nodes.get_mut(&node) {
            node.entity = Some(entity);
        }
    }

    /// Get node
    pub fn get_node(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    /// Get mutable node
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    /// Remove a node and its children
    pub fn remove_node(&mut self, id: NodeId) {
        if id == self.root {
            return; // Cannot remove root
        }

        // Collect children recursively
        let mut to_remove = Vec::new();
        self.collect_children(id, &mut to_remove);
        to_remove.push(id);

        // Remove from parent's children list
        if let Some(node) = self.nodes.get(&id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|&c| c != id);
                }
            }
        }

        // Remove all nodes and their entities
        for node_id in to_remove {
            if let Some(node) = self.nodes.remove(&node_id) {
                if let Some(entity) = node.entity {
                    self.world.destroy_entity(entity);
                }
            }
        }
    }

    fn collect_children(&self, id: NodeId, out: &mut Vec<NodeId>) {
        if let Some(node) = self.nodes.get(&id) {
            for &child in &node.children {
                self.collect_children(child, out);
                out.push(child);
            }
        }
    }

    /// Update the scene
    pub fn update(&mut self, dt: f32) {
        // Update transforms
        self.transform_system.update(&mut self.world);

        // Update LOD
        self.lod_system.update(&mut self.world);
    }

    /// Perform culling and prepare for rendering
    pub fn prepare_render(&mut self, camera: &lumina_3d::camera::Camera) -> RenderBatch {
        // Perform frustum culling
        let planes = camera.frustum_planes();
        self.culling_system.perform_culling(&self.world, &planes);

        // Collect visible entities
        let mut renderables = Vec::new();

        for (_, node) in &self.nodes {
            if !node.visible {
                continue;
            }

            if let Some(entity) = node.entity {
                if self.culling_system.is_visible(entity) {
                    // Get mesh and material components
                    if let Some(mesh) = self.world.get_component::<MeshComponent>(entity) {
                        let transform = self.world.get_component::<transform::Transform>(entity);
                        let material = self.world.get_component::<MaterialComponent>(entity);

                        renderables.push(RenderItem {
                            entity,
                            mesh_id: mesh.mesh_id,
                            material_id: material.map(|m| m.material_id).unwrap_or(0),
                            transform: transform.cloned().unwrap_or_default(),
                            lod_level: self.lod_system.get_lod(entity),
                        });
                    }
                }
            }
        }

        // Group by material and mesh for batching
        renderables.sort_by_key(|r| (r.material_id, r.mesh_id));

        // Prepare GPU instances
        self.instance_manager.prepare(&renderables);

        RenderBatch {
            items: renderables,
            instance_data: self.instance_manager.get_instance_buffer(),
        }
    }

    /// Get world for direct access
    pub fn world(&self) -> &ecs::World {
        &self.world
    }

    /// Get mutable world
    pub fn world_mut(&mut self) -> &mut ecs::World {
        &mut self.world
    }
}

/// Node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Scene node
pub struct SceneNode {
    pub name: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub entity: Option<ecs::Entity>,
    pub visible: bool,
    pub static_object: bool,
}

/// Render batch
pub struct RenderBatch {
    pub items: Vec<RenderItem>,
    pub instance_data: Vec<u8>,
}

/// Render item
#[derive(Debug, Clone)]
pub struct RenderItem {
    pub entity: ecs::Entity,
    pub mesh_id: u64,
    pub material_id: u64,
    pub transform: transform::Transform,
    pub lod_level: u8,
}

/// Mesh component
#[derive(Debug, Clone)]
pub struct MeshComponent {
    pub mesh_id: u64,
    pub sub_mesh: u32,
}

/// Material component
#[derive(Debug, Clone)]
pub struct MaterialComponent {
    pub material_id: u64,
}

/// Scene loader
pub struct SceneLoader;

impl SceneLoader {
    /// Load scene from glTF
    pub fn load_gltf(_data: &[u8]) -> Result<Scene, SceneLoadError> {
        // Would parse glTF and create scene
        Ok(Scene::new("Loaded Scene"))
    }

    /// Save scene
    pub fn save(_scene: &Scene) -> Vec<u8> {
        Vec::new()
    }
}

/// Scene load error
#[derive(Debug)]
pub enum SceneLoadError {
    ParseError,
    InvalidFormat,
    MissingData,
}
