use mmtk::{
    scheduler::{GCWork, GCWorker},
    util::ObjectReference,
};

use crate::{
    abi::GCThreadTLS, extra_assert, is_mmtk_object_safe, upcalls, weak_proc::Forwardable, Ruby,
};

pub trait GlobalTableProcessingWork {
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
        pub struct $name;
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

///////// END: Simple table updating work packets ////////
