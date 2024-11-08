//! Defines [`Component`].

use std::any::type_name;

use thiserror::Error;
pub use worldlines_macros::Component;

pub use self::bundle::*;
pub(crate) use self::info::*;
pub use self::set::*;
pub(crate) use self::storage::*;
use crate::entity::{EntityId, EntityMut};

mod bundle;
mod info;
mod set;
mod storage;
mod tuple_impl;

// TODO: docs for deriving hooks

/// Trait for components, the data stored in an entity.
///
/// # Deriving
///
/// `Component` can be derived. Note that this does not place any requirements
/// on input generics (unlike something like [`Clone`]), nor does it delegate
/// component hooks of fields components.
///
/// The derive macro accepts the attribute `#[component(...)]`. It can be used
/// to specify [`Component::after_insert`] and [`Component::before_remove`] with
/// `#[component(after_insert = after_insert_fn, before_remove =
/// before_remove_fn)]`.
pub trait Component: Send + Sync + 'static {
    /// Called after this component is added to an entity that does not already
    /// contain it, including when spawned.
    #[expect(unused)]
    fn after_insert(entity: EntityMut<'_>) {}

    /// Called before this component is removed from and entity, including
    /// despawn.
    #[expect(unused)]
    fn before_remove(entity: EntityMut<'_>) {}
}

/// Error when accessing a [`Component`] an entity does not contain.
#[derive(Debug, Clone, Copy, Error)]
#[error("component {component} not found for entity {entity:?}")]
pub struct ComponentNotFound {
    entity: EntityId,
    component: &'static str,
}

impl ComponentNotFound {
    pub(crate) fn new<C: Component>(entity: EntityId) -> Self {
        let component = type_name::<C>();

        Self { entity, component }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    #[derive(Component)]
    #[component(after_insert = panic!("boom!"))]
    struct Bomb;

    #[derive(Component)]
    #[component(before_remove = panic!("boom!"))]
    struct DeadManSwitch;

    #[test]
    #[should_panic]
    fn derived_on_insert_works() {
        let mut world = World::new();

        world.spawn(Bomb);
    }

    #[test]
    #[should_panic]
    fn derived_on_remove_works() {
        let mut world = World::new();

        world.spawn(DeadManSwitch).despawn();
    }
}
