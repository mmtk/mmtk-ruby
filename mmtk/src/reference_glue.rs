use crate::Ruby;
use mmtk::util::ObjectReference;
use mmtk::util::VMWorkerThread;
use mmtk::vm::ReferenceGlue;

pub struct VMReferenceGlue {}

impl ReferenceGlue<Ruby> for VMReferenceGlue {
    fn get_referent(_object: ObjectReference) -> ObjectReference {
        unimplemented!()
    }

    fn set_referent(_reff: ObjectReference, _referent: ObjectReference) {
        unimplemented!()
    }

    fn enqueue_references(_references: &[ObjectReference], _tls: VMWorkerThread) {
        unimplemented!()
    }
}
