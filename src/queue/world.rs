use crate::{
    Bundle,
    CommandQueue,
    Entities,
    Entity,
    EntityNotFound,
    EntityQueue,
    ReadOnlySystemInput,
    Resource,
    SystemInput,
    World,
    WorldAccess,
    WorldPtr,
};

/// A queue of commands to be performed on a [`World`]. Mimics the API of
/// [`World`].
pub struct WorldQueue<'w, 's> {
    pub(super) entities: &'w Entities,
    pub(super) queue: &'s mut CommandQueue,
}

impl<'w, 's> WorldQueue<'w, 's> {
    /// Returns an [`EntityQueue`] for an entity.
    pub fn entity(&mut self, entity: Entity) -> Result<EntityQueue<'_>, EntityNotFound> {
        self.entities
            .contains(entity)
            .then_some(EntityQueue { entity, queue: self.queue })
            .ok_or(EntityNotFound(entity))
    }

    /// Queue the spawn of a new entity.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityQueue<'_> {
        let entity = self.entities.reserve();

        self.queue.push_fn(move |world| {
            let table = unsafe { world.components.alloc(entity, bundle) };

            world.entities.set(entity, table);
        });

        EntityQueue { entity, queue: self.queue }
    }

    /// Queue despawning an entity.
    pub fn despawn(&mut self, entity: Entity) {
        self.queue.push_fn(move |world| {
            _ = world.despawn(entity);
        });
    }

    /// Queue despawning all entities.
    pub fn despawn_all(&mut self) {
        self.queue.push_fn(World::despawn_all);
    }

    /// Queue destroying a resource.
    pub fn destroy<R: Resource>(&mut self) {
        self.queue.push_fn(|world| {
            _ = world.destroy::<R>();
        });
    }

    /// Queue destroying all resources.
    pub fn destroy_all(&mut self) {
        self.queue.push_fn(World::destroy_all);
    }
}

unsafe impl SystemInput for WorldQueue<'_, '_> {
    type Output<'w, 's> = WorldQueue<'w, 's>;
    type State = CommandQueue;

    fn access(access: &mut WorldAccess) {
        // TODO?: add access for [`Entities`]
        access.world();
    }

    fn init(_world: &World) -> Self::State {
        CommandQueue::new()
    }

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { state.as_world_queue(world.as_ref()) }
    }

    fn should_apply(state: &Self::State) -> bool {
        !state.inner.is_empty()
    }

    fn apply(world: &mut World, state: &mut Self::State) {
        state.apply(world);
    }
}

// SAFETY: only mutates the world via [`SystemInput::apply`]
unsafe impl ReadOnlySystemInput for WorldQueue<'_, '_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemExt;

    #[test]
    fn run_queue_system() {
        fn system(mut queue: WorldQueue) {
            queue.spawn(());
        }

        let mut world = World::new();
        let mut state = system.init_state(&world);

        unsafe { system.run_from(world.as_ptr(), &mut state) };

        if system.should_apply(&state) {
            system.apply(&mut world, &mut state);
        }

        assert_eq!(world.len(), 1);
    }
}
