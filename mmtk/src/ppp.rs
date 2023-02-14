use std::sync::Mutex;

use atomic_refcell::AtomicRefCell;
use mmtk::{
    memory_manager,
    util::{ObjectReference, VMWorkerThread},
};

use crate::{abi::GCThreadTLS, upcalls, Ruby};

pub struct PPPRegistry {
    ppps: Mutex<Vec<ObjectReference>>,
    pinned_ppps: AtomicRefCell<Vec<ObjectReference>>,
}

impl PPPRegistry {
    pub fn new() -> Self {
        Self {
            ppps: Default::default(),
            pinned_ppps: Default::default(),
        }
    }

    pub fn register(&self, object: ObjectReference) {
        let mut ppps = self.ppps.lock().unwrap();
        ppps.push(object);
    }

    pub fn pin_ppps(&self, tls: VMWorkerThread) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };

        let mut newly_pinned_ppps = vec![];

        log::debug!("Pin children of PPPs...");

        {
            let ppps = self
                .ppps
                .try_lock()
                .expect("PPPRegistry should not have races during GC.");

            for obj in ppps.iter().cloned() {
                log::trace!("  PPP: {}", obj);

                let visit_object = |_worker, target_object: ObjectReference, pin| {
                    log::trace!(
                        "    -> {} {}",
                        if pin { "(pin)" } else { "     " },
                        target_object
                    );
                    if pin && memory_manager::pin_object::<Ruby>(target_object) {
                        newly_pinned_ppps.push(target_object);
                    }
                    target_object
                };
                gc_tls
                    .object_closure
                    .set_temporarily_and_run_code(visit_object, || {
                        (upcalls().call_gc_mark_children)(obj);
                    });
            }
        }

        {
            let mut pinned_ppps = self.pinned_ppps.borrow_mut();
            *pinned_ppps = newly_pinned_ppps;
        }
    }

    pub fn cleanup_ppps(&self) {
        log::debug!("Removing dead PPPs...");
        {
            let mut ppps = self
                .ppps
                .try_lock()
                .expect("PPPRegistry should not have races during GC.");

            ppps.retain_mut(|obj| {
                if obj.is_live() {
                    *obj = obj.get_forwarded_object().unwrap_or(*obj);
                    true
                } else {
                    log::trace!("  PPP removed: {}", *obj);
                    false
                }
            });
        }

        log::debug!("Unpinning pinned roots...");
        {
            let mut pinned_ppps = self.pinned_ppps.borrow_mut();
            for obj in pinned_ppps.drain(..) {
                let unpinned = memory_manager::unpin_object::<Ruby>(obj);
                debug_assert!(unpinned);
            }
        }
    }
}

impl Default for PPPRegistry {
    fn default() -> Self {
        Self::new()
    }
}
