use std::any::type_name;
use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;

use super::Resource;
use crate::storage::SparseIndex;

/// Info about a type implementing [`Resource`].
#[derive(Clone, Copy)]
pub struct ResourceInfo {
    index: ResourceIndex,
    vtable: &'static dyn ResourceVTable,
}

/// Newtype for the index of a resource in [`Resources`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceIndex(pub(crate) usize);

trait ResourceVTable: 'static {
    fn type_name(&self) -> &'static str;

    // may expand to include resource hooks
}

impl ResourceInfo {
    pub fn of<R: Resource>(index: usize) -> Self {
        Self { index: ResourceIndex(index), vtable: &PhantomData::<R> }
    }

    pub fn index(&self) -> ResourceIndex {
        self.index
    }

    pub fn type_name(&self) -> &'static str {
        self.vtable.type_name()
    }
}

// ---

impl SparseIndex for ResourceInfo {
    fn sparse_index(&self) -> usize {
        self.index.sparse_index()
    }
}

impl SparseIndex for ResourceIndex {
    fn sparse_index(&self) -> usize {
        self.0
    }
}

impl<R: Resource> ResourceVTable for PhantomData<R> {
    fn type_name(&self) -> &'static str {
        type_name::<R>()
    }
}

// ---

impl fmt::Debug for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceInfo")
            .field("type_name", &self.type_name())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ResourceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.type_name().fmt(f)
    }
}

impl Borrow<ResourceIndex> for ResourceInfo {
    fn borrow(&self) -> &ResourceIndex {
        &self.index
    }
}

impl PartialEq for ResourceInfo {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for ResourceInfo {}
