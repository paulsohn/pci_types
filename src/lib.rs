#![no_std]
#![feature(
    maybe_uninit_as_bytes,
    maybe_uninit_uninit_array,
    maybe_uninit_uninit_array_transpose,
    maybe_uninit_array_assume_init,
    maybe_uninit_slice,
    maybe_uninit_write_slice,
    slice_as_chunks,
    trait_alias,
)]

pub mod address;
pub use address::{
    PciAddress,
    DwordAccessMethod
};

#[deprecated = "Renamed. Use `ConfigRegionAccessMethod` instead."]
pub trait ConfigRegionAccess = DwordAccessMethod;

pub mod access;
pub mod accessor;
pub use accessor::{AccessorTrait, DwordAccessor};

pub mod dwords;
pub mod headers;
pub use headers::*;

pub mod device_type;

pub mod capability;
