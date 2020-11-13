#![feature(never_type)]

#![feature(unwrap_infallible)]
#![feature(arbitrary_self_types)]

#![feature(rustc_attrs)]

#![allow(incomplete_features)]
#![feature(const_generics)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

pub mod owned;

pub mod maybevalid;

pub mod pointee;
pub mod blob;

pub mod ptr;

pub mod load;
pub mod save;

pub mod primitive;
pub mod bag;

pub mod prelude {
    pub use super::{
        bag::Bag,
        pointee::Pointee,
        ptr::{
            Ptr,
            TryGet, TryGetMut,
            Get, GetMut,
            heap::Heap,
            key::{
                Key, KeyMut,
            },
        },
        load::{
            Load, LoadRef,
        },
        maybevalid::MaybeValid,
    };
}
