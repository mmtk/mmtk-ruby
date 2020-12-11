use mmtk::vm::Scanning;
use mmtk::{TransitiveClosure, SelectedPlan, Mutator};
use mmtk::util::{ObjectReference, SynchronizedCounter};
use mmtk::util::OpaquePointer;
use mmtk::scheduler::gc_works::*;
use crate::Ruby;

static COUNTER: SynchronizedCounter = SynchronizedCounter::new(0);

pub struct VMScanning {}

impl Scanning<Ruby> for VMScanning {
    fn scan_objects<W: ProcessEdgesWork<VM=Ruby>>(_objects: &[ObjectReference]) {
        unimplemented!()
    }
    fn scan_thread_roots<W: ProcessEdgesWork<VM=Ruby>>() {
        unimplemented!()
    }
    fn scan_thread_root<W: ProcessEdgesWork<VM=Ruby>>(_mutator: &'static mut Mutator<SelectedPlan<Ruby>>, _tls: OpaquePointer) {
        unimplemented!()
    }
    fn scan_vm_specific_roots<W: ProcessEdgesWork<VM=Ruby>>() {
        unimplemented!()
    }
    fn scan_object<T: TransitiveClosure>(_trace: &mut T, _object: ObjectReference, _tls: OpaquePointer) {
        unimplemented!()
    }

    fn reset_thread_counter() {
        COUNTER.reset();
    }

    fn notify_initial_thread_scan_complete(_partial_scan: bool, _tls: OpaquePointer) {
        unimplemented!()
    }

    fn supports_return_barrier() -> bool {
        unimplemented!()
    }
}