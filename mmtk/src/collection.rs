use crate::abi::GCThreadTLS;

use crate::{binding, mmtk, upcalls, Ruby};
use mmtk::scheduler::*;
use mmtk::util::{VMMutatorThread, VMThread, VMWorkerThread};
use mmtk::vm::{Collection, GCThreadContext, ProcessWeakRefsContext};
use mmtk::{memory_manager, MutatorContext};
use std::thread;

pub struct VMCollection {}

impl Collection<Ruby> for VMCollection {
    const COORDINATOR_ONLY_STW: bool = true;

    fn stop_all_mutators<F>(tls: VMWorkerThread, _mutator_visitor: F)
    where
        F: FnMut(&'static mut mmtk::Mutator<Ruby>),
    {
        (upcalls().stop_the_world)(tls);
    }

    fn resume_mutators(tls: VMWorkerThread) {
        (upcalls().resume_mutators)(tls);
    }

    fn block_for_gc(tls: VMMutatorThread) {
        (upcalls().block_for_gc)(tls);
    }

    fn spawn_gc_thread(_tls: VMThread, ctx: GCThreadContext<Ruby>) {
        match ctx {
            GCThreadContext::Controller(mut controller) => {
                thread::Builder::new()
                    .name("MMTk Controller Thread".to_string())
                    .spawn(move || {
                        debug!("Hello! This is MMTk Controller Thread running!");
                        let ptr_controller = &mut *controller as *mut GCController<Ruby>;
                        let gc_thread_tls =
                            Box::into_raw(Box::new(GCThreadTLS::for_controller(ptr_controller)));
                        (upcalls().init_gc_worker_thread)(gc_thread_tls);
                        memory_manager::start_control_collector(
                            mmtk(),
                            GCThreadTLS::to_vwt(gc_thread_tls),
                            &mut controller,
                        )
                    })
                    .unwrap();
            }
            GCThreadContext::Worker(mut worker) => {
                thread::Builder::new()
                    .name("MMTk Worker Thread".to_string())
                    .spawn(move || {
                        debug!("Hello! This is MMTk Worker Thread running!");
                        let ptr_worker = &mut *worker as *mut GCWorker<Ruby>;
                        let gc_thread_tls =
                            Box::into_raw(Box::new(GCThreadTLS::for_worker(ptr_worker)));
                        (upcalls().init_gc_worker_thread)(gc_thread_tls);
                        memory_manager::start_worker(
                            mmtk(),
                            GCThreadTLS::to_vwt(gc_thread_tls),
                            &mut worker,
                        )
                    })
                    .unwrap();
            }
        }
    }

    fn prepare_mutator<T: MutatorContext<Ruby>>(
        _tls_worker: VMWorkerThread,
        _tls_mutator: VMMutatorThread,
        _m: &T,
    ) {
        // do nothing
    }

    fn process_weak_refs(_tls: VMWorkerThread, context: impl ProcessWeakRefsContext) -> bool {
        binding().weak_proc.process_weak_stuff(context);
        false
    }
}
