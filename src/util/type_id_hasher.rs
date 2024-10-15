use std::any::TypeId;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use indexmap::IndexMap;

/// A [`HashMap`] mapping [`TypeId`]'s to values.
pub type TypeMap<V> = HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

/// An [`IndexMap`] mapping [`TypeId`]'s to values.
pub type IndexTypeMap<V> =
    IndexMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

/// A hasher that specializes in hashing [`TypeId`]s.
#[derive(Default)]
pub struct TypeIdHasher {
    hash: u64,
}

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("attempted to hash a non-`TypeId` with `TypeIdHasher`");
    }

    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }
}
