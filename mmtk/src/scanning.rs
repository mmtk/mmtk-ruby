use crate::abi::GCThreadTLS;

use crate::{upcalls, Ruby, RubySlot};
use mmtk::scheduler::GCWorker;
use mmtk::util::{ObjectReference, VMWorkerThread};
use mmtk::vm::{ObjectTracer, RootsWorkFactory, Scanning, SlotVisitor};
use mmtk::{Mutator, MutatorContext};

pub struct VMScanning {}

impl Scanning<Ruby> for VMScanning {
    fn support_slot_enqueuing(_tls: VMWorkerThread, _object: ObjectReference) -> bool {
        false
    }

    fn scan_object<EV: SlotVisitor<RubySlot>>(
        _tls: VMWorkerThread,
        _object: ObjectReference,
        _slot_visitor: &mut EV,
    ) {
        unreachable!("We have not enabled slot enqueuing for any types, yet.");
    }

    fn scan_object_and_trace_edges<OT: ObjectTracer>(
        tls: VMWorkerThread,
        object: ObjectReference,
        object_tracer: &mut OT,
    ) {
        debug_assert!(
            mmtk::memory_manager::is_mmtk_object(object.to_raw_address()),
            "Not an MMTk object: {object}",
        );
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        let visit_object = |_worker, target_object: ObjectReference, pin| {
            trace!(
                "Tracing edge: {} -> {}{}",
                object,
                target_object,
                if pin { " pin" } else { "" }
            );
            debug_assert!(
                mmtk::memory_manager::is_mmtk_object(target_object.to_raw_address()),
                "Destination is not an MMTk object. Src: {object} dst: {target_object}"
            );
            let forwarded_target = object_tracer.trace_object(target_object);
            if forwarded_target != target_object {
                trace!(
                    "  Forwarded target {} -> {}",
                    target_object,
                    forwarded_target
                );
            }
            forwarded_target
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

    fn scan_roots_in_mutator_thread(
        tls: VMWorkerThread,
        mutator: &'static mut Mutator<Ruby>,
        mut factory: impl RootsWorkFactory<RubySlot>,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        Self::collect_object_roots_in("scan_thread_root", gc_tls, &mut factory, || {
            (upcalls().scan_roots_in_mutator_thread)(mutator.get_tls(), tls);
        });
    }

    fn scan_vm_specific_roots(tls: VMWorkerThread, mut factory: impl RootsWorkFactory<RubySlot>) {
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

    fn process_weak_refs(
        worker: &mut GCWorker<Ruby>,
        tracer_context: impl mmtk::vm::ObjectTracerContext<Ruby>,
    ) -> bool {
        crate::binding()
            .weak_proc
            .process_weak_stuff(worker, tracer_context);
        crate::binding().ppp_registry.cleanup_ppps();
        false
    }

    fn forward_weak_refs(
        _worker: &mut GCWorker<Ruby>,
        _tracer_context: impl mmtk::vm::ObjectTracerContext<Ruby>,
    ) {
        panic!("We can't use MarkCompact in Ruby.");
    }
}

impl VMScanning {
    const OBJECT_BUFFER_SIZE: usize = 4096;

    fn collect_object_roots_in<F: FnMut()>(
        root_scan_kind: &str,
        gc_tls: &mut GCThreadTLS,
        factory: &mut impl RootsWorkFactory<RubySlot>,
        callback: F,
    ) {
        let mut buffer: Vec<ObjectReference> = Vec::new();
        let visit_object = |_, object: ObjectReference, pin| {
            debug!(
                "[{}] Visiting object: {}{}",
                root_scan_kind,
                object,
                if pin {
                    "(unmovable root)"
                } else {
                    "(movable, but we pin it anyway)"
                }
            );
            debug_assert!(
                mmtk::memory_manager::is_mmtk_object(object.to_raw_address()),
                "Root does not point to MMTk object.  object: {object}"
            );
            buffer.push(object);
            if buffer.len() >= Self::OBJECT_BUFFER_SIZE {
                factory.create_process_pinning_roots_work(std::mem::take(&mut buffer));
            }
            object
        };
        gc_tls
            .object_closure
            .set_temporarily_and_run_code(visit_object, callback);

        if !buffer.is_empty() {
            factory.create_process_pinning_roots_work(buffer);
        }
    }
}
