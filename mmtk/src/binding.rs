use std::collections::HashMap;
use std::ffi::CString;
use std::sync::Mutex;

use libc::c_void;
use mmtk::util::ObjectReference;
use mmtk::MMTK;

use crate::abi::RubyBindingOptions;
use crate::ppp::PPPRegistry;
use crate::weak_proc::WeakProcessor;
use crate::Ruby;
use crate::{abi, scanning};

#[derive(Default)]
pub struct RubyBindingFast {
    pub suffix_size: usize,
}

impl RubyBindingFast {
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

        if cfg!(feature = "env_var_fast_path_switch") {
            if let Ok(s) = std::env::var("RUBY_MMTK_USE_FAST_PATHS") {
                if let Ok(num) = s.parse::<usize>() {
                    if num == 1 {
                        scanning::USE_FAST_PATHS.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            if scanning::USE_FAST_PATHS.load(std::sync::atomic::Ordering::Relaxed) {
                eprintln!("Using fast paths.");
            } else {
                eprintln!("Using slow paths, only.");
            }
        }

        Self {
            mmtk,
            options: binding_options.clone(),
            upcalls,
            plan_name: Mutex::new(None),
            weak_proc: WeakProcessor::new(),
            ppp_registry: PPPRegistry::new(),
            moved_givtbl: Default::default(),
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
}
