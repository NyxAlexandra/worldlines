use std::hash::{BuildHasher, Hasher};

/// A hasher that specializes in hashing [`TypeId`]s.
#[repr(transparent)]
#[derive(Default, Clone, Copy)]
pub struct TypeIdHasher {
    inner: u64,
}

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.inner
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("attempted to hash a non-`TypeId` with `TypeIdHasher`");
    }

    fn write_u64(&mut self, i: u64) {
        self.inner = i;
    }
}

impl BuildHasher for TypeIdHasher {
    type Hasher = Self;

    fn build_hasher(&self) -> Self::Hasher {
        *self
    }
}
