use std::borrow::Cow;
use std::marker::PhantomData;

use crate::{MapSystem, NamedSystem, System, SystemInput, World, WorldPtr};

/// Extension methods for [`System`]. Implemented for all systems.
pub trait SystemExt<I, O = ()>
where
    Self: System<I, O>,
    I: SystemInput,
{
    /// Construct the input of this system to run it once.
    ///
    /// # Safety
    ///
    /// - The system access must be valid.
    /// - [`System::init`] must have already been called.
    unsafe fn run_once(&mut self, world: WorldPtr<'_>) -> O {
        let mut state = unsafe { I::init(world.as_ref()) };
        let input = unsafe { I::get(world, &mut state) };

        unsafe { self.run(input) }
    }

    /// Run this system using a [`WorldPtr`] and input state.
    ///
    /// # Safety
    ///
    /// - The system access must be valid.
    /// - [`System::init`] must have already been called.
    unsafe fn run_from(
        &mut self,
        world: WorldPtr<'_>,
        state: &mut I::State,
    ) -> O {
        unsafe {
            let input = I::get(world, state);

            self.run(input)
        }
    }

    /// Run this system from an [`&mut World`](World) and input state.
    ///
    /// # Safety
    ///
    /// - The system access must be valid.
    /// - [`System::init`] must have already been called.
    unsafe fn run_from_mut(
        &mut self,
        world: &mut World,
        state: &mut I::State,
    ) -> O {
        unsafe {
            let input = I::get(world.as_ptr_mut(), state);

            self.run(input)
        }
    }

    /// Run this system from an [`&mut World`](World) and input state once.
    ///
    /// # Safety
    ///
    /// - The system access must be valid.
    /// - [`System::init`] must have already been called.
    unsafe fn run_from_mut_once(&mut self, world: &mut World) -> O {
        let mut state = I::init(world);

        unsafe { self.run_from_mut(world, &mut state) }
    }

    /// Return a system with a new name.
    fn with_name(self, name: impl Into<Cow<'static, str>>) -> NamedSystem<Self>
    where
        Self: Sized,
    {
        NamedSystem { system: self, name: name.into() }
    }

    /// Map the output of this system.
    fn map<F, O_>(self, f: F) -> MapSystem<Self, F, O>
    where
        Self: Sized,
        F: FnMut(O) -> O_,
    {
        MapSystem { system: self, f, _marker: PhantomData }
    }

    /// initialize the state of this system's input.
    fn init_state(&self, world: &World) -> I::State {
        I::init(world)
    }

    /// Get the input of this system.
    ///
    /// # Safety
    ///
    /// See [`SystemInput::get`].
    unsafe fn get_input<'w, 's>(
        &self,
        world: WorldPtr<'w>,
        state: &'s mut I::State,
    ) -> I::Output<'w, 's> {
        unsafe { I::get(world, state) }
    }

    /// Return whether [`SystemInput::apply`] should be called.
    ///
    /// Defaults to `false`.
    fn should_apply(&self, state: &I::State) -> bool {
        I::should_apply(state)
    }

    /// Work to be performed after a system is ran.
    ///
    /// See also [`SystemInput::should_apply`].
    fn apply(&self, world: &mut World, state: &mut I::State) {
        I::apply(world, state);
    }
}

impl<S: ?Sized, I, O> SystemExt<I, O> for S
where
    S: System<I, O>,
    I: SystemInput,
{
}
