use crate::abi::GCThreadTLS;
use crate::address_buffer::FilledBuffer;
use crate::gc_work::ObjectsToObjectsWork;
use crate::{upcalls, Ruby, SINGLETON};
use mmtk::scheduler::{ProcessEdgesWork, WorkBucketStage};
use mmtk::util::{ObjectReference, VMWorkerThread};
use mmtk::vm::{Scanning, EdgeVisitor};
use mmtk::{memory_manager, Mutator, MutatorContext};


pub struct VMScanning {}

impl Scanning<Ruby> for VMScanning {
    const SINGLE_THREAD_MUTATOR_SCANNING: bool = false;

    fn scan_objects<EV: EdgeVisitor>(
        _tls: VMWorkerThread,
        _objects: &[ObjectReference],
        _edge_visitor: &mut EV,
    ) {
        panic!("This should not be called.  We scan objects by directly calling into Ruby");
    }

    fn scan_thread_roots<W: ProcessEdgesWork<VM = Ruby>>() {
        (upcalls().scan_thread_roots)()
    }

    fn scan_thread_root<W: ProcessEdgesWork<VM = Ruby>>(
        mutator: &'static mut Mutator<Ruby>,
        tls: VMWorkerThread,
    ) {
        let gc_tls = GCThreadTLS::from_vwt_check(tls);
        let callback = |_, filled_buffer: FilledBuffer| {
            debug!("[scan_thread_root] Buffer delivered.");
            let bucket = WorkBucketStage::Closure;
            let packet = ObjectsToObjectsWork::<W>::new(filled_buffer.as_objref_vec());
            memory_manager::add_work_packet(&SINGLETON, bucket, packet);
        };
        gc_tls.run_with_buffer_callback(callback, |_gc_tls| {
            (upcalls().scan_thread_root)(mutator.get_tls(), tls);
        });
    }

    fn scan_vm_specific_roots<W: ProcessEdgesWork<VM = Ruby>>() {
        let gc_tls = GCThreadTLS::from_upcall_check();
        let callback = |_, filled_buffer: FilledBuffer| {
            debug!("[scan_vm_specific_roots] Buffer delivered.");
            let bucket = WorkBucketStage::Closure;
            let packet = ObjectsToObjectsWork::<W>::new(filled_buffer.as_objref_vec());
            memory_manager::add_work_packet(&SINGLETON, bucket, packet);
        };
        gc_tls.run_with_buffer_callback(callback, |_gc_tls| {
            (upcalls().scan_vm_specific_roots)();
        });
        {
            // FIXME: This is a workaround.  Obviously it will keep all finalizable objects alive until program exits.
            debug!("[scan_vm_specific_roots] Enqueueing candidates.");
            let candidates = crate::binding()
                .finalizer_processor
                .with_candidates(|v| v.to_vec());
            let bucket = WorkBucketStage::Closure;
            let packet = ObjectsToObjectsWork::<W>::new(candidates);
            memory_manager::add_work_packet(&SINGLETON, bucket, packet);
        }
    }

    fn scan_object<EV: EdgeVisitor>(
        _tls: VMWorkerThread,
        _object: ObjectReference,
        _edge_visitor: &mut EV,
    ) {
        panic!("This should not be called.  We scan objects by directly calling into Ruby");
    }

    fn notify_initial_thread_scan_complete(_partial_scan: bool, _tls: VMWorkerThread) {
        // Do nothing
    }

    fn supports_return_barrier() -> bool {
        false
    }

    fn prepare_for_roots_re_scanning() {
        todo!()
    }
}
