use std::sync::Mutex;

use mmtk::{
    scheduler::{GCWork, GCWorker, WorkBucketStage},
    util::ObjectReference,
    vm::ObjectTracerContext,
};

use crate::{
    abi::{self, GCThreadTLS},
    extra_assert, is_mmtk_object_safe, upcalls,
    weak_proc::weak_global_tables::{
        UpdateCiTable, UpdateFinalizerAndObjIdTables, UpdateFrozenStringsTable,
        UpdateGenericFieldsTbl, UpdateGlobalSymbolsTable, UpdateOverloadedCmeTable,
    },
    Ruby,
};

pub mod concurrent_set_parallel;
pub mod st_table_parallel;
pub mod weak_global_tables;

/// Set this to true to use chunked processing optimization for the fstring table.
const SPECIALIZE_FSTRING_TABLE_PROCESSING: bool = true;

/// Set this to true to use chunked processing optimization for the global symbols table.
const SPECIALIZE_GLOBAL_SYMBOLS_TABLE_PROCESSING: bool = true;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum WeakConcurrentSetKind {
    FString = abi::MMTK_WEAK_CONCURRENT_SET_KIND_FSTRING,
    GlobalSymbols = abi::MMTK_WEAK_CONCURRENT_SET_KIND_GLOBAL_SYMBOLS,
}

pub struct WeakProcessor {
    /// Objects that needs `obj_free` called when dying.
    /// If it is a bottleneck, replace it with a lock-free data structure,
    /// or add candidates in batch.
    obj_free_candidates: Mutex<Vec<ObjectReference>>,
    /// Objects that contain weak fields.
    /// They are registered when such objects are allocated.
    objects_with_weak_fields: Mutex<Vec<ObjectReference>>,
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
            objects_with_weak_fields: Default::default(),
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

    pub fn declare_weak_references(&self, object: ObjectReference) {
        let mut objects_with_weak_fields = self.objects_with_weak_fields.lock().unwrap();
        objects_with_weak_fields.push(object);
        trace!("Pushed object with weak fields {object}");
    }

    pub fn get_all_objects_with_weak_fields(&self) -> Vec<ObjectReference> {
        let mut objects_with_weak_fields = self
            .objects_with_weak_fields
            .try_lock()
            .expect("Should not have contention.");
        std::mem::take(&mut objects_with_weak_fields)
    }

    pub fn re_add_objects_with_weak_fields(&self, objects: &[ObjectReference]) {
        let mut objects_with_weak_fields = self.objects_with_weak_fields.lock().unwrap();
        objects_with_weak_fields.extend_from_slice(objects);
    }

    pub fn process_weak_stuff(
        &self,
        worker: &mut GCWorker<Ruby>,
        _tracer_context: impl ObjectTracerContext<Ruby>,
    ) {
        worker.add_work(WorkBucketStage::VMRefClosure, ProcessObjFreeCandidates);

        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].bulk_add(vec![
            // BEGIN: Weak tables
            // Note: Follow the order of `rb_gc_vm_weak_table_foreach in `gc.c`
            Box::new(UpdateCiTable) as _,
            Box::new(UpdateOverloadedCmeTable) as _,
            // global symbols table specialized
            Box::new(UpdateFinalizerAndObjIdTables) as _,
            Box::new(UpdateGenericFieldsTbl) as _,
            // END: Weak tables
            Box::new(UpdateWbUnprotectedObjectsList) as _,
            Box::new(ProcessWeakReferences) as _,
        ]);

        if SPECIALIZE_FSTRING_TABLE_PROCESSING {
            concurrent_set_parallel::process_weak_concurrent_set_chunked(
                "fstring",
                (upcalls().get_fstring_table_obj)().into(),
                WeakConcurrentSetKind::FString,
                worker,
            );
        } else {
            worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure]
                .add_boxed(Box::new(UpdateFrozenStringsTable) as _);
        }

        if SPECIALIZE_GLOBAL_SYMBOLS_TABLE_PROCESSING {
            concurrent_set_parallel::process_weak_concurrent_set_chunked(
                "global symbols",
                (upcalls().get_global_symbols_table_obj)().into(),
                WeakConcurrentSetKind::GlobalSymbols,
                worker,
            );
        } else {
            worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure]
                .add_boxed(Box::new(UpdateGlobalSymbolsTable) as _);
        }
    }
}

struct ProcessObjFreeCandidates;

impl GCWork<Ruby> for ProcessObjFreeCandidates {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        // If it blocks, it is a bug.
        let mut obj_free_candidates = crate::binding()
            .weak_proc
            .obj_free_candidates
            .try_lock()
            .expect("It's GC time.  No mutators should hold this lock at this time.");

