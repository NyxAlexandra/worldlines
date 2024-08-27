#![allow(clippy::should_implement_trait)]

use std::ops::{Deref, DerefMut};
use std::{fmt, ptr};

use thiserror::Error;

use crate::{
    Component,
    Entity,
    QueryData,
    ReadOnlyQueryData,
    Table,
    TableId,
    TypeData,
    World,
    WorldAccess,
    WorldPtr,
};

/// An immutable reference to an entity.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EntityRef<'w> {
    pub(super) inner: EntityPtr<'w>,
}

/// A mutable reference to an entity.
#[repr(transparent)]
pub struct EntityMut<'w> {
    pub(super) inner: EntityPtr<'w>,
}

/// A mutable reference to an entity and the world.
#[repr(transparent)]
pub struct EntityWorld<'w> {
    inner: EntityPtr<'w>,
}

/// A pointer to an entity.
///
/// Does not guarantee liveness of the entity.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityPtr<'w> {
    world: WorldPtr<'w>,
    entity: Entity,
}

impl<'w> EntityPtr<'w> {
    pub(crate) const fn new(world: WorldPtr<'w>, entity: Entity) -> Self {
        Self { world, entity }
    }

    /// The [`Entity`] this points to.
    pub const fn id(self) -> Entity {
        self.entity
    }

    /// The amount of components in this entity.
    unsafe fn len(self) -> usize {
        unsafe { self.table() }.header().len()
    }

    /// Returns `true` if this entity has no components.
    unsafe fn is_empty(self) -> bool {
        unsafe { self.len() == 0 }
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

    /// Returns `true` if this entity contains the component.
    unsafe fn contains<C: Component>(self) -> bool {
        unsafe { self.table() }.header().contains::<C>()
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

    /// Insert a new component into this entity. Returns the previous value (if
    /// present).
    ///
    /// # Safety
    ///
    /// - The world pointer must point to a valid world and be usable for
    ///   structural writes.
    /// - The entity must be alive.
    unsafe fn insert<C: Component>(&mut self, component: C) -> Option<C> {
        if unsafe { self.contains::<C>() } {
            let table = unsafe { self.table_mut() };

            table.replace(self.entity, component)
        } else {
            let old_table = unsafe { self.table_id() };
            let new_table = unsafe {
                let components = self.world.components_mut();

                components
                    .realloc_with(
                        self.entity,
                        old_table,
                        TypeData::of::<C>(),
                        true,
                        |table| {
                            table.write(self.entity, component);
                        },
                    )
                    // SAFETY: the table is guaranteed to contain this entity
                    .unwrap_unchecked()
            };

            unsafe { self.world.entities_mut().set(self.entity, new_table) };

            None
        }
    }

    /// Remove a component from this entity.
    ///
    /// # Safety
    ///
    /// - The world pointer must point to a valid world and be usable for
    ///   structural writes.
    /// - The entity must be alive.
    unsafe fn remove<C: Component>(&mut self) -> Result<C, ComponentNotFound> {
        let component = TypeData::of::<C>();

        unsafe {
            self.contains::<C>()
                .then(|| {
                    let old_table = self.table_id();
                    let new_table = {
                        let components = self.world.components_mut();

                        components
                            .realloc_without(self.entity, old_table, component, false)
                            // SAFETY: the table is guaranteed to contain this entity
                            .unwrap_unchecked()
                    };

                    self.world.entities_mut().set(self.entity, new_table);

                    self.world
                        .components_mut()
                        .table_mut(old_table)
                        .unwrap_unchecked()
                        .get_ptr_unchecked_mut(self.entity, component)
                        .cast::<C>()
                        .read()
                })
                .ok_or(ComponentNotFound::new::<C>(self.entity))
        }
    }

    unsafe fn table_id(self) -> TableId {
        unsafe { self.world.entities().get(self.entity) }
            .expect("`EntityPtr` used to access non-alive entity")
    }

    unsafe fn table(&self) -> &'w Table {
        // SAFETY: [`EntityPtr::table_id`] panics if the entity isn't alive
        unsafe { self.world.components().table(self.table_id()).unwrap_unchecked() }
    }

    unsafe fn table_mut(&mut self) -> &'w mut Table {
        unsafe {
            // SAFETY: [`EntityPtr::table_id`] panics if the entity isn't alive
            self.world.components_mut().table_mut(self.table_id()).unwrap_unchecked()
        }
    }
}

