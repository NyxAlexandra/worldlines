//! Types for validating world access.

use core::fmt;

use thiserror::Error;

use crate::prelude::{
    Component,
    ComponentInfo,
    ComponentSet,
    ComponentVTable,
    Resource,
    ResourceInfo,
};
use crate::storage::{SparseIndex, SparseSet};

/// Type that verifies that world access is correct.
#[derive(Debug)]
pub struct WorldAccess {
    /// The current level of this access.
    level: Option<Level>,
    world: Option<Level>,
    all_entities: Option<Level>,
    components: SparseSet<ComponentAccess>,
    resources: SparseSet<ResourceAccess>,
    /// The first error encountered.
    ///
    /// If the error exists, no more accesses can be added.
    error: Option<AccessError>,
}

/// An error for conflicting access.
#[derive(Debug, Clone, Copy, Error)]
#[error("conflicting world access\n- lhs: {lhs}\n- rhs: {rhs}")]
pub struct AccessError {
    lhs: Access,
    rhs: Access,
}

/// A single access to the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Access {
    pub kind: AccessKind,
    pub level: Level,
}

/// The particular item accessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessKind {
    /// Direct access to the world.
    World,
    /// Access to all components of all entities.
    AllEntities,
    /// Access to a single component.
    Component { info: ComponentInfo, required: bool },
    /// Access to a single resource.
    Resource { info: ResourceInfo, required: bool },
}

/// Read or write access.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Read,
    Write,
}

/// Represents access to a particular component.
#[derive(Clone, Copy)]
struct ComponentAccess {
    info: ComponentInfo,
    level: Level,
    required: bool,
}

/// Represents access to a particular resource.
#[derive(Clone, Copy)]
struct ResourceAccess {
    info: ResourceInfo,
    level: Level,
    required: bool,
}

impl WorldAccess {
    /// Creates a new empty access set.
    pub const fn new() -> Self {
        let level = None;
        let world = None;
        let all_entities = None;
        let components = SparseSet::new();
        let resources = SparseSet::new();
        let error = None;

        Self { level, world, all_entities, components, resources, error }
    }

    /// The current level of this access.
    ///
    /// Returns `None` if nothing is accessed.
    pub fn level(&self) -> Option<Level> {
        self.level
    }

    /// Returns a result for this access, `Err` if there is an access error.
    pub fn result(&self) -> Result<(), AccessError> {
        self.error.map(Err).unwrap_or(Ok(()))
    }

    /// Returns an iterator over all accesses in this set.
    fn accesses(&self) -> impl Iterator<Item = Access> + use<'_> {
        let world = self.world.map(Access::world);
        let all_entities = self.all_entities.map(Access::all_entities);
        let components = self.components.iter().copied().map(Into::into);
        let resources = self.resources.iter().copied().map(Into::into);

