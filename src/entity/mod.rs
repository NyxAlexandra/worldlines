use std::fmt;

use thiserror::Error;

pub(crate) use self::allocator::*;
pub use self::ptr::*;
pub use self::world::*;
use crate::{
    Component,
    QueryData,
    ReadOnlyQueryData,
    SparseIndex,
    TypeData,
    World,
    WorldAccess,
    WorldPtr,
};

mod allocator;
mod ptr;
mod world;

/// A identifier for an entity in a [`World`].
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) version: u32,
}

/// An iterator yielding an [`EntityRef`] for entities in a [`World`].
#[derive(Clone)]
pub struct EntitiesIter<'w> {
    pub(crate) world: &'w World,
    pub(crate) ids: EntityIterIds<'w>,
}

/// An iterator yielding an [`EntityMut`] for entities in a [`World`].
pub struct EntitiesIterMut<'w> {
    pub(crate) world: WorldPtr<'w>,
    pub(crate) ids: EntityIterIds<'w>,
}

/// Error when trying to access an [`Entity`] that cannot be found.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Error)]
#[error("entity {0:?} not found")]
pub struct EntityNotFound(pub(crate) Entity);

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

impl SparseIndex for Entity {
    fn sparse_index(&self) -> usize {
        self.index as _
    }
}

unsafe impl QueryData for Entity {
    type Output<'w> = Self;

    fn access(_access: &mut WorldAccess) {}

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        Some(entity.id())
    }
}

unsafe impl ReadOnlyQueryData for Entity {}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.index, self.version)
    }
}

impl<'w> Iterator for EntitiesIter<'w> {
    type Item = EntityRef<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|entity| unsafe {
            self.world.entity(entity).unwrap_unchecked()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for EntitiesIter<'_> {
    fn len(&self) -> usize {
        self.ids.len()
    }
}

impl<'w> Iterator for EntitiesIterMut<'w> {
    type Item = EntityMut<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|entity| unsafe {
            self.world.as_mut().entity_mut(entity).unwrap_unchecked()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for EntitiesIterMut<'_> {
    fn len(&self) -> usize {
        self.ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn len() {
        #[derive(Component)]
        struct A;

        #[derive(Component)]
        struct B;

        let mut world = World::new();

        let e0 = world.spawn((A,)).id();
        let e1 = world.spawn((A, B)).id();

        assert_eq!(world.entity(e0).unwrap().len(), 1);
        assert_eq!(world.entity(e1).unwrap().len(), 2);
    }

    #[test]
    fn insert() {
        #[derive(Component)]
        struct Name(&'static str);

        #[derive(Component)]
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
        #[derive(Component)]
        struct Name(&'static str);

        #[derive(Component)]
        struct Age(#[allow(dead_code)] u32);

        let mut world = World::new();
        let mut entity = world.spawn((Name("Sasha"), Age(u32::MAX)));

        assert!(entity.remove::<Age>().is_ok());
        assert_eq!(entity.get::<Name>().unwrap().0, "Sasha");
        assert!(entity.get::<Age>().is_err());
    }

    #[test]
    fn component_not_found() {
        #[derive(Component)]
        struct A;

        #[derive(Component)]
        struct B;

        let mut world = World::new();
        let entity = world.spawn((A,));

        assert_eq!(
            entity.get::<B>().err(),
            Some(ComponentNotFound::new::<B>(entity.id())),
        );
    }

    #[test]
    fn on_insert() {
        struct A;

        #[derive(Component)]
        struct B(u32);

        impl Component for A {
            fn on_insert(mut entity: EntityMut<'_>) {
                entity.get_mut::<B>().unwrap().0 += 1;
            }
        }

        let mut world = World::new();
        let mut entity = world.spawn(());

        entity.insert(B(0));
        entity.insert(A);

        assert_eq!(entity.get::<B>().unwrap().0, 1);
    }

    #[test]
    fn on_remove() {
        struct A;

        #[derive(Component)]
        struct B(u32);

        impl Component for A {
            fn on_remove(mut entity: EntityMut<'_>) {
                entity.get_mut::<B>().unwrap().0 -= 1;
            }
        }

        let mut world = World::new();
        let mut entity = world.spawn((A, B(1)));

        entity.remove::<A>().unwrap();

        assert_eq!(entity.get::<B>().unwrap().0, 0);
    }
}
