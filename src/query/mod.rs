//! Queries of components in a world.

use std::any::type_name;
use std::marker::PhantomData;

use thiserror::Error;

use crate::access::{AccessError, Level, WorldAccess, WorldAccessBuilder};
use crate::entity::{EntityAddr, EntityId, EntityMut, EntityPtr, EntityRef};
use crate::prelude::{Component, TableIndex};
use crate::storage::{SparseIter, SparseSet, TableRow};
use crate::system::{ReadOnlySystemInput, SystemInput};
use crate::world::{World, WorldPtr};

mod tuple_impl;

/// A query of components of a world.
pub struct Query<'w, D: QueryData> {
    world: WorldPtr<'w>,
    /// Tables that this query matches.
    tables: SparseSet<TableIndex>,
    _marker: PhantomData<D>,
}

/// An iterator over data of a query.
pub struct QueryIter<'w, 's, D: QueryData> {
    world: WorldPtr<'w>,
    tables: SparseIter<'s, TableIndex>,
    /// The amount of matched entities left.
    len: usize,
    /// The current table.
    table: Option<TableIndex>,
    /// The current row in the table.
    row: TableRow,
    _marker: PhantomData<D>,
}

/// Trait for the data that can be retreived from an entity.
///
/// # Safety
///
/// [`QueryData::get`] must only access data set in [`QueryData::access`].
pub unsafe trait QueryData {
    /// The type of the output data.
    type Output<'w>;

    /// Adds the access of this query data to the set.
    ///
    /// Used to ensure that the query accesses the world safely and correctly.
    fn access(builder: &mut WorldAccessBuilder<'_>);

    /// Returns the query output for an entity.
    ///
    /// # Safety
    ///
    /// The access of this query data must have been validated. The entity
    /// pointer must be valid for the described access. All components
    /// that are required by [`QueryData::access`] must be present in the
    /// entity.
    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_>;
}

/// Trait for query data that doesn't need mutable access to components.
///
/// # Safety
///
/// The query data implementation must declare only read access and must never
/// mutate entities.
pub unsafe trait ReadOnlyQueryData: QueryData {}

// TODO: consider collapsing `EntityNotFound` into `Mismatch`

/// Error when accessing queried data.
#[derive(Debug, Clone, Copy, Error)]
pub enum QueryGetError {
    /// Error when the entity doesn't exist in the world.
    #[error("entity not found: {0:?}")]
    EntityNotFound(EntityId),
    /// Error when the entity exists but doesn't match the query.
    #[error("entity {entity:?} does not match the query {data}")]
    Mismatch {
        entity: EntityId,
        /// The type name of the query data.
        data: &'static str,
    },
}

impl<'w, D: QueryData> Query<'w, D> {
    /// Creates a new query.
    ///
    /// Returns an error if the query access is invalid.
    ///
    /// # Safety
    ///
    /// The world pointer must be valid for this query's access.
    pub unsafe fn new(world: WorldPtr<'w>) -> Result<Self, AccessError> {
        // SAFETY: access to world metadata is always valid
        let mut builder = WorldAccess::builder(unsafe { world.as_ref() });

        D::access(&mut builder);

        let access = builder.build();

        access.result().map(|_| {
            // TODO: optimize

            let mut tables = SparseSet::new();

            // SAFETY: access to world metadata is always valid
            for (index, table) in unsafe { world.as_ref().components.tables() }
            {
                if access.matches(table.components()) {
                    tables.insert(index);
                }
            }

            Self { world, tables, _marker: PhantomData }
        })
    }

