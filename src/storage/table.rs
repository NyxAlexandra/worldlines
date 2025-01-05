use std::mem;
use std::ptr::NonNull;

use super::{Column, SparseIndex, SparseIter, SparseMap};
use crate::component::{Component, ComponentId, ComponentSet};
use crate::entity::EntityId;

/// Storage for entities with the same components.
#[derive(Debug)]
pub struct Table {
    components: ComponentSet,
    entities: SparseMap<TableRow, EntityId>,
    columns: SparseMap<ComponentId, Column>,
}

/// The row in [`Table.entities`](Table) of an entity.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableRow(pub usize);

impl Table {
    const DEFAULT_CAPACITY: usize = 16;

    /// Creates a new table with at least the specified capacity.
    pub fn with_capacity(components: ComponentSet, capacity: usize) -> Self {
        let capacity = capacity.max(Self::DEFAULT_CAPACITY);
        let columns = components
            .slots()
            .map(|slot| {
                slot.map(|component| Column::with_capacity(component, capacity))
            })
            .collect();
        let entities = SparseMap::new();

        Self { components, entities, columns }
    }

    /// Returns a reference to the component set of this table.
    pub const fn components(&self) -> &ComponentSet {
        &self.components
    }

    /// Returns the entities in this table.
    pub fn entities(&self) -> SparseIter<'_, EntityId> {
        self.entities.iter()
    }

    /// Returns the amount of entities in this table.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Get the entity at the row.
    pub fn entity(&self, row: TableRow) -> Option<EntityId> {
        self.entities.get(&row).copied()
    }

    /// Pushes an entity to this table.
    ///
    /// # Safety
    ///
    /// The entity must not exist in the table. If it does, when the table is
    /// dropped it will drop each component twice.
    pub unsafe fn push(&mut self, entity: EntityId) -> TableRow {
        debug_assert!(
            !self.entities.iter().any(|&e| e == entity),
            "calling `Table::push` on an entity already contained within the \
             table causes undefined behavior",
        );

        let row = TableRow(self.entities.len());

        self.entities.insert(row, entity);

        row
    }

    /// Removes the entity at the given row.
    ///
    /// Does not drop components.
    pub fn remove(&mut self, row: TableRow) -> Option<EntityId> {
        self.entities.remove(&row)
    }

    /// Returns a pointer to a component of an entity.
    ///
    /// # Safety
    ///
    /// The table must contain the entity and the component.
    pub unsafe fn get_unchecked(
        &self,
        row: TableRow,
        component: ComponentId,
    ) -> NonNull<u8> {
        debug_assert!(self.components.contains(component));

        unsafe {
            self.columns.get(&component).unwrap_unchecked().get_unchecked(row)
        }
    }

    /// Returns a pointer to a component of an entity.
    ///
    /// # Safety
    ///
    /// The table must contain the entity and the component.
    pub unsafe fn get_unchecked_mut(
        &mut self,
        row: TableRow,
        component: ComponentId,
    ) -> NonNull<u8> {
        debug_assert!(self.components.contains(component));

        unsafe {
            self.columns
                .get_mut(&component)
                .unwrap_unchecked()
                .get_unchecked_mut(row)
        }
    }

    /// Writes a component value to an entity. The previous value is not read,
    /// so this can be used to initialize the component.
    ///
    /// Returns `Some` if the entity exists and contains the component.
    pub unsafe fn write<C: Component>(
        &mut self,
        row: TableRow,
        component: ComponentId,
        mut value: C,
    ) -> Option<()> {
        unsafe {
            self.write_ptr(row, component, NonNull::from(&mut value).cast())
                // this write has move semantics, so call `forget` to ensure
                // that `component` does not get dropped
                .inspect(|_| mem::forget(value))
        }
    }

    /// Copies the bytes of a component pointer to an entity. The previous value
    /// is not read, so this can be used to initialize the component.
    ///
    /// Returns `Some` if this table contains the component.
    ///
    /// # Safety
    ///
    /// The pointer must be a valid instance of the component the index refers
    /// to.
    pub unsafe fn write_ptr(
        &mut self,
        row: TableRow,
        component: ComponentId,
        value: NonNull<u8>,
    ) -> Option<()> {
        self.columns
            .get_mut(&component)
            .map(|column| unsafe { column.write(row, value) })
    }

    /// Replaces the previous component with a new value.
    ///
    /// # Safety
    ///
    /// The previous value must be initialized. The provided component index
    /// must refer to `C`.
    pub unsafe fn replace<C: Component>(
        &mut self,
        row: TableRow,
        component: ComponentId,
        value: C,
    ) -> C {
        unsafe {
            let prev = self.get_unchecked_mut(row, component).cast().read();

            self.write(row, component, value);

            prev
        }
    }

    /// Drops all the components of an entity at the row and removes it from the
    /// table.
    ///
    /// # Safety
    ///
    /// The table must contain the entity at the row.
    pub unsafe fn free(&mut self, row: TableRow) {
        // SAFETY: the caller ensures that this table contains the entity at the
        // provided row
        unsafe { self.entities.remove(&row).unwrap_unchecked() };

        for column in &mut self.columns {
            _ = unsafe { column.free(row) };
        }
    }

    /// Clears all data in this table.
    pub fn clear(&mut self) {
        for (i, entity) in self.entities.slots().enumerate() {
            if entity.is_none() {
                continue;
            }

            let row = TableRow(i);

            for column in &mut self.columns {
                _ = unsafe { column.free(row) };
            }
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        self.clear();
    }
}

impl SparseIndex for TableRow {
    fn sparse_index(&self) -> usize {
        self.0
    }
}
