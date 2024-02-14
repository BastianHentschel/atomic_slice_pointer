use std::mem::forget;
use std::ptr::null_mut;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use std::sync::atomic::{AtomicPtr, AtomicUsize};
use std::{ptr, slice};

/// A synchronization primitive for `[T]` which can be written to only once.
///
/// This is heavily inspired by [`OnceLock`] and tries to follow a mostly similar API.
///
/// [`OnceLock`]: crate::cell::OnceLock
/// It can be used in statics.
pub struct OnceSlicePtr<T> {
    ptr: AtomicPtr<T>,
    len: AtomicUsize,
}

impl<T> Default for OnceSlicePtr<T> {
    /// Returns an unset [`OnceSlicePtr<T>`].
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OnceSlicePtr<T> {
    /// Returns an unset [`OnceSlicePtr<T>`].
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(null_mut()),
            len: AtomicUsize::new(0),
        }
    }

    /// Tries to set the slice from a [`Box<[T]>`].
    /// [`Box<[T]>`]: std::
    /// Returns:
    /// `Ok(())` if it succeeded.
    /// `Err(Box<[T]>)` if it failed, returning the given Box.
    pub fn set(&self, mut value: Box<[T]>) -> Result<(), Box<[T]>> {
        let len = value.len();
        let ptr = value.as_mut_ptr();
        if self
            .ptr
            .compare_exchange(null_mut(), ptr, AcqRel, Acquire)
            .is_err()
        {
            Err(value)
        } else {
            self.len.store(len, Release);
            forget(value);
            Ok(())
        }
    }

    /// Gets the reference to the underlying value.
    ///
    /// Returns `None` if the cell is empty, or being initialized. This
    /// method never blocks.
    pub fn get(&self) -> Option<&[T]> {
        let ptr = self.ptr.load(Acquire);
        if !ptr.is_null() {
            let len = self.len.load(Acquire);
            if len != 0 {
                // SAFETY:
                // `self.ptr` can only be set via [`try_set`] and therefore came from an owned Box.
                // `self.len` can only be written with the len of the same Box from a [`try_set`]
                Some(unsafe { slice::from_raw_parts(ptr, len) })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Gets the mutable reference to the underlying value.
    ///
    /// Returns `None` if the cell is empty. This method never blocks.
    pub fn get_mut(&mut self) -> Option<&mut [T]> {
        let ptr = self.ptr.load(Acquire);
        if !ptr.is_null() {
            let len = self.len.load(Acquire);
            if len != 0 {
                // SAFETY:
                // `self.ptr` can only be set via [`try_set`] and therefore came from an owned Box.
                // `self.len` can only be written with the len of the same Box from a [`try_set`]
                Some(unsafe { slice::from_raw_parts_mut(ptr, len) })
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<T> Drop for OnceSlicePtr<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.load(Acquire);
        if !ptr.is_null() {
            // SAFETY:
            // `self.ptr` can only be set via [`try_set`] and therefore came from an owned Box.
            // `self.len` must be set, because `self.ptr` was non-null and there are no lingering
            // references because [`drop`] takes a &mut Self, therefore `self.len` has been written
            // in the same [`try_set`] as `self.ptr`.

            unsafe { ptr::slice_from_raw_parts_mut(ptr, self.len.load(Acquire)).drop_in_place() };
        }
    }
}
