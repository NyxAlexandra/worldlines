use std::alloc::Layout;
use std::any::{self, TypeId};
use std::hash::Hash;
use std::{cmp, fmt, hash};

use crate::{ComponentId, SparseIndex};

/// Describes how to handle a particular type.
#[derive(Clone, Copy)]
pub struct TypeData {
    // keep [`ComponentId`] here to avoid interactions with the id registry. Because the
    // [`ComponentId`] is used as the source of the sparse index for [`TypeData`], it is
    // accessed very often.
    component_id: ComponentId,
    // trick to majorly decrease the size of this type.
    inner: fn() -> Inner,
}

struct Inner {
    type_name: &'static str,
    type_id: TypeId,
    layout: Layout,
    drop: unsafe fn(*mut u8),
}

impl TypeData {
    /// Returns the [`TypeData`] of the provided type.
    pub fn of<T: 'static>() -> Self {
        Self {
            component_id: ComponentId::of::<T>(),
            inner: || Inner {
                type_name: any::type_name::<T>(),
                type_id: TypeId::of::<T>(),
                layout: Layout::new::<T>(),
                drop: |ptr| unsafe { ptr.cast::<T>().drop_in_place() },
            },
        }
    }

    /// Returns the [`TypeData`] of the provided type.
    pub fn of_val<T: 'static>(_: &T) -> Self {
        Self::of::<T>()
    }

    /// The [`std::any::type_name`] of this type.
    pub fn type_name(&self) -> &'static str {
        (self.inner)().type_name
    }

    /// The [`TypeId`] of this type.
    pub fn type_id(&self) -> TypeId {
        (self.inner)().type_id
    }

    /// The layout of this type in memory.
    pub fn layout(&self) -> Layout {
        (self.inner)().layout
    }

    /// Drop function for this type.
    pub fn drop(&self) -> unsafe fn(*mut u8) {
        (self.inner)().drop
    }

    /// The [`ComponentId`] of this type.
    pub(crate) const fn component_id(&self) -> ComponentId {
        self.component_id
    }
}

impl SparseIndex for TypeData {
    fn sparse_index(&self) -> usize {
        self.component_id().sparse_index()
    }
}

impl fmt::Debug for TypeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeData")
            .field("type_name", &self.type_name())
            .field("type_id", &self.type_id())
            .field("layout", &self.layout())
            .field("drop", &self.drop())
            .field("component_id", &self.component_id)
            .finish()
    }
}

impl fmt::Display for TypeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.type_name())
    }
}

impl PartialEq for TypeData {
    fn eq(&self, other: &Self) -> bool {
        self.component_id == other.component_id
    }
}

impl Eq for TypeData {}

impl PartialOrd for TypeData {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TypeData {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.component_id.cmp(&other.component_id)
    }
}

impl Hash for TypeData {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.component_id.hash(state);
    }
}
