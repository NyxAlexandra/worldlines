pub use worldlines_macros::Bundle;

use super::{Component, ComponentSetBuilder, Components};
use crate::commands::EntityQueue;
use crate::entity::EntityAddr;

/// A bundle of components to add to an entity.
///
/// # Safety
///
/// The output of [`Bundle::components`] must always set the same access.
/// [`Bundle::write`] must call [`ComponentWriter::write`] on every component
/// declared in [`Bundle::write`].
pub unsafe trait Bundle: Send + 'static {
    /// Returns the type set for the components of this bundle.
    fn components(builder: &mut ComponentSetBuilder<'_>);

    /// Writes the components of this bundle to ECS storage.
    fn write(self, writer: &mut ComponentWriter<'_, '_>);
}

/// A type used by [`Bundle`] implementations to write components to ECS
/// storage.
pub struct ComponentWriter<'w, 's> {
    queue: EntityQueue<'s>,
    components: &'w mut Components,
    addr: EntityAddr,
}

unsafe impl<C: Component> Bundle for C {
    fn components(builder: &mut ComponentSetBuilder<'_>) {
        builder.insert::<Self>();
    }

    fn write(self, writer: &mut ComponentWriter<'_, '_>) {
        writer.write(self);
    }
}

impl<'w, 's> ComponentWriter<'w, 's> {
    pub(crate) fn new(
        queue: EntityQueue<'s>,
        components: &'w mut Components,
        addr: EntityAddr,
    ) -> Self {
        Self { queue, components, addr }
    }

    /// Writes a component to storage.
    ///
    /// # Panics
    ///
    /// Panics if the entity doesn't contain the component.
    pub fn write<C: Component>(&mut self, component: C) {
        let info = self.components.register::<C>();

        unsafe {
            let table = self.components.get_unchecked_mut(self.addr.table);

            table.write(self.addr.row, info.index(), component).expect(
                "attempted to write a bundle component to an entity that \
                 doesn't contain the component",
            )
        };

        self.queue.push_fn(|mut entity| C::after_insert(entity.as_mut()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Component)]
    struct Person;

    #[derive(Component)]
    struct Name(&'static str);

    #[derive(Component)]
    struct Age(u32);

    #[derive(Bundle)]
    struct PersonBundle {
        person: Person,
        name: Name,
        age: Age,
    }

    #[test]
    fn derived_bundle() {
        let mut world = World::new();
        let entity = world.spawn(PersonBundle {
            person: Person,
            name: Name("Alexandra"),
            age: Age(u32::MAX),
        });

        assert!(entity.contains::<Person>());
        assert_eq!(entity.get::<Name>().unwrap().0, "Alexandra");
        assert_eq!(entity.get::<Age>().unwrap().0, u32::MAX);
    }
}