        [world, all_entities]
            .into_iter()
            .flatten()
            .chain(components)
            .chain(resources)
    }

    /// Returns `true` if the described component access is valid for a set of
    /// components.
    pub(crate) fn matches(&self, components: &ComponentSet) -> bool {
        for access in self.accesses() {
            match access.kind {
                // doesn't match if this access requires the component but
                // doesn't contain it
                AccessKind::Component { info, required: true }
                    if !components.contains(info.id()) =>
                {
                    return false;
                },
                _ => {},
            }
        }

        true
    }

    /// Adds a world borrow to the set.
    pub fn borrows_world(&mut self, level: Level) {
        self.add(Access::world(level));
    }

    /// Adds a borrow of all entities and their components to the set.
    pub fn borrows_all_entities(&mut self, level: Level) {
        self.add(Access::all_entities(level));
    }

    /// Adds a required component borrow to the set.
    ///
    /// If you don't require the component to exist, use
    /// [`WorldAccess::maybe_borrows_component`].
    pub fn borrows_component<C: Component>(&mut self, level: Level) {
        let info = ComponentInfo::of::<C>();

        self.add(Access::required_component(info, level));
    }

    /// Adds a non-required component borrow to the set.
    ///
    /// If you require the component to exist, use
    /// [`WorldAccess::borrows_component`].
    pub fn maybe_borrows_component<C: Component>(&mut self, level: Level) {
        let info = ComponentInfo::of::<C>();

        self.add(Access::component(info, level));
    }

    /// Adds a required component borrow to the set.
    ///
    /// If you don't require the component to exist, use
    /// [`WorldAccess::maybe_borrows_component`].
    pub fn borrows_resource<R: Resource>(&mut self, level: Level) {
        let info = ResourceInfo::of::<R>();

        self.add(Access::required_resource(info, level));
    }

    /// Adds a non-required component borrow to the set.
    ///
    /// If you require the component to exist, use
    /// [`WorldAccess::borrows_component`].
    pub fn maybe_borrows_resource<R: Resource>(&mut self, level: Level) {
        let info = ResourceInfo::of::<R>();

        self.add(Access::resource(info, level));
    }

    fn add(&mut self, access: Access) {
        if self.error.is_some() {
            return;
        }

        self.level = self.level.max(Some(access.level));

        let mut error = None;

        for existing_access in self.accesses() {
            if access.conflicts_with(existing_access) {
                error = Some(AccessError { lhs: access, rhs: existing_access });
                break;
            }
        }

        self.error = error;

        match access.kind {
            AccessKind::World => self.world = Some(access.level),
            AccessKind::AllEntities => self.all_entities = Some(access.level),
            AccessKind::Component { info, required } => {
                self.components.insert(ComponentAccess {
                    info,
                    level: access.level,
                    required,
                });
            },
            AccessKind::Resource { info, required } => {
                self.resources.insert(ResourceAccess {
                    info,
                    level: access.level,
                    required,
                });
            },
        }
    }
}

impl Access {
    const fn component(info: ComponentInfo, level: Level) -> Self {
        Self { kind: AccessKind::Component { info, required: false }, level }
    }

    const fn required_component(info: ComponentInfo, level: Level) -> Self {
        Self { kind: AccessKind::Component { info, required: true }, level }
    }

    const fn resource(info: ResourceInfo, level: Level) -> Self {
        Self { kind: AccessKind::Resource { info, required: false }, level }
    }

    const fn required_resource(info: ResourceInfo, level: Level) -> Self {
        Self { kind: AccessKind::Resource { info, required: true }, level }
    }

    const fn all_entities(level: Level) -> Self {
        Self { kind: AccessKind::AllEntities, level }
    }

    const fn world(level: Level) -> Self {
        Self { kind: AccessKind::World, level }
    }

    fn conflicts_with(self, other: Self) -> bool {
        (matches!(self.level, Level::Write)
            || matches!(other.level, Level::Write))
            && !self.kind.disjoint_with(other.kind)
    }
}

impl AccessKind {
    /// Returns `true` if the union of this access and another is disjoint.
    fn disjoint_with(self, other: Self) -> bool {
        match (self, other) {
            (
                Self::Component { info: lhs, .. },
                Self::Component { info: rhs, .. },
            ) => lhs != rhs,
            (
                Self::Resource { info: lhs, .. },
                Self::Resource { info: rhs, .. },
            ) => lhs != rhs,
            (Self::AllEntities, Self::Resource { .. })
            | (Self::Resource { .. }, Self::AllEntities) => true,
            _ => false,
        }
    }
}

// ---

impl SparseIndex for ComponentAccess {
    fn sparse_index(&self) -> usize {
        self.info.sparse_index()
    }
}

impl SparseIndex for ResourceAccess {
    fn sparse_index(&self) -> usize {
        self.info.sparse_index()
    }
}

// ---

