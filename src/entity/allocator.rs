//! Stolen and modified from [`hecs`](https://github.com/Ralith/hecs/blob/ed23dedf77602756ffad2194558d7b23f54e2fc1/src/entities.rs#L151).

use std::iter::{self, Enumerate};
use std::num::NonZeroU32;
use std::ops::Range;
use std::slice;
use std::slice::SliceIndex;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

use super::EntityId;
use crate::component::TableId;
use crate::storage::TableRow;

/// Manages and allocates the entities in a [`World`](crate::world::World).
#[derive(Debug)]
pub struct Entities {
    slots: Vec<EntitySlot>,
    cursor: AtomicIsize,
    pending: Vec<u32>,
    allocated: usize,
    reserved: AtomicUsize,
}

/// Describes a possibly-live entity.
#[derive(Debug, Clone, Copy)]
pub struct EntitySlot {
    /// The version of the entity in this slot.
    pub version: NonZeroU32,
    /// Whether the entity is currently alive or not.
    alive: bool,
    /// Is `None` until [`Entities::set`] is called for the entity that indexes
    /// this slot.
    pub addr: Option<EntityAddr>,
}

/// The exact location of an entity within its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityAddr {
    /// The table index of the entity.
    pub table: TableId,
    /// The index into `table.entities`.
    pub row: TableRow,
}

/// An iterator over entity slots in [`Entities`].
#[derive(Clone)]
pub struct EntitySlots<'w> {
    inner: Enumerate<slice::Iter<'w, EntitySlot>>,
}

impl Entities {
    pub fn new() -> Self {
        let slots = Vec::new();
        let cursor = AtomicIsize::new(0);
        let pending = Vec::new();
        let allocated = 0;
        let reserved = AtomicUsize::new(0);

        Self { slots, cursor, pending, allocated, reserved }
    }

    /// Amount of allocated entities.
    pub fn len(&self) -> usize {
        self.allocated + self.reserved.load(Ordering::Relaxed)
    }

    /// Whether there are any allocated entities.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the entity is currently alive.
    pub fn contains(&self, entity: EntityId) -> bool {
        if let Some(slot) = self.slots.get(entity.index as usize) {
            slot.alive && slot.version == entity.version
        } else {
            let n = self.cursor.load(Ordering::Relaxed);

            entity.version.get() == 1
                && n < 0
                // this is `<=` instead of `<` because we start indices at `0` instead of `1`
                && (entity.index as isize) <= (n.abs() + self.slots.len() as isize)
        }
    }

