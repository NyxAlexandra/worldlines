use crate::{CommandQueue, Component, Entity};

/// A queue of [`Command`](super::Command)s to be performed on an entity.
pub struct EntityQueue<'s> {
    pub(super) entity: Entity,
    pub(super) queue: &'s mut CommandQueue,
}

impl EntityQueue<'_> {
    /// The ID of this entity.
    pub const fn id(&self) -> Entity {
        self.entity
    }

    /// Queue inserting a component into this entity.
    pub fn insert<C: Component>(&mut self, component: C) {
        let entity = self.entity;

        self.queue.push_fn(move |world| {
            let Ok(mut entity) = world.entity_world(entity) else {
                return;
            };

            entity.insert(component);
        });
    }

    /// Queue inserting a component into this entity and return `self`.
    pub fn and_insert<C: Component>(&mut self, component: C) -> &mut Self {
        self.insert(component);

        self
    }

    /// Queue removing a component from the entity.
    pub fn remove<C: Component>(&mut self) {
        let entity = self.entity;

        self.queue.push_fn(move |world| {
            let Ok(mut entity) = world.entity_world(entity) else {
                return;
            };

            _ = entity.remove::<C>();
        });
    }

    /// Queue removing a component from this entity and return `self`.
    pub fn and_remove<C: Component>(&mut self) -> &mut Self {
        self.remove::<C>();

        self
    }

    /// Queue despawning this entity.
    pub fn despawn(self) {
        let Self { entity, queue } = self;

        queue.push_fn(move |world| unsafe {
            world.despawn(entity).unwrap_unchecked();
        });
    }
}
