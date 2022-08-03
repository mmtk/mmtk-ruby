use crate::mmtk;
use crate::upcalls;
use crate::Ruby;
use mmtk::util::opaque_pointer::*;
use mmtk::vm::ActivePlan;
use mmtk::Mutator;
use mmtk::Plan;

pub struct VMActivePlan {}

impl ActivePlan<Ruby> for VMActivePlan {
    fn global() -> &'static dyn Plan<VM = Ruby> {
        mmtk().get_plan()
    }

    fn number_of_mutators() -> usize {
        (upcalls().number_of_mutators)()
    }

    fn is_mutator(_tls: VMThread) -> bool {
        // FIXME
        true
    }

    fn mutator(_tls: VMMutatorThread) -> &'static mut Mutator<Ruby> {
        unimplemented!()
    }

    fn reset_mutator_iterator() {
        (upcalls().reset_mutator_iterator)();
    }

    fn get_next_mutator() -> Option<&'static mut Mutator<Ruby>> {
        let ptr = (upcalls().get_next_mutator)();
        if ptr == std::ptr::null_mut() {
            None
        } else {
            Some(unsafe { &mut (*ptr) as &'static mut Mutator<Ruby> })
        }
    }
}