    /// Iterate over the entities in storage.
    ///
    /// Only iterates over allocated entities so as to preserve order of index.
    /// As such, this will not include entities acquired from
    /// [`Entities::reserve`].
    pub fn iter(&self) -> EntitySlots<'_> {
        self.iter_slice(..)
    }

    /// Iterate over a range of entities the entities in storage.
    ///
    /// Only iterates over allocated entities so as to preserve order of index.
    /// As such, this will not include entities acquired from
    /// [`Entities::reserve`].
    pub fn iter_slice(
        &self,
        range: impl SliceIndex<[EntitySlot], Output = [EntitySlot]>,
    ) -> EntitySlots<'_> {
        EntitySlots { inner: self.slots[range].iter().enumerate() }
    }

    #[cfg(test)]
    fn iter_ids(&self) -> impl Iterator<Item = EntityId> + use<'_> {
        self.iter().map(|(id, _)| id)
    }

    /// Get the address of a entity.
    pub fn get(&self, entity: EntityId) -> Option<EntityAddr> {
        self.slots
            .get(entity.index as usize)
            .and_then(|EntitySlot { addr, .. }| *addr)
    }

    /// Returns slot(s) for an index.
    pub fn slot<I: SliceIndex<[EntitySlot]>>(
        &self,
        index: I,
    ) -> Option<&I::Output> {
        self.slots.get(index)
    }

    /// Returns slot(s) for an index.
    pub fn slot_mut<I: SliceIndex<[EntitySlot]>>(
        &mut self,
        index: I,
    ) -> Option<&mut I::Output> {
        self.slots.get_mut(index)
    }

    /// Allocate a new entity.
    ///
    /// This will also allocated all reserved entities.
    pub fn alloc(&mut self) -> EntityId {
        self.flush();

        self.allocated += 1;

        if let Some(index) = self.pending.pop() {
            *self.cursor.get_mut() = self.pending.len() as _;

            EntityId::new(index, self.slots[index as usize].version)
        } else {
            self.slots.push(EntitySlot::new());

            EntityId::from_index(
                u32::try_from(self.slots.len() + *self.reserved.get_mut() - 1)
                    .expect("entity overflow"),
            )
        }
    }

    /// Allocates an entity without reusing dead entities.
    ///
    /// Does not call [`Entities::flush`].
    pub fn alloc_end(&mut self) -> EntityId {
        self.allocated += 1;
        self.slots.push(EntitySlot::new());

        EntityId::from_index(
            u32::try_from(self.slots.len() + *self.reserved.get_mut() - 1)
                .expect("entity overflow"),
        )
    }

    /// Allocates multiple entities at once.
    ///
    /// Returns the range of allocated [`EntitySlot`]s.
    pub fn alloc_many(&mut self, count: usize) -> Range<usize> {
        self.flush();
        self.allocated += count;

        let start = self.slots.len();

        self.slots.extend(iter::repeat_with(EntitySlot::new).take(count));

        start..self.slots.len()
    }

    /// Reserve a new entity.
    ///
    /// Reserved entities are fully allocated (as in having a slot allocated)
    /// whenever a mutating method is called.
    pub fn reserve(&self) -> EntityId {
        self.reserved.fetch_add(1, Ordering::Relaxed);

        let n = self.cursor.fetch_sub(1, Ordering::Relaxed);

        if n > 0 {
            let index = self.pending[(n - 1) as usize];

            EntityId::new(index, self.slots[index as usize].version)
        } else {
            EntityId::from_index(
                u32::try_from(self.slots.len() as isize - n)
                    .expect("entity overflow"),
            )
        }
    }

    /// Free an entity, allowing its id to be reused.
    ///
    /// Returns the entity address if the entity existed (and thus was freed)
    /// and the table was set.
    pub fn free(&mut self, entity: EntityId) -> Option<EntityAddr> {
        self.flush();

        let slot = self.slots.get_mut(entity.index as usize)?;

        if entity.version != slot.version {
            return None;
        }

        let addr = slot.addr.take();

        slot.version =
            slot.version.checked_add(1).expect("entity version overflow");
        slot.alive = false;
        self.pending.push(entity.index);
        *self.cursor.get_mut() = self.pending.len() as _;
        // decrement `allocated` as all entities are guaranteed to be allocated
        // after [`Entities::flush`] was called above.
        self.allocated -= 1;

        addr
    }

    /// Set the address of an entity.
    ///
    /// Returns `Some` if the entity exists.
    pub fn set(&mut self, entity: EntityId, addr: EntityAddr) -> Option<()> {
        self.flush();

        self.slots
            .get_mut(entity.index as usize)
            .filter(|slot| slot.alive && slot.version == entity.version)
            .map(|slot| slot.addr = Some(addr))
    }

    /// Clear allocation state and all entities.
    pub fn clear(&mut self) {
        self.slots.clear();
        *self.cursor.get_mut() = 0;
        self.pending.clear();
        self.allocated = 0;
        *self.reserved.get_mut() = 0;
    }

    /// Fully allocates reserved entities.
    pub fn flush(&mut self) {
        if *self.reserved.get_mut() == 0 {
            return;
        }

        let cursor = self.cursor.get_mut();

        let new_cursor = if *cursor >= 0 {
            *cursor
        } else {
            let old_len = self.slots.len();
            let new_len = old_len + cursor.unsigned_abs();

            self.slots.resize(new_len, EntitySlot::new());
            self.allocated += -*cursor as usize;

            0
        };

        *cursor = new_cursor;

        self.allocated += self.pending.len() - new_cursor as usize;
        self.pending.clear();
        // all reserved entities are now fully allocated
        *self.reserved.get_mut() = 0;

        debug_assert_eq!(
            *self.reserved.get_mut(),
            0,
            "all entities that were reserved should have been allocated",
        );
    }
}

