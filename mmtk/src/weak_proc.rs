use std::sync::{Arc, Mutex};

use mmtk::{
    scheduler::{GCWork, GCWorker, WorkBucketStage},
    util::ObjectReference,
    vm::ObjectTracerContext,
};

use crate::{
    abi::{st_table, GCThreadTLS},
    binding::MovedGIVTblEntry,
    extra_assert, is_mmtk_object_safe, upcalls,
    utils::AfterAll,
    Ruby,
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
        _tracer_context: impl ObjectTracerContext<Ruby>,
    ) {
        worker.add_work(WorkBucketStage::VMRefClosure, ProcessObjFreeCandidates);

        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].bulk_add(vec![
            Box::new(UpdateGenericIvTbl) as _,
            Box::new(UpdateFrozenStringsTable) as _,
            Box::new(UpdateFinalizerAndObjIdTables) as _,
            // Box::new(UpdateGlobalSymbolsTable) as _,
            Box::new(UpdateOverloadedCmeTable) as _,
            Box::new(UpdateCiTable) as _,
            Box::new(UpdateWbUnprotectedObjectsList) as _,
        ]);

        let forward = crate::mmtk().get_plan().current_gc_may_move_object();

        Self::process_weak_table_chunked(
            "global symbols",
            (upcalls().get_global_symbols_table)(),
            false,
            true,
            forward,
            worker,
        );
    }

    pub fn process_weak_table_chunked(
        name: &'static str,
        table: *mut st_table,
        weak_keys: bool,
        weak_values: bool,
        forward: bool,
        worker: &mut GCWorker<Ruby>,
    ) {
        let mut entries_start = 0;
        let mut entries_bound = 0;
        let mut bins_num = 0;
        (upcalls().st_get_size_info)(table, &mut entries_start, &mut entries_bound, &mut bins_num);
        let num_entries = (upcalls().st_get_num_entries)(table);
        debug!(
            "name: {name}, entries_start: {entries_start}, entries_bound: {entries_bound}, bins_num: {bins_num}, num_entries: {num_entries}"
        );

        let table_name_ptr = name.as_ptr();
        let table_name_len = name.len();

        probe!(
            mmtk_ruby,
            initial_weak_table_stats,
            entries_start,
            entries_bound,
            bins_num,
            num_entries,
            table_name_ptr,
            table_name_len,
        );

        let entries_chunk_size = crate::binding().st_entries_chunk_size;
        let bins_chunk_size = crate::binding().st_bins_chunk_size;

        let after_all = Arc::new(AfterAll::new(WorkBucketStage::VMRefClosure));

        let entries_packets = (entries_start..entries_bound)
            .step_by(entries_chunk_size)
            .map(|begin| {
                let end = (begin + entries_chunk_size).min(entries_bound);
                let after_all = after_all.clone();
                Box::new(UpdateTableEntriesParallel {
                    name,
                    table,
                    begin,
                    end,
                    weak_keys,
                    weak_values,
                    forward,
                    after_all,
                }) as _
            })
            .collect::<Vec<_>>();
        after_all.count_up(entries_packets.len());

        let bins_packets = (0..bins_num)
            .step_by(entries_chunk_size)
            .map(|begin| {
                let end = (begin + bins_chunk_size).min(bins_num);
                Box::new(UpdateTableBinsParallel {
                    name: name.to_string(),
                    table,
                    begin,
                    end,
                }) as _
            })
            .collect::<Vec<_>>();
        after_all.add_packets(bins_packets);

        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].bulk_add(entries_packets);
    }

    /// Update generic instance variable tables.
    ///
    /// Objects moved during GC should have their entries in the global `generic_iv_tbl_` hash
    /// table updated, and dead objects should have their entries removed.
    fn update_generic_iv_tbl() {
        // Update `generic_iv_tbl_` entries for moved objects.  We could update the entries in
        // `ObjectModel::move`.  However, because `st_table` is not thread-safe, we postpone the
        // update until now in the VMRefClosure stage.
        log::debug!("Updating global ivtbl entries...");
        let items_moved = {
            let mut moved_givtbl = crate::binding()
                .moved_givtbl
                .try_lock()
                .expect("Should have no race in weak_proc");
            let items_moved = moved_givtbl.len();
            for (new_objref, MovedGIVTblEntry { old_objref, .. }) in moved_givtbl.drain() {
                trace!("  givtbl {} -> {}", old_objref, new_objref);
                (upcalls().move_givtbl)(old_objref, new_objref);
            }
            items_moved
        };
        log::debug!("Updated global ivtbl entries.  {items_moved} items moved.");

        // Clean up entries for dead objects.
        let generic_iv_tbl = (upcalls().get_generic_iv_tbl)();
        let old_size = (upcalls().st_get_num_entries)(generic_iv_tbl);
        log::debug!("Cleaning up global ivtbl entries ({old_size} entries before)...");
        (crate::upcalls().cleanup_generic_iv_tbl)();
        let new_size = (upcalls().st_get_num_entries)(generic_iv_tbl);
        log::debug!("Cleaning up global ivtbl entries ({new_size} entries after).");
        probe!(
            mmtk_ruby,
            update_generic_iv_tbl,
            items_moved,
            old_size,
            new_size
        );
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

define_global_table_processor!(UpdateGenericIvTbl, {
    WeakProcessor::update_generic_iv_tbl();
});

define_global_table_processor!(UpdateFrozenStringsTable, {
    let old_size = (upcalls().get_num_fstrings)();
    (upcalls().update_frozen_strings_table)();
    let new_size = (upcalls().get_num_fstrings)();
    probe!(mmtk_ruby, weak_table_size_change, old_size, new_size);
});

fn general_update_weak_table(getter: extern "C" fn() -> *mut st_table, cleaner: extern "C" fn()) {
    let table = getter();
    let old_size = (upcalls().st_get_num_entries)(table);
    cleaner();
    let new_size = (upcalls().st_get_num_entries)(table);
    probe!(mmtk_ruby, weak_table_size_change, old_size, new_size);
}

#[allow(dead_code)]
mod unused {
    use super::*;
    define_global_table_processor!(UpdateGlobalSymbolsTable, {
        general_update_weak_table(
            upcalls().get_global_symbols_table,
            upcalls().update_global_symbols_table,
        );
    });
}

fn st_get_num_entries_maybe_null(table: *const st_table) -> usize {
    if table.is_null() {
        0
    } else {
        (upcalls().st_get_num_entries)(table)
    }
}

define_global_table_processor!(UpdateFinalizerAndObjIdTables, {
    let finalizer_table = (upcalls().get_finalizer_table)();
    let id2ref_table = (upcalls().get_id2ref_table)();

    let old_size_finalizer = (upcalls().st_get_num_entries)(finalizer_table);
    let old_size_id_to_obj = st_get_num_entries_maybe_null(id2ref_table);

    (upcalls().update_finalizer_and_obj_id_tables)();

    let new_size_finalizer = (upcalls().st_get_num_entries)(finalizer_table);
    let new_size_id_to_obj = st_get_num_entries_maybe_null(id2ref_table);

    probe!(
        mmtk_ruby,
        update_finalizer_and_obj_id_tables,
        old_size_finalizer,
        new_size_finalizer,
        old_size_id_to_obj,
        new_size_id_to_obj,
    );
});

define_global_table_processor!(UpdateOverloadedCmeTable, {
    general_update_weak_table(
        upcalls().get_overloaded_cme_table,
        upcalls().update_overloaded_cme_table,
    );
});

define_global_table_processor!(UpdateCiTable, {
    general_update_weak_table(upcalls().get_ci_table, upcalls().update_ci_table);
});

struct UpdateTableEntriesParallel {
    name: &'static str,
    table: *mut st_table,
    begin: usize,
    end: usize,
    weak_keys: bool,
    weak_values: bool,
    forward: bool,
    after_all: Arc<AfterAll>,
}

unsafe impl Send for UpdateTableEntriesParallel {}

impl UpdateTableEntriesParallel {}

impl GCWork<Ruby> for UpdateTableEntriesParallel {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        debug!("Updating entries of {} table", self.name);
        let deleted_entries = (upcalls().st_update_entries_range)(
            self.table,
            self.begin,
            self.end,
            self.weak_keys,
            self.weak_values,
            self.forward,
        );
        debug!("Done updating entries of {} table", self.name);
        let table_name = self.name.as_ptr();
        let table_name_len = self.name.len();
        probe!(
            mmtk_ruby,
            update_table_entries_parallel,
            self.begin,
            self.end,
            deleted_entries,
            table_name,
            table_name_len
        );

        let is_last = self.after_all.count_down(worker);
        if is_last {
            let num_entries = (upcalls().st_get_num_entries)(self.table);
            probe!(
                mmtk_ruby,
                final_weak_table_stats,
                num_entries,
                table_name,
                table_name_len
            )
        }
    }
}

struct UpdateTableBinsParallel {
    name: String,
    table: *mut st_table,
    begin: usize,
    end: usize,
}

unsafe impl Send for UpdateTableBinsParallel {}

impl UpdateTableBinsParallel {}

impl GCWork<Ruby> for UpdateTableBinsParallel {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        debug!("Updating bins of {} table", self.name);
        let deleted_bins = (upcalls().st_update_bins_range)(self.table, self.begin, self.end);
        debug!("Done updating bins of {} table", self.name);
        let table_name = self.name.as_ptr();
        let table_name_len = self.name.len();
        probe!(
            mmtk_ruby,
            update_table_bins_parallel,
            self.begin,
            self.end,
            deleted_bins,
            table_name,
            table_name_len
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
