use std::marker::PhantomData;
use std::ptr::NonNull;

use super::{EntityId, EntityMut, EntityNotFound, EntityRef};
use crate::component::{Component, ComponentNotFound};
use crate::prelude::{ComponentInfo, ComponentVTable};
use crate::world::World;

/// A borrow of an entity and the world it resides in.
///
/// Allows structural changes (insertion, removal, and despawning).
pub struct EntityWorld<'w> {
    id: EntityId,
    world: NonNull<World>,
    _lt: PhantomData<&'w World>,
}

impl<'w> EntityWorld<'w> {
    /// Creates a new entity world.
    ///
    /// Returns an error if the entity doesn't exist in the world.
    pub fn new(
        id: EntityId,
        world: &'w mut World,
    ) -> Result<Self, EntityNotFound> {
        if world.contains(id) {
            Ok(unsafe { Self::new_unchecked(id, world) })
        } else {
            Err(EntityNotFound(id))
        }
    }

    /// Creates a new entity world without verifying that the entity exists.
    ///
    /// # Safety
    ///
    /// The entity must be alive in the world.
    pub unsafe fn new_unchecked(id: EntityId, world: &'w mut World) -> Self {
        let world = NonNull::from(world);

        Self { id, world, _lt: PhantomData }
    }

    /// Returns the id of this entity.
    pub const fn id(&self) -> EntityId {
        self.id
    }

    pub(crate) fn world(&self) -> &'w World {
        // SAFETY: this pointer is equivalent to a mutable world reference
        unsafe { self.world.as_ref() }
    }

    pub(crate) fn world_mut(&mut self) -> &'w mut World {
        // SAFETY: this pointer is equivalent to a mutable world reference
        unsafe { self.world.as_mut() }
    }

    /// Borrows this entity as an [`EntityRef`].
    pub fn as_ref(&self) -> EntityRef<'w> {
        unsafe { EntityRef::new_unchecked(self.id, self.world()) }
    }

    /// Borrows this entity as an [`EntityMut`].
    pub fn as_mut(&mut self) -> EntityMut<'w> {
        // SAFETY: the existence of this reference ensures that that entity is
        // alive
        unsafe { EntityMut::new_unchecked(self.id, self.world_mut()) }
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
        self.as_mut().get_mut()
    }

    /// Inserts a component into this entity.
    ///
    /// Returns the previous value if there was one.
    pub fn insert<C: Component>(&mut self, component: C) -> Option<C> {
        let world = self.world_mut();
        let info = ComponentInfo::of::<C>();
        let id = info.id();

        let old_addr =
            unsafe { world.entities.get(self.id).unwrap_unchecked() };

        // SAFETY: this entity is alive, so the address is valid
        if unsafe {
            world
                .components
                .get_unchecked(old_addr.table)
                .components()
                .contains(id)
        } {
            // replace

            unsafe {
                let old_table =
                    world.components.get_unchecked_mut(old_addr.table);

                Some(old_table.replace(old_addr.row, id, component))
            }
        } else {
            // insert new

            unsafe {
                let new_components = world
                    .components
                    .get_unchecked(old_addr.table)
                    .components()
                    .clone()
                    .and_insert(info);
                let new_addr =
                    world.components.realloc(self.id, old_addr, new_components);

                world.entities.set(self.id, new_addr);
                world.components.get_unchecked_mut(new_addr.table).write(
                    new_addr.row,
                    id,
                    component,
                );
            }

            C::after_insert(self.as_mut());

            None
        }
    }

    /// Removes a component from this entity.
    ///
    /// Returns an error if this entity doesn't contain the component.
    pub fn remove<C: Component>(&mut self) -> Result<C, ComponentNotFound> {
        if self.contains::<C>() {
            C::before_remove(self.as_mut());

            let world = self.world_mut();
            let info = ComponentInfo::of::<C>();
            let id = info.id();

            let old_addr =
            // SAFETY: this entity exists
                unsafe { world.entities.get(self.id).unwrap_unchecked() };
            let (prev, new_components) = {
                let old_table =
                    unsafe { world.components.get_unchecked(old_addr.table) };
                // SAFETY: the component exists because of the above
                // `.contains::<C>()`
                let prev = unsafe {
                    old_table
                        .get_unchecked(old_addr.row, id)
                        .as_ptr()
                        .cast::<C>()
                        .read()
                };
                let new_components =
                    old_table.components().clone().and_remove(id);

                (prev, new_components)
            };
            // SAFETY: this entity exists in the table at `old_addr`
            let new_addr = unsafe {
                world.components.realloc(self.id, old_addr, new_components)
            };

            world.entities.set(self.id, new_addr);

            Ok(prev)
        } else {
            Err(ComponentNotFound::new::<C>(self.id))
        }
    }

    /// Despawns this entity.
    pub fn despawn(mut self) {
        let world = self.world_mut();
        let (addr, components) = unsafe {
            // SAFETY: for this `EntityWorld` to exist, it must be a valid
            // entity in the world
            let addr = world.entities.get(self.id).unwrap_unchecked();
            // SAFETY: the entity address exists, so it must refer to a valid
            // table
            let table = world.components.get_unchecked_mut(addr.table);

            (addr, table.components().clone())
        };

        for component in &components {
            let hook = component.before_remove();

            hook(self.as_mut());
        }

        // SAFETY: same as above, the address is valid
        let table = unsafe { world.components.get_unchecked_mut(addr.table) };

        _ = world.entities.free(self.id);
        // SAFETY: same as above, the entity exists
        unsafe { table.free(addr.row) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct A(u32);

    #[derive(Component)]
    struct B(u64);

    #[test]
    fn insert() {
        let mut world = World::new();
        let mut entity = world.spawn(A(123));

        entity.insert(B(321));

        assert_eq!(entity.get::<A>().unwrap().0, 123);
        assert_eq!(entity.get::<B>().unwrap().0, 321);
    }

    #[test]
    fn remove() {
        let mut world = World::new();
        let mut entity = world.spawn((A(123), B(321)));

        entity.remove::<B>().unwrap();

        assert_eq!(entity.get::<A>().unwrap().0, 123);
        assert!(entity.get::<B>().is_err());
    }
}
