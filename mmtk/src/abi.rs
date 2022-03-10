use mmtk::scheduler::{GCController, GCWorker};
use mmtk::util::{Address, VMMutatorThread, VMWorkerThread, VMThread, OpaquePointer, ObjectReference};
use mmtk::Mutator;
use crate::{Ruby, upcalls};
use crate::address_buffer::{AddressBuffer, FilledBuffer};

pub const GC_THREAD_KIND_CONTROLLER: libc::c_int = 0;
pub const GC_THREAD_KIND_WORKER: libc::c_int = 1;

pub type BufferCallback = Box<dyn FnMut(&'static mut GCWorker<Ruby>, FilledBuffer)>;

#[repr(C)]
pub struct GCThreadTLS {
    pub kind: libc::c_int,
    pub gc_context: *mut libc::c_void,
    pub mark_buffer: AddressBuffer,
    // The following are only accessible from Rust
    pub buffer_callback: Option<BufferCallback>
}

impl GCThreadTLS {
    fn new(kind: libc::c_int, gc_context: *mut libc::c_void) -> Self {
        Self {
            kind,
            gc_context,
            mark_buffer: AddressBuffer::create(),
            buffer_callback: None,
        }
    }

    pub fn for_controller(gc_context: *mut GCController<Ruby>) -> Self {
        Self::new(GC_THREAD_KIND_CONTROLLER, gc_context as *mut libc::c_void)
    }

    pub fn for_worker(gc_context: *mut GCWorker<Ruby>) -> Self {
        Self::new(GC_THREAD_KIND_WORKER, gc_context as *mut libc::c_void)
    }

    pub fn from_vwt(vwt: VMWorkerThread) -> *mut GCThreadTLS {
        unsafe {
            std::mem::transmute(vwt)
        }
    }

    pub fn check_cast(ptr: *mut GCThreadTLS) -> &'static mut GCThreadTLS {
        assert!(ptr != std::ptr::null_mut());
        let result = unsafe {
            &mut *ptr
        };
        debug_assert!({
            let kind = result.kind;
            kind == GC_THREAD_KIND_CONTROLLER || kind == GC_THREAD_KIND_WORKER
        });
        result
    }

    pub fn from_vwt_check(vwt: VMWorkerThread) -> &'static mut GCThreadTLS {
        let ptr = Self::from_vwt(vwt);
        Self::check_cast(ptr)
    }

    pub fn to_vwt(ptr: *mut Self) -> VMWorkerThread {
        unsafe {
            std::mem::transmute(ptr)
        }
    }

    pub fn from_upcall_check() -> &'static mut GCThreadTLS {
        let ptr = (upcalls().get_gc_thread_tls)();
        Self::check_cast(ptr)
    }

    pub fn worker<'s, 'w>(&'s mut self) -> &'w mut GCWorker<Ruby> {
        // NOTE: The returned ref points to the worker which does not have the same lifetime as self.
        assert!(self.kind == GC_THREAD_KIND_WORKER);
        unsafe { &mut *(self.gc_context as *mut GCWorker<Ruby>) }
    }

    /// Executes `f`. During the execution of `f`, if the Ruby VM delivers a mark buffer,
    /// `callback` will be called with the filled buffer as the argument.
    ///
    /// Both `f` and `callback` may access and modify local variables in the environment where
    /// `run_with_buffer_callback` called.
    pub fn run_with_buffer_callback<'env, T, F1, F2>(&mut self, callback: F1, f: F2) -> T
            where F1: 'env + FnMut(&'static mut GCWorker<Ruby>, FilledBuffer),
                  F2: 'env + FnOnce(&mut Self) -> T {
        let boxed_callback: Box<dyn 'env + FnMut(&'static mut GCWorker<Ruby>, FilledBuffer)> = Box::new(callback);
        let boxed_callback: Box<dyn 'static + FnMut(&'static mut GCWorker<Ruby>, FilledBuffer)> =
            unsafe { std::mem::transmute(boxed_callback) };

        self.buffer_callback = Some(boxed_callback);
        let result = f(self);
        self.flush_buffer();
        self.buffer_callback = None;

        result
    }

    pub fn flush_buffer(&mut self) {
        if self.mark_buffer.is_empty() {
            return;
        }

        let gc_worker = self.worker();

        let maybe_callback = &mut self.buffer_callback;
        let callback = maybe_callback.as_deref_mut().unwrap_or_else(|| {
            panic!("buffer callback not set.  Current thread: {:?}",
                std::thread::current().name());
        });

        let filled_buffer = self.mark_buffer.take_as_filled_buffer();
        callback(gc_worker, filled_buffer);
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct RubyUpcalls {
    pub init_gc_worker_thread: extern "C" fn (gc_worker_tls: *mut GCThreadTLS),
    pub get_gc_thread_tls: extern "C" fn () -> *mut GCThreadTLS,
    pub stop_the_world: extern "C" fn (tls: VMWorkerThread),
    pub resume_mutators: extern "C" fn (tls: VMWorkerThread),
    pub block_for_gc: extern "C" fn (tls: VMMutatorThread),
    pub number_of_mutators: extern "C" fn () -> usize,
    pub reset_mutator_iterator: extern "C" fn (),
    pub get_next_mutator: extern "C" fn () -> *mut Mutator<Ruby>,
    pub scan_vm_specific_roots: extern "C" fn (),
    pub scan_thread_roots: extern "C" fn (),
    pub scan_thread_root: extern "C" fn (mutator_tls: VMMutatorThread, worker_tls: VMWorkerThread),
    pub scan_object_ruby_style: extern "C" fn (object: ObjectReference),
}

unsafe impl Sync for RubyUpcalls {}
