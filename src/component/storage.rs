use std::any::TypeId;
use std::collections::HashMap;
use std::mem::MaybeUninit;

use super::{Bundle, ComponentSet};
use crate::entity::{EntityAddr, EntityId};
use crate::prelude::ComponentVTable;
use crate::storage::{SparseIndex, Table, TableRow, TypeIdHasher};

/// Storage for all components.
#[derive(Debug)]
pub struct Components {
    bundle_indices: HashMap<TypeId, TableId, TypeIdHasher>,
    set_indices: HashMap<ComponentSet, TableId>,
    tables: Vec<Table>,
}

/// Newtype for the index of a table in [`Components`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableId(usize);

impl Components {
    /// Default table capacity.
    const DEFAULT_TABLES: usize = 16;

    /// Creates empty component storage.
    pub fn new() -> Self {
        let bundle_indices = HashMap::default();
        let set_indices = HashMap::with_capacity(Self::DEFAULT_TABLES);
        let tables = Vec::with_capacity(Self::DEFAULT_TABLES);

        Self { bundle_indices, set_indices, tables }
    }

    /// Returns a reference to the table with the given index.
    ///
    /// # Safety
    ///
    /// The table index must refer to an allocated table.
    pub unsafe fn get_unchecked(&self, index: TableId) -> &Table {
        unsafe { self.tables.get_unchecked(index.0) }
    }

    /// Returns a mutable reference to the table with the given index.
    ///
    /// # Safety
    ///
    /// The table index must refer to an allocated table.
    pub unsafe fn get_unchecked_mut(&mut self, index: TableId) -> &mut Table {
        unsafe { self.tables.get_unchecked_mut(index.0) }
    }

    /// Returns an iterator over the tables in storage.
    pub fn tables(&self) -> impl ExactSizeIterator<Item = (TableId, &Table)> {
        self.tables.iter().enumerate().map(|(i, table)| (TableId(i), table))
    }

    /// Returns the table for the specified bundle.
    ///
    /// Will allocate a new table if one for that bundle didn't already exist.
    pub fn alloc<B: Bundle>(&mut self, count: usize) -> EntityAddr {
        let table = self
            .bundle_indices
            .get(&TypeId::of::<B>())
            .copied()
            .unwrap_or_else(|| {
                let mut components = ComponentSet::new();

                B::components(&mut components);

                let table =
                    self.set_indices.get(&components).copied().unwrap_or_else(
                        || {
                            let table = TableId(self.tables.len());

                            self.set_indices.insert(components.clone(), table);
                            self.tables
                                .push(Table::with_capacity(components, count));

                            table
                        },
                    );

                self.bundle_indices.insert(TypeId::of::<B>(), table);

                table
            });
        let row = {
            let table = unsafe { self.get_unchecked_mut(table) };

            TableRow(table.entities().len())
        };

        EntityAddr { table, row }
    }

    /// Returns the table for the given component set.
    ///
    /// Will allocate a new table if one didn't already exist.
    pub fn alloc_set(
        &mut self,
        count: usize,
        components: ComponentSet,
    ) -> EntityAddr {
        let next = TableId(self.tables.len());
        let table =
            self.set_indices.get(&components).copied().unwrap_or_else(|| {
                self.set_indices.insert(components.clone(), next);
                self.tables.push(Table::with_capacity(components, count));

                next
            });
        let row = {
            let table = unsafe { self.get_unchecked_mut(table) };

            TableRow(table.entities().len())
        };

        EntityAddr { table, row }
    }

    /// Reallocates an entity from one table to another.
    ///
    /// This will copy over all components that are in both tables. Components
    /// that aren't moved are not dropped.
    ///
    /// # Safety
    ///
    /// The entity must be contained in the table and its components must be
    /// initialized.
    #[must_use = "the address must be used to set the correct `EntityAddr` in \
                  `Entities`"]
    pub unsafe fn realloc(
        &mut self,
        entity: EntityId,
        old_addr: EntityAddr,
        components: ComponentSet,
    ) -> EntityAddr {
        debug_assert!(old_addr.table.0 < self.tables.len());

        let new_addr = self.alloc_set(1, components);

        debug_assert_ne!(
            old_addr, new_addr,
            "cannot reallocate an entity to its own table",
        );

        let [old_table, new_table] = unsafe {
            get_many_unchecked_mut(
                &mut self.tables,
                [old_addr.table.0, new_addr.table.0],
            )
        };

        old_table.remove(old_addr.row);
        unsafe { new_table.push(entity) };

        let intersection =
            old_table.components().intersection(new_table.components());

        for component in intersection.iter() {
            let component = component.id();

            unsafe {
                let ptr = old_table.get_unchecked_mut(old_addr.row, component);

                new_table.write_ptr(new_addr.row, component, ptr);
            }
        }

        new_addr
    }

    /// Clears all tables in storage.
    pub fn clear(&mut self) {
        for table in &mut self.tables {
            table.clear();
        }
    }
}

pub unsafe fn get_many_unchecked_mut<T, const N: usize>(
    this: &mut [T],
    indices: [usize; N],
) -> [&mut T; N] {
    // adapted from the standard library

    let slice: *mut [T] = this;
    let mut arr: MaybeUninit<[&mut T; N]> = MaybeUninit::uninit();
    let arr_ptr = arr.as_mut_ptr();

    unsafe {
        for i in 0..N {
            let index = *indices.get_unchecked(i);

            *get_unchecked_mut(arr_ptr, i) =
                &mut *get_unchecked_mut(slice, index);
        }
        arr.assume_init()
    }
}

unsafe fn get_unchecked_mut<T>(this: *mut [T], index: usize) -> *mut T {
    let ptr: *mut T = this as _;

    unsafe { ptr.add(index) }
}

impl SparseIndex for TableId {
    fn sparse_index(&self) -> usize {
        self.0
    }
}
