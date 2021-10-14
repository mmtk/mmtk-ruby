use mmtk::util::{VMMutatorThread,VMWorkerThread, VMThread};
use mmtk::vm::Collection;
use mmtk::MutatorContext;
use mmtk::scheduler::*;
use crate::Ruby;

pub struct VMCollection {}

impl Collection<Ruby> for VMCollection {
    fn stop_all_mutators<E: ProcessEdgesWork<VM=Ruby>>(_tls: VMWorkerThread) {
        unimplemented!()
    }

    fn resume_mutators(_tls: VMWorkerThread) {
        unimplemented!()
    }

    fn block_for_gc(_tls: VMMutatorThread) {
        unimplemented!();
    }

    fn spawn_worker_thread(tls: VMThread, ctx: Option<&GCWorker<Ruby>>) {
        unimplemented!();
    }

    fn prepare_mutator<T: MutatorContext<Ruby>>(
        tls_worker: VMWorkerThread,
        tls_mutator: VMMutatorThread,
        m: &T,
    ) {
        unimplemented!()
    }
}
