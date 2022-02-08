use mmtk::util::{VMThread, VMMutatorThread, VMWorkerThread};
use mmtk::Mutator;
use crate::Ruby;

#[repr(C)]
#[derive(Clone)]
pub struct RubyUpcalls {
    pub init_gc_worker_thread: extern "C" fn (main_tls: VMThread) -> VMWorkerThread,
    pub stop_the_world: extern "C" fn (tls: VMWorkerThread),
    pub resume_mutators: extern "C" fn (tls: VMWorkerThread),
    pub block_for_gc: extern "C" fn (tls: VMMutatorThread),
    pub reset_mutator_iterator: extern "C" fn (),
    pub get_next_mutator: extern "C" fn () -> *mut Mutator<Ruby>,
}

unsafe impl Sync for RubyUpcalls {}
