use std::sync::atomic::AtomicBool;

use crate::abi::GCThreadTLS;

use crate::cruby_support::cruby::{
    rb_shape_obj_too_complex, RUBY_FL_EXIVAR, RUBY_T_ARRAY, RUBY_T_BIGNUM, RUBY_T_FLOAT,
    RUBY_T_IMEMO, RUBY_T_OBJECT, RUBY_T_STRING, RUBY_T_SYMBOL, SIZEOF_VALUE, VALUE,
};
use crate::cruby_support::cruby_extra::{
    my_special_const_p, rarray_embed_ary_addr, rarray_embed_len, robject_embed_ary_addr,
};
use crate::cruby_support::flag_tests;
use crate::{upcalls, Ruby, RubyEdge};
use mmtk::scheduler::GCWorker;
use mmtk::util::{Address, ObjectReference, VMWorkerThread};
use mmtk::vm::{EdgeVisitor, ObjectTracer, RootsWorkFactory, Scanning};
use mmtk::{Mutator, MutatorContext};

pub struct VMScanning {}

impl Scanning<Ruby> for VMScanning {
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
        debug_assert!(
            mmtk::memory_manager::is_mmtk_object(object.to_raw_address()),
            "Not an MMTk object: {object}",
        );

        let allow_fast_paths = if cfg!(feature = "env_var_fast_path_switch") {
            USE_FAST_PATHS.load(std::sync::atomic::Ordering::Relaxed)
        } else {
            true
        };

        if allow_fast_paths {
            if Self::scan_object_and_trace_edges_fast(object, object_tracer) {
                return;
            }
        }

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
        mut factory: impl RootsWorkFactory<RubyEdge>,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        Self::collect_object_roots_in("scan_thread_root", gc_tls, &mut factory, || {
            (upcalls().scan_roots_in_mutator_thread)(mutator.get_tls(), tls);
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

pub(crate) static USE_FAST_PATHS: AtomicBool = AtomicBool::new(false);

impl VMScanning {
    const OBJECT_BUFFER_SIZE: usize = 4096;

    fn collect_object_roots_in<F: FnMut()>(
        root_scan_kind: &str,
        gc_tls: &mut GCThreadTLS,
        factory: &mut impl RootsWorkFactory<RubyEdge>,
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

    /// Scan `object` in Rust.  This function shall handle the most common cases in Rust, but does
    /// not have to handle all types or all cases (not embedded, shared, etc.).
    ///
    /// Return `true` if the object has been scanned.
    /// Return `false` to fall back to the slow path in C.
    fn scan_object_and_trace_edges_fast<OT: ObjectTracer>(
        object: ObjectReference,
        object_tracer: &mut OT,
    ) -> bool {
        let ruby_value = VALUE::from(object);
        let ruby_flags = ruby_value.builtin_flags();
        let ruby_type = ruby_value.builtin_type();

        if flag_tests::robject_has_exivar(ruby_flags) {
            // Handle objects with generic ivars in C.
            return false;
        }

        match ruby_type {
            RUBY_T_FLOAT | RUBY_T_BIGNUM | RUBY_T_SYMBOL => {
                // Those objects have no children.
                return true;
            }

            RUBY_T_IMEMO => {
                // TODO: Some IMemos really should be handled in Rust.
                return false;
            }

            _ => {
                let handled = Self::scan_and_trace_common(
                    object,
                    ruby_value,
                    ruby_flags,
                    ruby_type,
                    object_tracer,
                );
                if handled {
                    let ptr_basic = ruby_value.as_basic();
                    let klass = unsafe { (*ptr_basic).klass };
                    let new_klass = VALUE::from(object_tracer.trace_object(klass.into()));
                    if new_klass != klass {
                        unsafe { (*ptr_basic).klass = new_klass }
                    }
                }
                handled
            }
        }
    }

    fn scan_and_trace_common<OT: ObjectTracer>(
        _object: ObjectReference,
        ruby_value: VALUE,
        ruby_flags: usize,
        ruby_type: u32,
        object_tracer: &mut OT,
    ) -> bool {
        match ruby_type {
            RUBY_T_OBJECT => {
                if unsafe { rb_shape_obj_too_complex(ruby_value) } {
                    // Too complex.  Fall back to C.
                    return false;
                }

                if flag_tests::robject_is_embedded(ruby_flags) {
                    return false;
                    // // Scan the embedded parts of the object.
                    // let payload_addr = robject_embed_ary_addr(ruby_value);
                    // let
                    // Self::scan_and_trace_array_slice(tls, object, payload_addr, len, object_tracer);
                    return true;
                }

                // Off-load other cases to C.
                return false;
            }
            RUBY_T_STRING => {
                // Match the semantics of `gc_ref_update_string` in C.

                if flag_tests::rstring_is_embedded(ruby_flags) {
                    // Embedded strings don't have children.
                    return true;
                }
                if flag_tests::rstring_no_free(ruby_flags) {
                    // If the string has "no free" flag, skip it.
                    return true;
                }

                // Off-load other cases to C.
                return false;
            }
            RUBY_T_ARRAY => {
                // Match the semantics of `gc_ref_update_array` in C.

                if flag_tests::rarray_is_embedded(ruby_flags) {
                    // Scan the embedded parts of the array.
                    let payload_addr = rarray_embed_ary_addr(ruby_value);
                    let len = rarray_embed_len(ruby_flags);
                    Self::scan_and_trace_array_slice(payload_addr, len, object_tracer);
                    return true;
                }

                // Off-load other cases to C.
                return false;
            }
            _ => {
                // For all other types, fall back to C.
                return false;
            }
        };
    }

    fn scan_and_trace_array_slice<OT: ObjectTracer>(
        array_begin: Address,
        array_len: usize,
        object_tracer: &mut OT,
    ) {
        for index in 0..array_len {
            let elem_addr = array_begin.add(index * SIZEOF_VALUE);
            let elem = unsafe { elem_addr.load::<usize>() };
            let ruby_value = VALUE(elem);
            if !my_special_const_p(ruby_value) {
                let objref = ObjectReference::from(ruby_value);
                let new_objref = object_tracer.trace_object(objref);
                if new_objref != objref {
                    unsafe { elem_addr.store(new_objref) }
                }
            }
        }
    }
}
