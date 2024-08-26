use std::fmt;

pub(crate) use self::allocator::*;
pub use self::ptr::*;
use crate::{QueryData, ReadOnlyQueryData, SparseIndex, World, WorldAccess, WorldPtr};

mod allocator;
mod ptr;

/// A identifier for an entity in a [`World`].
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) version: u32,
}

/// An iterator yielding an [`EntityRef`] for entities in a [`World`].
#[derive(Clone)]
pub struct EntitiesIter<'w> {
    pub(crate) world: &'w World,
    pub(crate) ids: EntityIterIds<'w>,
}

/// An iterator yielding an [`EntityMut`] for entities in a [`World`].
pub struct EntitiesIterMut<'w> {
    pub(crate) world: WorldPtr<'w>,
    pub(crate) ids: EntityIterIds<'w>,
}

impl SparseIndex for Entity {
    fn sparse_index(&self) -> usize {
        self.index as _
    }
}

unsafe impl QueryData for Entity {
    type Output<'w> = Self;

    fn access(_access: &mut WorldAccess) {}

    unsafe fn fetch(entity: EntityPtr<'_>) -> Option<Self::Output<'_>> {
        Some(entity.id())
    }
}

unsafe impl ReadOnlyQueryData for Entity {}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.index, self.version)
    }
}

impl<'w> Iterator for EntitiesIter<'w> {
    type Item = EntityRef<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids
            .next()
            .map(|entity| unsafe { self.world.entity(entity).unwrap_unchecked() })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for EntitiesIter<'_> {
    fn len(&self) -> usize {
        self.ids.len()
    }
}

impl<'w> Iterator for EntitiesIterMut<'w> {
    type Item = EntityMut<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|entity| unsafe {
            self.world.as_mut().entity_mut(entity).unwrap_unchecked()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for EntitiesIterMut<'_> {
    fn len(&self) -> usize {
        self.ids.len()
    }
}
