use super::{Commands, EntityQueue};
use crate::access::Level;
use crate::component::Bundle;
use crate::entity::{Entities, EntityId, EntityNotFound};
use crate::prelude::{WorldAccessBuilder, WorldPtr};
use crate::system::{ReadOnlySystemInput, SystemInput};
use crate::world::World;

/// [`Commands`] with a world reference to queue commands with a world-like
/// interface.
pub struct WorldQueue<'w, 's> {
    entities: &'w Entities,
    commands: &'s mut Commands,
}

impl<'w, 's> WorldQueue<'w, 's> {
    /// Creates a new world queue.
    pub const fn new(world: &'w World, commands: &'s mut Commands) -> Self {
        Self::from_entities(&world.entities, commands)
    }

    pub(crate) const fn from_entities(
        entities: &'w Entities,
        commands: &'s mut Commands,
    ) -> Self {
        Self { entities, commands }
    }

    /// Returns an entity queue for the given entity.
    ///
    /// Returns an error if the entity doesn't exist.
    pub fn entity(
        &mut self,
        entity: EntityId,
    ) -> Result<EntityQueue<'_>, EntityNotFound> {
        if self.entities.contains(entity) {
            Ok(EntityQueue::new(entity, self.commands))
        } else {
            Err(EntityNotFound(entity))
        }
    }

    /// Queues spawning a new entity with its components.
    pub fn spawn(&mut self, bundle: impl Bundle) -> EntityQueue<'_> {
        let entity = self.entities.reserve();

        self.commands.push_fn(move |world| unsafe {
            world.spawn_at(entity, bundle);
        });

        EntityQueue::new(entity, self.commands)
    }

    /// Queues despawning the entity with the given id.
    pub fn despawn(&mut self, entity: EntityId) -> Result<(), EntityNotFound> {
        self.entity(entity).map(EntityQueue::despawn)
    }
}

/// # Safety
///
/// The world queue borrows the world immutably and only declares immutable
/// access.
unsafe impl SystemInput for WorldQueue<'_, '_> {
    type Output<'w, 's> = WorldQueue<'w, 's>;
    type State = Commands;

    fn init(_world: &World) -> Self::State {
        Commands::new()
    }

    fn world_access(
        _state: &Self::State,
        builder: &mut WorldAccessBuilder<'_>,
    ) {
        builder.borrows_world(Level::Read);
    }

    unsafe fn get<'w, 's>(
        state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        state.as_world_queue(unsafe { world.as_ref() })
    }

    fn needs_sync(state: &Self::State) -> bool {
        !state.is_empty()
    }

    fn sync(state: &mut Self::State, world: &mut World) {
        state.apply(world);
    }
}

/// # Safety
///
/// The world queue only immutably accesses the world.
unsafe impl ReadOnlySystemInput for WorldQueue<'_, '_> {}
