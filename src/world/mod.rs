//! Defines the [`World`], the center of an ECS.

use std::mem;

pub use self::ptr::*;
use crate::access::AccessError;
use crate::commands::{Commands, EntityQueue};
use crate::component::{Bundle, ComponentWriter, Components};
use crate::entity::{
    Entities,
    EntityId,
    EntityNotFound,
    EntityRef,
    EntitySlots,
    EntityWorld,
};
use crate::query::{Query, QueryData, ReadOnlyQueryData};

mod ptr;
#[cfg(test)]
mod tests;

/// Stores all ECS data.
///
/// - [Entity methods](#entity-methods)
#[derive(Debug)]
pub struct World {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    /// Storage for internally-buffered commands.
    pub(crate) commands: Commands,
}

/// An iterator over all entities in a [`World`].
#[derive(Clone)]
pub struct EntitiesIter<'w> {
    inner: EntitySlots<'w>,
}

/// An iterator over entities created by [`World::spawn_iter`].
#[derive(Clone)]
pub struct SpawnIter<'w> {
    inner: EntitySlots<'w>,
}

impl World {
    /// Creates a new empty world.
    pub fn new() -> Self {
        let entities = Entities::new();
        let components = Components::new();
        let commands = Commands::new();

        Self { entities, components, commands }
    }

    /// Returns a pointer to this world.
    pub fn as_ptr(&self) -> WorldPtr<'_> {
        WorldPtr::from_ref(self)
    }

    /// Returns a pointer to this world.
    pub fn as_ptr_mut(&mut self) -> WorldPtr<'_> {
        WorldPtr::from_mut(self)
    }

    /// Removes all entities from the world.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.clear();
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// # Entity methods
///
/// Methods for creating and managing entities.
impl World {
    /// Returns the count of live entities in this world.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` if this world contains no entities.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Returns `true` if this world contains this entity.
    pub fn contains(&self, entity: EntityId) -> bool {
        self.entities.contains(entity)
    }

    /// Returns an iterator over the entities in this world.
    pub fn iter(&self) -> EntitiesIter<'_> {
        EntitiesIter { inner: self.entities.iter() }
    }

    /// Borrows an entity in this world.
    ///
    /// Returns an error if the entity doesn't exist in this world.
    pub fn entity(
        &self,
        entity: EntityId,
    ) -> Result<EntityRef<'_>, EntityNotFound> {
        EntityRef::new(entity, self)
    }

    /// Mutably borrows an entity and this world.
    ///
    /// Returns an error if the entity doesn't exist in this world.
    pub fn entity_mut(
        &mut self,
        entity: EntityId,
    ) -> Result<EntityWorld<'_>, EntityNotFound> {
        EntityWorld::new(entity, self)
    }

    /// Returns a query of data from this world.
    ///
    /// Returns an error if the query access is invalid.
    ///
    /// The query data must implement [`ReadOnlyQueryData`].
    pub fn query<D: ReadOnlyQueryData>(
        &self,
    ) -> Result<Query<'_, D>, AccessError> {
        Query::from_ref(self)
    }

    /// Returns a mutable query of data from this world.
    ///
    /// Returns an error if the query access is invalid.
    pub fn query_mut<D: QueryData>(
        &mut self,
    ) -> Result<Query<'_, D>, AccessError> {
        Query::from_mut(self)
    }

    /// Spawns a new entity with its components.
    ///
    /// Returns an [`EntityWorld`] to allow editing of the produced entity.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityWorld<'_> {
        let entity = self.entities.alloc();

        unsafe { self.spawn_at(entity, bundle) }
    }

    #[inline]
    pub(crate) unsafe fn spawn_at(
        &mut self,
        entity: EntityId,
        bundle: impl Bundle,
    ) -> EntityWorld<'_> {
        #[track_caller]
        #[inline(always)]
        unsafe fn spawn_at_inner<B: Bundle>(
            world: &mut World,
            entity: EntityId,
            bundle: B,
        ) -> EntityWorld<'_> {
            {
                let queue = EntityQueue::new(entity, &mut world.commands);
                let addr = world.components.alloc::<B>(1);

                world.entities.set(entity, addr);
                // SAFETY: the index is valid as it was just allocated and the
                // table doesn't contain this entity because it was
                // only allocated above
                unsafe {
                    world.components.get_unchecked_mut(addr.table).push(entity)
                };
                bundle.write(&mut ComponentWriter::new(
                    queue,
                    &mut world.components,
                    addr,
                ));
            }

            world.flush();

            // SAFETY: the entity was allocated above, so it must exist
            unsafe { EntityWorld::new_unchecked(entity, world) }
        }

        unsafe { spawn_at_inner(self, entity, bundle) }
    }

    /// Spawns an entity for each bundle in an iterator.
    ///
    /// More efficient than calling [`World::spawn`] on each bundle.
    pub fn spawn_iter<B: Bundle>(
        &mut self,
        bundles: impl IntoIterator<Item = B>,
    ) -> SpawnIter<'_> {
        self.entities.flush();

        let bundles = bundles.into_iter();

        let (lower, upper) = bundles.size_hint();
        let count = upper.unwrap_or(lower);

        let first_index = self.entities.len();
        // allocates enough space to hold the last entity
        let addr = self.components.alloc::<B>((first_index + count) as _);
        let mut allocated = self.entities.alloc_many(count);

        for bundle in bundles {
            let entity = allocated
                .next()
                .map(|index| index as _)
                .map(EntityId::from_index)
                .unwrap_or_else(|| self.entities.alloc_end());

            self.entities.set(entity, addr);
            bundle.write(&mut ComponentWriter::new(
                EntityQueue::new(entity, &mut self.commands),
                &mut self.components,
                addr,
            ));
        }

        self.flush();

        SpawnIter { inner: self.entities.iter_slice(first_index..) }
    }

    /// Despawns an entity.
    ///
    /// Returns an error if the entity doesn't exist in the world.
    pub fn despawn(&mut self, entity: EntityId) -> Result<(), EntityNotFound> {
        self.entity_mut(entity).map(EntityWorld::despawn)
    }

    /// Ensures all entities are allocated and applies all buffered commands.
    pub(crate) fn flush(&mut self) {
        self.entities.flush();

        let mut commands = mem::replace(&mut self.commands, Commands::new());

        commands.apply(self);
        self.commands = commands;
    }
}

impl Iterator for EntitiesIter<'_> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(entity, _)| entity)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for EntitiesIter<'_> {}

impl Iterator for SpawnIter<'_> {
    type Item = EntityId;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(entity, _)| entity)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for SpawnIter<'_> {}
