use std::borrow::Cow;
use std::fmt;

use crate::{System, SystemExt, SystemInput, World, WorldAccess, WorldPtr};

/// A [`System`] that erases its [`SystemInput`].
#[repr(transparent)]
pub struct AnySystem<O = ()> {
    inner: Box<dyn System<(WorldPtr<'static>,), O>>,
}

impl<O> AnySystem<O> {
    /// Creates an [`AnySystem`] from a [`System`].
    pub fn new<I: SystemInput + 'static>(system: impl System<I, O> + 'static) -> Self {
        struct Inner<S, I: SystemInput> {
            system: S,
            state: Option<I::State>,
        }

        unsafe impl<S, I, O> System<(WorldPtr<'_>,), O> for Inner<S, I>
        where
            S: System<I, O>,
            I: SystemInput,
        {
            unsafe fn run(
                &mut self,
                (world,): <(WorldPtr<'_>,) as SystemInput>::Output<'_, '_>,
            ) -> O {
                unsafe {
                    let state = self.state.as_mut().unwrap_unchecked();

                    self.system.run_from(world, state)
                }
            }

            fn init(&mut self, world: &World) {
                if self.state.is_none() {
                    self.state = Some(self.system.init_state(world));
                }
            }

            fn access(&self, access: &mut WorldAccess) {
                self.system.access(access);
            }

            fn name(&self) -> Cow<'static, str> {
                self.system.name()
            }

            fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.system.debug(f)
            }
        }

        Self { inner: Box::new(Inner { system, state: None }) }
    }

    /// Returns `true` if the system has pending deferred operations.
    pub fn should_apply(&self) -> bool {
        self.inner.should_apply(&((),))
    }

    /// Initializes the system state, if not already.
    pub fn init(&mut self, world: &World) {
        self.inner.init(world);
    }

    /// Updates a [`WorldAccess`] with the access of this system.
    pub fn access(&self, access: &mut WorldAccess) {
        self.inner.access(access);
    }

    /// Run this system.
    ///
    /// This will initialize system state (if necessary).
    ///
    /// # Safety
    ///
    /// - The system must have been initialized.
    /// - The system's access must be valid.
    /// - The pointer must be valid for the described access.
    pub unsafe fn run(&mut self, world: WorldPtr<'_>) -> O {
        self.inner.init(unsafe { world.as_ref() });

        unsafe { self.inner.run_once(world) }
    }

    /// Run this system.
    ///
    /// This will initialize system state (if necessary).
    ///
    /// # Safety
    ///
    /// - The system's access must be valid.
    /// - The inner system must be a [`ReadOnlySystem`](crate::ReadOnlySystem).
    pub unsafe fn run_from_ref(&mut self, world: &World) -> O {
        self.inner.init(world);

        unsafe { self.run(world.as_ptr()) }
    }

    /// Run this system.
    ///
    /// # Safety
    ///
    /// - The system's access must be valid.
    /// - The inner system must be a [`ReadOnlySystem`](crate::ReadOnlySystem).
    pub unsafe fn run_from_mut(&mut self, world: &mut World) -> O {
        self.inner.init(world);

        unsafe { self.run(world.as_ptr_mut()) }
    }

    /// Apply pending operations to the world.
    pub fn apply(&mut self, world: &mut World) {
        self.inner.apply(world, &mut ((),))
    }

    /// Apply pending operations to the world if [`SystemInput::should_apply`].
    pub fn try_apply(&mut self, world: &mut World) -> Option<()> {
        self.should_apply().then(|| self.apply(world))
    }
}

unsafe impl<O> System<(WorldPtr<'_>,), O> for AnySystem<O> {
    unsafe fn run(
        &mut self,
        input: <(WorldPtr<'_>,) as SystemInput>::Output<'_, '_>,
    ) -> O {
        unsafe { self.inner.run(input) }
    }

    fn init(&mut self, world: &World) {
        self.inner.init(world)
    }

    fn access(&self, access: &mut WorldAccess) {
        self.inner.access(access);
    }

    fn name(&self) -> Cow<'static, str> {
        self.inner.name()
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.debug(f)
    }
}

impl<O> fmt::Debug for AnySystem<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entity, Local, Query, WorldQueue};

    #[test]
    fn new_any_system() {
        fn system(_: &World, _: WorldQueue, _: Query<Entity>) {}

        let _any = AnySystem::new(system);
    }

    #[test]
    fn state_is_retained() {
        fn system(mut counter: Local<u32>) -> u32 {
            let counter = counter.get_or_default();

            *counter += 1;

            *counter
        }

        let world = World::new();
        let mut any = AnySystem::new(system);

        let result = unsafe {
            any.run_from_ref(&world);
            any.run_from_ref(&world)
        };

        assert_eq!(result, 2);
    }
}
