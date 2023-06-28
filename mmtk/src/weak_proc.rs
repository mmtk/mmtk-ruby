use std::sync::Mutex;

use mmtk::{
    scheduler::GCWorker,
    util::ObjectReference,
    vm::{ObjectModel, ObjectTracer, ObjectTracerContext},
};

use crate::{
    abi::{GCThreadTLS, RubyObjectAccess},
    binding::MovedGIVTblEntry,
    object_model::VMObjectModel,
    upcalls, Ruby,
};

pub struct WeakProcessor {
    /// Objects that needs `obj_free` called when dying.
    /// If it is a bottleneck, replace it with a lock-free data structure,
    /// or add candidates in batch.
    obj_free_candidates: Mutex<Vec<ObjectReference>>,
}

impl Default for WeakProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl WeakProcessor {
    pub fn new() -> Self {
        Self {
            obj_free_candidates: Mutex::new(Vec::new()),
        }
    }

    /// Add an object as a candidate for `obj_free`.
    ///
    /// Multiple mutators can call it concurrently, so it has `&self`.
    pub fn add_obj_free_candidate(&self, object: ObjectReference) {
        let mut obj_free_candidates = self.obj_free_candidates.lock().unwrap();
        obj_free_candidates.push(object);
    }

    /// Add many objects as candidates for `obj_free`.
    ///
    /// Multiple mutators can call it concurrently, so it has `&self`.
    pub fn add_obj_free_candidates(&self, objects: &[ObjectReference]) {
        let mut obj_free_candidates = self.obj_free_candidates.lock().unwrap();
        for object in objects.iter().copied() {
            obj_free_candidates.push(object);
        }
    }

    pub fn get_all_obj_free_candidates(&self) -> Vec<ObjectReference> {
        let mut obj_free_candidates = self.obj_free_candidates.lock().unwrap();
        std::mem::take(obj_free_candidates.as_mut())
    }

    pub fn process_weak_stuff(
        &self,
        worker: &mut GCWorker<Ruby>,
        tracer_context: impl ObjectTracerContext<Ruby>,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };

        // If it blocks, it is a bug.
        let mut obj_free_candidates = self
            .obj_free_candidates
            .try_lock()
            .expect("It's GC time.  No mutators should hold this lock at this time.");

        // Enable tracer in this scope.
        tracer_context.with_tracer(worker, |tracer| {
            // Forward some global weak tables that needs to be handled before obj_free.
            let forward_object = |_worker, object: ObjectReference, _pin| {
                debug_assert!(mmtk::memory_manager::is_mmtk_object(
                    VMObjectModel::ref_to_address(object)
                ));
                let result = tracer.trace_object(object);
                trace!("Forwarding reference: {} -> {}", object, result);
                result
            };

            gc_tls
                .object_closure
                .set_temporarily_and_run_code(forward_object, || {
                    log::debug!("Updating early global weak tables...");
                    (upcalls().update_global_weak_tables_early)();
                    log::debug!("Finished updating early global weak tables.");
                });

            // Process obj_free
            let mut new_candidates = Vec::new();

            for object in obj_free_candidates.iter().copied() {
                if object.is_reachable() {
                    // Forward and add back to the candidate list.
                    let new_object = tracer.trace_object(object);
                    trace!(
                        "Forwarding obj_free candidate: {} -> {}",
                        object,
                        new_object
                    );
                    new_candidates.push(new_object);
                } else {
                    (upcalls().call_obj_free)(object);
                }
            }

            *obj_free_candidates = new_candidates;

            // Forward other global weak tables
            let forward_object = |_worker, object: ObjectReference, _pin| {
                debug_assert!(mmtk::memory_manager::is_mmtk_object(
                    VMObjectModel::ref_to_address(object)
                ));
                let result = tracer.trace_object(object);
                trace!("Forwarding reference: {} -> {}", object, result);
                result
            };

            log::debug!("Updating global ivtbl entries...");
            {
                let mut moved_givtbl = crate::binding()
                    .moved_givtbl
                    .try_lock()
                    .expect("Should have no race in weak_proc");
                for (new_objref, MovedGIVTblEntry { old_objref, .. }) in moved_givtbl.drain() {
                    trace!("  givtbl {} -> {}", old_objref, new_objref);
                    RubyObjectAccess::from_objref(new_objref).clear_has_moved_givtbl();
                    (upcalls().move_givtbl)(old_objref, new_objref);
                }
            }
            log::debug!("Updated global ivtbl entries.");

            gc_tls
                .object_closure
                .set_temporarily_and_run_code(forward_object, || {
                    log::debug!("Updating global weak tables...");
                    (upcalls().update_global_weak_tables)();
                    log::debug!("Finished updating global weak tables.");
                });
        });
    }
}
