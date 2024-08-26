use std::ptr::NonNull;

#[cfg(feature = "derive")]
pub use macros::Bundle;

use crate::{TypeData, TypeSet};

/// Trait for bundling components that are often added together.
///
/// This trait can be derived if the `derive` feature is enabled.
///
/// # Safety
///
/// - The [`TypeSet`] returned by [`Bundle::types`] must be the same each time
///   it is called.
/// - [`Bundle::take`] must call `f` and make sure that none of the components
///   are dropped (likely via [`std::mem::forget`]).
/// - The [`TypeData`] and pointers must match and the pointer must point to a
///   valid instance of the described type.
pub unsafe trait Bundle: Send + 'static {
    /// The iterator constructed in [`Bundle::take`].
    type TakeIter: Iterator<Item = (TypeData, NonNull<u8>)>;

    /// The types in this bundle.
    fn types() -> TypeSet;

    /// Take each component out of the bundle.
    ///
    /// The function `f` is on an iterator over pointers to each component in
    /// the bundle. After `f` is called, each component is considered to be
    /// moved.
    fn take(self, f: impl FnOnce(Self::TakeIter));
}

macro_rules! no_op {
    ($_:tt) => {};
}

macro_rules! impl_bundle {
    ($($t:ident),*) => {
        impl_bundle!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        unsafe impl<$($t),*> Bundle for ($($t,)*)
        where
            Self: 'static,
            $($t: crate::Component),*
        {
            // this is so silly :3
            type TakeIter = <[(TypeData, NonNull<u8>); {
                #[allow(unused_mut)]
                let mut len = 0;

                $(
                    no_op!($t);

                    len += 1;
                )*

                len
            }] as IntoIterator>::IntoIter;

            fn types() -> TypeSet {
                TypeSet::from_iter([$(TypeData::of::<$t>()),*])
            }

            fn take(mut self, f: impl FnOnce(Self::TakeIter)) {
                #[allow(non_snake_case)]
                let ($($t,)*) = &mut self;

                f([
                    $((TypeData::of::<$t>(), NonNull::from($t).cast())),*
                ].into_iter());

                #[allow(forgetting_copy_types)]
                ::std::mem::forget(self);
            }
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_bundle!([$($rest)*] []);
        impl_bundle!([$($rest)* $head] [$($tail)*]);
    };
}

impl_bundle!(C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    #[cfg(feature = "derive")]
    fn derived_bundle_correctly_inserts() {
        struct Person;
        struct Name(&'static str);
        struct Age(u32);

        #[derive(Bundle)]
        struct PersonBundle {
            person: Person,
            name: Name,
            age: Age,
        }

        let mut world = World::new();

        let person = world.spawn(PersonBundle {
            person: Person,
            name: Name("Alexandra"),
            age: Age(123),
        });

        assert!(person.contains::<Person>());
        assert_eq!(person.get::<Name>().unwrap().0, "Alexandra");
        assert_eq!(person.get::<Age>().unwrap().0, 123);
    }
}
