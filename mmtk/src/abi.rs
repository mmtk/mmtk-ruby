use mmtk::util::{VMThread, VMMutatorThread, VMWorkerThread};

#[repr(C)]
#[derive(Clone)]
pub struct RubyUpcalls {
    pub init_gc_worker_thread: extern "C" fn (main_tls: VMThread) -> VMWorkerThread,
    pub stop_the_world: extern "C" fn (tls: VMWorkerThread),
    pub resume_mutators: extern "C" fn (tls: VMWorkerThread),
    pub block_for_gc: extern "C" fn (tls: VMMutatorThread),
}

unsafe impl Sync for RubyUpcalls {}
