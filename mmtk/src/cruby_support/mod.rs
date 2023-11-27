//! This module contains descriptions of the layouts of objects in CRuby.
//!
//! This module contains files stolen from CRuby (2-clause BSD licensed).
//! In the long run, we should share code properly with YJIT in CRuby.

use mmtk::util::{Address, ObjectReference};

use self::cruby::VALUE;

#[allow(unused)]
#[allow(clippy::all)]
pub mod cruby;
pub mod cruby_extra;
pub mod flag_tests;
pub mod mmtk_extra;

impl From<ObjectReference> for VALUE {
    fn from(value: ObjectReference) -> Self {
        Self(value.to_raw_address().as_usize())
    }
}

impl From<VALUE> for ObjectReference {
    fn from(value: VALUE) -> Self {
        Self::from_raw_address(Address::from(value))
    }
}

impl From<VALUE> for Address {
    fn from(value: VALUE) -> Self {
        unsafe { Self::from_usize(value.0) }
    }
}