/// Error when accessing a [`Component`] an [`Entity`] does not contain.
#[derive(Clone, Copy, PartialEq, Error)]
#[error("component {component} not found for entity {entity:?}")]
pub struct ComponentNotFound {
    entity: Entity,
    component: TypeData,
}

impl ComponentNotFound {
    fn new<C: Component>(entity: Entity) -> Self {
        Self { entity, component: TypeData::of::<C>() }
    }
}

impl fmt::Debug for ComponentNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'w> EntityRef<'w> {
    pub(crate) fn new(world: &'w World, entity: Entity) -> Result<Self, EntityNotFound> {
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
        unsafe { self.inner.len() }
    }

    /// Returns `true` if this entity has no components.
    pub fn is_empty(self) -> bool {
        unsafe { self.inner.is_empty() }
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

/// Error when trying to access an [`Entity`] that cannot be found.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Error)]
#[error("entity {0:?} not found")]
pub struct EntityNotFound(pub(crate) Entity);

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
    pub fn get_mut<C: Component>(&mut self) -> Result<&'w mut C, ComponentNotFound> {
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

    /// Returns `true` if this entity contains the component.
    pub fn contains<C: Component>(&self) -> bool {
        self.as_ref().contains::<C>()
    }

    /// Borrow a component of this entity.
    pub fn get<C: Component>(&self) -> Result<&'w C, ComponentNotFound> {
        self.as_ref().get()
    }

    /// Mutably borrow a component of this entity.
    pub fn get_mut<C: Component>(&mut self) -> Result<&'w mut C, ComponentNotFound> {
        self.as_mut().get_mut()
    }

    /// Insert a new component into this entity. Returns the previous value (if
    /// present).
    pub fn insert<C: Component>(&mut self, component: C) -> Option<C> {
        unsafe { self.inner.insert(component) }
    }

    /// Call [`EntityWorld::insert`] and return `self`.
    pub fn and_insert<C: Component>(&mut self, component: C) -> &mut Self {
        self.insert(component);

        self
    }

    /// Remove a component from this entity.
    pub fn remove<C: Component>(&mut self) -> Result<C, ComponentNotFound> {
        unsafe { self.inner.remove() }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn len() {
        struct A;
        struct B;

        let mut world = World::new();

        let e0 = world.spawn((A,)).id();
        let e1 = world.spawn((A, B)).id();

        unsafe {
            assert_eq!(world.as_ptr().entity(e0).len(), 1);
            assert_eq!(world.as_ptr().entity(e1).len(), 2);
        }
    }

    #[test]
    fn insert() {
        struct Name(&'static str);
        struct Age(u32);

        let mut world = World::new();
        let mut entity = world.spawn((Name("Sasha"),));

        assert_eq!(entity.get::<Name>().unwrap().0, "Sasha");

        assert!(entity.insert(Age(123)).is_none());

        assert_eq!(entity.get::<Name>().unwrap().0, "Sasha");
        assert_eq!(entity.get::<Age>().unwrap().0, 123);
    }

    #[test]
    fn remove() {
        struct Name(&'static str);
        struct Age(#[allow(dead_code)] u32);

        let mut world = World::new();
        let mut entity = world.spawn((Name("Sasha"), Age(u32::MAX)));

        assert!(entity.remove::<Age>().is_ok());
        assert_eq!(entity.get::<Name>().unwrap().0, "Sasha");
        assert!(entity.get::<Age>().is_err());
    }

    #[test]
    fn component_not_found() {
        struct A;
        struct B;

        let mut world = World::new();
        let entity = world.spawn((A,));

        assert_eq!(
            entity.get::<B>().err(),
            Some(ComponentNotFound::new::<B>(entity.id())),
        );
    }
}