    /// Creates a new query from a world reference.
    ///
    /// Returns an error if the query access is invalid.
    ///
    /// The query data must implement [`ReadOnlyQueryData`].
    pub fn from_ref(world: &'w World) -> Result<Self, AccessError>
    where
        D: ReadOnlyQueryData,
    {
        // SAFETY: the world must be valid as it's a reference
        unsafe { Self::new(world.as_ptr()) }
    }

    /// Creates a new query from a mutable world reference.
    ///
    /// Returns an error if the query access is invalid.
    pub fn from_mut(world: &'w mut World) -> Result<Self, AccessError> {
        // SAFETY: the world must be valid as it's a reference
        unsafe { Self::new(world.as_ptr_mut()) }
    }

    /// Returns the amount of entities matched by this query.
    pub fn len(&self) -> usize {
        self.tables
            .iter()
            .copied()
            // SAFETY: reads to ECS metadata should always be valid
            .map(|table| unsafe {
                self.world.as_ref().components.get_unchecked(table).len()
            })
            .sum()
    }

    /// Returns `true` if this query matched no entities.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if this query matches the entity.
    pub fn contains(&self, entity: EntityId) -> bool {
        let Some(addr) = (unsafe { self.world.as_ref().entities.get(entity) })
        else {
            return false;
        };

        self.tables.contains(&addr.table)
    }

    /// Gets the query data for a particular entity.
    ///
    /// The query data must implement [`ReadOnlyQueryData`].
    pub fn get(&self, entity: EntityId) -> Result<D::Output<'_>, QueryGetError>
    where
        D: ReadOnlyQueryData,
    {
        let addr = self
            .addr_of(entity)
            .ok_or(QueryGetError::EntityNotFound(entity))?;

        if self.tables.contains(&addr.table) {
            // SAFETY: the entity matches the query
            Ok(unsafe { D::get(self.world.entity(entity)) })
        } else {
            Err(QueryGetError::Mismatch { entity, data: type_name::<D>() })
        }
    }

    /// Gets the query data for a particular entity.
    pub fn get_mut(
        &mut self,
        entity: EntityId,
    ) -> Result<D::Output<'_>, QueryGetError> {
        let addr = self
            .addr_of(entity)
            .ok_or(QueryGetError::EntityNotFound(entity))?;

        if self.tables.contains(&addr.table) {
            // SAFETY: the entity matches the query
            Ok(unsafe { D::get(self.world.entity(entity)) })
        } else {
            Err(QueryGetError::Mismatch { entity, data: type_name::<D>() })
        }
    }

    fn addr_of(&self, entity: EntityId) -> Option<EntityAddr> {
        unsafe { self.world.as_ref().entities.get(entity) }
    }

    /// Returns an iterator over query data.
    ///
    /// The query data must implement [`ReadOnlyQueryData`].
    pub fn iter(&self) -> QueryIter<'w, '_, D>
    where
        D: ReadOnlyQueryData,
    {
        QueryIter {
            world: self.world,
            len: self.len(),
            tables: self.tables.iter(),
            table: None,
            row: TableRow(0),
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over query data.
    pub fn iter_mut(&mut self) -> QueryIter<'w, '_, D> {
        QueryIter {
            world: self.world,
            tables: self.tables.iter(),
            len: self.len(),
            table: None,
            row: TableRow(0),
            _marker: PhantomData,
        }
    }
}

/// # Safety
///
/// The query only accesses the world as its data does, which implementors
/// ensure perform only valid access.
unsafe impl<D: QueryData> SystemInput for Query<'_, D> {
    type Output<'w, 's> = Query<'w, D>;
    // TODO: cache matched tables
    type State = ();

    fn init(_world: &World) -> Self::State {}

    fn access(_state: &Self::State, builder: &mut WorldAccessBuilder<'_>) {
        D::access(builder);
    }

    unsafe fn get<'w, 's>(
        _state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        // SAFETY: the caller ensures that the access is valid
        unsafe { Query::new(world).unwrap_unchecked() }
    }
}

/// # Safety
///
/// The query only accesses the world as its data does, which implementors
/// ensure perform only read-only access.
unsafe impl<D: ReadOnlyQueryData> ReadOnlySystemInput for Query<'_, D> {}

impl<'w, 's, D: ReadOnlyQueryData> IntoIterator for &'s Query<'w, D> {
    type IntoIter = QueryIter<'w, 's, D>;
    type Item = D::Output<'w>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, 's, D: QueryData> IntoIterator for &'s mut Query<'w, D> {
    type IntoIter = QueryIter<'w, 's, D>;
    type Item = D::Output<'w>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'w, 's, D: QueryData> Iterator for QueryIter<'w, 's, D> {
    type Item = D::Output<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            return None;
        }

        let table = if let Some(table) = self.table {
            table
        } else {
            *self.table.get_or_insert(*self.tables.next()?)
        };
        let entity = unsafe {
            let table = self.world.as_ref().components.get_unchecked(table);

            table.entity(self.row).or_else(|| table.entities().next().copied())
        };

        if let Some(entity) = entity {
            self.len -= 1;

            Some(unsafe { D::get(self.world.entity(entity)) })
        } else if entity.is_none() && self.tables.len() != 0 {
            self.table = None;
            self.row = TableRow(0);

            self.next()
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'w, 's, D: QueryData> ExactSizeIterator for QueryIter<'w, 's, D> {}

/// # Safety
///
/// The access declares that it borrows `C`.
unsafe impl<C: Component> QueryData for &C {
    type Output<'w> = &'w C;

    fn access(builder: &mut WorldAccessBuilder<'_>) {
        builder.borrows_component::<C>(Level::Read);
    }

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        // SAFETY: the caller ensures that the entity contains `C` and that the
        // entity pointer is valid for reads to `C`
        unsafe { entity.get_unchecked() }
    }
}

/// # Safety
///
/// The access declares that it immutably borrows `C`.
unsafe impl<C: Component> ReadOnlyQueryData for &C {}

/// # Safety
///
/// The access declares that it mutable borrows `C`.
unsafe impl<C: Component> QueryData for &mut C {
    type Output<'w> = &'w mut C;

