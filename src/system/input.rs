// TODO: fallible system input construction

use crate::{ReadOnlySystemInput, World, WorldAccess, WorldPtr};

/// Trait for valid inputs to [`System`](crate::System)s.
pub unsafe trait SystemInput {
    /// The output of this system, returned by [`SystemInput::get`].
    ///
    /// This is to allow the lifetime to be constrained by the get function.
    type Output<'w, 's>: SystemInput<State = Self::State>;

    /// Retained state for this input.
    type State: Send + Sync + 'static;

    /// initialize the state of this input.
    fn init(world: &World) -> Self::State;

    /// Update the component access of this system.
    ///
    /// This is used to check that systems do not violate aliasing rules.
    fn access(access: &mut WorldAccess);

    /// Get this input.
    ///
    /// # Safety
    ///
    /// The this must not be called if the access is invalid (this allows
    /// implementors to rely on their access being valid). The pointer must
    /// be valid for the access specified in [`SystemInput::access`].
    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        state: &'s mut Self::State,
    ) -> Self::Output<'w, 's>;

    /// Return whether [`SystemInput::apply`] should be called.
    ///
    /// Defaults to `false`.
    fn should_apply(state: &Self::State) -> bool {
        _ = state;

        false
    }

    /// Work to be performed after a system is ran.
    ///
    /// See also [`SystemInput::should_apply`].
    fn apply(world: &mut World, state: &mut Self::State) {
        _ = (world, state);
    }
}

unsafe impl SystemInput for () {
    type Output<'w, 's> = ();
    type State = ();

    fn access(_access: &mut WorldAccess) {}

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        _world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
    }

    fn apply(_world: &mut World, _state: &mut Self::State) {}
}

unsafe impl ReadOnlySystemInput for () {}
