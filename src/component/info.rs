use std::alloc::Layout;
use std::any::{type_name, TypeId};
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::{fmt, ptr};

use super::Component;
use crate::entity::EntityMut;
use crate::storage::SparseIndex;

/// Info about a type implementing [`Component`].
#[derive(Clone, Copy)]
pub struct ComponentInfo {
    index: ComponentIndex,
    vtable: &'static dyn ComponentVTable,
}

/// The sparse index for components.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComponentIndex(pub(crate) usize);

trait ComponentVTable: 'static {
    fn type_id(&self) -> TypeId;

    fn type_name(&self) -> &'static str;

    fn layout(&self) -> Layout;

    fn drop(&self) -> unsafe fn(*mut u8);

    fn after_insert(&self) -> fn(EntityMut<'_>);

    fn before_remove(&self) -> fn(EntityMut<'_>);
}

impl ComponentInfo {
    pub fn of<C: Component>(index: usize) -> Self {
        Self::of_index::<C>(ComponentIndex(index))
    }

    pub const fn of_index<C: Component>(index: ComponentIndex) -> Self {
        let vtable = &PhantomData::<C>;

        Self { index, vtable }
    }

    pub fn index(self) -> ComponentIndex {
        self.index
    }

    pub fn after_insert(&self) -> fn(EntityMut<'_>) {
        self.vtable.after_insert()
    }

    pub fn before_remove(&self) -> fn(EntityMut<'_>) {
        self.vtable.before_remove()
    }

    pub fn type_name(&self) -> &'static str {
        self.vtable.type_name()
    }

    pub fn type_id(&self) -> TypeId {
        self.vtable.type_id()
    }

    pub fn layout(&self) -> Layout {
        self.vtable.layout()
    }

    pub fn drop(&self) -> unsafe fn(*mut u8) {
        self.vtable.drop()
    }
}

impl SparseIndex for ComponentInfo {
    fn sparse_index(&self) -> usize {
        self.index.sparse_index()
    }
}

impl Borrow<ComponentIndex> for ComponentInfo {
    fn borrow(&self) -> &ComponentIndex {
        &self.index
    }
}

impl fmt::Debug for ComponentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentInfo")
            .field("type_name", &self.type_name())
            .field("type_id", &self.type_id())
            .field("layout", &self.layout())
            .field("index", &self.index())
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
        self.index == other.index && ptr::eq(self.vtable, other.vtable)
    }
}

impl Eq for ComponentInfo {}

impl Hash for ComponentInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        ptr::hash(self.vtable, state);
    }
}

impl SparseIndex for ComponentIndex {
    fn sparse_index(&self) -> usize {
        self.0
    }
}

impl<C: Component> ComponentVTable for PhantomData<C> {
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
