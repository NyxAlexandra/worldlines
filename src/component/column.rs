use std::alloc::{self, Layout};
use std::fmt;
use std::ptr::NonNull;

use crate::TypeData;

pub struct Column {
    component: TypeData,
    capacity: usize,
    ptr: NonNull<u8>,
}

impl Column {
    pub fn new(component: TypeData) -> Self {
        let capacity = if component.layout().size() == 0 { usize::MAX } else { 0 };
        let ptr = NonNull::dangling();

        Self { component, capacity, ptr }
    }

    fn is_allocated(&self) -> bool {
        self.ptr != NonNull::dangling()
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> *const u8 {
        debug_assert!(index < self.capacity);

        unsafe {
            self.ptr
                .as_ptr()
                .cast_const()
                .byte_add(self.component.layout().size() * index)
        }
    }

    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> *mut u8 {
        debug_assert!(index < self.capacity);

        unsafe { self.ptr.as_ptr().byte_add(self.component.layout().size() * index) }
    }

    /// Write a pointer to an index.
    ///
    /// The index doesn't need to be in-bounds. Instead, this will grow if
    /// necessary.
    ///
    /// # Safety
    ///
    /// The pointer must be to a valid instance of the type held by this column.
    pub unsafe fn write(&mut self, index: usize, ptr: *mut u8) {
        if index >= self.capacity {
            self.grow(index - self.capacity + 1);
        }

        unsafe {
            // SAFETY: the index is guaranteed to be within capacity
            self.get_unchecked_mut(index)
                // SAFETY: the user guarantees that the pointers don't overlay
                .copy_from_nonoverlapping(ptr, self.component.layout().size());
        }
    }

    /// Drop the component at the index.
    ///
    /// # Safety
    ///
    /// - The index must be in-bounds.
    /// - This must not be used on an already-dropped or uninitialzed component.
    pub unsafe fn free(&mut self, index: usize) {
        unsafe {
            self.component.drop()(self.get_unchecked_mut(index));
        }
    }

    pub fn grow(&mut self, additional: usize) {
        // TODO: optimize allocation strategy
        self.grow_exact(additional);
    }

    pub fn grow_exact(&mut self, additional: usize) {
        if self.component.layout().size() == 0 {
            return;
        }

        let new_capacity = self.capacity + additional;
        let new_layout = array(self.component.layout(), new_capacity);

        if self.is_allocated() {
            let old_layout = array(self.component.layout(), self.capacity);

            self.ptr = NonNull::new(unsafe {
                alloc::realloc(self.ptr.as_ptr(), old_layout, new_layout.size())
            })
            .expect("global allocation failure");
        } else {
            self.ptr = NonNull::new(unsafe { alloc::alloc(new_layout) })
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
    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);

    len_rounded_up.wrapping_sub(len)
}

impl Drop for Column {
    fn drop(&mut self) {
        if self.is_allocated() {
            unsafe {
                alloc::dealloc(
                    self.ptr.as_ptr(),
                    array(self.component.layout(), self.capacity),
                )
            };
        }
    }
}

impl fmt::Debug for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `Column<T>`
        f.debug_struct(&format!("Column<{}>", self.component))
            .field("capacity", &self.capacity)
            .field("ptr", &self.ptr)
            .finish()
    }
}
