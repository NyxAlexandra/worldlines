use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::{LazyLock, OnceLock};

use dashmap::DashMap;

use super::Resource;
use crate::storage::{SparseIndex, UsizeHasher};

/// A unique identifier for a [`Resource`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub(crate) usize);

/// A [`ResourceVTable`] implementation for statically-known
/// [resources](Resource).
#[derive(Clone, Copy)]
pub struct ResourceInfo {
    inner: &'static dyn ResourceVTable,
}

/// Trait for types that provide the methods of [`Resource`].
pub trait ResourceVTable: Send + Sync + 'static {
    /// Returns the id of the resource.
    fn id(&self) -> ResourceId;

    /// Returns the [type name](std::any::type_name) of the resource.
    fn type_name(&self) -> &'static str;

    // may expand to include resource hooks
}

/// A static container for allocating [`ResourceId`]'s.
pub struct ResourceIdCell<R: Resource> {
    inner: OnceLock<ResourceId>,
    _marker: PhantomData<R>,
}

/// Registry of [`ResourceInfo`] by their [id](ResourceId).
///
/// The registry is written to when a [`Resource::id`] is called for the first
/// time.
static REGISTRY: LazyLock<DashMap<ResourceId, ResourceInfo, UsizeHasher>> =
    LazyLock::new(Default::default);

impl ResourceId {
    /// Returns the id of the given resource.
    pub fn of<R: Resource>() -> Self {
        R::id()
    }

    /// Used internally by [`Resource::id`].
    pub(super) fn next() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        Self(COUNTER.fetch_add(1, atomic::Ordering::Relaxed))
    }
}

impl ResourceInfo {
    /// Returns the info of the given resource.
    pub const fn of<R: Resource>() -> Self {
        Self { inner: &PhantomData::<R> }
    }

    /// Returns the info of the resource with the given id.
    pub fn of_id(id: ResourceId) -> Self {
        // SAFETY: resource id's are only constructed in `Resource::id`, which
        // adds the resource to the registry on first call. As it is marked
        // `#[doc(hidden)]`, it's the user's fault if they override it and break
        // this invariant.
        unsafe { *REGISTRY.get(&id).unwrap_unchecked() }
    }
}

impl<R: Resource> ResourceIdCell<R> {
    /// Creates a new resource id cell.
    pub const fn new() -> Self {
        let inner = OnceLock::new();

        Self { inner, _marker: PhantomData }
    }

    /// Returns the stored resource id, initializing it if necessary.
    pub fn get_or_init(&self) -> ResourceId {
        *self.inner.get_or_init(|| {
            let id = ResourceId::next();

            REGISTRY.insert(id, ResourceInfo::of::<R>());

            id
        })
    }
}

// ---

impl ResourceVTable for ResourceInfo {
    fn id(&self) -> ResourceId {
        self.inner.id()
    }

    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }
}

impl SparseIndex for ResourceInfo {
    fn sparse_index(&self) -> usize {
        self.id().sparse_index()
    }
}

impl SparseIndex for ResourceId {
    fn sparse_index(&self) -> usize {
        self.0
    }
}

// ---

impl fmt::Debug for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceInfo")
            .field("type_name", &self.type_name())
            .field("id", &self.id())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.type_name().fmt(f)
    }
}

impl PartialEq for ResourceInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Eq for ResourceInfo {}

// ---

impl<R: Resource> ResourceVTable for PhantomData<R> {
    fn id(&self) -> ResourceId {
        R::id()
    }

    fn type_name(&self) -> &'static str {
        type_name::<R>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Resource)]
    struct A;

    #[derive(Resource)]
    struct B;

    #[test]
    fn ids_are_unique() {
        assert_ne!(ResourceId::of::<A>(), ResourceId::of::<B>());
        assert_ne!(ResourceInfo::of::<A>(), ResourceInfo::of::<B>());
        assert_ne!(ResourceInfo::of::<A>().id(), ResourceInfo::of::<B>().id(),);
    }

    #[test]
    fn id_eq() {
        assert_eq!(ResourceInfo::of::<A>(), ResourceInfo::of::<A>());
        assert_eq!(ResourceInfo::of::<B>(), ResourceInfo::of::<B>());

        assert_ne!(ResourceInfo::of::<A>(), ResourceInfo::of::<B>());
        assert_ne!(ResourceInfo::of::<B>(), ResourceInfo::of::<A>());
    }
}
