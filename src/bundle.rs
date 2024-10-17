#[cfg(feature = "derive")]
pub use archetypal_ecs_macros::Bundle;

use crate::{Component, EntityPtr, TypeSet};

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
    /// The types in this bundle.
    fn types() -> TypeSet;

    /// Take each component out of the bundle.
    fn take(self, writer: &mut BundleWriter<'_>);
}

/// A type used by [`Bundle::take`] to write components to an entity.
pub struct BundleWriter<'w> {
    inner: EntityPtr<'w>,
}

impl<'w> BundleWriter<'w> {
    pub(crate) fn new(inner: EntityPtr<'w>) -> Self {
        Self { inner }
    }

    /// Writes a component to an entity's storage.
    pub fn write<C: Component>(&mut self, component: C) {
        unsafe {
            C::on_insert(self.inner.as_mut());
            self.inner.table_mut().write(self.inner.entity, component)
        };
    }
}

unsafe impl<C: Component> Bundle for C {
    fn types() -> TypeSet {
        TypeSet::new().with::<C>()
    }

    fn take(self, writer: &mut BundleWriter<'_>) {
        writer.write(self);
    }
}

macro_rules! impl_bundle {
    ($($t:ident),*) => {
        impl_bundle!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        unsafe impl<$($t),*> Bundle for ($($t,)*)
        where
            $($t: Bundle),*
        {
            fn types() -> TypeSet {
                let iter = ::core::iter::empty();

                $(
                    let types = $t::types();
                    let iter = iter.chain(types.iter());
                )*

                TypeSet::from_iter(iter)
            }

            #[allow(unused)]
            fn take(mut self, writer: &mut BundleWriter<'_>) {
                #[allow(non_snake_case)]
                let ($($t,)*) = self;

                $(
                    $t.take(writer);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    #[cfg(feature = "derive")]
    fn derived_bundle_correctly_inserts() {
        use crate::Component;

        #[derive(Component)]
        struct Person;

        #[derive(Component)]
        struct Name(&'static str);

        #[derive(Component)]
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