    fn access(builder: &mut WorldAccessBuilder<'_>) {
        builder.borrows_component::<C>(Level::Write);
    }

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        // SAFETY: the caller ensures that the entity contains `C` and that the
        // entity pointer is valid for reads/writes to `C`
        unsafe { entity.get_unchecked_mut() }
    }
}

/// # Safety
///
/// The access declares that it immutably borrows `C`.
unsafe impl<C: Component> QueryData for Option<&C> {
    type Output<'w> = Option<&'w C>;

    fn access(builder: &mut WorldAccessBuilder<'_>) {
        builder.maybe_borrows_component::<C>(Level::Read);
    }

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        unsafe { entity.get().ok() }
    }
}

/// # Safety
///
/// The access declares that it immutably borrows `C`.
unsafe impl<C: Component> ReadOnlyQueryData for Option<&C> {}

/// # Safety
///
/// The access declares that it mutably borrows `C`.
unsafe impl<C: Component> QueryData for Option<&mut C> {
    type Output<'w> = Option<&'w mut C>;

    fn access(builder: &mut WorldAccessBuilder<'_>) {
        builder.maybe_borrows_component::<C>(Level::Write);
    }

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        unsafe { entity.get_mut().ok() }
    }
}

/// # Safety
///
/// Nothing is accessed.
unsafe impl QueryData for EntityId {
    type Output<'w> = Self;

    fn access(_builder: &mut WorldAccessBuilder<'_>) {}

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        entity.id()
    }
}

/// # Safety
///
/// Nothing is accessed.
unsafe impl ReadOnlyQueryData for EntityId {}

/// # Safety
///
/// The access declares that it immutable borrows all components.
unsafe impl QueryData for EntityRef<'_> {
    type Output<'w> = EntityRef<'w>;

    fn access(builder: &mut WorldAccessBuilder<'_>) {
        builder.borrows_all_entities(Level::Read);
    }

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        unsafe { entity.as_ref() }
    }
}

/// # Safety
///
/// The access declares that it immutably borrows all components.
unsafe impl ReadOnlyQueryData for EntityRef<'_> {}

/// # Safety
///
/// The access declares that it mutably borrows all components.
unsafe impl QueryData for EntityMut<'_> {
    type Output<'w> = EntityMut<'w>;

    fn access(builder: &mut WorldAccessBuilder<'_>) {
        builder.borrows_all_entities(Level::Read);
    }

    unsafe fn get(entity: EntityPtr<'_>) -> Self::Output<'_> {
        unsafe { entity.as_mut() }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[derive(Component)]
    struct Human;

    #[derive(Component)]
    struct LaCreatura;

    #[derive(Component)]
    struct Caterpillar;

    #[derive(Component)]
    struct Butterfly;

    #[derive(Component)]
    struct Hp(usize);

    #[test]
    fn query_iter() {
        let mut world = World::new();

        let human = world.spawn((Human, Hp(24))).id();
        let la_creatura = world.spawn((LaCreatura, Hp(128))).id();
        let butterfly = {
            let mut caterpillar = world.spawn((Caterpillar, Hp(1)));

            caterpillar.remove::<Caterpillar>().unwrap();
            caterpillar.insert(Butterfly);
            caterpillar.insert(Hp(3));

            caterpillar.id()
        };

        let query = world.query::<(EntityId, &Hp)>().unwrap();

        assert_eq!(query.len(), 3);

        for (entity, _) in &query {
            assert!([human, la_creatura, butterfly].contains(&entity));
        }
    }

    #[test]
    fn query_get() {
        let mut world = World::new();

        let human = world.spawn((Human, Hp(24))).id();
        let la_creatura = world.spawn((LaCreatura, Hp(128))).id();

        let query = world.query_mut::<(EntityId, &Hp)>().unwrap();

        {
            let (entity, hp) = query.get(human).unwrap();

            assert_eq!(entity, human);
            assert_eq!(hp.0, 24);
        }

        {
            let (entity, hp) = query.get(la_creatura).unwrap();

            assert_eq!(entity, la_creatura);
            assert_eq!(hp.0, 128);
        }
    }
}
