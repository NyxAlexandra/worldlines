use core::slice;
use std::borrow::Borrow;
use std::fmt;
use std::hash::{Hash, Hasher};

use super::{SparseIndex, SparseIter};

/// A set of sparse indices.
///
/// Sparse sets allow for fast `.contains(index)` calls as it only has to check
/// if the index refers to an actual slot.
#[derive(Clone)]
pub struct SparseSet<I: SparseIndex> {
    inner: Vec<Option<I>>,
    /// The amount of filled slots.
    len: usize,
}

impl<I: SparseIndex> SparseSet<I> {
    /// Creates a new empty sparse set.
    pub const fn new() -> Self {
        let inner = Vec::new();
        let len = 0;

        Self { inner, len }
    }

    /// Returns the amount of indices in this set.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the set contains no indices.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the indices in this set.
    pub fn iter(&self) -> SparseIter<'_, I> {
        SparseIter { inner: self.inner.iter(), len: self.len }
    }

    /// Returns an iterator over the slots in this set.
    pub fn slots(&self) -> slice::Iter<'_, Option<I>> {
        self.inner.iter()
    }

    /// Returns `true` if the set contains the given index.
    ///
    /// The index is any type that the actual key can borrow as.
    pub fn contains<Q>(&self, index: &Q) -> bool
    where
        Q: SparseIndex,
        I: Borrow<Q>,
    {
        self.inner.get(index.sparse_index()).is_some_and(Option::is_some)
    }

    /// Inserts an index into the set.
    ///
    /// Returns the previous value if it exists.
    pub fn insert(&mut self, index: I) -> Option<I> {
        let sparse = index.sparse_index();

        if sparse >= self.inner.len() {
            self.inner.resize_with(sparse + 1, || None);
        }

        // SAFETY: guaranteed to exist due to above resize
        let result =
            unsafe { self.inner.get_unchecked_mut(sparse) }.replace(index);

        if result.is_none() {
            self.len += 1;
        }

        result
    }

    /// Removes an index from the set.
    ///
    /// Returns the previous value if it existed in the set.
    pub fn remove<Q>(&mut self, index: &Q) -> Option<I>
    where
        Q: SparseIndex,
        I: Borrow<Q>,
    {
        self.inner
            .get_mut(index.sparse_index())
            .and_then(Option::take)
            .inspect(|_| self.len -= 1)
    }

    /// Clears all indices from the set.
    pub fn clear(&mut self) {
        self.inner.clear();
        self.len = 0;
    }
}

impl<I: SparseIndex + fmt::Debug> fmt::Debug for SparseSet<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<I: SparseIndex> Default for SparseSet<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: SparseIndex + PartialEq> PartialEq for SparseSet<I> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other)
    }
}

impl<I: SparseIndex + Eq> Eq for SparseSet<I> {}

impl<I: SparseIndex + Hash> Hash for SparseSet<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // hash values, not slots
        for value in self {
            value.hash(state);
        }
    }
}

impl<'a, I: SparseIndex> IntoIterator for &'a SparseSet<I> {
    type IntoIter = SparseIter<'a, I>;
    type Item = &'a I;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: add tests for validity of `(Partial)Eq` and `Hash`

    #[test]
    fn insert_remove() {
        let mut set = SparseSet::new();

        assert!(set.is_empty());

        set.insert(0);
        set.insert(1);
        set.insert(3);

        assert_eq!(set.len(), 3);

        set.remove(&1);

        assert_eq!(set.len(), 2);

        set.clear();

        assert!(set.is_empty());
    }
}
