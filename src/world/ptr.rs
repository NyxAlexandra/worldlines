use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::{fmt, ptr};

use crate::{
    Components, Entities, Entity, EntityPtr, ReadOnlySystemInput, SystemInput, World,
    WorldAccess,
};

/// A pointer to a [`World`].
///
/// Is not guaranteed to be valid.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldPtr<'w> {
    world: *mut World,
    // stolen from Bevy
    _marker: PhantomData<(&'w World, &'w UnsafeCell<World>)>,
}

impl<'w> WorldPtr<'w> {
    pub(crate) const fn from_ref(world: &'w World) -> Self {
        Self { world: ptr::from_ref(world).cast_mut(), _marker: PhantomData }
    }

    pub(crate) fn from_mut(world: &'w mut World) -> Self {
        Self { world, _marker: PhantomData }
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

    /// Returns an [`EntityPtr`].
    pub fn entity(self, entity: Entity) -> EntityPtr<'w> {
        EntityPtr::new(self, entity)
    }

    pub(crate) unsafe fn components(&self) -> &'w Components {
        unsafe { &self.as_ref().components }
    }

    pub(crate) unsafe fn components_mut(&mut self) -> &'w mut Components {
        unsafe { &mut self.as_mut().components }
    }

    pub(crate) unsafe fn entities(&self) -> &'w Entities {
        unsafe { &self.as_ref().entities }
    }

    pub(crate) unsafe fn entities_mut(&mut self) -> &'w mut Entities {
        unsafe { &mut self.as_mut().entities }
    }
}

impl fmt::Debug for WorldPtr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.world.fmt(f)
    }
}

unsafe impl SystemInput for WorldPtr<'_> {
    type Output<'w, 's> = WorldPtr<'w>;
    type State = ();

    fn access(_access: &mut WorldAccess) {}

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        world
    }
}

unsafe impl ReadOnlySystemInput for WorldPtr<'_> {}
