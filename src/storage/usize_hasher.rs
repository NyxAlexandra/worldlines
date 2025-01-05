use std::hash::{BuildHasher, Hasher};

/// A hasher that specializes in hashing [`usize`]'s.
#[repr(transparent)]
#[derive(Default, Clone, Copy)]
pub struct UsizeHasher {
    inner: usize,
}

impl Hasher for UsizeHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.inner as _
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("attempted to hash a non-`usize` with `UsizeHasher`");
    }

    #[inline(always)]
    fn write_usize(&mut self, i: usize) {
        self.inner = i;
    }
}

impl BuildHasher for UsizeHasher {
    type Hasher = Self;

    fn build_hasher(&self) -> Self::Hasher {
        *self
    }
}
