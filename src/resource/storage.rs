use core::fmt;
use std::any::{type_name, Any};

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};

use super::{Res, ResMut, Resource, ResourceError, ResourceId};
use crate::storage::SparseMap;

/// Storage for all resources.
#[derive(Debug)]
pub struct Resources {
    inner: SparseMap<ResourceId, ResourceBox>,
}

/// Storage for a single resource.
#[repr(transparent)]
struct ResourceBox {
    inner: AtomicRefCell<Box<dyn Any>>,
}

impl Resources {
    pub fn new() -> Self {
        let inner = SparseMap::new();

        Self { inner }
    }

    pub fn contains<R: Resource>(&self) -> bool {
        self.inner.contains(&ResourceId::of::<R>())
    }

    pub fn get<R: Resource>(&self) -> Result<Res<'_, R>, ResourceError> {
        self.inner
            .get(&ResourceId::of::<R>())
            .ok_or(ResourceError::NotFound(type_name::<R>()))
            .and_then(|boxed| unsafe { boxed.get() })
    }

    pub fn get_mut<R: Resource>(&self) -> Result<ResMut<'_, R>, ResourceError> {
        self.inner
            .get(&ResourceId::of::<R>())
            .ok_or(ResourceError::NotFound(type_name::<R>()))
            .and_then(|boxed| unsafe { boxed.get_mut() })
    }

    pub fn insert<R: Resource>(&mut self, resource: R) -> Option<R> {
        self.inner
            .insert(ResourceId::of::<R>(), ResourceBox::new(resource))
            // SAFETY: the inner type is `R` because it was located at the index
            // of `R` in the registry
            .map(|boxed| unsafe { boxed.into_inner() })
    }

    pub fn remove<R: Resource>(&mut self) -> Result<R, ResourceError> {
        self.inner
            .remove(&ResourceId::of::<R>())
            .ok_or(ResourceError::NotFound(type_name::<R>()))
            // SAFETY: the inner type is `R` because it was located at the index
            // of `R` in the registry
            .map(|boxed| unsafe { boxed.into_inner() })
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl ResourceBox {
    fn new<R: Any>(resource: R) -> Self {
        let inner = AtomicRefCell::new(Box::new(resource) as _);

        Self { inner }
    }

    /// ## Safety
    ///
    /// The type `R` must match the type in the box.
    unsafe fn get<R: Resource>(&self) -> Result<Res<'_, R>, ResourceError> {
        self.inner
            .try_borrow()
            .map(|any| {
                Res::new(AtomicRef::map(any, |any| unsafe {
                    any.downcast_ref().unwrap_unchecked()
                }))
            })
            .map_err(|_| ResourceError::AlreadyBorrowed(type_name::<R>()))
    }

    /// ## Safety
    ///
    /// The type `R` must match the type in the box.
    unsafe fn get_mut<R: Resource>(
        &self,
    ) -> Result<ResMut<'_, R>, ResourceError> {
        self.inner
            .try_borrow_mut()
            .map(|any| {
                ResMut::new(AtomicRefMut::map(any, |any| unsafe {
                    any.downcast_mut().unwrap_unchecked()
                }))
            })
            .map_err(|_| ResourceError::AlreadyBorrowed(type_name::<R>()))
    }

    /// Consume the box and downcast to a specific resource type.
    ///
    /// # Safety
    ///
    /// The inner type must be `R`.
    unsafe fn into_inner<R: Resource>(self) -> R {
        unsafe { *self.inner.into_inner().downcast().unwrap_unchecked() }
    }
}

impl fmt::Debug for ResourceBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.borrow().as_ref().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Debug, PartialEq)]
    struct Counter(u32);

    #[test]
    fn insert_and_remove() {
        let mut resources = Resources::new();

        assert!(matches!(
            resources.get::<Counter>(),
            Err(ResourceError::NotFound(_)),
        ));

        resources.insert(Counter(123));

        assert_eq!(&*resources.get::<Counter>().unwrap(), &Counter(123));
        assert_eq!(resources.remove::<Counter>().unwrap(), Counter(123));
    }

    #[test]
    fn get() {
        let resource = ResourceBox::new(Counter(0));

        unsafe {
            let _borrow = resource.get::<Counter>().unwrap();

            assert!(resource.get::<Counter>().is_ok());
            assert!(resource.get_mut::<Counter>().is_err());
        }
    }

    #[test]
    fn resource_box_into_inner() {
        let resource = ResourceBox::new(Counter(123));
        let inner = unsafe { resource.into_inner::<Counter>() };

        assert_eq!(inner, Counter(123));
    }
}
