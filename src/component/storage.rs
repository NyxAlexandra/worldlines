use std::any::TypeId;
use std::collections::HashMap;

use super::Table;
use crate::{array, Bundle, Entity, TypeData, TypeMap, TypeSet};

/// Stores the components of entities in a [`World`](crate::World).
#[derive(Debug)]
pub struct Components {
    bundles: TypeMap<TableId>,
    type_sets: HashMap<TypeSet, TableId>,
    // not sparse as `TableId` is there is only one instance per `World`
    tables: Vec<Table>,
}

/// An identifier for a table.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableId(usize);

impl Components {
    pub fn new() -> Self {
        let bundles = TypeMap::default();
        let type_sets = HashMap::default();
        let tables = Vec::new();

        Self { bundles, type_sets, tables }
    }

    pub fn table(&self, table: TableId) -> Option<&Table> {
        self.tables.get(table.0)
    }

    pub fn table_mut(&mut self, table: TableId) -> Option<&mut Table> {
        self.tables.get_mut(table.0)
    }

    /// Reserve space for an amount of new entities containing bundle `B`.
    pub fn reserve<B: Bundle>(&mut self, additional: usize) -> TableId {
        let id = self.bundles.get(&TypeId::of::<B>()).copied().unwrap_or_else(
            || {
                let header = B::types();

                self.type_sets.get(&header).copied().unwrap_or_else(|| {
                    let table = Table::new(header.clone());
                    let id = TableId(self.tables.len());

                    self.bundles.insert(TypeId::of::<B>(), id);
                    self.type_sets.insert(header, id);
                    self.tables.push(table);

                    id
                })
            },
        );
        let table = unsafe { self.table_mut(id).unwrap_unchecked() };

        table.reserve(additional);

        id
    }

    /// Allocate an entity in the table for a bundle.
    ///
    /// # Safety
    ///
    /// The entity must not have already been allocated.
    pub unsafe fn alloc<B: Bundle>(&mut self, entity: Entity) -> TableId {
        let id = self.reserve::<B>(1);
        let table = unsafe { self.table_mut(id).unwrap_unchecked() };

        table.insert(entity);

        id
    }

    /// Reallocate the components of an entity to another table. `init` is
    /// called on the new table to initialize new components (if they
    /// exist).
    ///
    /// Components that are shared between the old and the new table are copied.
    /// Components that aren't are dropped if `drop` is set.
    ///
    /// Returns `None` if the entity isn't in `old_table` or if the new header
    /// is equivalent to the old one.
    ///
    /// # Safety
    ///
    /// `init` must initialize new components, if necessary.
    pub unsafe fn realloc(
        &mut self,
        entity: Entity,
        old_table: TableId,
        new_header: TypeSet,
        drop: bool,
        init: impl FnOnce(&mut Table),
    ) -> Option<TableId> {
        {
            let table = self.table(old_table)?;

            if !table.contains(entity) || table.header() == &new_header {
                return None;
            }
        }

        let new_table =
            self.type_sets.get(&new_header).copied().unwrap_or_else(|| {
                let new_table = TableId(self.tables.len());

                self.type_sets.insert(new_header.clone(), new_table);
                self.tables.push(Table::new(new_header.clone()));

                new_table
            });

        if old_table == new_table {
            return None;
        }

        {
            let [old_table, new_table] = unsafe {
                array::get_many_unchecked_mut(
                    &mut self.tables,
                    [old_table.0, new_table.0],
                )
            };

            let intersection = old_table.header().intersection(&new_header);

            // move existing components
            for component in &intersection {
                unsafe {
                    let ptr =
                        old_table.get_ptr_unchecked_mut(entity, component);

                    new_table.insert(entity);
                    new_table.write_ptr(entity, component, ptr);
                    old_table.remove(entity);
                }
            }

            // drop components that weren't moved
            if drop {
                let difference = old_table.header().difference(&new_header);

                for component in &difference {
                    unsafe {
                        let ptr =
                            old_table.get_ptr_unchecked_mut(entity, component);

                        component.drop()(ptr);
                    }
                }
            }

            if new_header.len() > old_table.header().len() {
                init(new_table);
            }
        }

        Some(new_table)
    }

    /// Reallocate the components of an entity to another table. `init` is
    /// called to initialize the new component.
    ///
    /// The component is dropped if `drop` is set.
    ///
    /// Returns `None` if `old_table` doesn't contain the entity.
    ///
    /// # Safety
    ///
    /// `init` must only initialize the component that is added.
    pub unsafe fn realloc_with(
        &mut self,
        entity: Entity,
        old_table: TableId,
        component: TypeData,
        drop: bool,
        init: impl FnOnce(&mut Table),
    ) -> Option<TableId> {
        let new_header =
            self.table(old_table)?.header().clone().with_type_data(component);

        unsafe { self.realloc(entity, old_table, new_header, drop, init) }
    }

    /// Reallocate the components of an entity to another table. `init` is
    /// called to initialize the new component.
    ///
    /// The component is dropped if `drop` is set.
    ///
    /// Returns `None` if `old_table` doesn't contain the entity.
    pub fn realloc_without(
        &mut self,
        entity: Entity,
        old_table: TableId,
        component: TypeData,
        drop: bool,
    ) -> Option<TableId> {
        let new_header = self
            .table(old_table)?
            .header()
            .clone()
            .without_type_data(component);

        unsafe {
            self.realloc(
                entity,
                old_table,
                new_header,
                drop,
                |_| unreachable!(),
            )
        }
    }

    /// Drop the components of an entity.
    pub fn free(&mut self, entity: Entity, table: TableId) -> Option<()> {
        self.table_mut(table)?.free(entity)
    }

    /// Drop all components, but not the tables containing them.
    pub fn clear(&mut self) {
        // TODO: determine the semantics of clearing

        for table in &mut self.tables {
            table.clear();
        }
    }
}
