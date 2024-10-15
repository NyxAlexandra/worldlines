use std::iter::Enumerate;
use std::slice;

use thiserror::Error;

pub use self::ptr::*;
use crate::{
    array,
    Bundle,
    Components,
    Entities,
    EntitiesIter,
    EntitiesIterMut,
    Entity,
    EntityMut,
    EntityNotFound,
    EntityRef,
    EntitySlot,
    EntityWorld,
    Query,
    QueryData,
    QueryFilter,
    ReadOnlyQueryData,
    ReadOnlySystemInput,
    Res,
    ResMut,
    Resource,
    ResourceError,
    Resources,
    SystemInput,
    WorldAccess,
    WorldAccessError,
};

mod ptr;

/// The center of an ECS. Stores all entities, their components, and resources.
#[derive(Debug)]
pub struct World {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    pub(crate) resources: Resources,
}

/// An iterator over entities created in [`World::spawn_iter`].
#[derive(Clone)]
pub struct SpawnIter<'w> {
    inner: Enumerate<slice::Iter<'w, EntitySlot>>,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> Self {
        let entities = Entities::new();
        let components = Components::new();
        let resources = Resources::new();

        Self { entities, components, resources }
    }

    /// Borrow this world as a [`WorldPtr`].
    ///
    /// The pointer can be safely used for operations usable by `&World`.
    pub fn as_ptr(&self) -> WorldPtr<'_> {
        WorldPtr::from_ref(self)
    }

    /// Mutably borrow this world as a [`WorldPtr`].
    ///
    /// The pointer can be safely used for operations usable by `&mut World`.
    pub fn as_ptr_mut(&mut self) -> WorldPtr<'_> {
        WorldPtr::from_mut(self)
    }

    /// Despawn all entities and destroy all resources.
    pub fn clear(&mut self) {
        self.despawn_all();
        self.destroy_all();
    }

    // entities ---

    /// The amount of entities in this world.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` if there are no entities in this world.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if this world contains the entity.
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }

    /// Borrow an entity.
    pub fn entity(
        &self,
        entity: Entity,
    ) -> Result<EntityRef<'_>, EntityNotFound> {
        EntityRef::new(self, entity)
    }

    /// Mutably borrow an entity.
    pub fn entity_mut(
        &mut self,
        entity: Entity,
    ) -> Result<EntityMut<'_>, EntityNotFound> {
        EntityMut::new(self, entity)
    }

    /// Mutably borrow an entity and the world.
    pub fn entity_world(
        &mut self,
        entity: Entity,
    ) -> Result<EntityWorld<'_>, EntityNotFound> {
        EntityWorld::new(self, entity)
    }

    /// Mutably borrow multiple entities in a scope.
    pub fn entity_scope<const N: usize>(
        &mut self,
        entities: [Entity; N],
        f: impl FnOnce([EntityMut<'_>; N]),
    ) -> Result<(), EntityScopeError> {
        for (i, entity) in entities.iter().enumerate() {
            let Some(slice) = entities.get((i + 1)..) else {
                continue;
            };

            if slice.contains(entity) {
                return Err(EntityScopeError::EntityAliasing(*entity));
            }
        }

        let ptr = self.as_ptr_mut();

        f(array::try_map(entities, |entity| unsafe {
            EntityMut::new(ptr.as_mut(), entity)
        })
        .map_err(|EntityNotFound(entity)| {
            EntityScopeError::EntityNotFound(entity)
        })?);

        Ok(())
    }

    /// Return an iterator that borrows each entity in this world.
    pub fn entities(&self) -> EntitiesIter<'_> {
        EntitiesIter { world: self, ids: self.entities.iter() }
    }

    /// Return an iterator that mutably borrows each entity in this world.
    pub fn entities_mut(&mut self) -> EntitiesIterMut<'_> {
        EntitiesIterMut { world: self.as_ptr(), ids: self.entities.iter() }
    }

    /// Return a read-only [`Query`] of the entities of this world.
    pub fn query<D, F>(&self) -> Result<Query<'_, D, F>, WorldAccessError>
    where
        D: ReadOnlyQueryData,
        F: QueryFilter,
    {
        // SAFETY: the data is read-only
        unsafe { Query::new(self.as_ptr()) }
    }

    /// Return a [`Query`] of the entities of this world.
    pub fn query_mut<D, F>(
        &mut self,
    ) -> Result<Query<'_, D, F>, WorldAccessError>
    where
        D: QueryData,
        F: QueryFilter,
    {
        unsafe { Query::new(self.as_ptr_mut()) }
    }

    /// Spawn a new entity with a [`Bundle`] its components. Returns an
    /// [`EntityWorld`] which can be used to further mutate the entity.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> EntityWorld<'_> {
        let entity = self.entities.alloc();

        unsafe {
            let table = self.components.alloc(entity, bundle);

            self.entities.set(entity, table);
        }

        // SAFETY: the entity is alive
        unsafe { EntityWorld::new(self, entity).unwrap_unchecked() }
    }

    /// Spawn entities in bulk.
    pub fn spawn_iter<B: Bundle>(
        &mut self,
        iter: impl IntoIterator<Item = B>,
    ) -> SpawnIter<'_> {
        self.entities.flush();

        let bundles = iter.into_iter();

        let (lower, upper) = bundles.size_hint();
        let count = upper.unwrap_or(lower);

        let id = self.components.reserve::<B>(count);
        let table = unsafe { self.components.table_mut(id).unwrap_unchecked() };

        let mut allocated = self.entities.alloc_many(count);
        let start = allocated.start;

        for bundle in bundles {
            let index = allocated.next().unwrap_or_else(|| {
                let index = allocated.end;

                allocated.end += 1;
                self.entities.alloc_end();

                index
            });
            let entity = unsafe {
                Entity {
                    index: index as _,
                    version: self
                        .entities
                        .slot(index)
                        .unwrap_unchecked()
                        .version,
                }
            };

            self.entities.set(entity, id);
            table.insert(entity);
            bundle.take(|components| {
                for (component, ptr) in components {
                    unsafe { table.write_ptr(entity, component, ptr.as_ptr()) };
                }
            });
        }

        SpawnIter {
            inner: unsafe {
                self.entities
                    .slot(start..allocated.end)
                    .unwrap_unchecked()
                    .iter()
                    .enumerate()
            },
        }
    }

    /// Despawn an entity.
    pub fn despawn(&mut self, entity: Entity) -> Result<(), EntityNotFound> {
        let table = self.entities.free(entity).ok_or(EntityNotFound(entity))?;

        // SAFETY: the table id is guaranteed to be correct
        self.components.free(entity, table).ok_or_else(|| unreachable!())
    }

    /// Despawn all entities.
    pub fn despawn_all(&mut self) {
        self.entities.clear();
        self.components.clear();
    }

    // resources ---

    /// Returns `true` if this world contains the resource.
    #[doc(alias = "contains_resource")]
    pub fn has<R: Resource>(&self) -> bool {
        self.resources.contains::<R>()
    }

    /// Borrow a resource.
    pub fn resource<R: Resource>(&self) -> Result<Res<'_, R>, ResourceError> {
        self.resources.get()
    }

    /// Mutably borrow a resource.
    pub fn resource_mut<R: Resource>(
        &self,
    ) -> Result<ResMut<'_, R>, ResourceError> {
        self.resources.get_mut()
    }

    /// Create a new resource.
    #[doc(alias = "insert_resource")]
    pub fn create<R: Resource>(&mut self, resource: R) {
        self.resources.insert(resource);
    }

    /// Remove a resource.
    #[doc(alias = "remove_resource")]
    pub fn destroy<R: Resource>(&mut self) -> Result<R, ResourceError> {
        self.resources.remove()
    }

    /// Remove all resources.
    #[doc(alias = "clear_resources")]
    pub fn destroy_all(&mut self) {
        self.resources.clear();
    }
}

