macro_rules! impl_bundle {
    ($($t:ident),*) => {
        impl_bundle!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        unsafe impl<$($t),*> crate::component::Bundle for ($($t,)*)
        where
            $($t: crate::component::Bundle),*
        {
            #[allow(unused, non_snake_case)]
            fn components(builder: &mut crate::component::ComponentSetBuilder<'_>) {
                $($t::components(builder));*
            }

            #[allow(unused, non_snake_case)]
            fn write(self, writer: &mut crate::component::ComponentWriter<'_, '_>) {
                let ($($t,)*) = self;

                $(
                    $t.write(writer);
                )*
            }
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_bundle!([$($rest)*] []);
        impl_bundle!([$($rest)* $head] [$($tail)*]);
    };
}

impl_bundle!(
    C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15
);
