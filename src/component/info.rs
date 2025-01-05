use std::alloc::Layout;
use std::any::{type_name, TypeId};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::{LazyLock, OnceLock};
use std::{fmt, ptr};

use dashmap::DashMap;

use super::Component;
use crate::entity::EntityMut;
use crate::storage::{SparseIndex, UsizeHasher};

/// The sparse index for components.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentId(pub(crate) usize);

/// A [`ComponentVTable`] implementation for statically-known
/// [components](Component).
#[derive(Clone, Copy)]
pub struct ComponentInfo {
    inner: &'static dyn ComponentVTable,
}

/// Trait for types that provide the methods of [`Component`].
///
/// # Safety
///
/// [`ComponentVTable::drop`] must drop the component represented by this
/// vtable.
pub unsafe trait ComponentVTable: Send + Sync + 'static {
    /// Returns the id of the component.
    fn id(&self) -> ComponentId;

    /// Returns the type id of the component.
    fn type_id(&self) -> TypeId;

    /// Returns the [type name](std::any::type_name) of the component.
    fn type_name(&self) -> &'static str;

    /// Returns the layout of the component in memory.
    fn layout(&self) -> Layout;

    /// Returns a function that [drops the component
    /// in-place](std::ptr::drop_in_place).
    fn drop(&self) -> unsafe fn(*mut u8);

    /// Returns the [`Component::after_insert`] function.
    fn after_insert(&self) -> fn(EntityMut<'_>);

    /// Returns the [`Component::before_remove`] function.
    fn before_remove(&self) -> fn(EntityMut<'_>);
}

/// A static container for allocating [`ComponentId`]'s.
pub struct ComponentIdCell<C: Component> {
    inner: OnceLock<ComponentId>,
    _marker: PhantomData<C>,
}

/// Registry of [`ComponentInfo`] by their [id](ComponentId).
///
/// The registry is written to when a [`Component::id`] is called for the first
/// time.
static REGISTRY: LazyLock<DashMap<ComponentId, ComponentInfo, UsizeHasher>> =
    LazyLock::new(Default::default);

impl ComponentId {
    /// Returns the id of the given component.
    pub fn of<C: Component>() -> Self {
        C::id()
    }

    /// Used internally by [`Component::id`].
    pub(super) fn next() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        Self(COUNTER.fetch_add(1, atomic::Ordering::Relaxed))
    }
}

impl ComponentInfo {
    /// Returns the component info of the provided component.
    pub const fn of<C: Component>() -> Self {
        Self { inner: &PhantomData::<C> }
    }

    /// Returns the info of the component with the given id.
    pub fn of_id(id: ComponentId) -> Self {
        // SAFETY: component id's are only constructed in `Component::id`, which
        // adds the component to the registry on first call. As it is marked
        // `#[doc(hidden)]`, it's the user's fault if they override it and break
        // this invariant.
        unsafe { *REGISTRY.get(&id).unwrap_unchecked() }
    }
}

impl<C: Component> ComponentIdCell<C> {
    /// Creates a new component id cell.
    pub const fn new() -> Self {
        let inner = OnceLock::new();

        Self { inner, _marker: PhantomData }
    }

    /// Returns the stored component id, initializing it if necessary.
    pub fn get_or_init(&self) -> ComponentId {
        *self.inner.get_or_init(|| {
            let id = ComponentId::next();

            REGISTRY.insert(id, ComponentInfo::of::<C>());

            id
        })
    }
}

// ---

/// # Safety
///
/// Delegates to another implementation of [`ComponentVTable`].
unsafe impl ComponentVTable for ComponentInfo {
    fn id(&self) -> ComponentId {
        self.inner.id()
    }

    fn type_id(&self) -> TypeId {
        self.inner.type_id()
    }

    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    fn layout(&self) -> Layout {
        self.inner.layout()
    }

    fn drop(&self) -> unsafe fn(*mut u8) {
        self.inner.drop()
    }

    fn after_insert(&self) -> fn(EntityMut<'_>) {
        self.inner.after_insert()
    }

    fn before_remove(&self) -> fn(EntityMut<'_>) {
        self.inner.before_remove()
    }
}

impl SparseIndex for ComponentInfo {
    fn sparse_index(&self) -> usize {
        self.id().sparse_index()
    }
}

impl SparseIndex for ComponentId {
    fn sparse_index(&self) -> usize {
        self.0
    }
}

// ---

impl fmt::Debug for ComponentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentInfo")
            .field("type_name", &self.type_name())
            .field("id", &self.id())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ComponentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.type_name().fmt(f)
    }
}

impl PartialEq for ComponentInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl PartialEq<ComponentId> for ComponentInfo {
    fn eq(&self, other: &ComponentId) -> bool {
        self.id() == *other
    }
}

impl Eq for ComponentInfo {}

impl Hash for ComponentInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self.inner, state);
    }
}

impl PartialEq<ComponentInfo> for ComponentId {
    fn eq(&self, other: &ComponentInfo) -> bool {
        *self == other.id()
    }
}

// ---

/// # Safety
///
/// [`ComponentVTable::drop`] is a valid drop function pointer.
unsafe impl<C: Component> ComponentVTable for PhantomData<C> {
    fn id(&self) -> ComponentId {
        C::id()
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<C>()
    }

    fn type_name(&self) -> &'static str {
        type_name::<C>()
    }

    fn layout(&self) -> Layout {
        Layout::new::<C>()
    }

    fn drop(&self) -> unsafe fn(*mut u8) {
        |ptr| unsafe { ptr::drop_in_place(ptr.cast::<C>()) }
    }

    fn after_insert(&self) -> fn(EntityMut<'_>) {
        C::after_insert
    }

    fn before_remove(&self) -> fn(EntityMut<'_>) {
        C::before_remove
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[test]
    fn ids_are_unique() {
        assert_ne!(ComponentId::of::<A>(), ComponentId::of::<B>());
        assert_ne!(ComponentInfo::of::<A>(), ComponentInfo::of::<B>());
        assert_ne!(
            ComponentInfo::of::<A>().id(),
            ComponentInfo::of::<B>().id(),
        );
    }

    #[test]
    fn id_eq() {
        assert_eq!(ComponentInfo::of::<A>(), ComponentInfo::of::<A>());
        assert_eq!(ComponentInfo::of::<B>(), ComponentInfo::of::<B>());

        assert_ne!(ComponentInfo::of::<A>(), ComponentInfo::of::<B>());
        assert_ne!(ComponentInfo::of::<B>(), ComponentInfo::of::<A>());
    }
}
