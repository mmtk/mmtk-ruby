use mmtk::Mutator;
use mmtk::Plan;
use mmtk::vm::ActivePlan;
use mmtk::util::opaque_pointer::*;
use crate::Ruby;
use crate::SINGLETON;
use crate::upcalls;

pub struct VMActivePlan<> {}

impl ActivePlan<Ruby> for VMActivePlan {
    fn global() -> &'static dyn Plan<VM = Ruby> {
        SINGLETON.get_plan()
    }

    fn number_of_mutators() -> usize {
        unimplemented!()
    }

    fn is_mutator(tls: VMThread) -> bool {
        // FIXME
        true
    }

    fn mutator(tls: VMMutatorThread) -> &'static mut Mutator<Ruby> {
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
