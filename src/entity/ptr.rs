use super::{EntityId, EntityMut, EntityRef};
use crate::component::Component;
use crate::prelude::ComponentNotFound;
use crate::world::WorldPtr;

/// A semantic pointer to an entity in the ECS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityPtr<'w> {
    id: EntityId,
    world: WorldPtr<'w>,
}

impl<'w> EntityPtr<'w> {
    /// Creates a new entity pointer.
    pub const fn new(id: EntityId, world: WorldPtr<'w>) -> Self {
        Self { id, world }
    }

    /// Returns the id of this entity.
    ///
    /// Not guaranteed to be valid.
    pub const fn id(self) -> EntityId {
        self.id
    }

    /// Returns the inner world pointer of this entity.
    pub fn world(self) -> WorldPtr<'w> {
        self.world
    }

    /// Get an entity reference from this pointer.
    ///
    /// Fails if the world doesn't contain this entity.
    ///
    /// # Safety
    ///
    /// The entity must exist in the world. The world must be valid for reads to
    /// this entity.
    pub unsafe fn as_ref(self) -> EntityRef<'w> {
        unsafe { EntityRef::new_unchecked(self.id, self.world.as_ref()) }
    }

    /// Get a mutable entity reference from this pointer.
    ///
    /// Fails if the world doesn't contain this entity.
    ///
    /// # Safety
    ///
    /// The entity must exist in the world. The world must be valid for
    /// reads/writes to this entity.
    pub unsafe fn as_mut(self) -> EntityMut<'w> {
        unsafe { EntityMut::new_unchecked(self.id, self.world.as_mut()) }
    }

    /// Borrows a component of this entity.
    ///
    /// # Safety
    ///
    ///  The world reference must be valid for reads to this entity.
    pub unsafe fn get<C: Component>(self) -> Result<&'w C, ComponentNotFound> {
        unsafe { self.as_ref().get() }
    }

    /// Mutably borrows a component of this entity.
    ///
    /// # Safety
    ///
    ///  The world reference must be valid for reads to this entity.
    pub unsafe fn get_mut<C: Component>(
        self,
    ) -> Result<&'w mut C, ComponentNotFound> {
        unsafe { self.as_mut().get_mut() }
    }

    /// Borrows a component of this entity.
    ///
    /// # Safety
    ///
    ///  The world reference must be valid for reads to this entity. The entity
    /// must contain the component.
    pub unsafe fn get_unchecked<C: Component>(self) -> &'w C {
        unsafe { self.get().unwrap_unchecked() }
    }

    /// Mutably borrows a component of this entity.
    ///
    /// # Safety
    ///
    /// The world reference must be valid for reads/writes to this entity. The
    /// entity must contain the component.
    pub unsafe fn get_unchecked_mut<C: Component>(self) -> &'w mut C {
        unsafe { self.get_mut().unwrap_unchecked() }
    }
}
