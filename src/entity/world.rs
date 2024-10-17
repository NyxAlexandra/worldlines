use std::ops::{Deref, DerefMut};
use std::ptr;

use super::{
    ComponentNotFound,
    Entity,
    EntityMut,
    EntityNotFound,
    EntityPtr,
    EntityRef,
};
use crate::{Component, TypeData, World};

/// A mutable reference to an entity and the world.
#[repr(transparent)]
pub struct EntityWorld<'w> {
    pub(crate) inner: EntityPtr<'w>,
}

impl<'w> EntityWorld<'w> {
    pub(crate) fn new(
        world: &'w mut World,
        entity: Entity,
    ) -> Result<Self, EntityNotFound> {
        world
            .contains(entity)
            .then(|| Self { inner: world.as_ptr_mut().entity(entity) })
            .ok_or(EntityNotFound(entity))
    }

    /// The [`Entity`] this points to.
    pub const fn id(&self) -> Entity {
        self.inner.id()
    }

    /// Get the inner [`EntityPtr`].
    pub fn as_ptr(&self) -> EntityPtr<'w> {
        self.inner
    }

    /// The amount of components in this entity.
    pub fn len(&self) -> usize {
        self.as_ref().len()
    }

    /// Returns `true` if this entity has no components.
    pub fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    /// Borrow this entity as an [`EntityRef`].
    pub fn as_ref(&self) -> EntityRef<'w> {
        EntityRef { inner: self.inner }
    }

    /// Borrow this entity as an [`EntityMut`].
    pub fn as_mut(&mut self) -> EntityMut<'w> {
        EntityMut { inner: self.inner }
    }

    /// Despawns this entity.
    pub fn despawn(self) {
        unsafe {
            self.inner.world.as_mut().despawn(self.id()).unwrap();
        }
    }
}

/// Methods for accessing components.
impl<'w> EntityWorld<'w> {
    /// Returns `true` if this entity contains the component.
    pub fn contains<C: Component>(&self) -> bool {
        self.as_ref().contains::<C>()
    }

    /// Borrow a component of this entity.
    pub fn get<C: Component>(&self) -> Result<&'w C, ComponentNotFound> {
        self.as_ref().get()
    }

    /// Mutably borrow a component of this entity.
    pub fn get_mut<C: Component>(
        &mut self,
    ) -> Result<&'w mut C, ComponentNotFound> {
        self.as_mut().get_mut()
    }

    /// Returns a mutable reference to a component, inserting it if the entity
    /// doesn't have it.
    pub fn get_or_insert<C: Component>(&mut self, component: C) -> &'w mut C {
        self.get_or_insert_with(|| component)
    }

    /// Returns a mutable reference to a component, inserting it if the entity
    /// doesn't have it.
    pub fn get_or_insert_with<C: Component>(
        &mut self,
        component: impl FnOnce() -> C,
    ) -> &'w mut C {
        if !self.contains::<C>() {
            self.insert(component()).unwrap();
        }

        self.get_mut().unwrap()
    }

    /// Returns a mutable reference to a component, inserting the default value
    /// if the entity doesn't have it.
    pub fn get_or_default<C: Component + Default>(&mut self) -> &'w mut C {
        self.get_or_insert_with(Default::default)
    }

    /// Insert a new component into this entity. Returns the previous value (if
    /// present).
    pub fn insert<C: Component>(&mut self, component: C) -> Option<C> {
        if unsafe { self.inner.table() }.header().contains::<C>() {
            let table = unsafe { self.inner.table_mut() };

            table.replace(self.inner.entity, component)
        } else {
            unsafe {
                let old_table = self.inner.table_id();
                let new_table = {
                    let components = self.inner.world.components_mut();

                    components
                        .realloc_with(
                            self.inner.entity,
                            old_table,
                            TypeData::of::<C>(),
                            true,
                            |table| {
                                table.write(self.inner.entity, component);
                            },
                        )
                        // SAFETY: the table is guaranteed to contain self.inner
                        // entity
                        .unwrap_unchecked()
                };

                self.inner
                    .world
                    .entities_mut()
                    .set(self.inner.entity, new_table)
            };

            None
        }
    }

    /// Call [`EntityWorld::insert`] and return `self`.
    pub fn and_insert<C: Component>(&mut self, component: C) -> &mut Self {
        self.insert(component);

        self
    }

    /// Remove a component from this entity.
    pub fn remove<C: Component>(&mut self) -> Result<C, ComponentNotFound> {
        let component = TypeData::of::<C>();

        unsafe { self.inner.table() }
            .header()
            .contains::<C>()
            .then(|| unsafe {
                let old_table = self.inner.table_id();
                let new_table = {
                    let components = self.inner.world.components_mut();

                    components
                        .realloc_without(
                            self.inner.entity,
                            old_table,
                            component,
                            false,
                        )
                        // SAFETY: the table is guaranteed to contain self
                        // entity
                        .unwrap_unchecked()
                };

                self.inner
                    .world
                    .entities_mut()
                    .set(self.inner.entity, new_table);

                self.inner
                    .world
                    .components_mut()
                    .table_mut(old_table)
                    .unwrap_unchecked()
                    .get_ptr_unchecked_mut(self.inner.entity, component)
                    .cast::<C>()
                    .read()
            })
            .ok_or(ComponentNotFound::new::<C>(self.inner.entity))
    }

    /// Call [`EntityWorld::remove`] and return `self`.
    pub fn and_remove<C: Component>(&mut self) -> &mut Self {
        _ = self.remove::<C>();

        self
    }
}

impl<'w> Deref for EntityWorld<'w> {
    type Target = EntityMut<'w>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*ptr::from_ref(self).cast() }
    }
}

impl DerefMut for EntityWorld<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *ptr::from_mut(self).cast() }
    }
}
