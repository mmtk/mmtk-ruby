use std::collections::HashMap;
use std::ffi::CString;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::thread::JoinHandle;

use libc::c_void;
use mmtk::util::ObjectReference;
use mmtk::MMTK;

use crate::abi;
use crate::abi::RubyBindingOptions;
use crate::ppp::PPPRegistry;
use crate::weak_proc::WeakProcessor;
use crate::Ruby;

pub struct RubyBindingFast {
    pub gc_enabled: AtomicBool,
}

impl RubyBindingFast {
    pub const fn new() -> Self {
        Self {
            // Mimic the old behavior when the gc_enabled flag was in mmtk-core.
            // We may refactor it so that it is false by default.
            gc_enabled: AtomicBool::new(true),
        }
    }
}

pub struct RubyBindingFastMut {
    pub suffix_size: usize,
}

impl RubyBindingFastMut {
    pub const fn new() -> Self {
        Self { suffix_size: 0 }
    }
}

pub(crate) struct MovedGIVTblEntry {
    pub old_objref: ObjectReference,
    pub gen_ivtbl: *mut c_void,
}

pub struct RubyBinding {
    pub mmtk: &'static MMTK<Ruby>,
    pub options: RubyBindingOptions,
    pub upcalls: *const abi::RubyUpcalls,
    pub plan_name: Mutex<Option<CString>>,
    pub weak_proc: WeakProcessor,
    pub ppp_registry: PPPRegistry,
    pub(crate) moved_givtbl: Mutex<HashMap<ObjectReference, MovedGIVTblEntry>>,
    pub gc_thread_join_handles: Mutex<Vec<JoinHandle<()>>>,
}

unsafe impl Sync for RubyBinding {}
unsafe impl Send for RubyBinding {}

impl RubyBinding {
    pub fn new(
        mmtk: &'static MMTK<Ruby>,
        binding_options: &RubyBindingOptions,
        upcalls: *const abi::RubyUpcalls,
    ) -> Self {
        unsafe {
            crate::BINDING_FAST_MUT.suffix_size = binding_options.suffix_size;
        }
        Self {
            mmtk,
            options: binding_options.clone(),
            upcalls,
            plan_name: Mutex::new(None),
            weak_proc: WeakProcessor::new(),
            ppp_registry: PPPRegistry::new(),
            moved_givtbl: Default::default(),
            gc_thread_join_handles: Default::default(),
        }
    }

    pub fn upcalls(&self) -> &'static abi::RubyUpcalls {
        unsafe { &*self.upcalls as &'static abi::RubyUpcalls }
    }

    pub fn get_plan_name_c(&self) -> *const libc::c_char {
        let mut plan_name = self.plan_name.lock().unwrap();
        if plan_name.is_none() {
            let name_string = format!("{:?}", *self.mmtk.get_options().plan);
            let c_string = CString::new(name_string)
                .unwrap_or_else(|e| panic!("Failed converting plan name to CString: {e}"));
            *plan_name = Some(c_string);
        }
        plan_name.as_deref().unwrap().as_ptr()
    }

    pub fn join_all_gc_threads(&self) {
        let handles = {
            let mut guard = self.gc_thread_join_handles.lock().unwrap();
            std::mem::take(&mut *guard)
        };

        debug!("Joining GC threads...");
        let total = handles.len();
        let mut joined = 0;
        for handle in handles {
            handle.join().unwrap();
            joined += 1;
            debug!("{joined}/{total} GC threads joined.");
        }
    }
}
