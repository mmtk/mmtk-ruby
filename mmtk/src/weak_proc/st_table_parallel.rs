use std::sync::Arc;

use mmtk::scheduler::{GCWork, GCWorker, WorkBucketStage};

use crate::{abi::st_table, upcalls, utils::AfterAll, Ruby};

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
        weak_st_par_init,
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
            weak_st_par_entries,
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
                weak_st_par_final,
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
            weak_st_par_bins,
            self.begin,
            self.end,
            deleted_bins,
            table_name,
            table_name_len
        );
    }
}
