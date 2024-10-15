#![allow(clippy::needless_pub_self)]

use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use dashmap::DashMap;

pub(self) use self::column::*;
pub(crate) use self::storage::*;
pub(crate) use self::table::*;
use crate::{EntityPtr, QueryData, ReadOnlyQueryData, SparseIndex, WorldAccess};

mod column;
mod storage;
mod table;

/// A single value in an ECS.
pub trait Component: Send + Sync + 'static {}

impl<C: Send + Sync + 'static> Component for C {}

/// A unique identifer for a [`Component`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ComponentId(usize);

impl ComponentId {
    pub fn of<T: 'static>() -> Self {
        static REGISTRY: OnceLock<DashMap<TypeId, ComponentId>> = OnceLock::new();
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        *REGISTRY
            .get_or_init(Default::default)
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Self(COUNTER.fetch_add(1, Ordering::Relaxed)))
    }
}

impl SparseIndex for ComponentId {
    fn sparse_index(&self) -> usize {
        self.0
    }
}

unsafe impl<C: Component> QueryData for &C {
    type Output<'w> = &'w C;

    fn access(access: &mut WorldAccess) {
        access.component::<C>();
    }

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        unsafe { entity.get().ok() }
    }
}

unsafe impl<C: Component> ReadOnlyQueryData for &C {}

unsafe impl<C: Component> QueryData for &mut C {
    type Output<'w> = &'w mut C;

    fn access(access: &mut WorldAccess) {
        access.component_mut::<C>();
    }

    unsafe fn fetch(mut entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        unsafe { entity.get_mut().ok() }
    }
}

unsafe impl<C: Component> QueryData for Option<&C> {
    type Output<'w> = Option<&'w C>;

    fn access(access: &mut WorldAccess) {
        access.component::<C>();
    }

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        unsafe { Some(entity.get().ok()) }
    }
}

unsafe impl<C: Component> ReadOnlyQueryData for Option<&C> {}

unsafe impl<C: Component> QueryData for Option<&mut C> {
    type Output<'w> = Option<&'w mut C>;

    fn access(access: &mut WorldAccess) {
        access.component_mut::<C>();
    }

    unsafe fn fetch(mut entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        unsafe { Some(entity.get_mut().ok()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_id_unique() {
        struct A;
        struct B;

        assert_ne!(ComponentId::of::<A>(), ComponentId::of::<B>());
    }
}
