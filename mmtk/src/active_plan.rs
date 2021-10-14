use mmtk::Mutator;
use mmtk::Plan;
use mmtk::vm::ActivePlan;
use mmtk::util::opaque_pointer::*;
use crate::Ruby;
use crate::SINGLETON;

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
        unimplemented!()
    }

    fn get_next_mutator() -> Option<&'static mut Mutator<Ruby>> {
        unimplemented!()
    }
}
