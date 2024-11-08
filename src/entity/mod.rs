//! Defines entities, the individuals objects in an ECS.

use std::num::NonZeroU32;

use thiserror::Error;

pub(crate) use self::allocator::*;
pub use self::ptr::*;
pub use self::reference::*;
pub use self::world::*;
use crate::storage::SparseIndex;

mod allocator;
mod ptr;
mod reference;
#[cfg(test)]
mod tests;
mod world;

/// An identifier for an entity in the ECS.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId {
    #[cfg(target_endian = "little")]
    pub(crate) index: u32,
    pub(crate) version: NonZeroU32,
    #[cfg(target_endian = "big")]
    pub(crate) index: u32,
}

/// An error for when a requested entity was not found in the world.
#[derive(Debug, Clone, Copy, Error)]
#[error("entity not found: {0:?}")]
pub struct EntityNotFound(pub EntityId);

impl EntityId {
    pub(crate) const fn new(index: u32, version: NonZeroU32) -> Self {
        Self { index, version }
    }

    pub(crate) const fn from_index(index: u32) -> Self {
        Self::new(index, unsafe { NonZeroU32::new_unchecked(1) })
    }
}

impl SparseIndex for EntityId {
    fn sparse_index(&self) -> usize {
        self.index as _
    }
}
