use mmtk::util::{VMMutatorThread,VMWorkerThread, VMThread, Address, OpaquePointer};
use mmtk::vm::{Collection, GCThreadContext};
use mmtk::{MutatorContext, memory_manager};
use mmtk::scheduler::*;
use crate::abi;
use crate::abi::GCThreadTLS;
use crate::address_buffer::AddressBuffer;
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

    fn spawn_gc_thread(_tls: VMThread, ctx: GCThreadContext<Ruby>) {
        match ctx {
            GCThreadContext::Controller(mut controller) => {
                thread::Builder::new().name("MMTk Controller Thread".to_string()).spawn(move || {
                    debug!("Hello! This is MMTk Controller Thread running!");
                    let ptr_controller = &mut *controller as *mut GCController<Ruby>;
                    let gc_thread_tls = Box::into_raw(Box::new(GCThreadTLS::for_controller(ptr_controller)));
                    (upcalls().init_gc_worker_thread)(gc_thread_tls);
                    memory_manager::start_control_collector(&SINGLETON, GCThreadTLS::to_vwt(gc_thread_tls), &mut controller)
                }).unwrap();
            }
            GCThreadContext::Worker(mut worker) => {
                thread::Builder::new().name("MMTk Worker Thread".to_string()).spawn(move || {
                    debug!("Hello! This is MMTk Worker Thread running!");
                    let ptr_worker = &mut *worker as *mut GCWorker<Ruby>;
                    let gc_thread_tls = Box::into_raw(Box::new(GCThreadTLS::for_worker(ptr_worker)));
                    (upcalls().init_gc_worker_thread)(gc_thread_tls);
                    memory_manager::start_worker(&SINGLETON, GCThreadTLS::to_vwt(gc_thread_tls), &mut worker)
                }).unwrap();
            }
        }
    }

    fn prepare_mutator<T: MutatorContext<Ruby>>(
        tls_worker: VMWorkerThread,
        tls_mutator: VMMutatorThread,
        m: &T,
    ) {
        // do nothing
    }
}
