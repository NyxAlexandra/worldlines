use std::fmt;
use std::iter::Copied;

use crate::{SparseIter, SparseSet, TypeData};

/// A set of types by their [`TypeData`].
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeSet {
    inner: SparseSet<TypeData>,
}

/// An iterator over the types in a [`TypeSet`].
pub struct TypeSetIter<'a> {
    inner: Copied<SparseIter<'a, TypeData>>,
}

impl TypeSet {
    /// Returns an empty set.
    pub const fn new() -> Self {
        Self { inner: SparseSet::new() }
    }

    /// Returns the amount of types in this set.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if this set is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the types in this set.
    pub fn iter(&self) -> TypeSetIter<'_> {
        TypeSetIter { inner: self.inner.iter().copied() }
    }

    pub(crate) fn slots(&self) -> impl Iterator<Item = Option<TypeData>> + '_ {
        self.inner.slots().copied()
    }

    /// Returns `true` if this set contains the type.
    pub fn contains<T: 'static>(&self) -> bool {
        self.contains_type_data(TypeData::of::<T>())
    }

    /// Returns `true` if this set contains the type.
    pub fn contains_type_data(&self, type_data: TypeData) -> bool {
        self.inner.contains(&type_data)
    }

    /// Returns a new set containing the intersection this set and another.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut out = self.clone();

        for type_data in self {
            if !other.contains_type_data(type_data) {
                out.remove_type_data(type_data);
            }
        }

        out
    }

    /// Returns a new set containing the difference of this set and another.
    ///
    /// See also [`TypeSet::symmetric_difference`].
    pub fn difference(&self, other: &Self) -> Self {
        let mut out = self.clone();

        for type_data in self {
            if other.contains_type_data(type_data) {
                out.remove_type_data(type_data);
            }
        }

        out
    }

    /// Returns a new set containing the symmetric difference of this set and
    /// another.
    ///
    /// See also [`TypeSet::difference`].
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let mut out = self.difference(other);

        for type_data in other {
            if !self.contains_type_data(type_data) {
                out.insert_type_data(type_data);
            }
        }

        out
    }

    /// Inserts a new type into the set.
    pub fn insert<T: 'static>(&mut self) {
        self.insert_type_data(TypeData::of::<T>())
    }

    /// Inserts a new type into the set.
    pub fn insert_type_data(&mut self, type_data: TypeData) {
        self.inner.insert(type_data);
    }

    /// Inserts a new type into the set and return `self`.
    pub fn with<T: 'static>(mut self) -> Self {
        self.insert::<T>();

        self
    }

    /// Inserts a new type into the set and returns `self`.
    pub fn with_type_data(mut self, type_data: TypeData) -> Self {
        self.insert_type_data(type_data);

        self
    }

    /// Removes a type from the set and returns `Some` if it was removed.
    pub fn remove<T: 'static>(&mut self) -> Option<()> {
        self.remove_type_data(TypeData::of::<T>())
    }

    /// Removes a type from the set and returns `Some` if it was removed.
    pub fn remove_type_data(&mut self, type_data: TypeData) -> Option<()> {
        self.inner.remove(&type_data).map(|_| ())
    }

    /// Removes a type from the set and returns `self`.
    pub fn without<T: 'static>(mut self) -> Self {
        self.remove::<T>();

        self
    }

    /// Removes a type from the set and returns `self`.
    pub fn without_type_data(mut self, type_data: TypeData) -> Self {
        self.remove_type_data(type_data);

        self
    }

    /// Clears all types from the set.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl<'a> IntoIterator for &'a TypeSet {
    type IntoIter = TypeSetIter<'a>;
    type Item = TypeData;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<TypeData> for TypeSet {
    fn from_iter<I: IntoIterator<Item = TypeData>>(iter: I) -> Self {
        Self { inner: SparseSet::from_iter(iter) }
    }
}

impl fmt::Debug for TypeSet {
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

impl Iterator for TypeSetIter<'_> {
    type Item = TypeData;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for TypeSetIter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection() {
        struct A;
        struct B;
        struct C;
        struct D;

        let a = TypeSet::new().with::<A>();
        let a_b = a.clone().with::<B>();
        let a_b_c = a_b.clone().with::<C>();
        let d = TypeSet::new().with::<D>();

        let empty = TypeSet::new();

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

    #[test]
    fn difference() {
        struct A;
        struct B;

        let a = TypeSet::new().with::<A>();
        let b = TypeSet::new().with::<B>();
        let a_b = a.clone().with::<B>();

        let empty = TypeSet::new();

        assert_eq!(&a.difference(&a), &empty);
        assert_eq!(&a.difference(&a_b), &empty);

        assert_eq!(&a_b.difference(&a), &b);
        assert_eq!(&a_b.difference(&a_b), &empty);
    }

    #[test]
    fn symmetric_difference() {
        struct A;
        struct B;

        let a = TypeSet::new().with::<A>();
        let b = TypeSet::new().with::<B>();
        let a_b = a.clone().with::<B>();

        let empty = TypeSet::new();

        assert_eq!(&a.symmetric_difference(&a), &empty);
        assert_eq!(&a.symmetric_difference(&a_b), &b);

        assert_eq!(&a_b.symmetric_difference(&a), &b);
        assert_eq!(&a_b.symmetric_difference(&a_b), &empty);
    }
}
