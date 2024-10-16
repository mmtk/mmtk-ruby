use std::sync::Mutex;

use mmtk::{
    memory_manager,
    scheduler::{GCWork, GCWorker, WorkBucketStage},
    util::{ObjectReference, VMWorkerThread},
    MMTK,
};

use crate::{abi::GCThreadTLS, upcalls, Ruby};

pub struct PPPRegistry {
    ppps: Mutex<Vec<ObjectReference>>,
    pinned_ppp_children: Mutex<Vec<ObjectReference>>,
}

impl PPPRegistry {
    pub fn new() -> Self {
        Self {
            ppps: Default::default(),
            pinned_ppp_children: Default::default(),
        }
    }

    pub fn register(&self, object: ObjectReference) {
        let mut ppps = self.ppps.lock().unwrap();
        ppps.push(object);
    }

    pub fn register_many(&self, objects: &[ObjectReference]) {
        let mut ppps = self.ppps.lock().unwrap();
        for object in objects.iter().copied() {
            ppps.push(object);
        }
    }

    pub fn pin_ppp_children(&self, tls: VMWorkerThread) {
        log::debug!("Pin children of PPPs...");

        if !crate::mmtk().get_plan().current_gc_may_move_object() {
            log::debug!("The current GC is non-moving.  Skipped pinning PPP children.");
            return;
        }

        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(tls) };
        let worker = gc_tls.worker();

        {
            let ppps = self
                .ppps
                .try_lock()
                .expect("PPPRegistry should not have races during GC.");

            // I tried several packet sizes and 512 works pretty well.  It should be adjustable.
            let packet_size = 512;
            let work_packets = ppps
                .chunks(packet_size)
                .map(|chunk| {
                    Box::new(PinPPPChildren {
                        ppps: chunk.to_vec(),
                    }) as _
                })
                .collect();

            worker.scheduler().work_buckets[WorkBucketStage::Prepare].bulk_add(work_packets);
        }
    }

    pub fn cleanup_ppps(&self, worker: &mut GCWorker<Ruby>) {
        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].add(RemoveDeadPPPs);
        if crate::mmtk().get_plan().current_gc_may_move_object() {
            let packet = {
                let mut pinned_ppp_children = self
                    .pinned_ppp_children
                    .try_lock()
                    .expect("Unexpected contention on pinned_ppp_children");
                UnpinPPPChildren {
                    children: std::mem::take(&mut pinned_ppp_children),
                }
            };

            worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].add(packet);
        } else {
            debug!("Skipping unpinning PPP children because the current GC is non-copying.");
            debug_assert_eq!(
                {
                    let pinned_ppp_children = self
                        .pinned_ppp_children
                        .try_lock()
                        .expect("Unexpected contention on pinned_ppp_children");
                    pinned_ppp_children.len()
                },
                0
            );
        }
    }
}

impl Default for PPPRegistry {
    fn default() -> Self {
        Self::new()
    }
}

struct PinPPPChildren {
    ppps: Vec<ObjectReference>,
}

impl GCWork<Ruby> for PinPPPChildren {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static MMTK<Ruby>) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };
        let num_ppps = self.ppps.len();
        let mut ppp_children = vec![];
        let mut newly_pinned_ppp_children = vec![];
        let mut num_no_longer_ppps = 0usize;

        let visit_object = |_worker, target_object: ObjectReference, pin| {
            log::trace!(
                "    -> {} {}",
                if pin { "(pin)" } else { "     " },
                target_object
            );
            if pin {
                ppp_children.push(target_object);
            }
            target_object
        };

        gc_tls
            .object_closure
            .set_temporarily_and_run_code(visit_object, || {
                for obj in self.ppps.iter().cloned() {
                    log::trace!("  PPP: {}", obj);
                    if (upcalls().is_no_longer_ppp)(obj) {
                        num_no_longer_ppps += 1;
                        log::trace!("    No longer PPP. Skip: {}", obj);
                        continue;
                    }
                    (upcalls().call_gc_mark_children)(obj);
                }
            });

        for target_object in ppp_children {
            if memory_manager::pin_object(target_object) {
                newly_pinned_ppp_children.push(target_object);
            }
        }

        let num_pinned_children = newly_pinned_ppp_children.len();

        probe!(
            mmtk_ruby,
            pin_ppp_children,
            num_ppps,
            num_no_longer_ppps,
            num_pinned_children
        );

        {
            let mut pinned_ppp_children = crate::binding()
                .ppp_registry
                .pinned_ppp_children
                .lock()
                .unwrap();
            pinned_ppp_children.append(&mut newly_pinned_ppp_children);
        }
    }
}

struct RemoveDeadPPPs;

impl GCWork<Ruby> for RemoveDeadPPPs {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static MMTK<Ruby>) {
        log::debug!("Removing dead PPPs...");

        let registry = &crate::binding().ppp_registry;
        {
            let mut ppps = registry
                .ppps
                .try_lock()
                .expect("PPPRegistry::ppps should not have races during GC.");

            let num_ppps = ppps.len();
            let mut num_no_longer_ppps = 0usize;
            let mut num_dead_ppps = 0usize;

            ppps.retain_mut(|obj| {
                if obj.is_live() {
                    let new_obj = obj.get_forwarded_object().unwrap_or(*obj);
                    if (upcalls().is_no_longer_ppp)(new_obj) {
                        num_no_longer_ppps += 1;
                        log::trace!("  No longer PPP. Remove: {}", new_obj);
                        false
                    } else {
                        *obj = new_obj;
                        true
                    }
                } else {
                    num_dead_ppps += 1;
                    log::trace!("  Dead PPP removed: {}", *obj);
                    false
                }
            });

            probe!(
                mmtk_ruby,
                remove_dead_ppps,
                num_ppps,
                num_no_longer_ppps,
                num_dead_ppps
            );
        }
    }
}

struct UnpinPPPChildren {
    children: Vec<ObjectReference>,
}

impl GCWork<Ruby> for UnpinPPPChildren {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static MMTK<Ruby>) {
        log::debug!("Unpinning pinned PPP children...");

        let num_children = self.children.len();

        probe!(mmtk_ruby, unpin_ppp_children, num_children);

        for obj in self.children.iter() {
            let unpinned = memory_manager::unpin_object(*obj);
            debug_assert!(unpinned);
        }
    }
}
