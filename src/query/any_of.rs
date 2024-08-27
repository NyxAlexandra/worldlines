use std::marker::PhantomData;

use crate::{EntityRef, QueryFilter, QueryFilterSet};

// TODO: remove `Or` and replace it with this

/// A [`QueryFilter`] that includes entities that match any of the filters in a
/// [`QueryFilterSet`].
pub struct AnyOf<F: QueryFilterSet>(PhantomData<F>);

impl<F: QueryFilterSet> QueryFilter for AnyOf<F> {
    fn include(entity: EntityRef<'_>) -> bool {
        F::filters().any(|f| f(entity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contains, Entity, World};

    #[test]
    fn any_of_just_one() {
        struct A;
        struct B;

        let mut world = World::new();

        let e0 = world.spawn((A,)).id();
        let _e1 = world.spawn((B,)).id();
        let e2 = world.spawn((A, B)).id();

        let mut query = world.query::<Entity, AnyOf<(Contains<A>,)>>().unwrap();

        assert_eq!(query.next(), Some(e0));
        assert_eq!(query.next(), Some(e2));
        assert!(query.next().is_none());
    }

    #[test]
    fn any_of_query() {
        struct A;
        struct B;
        struct C;

        let mut world = World::new();

        let e0 = world.spawn((A,)).id();
        let e1 = world.spawn((B,)).id();
        let _e2 = world.spawn((C,)).id();
        let e3 = world.spawn((A, B)).id();
        let e4 = world.spawn((A, C)).id();
        let e5 = world.spawn((B, C)).id();

        let mut query =
            world.query::<Entity, AnyOf<(Contains<A>, Contains<B>)>>().unwrap();

        assert_eq!(query.next().unwrap(), e0);
        assert_eq!(query.next().unwrap(), e1);
        assert_eq!(query.next().unwrap(), e3);
        assert_eq!(query.next().unwrap(), e4);
        assert_eq!(query.next().unwrap(), e5);
        assert!(query.next().is_none());
    }
}
