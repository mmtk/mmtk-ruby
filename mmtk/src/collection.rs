use mmtk::util::{VMMutatorThread,VMWorkerThread, VMThread};
use mmtk::vm::Collection;
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

    fn spawn_worker_thread(tls: VMThread, ctx: Option<Box<GCWorker<Ruby>>>) {
        match ctx {
            None => {
                thread::Builder::new().name("MMTk Controller Thread".to_string()).spawn(move || {
                    debug!("Hello! This is MMTk Controller Thread running!");
                    let my_tls = (upcalls().init_gc_worker_thread)(tls);
                    memory_manager::start_control_collector(&SINGLETON, my_tls)
                }).unwrap();
            }
            Some(mut worker) => {
                thread::Builder::new().name("MMTk Worker Thread".to_string()).spawn(move || {
                    debug!("Hello! This is MMTk Worker Thread running!");
                    let my_tls = (upcalls().init_gc_worker_thread)(tls);
                    memory_manager::start_worker(my_tls, &mut worker, &SINGLETON)
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
