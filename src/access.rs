//! Types for validating world access.

use core::fmt;

use thiserror::Error;

use crate::prelude::{Component, ComponentInfo, ComponentSet};
use crate::storage::{SparseIndex, SparseSet};
use crate::world::World;

/// Type that verifies that world access is correct.
#[derive(Debug)]
pub struct WorldAccess {
    /// The current level of this access.
    level: Option<Level>,
    world: Option<Level>,
    all_entities: Option<Level>,
    components: SparseSet<ComponentAccess>,
    /// The first error encountered.
    ///
    /// If the error exists, no more accesses can be added.
    error: Option<AccessError>,
}

/// Builder for [`WorldAccess`].
pub struct WorldAccessBuilder<'w> {
    world: &'w World,
    output: WorldAccess,
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

/// Represents access to a particular component.
#[derive(Clone, Copy)]
struct ComponentAccess {
    info: ComponentInfo,
    level: Level,
    required: bool,
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
}

/// Read or write access.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Read,
    Write,
}

impl WorldAccess {
    /// Creates a new empty access set.
    pub const fn new() -> Self {
        let level = None;
        let world = None;
        let all_entities = None;
        let components = SparseSet::new();
        let error = None;

        Self { level, world, all_entities, components, error }
    }

    /// Creates a new world access builder.
    pub const fn builder(world: &World) -> WorldAccessBuilder<'_> {
        WorldAccessBuilder::new(world)
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
    fn accesses(&self) -> impl Iterator<Item = Access> + '_ {
        let world = self.world.map(Access::world);
        let all_entities = self.all_entities.map(Access::all_entities);
        let components = self.components.iter().copied().map(Into::into);

        [world, all_entities].into_iter().flatten().chain(components)
    }

    /// Returns `true` if the described component access is valid for a set of
    /// components.
    pub(crate) fn matches(&self, components: &ComponentSet) -> bool {
        for access in self.accesses() {
            let matches = match access.kind {
                AccessKind::World | AccessKind::AllEntities => true,
                AccessKind::Component { info, required } => {
                    components.contains(info) || !required
                },
            };

            if !matches {
                return false;
            }
        }

        true
    }

    /// Returns a builder for adding new access to this set.
    pub fn into_builder(self, world: &World) -> WorldAccessBuilder<'_> {
        WorldAccessBuilder { world, output: self }
    }
}

impl Default for WorldAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl<'w> WorldAccessBuilder<'w> {
    /// Creates a new world access builder.
    pub const fn new(world: &'w World) -> Self {
        let output = WorldAccess::new();

        Self { world, output }
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
    /// [`WorldAccessBuilder::maybe_borrows_component`].
    pub fn borrows_component<C: Component>(&mut self, level: Level) {
        let info = self.world.components.register::<C>();

        self.add(Access::required_component(info, level));
    }

    /// Adds a non-required component borrow to the set.
    ///
    /// If you require the component to exist, use
    /// [`WorldAccessBuilder::borrows_component`].
    pub fn maybe_borrows_component<C: Component>(&mut self, level: Level) {
        let info = self.world.components.register::<C>();

        self.add(Access::component(info, level));
    }

    /// Builds the world access.
    pub fn build(self) -> WorldAccess {
        self.output
    }

    fn add(&mut self, access: Access) {
        if self.output.error.is_some() {
            return;
        }

        self.output.level = self.output.level.max(Some(access.level));

        let mut error = None;

        for existing_access in self.output.accesses() {
            if access.conflicts_with(existing_access) {
                error = Some(AccessError { lhs: access, rhs: existing_access });
                break;
            }
        }

        if let Some(conflict) = error {
            self.output.error = Some(conflict);
            return;
        }

        match access.kind {
            AccessKind::World => self.output.world = Some(access.level),
            AccessKind::AllEntities => {
                self.output.all_entities = Some(access.level)
            },
            AccessKind::Component { info, required } => {
                self.output.components.insert(ComponentAccess {
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

impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            AccessKind::World => write!(f, "{}World", self.level),
            AccessKind::AllEntities => write!(f, "{}*", self.level),
            AccessKind::Component { info, .. } => {
                write!(f, "{}{}", self.level, info)
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

impl SparseIndex for ComponentAccess {
    fn sparse_index(&self) -> usize {
        self.info.sparse_index()
    }
}

impl fmt::Debug for ComponentAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { info, level, required } = *self;

        Access { kind: AccessKind::Component { info, required }, level }.fmt(f)
    }
}

impl AccessKind {
    /// Returns `true` if the union of this access and another is disjoint.
    fn disjoint_with(self, other: Self) -> bool {
        match (self, other) {
            (
                AccessKind::Component { info: lhs, .. },
                AccessKind::Component { info: rhs, .. },
            ) => lhs != rhs,
            _ => false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[test]
    fn level_ord() {
        assert_eq!(Level::Read.max(Level::Write), Level::Write);
        assert_eq!(Level::Read.min(Level::Write), Level::Read);
    }

    #[test]
    fn component_aliasing() {
        let a = ComponentInfo::of::<A>(0);
        let b = ComponentInfo::of::<B>(1);

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
    fn entities_conflict_with_components() {
        let a = ComponentInfo::of::<A>(0);

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
}