/// Error when calling [`World::entity_scope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum EntityScopeError {
    #[error("entity {0:?} passed to `World::entity_scope` multiple times")]
    EntityAliasing(Entity),
    #[error("entity {0:?} not found")]
    EntityNotFound(Entity),
}

// SAFETY: all resources and components are `Send + Sync`
unsafe impl Send for World {}
// SAFETY: all resources and components are `Send + Sync`
unsafe impl Sync for World {}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl SystemInput for &World {
    type Output<'w, 's> = &'w World;
    type State = ();

    fn access(access: &mut WorldAccess) {
        access.world();
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { world.as_ref() }
    }
}

unsafe impl ReadOnlySystemInput for &World {}

unsafe impl SystemInput for &mut World {
    type Output<'w, 's> = &'w mut World;
    type State = ();

    fn access(access: &mut WorldAccess) {
        access.world_mut();
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { world.as_mut() }
    }
}

impl Iterator for SpawnIter<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, slot)| Entity {
            index: index as _,
            version: slot.version,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for SpawnIter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn spawn_many() {
        struct A(#[allow(dead_code)] u32);
        struct B(#[allow(dead_code)] u64);

        let mut world = World::new();

        for _ in 0..1000 {
            world.spawn((A(123), B(321)));
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn spawn_big_iter() {
        struct A(#[allow(dead_code)] u32);
        struct B(#[allow(dead_code)] u64);

        let mut world = World::new();

        world.spawn_iter((0..10000).map(|_| (A(123), B(321))));
    }

    #[test]
    fn entity_scope() {
        let mut world = World::new();

        let e0 = world.spawn(()).id();
        let e1 = world.spawn(()).id();
        let e2 = world.spawn(()).id();

        println!("{:#?}", world);

        world.entity_scope([e0, e1, e2], |_| {}).unwrap();

        assert_eq!(
            world.entity_scope([e0, e1, e0], |_| {}),
            Err(EntityScopeError::EntityAliasing(e0)),
        );

        let despawned = world.spawn(()).id();

        world.despawn(despawned).unwrap();

        assert_eq!(
            world.entity_scope([despawned], |_| {}),
            Err(EntityScopeError::EntityNotFound(despawned)),
        );

        assert_eq!(
            world.entity_scope([despawned, e0, e0], |_| {}),
            Err(EntityScopeError::EntityAliasing(e0)),
            "aliasing should be checked before liveness",
        );
    }

    #[test]
    fn spawn_iter() {
        let mut world = World::new();

        world.spawn_iter([("e0", 0), ("e1", 1), ("e2", 2)]);

        println!("{:#?}", world);

        let mut iter = world.query::<(&&str, &i32), ()>().unwrap();

        assert_eq!(iter.next(), Some((&"e0", &0)));
        assert_eq!(iter.next(), Some((&"e1", &1)));
        assert_eq!(iter.next(), Some((&"e2", &2)));
        assert!(iter.next().is_none());
    }

    #[test]
    fn query_mut() {
        struct Human;
        struct Goblin;
        struct Hp(u32);
        struct Poisoned(u32);

        let mut world = World::new();

        let healthy = world.spawn((Human, Hp(10))).id();
        let poisoned = world.spawn((Human, Hp(15), Poisoned(3))).id();
        let goblin = world.spawn((Goblin, Hp(5), Poisoned(1))).id();

        for (Hp(ref mut hp), Poisoned(dmg)) in
            world.query_mut::<(&mut Hp, &Poisoned), ()>().unwrap()
        {
            *hp -= dmg;
        }

        assert_eq!(world.entity(healthy).unwrap().get::<Hp>().unwrap().0, 10);
        assert_eq!(world.entity(poisoned).unwrap().get::<Hp>().unwrap().0, 12);
        assert_eq!(world.entity(goblin).unwrap().get::<Hp>().unwrap().0, 4);
    }
}
