use std::marker::PhantomData;

pub use self::any_of::*;
pub use self::contains::*;
pub use self::not::*;
pub use self::or::*;
use crate::{
    EntityIterIds,
    EntityPtr,
    EntityRef,
    ReadOnlySystemInput,
    SystemInput,
    World,
    WorldAccess,
    WorldAccessError,
    WorldPtr,
};

mod any_of;
mod contains;
mod not;
mod or;

/// A query of data in a [`World`](crate::World).
pub struct Query<'w, D, F = ()>
where
    D: QueryData,
    F: QueryFilter,
{
    world: WorldPtr<'w>,
    entities: EntityIterIds<'w>,
    _marker: PhantomData<(D, F)>,
}

/// The data that is retreived in a [`Query`].
///
/// # Safety
///
/// [`QueryData::access`] must accurately set what this query data accesses.
pub unsafe trait QueryData {
    /// The output of this query input.
    type Output<'w>;

    /// Set what this query accesses.
    ///
    /// Used to make sure that queries are safely constructed.
    fn access(access: &mut WorldAccess);

    /// Fetch the query item.
    ///
    /// # Safety
    ///
    /// The provided pointer must be valid for the access described in
    /// [`QueryData::access`].
    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>>;
}

/// Trait for [`QueryData`] that can be fetched from an immutable reference.
///
/// ## Implementation Safety
///
/// Must not mutably access data. This also means that [`QueryData::access`]
/// must only declare immutable access.
pub unsafe trait ReadOnlyQueryData: QueryData {}

/// Additional filters to place on a [`Query`].
pub trait QueryFilter {
    /// Returns `true` if the entity should be included in the [`Query`].
    fn include(entity: EntityRef<'_>) -> bool;
}

/// A set of [`QueryFilter`]s.
pub trait QueryFilterSet: QueryFilter {
    /// Return the function pointers of each filter in this set.
    fn filters() -> impl Iterator<Item = fn(EntityRef<'_>) -> bool>;
}

impl<'w, D, F> Query<'w, D, F>
where
    D: QueryData,
    F: QueryFilter,
{
    /// ## Safety
    ///
    /// The pointer must be valid for the access of the query.
    pub(crate) unsafe fn new(world: WorldPtr<'w>) -> Result<Self, WorldAccessError> {
        let mut access = WorldAccess::new();

        D::access(&mut access);

        if let Some(error) = access.error() {
            Err(error)
        } else {
            Ok(Self {
                world,
                entities: unsafe { world.entities().iter() },
                _marker: PhantomData,
            })
        }
    }
}

unsafe impl<D, F> SystemInput for Query<'_, D, F>
where
    D: QueryData,
    F: QueryFilter,
{
    type Output<'w, 's> = Query<'w, D, F>;
    type State = ();

    fn access(access: &mut WorldAccess) {
        D::access(access);
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        // SAFETY: caller must ensure that the access did not alias and the pointer is
        // valid for the query's access
        unsafe { Query::new(world).unwrap_unchecked() }
    }
}

unsafe impl<D, F> ReadOnlySystemInput for Query<'_, D, F>
where
    D: ReadOnlyQueryData,
    F: QueryFilter,
{
}

impl<'w, D, F> Iterator for Query<'w, D, F>
where
    D: QueryData,
    F: QueryFilter,
{
    type Item = D::Output<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.world.entity(self.entities.next()?);

        F::include(unsafe { entity.as_ref() })
            .then(|| unsafe { D::fetch(entity) })
            .flatten()
            .or_else(|| self.next())
    }
}

macro_rules! impl_query_data {
    ($($t:ident),*) => {
        impl_query_data!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        unsafe impl<$($t: QueryData),*> QueryData for ($($t,)*) {
            type Output<'w> = ($($t::Output<'w>,)*);

            #[allow(unused_variables)]
            fn access(access: &mut WorldAccess) {
                $(
                    $t::access(access);
                )*
            }

            #[allow(unused_variables)]
            unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
                Some(($(unsafe { $t::fetch(entity)? },)*))
            }
        }

        unsafe impl<$($t),*> ReadOnlyQueryData for ($($t,)*)
        where
            $($t: ReadOnlyQueryData,)*
        {
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_query_data!([$($rest)*] []);
        impl_query_data!([$($rest)* $head] [$($tail)*]);
    };
}

impl_query_data!(D0, D1, D2, D3, D4, D5, D6, D7);

