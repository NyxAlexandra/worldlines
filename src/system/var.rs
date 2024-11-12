use super::SystemInput;
use crate::prelude::{World, WorldAccessBuilder, WorldPtr};

/// A system-local variable that is retained between runs.
#[repr(transparent)]
pub struct Var<'s, T> {
    state: &'s mut Option<T>,
}

impl<T> Var<'_, T> {
    /// Returns a reference to the value, inserting it via a function if not
    /// already present.
    pub fn get_or_insert(&mut self, f: impl FnOnce() -> T) -> &mut T {
        self.state.get_or_insert_with(f)
    }

    /// Returns a reference to the value, inserting the default value if not
    /// already present.
    pub fn get_or_default(&mut self) -> &mut T
    where
        T: Default,
    {
        self.get_or_insert(Default::default)
    }
}

/// # Safety
///
/// `Var` declares no access and doesn't access the world.
unsafe impl<T: Send + Sync + 'static> SystemInput for Var<'_, T> {
    type Output<'w, 's> = Var<'s, T>;
    type State = Option<T>;

    fn init(_world: &World) -> Self::State {
        None
    }

    fn world_access(
        _state: &Self::State,
        _builder: &mut WorldAccessBuilder<'_>,
    ) {
    }

    unsafe fn get<'w, 's>(
        state: &'s mut Self::State,
        _world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        Var { state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::System;

    #[test]
    fn var_is_retained() {
        fn system(mut counter: Var<u32>) {
            let counter = counter.get_or_default();

            *counter += 1;
        }

        let world = World::new();
        let mut state = system.init(&world);

        // SAFETY: the system access is valid as it doesn't access anything, the
        // world pointer is valid
        unsafe { system.run(&mut state, world.as_ptr()) };

        assert_eq!(state.0, Some(1));
    }
}
