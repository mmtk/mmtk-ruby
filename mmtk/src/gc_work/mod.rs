use mmtk::{util::{Address, ObjectReference}, scheduler::{GCWork, GCWorker, ProcessEdgesWork}, MMTK, memory_manager};

use crate::{Ruby, SINGLETON, upcalls, abi::GCThreadTLS, address_buffer::FilledBuffer};
use crate::abi::BufferCallback;

pub struct ObjectsToObjectsWork<PE: ProcessEdgesWork> {
    process_edges: PE,
    src_objs: Vec<ObjectReference>,
}

impl<PE: ProcessEdgesWork<VM=Ruby>> ObjectsToObjectsWork<PE> {
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

impl<PE: ProcessEdgesWork<VM=Ruby>> GCWork<Ruby> for ObjectsToObjectsWork<PE> {
    fn do_work(&mut self, worker: &mut GCWorker<Ruby>, _mmtk: &'static MMTK<Ruby>) {
        debug!("ObjectsToObjectsWork Begin");

        self.process_edges.set_worker(worker);

        debug!("[ObjectsToObjectsWork::do_work] Addresses:");
        for obj in self.src_objs.iter() {
            debug!("[ObjectsToObjectsWork::do_work] Trace object: {}", obj);
            assert!(!obj.is_null());

            self.process_edges.trace_object(*obj);
        }

        // Unlike ProcessEdgesWork, we collect the list of objects and immediately scan all of them.
        let objects_to_scan = self.process_edges.nodes.drain(..).collect::<Vec<_>>();
        let mut dest_objs = Vec::<ObjectReference>::new();

        let gc_thread_tls = GCThreadTLS::from_upcall_check();
        let callback = |_, filled_buffer: FilledBuffer| {
            dest_objs.extend(filled_buffer.as_objref_vec().iter());
        };
        gc_thread_tls.run_with_buffer_callback(callback, |_| {
            for obj in objects_to_scan.iter() {
                assert!(!obj.is_null());
                (upcalls().scan_object_ruby_style)(*obj);
            }
        });

        if dest_objs.is_empty() {
            let next_packet = Self::new(dest_objs);
            memory_manager::add_work_packet(&SINGLETON, mmtk::scheduler::WorkBucketStage::Closure, next_packet);
        }

        // TODO: scan the objects in the buffer.
        debug!("ObjectsToObjectsWork End");
    }
}
