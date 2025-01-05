macro_rules! tuple_impl {
    ($($d:ident),*) => {
        tuple_impl!([] [$($d)*]);
    };

    ([$($d:ident)*] []) => {
        unsafe impl<$($d: crate::query::QueryData),*> crate::query::QueryData for ($($d,)*) {
            type Output<'w> = ($($d::Output<'w>,)*);

            #[allow(unused)]
            fn world_access(access: &mut crate::access::WorldAccess) {
                $( $d::world_access(access) );*
            }

            #[allow(unused)]
            unsafe fn get(entity: crate::entity::EntityPtr<'_>) -> Self::Output<'_> {
                #[allow(clippy::unused_unit)]
                ($(unsafe { $d::get(entity) },)*)
            }
        }

        unsafe impl<$($d),*> crate::query::ReadOnlyQueryData for ($($d,)*)
        where
            $($d: crate::query::ReadOnlyQueryData,)*
        {
        }
    };

    ([$($rest:ident)*]  [$head:ident $($dail:ident)*]) => {
        tuple_impl!([$($rest)*] []);
        tuple_impl!([$($rest)* $head] [$($dail)*]);
    };
}

tuple_impl!(D0, D1, D2, D3, D4, D5, D6, D7);
