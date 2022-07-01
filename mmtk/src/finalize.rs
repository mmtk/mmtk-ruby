use mmtk::util::ObjectReference;

use std::{sync::Mutex, vec};

/// TODO: This is a workaround to support Ruby's semantics that finalizes everything on exit.
/// In the future, it should be supported to upstream.
pub struct FinalizerProcessor {
    candidates: Mutex<Vec<ObjectReference>>,
}

impl FinalizerProcessor {
    pub fn new() -> Self {
        Self {
            candidates: Mutex::new(vec![]),
        }
    }

    pub(crate) fn with_candidates<T, F>(&self, callback: F) -> T
    where
        F: FnOnce(&Vec<ObjectReference>) -> T,
    {
        let guard = self.candidates.lock().unwrap();
        callback(&guard)
    }

    pub fn register_finalizable(&self, reff: ObjectReference) {
        self.candidates.lock().unwrap().push(reff);
    }

    pub fn poll_finalizable(&self, include_live: bool) -> Option<ObjectReference> {
        if include_live {
            self.candidates.lock().unwrap().pop()
        } else {
            None
        }
    }
}
