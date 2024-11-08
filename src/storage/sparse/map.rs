use core::slice;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use super::{SparseIndex, SparseIter, SparseIterMut};

/// A list of sparse values accessed by a sparse index.
///
/// Doesn't store indices.
#[derive(Clone)]
pub struct SparseMap<K: SparseIndex, V> {
    inner: Vec<Option<V>>,
    /// The amount of filled slots.
    len: usize,
    _key: PhantomData<fn(&K)>,
}

impl<K: SparseIndex, V> SparseMap<K, V> {
    /// Creates a new empty sparse map.
    pub const fn new() -> Self {
        let inner = Vec::new();
        let len = 0;

        Self { inner, len, _key: PhantomData }
    }

    /// Returns the amount of values in the sparse map.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the map is empty.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the indices in this set.
    pub fn iter(&self) -> SparseIter<'_, V> {
        SparseIter { inner: self.inner.iter(), len: self.len }
    }

    /// Returns an iterator over the indices in this set.
    pub fn iter_mut(&mut self) -> SparseIterMut<'_, V> {
        SparseIterMut { inner: self.inner.iter_mut(), len: self.len }
    }

    /// Returns an iterator over the slots in this map.
    pub fn slots(&self) -> slice::Iter<'_, Option<V>> {
        self.inner.iter()
    }

    /// Returns `true` if the map contains a value corresponding to the index.
    pub fn contains(&self, index: &K) -> bool {
        self.inner.get(index.sparse_index()).is_some_and(Option::is_some)
    }

    /// Returns a reference to the value assosciated with the index.
    pub fn get(&self, index: &K) -> Option<&V> {
        self.inner.get(index.sparse_index()).and_then(Option::as_ref)
    }

    /// Returns a mutable reference to the value assosciated with the index.
    pub fn get_mut(&mut self, index: &K) -> Option<&mut V> {
        self.inner.get_mut(index.sparse_index()).and_then(Option::as_mut)
    }

    /// Returns a mutable reference to the value, inserting a value if it
    /// doesn't exist.
    pub fn get_or_insert_with(
        &mut self,
        index: K,
        f: impl FnOnce() -> V,
    ) -> &mut V {
        let sparse = index.sparse_index();

        if !self.contains(&index) {
            self.insert(index, f());
        }

        unsafe {
            self.inner.get_unchecked_mut(sparse).as_mut().unwrap_unchecked()
        }
    }

    /// Returns a mutable reference to a value, inserting the default if it
    /// doesn't exist.
    pub fn get_or_default(&mut self, index: K) -> &mut V
    where
        V: Default,
    {
        self.get_or_insert_with(index, Default::default)
    }

    /// Inserts a value at an index.
    ///
    /// Returns the previous value if it exists.
    pub fn insert(&mut self, index: K, value: V) -> Option<V> {
        let sparse = index.sparse_index();

        if sparse >= self.inner.len() {
            self.inner.resize_with(sparse + 1, || None);
        }

        let result =
            unsafe { self.inner.get_unchecked_mut(sparse) }.replace(value);

        if result.is_none() {
            self.len += 1;
        }

        result
    }

    /// Removes the value at the index.
    pub fn remove(&mut self, index: &K) -> Option<V> {
        self.inner
            .get_mut(index.sparse_index())
            .and_then(Option::take)
            .inspect(|_| self.len -= 1)
    }

    /// Removes all values from the map.
    pub fn clear(&mut self) {
        self.inner.clear();
        self.len = 0;
    }
}

impl<K: SparseIndex, V: fmt::Debug> fmt::Debug for SparseMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<K: SparseIndex, V> Default for SparseMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: SparseIndex, V: PartialEq> PartialEq for SparseMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other)
    }
}

impl<K: SparseIndex, V: Eq> Eq for SparseMap<K, V> {}

impl<K: SparseIndex, V: Hash> Hash for SparseMap<K, V> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // hash values, not slots
        for value in self {
            value.hash(state);
        }
    }
}

impl<'a, K: SparseIndex, V> IntoIterator for &'a SparseMap<K, V> {
    type IntoIter = SparseIter<'a, V>;
    type Item = &'a V;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K: SparseIndex, V> IntoIterator for &'a mut SparseMap<K, V> {
    type IntoIter = SparseIterMut<'a, V>;
    type Item = &'a mut V;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<K: SparseIndex, V> FromIterator<Option<V>> for SparseMap<K, V> {
    fn from_iter<I: IntoIterator<Item = Option<V>>>(iter: I) -> Self {
        let inner: Vec<_> = iter.into_iter().collect();
        let len =
            inner.iter().map(Option::as_ref).filter(Option::is_some).count();

        Self { inner, len, _key: PhantomData }
    }
}
