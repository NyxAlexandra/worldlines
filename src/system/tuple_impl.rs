macro_rules! tuple_impl {
    ($($i:ident),*) => {
        tuple_impl!([] [$($i)*]);
    };

    ([$($i:ident)*] []) => {
        unsafe impl<F, $($i,)* O> $crate::system::System
            for $crate::system::FunctionSystem<($($i,)*), O, F>
        where
            F: FnMut($($i,)*) -> O,
            F: for<'w, 's> FnMut($($i::Output<'w, 's>,)*) -> O,
            $($i: $crate::system::SystemInput,)*
        {
            type Input = ($($i,)*);
            type Output = O;

            fn needs_init(&self) -> bool {
                self.state.is_none()
            }

            fn init(&mut self, world: &$crate::world::World) {
                self.state
                .get_or_insert_with(|| <($($i,)*) as $crate::system::SystemInput>::init(world));
            }

            unsafe fn world_access(
                &mut self,
                builder: &mut $crate::access::WorldAccessBuilder<'_>,
            ) {
                // SAFETY: the caller ensures that [`System::init`] has been called
                let state = unsafe { self.state.as_mut().unwrap_unchecked() };

                <($($i,)*) as $crate::system::SystemInput>::world_access(state, builder);
            }

            #[allow(unused_variables)]
            unsafe fn run(&mut self, world: $crate::world::WorldPtr<'_>) -> Self::Output {
                // SAFETY: the caller ensures that [`System::init`] has been called
                let state = unsafe { self.state.as_mut().unwrap_unchecked() };
                // SAFETY: the caller ensures that the access is valid, that the world
                // pointer is valid for the described access
                #[allow(non_snake_case)]
                let ($($i,)*) =
                    unsafe { <($($i,)*) as $crate::system::SystemInput>::get(state, world) };

                (self.function)($($i),*)
            }

            #[allow(unused_variables)]
            fn needs_sync(&self) -> bool {
                // SAFETY: the caller ensures that [`System::init`] has been called
                #[allow(non_snake_case)]
                let state = unsafe { self.state.as_ref().unwrap_unchecked()};


                <($($i,)*) as $crate::system::SystemInput>::needs_sync(state)
            }

            #[allow(unused_variables)]
            unsafe fn sync(&mut self, world: &mut $crate::world::World) {
                // SAFETY: the caller ensures that [`System::init`] has been called
                let state = unsafe { self.state.as_mut().unwrap_unchecked() };

                <($($i,)*) as $crate::system::SystemInput>::sync(state, world);
            }
        }

        impl<F, $($i,)* O> $crate::system::IntoSystem<($($i,)*), O> for F
        where
            F: FnMut($($i,)*) -> O,
            F: for<'w, 's> FnMut($($i::Output<'w, 's>,)*) -> O,
            $($i: $crate::system::SystemInput,)*
        {
            type Output = $crate::system::FunctionSystem<($($i,)*), O, F>;

            fn into_system(self) -> Self::Output {
                $crate::system::FunctionSystem::new(self)
            }
        }

        unsafe impl<$($i),*> $crate::system::SystemInput for ($($i,)*)
        where
            $($i: $crate::system::SystemInput,)*
        {
            type Output<'w, 's> = ($($i::Output<'w, 's>,)*);
            type State = ($($i::State,)*);


            #[allow(unused_variables, clippy::unused_unit)]
            fn init(world: &$crate::world::World) -> Self::State {
                ($($i::init(world),)*)
            }

            fn world_access(
                state: &Self::State,
                #[allow(unused)]
                builder: &mut $crate::access::WorldAccessBuilder<'_>,
            ) {
                #[allow(non_snake_case)]
                let ($($i,)*) = state;

                $($i::world_access($i, builder));*
            }

            #[allow(unused_variables, clippy::unused_unit)]
            unsafe fn get<'w, 's>(
                state: &'s mut Self::State,
                world: $crate::world::WorldPtr<'w>,
            ) -> Self::Output<'w, 's> {
                #[allow(non_snake_case)]
                let ($($i,)*) = state;

                ($(unsafe { $i::get($i, world) },)*)
            }

            fn needs_sync(state: &Self::State) -> bool {
                #[allow(non_snake_case)]
                let ($($i,)*) = state;

                false $(|| <$i as $crate::system::SystemInput>::needs_sync($i))*
            }

            #[allow(unused_variables)]
            fn sync(state: &mut Self::State, world: &mut $crate::world::World) {
                #[allow(non_snake_case)]
                let ($($i,)*) = state;

                $(<$i as $crate::system::SystemInput>::sync($i, world));*
            }
        }

        unsafe impl<F, $($i,)* O> $crate::system::ReadOnlySystem
            for $crate::system::FunctionSystem<($($i,)*), O, F>
        where
            F: FnMut($($i,)*) -> O,
            F: for<'w, 's> FnMut($($i::Output<'w, 's>,)*) -> O,
            $($i: $crate::system::ReadOnlySystemInput,)*
        {
        }

        unsafe impl<$($i),*> $crate::system::ReadOnlySystemInput for ($($i,)*)
        where
            $($i: $crate::system::ReadOnlySystemInput,)*
        {
        }
    };

    ([$($rest:ident)*]  [$head:ident $($iail:ident)*]) => {
        tuple_impl!([$($rest)*] []);
        tuple_impl!([$($rest)* $head] [$($iail)*]);
    };
}

tuple_impl!(
    I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15
);
