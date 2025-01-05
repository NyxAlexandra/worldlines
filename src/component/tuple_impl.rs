macro_rules! tuple_impl {
    ($($c:ident),*) => {
        tuple_impl!([] [$($c)*]);
    };

    ([$($c:ident)*] []) => {
        unsafe impl<$($c),*> crate::component::Bundle for ($($c,)*)
        where
            $($c: crate::component::Bundle),*
        {
            #[allow(unused, non_snake_case)]
            fn components(components: &mut crate::component::ComponentSet) {
                $($c::components(components));*
            }

            #[allow(unused, non_snake_case)]
            fn write(self, writer: &mut crate::component::ComponentWriter<'_, '_>) {
                let ($($c,)*) = self;

                $(
                    $c.write(writer);
                )*
            }
        }
    };

    ([$($rest:ident)*]  [$head:ident $($cail:ident)*]) => {
        tuple_impl!([$($rest)*] []);
        tuple_impl!([$($rest)* $head] [$($cail)*]);
    };
}

tuple_impl!(
    C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15
);
