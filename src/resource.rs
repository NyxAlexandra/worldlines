pub(crate) use self::storage::*;

mod storage;

use std::fmt;
use std::ops::{Deref, DerefMut};

use atomic_refcell::{AtomicRef, AtomicRefMut};
use thiserror::Error;

use crate::{
    Component,
    ReadOnlySystemInput,
    SystemInput,
    TypeData,
    World,
    WorldAccess,
    WorldPtr,
};

/// Trait for resources.
pub trait Resource: Component {}

impl<R: Component> Resource for R {}

/// A reference to a [`Resource`].
#[derive(Debug)]
pub struct Res<'w, R: ?Sized> {
    guard: AtomicRef<'w, R>,
}

/// A mutable reference to a [`Resource`].
pub struct ResMut<'w, R: ?Sized> {
    guard: AtomicRefMut<'w, R>,
}

/// Error when working with [`Resource`]s.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[error("error accessing resource {resource}: {kind}")]
pub struct ResourceError {
    resource: TypeData,
    kind: ResourceErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Error)]
enum ResourceErrorKind {
    NotFound,
    AlreadyBorrowed,
}

impl<'w, R: ?Sized> Res<'w, R> {
    pub(crate) const fn new(guard: AtomicRef<'w, R>) -> Self {
        Self { guard }
    }

    /// Clone this reference.
    ///
    /// This is an assosciated reference so as to not interfere with
    /// dereferencing.
    #[allow(clippy::should_implement_trait)]
    pub fn clone(this: &Self) -> Self {
        Self { guard: AtomicRef::clone(&this.guard) }
    }

    /// Map this reference `R -> U`.
    ///
    /// This is usually used to borrow a field of `R`.
    pub fn map<U: ?Sized>(this: Self, f: impl FnOnce(&R) -> &U) -> Res<'w, U> {
        Res { guard: AtomicRef::map(this.guard, f) }
    }
}

unsafe impl<R: Resource> SystemInput for Res<'_, R> {
    type Output<'w, 's> = Res<'w, R>;
    type State = ();

    fn access(access: &mut WorldAccess) {
        access.resource::<R>();
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { world.as_ref().resource::<R>().unwrap() }
    }
}

unsafe impl<R: Resource> ReadOnlySystemInput for Res<'_, R> {}

impl<R: ?Sized> Deref for Res<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

unsafe impl<R: Resource> SystemInput for Option<Res<'_, R>> {
    type Output<'w, 's> = Option<Res<'w, R>>;
    type State = ();

    fn access(access: &mut WorldAccess) {
        access.resource::<R>();
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { world.as_ref().resource::<R>().ok() }
    }
}

unsafe impl<R: Resource> ReadOnlySystemInput for Option<Res<'_, R>> {}

impl<'w, R: ?Sized> ResMut<'w, R> {
    pub(crate) const fn new(guard: AtomicRefMut<'w, R>) -> Self {
        Self { guard }
    }

    /// Map this reference `R -> U`.
    ///
    /// This is usually used to borrow a field of `R`.
    pub fn map<U: ?Sized>(
        this: Self,
        f: impl FnOnce(&mut R) -> &mut U,
    ) -> ResMut<'w, U> {
        ResMut { guard: AtomicRefMut::map(this.guard, f) }
    }
}

unsafe impl<R: Resource> SystemInput for ResMut<'_, R> {
    type Output<'w, 's> = ResMut<'w, R>;
    type State = ();

    fn access(access: &mut WorldAccess) {
        access.resource_mut::<R>();
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { world.as_ref().resource_mut::<R>().unwrap() }
    }
}

// SAFETY: resources can be mutably borrowed from an immutable reference
unsafe impl<R: Resource> ReadOnlySystemInput for ResMut<'_, R> {}

unsafe impl<R: Resource> SystemInput for Option<ResMut<'_, R>> {
    type Output<'w, 's> = Option<ResMut<'w, R>>;
    type State = ();

    fn access(access: &mut WorldAccess) {
        access.resource_mut::<R>();
    }

    fn init(_world: &World) -> Self::State {}

    unsafe fn get<'w, 's>(
        world: WorldPtr<'w>,
        _state: &'s mut Self::State,
    ) -> Self::Output<'w, 's> {
        unsafe { world.as_mut().resource_mut::<R>().ok() }
    }
}

unsafe impl<R: Resource> ReadOnlySystemInput for Option<ResMut<'_, R>> {}

impl<R: ?Sized> Deref for ResMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<R: ?Sized> DerefMut for ResMut<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

impl ResourceError {
    pub(crate) fn not_found<R: 'static>() -> Self {
        let resource = TypeData::of::<R>();
        let kind = ResourceErrorKind::NotFound;

        Self { resource, kind }
    }

    pub(crate) fn already_borrowed<R: 'static>() -> Self {
        let resource = TypeData::of::<R>();
        let kind = ResourceErrorKind::AlreadyBorrowed;

        Self { resource, kind }
    }
}

impl fmt::Display for ResourceErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ResourceErrorKind::NotFound => "not found",
            ResourceErrorKind::AlreadyBorrowed => "already borrowed",
        })
    }
}
