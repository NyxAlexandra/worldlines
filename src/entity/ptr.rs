#![allow(clippy::should_implement_trait)]

use std::ops::Deref;
use std::ptr;

use super::{ComponentNotFound, EntityNotFound, EntityWorld};
use crate::{
    Component,
    Entity,
    QueryData,
    ReadOnlyQueryData,
    Table,
    TableId,
    World,
    WorldAccess,
    WorldPtr,
};

/// A pointer to an entity.
///
/// Does not guarantee liveness of the entity.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityPtr<'w> {
    pub(crate) world: WorldPtr<'w>,
    pub(crate) entity: Entity,
}

/// An immutable reference to an entity.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EntityRef<'w> {
    pub(crate) inner: EntityPtr<'w>,
}

/// A mutable reference to an entity.
#[repr(transparent)]
pub struct EntityMut<'w> {
    pub(crate) inner: EntityPtr<'w>,
}

impl<'w> EntityPtr<'w> {
    pub(crate) const fn new(world: WorldPtr<'w>, entity: Entity) -> Self {
        Self { world, entity }
    }

    /// The [`Entity`] this points to.
    pub const fn id(self) -> Entity {
        self.entity
    }

    /// Convert this pointer to an [`EntityRef`].
    ///
    /// # Safety
    ///
    /// - The entity must be alive.
    /// - The pointer must be valid to access this entity.
    pub unsafe fn as_ref(self) -> EntityRef<'w> {
        EntityRef { inner: self }
    }

    /// Convert this pointer to an [`EntityMut`].
    ///
    /// # Safety
    ///
    /// - The entity must be alive.
    /// - The pointer must be valid to access this entity.
    pub unsafe fn as_mut(self) -> EntityMut<'w> {
        EntityMut { inner: self }
    }

    /// Convert this pointer to an [`EntityWorld`].
    ///
    /// # Safety
    ///
    /// - The entity must be alive.
    /// - The pointer must be valid to access the entire world mutably.
    pub unsafe fn as_world(self) -> EntityWorld<'w> {
        EntityWorld { inner: self }
    }

    /// Returns this pointer with a new entity id.
    pub fn with_id(self, entity: Entity) -> Self {
        Self { entity, ..self }
    }

    /// # Safety
    ///
    /// This must not be used to alias access to components.
    pub unsafe fn get<C: Component>(&self) -> Result<&'w C, ComponentNotFound> {
        unsafe { self.table() }
            .get(self.entity)
            .ok_or(ComponentNotFound::new::<C>(self.entity))
    }

    /// # Safety
    ///
    /// This must not be used to alias access to components.
    pub unsafe fn get_mut<C: Component>(
        &mut self,
    ) -> Result<&'w mut C, ComponentNotFound> {
        unsafe { self.table_mut() }
            .get_mut(self.entity)
            .ok_or(ComponentNotFound::new::<C>(self.entity))
    }

    pub(crate) unsafe fn table_id(self) -> TableId {
        unsafe { self.world.entities().get(self.entity) }
            .expect("`EntityPtr` used to access non-alive entity")
    }

    pub(crate) unsafe fn table(&self) -> &'w Table {
        // SAFETY: [`EntityPtr::table_id`] panics if the entity isn't alive
        unsafe {
            self.world.components().table(self.table_id()).unwrap_unchecked()
        }
    }

    pub(crate) unsafe fn table_mut(&mut self) -> &'w mut Table {
        unsafe {
            // SAFETY: [`EntityPtr::table_id`] panics if the entity isn't alive
            self.world
                .components_mut()
                .table_mut(self.table_id())
                .unwrap_unchecked()
        }
    }
}

impl<'w> EntityRef<'w> {
    pub(crate) fn new(
        world: &'w World,
        entity: Entity,
    ) -> Result<Self, EntityNotFound> {
        world
            .contains(entity)
            .then(|| Self { inner: world.as_ptr().entity(entity) })
            .ok_or(EntityNotFound(entity))
    }

    /// The [`Entity`] this points to.
    pub const fn id(self) -> Entity {
        self.inner.id()
    }

    /// Get the inner [`EntityPtr`].
    pub fn as_ptr(self) -> EntityPtr<'w> {
        self.inner
    }

    /// The amount of components in this entity.
    pub fn len(self) -> usize {
        unsafe { self.inner.table() }.header().len()
    }

    /// Returns `true` if this entity has no components.
    pub fn is_empty(self) -> bool {
        unsafe { self.inner.table() }.header().len() == 0
    }

    /// Returns `true` if this entity contains the component.
    pub fn contains<C: Component>(self) -> bool {
        unsafe { self.inner.table() }.header().contains::<C>()
    }

    /// Borrow a component of this entity.
    pub fn get<C: Component>(self) -> Result<&'w C, ComponentNotFound> {
        unsafe { self.inner.get() }
    }
}

unsafe impl QueryData for EntityRef<'_> {
    type Output<'w> = EntityRef<'w>;

    fn access(access: &mut WorldAccess) {
        access.entities();
    }

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        Some(unsafe { entity.as_ref() })
    }
}

unsafe impl ReadOnlyQueryData for EntityRef<'_> {}

impl<'w> EntityMut<'w> {
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
    pub const fn id(self) -> Entity {
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
        unsafe { self.inner.get_mut() }
    }
}

unsafe impl QueryData for EntityMut<'_> {
    type Output<'w> = EntityMut<'w>;

    fn access(access: &mut WorldAccess) {
        access.entities_mut();
    }

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        Some(unsafe { entity.as_mut() })
    }
}

impl<'w> Deref for EntityMut<'w> {
    type Target = EntityRef<'w>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*ptr::from_ref(self).cast() }
    }
}
