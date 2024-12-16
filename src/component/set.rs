use std::any::TypeId;
use std::borrow::Borrow;
use std::fmt;
use std::iter::Copied;

use super::{Component, ComponentInfo, ComponentRegistry};
use crate::storage::{SparseIndex, SparseIter, SparseSet};

/// A set of component types.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ComponentSet {
    inner: SparseSet<ComponentInfo>,
}

/// Builder for a set of component types.
///
/// Used by [`Bundle::components`](super::Bundle::components) to set which
/// components it contains.
pub struct ComponentSetBuilder<'w> {
    inner: SparseSet<ComponentInfo>,
    registry: &'w mut ComponentRegistry,
}

impl ComponentSet {
    // Used by `Components`.
    pub fn iter(&self) -> Copied<SparseIter<'_, ComponentInfo>> {
        self.inner.iter().copied()
    }

    // Used by `Table`.
    pub fn slots(
        &self,
    ) -> impl Iterator<Item = Option<ComponentInfo>> + use<'_> {
        self.inner.slots().copied()
    }

    pub fn contains<Q>(&self, component: Q) -> bool
    where
        Q: SparseIndex,
        ComponentInfo: Borrow<Q>,
    {
        self.inner.contains(&component)
    }

    pub fn intersection(&self, other: &ComponentSet) -> Self {
        let mut intersection = self.clone();

        for &component in &self.inner {
            if !other.contains(component) {
                intersection.remove(component);
            }
        }

        intersection
    }

    pub fn insert(&mut self, info: ComponentInfo) {
        self.inner.insert(info);
    }

    pub fn and_insert(mut self, info: ComponentInfo) -> Self {
        self.insert(info);

        self
    }

    pub fn remove(&mut self, info: ComponentInfo) {
        self.inner.remove(&info);
    }

    pub fn and_remove(mut self, info: ComponentInfo) -> Self {
        self.remove(info);

        self
    }
}

impl<'w> ComponentSetBuilder<'w> {
    pub(crate) fn new(registry: &'w mut ComponentRegistry) -> Self {
        let inner = SparseSet::new();

        Self { inner, registry }
    }

    /// Inserts the info for the given component into the builder.
    pub fn insert<C: Component>(&mut self) {
        let next = ComponentInfo::of::<C>(self.registry.len());
        let info = *self.registry.entry(TypeId::of::<C>()).or_insert(next);

        self.inner.insert(info);
    }

    /// Builds the component set.
    pub(crate) fn build(self) -> ComponentSet {
        let Self { inner, .. } = self;

        ComponentSet { inner }
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
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection() {
        #[derive(Component)]
        struct A;

        #[derive(Component)]
        struct B;

        #[derive(Component)]
        struct C;

        #[derive(Component)]
        struct D;

        let a = ComponentSet::default().and_insert(ComponentInfo::of::<A>(0));
        let a_b = a.clone().and_insert(ComponentInfo::of::<B>(1));
        let a_b_c = a_b.clone().and_insert(ComponentInfo::of::<C>(2));
        let d = ComponentSet::default().and_insert(ComponentInfo::of::<D>(3));

        let empty = ComponentSet::default();

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
