//! Defines [`EntityRef`] and [`EntityMut`], references to entities in the
//! world.

use std::ptr;

use super::{EntityAddr, EntityId, EntityNotFound, EntityPtr};
use crate::component::{Component, ComponentNotFound};
use crate::prelude::ComponentId;
use crate::storage::Table;
use crate::world::World;

/// A reference to an entity and its components.
#[derive(Debug, Clone, Copy)]
pub struct EntityRef<'w> {
    ptr: EntityPtr<'w>,
    addr: EntityAddr,
}

/// A reference to an entity and its components.
#[derive(Debug)]
pub struct EntityMut<'w> {
    ptr: EntityPtr<'w>,
    addr: EntityAddr,
}

impl<'w> EntityRef<'w> {
    /// Creates a new entity reference.
    ///
    /// Returns an error if the entity doesn't exist in the world.
    pub fn new(id: EntityId, world: &'w World) -> Result<Self, EntityNotFound> {
        if world.contains(id) {
            Ok(unsafe { Self::new_unchecked(id, world) })
        } else {
            Err(EntityNotFound(id))
        }
    }

    /// Creates a new entity reference without checking liveness.
    ///
    /// # Safety
    ///
    /// The entity must be alive.
    pub unsafe fn new_unchecked(id: EntityId, world: &'w World) -> Self {
        // SAFETY: the world contains this entity
        let addr = unsafe { world.entities.get(id).unwrap_unchecked() };
        let ptr = world.as_ptr().entity(id);

        Self { ptr, addr }
    }

    /// Returns the id of this entity.
    pub const fn id(self) -> EntityId {
        self.ptr.id()
    }

    fn table(self) -> &'w Table {
        unsafe {
            self.ptr.world().as_ref().components.get_unchecked(self.addr.table)
        }
    }

    /// Returns `true` if this entity contains the component.
    pub fn contains<C: Component>(self) -> bool {
        self.table().components().contains(ComponentId::of::<C>())
    }

    /// Returns a reference to a component of this entity.
    ///
    /// Returns an error if the component doesn't exist.
    pub fn get<C: Component>(self) -> Result<&'w C, ComponentNotFound> {
        let component = ComponentId::of::<C>();
        let table = self.table();

        table
            .components()
            .contains(component)
            .then(|| unsafe {
                self.table()
                    .get_unchecked(self.addr.row, component)
                    .cast()
                    .as_ref()
            })
            .ok_or(ComponentNotFound::new::<C>(self.id()))
    }
}

impl<'w> EntityMut<'w> {
    /// Creates a new mutable entity reference.
    ///
    /// Returns an error if the entity doesn't exist in the world.
    pub fn new(
        id: EntityId,
        world: &'w mut World,
    ) -> Result<Self, EntityNotFound> {
        if world.contains(id) {
            // SAFETY: the world contains this entity
            let table = unsafe { world.entities.get(id).unwrap_unchecked() };
            let ptr = world.as_ptr_mut().entity(id);

            Ok(Self { ptr, addr: table })
        } else {
            Err(EntityNotFound(id))
        }
    }

    /// Creates a new mutable entity reference without checking liveness.
    ///
    /// # Safety
    ///
    /// The entity must be alive.
    pub unsafe fn new_unchecked(id: EntityId, world: &'w mut World) -> Self {
        let table = unsafe { world.entities.get(id).unwrap_unchecked() };
        let ptr = world.as_ptr_mut().entity(id);

        Self { ptr, addr: table }
    }

    /// Returns the id of this entity.
    pub const fn id(&self) -> EntityId {
        self.ptr.id()
    }

    fn table_mut(&mut self) -> &mut Table {
        unsafe {
            self.ptr
                .world()
                .as_mut()
                .components
                .get_unchecked_mut(self.addr.table)
        }
    }

    /// Borrows this entity as an [`EntityRef`].
    pub fn as_ref(&self) -> EntityRef<'w> {
        // SAFETY: `EntityRef` and `EntityMut` have the same layout in memory
        unsafe { *ptr::from_ref(self).cast() }
    }

    /// Returns `true` if this entity contains the component.
    pub fn contains<C: Component>(&self) -> bool {
        self.as_ref().contains::<C>()
    }

    /// Returns a reference to a component of this entity.
    ///
    /// Returns an error if the component doesn't exist.
    pub fn get<C: Component>(&self) -> Result<&'w C, ComponentNotFound> {
        self.as_ref().get()
    }

    /// Returns a mutable reference to a component of this entity.
    ///
    /// Returns an error if the component doesn't exist.
    pub fn get_mut<C: Component>(
        &mut self,
    ) -> Result<&'w mut C, ComponentNotFound> {
        let component = ComponentId::of::<C>();
        let row = self.addr.row;
        let table = self.table_mut();

        table
            .components()
            .contains(component)
            .then(|| unsafe {
                table.get_unchecked_mut(row, component).cast::<C>().as_mut()
            })
            .ok_or(ComponentNotFound::new::<C>(self.id()))
    }
}
