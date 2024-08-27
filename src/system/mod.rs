use std::borrow::Cow;
use std::fmt;

pub use self::any::*;
pub use self::ext::*;
pub use self::input::*;
pub use self::local::*;
pub use self::map::*;
pub use self::named::*;
pub use self::node::*;
pub use self::stateless::*;
use crate::{World, WorldAccess, WorldPtr};

mod any;
mod ext;
mod input;
mod local;
mod map;
mod named;
mod node;
mod stateless;

// TODO: make `I: SystemInput + Tuple`
//
// gives better errors for users that mistakenly think that `fn(I0): System<I0>`
// when it in actuality functions are `fn(I0): System<(I0,)>`

/// A unit of work in an ECS.
///
/// # Function Systems
///
/// All types `F: FnMut(I0, .., In) -> O` implement `System<(I0, .., In), O>`.
/// Note that a function that takes a single [`SystemInput`] implements
/// `System<(I0,), O>` and not `System<I0, O>`.
///
/// Compiles:
///
/// ```
/// # use archetypal_ecs::{World, assert_system};
/// #
/// fn system(_world: &World) {}
///
/// assert_system::<(&World,), _>(&system);
/// ```
///
/// Does not compile:
///
/// ```compile_fail
/// # use archetypal_ecs::{World, assert_system};
/// #
/// fn system(_world: &World) {}
///
/// assert_system::<&World, _>(&system);
/// ```
///
/// # Safety
///
/// [`System::access`] must follow the safety requirements for
/// [`SystemInput::access`]. This is only important if you're overriding it
/// manually.
pub unsafe trait System<I: SystemInput, O = ()> {
    /// Run this system.
    ///
    /// # Safety
    ///
    /// - The access of this system is valid.
    /// - [`System::init`] must be called first.
    unsafe fn run(&mut self, input: I::Output<'_, '_>) -> O;

    /// Initialize this system.
    fn init(&mut self, world: &World) {
        _ = world;
    }

    /// What this system accesses.
    ///
    /// See [`SystemInput::access`].
    fn access(&self, access: &mut WorldAccess) {
        I::access(access);
    }

    /// An identifier for this system.
    fn name(&self) -> Cow<'static, str> {
        Cow::from(std::any::type_name_of_val(self))
    }

    /// Debug format this system.
    ///
    /// Defaults to [`System::name`].
    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.name(), f)
    }
}

/// A [`System`] that can be ran from an immutable reference.
///
/// # Safety
///
/// [`System::access`] must match how the world is accessed.
pub unsafe trait ReadOnlySystem<I, O = ()>
where
    Self: System<I, O>,
    I: ReadOnlySystemInput,
{
    /// Run this system from an [`&World`](World) and input state.
    ///
    /// # Safety
    ///
    /// - The system access must be valid.
    /// - [`System::init`] must have already been called.
    unsafe fn run_from_ref(&mut self, world: &World, state: &mut I::State) -> O {
        let input = unsafe { I::get(world.as_ptr(), state) };

        unsafe { self.run(input) }
    }

    /// Run this system from an [`&World`](World) and input state once.
    ///
    /// # Safety
    ///
    /// - The system access must be valid.
    /// - [`System::init`] must have already been called.
    unsafe fn run_from_ref_once(&mut self, world: &World) -> O {
        let mut state = I::init(world);

        unsafe { self.run_from_ref(world, &mut state) }
    }
}

/// Trait for [`SystemInput`]s that can be constructed from an immutable
/// reference.
///
/// # Safety
///
/// The system must only set immutable acceses in [`SystemInput::access`]
/// and not access the world mutably in [`SystemInput::get`].
pub unsafe trait ReadOnlySystemInput: SystemInput {}

/// Function to assert that a variable implements [`System`].
pub const fn assert_system<I: SystemInput, O>(_: &impl System<I, O>) {}