impl Default for Entities {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a Entities {
    type IntoIter = EntitySlots<'a>;
    type Item = (EntityId, Option<EntityAddr>);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl EntitySlot {
    /// A new live entity slot.
    ///
    /// Starts at version `1` and without an address.
    const fn new() -> Self {
        Self {
            version: unsafe { NonZeroU32::new_unchecked(1) },
            alive: true,
            addr: None,
        }
    }
}

impl Iterator for EntitySlots<'_> {
    type Item = (EntityId, Option<EntityAddr>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().and_then(|(index, slot)| {
            if slot.alive {
                Some((EntityId::new(index as _, slot.version), slot.addr))
            } else {
                self.next()
            }
        })
    }
}

impl ExactSizeIterator for EntitySlots<'_> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_alloc_and_free() {
        let mut entities = Entities::new();

        assert!(entities.is_empty());

        let e0 = entities.reserve();

        assert_eq!(e0.index, 0);
        assert_eq!(e0.version.get(), 1);
        assert!(entities.contains(e0));

        assert_eq!(entities.len(), 1);
        assert_eq!(entities.allocated, 0);
        assert_eq!(*entities.reserved.get_mut(), 1);

        let e1 = entities.alloc();

        assert_eq!(e1.index, 1);
        assert_eq!(e1.version.get(), 1);
        assert!(entities.contains(e1));

        assert_eq!(entities.len(), 2);
        assert_eq!(entities.allocated, 2);

        for slot in &entities.slots {
            assert!(slot.alive);
        }

        // don't unwrap here as we aren't setting the table
        _ = entities.free(e0);

        assert!(!entities.contains(e0));
        assert_eq!(entities.len(), 1);
        assert_eq!(entities.allocated, 1);

        _ = entities.free(e1);

        assert!(!entities.contains(e1));
        assert!(entities.is_empty());
        assert_eq!(entities.allocated, 0);
    }

    #[test]
    fn clear() {
        let mut entities = Entities::new();

        let [e0, e1, e2] =
            [entities.alloc(), entities.alloc(), entities.alloc()];

        assert_eq!(entities.len(), 3);

        assert!(entities.contains(e0));
        assert!(entities.contains(e1));
        assert!(entities.contains(e2));

        entities.clear();

        assert!(entities.is_empty());

        assert!(!entities.contains(e0));
        assert!(!entities.contains(e1));
        assert!(!entities.contains(e2));
    }

    #[test]
    fn iter() {
        let mut entities = Entities::new();

        assert!(entities.iter().next().is_none());

        let [e0, e1, e2, e3] = [
            entities.alloc(),
            entities.alloc(),
            entities.alloc(),
            entities.alloc(),
        ];

        {
            let mut iter = entities.iter().map(|(id, _)| id);

            assert_eq!(iter.next(), Some(e0));
            assert_eq!(iter.next(), Some(e1));
            assert_eq!(iter.next(), Some(e2));
            assert_eq!(iter.next(), Some(e3));
            assert_eq!(iter.next(), None);
        }

        entities.free(e1);

        {
            let mut iter = entities.iter_ids();

            assert_eq!(iter.next(), Some(e0));
            assert_eq!(iter.next(), Some(e2));
            assert_eq!(iter.next(), Some(e3));
            assert_eq!(iter.next(), None);
        }

        entities.free(e2);

        {
            let mut iter = entities.iter_ids();

            assert_eq!(iter.next(), Some(e0));
            assert_eq!(iter.next(), Some(e3));
            assert_eq!(iter.next(), None);
        }

        entities.free(e0);
        entities.free(e3);

        assert!(entities.iter().next().is_none());
    }

    #[test]
    fn alloc_many_len() {
        let mut entities = Entities::new();

        entities.alloc_many(3);
        entities.flush();

        assert_eq!(entities.len(), 3);
    }

    #[test]
    fn alloc_many_iter_count() {
        let mut entities = Entities::new();
        let mut iter = entities.alloc_many(3);

        assert_eq!(iter.len(), 3);

        iter.next();
        iter.next();
        iter.next();

        assert!(iter.next().is_none());
    }
}
