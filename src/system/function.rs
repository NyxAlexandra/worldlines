use std::marker::PhantomData;

use super::SystemInput;
use crate::access::WorldAccess;

/// A type that wraps functions to implement [`System`].
pub struct FunctionSystem<I, O, F>
where
    I: SystemInput,
{
    pub(super) function: F,
    pub(super) state: Option<I::State>,
    pub(super) access: Option<WorldAccess>,
    pub(super) _output: PhantomData<fn() -> O>,
}

impl<I: SystemInput, O, F> FunctionSystem<I, O, F> {
    /// Creates a new function system.
    pub fn new(function: F) -> Self {
        let state = None;
        let access = None;

        Self { function, state, access, _output: PhantomData }
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
