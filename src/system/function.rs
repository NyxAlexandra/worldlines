use std::marker::PhantomData;

use super::{IntoSystem, System, SystemInput};

/// A type that wraps functions to implement [`System`].
pub struct FunctionSystem<I, O, F>
where
    I: SystemInput,
{
    pub(super) function: F,
    pub(super) _marker: PhantomData<fn(I) -> O>,
}

impl<I: SystemInput, O, F> FunctionSystem<I, O, F> {
    /// Creates a new function system.
    pub fn new(function: F) -> Self {
        Self { function, _marker: PhantomData }
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
