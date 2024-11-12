//! Rich functions that perform operations on the world.

pub use self::var::*;
use crate::access::WorldAccessBuilder;
use crate::world::{World, WorldPtr};

mod tuple_impl;
mod var;

/// A unit of work in an ECS.
///
/// # Safety
///
/// [`System::run`] must only access what it declares in
/// [`System::world_access`].
pub unsafe trait System<I: SystemInput, O = ()> {
    /// The state of this system, retained between runs.
    ///
    /// Usually just the state of the system input.
    type State: Send + Sync + 'static;

    /// Initializes the state of this system.
    fn init(&mut self, world: &World) -> Self::State;

    /// Adds the access of this system to the set.
    fn world_access(
        &mut self,
        state: &Self::State,
        builder: &mut WorldAccessBuilder<'_>,
    );

    /// Runs this system.
    ///
    /// # Safety
    ///
    /// The access of this system must have be valid. The world pointer must be
    /// valid for the described access. All required items need to be
    /// present.
    unsafe fn run(&mut self, state: &mut Self::State, world: WorldPtr<'_>)
        -> O;

    /// Returns `true` if this system has work to apply in [`System::sync`].
    #[expect(unused)]
    fn needs_sync(&self, state: &Self::State) -> bool {
        false
    }

    /// Applies any deferred work to the world.
    #[expect(unused)]
    fn sync(&mut self, state: &mut Self::State, world: &mut World) {}
}

/// Trait for valid inputs to [`System`]s.
///
/// # Safety
///
/// The access of this system set by [`SystemInput::world_access`] must be the
/// same every time and must always match how the world is accessed in
/// [`SystemInput::get`].
pub unsafe trait SystemInput {
    /// This system input borrowed for a lifetime.
    type Output<'w, 's>: SystemInput<State = Self::State>;
    /// The state of this input, retained between runs.
    type State: Send + Sync + 'static;

    /// Creates the state of this system input.
    fn init(world: &World) -> Self::State;

    /// Adds the access of this system input to the set.
    fn world_access(state: &Self::State, builder: &mut WorldAccessBuilder<'_>);

    /// Produces this system input from the world and state.
    ///
    /// # Safety
    ///
    /// The access of the system input must be valid. THe world pointer must be
    /// valid for the described access. All required items need to be present.
    unsafe fn get<'w, 's>(
        state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's>;

    /// Returns `true` if this input has work to apply in [`SystemInput::sync`].
    #[expect(unused)]
    fn needs_sync(state: &Self::State) -> bool {
        false
    }

    /// Applies any deferred work to the world.
    #[expect(unused)]
    fn sync(state: &mut Self::State, world: &mut World) {}
}

/// Trait for systems that don't need mutable access.
///
/// # Safety
///
/// The implementation must declare only read access and must never mutate the
/// world.
pub unsafe trait ReadOnlySystem<I: ReadOnlySystemInput, O>:
    System<I, O>
{
    /// Runs this read-only system from a reference to the world.
    fn run_from_ref(&mut self, state: &mut Self::State, world: &World) -> O {
        // SAFETY: the access is valid, as no combination of read-only accesses
        // can alias. the world pointer is valid for all reads.
        unsafe { self.run(state, world.as_ptr()) }
    }
}

/// Trait for system inputs that don't need mutable access.
///
/// # Safety
///
/// The implementation must declare only read access and must never mutate the
/// world.
pub unsafe trait ReadOnlySystemInput: SystemInput {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::WorldQueue;

    /// Ensures that the implementation of [`System`] for functions passes
    /// through [`SystemInput::needs_sync`] and calls [`SystemInput::sync`].
    #[test]
    fn fn_system_impl_applies_sync() {
        fn system(mut queue: WorldQueue) {
            queue.spawn(());
            queue.spawn(());
        }

        let mut world = World::new();
        let mut state = system.init(&world);

        // SAFETY: we know that `WorldQueue` has valid access and that the world
        // pointer is valid as it was created from a reference
        unsafe { system.run(&mut state, world.as_ptr()) };
        assert!(system.needs_sync(&state));

        system.sync(&mut state, &mut world);
        assert!(!system.needs_sync(&state));

        assert_eq!(world.len(), 2);
    }
}
