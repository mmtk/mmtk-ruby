use crate::{upcalls, Ruby};
use mmtk::scheduler::{GCController, GCWorker};
use mmtk::util::{ObjectReference, VMMutatorThread, VMWorkerThread};
use mmtk::Mutator;

pub const GC_THREAD_KIND_CONTROLLER: libc::c_int = 0;
pub const GC_THREAD_KIND_WORKER: libc::c_int = 1;

#[repr(C)]
pub struct ObjectClosure {
    /// The function to be called from C.  Must match the signature of `ObjectClosure::c_function`
    pub c_function:
        *const fn(*mut libc::c_void, *mut libc::c_void, ObjectReference) -> ObjectReference,
    /// The pointer to the Rust-level closure object.
    pub rust_closure: *mut libc::c_void,
}

impl Default for ObjectClosure {
    fn default() -> Self {
        Self {
            c_function: Self::c_function_unregistered as _,
            rust_closure: std::ptr::null_mut(),
        }
    }
}

impl ObjectClosure {
    /// Set this ObjectClosure temporarily to `visit_object`, and execute `f`.  During the execution of
    /// `f`, the Ruby VM may call this ObjectClosure.  When the Ruby VM calls this ObjectClosure,
    /// it effectively calls `visit_object`.
    ///
    /// This method is intended to run Ruby VM code in `f` with temporarily modified behavior of
    /// `rb_gc_mark`, `rb_gc_mark_movable` and `rb_gc_location`
    ///
    /// Both `f` and `visit_object` may access and modify local variables in the environment where
    /// `set_temporarily_and_run_code` called.
    ///
    /// Note that this function is not reentrant.  Don't call this function in either `callback` or
    /// `f`.
    pub fn set_temporarily_and_run_code<'env, T, F1, F2>(
        &mut self,
        mut visit_object: F1,
        f: F2,
    ) -> T
    where
        F1: 'env + FnMut(&'static mut GCWorker<Ruby>, ObjectReference) -> ObjectReference,
        F2: 'env + FnOnce() -> T,
    {
        debug_assert!(
            self.c_function == Self::c_function_unregistered as *const _,
            "set_temporarily_and_run_code is recursively called."
        );
        self.c_function = Self::c_function_registered::<F1> as *const _;
        self.rust_closure = &mut visit_object as *mut F1 as *mut libc::c_void;
        let result = f();
        *self = Default::default();
        result
    }

    extern "C" fn c_function_registered<F>(
        rust_closure: *mut libc::c_void,
        worker: *mut libc::c_void,
        object: ObjectReference,
    ) -> ObjectReference
    where
        F: FnMut(&'static mut GCWorker<Ruby>, ObjectReference) -> ObjectReference,
    {
        let rust_closure = unsafe { &mut *(rust_closure as *mut F) };
        let worker = unsafe { &mut *(worker as *mut GCWorker<Ruby>) };
        rust_closure(worker, object)
    }

    extern "C" fn c_function_unregistered(
        _rust_closure: *mut libc::c_void,
        worker: *mut libc::c_void,
        object: ObjectReference,
    ) -> ObjectReference {
        let worker = unsafe { &mut *(worker as *mut GCWorker<Ruby>) };
        panic!(
            "object_closure is not set.  worker ordinal: {}, object: {}",
            worker.ordinal, object
        );
    }
}

#[repr(C)]
pub struct GCThreadTLS {
    pub kind: libc::c_int,
    pub gc_context: *mut libc::c_void,
    pub object_closure: ObjectClosure,
}

impl GCThreadTLS {
    fn new(kind: libc::c_int, gc_context: *mut libc::c_void) -> Self {
        Self {
            kind,
            gc_context,
            object_closure: Default::default(),
        }
    }

    pub fn for_controller(gc_context: *mut GCController<Ruby>) -> Self {
        Self::new(GC_THREAD_KIND_CONTROLLER, gc_context as *mut libc::c_void)
    }

    pub fn for_worker(gc_context: *mut GCWorker<Ruby>) -> Self {
        Self::new(GC_THREAD_KIND_WORKER, gc_context as *mut libc::c_void)
    }

    pub fn from_vwt(vwt: VMWorkerThread) -> *mut GCThreadTLS {
        unsafe { std::mem::transmute(vwt) }
    }

    pub fn check_cast(ptr: *mut GCThreadTLS) -> &'static mut GCThreadTLS {
        assert!(ptr != std::ptr::null_mut());
        let result = unsafe { &mut *ptr };
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
        unsafe { std::mem::transmute(ptr) }
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
}

#[repr(C)]
#[derive(Clone)]
pub struct RawVecOfObjRef {
    pub ptr: *mut ObjectReference,
    pub len: usize,
    pub capa: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct RubyUpcalls {
    pub init_gc_worker_thread: extern "C" fn(gc_worker_tls: *mut GCThreadTLS),
    pub get_gc_thread_tls: extern "C" fn() -> *mut GCThreadTLS,
    pub stop_the_world: extern "C" fn(tls: VMWorkerThread),
    pub resume_mutators: extern "C" fn(tls: VMWorkerThread),
    pub block_for_gc: extern "C" fn(tls: VMMutatorThread),
    pub number_of_mutators: extern "C" fn() -> usize,
    pub reset_mutator_iterator: extern "C" fn(),
    pub get_next_mutator: extern "C" fn() -> *mut Mutator<Ruby>,
    pub scan_vm_specific_roots: extern "C" fn(),
    pub scan_thread_roots: extern "C" fn(),
    pub scan_thread_root: extern "C" fn(mutator_tls: VMMutatorThread, worker_tls: VMWorkerThread),
    pub scan_object_ruby_style: extern "C" fn(object: ObjectReference),
    pub obj_free: extern "C" fn(object: ObjectReference),
}

unsafe impl Sync for RubyUpcalls {}
