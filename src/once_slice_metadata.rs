use std::cell::UnsafeCell;
use std::mem::{forget, MaybeUninit};
use std::ptr::null_mut;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize};
use std::{ptr, slice};

/// A synchronization primitive for `[T]` which can be written to only once.
///
/// This is heavily inspired by [`OnceLock`] and tries to follow a mostly similar API.
/// It can be used in statics.
///
/// This holds an additional metadata, which will also be initialized together with the given slice.
///
/// [`OnceLock`]: crate::cell::OnceLock
///
pub struct OnceSlicePtrMetadata<T, M> {
    metadata_flag: AtomicBool,
    metadata: MaybeUninit<UnsafeCell<M>>,
    ptr: AtomicPtr<T>,
    len: AtomicUsize,
}

impl<T, M> Default for OnceSlicePtrMetadata<T, M> {
    /// Returns an unset slice-pointer.
    fn default() -> Self {
        Self::new()
    }
}

impl<T, M> OnceSlicePtrMetadata<T, M> {
    /// Returns an unset slice-pointer.
    pub const fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(null_mut()),
            len: AtomicUsize::new(0),
            metadata: MaybeUninit::uninit(),
            metadata_flag: AtomicBool::new(false),
        }
    }

    /// Tries to set the slice from a [`Box<[T]>`].
    /// [`Box<[T]>`]: std::
    /// Returns:
    /// `Ok(())` if it succeeded.
    /// `Err(Box<[T]>, M)` if it failed, returning the given Box.
    pub fn set(&self, value: (Box<[T]>, M)) -> Result<(), (Box<[T]>, M)> {
        let (mut boxed, metadata) = value;
        let len = boxed.len();
        let ptr = boxed.as_mut_ptr();
        if self
            .ptr
            .compare_exchange(null_mut(), ptr, AcqRel, Acquire)
            .is_err()
        {
            Err((boxed, metadata))
        } else {
            self.len.store(len, Release);
            // SAFETY:
            // compare exchange succeeded, therefore it is safe to write as nobody else can succeed
            unsafe { self.metadata.assume_init_ref().get().write(metadata) };
            forget(boxed);
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
    /// Gets the reference to the metadata.
    ///
    /// Returns `None` if the cell is empty, or being initialized. This method never blocks.
    pub fn get_metadata<'a>(&'a self) -> Option<&'a M> {
        if self.metadata_flag.load(Acquire) {
            // SAFETY:
            // metadata is written and valid as it is only written from a valid M in `set`.
            unsafe { self.metadata.assume_init_ref().get().as_ref::<'a>() }
        } else {
            None
        }

        // SAFETY:
        // Reference is bounded to self, and `self.metadata` gets only dropped when self gets dropped.
        // It is initialized as its created from an M in `set`.
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

    /// Gets the mutable reference to the underlying value.
    ///
    /// Returns `None` if the cell is empty. This method never blocks.
    pub fn get_mut_metadata<'a>(&'a mut self) -> Option<&'a mut M> {
        if self.metadata_flag.load(Acquire) {
            // SAFETY:
            // metadata is written and valid as it is only written from a valid M in `set`.
            // as_mut is sound, because we hold a mutable reference to self, asserting that
            // no other references exist.

            unsafe { self.metadata.assume_init_ref().get().as_mut::<'a>() }
        } else {
            None
        }
    }
}

impl<T, M> Drop for OnceSlicePtrMetadata<T, M> {
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