/// Function to assert that a variable implements [`ReadOnlySystem`].
pub const fn assert_read_only_system<I: ReadOnlySystemInput, O>(
    _: &impl ReadOnlySystem<I, O>,
) {
}

unsafe impl<F, O> System<(), O> for F
where
    F: FnMut() -> O,
{
    unsafe fn run(&mut self, _input: ()) -> O {
        self()
    }
}

impl<I, O> fmt::Debug for dyn System<I, O>
where
    I: SystemInput + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

unsafe impl<F, O> ReadOnlySystem<(), O> for F where F: FnMut() -> O {}

impl<I, O> fmt::Debug for dyn ReadOnlySystem<I, O>
where
    I: ReadOnlySystemInput + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

macro_rules! impl_system {
    ($($t:ident),*) => {
        impl_system!([] [$($t)*]);
    };

    ([] []) => {};

    ([$($t:ident)*] []) => {
        unsafe impl<F, $($t,)* O> System<($($t,)*), O> for F
        where
            F: FnMut($($t,)*) -> O,
            F: for<'w, 's> FnMut($($t::Output<'w, 's>,)*) -> O,
            $($t: SystemInput,)*
        {
            #[allow(unused_variables, non_snake_case, clippy::needless_lifetimes)]
            unsafe fn run<'w, 's>(&mut self, input: ($($t::Output<'w, 's>,)*)) -> O {
                let ($($t,)*) = input;

                self($($t,)*)
            }
        }

        unsafe impl<F, $($t,)* O> ReadOnlySystem<($($t,)*), O> for F
        where
            F: FnMut($($t,)*) -> O,
            F: for<'w, 's> FnMut($($t::Output<'w, 's>,)*) -> O,
            $($t: ReadOnlySystemInput,)*
        {
        }

        unsafe impl<$($t),*> SystemInput for ($($t,)*)
        where
            $($t: SystemInput,)*
        {
            type Output<'w, 's> = ($($t::Output<'w, 's>,)*);
            type State = ($($t::State,)*);

            #[allow(unused_variables)]
            fn access(access: &mut WorldAccess) {
                $(
                    $t::access(access);
                )*
            }

            #[allow(unused_variables, clippy::unused_unit)]
            fn init(world: &World) -> Self::State {
                ($($t::init(world),)*)
            }

            #[allow(non_snake_case, unused_variables, clippy::unused_unit)]
            unsafe fn get<'w, 's>(
                world: WorldPtr<'w>,
                state: &'s mut Self::State,
            ) -> Self::Output<'w, 's> {
                let ($($t,)*) = state;

                ($(unsafe { $t::get(world, $t) },)*)
            }

            #[allow(non_snake_case, unused_variables)]
            fn apply(world: &mut World, state: &mut Self::State) {
                let ($($t,)*) = state;

                $(
                    $t::apply(world, $t);
                )*
            }
        }

        unsafe impl<$($t),*> ReadOnlySystemInput for ($($t,)*)
        where
            $($t: ReadOnlySystemInput,)*
        {
        }

        impl<$($t),*> StatelessSystemInput for ($($t,)*)
        where
            $($t: StatelessSystemInput,)*
        {
            fn init() -> Self::State {
                ($(<$t as StatelessSystemInput>::init(),)*)
            }
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_system!([$($rest)*] []);
        impl_system!([$($rest)* $head] [$($tail)*]);
    };
}

impl_system!(I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entity, Query};

    #[test]
    fn boxed_system_dispatch() {
        fn system(_world: &World, _query: Query<Entity>) {}

        assert_read_only_system(&system);

        let world = World::new();
        let mut boxed: Box<dyn ReadOnlySystem<_>> = Box::new(system) as _;

        unsafe {
            system.run_once(world.as_ptr());
            boxed.run_once(world.as_ptr());

            system.run_from_ref_once(&world);
            boxed.run_from_ref_once(&world);
        }
    }
}
