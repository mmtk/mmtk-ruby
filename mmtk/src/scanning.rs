use crate::abi::GCThreadTLS;

use crate::{upcalls, Ruby, RubyEdge};
use mmtk::util::{ObjectReference, VMWorkerThread};
use mmtk::vm::{EdgeVisitor, ObjectTracer, RootsWorkFactory, Scanning};
use mmtk::{Mutator, MutatorContext};

pub struct VMScanning {}

impl Scanning<Ruby> for VMScanning {
    const SINGLE_THREAD_MUTATOR_SCANNING: bool = false;

    fn support_edge_enqueuing(_tls: VMWorkerThread, _object: ObjectReference) -> bool {
        false
    }

    fn scan_object<EV: EdgeVisitor<RubyEdge>>(
        _tls: VMWorkerThread,
        _object: ObjectReference,
        _edge_visitor: &mut EV,
    ) {
        unreachable!("We have not enabled edge enqueuing for any types, yet.");
    }

    fn scan_object_and_trace_edges<OT: ObjectTracer>(
        tls: VMWorkerThread,
        object: ObjectReference,
        object_tracer: &mut OT,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        let visit_object = |_worker, target_object: ObjectReference| {
            trace!("Tracing object: {} -> {}", object, target_object);
            debug_assert!(mmtk::memory_manager::is_mmtk_object(
                target_object.to_address()
            ));
            object_tracer.trace_object(target_object)
        };
        gc_tls
            .object_closure
            .set_temporarily_and_run_code(visit_object, || {
                (upcalls().scan_object_ruby_style)(object);
            });
    }

    fn notify_initial_thread_scan_complete(_partial_scan: bool, _tls: VMWorkerThread) {
        // Do nothing
    }

    fn scan_thread_roots(_tls: VMWorkerThread, _factory: impl RootsWorkFactory<RubyEdge>) {
        unreachable!();
    }

    fn scan_thread_root(
        tls: VMWorkerThread,
        mutator: &'static mut Mutator<Ruby>,
        mut factory: impl RootsWorkFactory<RubyEdge>,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        Self::collect_object_roots_in("scan_thread_root", gc_tls, &mut factory, || {
            (upcalls().scan_thread_root)(mutator.get_tls(), tls);
        });
    }

    fn scan_vm_specific_roots(tls: VMWorkerThread, mut factory: impl RootsWorkFactory<RubyEdge>) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        Self::collect_object_roots_in("scan_vm_specific_roots", gc_tls, &mut factory, || {
            (upcalls().scan_vm_specific_roots)();
        });
    }

    fn supports_return_barrier() -> bool {
        false
    }

    fn prepare_for_roots_re_scanning() {
        todo!()
    }
}

impl VMScanning {
    const OBJECT_BUFFER_SIZE: usize = 4096;

    fn collect_object_roots_in<F: FnMut()>(
        root_scan_kind: &str,
        gc_tls: &mut GCThreadTLS,
        factory: &mut impl RootsWorkFactory<RubyEdge>,
        callback: F,
    ) {
        let mut buffer: Vec<ObjectReference> = Vec::new();
        let visit_object = |_, object: ObjectReference| {
            debug!("[{}] Scanning object: {}", root_scan_kind, object);
            buffer.push(object);
            if buffer.len() >= Self::OBJECT_BUFFER_SIZE {
                factory.create_process_node_roots_work(std::mem::take(&mut buffer));
            }
            object
        };
        gc_tls
            .object_closure
            .set_temporarily_and_run_code(visit_object, callback);
        if !buffer.is_empty() {
            factory.create_process_node_roots_work(buffer);
        }
    }
}