        let old_cands = obj_free_candidates.len();
        debug!("Total: {} candidates", old_cands);

        let mut freed = 0usize;
        let mut elided = 0usize;

        // Process obj_free
        let mut new_candidates = Vec::new();

        for object in obj_free_candidates.iter().copied() {
            if object.is_reachable() {
                // Forward and add back to the candidate list.
                let new_object = object.forward();
                trace!(
                    "Forwarding obj_free candidate: {} -> {}",
                    object,
                    new_object
                );
                new_candidates.push(new_object);
            } else if (upcalls().obj_needs_cleanup_p)(object) {
                (upcalls().call_obj_free)(object);
                freed += 1;
            } else {
                elided += 1;
            }
        }

        let new_cands = new_candidates.len();
        *obj_free_candidates = new_candidates;
        probe!(
            mmtk_ruby,
            process_obj_free_candidates,
            old_cands,
            new_cands,
            freed,
            elided,
        );
    }
}

struct UpdateWbUnprotectedObjectsList;

impl GCWork<Ruby> for UpdateWbUnprotectedObjectsList {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let mut objects = crate::binding().wb_unprotected_objects.try_lock().expect(
            "Someone is holding the lock of wb_unprotected_objects during weak processing phase?",
        );

        let old_objects = std::mem::take(&mut *objects);
        let old_size = old_objects.len();

        debug!("Updating {old_size} WB-unprotected objects");

        for object in old_objects {
            if object.is_reachable() {
                // Forward and add back to the candidate list.
                let new_object = object.forward();
                trace!(
                    "Forwarding WB-unprotected object: {} -> {}",
                    object,
                    new_object
                );
                objects.insert(new_object);
            } else {
                trace!("Removing WB-unprotected object from list: {}", object);
            }
        }

        let new_size = objects.len();
        debug!("Retained {new_size} live WB-unprotected objects.");

        probe!(
            mmtk_ruby,
            update_wb_unprotected_objects_list,
            old_size,
            new_size
        );
    }
}

// Provide a shorthand `object.forward()`.
trait Forwardable {
    fn forward(&self) -> Self;
}

impl Forwardable for ObjectReference {
    fn forward(&self) -> Self {
        self.get_forwarded_object().unwrap_or(*self)
    }
}

struct ProcessWeakReferences;

impl GCWork<Ruby> for ProcessWeakReferences {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let is_moving_gc = crate::mmtk().get_plan().current_gc_may_move_object();

        let objects_with_weak_fields = crate::binding()
            .weak_proc
            .get_all_objects_with_weak_fields();

        let num_objects = objects_with_weak_fields.len();
        let mut live = 0usize;

        debug!("Processing {num_objects} objects with weak fields...");

        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };

        let mut live_objects = vec![];

        for old_object in objects_with_weak_fields {
            trace!("  Object with weak fields: {old_object}");
            if old_object.is_reachable() {
                trace!("    Object {old_object} is live");
                live += 1;

                // Forward old_object if it is moved.
                let object = if let Some(new_object) = old_object.get_forwarded_object() {
                    trace!("    Object is moved: {old_object} -> {new_object}");
                    new_object
                } else {
                    old_object
                };

                // We bind the rb_gc_location method to `ObjectReference::get_forwarded_object`
                // because we are forwarding references in objects that have weak references.  We
                // don't trace them.
                let visit_object = |_worker, target_object: ObjectReference, pin: bool| {
                    trace!(
                        "Forwarding edge: {} -> {}{}",
                        object,
                        target_object,
                        if pin { " pin" } else { "" }
                    );
                    extra_assert!(!pin, "Should not pin when forwarding reference.");
                    extra_assert!(
                        is_mmtk_object_safe(target_object.to_raw_address()),
                        "Destination is not an MMTk object. Src: {object} dst: {target_object}"
                    );
                    if let Some(forwarded_target) = target_object.get_forwarded_object() {
                        trace!(
                            "  Forwarded target {} -> {}",
                            target_object,
                            forwarded_target
                        );
                        forwarded_target
                    } else {
                        target_object
                    }
                };
                gc_tls
                    .object_closure
                    .set_temporarily_and_run_code(visit_object, || {
                        (upcalls().handle_weak_references)(object, is_moving_gc);
                    });

                live_objects.push(object);
            }
        }

        crate::binding()
            .weak_proc
            .re_add_objects_with_weak_fields(&live_objects);

        debug!("Processed {num_objects} objects with weak fields.  {live} live.");
        probe!(mmtk_ruby, process_weak_references, num_objects, live);
    }
}
