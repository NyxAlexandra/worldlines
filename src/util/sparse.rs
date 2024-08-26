use std::hash::{self, Hash};
use std::marker::PhantomData;
use std::{fmt, slice};

pub trait SparseIndex {
    fn sparse_index(&self) -> usize;
}

#[derive(Clone, Eq, PartialOrd, Ord)]
pub struct SparseSet<I: SparseIndex> {
    inner: Vec<Option<I>>,
    count: usize,
}

#[derive(Clone, Eq, PartialOrd, Ord)]
pub struct SparseMap<I: SparseIndex, T> {
    inner: Vec<Option<T>>,
    count: usize,
    _index: PhantomData<I>,
}

pub struct SparseIter<'a, T> {
    inner: slice::Iter<'a, Option<T>>,
}

pub struct SparseIterMut<'a, T> {
    inner: slice::IterMut<'a, Option<T>>,
}

impl SparseIndex for usize {
    fn sparse_index(&self) -> usize {
        *self
    }
}

impl<I: SparseIndex> SparseSet<I> {
    pub const fn new() -> Self {
        Self { inner: Vec::new(), count: 0 }
    }

    pub const fn len(&self) -> usize {
        self.count
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> SparseIter<'_, I> {
        SparseIter { inner: self.inner.iter() }
    }

    pub fn slots(&self) -> slice::Iter<'_, Option<I>> {
        self.inner.iter()
    }

    pub fn contains(&self, index: &I) -> bool {
        self.inner.get(index.sparse_index()).is_some_and(Option::is_some)
    }

    pub fn insert(&mut self, index: I) -> Option<I> {
        let sparse = index.sparse_index();

        if sparse >= self.inner.len() {
            self.inner.resize_with(sparse + 1, || None);
        }

        // SAFETY: guaranteed to exist due to above resize
        let result = unsafe { self.inner.get_unchecked_mut(sparse) }.replace(index);

        if result.is_none() {
            self.count += 1;
        }

        result
    }

    pub fn remove(&mut self, index: &I) -> Option<I> {
        self.inner
            .get_mut(index.sparse_index())
            .and_then(Option::take)
            .inspect(|_| self.count -= 1)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.count = 0;
    }
}

impl<I: SparseIndex> Default for SparseSet<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: SparseIndex + PartialEq> PartialEq for SparseSet<I> {
    fn eq(&self, other: &Self) -> bool {
        // compare values, not slots
        self.iter().eq(other.iter())
    }
}

impl<I: SparseIndex + Hash> Hash for SparseSet<I> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // hash values, not slots
        for index in self {
            index.hash(state);
        }
    }
}

impl<I: SparseIndex + fmt::Debug> fmt::Debug for SparseSet<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<'a, I: SparseIndex> IntoIterator for &'a SparseSet<I> {
    type IntoIter = SparseIter<'a, I>;
    type Item = &'a I;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<I: SparseIndex> FromIterator<I> for SparseSet<I> {
    fn from_iter<I_: IntoIterator<Item = I>>(iter: I_) -> Self {
        // TODO?: optimize

        let mut out = Self::new();

        for index in iter.into_iter() {
            out.insert(index);
        }

        out
    }
}

impl<I: SparseIndex, T> SparseMap<I, T> {
    pub const fn new() -> Self {
        Self { inner: Vec::new(), count: 0, _index: PhantomData }
    }

    pub const fn len(&self) -> usize {
        self.count
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> SparseIter<'_, T> {
        SparseIter { inner: self.inner.iter() }
    }

    pub fn iter_mut(&mut self) -> SparseIterMut<'_, T> {
        SparseIterMut { inner: self.inner.iter_mut() }
    }

    pub fn slots(&self) -> slice::Iter<'_, Option<T>> {
        self.inner.iter()
    }

    pub fn contains(&self, index: &I) -> bool {
        self.inner.get(index.sparse_index()).is_some_and(Option::is_some)
    }

    pub fn get(&self, index: &I) -> Option<&T> {
        self.inner.get(index.sparse_index()).and_then(Option::as_ref)
    }

    pub fn get_mut(&mut self, index: &I) -> Option<&mut T> {
        self.inner.get_mut(index.sparse_index()).and_then(Option::as_mut)
    }

    pub fn get_or_insert_with(&mut self, index: I, f: impl FnOnce() -> T) -> &mut T {
        let sparse = index.sparse_index();

        if !self.contains(&index) {
            self.insert(index, f());
        }

        unsafe { self.inner.get_unchecked_mut(sparse).as_mut().unwrap_unchecked() }
    }

    pub fn get_or_default(&mut self, index: I) -> &mut T
    where
        T: Default,
    {
        self.get_or_insert_with(index, T::default)
    }

    pub fn insert(&mut self, index: I, value: T) -> Option<T> {
        let sparse = index.sparse_index();

        if sparse >= self.inner.len() {
            self.inner.resize_with(sparse + 1, || None);
        }

        let result = unsafe { self.inner.get_unchecked_mut(sparse) }.replace(value);

        if result.is_none() {
            self.count += 1;
        }

        result
    }

    pub fn remove(&mut self, index: &I) -> Option<T> {
        self.inner
            .get_mut(index.sparse_index())
            .and_then(Option::take)
            .inspect(|_| self.count -= 1)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.count = 0;
    }
}

impl<I: SparseIndex, T> Default for SparseMap<I, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: SparseIndex, T: PartialEq> PartialEq for SparseMap<I, T> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<I: SparseIndex, T: Hash> Hash for SparseMap<I, T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // hash values, not slots
        for value in self {
            value.hash(state);
        }
    }
}

impl<I: SparseIndex, T: fmt::Debug> fmt::Debug for SparseMap<I, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, I: SparseIndex, T> IntoIterator for &'a SparseMap<I, T> {
    type IntoIter = SparseIter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, I: SparseIndex, T> IntoIterator for &'a mut SparseMap<I, T> {
    type IntoIter = SparseIterMut<'a, T>;
    type Item = &'a mut T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<I: SparseIndex, T> FromIterator<Option<T>> for SparseMap<I, T> {
    fn from_iter<I_: IntoIterator<Item = Option<T>>>(iter: I_) -> Self {
        let inner = Vec::from_iter(iter);
        let count = inner.iter().filter(|slot| slot.is_some()).count();

        Self { inner, count, _index: PhantomData }
    }
}

impl<'a, T> Iterator for SparseIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(Option::as_ref)
            .and_then(|slot| slot.or_else(|| self.next()))
    }
}

impl<'a, T> Iterator for SparseIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(Option::as_mut)
            .and_then(|slot| slot.or_else(|| self.next()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: add tests for validity of `(Partial)Eq` and `Hash`

    #[test]
    fn sparse_set_insert_remove() {
        let mut set = SparseSet::new();

        assert!(set.is_empty());

        set.insert(0);
        set.insert(1);
        set.insert(3);

        assert_eq!(set.count, 3);
        assert_eq!(set.len(), 3);

        set.remove(&1);

        assert_eq!(set.len(), 2);

        set.clear();

        assert!(set.is_empty());
    }

    #[test]
    fn sparse_iter() {
        let mut set = SparseSet::new();

        set.insert(0);
        set.insert(1);
        set.insert(3);

        let mut iter = set.iter();

        assert_eq!(iter.next(), Some(&0));
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&3));
        assert!(iter.next().is_none());
    }
}
