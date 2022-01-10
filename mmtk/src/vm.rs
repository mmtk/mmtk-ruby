use crate::finalize;

pub struct BindingState {
    pub finalizer_processor: finalize::FinalizerProcessor,
}

impl BindingState {
    pub fn new() -> Self {
        Self {
            finalizer_processor: finalize::FinalizerProcessor::new(),
        }
    }
}
