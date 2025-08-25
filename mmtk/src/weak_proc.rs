use std::sync::Mutex;

use mmtk::{
    scheduler::{GCWork, GCWorker, WorkBucketStage},
    util::{Address, ObjectReference},
    vm::ObjectTracerContext,
};

use crate::{
    abi::{self, GCThreadTLS, Qundef, VALUE},
    extra_assert, is_mmtk_object_safe, upcalls, Ruby,
};

pub mod concurrent_set_parallel;
pub mod st_table_parallel;

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

pub type FieldType = *mut VALUE;

pub struct WeakProcessor {
    /// Objects that needs `obj_free` called when dying.
    /// If it is a bottleneck, replace it with a lock-free data structure,
    /// or add candidates in batch.
    obj_free_candidates: Mutex<Vec<ObjectReference>>,
    /// Weak fields discovered during the current GC.
    weak_fields: Mutex<Vec<FieldType>>,
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
            weak_fields: Default::default(),
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

    pub fn clear_weak_fields(&self) {
        let mut weak_fields = self
            .weak_fields
            .try_lock()
            .expect("Should not have contention.");
        weak_fields.clear();
    }

    pub fn discover_weak_field(&self, field: FieldType) {
        let mut weak_fields = self.weak_fields.lock().unwrap();
        weak_fields.push(field);
        trace!("Pushed weak field {field:?}");
    }

    pub fn get_all_weak_fields(&self) -> Vec<FieldType> {
        let mut weak_fields = self
            .weak_fields
            .try_lock()
            .expect("Should not have contention.");
        std::mem::take(&mut weak_fields)
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
            Box::new(UpdateCCRefinementTable) as _,
            // END: Weak tables
            Box::new(UpdateWbUnprotectedObjectsList) as _,
            Box::new(UpdateWeakFields) as _,
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
            } else {
                (upcalls().call_obj_free)(object);
            }
        }

        let new_cands = new_candidates.len();
        *obj_free_candidates = new_candidates;
        probe!(mmtk_ruby, process_obj_free_candidates, old_cands, new_cands);
    }
}

trait GlobalTableProcessingWork {
    fn process_table(&mut self);

    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };

        // `hash_foreach_replace` depends on `gb_object_moved_p` which has to have the semantics
        // of `trace_object` due to the way it is used in `UPDATE_IF_MOVED`.
        let forward_object = |_worker, object: ObjectReference, _pin| {
            extra_assert!(
                is_mmtk_object_safe(object.to_raw_address()),
                "{} is not an MMTk object",
                object
            );
            let result = object.forward();
            trace!("Forwarding reference: {} -> {}", object, result);
            result
        };

        gc_tls
            .object_closure
            .set_temporarily_and_run_code(forward_object, || {
                self.process_table();
            });
    }
}

macro_rules! define_global_table_processor {
    ($name: ident, $code: expr) => {
        struct $name;
        impl GlobalTableProcessingWork for $name {
            fn process_table(&mut self) {
                $code
            }
        }
        impl GCWork<Ruby> for $name {
            fn do_work(&mut self, worker: &mut GCWorker<Ruby>, mmtk: &'static mmtk::MMTK<Ruby>) {
                GlobalTableProcessingWork::do_work(self, worker, mmtk);
            }
        }
    };
}

fn general_update_weak_table(size_getter: extern "C" fn() -> usize, cleaner: extern "C" fn()) {
    let old_size = size_getter();
    cleaner();
    let new_size = size_getter();
    probe!(mmtk_ruby, weak_table_size_change, old_size, new_size);
}

///////// BEGIN: Simple table updating work packets ////////
// Note: Follow the order of `rb_gc_vm_weak_table_foreach in `gc.c`

define_global_table_processor!(UpdateCiTable, {
    general_update_weak_table(upcalls().get_ci_table_size, upcalls().update_ci_table);
});

define_global_table_processor!(UpdateOverloadedCmeTable, {
    general_update_weak_table(
        upcalls().get_overloaded_cme_table_size,
        upcalls().update_overloaded_cme_table,
    );
});

define_global_table_processor!(UpdateGlobalSymbolsTable, {
    general_update_weak_table(
        upcalls().get_global_symbols_table_size,
        upcalls().update_global_symbols_table,
    );
});

define_global_table_processor!(UpdateFinalizerAndObjIdTables, {
    let old_size_finalizer = (upcalls().get_finalizer_table_size)();
    let old_size_id_to_obj = (upcalls().get_id2ref_table_size)();

    (upcalls().update_finalizer_and_obj_id_tables)();

    let new_size_finalizer = (upcalls().get_finalizer_table_size)();
    let new_size_id_to_obj = (upcalls().get_id2ref_table_size)();

    probe!(
        mmtk_ruby,
        update_finalizer_and_obj_id_tables,
        old_size_finalizer,
        new_size_finalizer,
        old_size_id_to_obj,
        new_size_id_to_obj,
    );
});

define_global_table_processor!(UpdateGenericFieldsTbl, {
    general_update_weak_table(
        upcalls().get_generic_fields_tbl_size,
        upcalls().update_generic_fields_table,
    );
});

define_global_table_processor!(UpdateFrozenStringsTable, {
    general_update_weak_table(
        upcalls().get_frozen_strings_table_size,
        upcalls().update_frozen_strings_table,
    );
});

define_global_table_processor!(UpdateCCRefinementTable, {
    general_update_weak_table(
        upcalls().get_cc_refinement_table_size,
        upcalls().update_cc_refinement_table,
    );
});

///////// END: Simple table updating work packets ////////

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

struct UpdateWeakFields;

impl GCWork<Ruby> for UpdateWeakFields {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        let weak_fields = crate::binding().weak_proc.get_all_weak_fields();

        let num_fields = weak_fields.len();
        let mut live = 0usize;
        let mut forwarded = 0usize;

        debug!("Updating {num_fields} weak fields...");

        for field in weak_fields {
            let old_value = unsafe { *field };
            trace!("  Visiting weak field {field:?} -> {old_value:?}");

            if old_value.is_special_const() {
                continue;
            }

            let addr = unsafe { Address::from_usize(old_value.0) };
            if !addr.is_mapped() {
                panic!("Field {field:?} value {addr} points to unmapped area");
            }
            let Some(old_objref) = mmtk::memory_manager::is_mmtk_object(addr) else {
                panic!("Field {field:?} value {addr} is an invalid object reference");
            };

            if old_objref.is_reachable() {
                live += 1;
                if let Some(new_objref) = old_objref.get_forwarded_object() {
                    forwarded += 1;
                    let new_value = VALUE::from(new_objref);
                    trace!("    Updated weak field {field:?} to {new_value:?}");
                    unsafe { *field = new_value };
                }
            } else {
                unsafe { *field = Qundef };
            }
        }

        debug!("Updated {num_fields} weak fields.  {live} live, {forwarded} forwarded.");
        probe!(mmtk_ruby, update_weak_fields, num_fields, live, forwarded);
    }
}
