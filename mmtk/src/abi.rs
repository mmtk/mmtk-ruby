use mmtk::util::{VMMutatorThread, VMWorkerThread};
use mmtk::Mutator;
use crate::Ruby;
use crate::address_buffer::AddressBuffer;

pub const GC_THREAD_KIND_CONTROLLER: libc::c_int = 0;
pub const GC_THREAD_KIND_WORKER: libc::c_int = 1;

#[repr(C)]
#[derive(Clone)]
pub struct GCThreadTLS {
    pub kind: libc::c_int,
    pub gc_context: *mut libc::c_void,
    pub mark_buffer: AddressBuffer,
}

#[repr(C)]
#[derive(Clone)]
pub struct RubyUpcalls {
    pub init_gc_worker_thread: extern "C" fn (gc_worker_tls: *mut GCThreadTLS),
    pub stop_the_world: extern "C" fn (tls: VMWorkerThread),
    pub resume_mutators: extern "C" fn (tls: VMWorkerThread),
    pub block_for_gc: extern "C" fn (tls: VMMutatorThread),
    pub number_of_mutators: extern "C" fn () -> usize,
    pub reset_mutator_iterator: extern "C" fn (),
    pub get_next_mutator: extern "C" fn () -> *mut Mutator<Ruby>,
    pub scan_vm_specific_roots: extern "C" fn (),
    pub scan_thread_roots: extern "C" fn (),
    pub scan_thread_root: extern "C" fn (mutator_tls: VMMutatorThread, worker_tls: VMWorkerThread),
}

unsafe impl Sync for RubyUpcalls {}
