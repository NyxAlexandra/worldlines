use std::any::TypeId;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::mem::MaybeUninit;
use std::sync::{RwLock, RwLockWriteGuard};

use indexmap::IndexMap;

use super::{
    Bundle,
    Component,
    ComponentInfo,
    ComponentSet,
    ComponentSetBuilder,
};
use crate::entity::{EntityAddr, EntityId};
use crate::storage::{SparseIndex, Table, TableRow, TypeIdHasher, TypeMap};

/// Storage for all components.
#[derive(Debug)]
pub struct Components {
    info: RwLock<ComponentRegistry>,
    bundle_indices: TypeMap<TableIndex>,
    set_indices: HashMap<ComponentSet, TableIndex>,
    tables: Vec<Table>,
}

// TODO: optimize

/// The type used to store component info.
pub type ComponentRegistry =
    IndexMap<TypeId, ComponentInfo, BuildHasherDefault<TypeIdHasher>>;

/// Newtype for the index of a table in [`Components`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableIndex(usize);

impl Components {
    /// Default table capacity.
    const DEFAULT_TABLES: usize = 16;

    /// Creates empty component storage.
    pub fn new() -> Self {
        let info = RwLock::default();
        let bundle_indices = TypeMap::default();
        let set_indices = HashMap::with_capacity(Self::DEFAULT_TABLES);
        let tables = Vec::with_capacity(Self::DEFAULT_TABLES);

        Self { info, bundle_indices, set_indices, tables }
    }

    /// Returns a mutable reference to the component info registry.
    pub fn registry(&self) -> RwLockWriteGuard<'_, ComponentRegistry> {
        self.info.write().unwrap()
    }

    /// Returns the component index for a component [`TypeId`].
    ///
    /// Returns `None` if the component hasn't been registered yet.
    pub fn info_of_id(&self, type_id: TypeId) -> Option<ComponentInfo> {
        self.info.read().unwrap().get(&type_id).copied()
    }

    /// Registers a component, returning its info.
    pub fn register<C: Component>(&self) -> ComponentInfo {
        let mut registry = self.registry();
        let next = ComponentInfo::of::<C>(registry.len());

        *registry.entry(TypeId::of::<C>()).or_insert(next)
    }

    /// Returns a reference to the table with the given index.
    ///
    /// # Safety
    ///
    /// The table index must refer to an allocated table.
    pub unsafe fn get_unchecked(&self, index: TableIndex) -> &Table {
        unsafe { self.tables.get_unchecked(index.0) }
    }

    /// Returns a mutable reference to the table with the given index.
    ///
    /// # Safety
    ///
    /// The table index must refer to an allocated table.
    pub unsafe fn get_unchecked_mut(
        &mut self,
        index: TableIndex,
    ) -> &mut Table {
        unsafe { self.tables.get_unchecked_mut(index.0) }
    }

    /// Returns an iterator over the tables in storage.
    pub fn tables(
        &self,
    ) -> impl ExactSizeIterator<Item = (TableIndex, &Table)> {
        self.tables.iter().enumerate().map(|(i, table)| (TableIndex(i), table))
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
                let mut builder =
                    ComponentSetBuilder::new(self.info.get_mut().unwrap());

                B::components(&mut builder);

                let components = builder.build();
                let table =
                    self.set_indices.get(&components).copied().unwrap_or_else(
                        || {
                            let table = TableIndex(self.tables.len());

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
        let next = TableIndex(self.tables.len());
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
            let component = component.index();

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

impl SparseIndex for TableIndex {
    fn sparse_index(&self) -> usize {
        self.0
    }
}
