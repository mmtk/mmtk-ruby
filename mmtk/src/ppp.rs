use std::sync::Mutex;

use mmtk::{
    memory_manager,
    scheduler::{GCWork, GCWorker, WorkBucketStage},
    util::ObjectReference,
};

use crate::{abi::GCThreadTLS, upcalls, utils::GenList, Ruby};

pub struct PPPRegistry {
    ppps: Mutex<GenList<ObjectReference>>,
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
        ppps.extend(objects.iter().copied());
    }

    pub fn pin_ppp_children(&self, worker: &mut GCWorker<Ruby>) {
        log::debug!("Pin children of PPPs...");

        if !crate::binding().current_gc_may_move_object() {
            log::debug!("The current GC is non-moving.  Skipped pinning PPP children.");
            return;
        }

        {
            let ppps = self
                .ppps
                .try_lock()
                .expect("PPPRegistry should not have races during GC.");

            probe!(mmtk_ruby, pin_ppps_prepare, ppps.young().len(), ppps.old().len());

            // I tried several packet sizes and 512 works pretty well.  It should be adjustable.
            let packet_size = 512;
            let mut work_packets = Vec::new();
            let mut visit_refs = |s: &[ObjectReference]| {
                work_packets.extend(s.chunks(packet_size).map(|chunk| {
                    Box::new(PinPPPChildren {
                        ppps: chunk.to_vec(),
                    }) as _
                }))
            };
            visit_refs(ppps.young());
            if !crate::binding().is_current_gc_nursery() {
                visit_refs(ppps.old());
            }

            worker.scheduler().work_buckets[WorkBucketStage::Prepare].bulk_add(work_packets);
        }
    }

    pub fn cleanup_ppps(&self, worker: &mut GCWorker<Ruby>) {
        worker.add_work(WorkBucketStage::VMRefClosure, RemoveDeadPPPs);
        if !crate::binding().current_gc_may_move_object() {
            log::debug!("The current GC is non-moving.  Skipped unpinning PPP children.");
        } else {
            worker.add_work(WorkBucketStage::VMRefClosure, UnpinPPPChildren);
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
    fn do_work(
        &mut self,
        worker: &mut mmtk::scheduler::GCWorker<Ruby>,
        _mmtk: &'static mmtk::MMTK<Ruby>,
    ) {
        let gc_tls = unsafe { GCThreadTLS::from_vwt_check(worker.tls) };
        let mut ppp_children = vec![];
        let mut newly_pinned_ppp_children = vec![];

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
                    (upcalls().call_gc_mark_children)(obj);
                }
            });

        for target_object in ppp_children {
            if memory_manager::pin_object::<Ruby>(target_object) {
                newly_pinned_ppp_children.push(target_object);
            }
        }

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
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        log::debug!("Removing dead PPPs...");

        let mut ppps = crate::binding()
            .ppp_registry
            .ppps
            .try_lock()
            .expect("PPPRegistry::ppps should not have races during GC.");

        let young_ppps = ppps.young().len();
        let old_ppps = ppps.old().len();

        let mut dead_ppps = 0;
        let mut no_longer_ppps = 0;

        let mut visit_ppp = |obj: &mut ObjectReference| {
            if obj.is_live::<Ruby>() {
                if (upcalls().is_ppp)(*obj) {
                    *obj = obj.get_forwarded_object::<Ruby>().unwrap_or(*obj);
                    true
                } else {
                    no_longer_ppps += 1;
                    log::trace!("  No longer PPP.  Removed: {}", *obj);
                    false
                }
            } else {
                dead_ppps += 1;
                log::trace!("  Dead PPP removed: {}", *obj);
                false
            }
        };

        log::debug!("Removing dead young PPPs...");
        ppps.retain_mut_young(&mut visit_ppp);

        if !crate::binding().is_current_gc_nursery() {
            log::debug!("Removing dead old PPPs...");
            ppps.retain_mut_old(&mut visit_ppp);
        }

        probe!(mmtk_ruby, remove_dead_ppps, young_ppps, old_ppps, dead_ppps, no_longer_ppps);

        ppps.promote();
    }
}

struct UnpinPPPChildren;

impl GCWork<Ruby> for UnpinPPPChildren {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        log::debug!("Unpinning pinned PPP children...");

        let mut pinned_ppps = crate::binding()
            .ppp_registry
            .pinned_ppp_children
            .try_lock()
            .expect("PPPRegistry::pinned_ppp_children should not have races during GC.");

        probe!(mmtk_ruby, unpin_ppp_children, pinned_ppps.len());

        for obj in pinned_ppps.drain(..) {
            let unpinned = memory_manager::unpin_object::<Ruby>(obj);
            debug_assert!(unpinned);
        }
    }
}
