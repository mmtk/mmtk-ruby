use std::ptr::copy_nonoverlapping;

use crate::abi::{RubyObjectAccess, MIN_OBJ_ALIGN, OBJREF_OFFSET};
use crate::{abi, Ruby};
use mmtk::util::constants::BITS_IN_BYTE;
use mmtk::util::copy::{CopySemantics, GCWorkerCopyContext};
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::*;

pub struct VMObjectModel {}

impl VMObjectModel {
    const OBJREF_OFFSET: usize = abi::OBJREF_OFFSET;
}

impl ObjectModel<Ruby> for VMObjectModel {
    const GLOBAL_LOG_BIT_SPEC: VMGlobalLogBitSpec = VMGlobalLogBitSpec::side_first();

    // We overwrite the prepended word which were used to hold object sizes.
    const LOCAL_FORWARDING_POINTER_SPEC: VMLocalForwardingPointerSpec =
        VMLocalForwardingPointerSpec::in_header(-((OBJREF_OFFSET * BITS_IN_BYTE) as isize));

    const LOCAL_FORWARDING_BITS_SPEC: VMLocalForwardingBitsSpec =
        VMLocalForwardingBitsSpec::side_first();

    const LOCAL_MARK_BIT_SPEC: VMLocalMarkBitSpec =
        VMLocalMarkBitSpec::side_after(Self::LOCAL_FORWARDING_BITS_SPEC.as_spec());

    const LOCAL_PINNING_BIT_SPEC: VMLocalPinningBitSpec =
        VMLocalPinningBitSpec::side_after(Self::LOCAL_MARK_BIT_SPEC.as_spec());

    const LOCAL_LOS_MARK_NURSERY_SPEC: VMLocalLOSMarkNurserySpec =
        VMLocalLOSMarkNurserySpec::side_after(Self::LOCAL_PINNING_BIT_SPEC.as_spec());

    const UNIFIED_OBJECT_REFERENCE_ADDRESS: bool = false;
    const OBJECT_REF_OFFSET_LOWER_BOUND: isize = Self::OBJREF_OFFSET as isize;

    fn copy(
        from: ObjectReference,
        semantics: CopySemantics,
        copy_context: &mut GCWorkerCopyContext<Ruby>,
    ) -> ObjectReference {
        let from_acc = RubyObjectAccess::from_objref(from);
        let from_start = from_acc.obj_start();
        let object_size = from_acc.object_size();
        let to_start = copy_context.alloc_copy(from, object_size, MIN_OBJ_ALIGN, 0, semantics);
        let to_payload = to_start.add(OBJREF_OFFSET);
        unsafe {
            copy_nonoverlapping::<u8>(from_start.to_ptr(), to_start.to_mut_ptr(), object_size);
        }
        let to_obj = ObjectReference::from_raw_address(to_payload);
        copy_context.post_copy(to_obj, object_size, semantics);

        if cfg!(feature = "clear_old_copy") {
            // For debug purpose, we clear the old copy so that if the Ruby VM reads from the old
            // copy again, it will likely result in an error.
            unsafe { std::ptr::write_bytes::<u8>(from_start.to_mut_ptr(), 0, object_size) }
        }

        to_obj
    }

    fn copy_to(_from: ObjectReference, _to: ObjectReference, _region: Address) -> Address {
        unimplemented!(
            "This function cannot be called because we do not support MarkCompact for Ruby."
        )
    }

    fn get_reference_when_copied_to(_from: ObjectReference, _to: Address) -> ObjectReference {
        unimplemented!(
            "This function cannot be called because we do not support MarkCompact for Ruby."
        )
    }

    fn get_current_size(object: ObjectReference) -> usize {
        RubyObjectAccess::from_objref(object).object_size()
    }

    fn get_type_descriptor(_reference: ObjectReference) -> &'static [i8] {
        todo!()
    }

    fn ref_to_address(object: ObjectReference) -> Address {
        object.to_raw_address()
    }

    fn ref_to_object_start(object: ObjectReference) -> Address {
        RubyObjectAccess::from_objref(object).obj_start()
    }

    fn ref_to_header(object: ObjectReference) -> Address {
        RubyObjectAccess::from_objref(object).payload_addr()
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
