use mmtk::vm::Collection;
use mmtk::MutatorContext;
use mmtk::util::OpaquePointer;
use mmtk::MMTK;
use mmtk::scheduler::*;
use mmtk::scheduler::gc_works::*;
use crate::Ruby;

pub struct VMCollection {}

impl Collection<Ruby> for VMCollection {
    fn stop_all_mutators<E: ProcessEdgesWork<VM=Ruby>>(_tls: OpaquePointer) {
        unimplemented!()
    }

    fn resume_mutators(_tls: OpaquePointer) {
        unimplemented!()
    }

    fn block_for_gc(_tls: OpaquePointer) {
        unimplemented!();
    }

    fn spawn_worker_thread(_tls: OpaquePointer, _ctx: Option<&Worker<MMTK<Ruby>>>) {
        unimplemented!();
    }

    fn prepare_mutator<T: MutatorContext<Ruby>>(_tls: OpaquePointer, _mutator: &T) {
        unimplemented!()
    }
}