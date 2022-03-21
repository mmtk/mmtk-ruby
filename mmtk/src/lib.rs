#![feature(vec_into_raw_parts)]
#![feature(const_ptr_offset_from)]

extern crate libc;
extern crate mmtk;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate memoffset;

use binding::RubyBinding;
use mmtk::vm::VMBinding;
use mmtk::MMTK;

pub mod abi;
pub mod active_plan;
pub mod address_buffer;
pub mod api;
pub mod binding;
pub mod collection;
pub mod finalize;
pub mod gc_work;
pub mod object_model;
pub mod reference_glue;
pub mod scanning;

#[derive(Default)]
pub struct Ruby;

impl VMBinding for Ruby {
    type VMObjectModel = object_model::VMObjectModel;
    type VMScanning = scanning::VMScanning;
    type VMCollection = collection::VMCollection;
    type VMActivePlan = active_plan::VMActivePlan;
    type VMReferenceGlue = reference_glue::VMReferenceGlue;
}

lazy_static! {
    pub static ref SINGLETON: MMTK<Ruby> = MMTK::new();
    pub static ref BINDING: RubyBinding = RubyBinding::new();
}

pub static mut UPCALLS: *const abi::RubyUpcalls = std::ptr::null();

pub fn binding() -> &'static RubyBinding {
    &BINDING
}

pub fn upcalls() -> &'static abi::RubyUpcalls {
    unsafe { &*UPCALLS }
}
