use std::ptr::null;

use crate::abi;
use crate::finalize;

pub struct RubyBinding {
    pub upcalls: *const abi::RubyUpcalls,
    pub finalizer_processor: finalize::FinalizerProcessor,
}

unsafe impl Sync for RubyBinding {}

impl RubyBinding {
    pub fn new() -> Self {
        Self {
            upcalls: null(),
            finalizer_processor: finalize::FinalizerProcessor::new(),
        }
    }

    pub fn register_upcalls(&mut self, upcalls: *const abi::RubyUpcalls) {
        self.upcalls = upcalls;
    }

    pub fn upcalls(&self) -> &'static abi::RubyUpcalls {
        unsafe { &*self.upcalls as &'static abi::RubyUpcalls }
    }
}
