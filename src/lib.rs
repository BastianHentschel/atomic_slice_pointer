//! Thread-safe, lock-free, and atomic slice-pointers.
#![deny(missing_docs)]
#![deny(clippy::undocumented_unsafe_blocks)]

mod once_slice;
mod once_slice_metadata;

pub use once_slice::OnceSlicePtr;
pub use once_slice_metadata::OnceSlicePtrMetadata;