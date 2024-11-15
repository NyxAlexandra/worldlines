use std::marker::PhantomData;

use super::{IntoSystem, System, SystemInput};

/// A type that wraps functions to implement [`System`].
pub struct FunctionSystem<I, O, F>
where
    I: SystemInput,
{
    pub(super) function: F,
    pub(super) state: Option<I::State>,
    pub(super) _output: PhantomData<fn() -> O>,
}

impl<I: SystemInput, O, F> FunctionSystem<I, O, F> {
    /// Creates a new function system.
    pub fn new(function: F) -> Self {
        let state = None;

        Self { function, state, _output: PhantomData }
    }

    /// Returns a reference to the state of this system.
    pub fn state(&self) -> Option<&I::State> {
        self.state.as_ref()
    }

    /// Returns a mutable reference to the state of this system.
    pub fn state_mut(&mut self) -> Option<&mut I::State> {
        self.state.as_mut()
    }
}

// impls of `System` are in `tuple_impl.rs`

impl<I, O, F> IntoSystem<I, O> for FunctionSystem<I, O, F>
where
    Self: System<Input = I, Output = O>,
    I: SystemInput,
{
    type Output = Self;

    fn into_system(self) -> Self::Output {
        self
    }
}
