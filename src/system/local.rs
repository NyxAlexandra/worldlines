use crate::{ReadOnlySystemInput, SystemInput, World, WorldAccess, WorldPtr};

// TODO: allow `!Send + !Sync`

/// A local variable for a [`System`](crate::System).
///
/// The value of a [`Local`] are retained across runs (assuming that the state
/// has been preserved).
///
/// ```
/// # use archetypal_ecs::Local;
/// #
/// fn system(mut runs: Local<u32>) {
///     let runs = runs.get_or_default();
///
///     *runs += 1;
/// }
/// ```
#[repr(transparent)]
pub struct Local<'s, T: Send + Sync + 'static> {
    state: &'s mut Option<T>,
}

impl<T: Send + Sync + 'static> Local<'_, T> {
    /// Get the value of the local, initalizing it if not already.
    pub fn get_or_init(&mut self, f: impl FnOnce() -> T) -> &mut T {
        self.state.get_or_insert_with(f)
    }

    /// Get the value of the local, possibly initializing it to the deafult.
    pub fn get_or_default(&mut self) -> &mut T
    where
        T: Default,
    {
        self.get_or_init(Default::default)
    }
}

unsafe impl<T: Send + Sync + 'static> SystemInput for Local<'_, T> {
    type Output<'w, 's> = Local<'s, T>;
    type State = Option<T>;

    fn access(_access: &mut WorldAccess) {
        // only accesses itself
    }

    fn init(_world: &World) -> Self::State {
        None
    }

    unsafe fn get<'w, 's>(
        _world: WorldPtr<'w>,
        state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        Local { state }
    }
}

unsafe impl<T: Send + Sync + 'static> ReadOnlySystemInput for Local<'_, T> {}

#[cfg(test)]
mod tests {
    use crate::{Local, ReadOnlySystem, System, SystemExt, World};

    #[test]
    fn local_state_retained() {
        fn system(mut counter: Local<u32>) {
            let counter = counter.get_or_default();

            *counter += 1;
        }

        let world = World::new();

        let mut state = system.init_state(&world);

        assert!(state.0.is_none());

        system.init(&world);
        unsafe { system.run_from_ref(&world, &mut state) };

        assert_eq!(state.0, Some(1));
    }
}
