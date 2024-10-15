use std::any::{Any, TypeId};
use std::fmt;

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};

use super::{Res, ResMut, Resource, ResourceError};
use crate::TypeMap;

/// Storage for resources.
#[derive(Debug)]
pub struct Resources {
    inner: TypeMap<ResourceBox>,
}

struct ResourceBox {
    inner: AtomicRefCell<Box<dyn Any>>,
}

impl Resources {
    pub fn new() -> Self {
        let inner = TypeMap::default();

        Self { inner }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn contains<R: Resource>(&self) -> bool {
        self.contains_id(TypeId::of::<R>())
    }

    pub fn contains_id(&self, type_id: TypeId) -> bool {
        self.inner.contains_key(&type_id)
    }

    pub fn get<R: Resource>(&self) -> Result<Res<'_, R>, ResourceError> {
        self.inner
            .get(&TypeId::of::<R>())
            .ok_or(ResourceError::not_found::<R>())
            .and_then(|boxed| unsafe { boxed.get() })
    }

    pub fn get_mut<R: Resource>(&self) -> Result<ResMut<'_, R>, ResourceError> {
        self.inner
            .get(&TypeId::of::<R>())
            .ok_or(ResourceError::not_found::<R>())
            .and_then(|boxed| unsafe { boxed.get_mut() })
    }

    pub fn insert<R: Resource>(&mut self, resource: R) {
        self.inner.insert(TypeId::of::<R>(), ResourceBox::new(resource));
    }

    pub fn remove<R: Resource>(&mut self) -> Result<R, ResourceError> {
        self.inner
            .remove(&TypeId::of::<R>())
            .map(|boxed| unsafe { boxed.into_inner().unwrap_unchecked() })
            .ok_or(ResourceError::not_found::<R>())
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
            .map_err(|_| ResourceError::already_borrowed::<R>())
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
            .map_err(|_| ResourceError::already_borrowed::<R>())
    }

    /// Consume the box and downcast to a specific resource type.
    ///
    /// Returns `Err(self)` if the types don't match.
    fn into_inner<R: Resource>(self) -> Result<R, Self> {
        self.inner
            .into_inner()
            .downcast()
            // [`CoerceUnsized`]
            .map(|boxed| *boxed)
            .map_err(|boxed| Self { inner: AtomicRefCell::new(boxed) })
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

    #[test]
    fn resource_box_get() {
        let resource = ResourceBox::new(true);

        unsafe {
            let _borrow = resource.get::<bool>().unwrap();

            assert!(resource.get::<bool>().is_ok());
            assert!(resource.get_mut::<bool>().is_err());
        }
    }

    #[test]
    fn resource_box_into_inner() {
        let resource = ResourceBox::new::<i32>(123);
        let resource = resource.into_inner::<u32>().unwrap_err();

        assert_eq!(resource.into_inner::<i32>().ok(), Some(123));
    }
}
