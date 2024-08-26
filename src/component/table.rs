use std::mem;
use std::ptr::{self};

use super::Column;
use crate::{ComponentId, Entity, SparseIndex, SparseMap, SparseSet, TypeData, TypeSet};

#[derive(Debug)]
pub struct Table {
    header: TypeSet,
    entities: SparseSet<Entity>,
    columns: SparseMap<ComponentId, Column>,
}

impl Table {
    pub fn new(header: TypeSet) -> Self {
        let entities = SparseSet::new();
        let columns = header.slots().map(|slot| slot.map(Column::new)).collect();

        Self { header, entities, columns }
    }

    pub fn header(&self) -> &TypeSet {
        &self.header
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    pub fn reserve(&mut self, additional: usize) {
        for column in &mut self.columns {
            column.grow(additional);
        }
    }

    pub fn get<C: 'static>(&self, entity: Entity) -> Option<&C> {
        (self.contains(entity))
            .then(|| {
                self.columns
                    .get(&ComponentId::of::<C>())
                    // SAFETY: the component is live as this table contains the entity
                    .map(|column| unsafe { column.get_unchecked(entity.sparse_index()) })
                    // SAFETY: the pointer is guaranteed to be non-null and a valid `C`
                    .and_then(|ptr| unsafe { ptr.cast::<C>().as_ref() })
            })
            .flatten()
    }

    pub fn get_mut<C: 'static>(&mut self, entity: Entity) -> Option<&mut C> {
        (self.contains(entity))
            .then(|| {
                self.columns
                    .get_mut(&ComponentId::of::<C>())
                    // SAFETY: the component is live as this table contains the entity
                    .map(|column| unsafe {
                        column.get_unchecked_mut(entity.sparse_index())
                    })
                    // SAFETY: the pointer is guaranteed to be non-null and a valid `C`
                    .and_then(|ptr| unsafe { ptr.cast::<C>().as_mut() })
            })
            .flatten()
    }

    pub unsafe fn get_ptr_unchecked_mut(
        &mut self,
        entity: Entity,
        component: TypeData,
    ) -> *mut u8 {
        unsafe {
            self.columns
                .get_mut(&component.component_id())
                .unwrap_unchecked()
                .get_unchecked_mut(entity.sparse_index())
        }
    }

    pub fn replace<C: 'static>(&mut self, entity: Entity, component: C) -> Option<C> {
        self.entities.remove(&entity)?;

        let prev = unsafe {
            self.get_ptr_unchecked_mut(entity, TypeData::of::<C>()).cast::<C>().read()
        };

        self.write(entity, component);

        Some(prev)
    }

    /// Insert an entity into the table.
    pub fn insert(&mut self, entity: Entity) -> Option<()> {
        self.entities.insert(entity).map(|_| ())
    }

    /// Remove an entity. Does not drop its components.
    pub fn remove(&mut self, entity: Entity) -> Option<()> {
        self.entities.remove(&entity).map(|_| ())
    }

    /// Write to a component. If the entity didn't exist in the table, it is
    /// inserted if the entity contained the component.
    ///
    /// Returns `None` if the table doesn't contain the component.
    pub fn write<T: 'static>(&mut self, entity: Entity, mut component: T) -> Option<()> {
        unsafe {
            self.write_ptr(
                entity,
                TypeData::of::<T>(),
                ptr::from_mut(&mut component).cast(),
            )
            // if success, call forget so as to not call drop
            .inspect(|_| mem::forget(component))
        }
    }

    /// Write to a component. If the entity didn't exist in the table, it is
    /// inserted if the entity contained the component.
    ///
    /// Returns `None` if the table doesn't contain the component.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid instance of the type described by the
    /// [`TypeData`].
    pub unsafe fn write_ptr(
        &mut self,
        entity: Entity,
        type_data: TypeData,
        ptr: *mut u8,
    ) -> Option<()> {
        let column = self.columns.get_mut(&type_data.component_id())?;

        self.entities.insert(entity);
        unsafe { column.write(entity.sparse_index(), ptr) };

        Some(())
    }

    /// Drop all the components of an entity, removing it.
    ///
    /// Returns `Some` if the entity existed and was thus dropped.
    pub fn free(&mut self, entity: Entity) -> Option<()> {
        self.entities.remove(&entity)?;

        for column in &mut self.columns {
            unsafe { column.free(entity.sparse_index()) };
        }

        Some(())
    }

    /// Drop all components and remove all entities.
    pub fn clear(&mut self) {
        self.entities.clear();

        for column in &mut self.columns {
            for entity in &self.entities {
                unsafe { column.free(entity.sparse_index()) };
            }
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Entities;

    #[test]
    fn write_and_read_multiple_components() {
        struct Name(&'static str);
        struct Age(u32);

        let mut entities = Entities::new();
        let mut table = Table::new(TypeSet::new().with::<Name>().with::<Age>());

        assert!(table.is_empty());

        let [e0, e1] = [entities.alloc(), entities.alloc()];

        table.write(e0, Name("e0")).unwrap();
        table.write(e0, Age(123)).unwrap();

        assert_eq!(table.len(), 1);

        table.write(e1, Name("e1")).unwrap();
        table.write(e1, Age(321)).unwrap();

        assert_eq!(table.len(), 2);

        {
            assert_eq!(table.get::<Name>(e0).unwrap().0, "e0");
            assert_eq!(table.get::<Age>(e0).unwrap().0, 123);

            assert_eq!(table.get::<Name>(e1).unwrap().0, "e1");
            assert_eq!(table.get::<Age>(e1).unwrap().0, 321);
        }
    }

    /// Tests that [`Table::write`] has move semantics.
    #[test]
    fn write_doesnt_drop_component() {
        struct A;

        impl Drop for A {
            fn drop(&mut self) {
                panic!()
            }
        }

        let mut entities = Entities::new();
        let mut table = Table::new(TypeSet::new().with::<A>());

        table.write(entities.alloc(), A);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn large_columns() {
        struct A(#[allow(dead_code)] u32);
        struct B(#[allow(dead_code)] u64);

        let mut entities = Entities::new();
        let mut table = Table::new(TypeSet::new().with::<A>().with::<B>());

        entities.alloc_many(10000);

        for (i, entity) in entities.iter().enumerate() {
            table.insert(entity);
            table.write(entity, A(i as _)).unwrap();
            table.write(entity, B(i as _)).unwrap();
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn reserve_large_columns() {
        struct A(#[allow(dead_code)] u32);
        struct B(#[allow(dead_code)] u64);

        const COUNT: usize = 10000;

        let mut entities = Entities::new();
        let mut table = Table::new(TypeSet::new().with::<A>().with::<B>());

        table.reserve(COUNT);
        entities.alloc_many(COUNT);

        for (i, entity) in entities.iter().enumerate() {
            table.insert(entity);
            table.write(entity, A(i as _)).unwrap();
            table.write(entity, B(i as _)).unwrap();
        }
    }
}