impl Default for WorldAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            AccessKind::World => write!(f, "{}World", self.level),
            AccessKind::AllEntities => write!(f, "{}*", self.level),
            AccessKind::Component { info, .. } => {
                write!(f, "{}{}", self.level, info)
            },
            AccessKind::Resource { info, .. } => match self.level {
                Level::Read => write!(f, "Res<{}>", info),
                Level::Write => write!(f, "ResMut<{}>", info),
            },
        }
    }
}

impl From<ComponentAccess> for Access {
    fn from(component_access: ComponentAccess) -> Self {
        let ComponentAccess { info, level, required } = component_access;

        Self { kind: AccessKind::Component { info, required }, level }
    }
}

impl From<ResourceAccess> for Access {
    fn from(resource_access: ResourceAccess) -> Self {
        let ResourceAccess { info, level, required } = resource_access;

        Self { kind: AccessKind::Resource { info, required }, level }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Read => "&",
            Level::Write => "&mut ",
        })
    }
}

impl fmt::Debug for ComponentAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { info, level, required } = *self;

        Access { kind: AccessKind::Component { info, required }, level }.fmt(f)
    }
}

impl fmt::Debug for ResourceAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { info, level, required } = *self;

        Access { kind: AccessKind::Resource { info, required }, level }.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Component, Resource)]
    struct A;

    #[derive(Component, Resource)]
    struct B;

    #[test]
    fn level_ord() {
        assert_eq!(Level::Read.max(Level::Write), Level::Write);
        assert_eq!(Level::Read.min(Level::Write), Level::Read);
    }

    #[test]
    fn component_aliasing() {
        let a = ComponentInfo::of::<A>();
        let b = ComponentInfo::of::<B>();

        assert!(
            !Access::component(a, Level::Read)
                .conflicts_with(Access::component(a, Level::Read)),
            "multiple reads to the same component don't alias",
        );
        assert!(
            !Access::component(a, Level::Write)
                .conflicts_with(Access::component(b, Level::Write)),
            "multiple writes to different components don't alias",
        );
        assert!(
            Access::component(a, Level::Write)
                .conflicts_with(Access::component(a, Level::Read)),
            "write and read access to a component alias",
        );
        assert!(
            Access::component(a, Level::Write)
                .conflicts_with(Access::component(a, Level::Write)),
            "multiple writes to a component alias",
        );
    }

    #[test]
    fn resource_aliasing() {
        let a = ResourceInfo::of::<A>();
        let b = ResourceInfo::of::<B>();

        assert!(
            !Access::resource(a, Level::Read)
                .conflicts_with(Access::resource(a, Level::Read)),
            "multiple reads to the same resource don't alias",
        );
        assert!(
            !Access::resource(a, Level::Write)
                .conflicts_with(Access::resource(b, Level::Write)),
            "multiple writes to different resources don't alias",
        );
        assert!(
            Access::resource(a, Level::Write)
                .conflicts_with(Access::resource(a, Level::Read)),
            "write and read access to a resource alias",
        );
        assert!(
            Access::resource(a, Level::Write)
                .conflicts_with(Access::resource(a, Level::Write)),
            "multiple writes to a resource alias",
        );
    }

    #[test]
    fn entities_conflict_with_components() {
        let a = ComponentInfo::of::<A>();

        assert!(
            Access::all_entities(Level::Write)
                .conflicts_with(Access::component(a, Level::Write)),
            "entities require access to all components",
        );
        assert!(
            Access::all_entities(Level::Read)
                .conflicts_with(Access::component(a, Level::Write)),
            "entities should require access to all components",
        );
    }

    #[test]
    fn entities_do_not_conflict_with_resources() {
        let a = ResourceInfo::of::<A>();

        assert!(
            !Access::all_entities(Level::Write)
                .conflicts_with(Access::resource(a, Level::Write)),
            "entities don't access resources",
        );
        assert!(
            !Access::all_entities(Level::Read)
                .conflicts_with(Access::resource(a, Level::Write)),
            "entities don't access resources",
        );
    }
}
