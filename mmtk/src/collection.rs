use mmtk::util::{VMMutatorThread,VMWorkerThread, VMThread};
use mmtk::vm::{Collection, GCThreadContext};
use mmtk::{MutatorContext, memory_manager};
use mmtk::scheduler::*;
use crate::{Ruby, SINGLETON, upcalls};
use std::thread;

pub struct VMCollection {}

impl Collection<Ruby> for VMCollection {
    fn stop_all_mutators<E: ProcessEdgesWork<VM=Ruby>>(tls: VMWorkerThread) {
        (upcalls().stop_the_world)(tls);
    }

    fn resume_mutators(tls: VMWorkerThread) {
        (upcalls().resume_mutators)(tls);
    }

    fn block_for_gc(tls: VMMutatorThread) {
        (upcalls().block_for_gc)(tls);
    }

    fn spawn_gc_thread(tls: VMThread, ctx: GCThreadContext<Ruby>) {
        match ctx {
            GCThreadContext::Controller(mut controller) => {
                thread::Builder::new().name("MMTk Controller Thread".to_string()).spawn(move || {
                    debug!("Hello! This is MMTk Controller Thread running!");
                    let my_tls = (upcalls().init_gc_worker_thread)(tls);
                    memory_manager::start_control_collector(&SINGLETON, my_tls, &mut controller)
                }).unwrap();
            }
            GCThreadContext::Worker(mut worker) => {
                thread::Builder::new().name("MMTk Worker Thread".to_string()).spawn(move || {
                    debug!("Hello! This is MMTk Worker Thread running!");
                    let my_tls = (upcalls().init_gc_worker_thread)(tls);
                    memory_manager::start_worker(&SINGLETON, my_tls, &mut worker)
                }).unwrap();            
            }
        }
    }

    fn prepare_mutator<T: MutatorContext<Ruby>>(
        tls_worker: VMWorkerThread,
        tls_mutator: VMMutatorThread,
        m: &T,
    ) {
        unimplemented!()
    }
}
