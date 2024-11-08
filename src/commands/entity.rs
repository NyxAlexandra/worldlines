use super::{Commands, EntityCommand};
use crate::entity::{EntityId, EntityWorld};

/// A type to queue commands to perform on entities.
pub struct EntityQueue<'s> {
    id: EntityId,
    commands: &'s mut Commands,
}

impl<'s> EntityQueue<'s> {
    /// Creates a new queue for the given entity.
    pub(crate) fn new(id: EntityId, commands: &'s mut Commands) -> Self {
        Self { id, commands }
    }

    /// Returns the id of this entity.
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// Pushes an entity command to the queue.
    pub fn push(&mut self, command: impl EntityCommand) {
        self.push_fn(move |world| command.apply(world));
    }

    /// Pushes a function command to the entity queue.
    ///
    /// Helpful as using [`EntityQueue::push`] on a closure fails type elision.
    pub fn push_fn(
        &mut self,
        f: impl FnOnce(EntityWorld<'_>) + Send + 'static,
    ) {
        let entity = self.id;

        self.commands.push_fn(move |world| {
            let Ok(entity) = EntityWorld::new(entity, world) else {
                return;
            };

            f(entity);
        })
    }

    /// Queues a command to despawn this entity.
    pub fn despawn(self) {
        self.commands.push_fn(move |world| {
            _ = world.despawn(self.id);
        });
    }
}
