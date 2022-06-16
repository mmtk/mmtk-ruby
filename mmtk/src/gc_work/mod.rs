use mmtk::{
    memory_manager,
    scheduler::{GCWork, GCWorker, ProcessEdgesWork},
    util::{Address, ObjectReference},
    MMTK,
};

use crate::{abi::GCThreadTLS, address_buffer::FilledBuffer, upcalls, Ruby, SINGLETON};

pub struct ObjectsToObjectsWork<PE: ProcessEdgesWork> {
    process_edges: PE,
    src_objs: Vec<ObjectReference>,
}

impl<PE: ProcessEdgesWork<VM = Ruby>> ObjectsToObjectsWork<PE> {
    pub fn from_addr_vec(addrs: Vec<Address>) -> Self {
        let src_objs = addrs
            .into_iter()
            .map(|a| unsafe { a.to_object_reference() })
            .collect();
        Self::new(src_objs)
    }

    pub fn new(src_objs: Vec<ObjectReference>) -> Self {
        Self {
            process_edges: PE::new(Vec::new(), false, &SINGLETON),
            src_objs,
        }
    }
}

impl<PE: ProcessEdgesWork<VM = Ruby>> GCWork<Ruby> for ObjectsToObjectsWork<PE> {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static MMTK<Ruby>) {
        trace!("ObjectsToObjectsWork begins");

        self.process_edges.set_worker(worker);

        trace!("Begin: tracing objects");
        for obj in self.src_objs.iter() {
            trace!("Trace object: {}", obj);
            assert!(!obj.is_null());

            self.process_edges.trace_object(*obj);
        }
        trace!("End: tracing objects");

        // Unlike ProcessEdgesWork, we collect the list of objects and immediately scan all of them.
        let objects_to_scan = self.process_edges.nodes.take();
        let mut dest_objs = Vec::<ObjectReference>::new();

        let gc_thread_tls = GCThreadTLS::from_upcall_check();
        let callback = |_, filled_buffer: FilledBuffer| {
            dest_objs.extend(filled_buffer.as_objref_vec().iter());
        };
        trace!("Begin: run_with_buffer_callback");
        gc_thread_tls.run_with_buffer_callback(callback, |_| {
            for obj in objects_to_scan.iter() {
                trace!("Scan object: {}", obj);
                assert!(!obj.is_null());
                (upcalls().scan_object_ruby_style)(*obj);
            }
        });
        trace!("End: run_with_buffer_callback");

        if log_enabled!(log::Level::Trace) {
            trace!("Begin reference in the next packet");
            for obj in dest_objs.iter() {
                trace!("  reference: {}", obj);
            }
            trace!("End reference in the next packet");
        }

        if !dest_objs.is_empty() {
            // Just use ProcessEdgesWork::CAPACITY as a heurestic for slicing packets up.
            for segment in dest_objs.chunks(PE::CAPACITY) {
                let next_packet = Self::new(segment.to_vec());
                memory_manager::add_work_packet(
                    &SINGLETON,
                    mmtk::scheduler::WorkBucketStage::Closure,
                    next_packet,
                );
            }
        }

        trace!("ObjectsToObjectsWork ends");
    }
}
