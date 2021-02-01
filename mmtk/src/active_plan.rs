use mmtk::{Plan, SelectedPlan};
use mmtk::vm::ActivePlan;
use mmtk::util::OpaquePointer;
use mmtk::scheduler::*;
use crate::Ruby;
use crate::SINGLETON;

pub struct VMActivePlan<> {}

impl ActivePlan<Ruby> for VMActivePlan {
    fn global() -> &'static SelectedPlan<Ruby> {
        &SINGLETON.plan
    }

    unsafe fn worker(_tls: OpaquePointer) -> &'static mut GCWorker<Ruby> {
        unimplemented!()
    }

    fn number_of_mutators() -> usize {
        unimplemented!()
    }

    unsafe fn is_mutator(_tls: OpaquePointer) -> bool {
        // FIXME
        true
    }

    unsafe fn mutator(_tls: OpaquePointer) -> &'static mut <SelectedPlan<Ruby> as Plan>::Mutator {
        unimplemented!()
    }

    fn reset_mutator_iterator() {
        unimplemented!()
    }

    fn get_next_mutator() -> Option<&'static mut <SelectedPlan<Ruby> as Plan>::Mutator> {
        unimplemented!()
    }
}