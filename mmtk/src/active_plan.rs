use std::marker::PhantomData;

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
        (upcalls().is_mutator)()
    }

    fn mutator(_tls: VMMutatorThread) -> &'static mut Mutator<Ruby> {
        unimplemented!()
    }

    fn mutators<'a>() -> Box<dyn Iterator<Item = &'a mut Mutator<Ruby>> + 'a> {
        let mut mutators = vec![];
        (upcalls().get_mutators)(
            add_mutator_to_vec,
            &mut mutators as *mut Vec<*mut Mutator<Ruby>> as _,
        );

        Box::new(RubyMutatorIterator {
            mutators,
            cursor: 0,
            phantom_data: PhantomData,
        })
    }
}

extern "C" fn add_mutator_to_vec(mutator: *mut Mutator<Ruby>, mutators: *mut libc::c_void) {
    let mutators = unsafe { &mut *(mutators as *mut Vec<*mut Mutator<Ruby>>) };
    mutators.push(mutator);
}

struct RubyMutatorIterator<'a> {
    mutators: Vec<*mut Mutator<Ruby>>,
    cursor: usize,
    phantom_data: PhantomData<&'a ()>,
}

impl<'a> Iterator for RubyMutatorIterator<'a> {
    type Item = &'a mut Mutator<Ruby>;

    fn next(&mut self) -> Option<Self::Item> {
        self.mutators.get(self.cursor).cloned().map(|mutator_ptr| {
            self.cursor += 1;
            unsafe { &mut *mutator_ptr as _ }
        })
    }
}
