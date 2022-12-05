use std::ffi::CString;
use std::sync::Mutex;

use mmtk::MMTK;

use crate::abi;
use crate::weak_proc::WeakProcessor;
use crate::abi::RubyBindingOptions;
use crate::Ruby;

#[derive(Default)]
pub struct RubyBindingFast {
    pub suffix_size: usize,
}

impl RubyBindingFast {
    pub const fn new() -> Self {
        Self { suffix_size: 0 }
    }
}

pub struct RubyBinding {
    pub mmtk: &'static MMTK<Ruby>,
    pub options: RubyBindingOptions,
    pub upcalls: *const abi::RubyUpcalls,
    pub plan_name: Mutex<Option<CString>>,
    pub weak_proc: WeakProcessor,
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
            crate::BINDING_FAST.suffix_size = binding_options.suffix_size;
        }
        Self {
            mmtk,
            options: binding_options.clone(),
            upcalls,
            plan_name: Mutex::new(None),
            weak_proc: WeakProcessor::new(),
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
                .unwrap_or_else(|e| panic!("Failed converting plan name to CString: {}", e));
            *plan_name = Some(c_string);
        }
        plan_name.as_deref().unwrap().as_ptr()
    }
}
