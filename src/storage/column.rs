use std::alloc::{alloc, dealloc, realloc, Layout};
use std::fmt;
use std::ptr::NonNull;

use super::{SparseIndex, TableRow};
use crate::component::ComponentInfo;
use crate::prelude::ComponentVTable;

/// Storage for a single component type.
pub struct Column {
    component: ComponentInfo,
    capacity: usize,
    ptr: NonNull<u8>,
}

impl Column {
    /// Creates an empty column without allocating.
    pub fn new(component: ComponentInfo) -> Self {
        let capacity =
            if component.layout().size() == 0 { usize::MAX } else { 0 };
        let ptr = NonNull::dangling();

        Self { component, capacity, ptr }
    }

    /// Creates a new column with at least the specified capacity.
    pub fn with_capacity(component: ComponentInfo, capacity: usize) -> Self {
        let mut new = Self::new(component);

        new.grow(capacity);

        new
    }

    fn is_allocated(&self) -> bool {
        self.ptr != NonNull::dangling()
    }

    /// Returns a pointer to the component for a row.
    ///
    /// Returns `None` if the entity is not within bounds.
    pub fn get_mut(&mut self, row: TableRow) -> Option<NonNull<u8>> {
        if row.sparse_index() < self.capacity {
            Some(unsafe { self.get_unchecked_mut(row) })
        } else {
            None
        }
    }

    /// Returns a pointer to the component for a row.
    ///
    /// # Safety
    ///
    /// The entity's index must be within bounds.
    pub unsafe fn get_unchecked(&self, row: TableRow) -> NonNull<u8> {
        let index = row.sparse_index();

        debug_assert!(index < self.capacity);

        unsafe { self.ptr.byte_add(self.component.layout().size() * index) }
    }

    /// Returns a pointer to the component for a row.
    ///
    /// # Safety
    ///
    /// The entity's index must be within bounds.
    pub unsafe fn get_unchecked_mut(&mut self, row: TableRow) -> NonNull<u8> {
        let index = row.sparse_index();

        debug_assert!(index < self.capacity);

        unsafe { self.ptr.byte_add(self.component.layout().size() * index) }
    }

    /// Returns a pointer to the component data for an entity, allocating if
    /// there is not enough capacity.
    pub fn get_or_alloc(&mut self, row: TableRow) -> NonNull<u8> {
        self.get_mut(row).unwrap_or_else(|| {
            self.grow(row.0 - self.capacity + 1);

            // SAFETY: we ensure that the entity's index is within bounds with
            // the above grow
            unsafe { self.get_unchecked_mut(row) }
        })
    }

    /// Writes a component to a row from a component pointer.
    ///
    /// Will reallocate if the row is out of bounds.
    ///
    /// # Safety
    ///
    /// The pointer must refer to a valid instance of the component this column
    /// was created for, and must not overlap.
    pub unsafe fn write(&mut self, row: TableRow, ptr: NonNull<u8>) {
        unsafe {
            self.get_or_alloc(row)
                .copy_from_nonoverlapping(ptr, self.component.layout().size());
        }
    }

    /// Drops a component at a row.
    ///
    /// # Safety
    ///
    /// The component must have been allocated and not already dropped.
    pub unsafe fn free(&mut self, row: TableRow) -> Option<()> {
        if let Some(ptr) = self.get_mut(row) {
            let drop = self.component.drop();

            unsafe { drop(ptr.as_ptr()) };

            Some(())
        } else {
            None
        }
    }

    /// Grows storage by at least an amount.
    pub fn grow(&mut self, additional: usize) {
        // if ZST
        if self.capacity == usize::MAX {
            return;
        }

        // TODO: optimize allocation strategy
        let new_capacity = (self.capacity + additional)
            .max(self.capacity.checked_mul(2).unwrap_or_default());
        let new_layout = array(self.component.layout(), new_capacity);

        if self.is_allocated() {
            let old_layout = array(self.component.layout(), self.capacity);

            self.ptr = NonNull::new(unsafe {
                realloc(self.ptr.as_ptr(), old_layout, new_layout.size())
            })
            .expect("global allocation failure");
        } else {
            self.ptr = NonNull::new(unsafe { alloc(new_layout) })
                .expect("global allocation failure");
        }

        self.capacity = new_capacity;
    }
}

/// The layout of an array of items size `n`.
fn array(layout: Layout, n: usize) -> Layout {
    // from [Bevy](https://github.com/bevyengine/bevy/blob/dcb191bb1837027156584260c3999558dd6368c0/crates/bevy_ecs/src/storage/blob_vec.rs#L457).

    let align = layout.align();
    let size = (layout.size() + padding_needed_for(layout, align)) * n;

    Layout::from_size_align(size, align).unwrap()
}

fn padding_needed_for(layout: Layout, align: usize) -> usize {
    let len = layout.size();
    let len_rounded_up =
        len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);

    len_rounded_up.wrapping_sub(len)
}

impl Drop for Column {
    fn drop(&mut self) {
        if self.is_allocated() {
            unsafe {
                dealloc(
                    self.ptr.as_ptr(),
                    array(self.component.layout(), self.capacity),
                )
            };
        }
    }
}

impl fmt::Debug for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Column<{}>", self.component))
            .finish_non_exhaustive()
    }
}
