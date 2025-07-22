use mmtk::{
    scheduler::{GCWork, WorkBucketStage},
    util::VMWorkerThread,
};

use crate::{abi::GCThreadTLS, upcalls, Ruby};

struct BeforeUpdatingJitCode;

impl GCWork<Ruby> for BeforeUpdatingJitCode {
    fn do_work(
        &mut self,
        _worker: &mut mmtk::scheduler::GCWorker<Ruby>,
        _mmtk: &'static mmtk::MMTK<Ruby>,
    ) {
        (upcalls().before_updating_jit_code)();
    }
}

struct AfterUpdatingJitCode;

impl GCWork<Ruby> for AfterUpdatingJitCode {
    fn do_work(
        &mut self,
        _worker: &mut mmtk::scheduler::GCWorker<Ruby>,
        _mmtk: &'static mmtk::MMTK<Ruby>,
    ) {
        (upcalls().after_updating_jit_code)();
    }
}

pub fn schedule_jit_code_protection_work_packets(tls: VMWorkerThread) {
    let gc_tls: &'static mut GCThreadTLS = unsafe { GCThreadTLS::from_vwt_check(tls) };
    let worker = gc_tls.worker();
    if crate::mmtk().get_plan().current_gc_may_move_object() {
        worker.scheduler().work_buckets[WorkBucketStage::Prepare].add(BeforeUpdatingJitCode);
        worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].add(AfterUpdatingJitCode);
    }
}
