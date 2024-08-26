use crate::{System, SystemInput};

/// A [`System`] that has no state.
///
/// Automatically implemented for all systems whose [`SystemInput`] implements
/// [`StatelessSystemInput`].
pub trait StatelessSystem<I, O = ()>
where
    Self: System<I, O>,
    I: StatelessSystemInput,
{
}

impl<S, I, O> StatelessSystem<I, O> for S
where
    S: System<I, O>,
    I: StatelessSystemInput,
{
}

/// A [`SystemInput`] without any state.
///
/// Automatically implemented for any `SystemInput<Output = ()>`.
pub trait StatelessSystemInput: SystemInput {
    /// Returns the "state" of a stateless system.
    fn init() -> Self::State;
}

impl<I: SystemInput<State = ()>> StatelessSystemInput for I {
    fn init() -> Self::State {}
}
