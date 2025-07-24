use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use mmtk::{
    scheduler::{GCWork, GCWorker, WorkBucketStage},
    util::ObjectReference,
};

use crate::{abi::ConcurrentSetStats, upcalls, weak_proc::WeakConcurrentSetKind, Ruby};

pub fn process_weak_concurrent_set_chunked(
    name: &'static str,
    set: Option<ObjectReference>,
    kind: WeakConcurrentSetKind,
    worker: &mut GCWorker<Ruby>,
) {
    let Some(set) = set else {
        debug!("Set {name} is empty.  Skipping.");
        return;
    };

    let num_entries = (upcalls().concurrent_set_get_num_entries)(set);
    let capacity = (upcalls().concurrent_set_get_capacity)(set);
    debug!("name: {name}, num_entries: {num_entries}, capacity: {capacity}");

    let set_name_ptr = name.as_ptr();
    let set_name_len = name.len();

    probe!(
        mmtk_ruby,
        weak_cs_par_init,
        num_entries,
        capacity,
        set_name_ptr,
        set_name_len,
    );

    let chunk_size = crate::binding().concurrent_set_chunk_size;

    let counter = Arc::new(AtomicUsize::new(0));

    let entries_packets = (0..capacity)
        .step_by(chunk_size)
        .map(|begin| {
            let end = (begin + chunk_size).min(capacity);
            Box::new(UpdateConcurrentSetEntriesParallel {
                name,
                set,
                begin,
                end,
                kind: kind as u8,
                counter: counter.clone(),
            }) as _
        })
        .collect::<Vec<_>>();

    counter.fetch_add(entries_packets.len(), Ordering::SeqCst);

    worker.scheduler().work_buckets[WorkBucketStage::VMRefClosure].bulk_add(entries_packets);
}

struct UpdateConcurrentSetEntriesParallel {
    name: &'static str,
    set: ObjectReference,
    begin: usize,
    end: usize,
    kind: u8,
    counter: Arc<AtomicUsize>,
}

unsafe impl Send for UpdateConcurrentSetEntriesParallel {}

impl GCWork<Ruby> for UpdateConcurrentSetEntriesParallel {
    fn do_work(&mut self, _worker: &mut GCWorker<Ruby>, _mmtk: &'static mmtk::MMTK<Ruby>) {
        debug!(
            "Updating concurrent set '{}' range {}-{}",
            self.name, self.begin, self.end
        );
        let set_name = self.name.as_ptr();
        let set_name_len = self.name.len();
        probe!(
            mmtk_ruby,
            weak_cs_par_entries_begin,
            self.begin,
            self.end,
            set_name,
            set_name_len
        );

        let mut stats = ConcurrentSetStats::default();
        (upcalls().concurrent_set_update_entries_range)(
            self.set, self.begin, self.end, self.kind, &mut stats,
        );

        debug!(
            "Done updating entries of concurrent set '{}' range {}-{}, live: {}, moved: {}, deleted: {}",
            self.name, self.begin, self.end, stats.live, stats.moved, stats.deleted
        );
        probe!(
            mmtk_ruby,
            weak_cs_par_entries_end,
            stats.live,
            stats.moved,
            stats.deleted,
        );

        let old_counter = self.counter.fetch_sub(1, Ordering::SeqCst);
        if old_counter == 1 {
            let num_entries = (upcalls().concurrent_set_get_num_entries)(self.set);
            probe!(mmtk_ruby, weak_cs_par_final, num_entries)
        }
    }
}
