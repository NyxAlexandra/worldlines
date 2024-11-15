macro_rules! impl_tuples {
    ($($t:ident),*) => {
        impl_tuples!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        unsafe impl<F, $($t,)* O> $crate::system::System
            for $crate::system::FunctionSystem<($($t,)*), O, F>
        where
            F: FnMut($($t,)*) -> O,
            F: for<'w, 's> FnMut($($t::Output<'w, 's>,)*) -> O,
            $($t: $crate::system::SystemInput,)*
        {
            type Input = ($($t,)*);
            type Output = O;

            fn needs_init(&self) -> bool {
                self.state.is_none()
            }

            fn init(&mut self, world: &$crate::world::World) {
                self.state
                .get_or_insert_with(|| <($($t,)*) as $crate::system::SystemInput>::init(world));
            }

            unsafe fn world_access(
                &mut self,
                builder: &mut $crate::access::WorldAccessBuilder<'_>,
            ) {
                // SAFETY: the caller ensures that [`System::init`] has been called
                let state = unsafe { self.state.as_mut().unwrap_unchecked() };

                <($($t,)*) as $crate::system::SystemInput>::world_access(state, builder);
            }

            #[allow(unused_variables)]
            unsafe fn run(&mut self, world: $crate::world::WorldPtr<'_>) -> Self::Output {
                // SAFETY: the caller ensures that [`System::init`] has been called
                let state = unsafe { self.state.as_mut().unwrap_unchecked() };
                // SAFETY: the caller ensures that the access is valid, that the world
                // pointer is valid for the described access
                #[allow(non_snake_case)]
                let ($($t,)*) =
                    unsafe { <($($t,)*) as $crate::system::SystemInput>::get(state, world) };

                (self.function)($($t),*)
            }

            #[allow(unused_variables)]
            fn needs_sync(&self) -> bool {
                // SAFETY: the caller ensures that [`System::init`] has been called
                #[allow(non_snake_case)]
                let state = unsafe { self.state.as_ref().unwrap_unchecked()};


                <($($t,)*) as $crate::system::SystemInput>::needs_sync(state)
            }

            #[allow(unused_variables)]
            unsafe fn sync(&mut self, world: &mut $crate::world::World) {
                // SAFETY: the caller ensures that [`System::init`] has been called
                let state = unsafe { self.state.as_mut().unwrap_unchecked() };

                <($($t,)*) as $crate::system::SystemInput>::sync(state, world);
            }
        }

        impl<F, $($t,)* O> $crate::system::IntoSystem<($($t,)*), O> for F
        where
            F: FnMut($($t,)*) -> O,
            F: for<'w, 's> FnMut($($t::Output<'w, 's>,)*) -> O,
            $($t: $crate::system::SystemInput,)*
        {
            type Output = $crate::system::FunctionSystem<($($t,)*), O, F>;

            fn into_system(self) -> Self::Output {
                $crate::system::FunctionSystem::new(self)
            }
        }

        unsafe impl<$($t),*> $crate::system::SystemInput for ($($t,)*)
        where
            $($t: $crate::system::SystemInput,)*
        {
            type Output<'w, 's> = ($($t::Output<'w, 's>,)*);
            type State = ($($t::State,)*);


            #[allow(unused_variables, clippy::unused_unit)]
            fn init(world: &$crate::world::World) -> Self::State {
                ($($t::init(world),)*)
            }

            fn world_access(
                state: &Self::State,
                #[allow(unused)]
                builder: &mut $crate::access::WorldAccessBuilder<'_>,
            ) {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                $($t::world_access($t, builder));*
            }

            #[allow(unused_variables, clippy::unused_unit)]
            unsafe fn get<'w, 's>(
                state: &'s mut Self::State,
                world: $crate::world::WorldPtr<'w>,
            ) -> Self::Output<'w, 's> {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                ($(unsafe { $t::get($t, world) },)*)
            }

            fn needs_sync(state: &Self::State) -> bool {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                false $(|| <$t as $crate::system::SystemInput>::needs_sync($t))*
            }

            #[allow(unused_variables)]
            fn sync(state: &mut Self::State, world: &mut $crate::world::World) {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                $(<$t as $crate::system::SystemInput>::sync($t, world));*
            }
        }

        unsafe impl<F, $($t,)* O> $crate::system::ReadOnlySystem
            for $crate::system::FunctionSystem<($($t,)*), O, F>
        where
            F: FnMut($($t,)*) -> O,
            F: for<'w, 's> FnMut($($t::Output<'w, 's>,)*) -> O,
            $($t: $crate::system::ReadOnlySystemInput,)*
        {
        }

        unsafe impl<$($t),*> $crate::system::ReadOnlySystemInput for ($($t,)*)
        where
            $($t: $crate::system::ReadOnlySystemInput,)*
        {
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_tuples!([$($rest)*] []);
        impl_tuples!([$($rest)* $head] [$($tail)*]);
    };
}

impl_tuples!(
    I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15
);
