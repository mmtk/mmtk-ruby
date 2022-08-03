extern crate libc;
extern crate mmtk;
#[macro_use]
extern crate log;

use abi::RubyUpcalls;
use binding::RubyBinding;
use mmtk::vm::VMBinding;
use mmtk::MMTK;
use once_cell::sync::OnceCell;

pub mod abi;
pub mod active_plan;
pub mod api;
pub mod binding;
pub mod collection;
pub mod finalize;
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

pub static BINDING: OnceCell<RubyBinding> = OnceCell::new();

pub fn binding<'b>() -> &'b RubyBinding {
    BINDING.get().expect("Attempt to use the binding before it is initialization")
}

pub fn mmtk() -> &'static MMTK<Ruby> {
    binding().mmtk
}

pub fn upcalls() -> &'static RubyUpcalls {
    binding().upcalls()
}
