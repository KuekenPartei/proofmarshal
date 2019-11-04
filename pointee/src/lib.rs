//! Targets of pointers.

#![feature(slice_from_raw_parts)]
#![feature(alloc_layout_extra)]


use core::fmt;
use core::hash::Hash;
use core::ptr::NonNull;
use core::mem::{self, MaybeUninit};

use core::alloc::Layout;

pub mod slice;

/// A target of a pointer.
///
/// # Safety
///
/// Other code can assume `Pointee` is implemented correctly.
pub unsafe trait Pointee {
    /// Fat pointer metadata.
    type Metadata : Sized + Copy + fmt::Debug + Eq + Ord + Hash;

    fn ptr_metadata(&self) -> Self::Metadata;

    /// Makes the metadata for a sized type.
    ///
    /// Sized types have no metadata, so this is always possible.
    fn make_sized_metadata() -> Self::Metadata
        where Self: Sized
    {
        unreachable!()
    }

    /// Makes a fat pointer from a thin pointer.
    fn make_fat_ptr(thin: *const (), metadata: Self::Metadata) -> *const Self;

    /// Makes a mutable fat pointer from a thin pointer.
    fn make_fat_ptr_mut(thin: *mut (), metadata: Self::Metadata) -> *mut Self;

    /// Makes a fat `NonNull` from a thin `NonNull`.
    #[inline(always)]
    fn make_fat_non_null(thin: NonNull<()>, metadata: Self::Metadata) -> NonNull<Self> {
        let p: *mut Self = Self::make_fat_ptr_mut(thin.as_ptr(), metadata);
        unsafe {
            NonNull::new_unchecked(p)
        }
    }

    /// Computes alignment from metadata.
    fn align(metadata: Self::Metadata) -> usize;
}

/// A type whose size can be computed at runtime from pointer metadata.
///
/// # Safety
///
/// Other code can assume `DynSized` is implemented correctly.
pub unsafe trait DynSized : Pointee {
    /// Computes the size from the metadata.
    fn size(metadata: Self::Metadata) -> usize;

    /// Computes a `Layout` from the metadata.
    #[inline(always)]
    fn layout(metadata: Self::Metadata) -> Layout {
        let size = Self::size(metadata);
        let align = Self::align(metadata);
        unsafe {
            Layout::from_size_align_unchecked(size, align)
        }
    }
}

unsafe impl<T> Pointee for T {
    type Metadata = ();

    fn ptr_metadata(&self) -> Self::Metadata {
        Self::make_sized_metadata()
    }

    fn make_sized_metadata() -> Self::Metadata {
        unsafe {
            MaybeUninit::uninit().assume_init()
        }
    }

    #[inline(always)]
    fn make_fat_ptr(thin: *const (), _: Self::Metadata) -> *const Self {
        thin as *const Self
    }

    #[inline(always)]
    fn make_fat_ptr_mut(thin: *mut (), _: Self::Metadata) -> *mut Self {
        thin as *mut Self
    }

    #[inline(always)]
    fn align(_: ()) -> usize {
        mem::align_of::<Self>()
    }
}

unsafe impl<T> DynSized for T {
    #[inline(always)]
    fn size(_: ()) -> usize {
        mem::size_of::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sized_metadata() {
        let _:() = ().ptr_metadata();
    }
}