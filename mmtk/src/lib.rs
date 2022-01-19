extern crate mmtk;
extern crate libc;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

use binding::RubyBinding;
use mmtk::vm::VMBinding;
use mmtk::MMTK;

pub mod scanning;
pub mod collection;
pub mod object_model;
pub mod active_plan;
pub mod reference_glue;
pub mod abi;
pub mod api;
pub mod finalize;
pub mod binding;

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
