use std::fmt;

use thiserror::Error;

use crate::{Component, ComponentId, Resource, SparseMap, TypeData};

// TODO: refactor this to make it neater (and possibly improve errors)

/// Tracks access to parts of a [`World`](crate::World).
///
/// Used to verify that [queries](crate::Query) and [systems](crate::System)
/// don't alias internally or when constructing in parallel.
#[derive(Debug, Clone)]
pub struct WorldAccess {
    level: Level,
    world: Option<Level>,
    entities: Option<Level>,
    components: SparseMap<ComponentId, ComponentAccess>,
    resources: SparseMap<ComponentId, ResourceAccess>,
    error: Option<WorldAccessError>,
}

/// An error for [`WorldAccess`] that aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("conflicting world access:\n- {first:?}\n- {second:?}")]
pub struct WorldAccessError {
    first: LeveledAccess,
    second: LeveledAccess,
}

#[derive(Clone, Copy)]
struct ComponentAccess {
    component: TypeData,
    level: Level,
}

#[derive(Clone, Copy)]
struct ResourceAccess {
    resource: TypeData,
    level: Level,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct LeveledAccess {
    access: Access,
    level: Level,
}

/// A particular access to a [`World`](crate::World).
///
/// See [`WorldAccess`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    World,
    Entities,
    Component(TypeData),
    Resource(TypeData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Read,
    Write,
}

impl WorldAccess {
    /// Returns a new [`WorldAccess`].
    pub fn new() -> Self {
        let level = Level::Read;
        let world = None;
        let entities = None;
        let resources = SparseMap::new();
        let components = SparseMap::new();
        let error = None;

        Self { level, world, entities, components, resources, error }
    }

    /// Returns `true` if the access does not have aliasing issues.
    pub fn is_valid(&self) -> bool {
        self.error.is_none()
    }

    /// Returns `true` if the access requires a mutable reference.
    pub fn is_mutable(&self) -> bool {
        self.level.is_write()
    }

    /// Returns `true` if the access requires an immutable reference.
    pub fn is_immutable(&self) -> bool {
        self.level.is_read()
    }

    /// The first error encountered, if any.
    pub fn error(&self) -> Option<WorldAccessError> {
        self.error
    }

    /// Returns `true` if this accesses a particular resource.
    pub fn contains(&self, access: Access) -> bool {
        self.access(access).is_some()
    }

    fn access(&self, access: Access) -> Option<LeveledAccess> {
        match access {
            Access::World => self
                .world
                .map(|level| LeveledAccess { access: Access::World, level }),
            Access::Entities => self
                .entities
                .map(|level| LeveledAccess { access: Access::Entities, level }),
            Access::Component(component) => {
                self.components.get(&component.component_id()).copied().map(
                    |ComponentAccess { component, level }| LeveledAccess {
                        access: Access::Component(component),
                        level,
                    },
                )
            },
            Access::Resource(resource) => {
                self.resources.get(&resource.component_id()).copied().map(
                    |ResourceAccess { resource, level }| LeveledAccess {
                        access: Access::Resource(resource),
                        level,
                    },
                )
            },
        }
    }

    /// Adds a read of the entire world.
    pub fn world(&mut self) {
        self.add(Access::World, Level::Read);
    }

    /// Adds a write of the entire world.
    pub fn world_mut(&mut self) {
        self.add(Access::World, Level::Write);
    }

    /// Adds a read to all entities ([`World::entities`](crate::World)).
    pub fn entities(&mut self) {
        self.add(Access::Entities, Level::Read);
    }

    /// Adds a write to all entities ([`World::entities_mut`](crate::World)).
    pub fn entities_mut(&mut self) {
        self.add(Access::Entities, Level::Write);
    }

    /// Adds a read to all instances of component `C`.
    pub fn component<C: Component>(&mut self) {
        self.add(Access::Component(TypeData::of::<C>()), Level::Read);
    }

    /// Adds a write to all instances of component `C`.
    pub fn component_mut<C: Component>(&mut self) {
        self.add(Access::Component(TypeData::of::<C>()), Level::Write);
    }

    /// Adds a read of a resource `R`.
    pub fn resource<R: Resource>(&mut self) {
        self.add(Access::Resource(TypeData::of::<R>()), Level::Read);
    }

    /// Adds a read of a resource `R`.
    pub fn resource_mut<R: Resource>(&mut self) {
        self.add(Access::Resource(TypeData::of::<R>()), Level::Write);
    }

    /// Clears this [`WorldAccess`].
    pub fn clear(&mut self) {
        self.level = Level::Read;
        self.world = None;
        self.entities = None;
        self.components.clear();
        self.resources.clear();
        self.error = None;
    }