macro_rules! impl_query_filter {
    ($($t:ident),*) => {
        impl_query_filter!([] [$($t)*]);
    };

    ([$($t:ident)*] []) => {
        impl<$($t),*> QueryFilter for ($($t,)*)
        where
            $($t: QueryFilter,)*
        {
            #[allow(unused_variables)]
            fn include(entity: EntityRef<'_>) -> bool {
                true $(&& $t::include(entity))*
            }
        }

        impl<$($t),*> QueryFilterSet for ($($t,)*)
        where
            $($t: QueryFilter,)*
        {
            // compiler seems to be falsely emitting this warning
            #[allow(refining_impl_trait)]
            fn filters() -> impl Iterator<Item = fn(EntityRef<'_>) -> bool> {
                [$($t::include as fn(EntityRef<'_>) -> bool),*].into_iter()
            }
        }
    };

    ([$($rest:ident)*]  [$head:ident $($tail:ident)*]) => {
        impl_query_filter!([$($rest)*] []);
        impl_query_filter!([$($rest)* $head] [$($tail)*]);
    };
}

impl_query_filter!(F0, F1, F2, F3, F4, F5, F6, F7);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Entity, EntityMut, World};

    fn _assert_impls<C: Component, F: QueryFilter>() {
        fn assert_query_data<D: QueryData>() {}
        fn assert_query_filter<F: QueryFilter>() {}

        assert_query_data::<()>();
        assert_query_data::<&C>();
        assert_query_data::<&mut C>();
        assert_query_data::<(&C, &C)>();
        assert_query_data::<(&C, &C, &mut C)>();
        assert_query_data::<(&mut C, &mut C, &mut C)>();
        assert_query_data::<(Entity, EntityRef, EntityMut)>();

        assert_query_filter::<()>();
        assert_query_filter::<F>();
        assert_query_filter::<(F, F)>();
        assert_query_filter::<(F, F, F)>();
    }

    #[test]
    fn query_no_filter() {
        let mut world = World::new();

        let e0 = world.spawn(("e0",)).id();
        let e1 = world.spawn(("e1", true)).id();
        let e2 = world.spawn((true,)).id();

        let mut query = world.query::<Entity, ()>().unwrap();

        assert_eq!(query.next(), Some(e0));
        assert_eq!(query.next(), Some(e1));
        assert_eq!(query.next(), Some(e2));
        assert!(query.next().is_none());
    }

    #[test]
    fn query_component() {
        struct A(u32);
        struct B;

        let mut world = World::new();

        world.spawn((A(0), B));
        world.spawn((B,));
        world.spawn((A(1),));

        let mut query = world.query::<&A, ()>().unwrap();

        assert_eq!(query.next().unwrap().0, 0);
        assert_eq!(query.next().unwrap().0, 1);
        assert!(query.next().is_none());
    }

    #[test]
    fn invalid_multiple_borrow() {
        struct A;

        let mut world = World::new();

        assert!(world.query_mut::<(&A, &mut A), ()>().is_err());
        assert!(world.query_mut::<(&mut A, &mut A), ()>().is_err());
        assert!(world.query_mut::<(&mut A, &mut A), ()>().is_err());
    }

    #[test]
    fn valid_multiple_borrow() {
        struct A;
        struct B;

        let mut world = World::new();

        assert!(world.query_mut::<(&mut A, &mut B), ()>().is_ok());
        assert!(world.query_mut::<(&A, &mut B), ()>().is_ok());
        assert!(world.query_mut::<(&mut A, &B), ()>().is_ok());
        assert!(world.query::<(&A, &B), ()>().is_ok());
    }

    #[test]
    fn entity_ptr_valadity() {
        let mut world = World::new();

        assert!(world.query::<EntityRef<'_>, ()>().is_ok());
        assert!(world.query_mut::<EntityMut<'_>, ()>().is_ok());
        assert!(world.query::<(EntityRef<'_>, EntityRef<'_>), ()>().is_ok());
        assert!(world.query_mut::<(EntityMut<'_>, EntityMut<'_>), ()>().is_err());
    }

    #[test]
    fn entity_and_component_borrow() {
        struct A;

        let mut world = World::new();

        {
            let mut access = WorldAccess::new();

            <(&mut A, EntityRef)>::access(&mut access);

            println!("<(&mut A, EntityRef)>::access(); -> {:?}", access);
        }

        assert!(world.query::<(&A, EntityRef<'_>), ()>().is_ok());
        assert!(world.query_mut::<(&mut A, EntityRef<'_>), ()>().is_err());
        assert!(world.query_mut::<(&A, EntityMut<'_>), ()>().is_err());
        assert!(world.query_mut::<(&mut A, EntityMut<'_>), ()>().is_err());
    }
}
