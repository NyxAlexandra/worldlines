use std::slice;

pub use self::map::*;
pub use self::set::*;

mod map;
mod set;

/// Trait for types that can provide a `usize` index for a sparse type.
pub trait SparseIndex {
    /// The index into a sparse datatype that this value represents.
    fn sparse_index(&self) -> usize;
}

/// Iterator over values in a sparse datatype.
pub struct SparseIter<'a, T> {
    inner: slice::Iter<'a, Option<T>>,
    /// The amount of filled slots left.
    len: usize,
}

/// Iterator over values in a sparse datatype.
pub struct SparseIterMut<'a, T> {
    inner: slice::IterMut<'a, Option<T>>,
    /// The amount of filled slots left.
    len: usize,
}

impl SparseIndex for usize {
    fn sparse_index(&self) -> usize {
        *self
    }
}

impl<'a, T> Iterator for SparseIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Option::as_ref).and_then(|slot| {
            slot.inspect(|_| self.len -= 1).or_else(|| self.next())
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> ExactSizeIterator for SparseIter<'_, T> {}

impl<'a, T> Iterator for SparseIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Option::as_mut).and_then(|slot| {
            slot.inspect(|_| self.len -= 1).or_else(|| self.next())
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> ExactSizeIterator for SparseIterMut<'_, T> {}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`SparseIter`] and [`SparseIterMut`] needs to yield values regardless of
    /// if there is a `None` between filled slots.
    #[test]
    fn sparse_iter() {
        let mut map = SparseMap::new();

        map.insert(0, 0);
        map.insert(1, 1);
        map.insert(3, 3);

        fn iter_asserts<'a>(mut iter: impl ExactSizeIterator<Item = usize>) {
            assert_eq!(iter.len(), 3);
            assert_eq!(iter.next(), Some(0));

            assert_eq!(iter.len(), 2);
            assert_eq!(iter.next(), Some(1));

            assert_eq!(iter.len(), 1);
            assert_eq!(iter.next(), Some(3));
            assert!(iter.next().is_none());
        }

        iter_asserts(map.iter().copied());
        iter_asserts(map.iter_mut().map(|value| *value));
    }
}
