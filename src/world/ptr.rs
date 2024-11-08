use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::{fmt, ptr};

use super::World;
use crate::entity::{EntityId, EntityPtr};

/// A pointer to a [`World`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldPtr<'w> {
    world: *mut World,
    // stolen from Bevy
    _marker: PhantomData<(&'w World, &'w UnsafeCell<World>)>,
}

impl<'w> WorldPtr<'w> {
    /// Creates a world pointer from a world reference.
    pub const fn from_ref(world: &'w World) -> Self {
        Self { world: ptr::from_ref(world).cast_mut(), _marker: PhantomData }
    }

    /// Creates a world pointer from a world reference.
    pub const fn from_mut(world: &'w mut World) -> Self {
        Self { world: ptr::from_mut(world), _marker: PhantomData }
    }

    /// Dereferences the pointer.
    ///
    /// ## Safety
    ///
    /// This must not be used to alias access to the world.
    pub unsafe fn as_ref(self) -> &'w World {
        unsafe { &*self.world }
    }

    /// Mutably dereferences the pointer.
    ///
    /// ## Safety
    ///
    /// This must not be used to alias access to the world.
    pub unsafe fn as_mut(self) -> &'w mut World {
        unsafe { &mut *self.world }
    }

    /// Returns an entity pointer for the given id.
    pub fn entity(self, entity: EntityId) -> EntityPtr<'w> {
        EntityPtr::new(entity, self)
    }
}

impl<'w> fmt::Debug for WorldPtr<'w> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.world.fmt(f)
    }
}
