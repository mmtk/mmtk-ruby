use crate::{abi, Ruby};
use mmtk::util::copy::{CopySemantics, GCWorkerCopyContext};
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::*;

pub struct VMObjectModel {}

impl VMObjectModel {
    const OBJREF_OFFSET: usize = abi::OBJREF_OFFSET;
}

impl ObjectModel<Ruby> for VMObjectModel {
    const GLOBAL_LOG_BIT_SPEC: VMGlobalLogBitSpec = VMGlobalLogBitSpec::side_first();

    // FIXME: 0 is probably not right.  We will correct this once we start to support copying GC.
    const LOCAL_FORWARDING_POINTER_SPEC: VMLocalForwardingPointerSpec =
        VMLocalForwardingPointerSpec::in_header(0);

    const LOCAL_FORWARDING_BITS_SPEC: VMLocalForwardingBitsSpec =
        VMLocalForwardingBitsSpec::side_first();

    const LOCAL_MARK_BIT_SPEC: VMLocalMarkBitSpec =
        VMLocalMarkBitSpec::side_after(Self::LOCAL_FORWARDING_BITS_SPEC.as_spec());

    const LOCAL_LOS_MARK_NURSERY_SPEC: VMLocalLOSMarkNurserySpec =
        VMLocalLOSMarkNurserySpec::side_after(Self::LOCAL_MARK_BIT_SPEC.as_spec());

    const UNIFIED_OBJECT_REFERENCE_ADDRESS: bool = false;
    const OBJECT_REF_OFFSET_LOWER_BOUND: isize = Self::OBJREF_OFFSET as isize;

    fn copy(
        _from: ObjectReference,
        _semantics: CopySemantics,
        _copy_context: &mut GCWorkerCopyContext<Ruby>,
    ) -> ObjectReference {
        todo!()
    }

    fn copy_to(_from: ObjectReference, _to: ObjectReference, _region: Address) -> Address {
        todo!()
    }

    fn get_reference_when_copied_to(_from: ObjectReference, _to: Address) -> ObjectReference {
        todo!()
    }

    fn get_current_size(object: ObjectReference) -> usize {
        // Currently, a hidden size field of word size is placed before each object.
        let prefix_size = abi::OBJREF_OFFSET;

        // That hidden field holds the payload size.
        let start = Self::ref_to_object_start(object);
        let payload_size = unsafe { start.load::<usize>() };

        // In RACTOR_CHECK_MODE, Ruby hides a field after each object to hold the Ractor ID.
        let suffix_size = unsafe { crate::BINDING_FAST.suffix_size };
        prefix_size + payload_size + suffix_size
    }

    fn get_type_descriptor(_reference: ObjectReference) -> &'static [i8] {
        todo!()
    }

    fn ref_to_address(object: ObjectReference) -> Address {
        object.to_raw_address()
    }

    fn ref_to_object_start(object: ObjectReference) -> Address {
        object.to_raw_address() - Self::OBJREF_OFFSET
    }

    fn ref_to_header(object: ObjectReference) -> Address {
        object.to_raw_address()
    }

    fn address_to_ref(addr: Address) -> ObjectReference {
        ObjectReference::from_raw_address(addr)
    }

    fn get_size_when_copied(object: ObjectReference) -> usize {
        Self::get_current_size(object)
    }

    fn get_align_when_copied(_object: ObjectReference) -> usize {
        todo!()
    }

    fn get_align_offset_when_copied(_object: ObjectReference) -> isize {
        todo!()
    }

    fn dump_object(_object: ObjectReference) {
        todo!()
    }
}
