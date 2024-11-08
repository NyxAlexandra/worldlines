macro_rules! impl_query_data {
    ($($t:ident),*) => {
        impl_query_data!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        unsafe impl<$($t: crate::query::QueryData),*> crate::query::QueryData for ($($t,)*) {
            type Output<'w> = ($($t::Output<'w>,)*);

            #[allow(unused)]
            fn access(builder: &mut crate::access::WorldAccessBuilder<'_>) {
                $( $t::access(builder) );*
            }

            #[allow(unused)]
            unsafe fn get(entity: crate::entity::EntityPtr<'_>) -> Self::Output<'_> {
                #[allow(clippy::unused_unit)]
                ($(unsafe { $t::get(entity) },)*)
            }
        }

        unsafe impl<$($t),*> crate::query::ReadOnlyQueryData for ($($t,)*)
        where
            $($t: crate::query::ReadOnlyQueryData,)*
        {
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_query_data!([$($rest)*] []);
        impl_query_data!([$($rest)* $head] [$($tail)*]);
    };
}

impl_query_data!(D0, D1, D2, D3, D4, D5, D6, D7);
