//! # Entity-Component-System
//!
//! Lightweight ECS implementation.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::any::{Any, TypeId};

/// ECS World
pub struct World {
    entities: Vec<EntityData>,
    free_entities: Vec<u32>,
    components: BTreeMap<TypeId, ComponentStorage>,
    next_entity: u32,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            free_entities: Vec::new(),
            components: BTreeMap::new(),
            next_entity: 0,
        }
    }

    /// Create a new entity
    pub fn create_entity(&mut self) -> Entity {
        let id = if let Some(recycled) = self.free_entities.pop() {
            let gen = self.entities[recycled as usize].generation + 1;
            self.entities[recycled as usize] = EntityData {
                generation: gen,
                alive: true,
            };
            Entity {
                id: recycled,
                generation: gen,
            }
        } else {
            let id = self.next_entity;
            self.next_entity += 1;
            self.entities.push(EntityData {
                generation: 0,
                alive: true,
            });
            Entity { id, generation: 0 }
        };

        id
    }

    /// Destroy an entity
    pub fn destroy_entity(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
            return;
        }

        self.entities[entity.id as usize].alive = false;
        self.free_entities.push(entity.id);

        // Remove all components
        for storage in self.components.values_mut() {
            storage.remove(entity);
        }
    }

    /// Check if entity is alive
    pub fn is_alive(&self, entity: Entity) -> bool {
        if let Some(data) = self.entities.get(entity.id as usize) {
            data.alive && data.generation == entity.generation
        } else {
            false
        }
    }

    /// Add a component to an entity
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();

        self.components
            .entry(type_id)
            .or_insert_with(ComponentStorage::new::<T>)
            .insert(entity, Box::new(component));
    }

    /// Get a component
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        self.components
            .get(&type_id)?
            .get(entity)?
            .downcast_ref::<T>()
    }

    /// Get a mutable component
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();

        self.components
            .get_mut(&type_id)?
            .get_mut(entity)?
            .downcast_mut::<T>()
    }

    /// Remove a component
    pub fn remove_component<T: Component>(&mut self, entity: Entity) {
        let type_id = TypeId::of::<T>();

        if let Some(storage) = self.components.get_mut(&type_id) {
            storage.remove(entity);
        }
    }

    /// Check if entity has component
    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        let type_id = TypeId::of::<T>();

        self.components
            .get(&type_id)
            .map(|s| s.contains(entity))
            .unwrap_or(false)
    }

    /// Query entities with specific components
    pub fn query<T: Component>(&self) -> QueryIter<'_, T> {
        let type_id = TypeId::of::<T>();

        QueryIter {
            storage: self.components.get(&type_id),
            index: 0,
            _marker: core::marker::PhantomData,
        }
    }

    /// Get entity count
    pub fn entity_count(&self) -> usize {
        self.entities.iter().filter(|e| e.alive).count()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Entity handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    pub id: u32,
    pub generation: u32,
}

/// Entity internal data
struct EntityData {
    generation: u32,
    alive: bool,
}

/// Component trait
pub trait Component: Any + Send + Sync + 'static {}

impl<T: Any + Send + Sync + 'static> Component for T {}

/// Component storage
struct ComponentStorage {
    data: BTreeMap<u32, Box<dyn Any + Send + Sync>>,
    generations: BTreeMap<u32, u32>,
}

impl ComponentStorage {
    fn new<T: Component>() -> Self {
        Self {
            data: BTreeMap::new(),
            generations: BTreeMap::new(),
        }
    }

    fn insert(&mut self, entity: Entity, component: Box<dyn Any + Send + Sync>) {
        self.data.insert(entity.id, component);
        self.generations.insert(entity.id, entity.generation);
    }

    fn get(&self, entity: Entity) -> Option<&Box<dyn Any + Send + Sync>> {
        if self.generations.get(&entity.id) == Some(&entity.generation) {
            self.data.get(&entity.id)
        } else {
            None
        }
    }

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Box<dyn Any + Send + Sync>> {
        if self.generations.get(&entity.id) == Some(&entity.generation) {
            self.data.get_mut(&entity.id)
        } else {
            None
        }
    }

    fn remove(&mut self, entity: Entity) {
        if self.generations.get(&entity.id) == Some(&entity.generation) {
            self.data.remove(&entity.id);
            self.generations.remove(&entity.id);
        }
    }

    fn contains(&self, entity: Entity) -> bool {
        self.generations.get(&entity.id) == Some(&entity.generation)
            && self.data.contains_key(&entity.id)
    }
}

/// Query iterator
pub struct QueryIter<'a, T> {
    storage: Option<&'a ComponentStorage>,
    index: usize,
    _marker: core::marker::PhantomData<T>,
}

impl<'a, T: Component> Iterator for QueryIter<'a, T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let storage = self.storage?;

        let entries: Vec<_> = storage.data.iter().collect();

        while self.index < entries.len() {
            let (id, component) = entries[self.index];
            self.index += 1;

            if let Some(&gen) = storage.generations.get(id) {
                if let Some(c) = component.downcast_ref::<T>() {
                    return Some((
                        Entity {
                            id: *id,
                            generation: gen,
                        },
                        c,
                    ));
                }
            }
        }

        None
    }
}

/// System trait
pub trait System {
    fn update(&mut self, world: &mut World, dt: f32);
}

/// System scheduler
pub struct SystemScheduler {
    systems: Vec<Box<dyn System + Send + Sync>>,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add<S: System + Send + Sync + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }

    pub fn update(&mut self, world: &mut World, dt: f32) {
        for system in &mut self.systems {
            system.update(world, dt);
        }
    }
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource container
pub struct Resources {
    resources: BTreeMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
        }
    }

    pub fn insert<T: Any + Send + Sync + 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.resources.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn get_mut<T: Any + Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<T>()
    }

    pub fn remove<T: Any + Send + Sync + 'static>(&mut self) -> Option<T> {
        let boxed = self.resources.remove(&TypeId::of::<T>())?;
        boxed.downcast::<T>().ok().map(|b| *b)
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}
