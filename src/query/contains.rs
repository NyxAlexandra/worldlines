use std::marker::PhantomData;

use crate::{
    Component,
    EntityPtr,
    EntityRef,
    QueryData,
    QueryFilter,
    ReadOnlyQueryData,
    WorldAccess,
};

/// A [`Query`](crate::Query) parameter that can be used to check if an entity
/// contains a component or filter only entities that contain the component.
///
/// ```
/// # use archetypal_ecs::{Query, Contains, Component};
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # #[derive(Component)]
/// # struct B;
/// #
/// # #[derive(Component)]
/// # struct C;
///
/// fn system(mut query: Query<(&A, Contains<B>), Contains<C>>) {
///     for (a, contains_b) in query {
///         // ...
///     }
/// }
/// ```
pub struct Contains<C: Component>(PhantomData<C>);

unsafe impl<C: Component> QueryData for Contains<C> {
    type Output<'w> = bool;

    fn access(_access: &mut WorldAccess) {}

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        // SAFETY: the pointer provided to [`QueryData::fetch`] must always be
        // valid to read metadata
        Some(unsafe { entity.as_ref().contains::<C>() })
    }
}

unsafe impl<C: Component> ReadOnlyQueryData for Contains<C> {}

impl<C: Component> QueryFilter for Contains<C> {
    fn include(entity: EntityRef<'_>) -> bool {
        entity.contains::<C>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[test]
    fn filter_not_contains() {
        #[derive(Component)]
        struct Poisoned;

        #[derive(Component)]
        struct Hp(u32);

        let mut world = World::new();

        let healthy = world.spawn((Hp(3),)).id();
        let poisoned = world.spawn((Hp(3), Poisoned)).id();

        for Hp(hp) in world.query_mut::<&mut Hp, Contains<Poisoned>>().unwrap()
        {
            *hp -= 1;
        }

        assert_eq!(world.entity(healthy).unwrap().get::<Hp>().unwrap().0, 3);
        assert_eq!(world.entity(poisoned).unwrap().get::<Hp>().unwrap().0, 2);
    }
}
