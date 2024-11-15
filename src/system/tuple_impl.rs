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
            type State = ($(<$t as $crate::system::SystemInput>::State,)*);

            fn init(&mut self, world: &$crate::world::World) -> Self::State {
                <($($t,)*) as $crate::system::SystemInput>::init(world)
            }

            fn world_access(
                &mut self,
                state: &Self::State,
                builder: &mut $crate::access::WorldAccessBuilder<'_>,
            ) {
                <($($t,)*) as $crate::system::SystemInput>::world_access(state, builder);
            }

            #[allow(unused_variables)]
            unsafe fn run(
                &mut self,
                state: &mut Self::State,
                world: $crate::world::WorldPtr<'_>,
            ) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                (self.function)($(unsafe { $t::get($t, world) },)*)
            }

            #[allow(unused_variables)]
            fn needs_sync(&self, state: &Self::State) -> bool {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                false $(|| <$t as $crate::system::SystemInput>::needs_sync($t))*
            }

            #[allow(unused_variables)]
            fn sync(&mut self, state: &mut Self::State, world: &mut $crate::world::World) {
                #[allow(non_snake_case)]
                let ($($t,)*) = state;

                $($t::sync($t, world));*
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
