use std::ffi::CString;
use std::sync::Mutex;

use mmtk::MMTK;

use crate::Ruby;
use crate::abi;
use crate::finalize;

pub struct RubyBinding {
    pub mmtk: &'static MMTK<Ruby>,
    pub upcalls: *const abi::RubyUpcalls,
    pub finalizer_processor: finalize::FinalizerProcessor,
    pub plan_name: Mutex<Option<CString>>,
}

unsafe impl Sync for RubyBinding {}
unsafe impl Send for RubyBinding {}

impl RubyBinding {
    pub fn new(mmtk: &'static MMTK<Ruby>, upcalls: *const abi::RubyUpcalls) -> Self {
        Self {
            mmtk,
            upcalls,
            finalizer_processor: finalize::FinalizerProcessor::new(),
            plan_name: Mutex::new(None),
        }
    }

    pub fn register_upcalls(&mut self, upcalls: *const abi::RubyUpcalls) {
        self.upcalls = upcalls;
    }

    pub fn upcalls(&self) -> &'static abi::RubyUpcalls {
        unsafe { &*self.upcalls as &'static abi::RubyUpcalls }
    }

    pub fn get_plan_name_c(&self) -> *const libc::c_char {
        let mut plan_name = self.plan_name.lock().unwrap();
        if plan_name.is_none() {
            let name_string = format!("{:?}", *self.mmtk.get_options().plan);
            let c_string = CString::new(name_string).unwrap_or_else(|e| {
                panic!("Failed converting plan name to CString: {}",
                    e)
            });
            *plan_name = Some(c_string);
        }
        plan_name.as_deref().unwrap().as_ptr()
    }
}
