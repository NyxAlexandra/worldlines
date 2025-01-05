use std::fmt;
use std::iter::Copied;

use super::{ComponentId, ComponentInfo, ComponentVTable};
use crate::storage::{SparseIter, SparseSet};

/// A set of component types.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct ComponentSet {
    inner: SparseSet<ComponentInfo>,
}

impl ComponentSet {
    /// Returns a new empty component set.
    pub const fn new() -> Self {
        let inner = SparseSet::new();

        Self { inner }
    }

    /// Returns an iterator over the component info in the set
    pub fn iter(&self) -> impl Iterator<Item = ComponentInfo> + use<'_> {
        self.into_iter()
    }

    // Used by `Table`.
    pub(crate) fn slots(
        &self,
    ) -> impl Iterator<Item = Option<ComponentInfo>> + use<'_> {
        self.inner.slots().copied()
    }

    /// Returns `true` if the set contains the given component.
    pub fn contains(&self, component: ComponentId) -> bool {
        self.inner.contains(&component)
    }

    /// Returns a new component set containing the intersection of `self` and
    /// `other`.
    pub fn intersection(&self, other: &ComponentSet) -> Self {
        let mut intersection = self.clone();

        for &component in &self.inner {
            let id = component.id();

            if !other.contains(id) {
                intersection.remove(id);
            }
        }

        intersection
    }

    /// Inserts a new component type into the set.
    pub fn insert(&mut self, component: ComponentInfo) {
        self.inner.insert(component);
    }

    /// Inserts a new component type into the set and returns `self`.
    pub fn and_insert(mut self, component: ComponentInfo) -> Self {
        self.insert(component);

        self
    }

    /// Removes a component type from the set.
    pub fn remove(&mut self, component: ComponentId) -> Option<ComponentInfo> {
        self.inner.remove(&component)
    }

    /// Removes a component type from the set and returns `self`.
    pub fn and_remove(mut self, component: ComponentId) -> Self {
        self.remove(component);

        self
    }
}

// ---

impl fmt::Debug for ComponentSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugDisplay<'a, T: fmt::Display>(&'a T);

        impl<T: fmt::Display> fmt::Debug for DebugDisplay<'_, T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        f.debug_set().entries(self.inner.iter().map(DebugDisplay)).finish()
    }
}

impl<'a> IntoIterator for &'a ComponentSet {
    type IntoIter = Copied<SparseIter<'a, ComponentInfo>>;
    type Item = ComponentInfo;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    struct C;

    #[derive(Component)]
    struct D;

    #[test]
    fn contains() {
        let a = ComponentSet::new().and_insert(ComponentInfo::of::<A>());
        let b = ComponentSet::new().and_insert(ComponentInfo::of::<B>());

        assert!(a.contains(ComponentId::of::<A>()));
        assert!(b.contains(ComponentId::of::<B>()));

        assert!(!a.contains(ComponentId::of::<B>()));
        assert!(!b.contains(ComponentId::of::<A>()));
    }

    #[test]
    fn intersection() {
        let a = ComponentSet::new().and_insert(ComponentInfo::of::<A>());
        let a_b = a.clone().and_insert(ComponentInfo::of::<B>());
        let a_b_c = a_b.clone().and_insert(ComponentInfo::of::<C>());
        let d = ComponentSet::new().and_insert(ComponentInfo::of::<D>());

        let empty = ComponentSet::new();

        assert_eq!(&a.intersection(&a), &a);
        assert_eq!(&a.intersection(&a_b), &a);
        assert_eq!(&a.intersection(&a_b_c), &a);
        assert_eq!(&a.intersection(&d), &empty);

        assert_eq!(&a_b.intersection(&a), &a);
        assert_eq!(&a_b.intersection(&a_b), &a_b);
        assert_eq!(&a_b.intersection(&a_b_c), &a_b);
        assert_eq!(&a_b.intersection(&d), &empty);

        assert_eq!(&a_b_c.intersection(&a), &a);
        assert_eq!(&a_b_c.intersection(&a_b), &a_b);
        assert_eq!(&a_b_c.intersection(&a_b_c), &a_b_c);
        assert_eq!(&a_b_c.intersection(&d), &empty);
    }
}
