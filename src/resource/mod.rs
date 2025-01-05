//! Unique values in the ECS.

use std::ops::{Deref, DerefMut};

use atomic_refcell::{AtomicRef, AtomicRefMut};
use thiserror::Error;
pub use worldlines_macros::Resource;

pub use self::info::*;
pub(crate) use self::storage::*;
use crate::access::{Level, WorldAccess};
use crate::prelude::{World, WorldPtr};
use crate::system::{ReadOnlySystemInput, SystemInput};

mod info;
mod storage;

/// Trait for unique ECS values.
///
/// A world can have at most 1 instance of a resource.
///
/// # Safety
///
/// The implementation of [`Resource::id`] must use a static
/// [`ResourceIdCell`] to store the id. The implementation must only create a
/// [`ResourceIdCell`] for `Self`.
///
/// ```
/// # use worldlines::prelude::*;
/// #
/// struct A;
///
/// unsafe impl Resource for A {
///     fn id() -> ResourceId {
///         static ID: ResourceIdCell<A> = ResourceIdCell::new();
///
///         ID.get_or_init()
///     }
/// }
/// ```
pub unsafe trait Resource: Send + Sync + 'static {
    /// Returns the id of this resource.
    fn id() -> ResourceId;
}

/// A reference to a [resource](Resource) in a world.
pub struct Res<'w, R: Resource> {
    inner: AtomicRef<'w, R>,
}

/// A mutable reference to a [resource](Resource) in a world.
pub struct ResMut<'w, R: Resource> {
    inner: AtomicRefMut<'w, R>,
}

/// Error for when a resource wasn't found.
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("resource not found: {0}")]
    NotFound(&'static str),
    #[error("resource already borrowed: {0}")]
    AlreadyBorrowed(&'static str),
}

impl<'w, R: Resource> Res<'w, R> {
    fn new(inner: AtomicRef<'w, R>) -> Self {
        Self { inner }
    }

    /// Clone this reference.
    ///
    /// This is an assosciated reference so as to not interfere with
    /// dereferencing.
    #[expect(clippy::should_implement_trait)]
    pub fn clone(this: &Self) -> Self {
        Self { inner: AtomicRef::clone(&this.inner) }
    }

    /// Map this reference `R -> R_`.
    ///
    /// This is usually used to borrow a field of `R`.
    pub fn map<R_: Resource>(
        this: Self,
        f: impl FnOnce(&R) -> &R_,
    ) -> Res<'w, R_> {
        Res { inner: AtomicRef::map(this.inner, f) }
    }
}

impl<'w, R: Resource> ResMut<'w, R> {
    fn new(inner: AtomicRefMut<'w, R>) -> Self {
        Self { inner }
    }

    /// Map this reference `R -> R_`.
    ///
    /// This is usually used to borrow a field of `R`.
    pub fn map<R_: Resource>(
        this: Self,
        f: impl FnOnce(&mut R) -> &mut R_,
    ) -> ResMut<'w, R_> {
        ResMut { inner: AtomicRefMut::map(this.inner, f) }
    }
}

// ---

/// # Safety
///
/// [`SystemInput::get`] matches [`SystemInput::world_access`].
unsafe impl<R: Resource> SystemInput for Res<'_, R> {
    type Output<'w, 's> = Res<'w, R>;
    type State = ();

    fn init(_world: &World) -> Self::State {}

    fn world_access(_state: &Self::State, access: &mut WorldAccess) {
        access.borrows_resource::<R>(Level::Read);
    }

    unsafe fn get<'w, 's>(
        _state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        // SAFETY: the caller ensures that the world contains this resource and
        // that it is not already mutably borrowed
        unsafe { world.as_ref().resource().unwrap_unchecked() }
    }
}

/// # Safety
///
/// [`Res`] performs only immutable access.
unsafe impl<R: Resource> ReadOnlySystemInput for Res<'_, R> {}

/// # Safety
///
/// [`SystemInput::get`] matches [`SystemInput::world_access`].
unsafe impl<R: Resource> SystemInput for ResMut<'_, R> {
    type Output<'w, 's> = ResMut<'w, R>;
    type State = ();

    fn init(_world: &World) -> Self::State {}

    fn world_access(_state: &Self::State, access: &mut WorldAccess) {
        access.borrows_resource::<R>(Level::Write);
    }

    unsafe fn get<'w, 's>(
        _state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        // SAFETY: the caller ensures that the world contains this resource and
        // that it is not already borrowed
        unsafe { world.as_ref().resource_mut().unwrap_unchecked() }
    }
}

// ---

/// # Safety
///
/// [`SystemInput::get`] matches [`SystemInput::world_access`].
unsafe impl<R: Resource> SystemInput for Option<Res<'_, R>> {
    type Output<'w, 's> = Option<Res<'w, R>>;
    type State = ();

    fn init(_world: &World) -> Self::State {}

    fn world_access(_state: &Self::State, access: &mut WorldAccess) {
        access.maybe_borrows_resource::<R>(Level::Read);
    }

    unsafe fn get<'w, 's>(
        _state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        // SAFETY: the caller ensures that the world is valid for this access
        unsafe { world.as_ref().resource().ok() }
    }
}

/// # Safety
///
/// [`Res`] performs only immutable access.
unsafe impl<R: Resource> ReadOnlySystemInput for Option<Res<'_, R>> {}

/// # Safety
///
/// [`SystemInput::get`] matches [`SystemInput::world_access`].
unsafe impl<R: Resource> SystemInput for Option<ResMut<'_, R>> {
    type Output<'w, 's> = Option<ResMut<'w, R>>;
    type State = ();

    fn init(_world: &World) -> Self::State {}

    fn world_access(_state: &Self::State, access: &mut WorldAccess) {
        access.maybe_borrows_resource::<R>(Level::Write);
    }

    unsafe fn get<'w, 's>(
        _state: &'s mut Self::State,
        world: WorldPtr<'w>,
    ) -> Self::Output<'w, 's> {
        // SAFETY: the caller ensures that the world is valid for this access
        unsafe { world.as_mut().resource_mut().ok() }
    }
}

// ---

impl<R: Resource> Deref for Res<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R: Resource> Deref for ResMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R: Resource> DerefMut for ResMut<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Resource)]
    struct Counter(usize);

    #[test]
    fn system_input() {
        fn requiring_system(mut resource: ResMut<Counter>) {
            resource.0 += 1;
        }

        fn non_requiring_system(_resource: Option<Res<Counter>>) {}

        let mut world = World::new();

        {
            let mut system = non_requiring_system.into_system();

            system.init(&world);

            // SAFETY: The system initialized and is read-only and world pointer
            // is valid as it was constructed from a reference.
            unsafe { system.run(world.as_ptr()) };
        }

        world.create(Counter(0));

        {
            let mut system = requiring_system.into_system();

            system.init(&world);

            // SAFETY: The system is initialized and the world pointer is valid
            // as it was constructed from a mutable reference. The
            // required accesses (only `Counter`) exists in the
            // world
            unsafe { system.run(world.as_ptr_mut()) };
        }

        let counter: Res<'_, Counter> = world.resource().unwrap();

        assert_eq!(counter.0, 1);
    }
}