    fn add(&mut self, access: Access, level: Level) {
        if self.error.is_some() {
            return;
        }

        if level.is_write() {
            self.level = level;
        }

        let second = LeveledAccess { access, level };

        for first in [
            self.access(Access::World),
            self.access(Access::Entities),
            if let Access::Component(component) = access {
                self.access(Access::Component(component))
            } else {
                None
            },
            if let Access::Resource(resource) = access {
                self.access(Access::Resource(resource))
            } else {
                None
            },
        ]
        .into_iter()
        // TODO: replace with a `highest_component` field
        .chain(self.components.iter().copied().map(
            |ComponentAccess { component, level }| {
                Some(LeveledAccess {
                    access: Access::Component(component),
                    level,
                })
            },
        ))
        .chain(self.resources.iter().copied().map(
            |ResourceAccess { resource, level }| {
                Some(LeveledAccess {
                    access: Access::Resource(resource),
                    level,
                })
            },
        ))
        .flatten()
        {
            if first.conflicts_with(second) {
                self.error = Some(WorldAccessError { first, second });

                return;
            }
        }

        match access {
            Access::World => self.world = Some(level),
            Access::Entities => self.entities = Some(level),
            Access::Component(component) => {
                self.components.insert(
                    component.component_id(),
                    ComponentAccess { component, level },
                );
            },
            Access::Resource(resource) => {
                self.resources.insert(
                    resource.component_id(),
                    ResourceAccess { resource, level },
                );
            },
        }
    }
}

impl Default for WorldAccess {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ComponentAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LeveledAccess {
            access: Access::Component(self.component),
            level: self.level,
        }
        .fmt(f)
    }
}

impl fmt::Debug for ResourceAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LeveledAccess {
            access: Access::Resource(self.resource),
            level: self.level,
        }
        .fmt(f)
    }
}

impl LeveledAccess {
    fn conflicts_with(self, other: Self) -> bool {
        (self.level.is_write() || other.level.is_write())
            && match (self.access, other.access) {
                // conflict if multiple access to same component/resource
                (Access::Component(first), Access::Component(second))
                | (Access::Resource(first), Access::Resource(second)) => {
                    first == second
                },
                // accesses to components and resources don't conflict
                (Access::Component(_), Access::Resource(_))
                | (Access::Resource(_), Access::Component(_)) => false,
                // multiple-mutable access is only valid among
                // components/resources
                _ => true,
            }
    }
}

impl fmt::Debug for LeveledAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.access {
            Access::World => {
                f.write_str(match self.level {
                    Level::Read => "&",
                    Level::Write => "&mut ",
                })?;

                f.write_str("World")
            },
            Access::Entities => f.write_str(match self.level {
                Level::Read => "Entities",
                Level::Write => "EntitiesMut",
            }),
            Access::Component(component) => {
                f.write_str(match self.level {
                    Level::Read => "&",
                    Level::Write => "&mut ",
                })?;

                write!(f, "{}", component)
            },
            Access::Resource(resource) => {
                f.write_str(match self.level {
                    Level::Read => "Res<",
                    Level::Write => "ResMut<",
                })?;

                write!(f, "{}", resource)?;

                f.write_str(">")
            },
        }
    }
}

impl Level {
    fn is_read(self) -> bool {
        matches!(self, Level::Read)
    }

    fn is_write(self) -> bool {
        matches!(self, Level::Write)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_component_aliasing() {
        struct A;

        assert!(
            !LeveledAccess {
                access: Access::Component(TypeData::of::<A>()),
                level: Level::Read,
            }
            .conflicts_with(LeveledAccess {
                access: Access::Component(TypeData::of::<A>()),
                level: Level::Read,
            }),
            "accesses to the same component don't alias if they are both reads",
        );

        assert!(
            LeveledAccess {
                access: Access::Component(TypeData::of::<A>()),
                level: Level::Write,
            }
            .conflicts_with(LeveledAccess {
                access: Access::Component(TypeData::of::<A>()),
                level: Level::Read,
            }),
            "accesses to the same component alias if one is a write",
        );
    }

    #[test]
    fn entity_conflicts_with_component() {
        struct A;

        assert!(
            LeveledAccess { access: Access::Entities, level: Level::Write }
                .conflicts_with(LeveledAccess {
                    access: Access::Component(TypeData::of::<A>()),
                    level: Level::Read,
                }),
            "access to all entities conflicts with access to components",
        );
        assert!(
            LeveledAccess { access: Access::Entities, level: Level::Read }
                .conflicts_with(LeveledAccess {
                    access: Access::Component(TypeData::of::<A>()),
                    level: Level::Write,
                }),
            "access to all entities conflicts with access to components",
        );
        assert!(
            LeveledAccess { access: Access::Entities, level: Level::Write }
                .conflicts_with(LeveledAccess {
                    access: Access::Component(TypeData::of::<A>()),
                    level: Level::Write,
                }),
            "access to all entities conflicts with access to components",
        );
        assert!(!LeveledAccess {
            access: Access::Entities,
            level: Level::Read
        }
        .conflicts_with(LeveledAccess {
            access: Access::Component(TypeData::of::<A>()),
            level: Level::Read,
        }),);
    }

    #[test]
    fn resource_conflicts_with_world() {
        struct A;

        assert!(
            LeveledAccess { access: Access::World, level: Level::Write }
                .conflicts_with(LeveledAccess {
                    access: Access::Resource(TypeData::of::<A>()),
                    level: Level::Read,
                }),
            "access to the world conflicts with access to resources",
        );
        assert!(
            LeveledAccess { access: Access::World, level: Level::Read }
                .conflicts_with(LeveledAccess {
                    access: Access::Resource(TypeData::of::<A>()),
                    level: Level::Write,
                }),
            "access to the world conflicts with access to resources",
        );
        assert!(
            LeveledAccess { access: Access::World, level: Level::Write }
                .conflicts_with(LeveledAccess {
                    access: Access::Resource(TypeData::of::<A>()),
                    level: Level::Write,
                }),
            "access to the world conflicts with access to resources",
        );
        assert!(!LeveledAccess { access: Access::World, level: Level::Read }
            .conflicts_with(LeveledAccess {
                access: Access::Resource(TypeData::of::<A>()),
                level: Level::Read,
            }),);
    }
}
