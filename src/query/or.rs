use std::marker::PhantomData;

use crate::{EntityRef, QueryFilter};

/// A [`QueryFilter`] that computes `A || B`.
pub struct Or<A: QueryFilter, B: QueryFilter>(PhantomData<(A, B)>);

impl<A: QueryFilter, B: QueryFilter> QueryFilter for Or<A, B> {
    fn include(entity: EntityRef<'_>) -> bool {
        A::include(entity) || B::include(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Contains, Entity, World};

    #[test]
    fn or_query() {
        #[derive(Component)]
        struct A;

        #[derive(Component)]
        struct B;

        #[derive(Component)]
        struct C;

        let mut world = World::new();

        let e0 = world.spawn((A,)).id();
        let e1 = world.spawn((B,)).id();
        let _e2 = world.spawn((C,)).id();
        let e3 = world.spawn((A, B)).id();
        let e4 = world.spawn((A, C)).id();
        let e5 = world.spawn((B, C)).id();

        let mut query =
            world.query::<Entity, Or<Contains<A>, Contains<B>>>().unwrap();

        assert_eq!(query.next().unwrap(), e0);
        assert_eq!(query.next().unwrap(), e1);
        assert_eq!(query.next().unwrap(), e3);
        assert_eq!(query.next().unwrap(), e4);
        assert_eq!(query.next().unwrap(), e5);
        assert!(query.next().is_none());
    }
}
