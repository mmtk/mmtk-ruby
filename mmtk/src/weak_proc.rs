use std::sync::Mutex;

use mmtk::{
    util::{ObjectReference, VMWorkerThread},
    vm::{ObjectModel, ProcessWeakRefsContext},
};

use crate::{abi::GCThreadTLS, object_model::VMObjectModel, upcalls};

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

    /// Add an object as a candicate for `obj_free`.
    ///
    /// Multiple mutators can call it concurrently, so it has `&self`.
    pub fn add_obj_free_candidate(&self, object: ObjectReference) {
        let mut obj_free_candidates = self.obj_free_candidates.lock().unwrap();
        obj_free_candidates.push(object);
    }

    pub fn get_all_obj_free_candidates(&self) -> Vec<ObjectReference> {
        let mut obj_free_candidates = self.obj_free_candidates.lock().unwrap();
        std::mem::take(obj_free_candidates.as_mut())
    }

    pub fn process_weak_stuff(
        &self,
        tls: VMWorkerThread,
        mut context: impl ProcessWeakRefsContext,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };

        if context.forwarding() {
            panic!("We can't use MarkCompact in Ruby.");
        }

        // If it blocks, it is a bug.
        let mut obj_free_candidates = self
            .obj_free_candidates
            .try_lock()
            .expect("It's GC time.  No mutators should hold this lock at this time.");

        let mut new_candidates = Vec::new();

        for object in obj_free_candidates.iter().copied() {
            if object.is_reachable() {
                new_candidates.push(object);
            } else {
                (upcalls().call_obj_free)(object);
            }
        }

        *obj_free_candidates = new_candidates;

        let forward_object = |_worker, object: ObjectReference| {
            debug_assert!(mmtk::memory_manager::is_mmtk_object(
                VMObjectModel::ref_to_address(object)
            ));
            let result = context.trace_object(object);
            trace!("Forwarding reference: {} -> {}", object, result);
            result
        };

        gc_tls
            .object_closure
            .set_temporarily_and_run_code(forward_object, || {
                log::debug!("Updating global weak tables...");
                (upcalls().update_global_weak_tables)();
                log::debug!("Finished updating global weak tables.");
            });
    }
}
