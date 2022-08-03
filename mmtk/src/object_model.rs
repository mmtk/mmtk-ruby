use crate::Ruby;
use mmtk::util::copy::{CopySemantics, GCWorkerCopyContext};
use mmtk::util::{Address, ObjectReference};
use mmtk::vm::*;

pub struct VMObjectModel {}

impl VMObjectModel {
    const OBJREF_OFFSET: usize = 8;
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

    fn load_metadata(
        _metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        _object: ObjectReference,
        _mask: Option<usize>,
        _atomic_ordering: Option<std::sync::atomic::Ordering>,
    ) -> usize {
        todo!()
    }

    fn store_metadata(
        _metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        _object: ObjectReference,
        _val: usize,
        _mask: Option<usize>,
        _atomic_ordering: Option<std::sync::atomic::Ordering>,
    ) {
        todo!()
    }

    fn compare_exchange_metadata(
        _metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        _object: ObjectReference,
        _old_val: usize,
        _new_val: usize,
        _mask: Option<usize>,
        _success_order: std::sync::atomic::Ordering,
        _failure_order: std::sync::atomic::Ordering,
    ) -> bool {
        todo!()
    }

    fn fetch_add_metadata(
        _metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        _object: ObjectReference,
        _val: usize,
        _order: std::sync::atomic::Ordering,
    ) -> usize {
        todo!()
    }

    fn fetch_sub_metadata(
        _metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        _object: ObjectReference,
        _val: usize,
        _order: std::sync::atomic::Ordering,
    ) -> usize {
        todo!()
    }

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

    fn get_current_size(_object: ObjectReference) -> usize {
        todo!()
    }

    fn get_type_descriptor(_reference: ObjectReference) -> &'static [i8] {
        todo!()
    }

    fn object_start_ref(object: ObjectReference) -> Address {
        object.to_address() - Self::OBJREF_OFFSET
    }

    fn ref_to_address(_object: ObjectReference) -> Address {
        todo!()
    }

    fn dump_object(_object: ObjectReference) {
        todo!()
    }

    fn get_size_when_copied(_object: ObjectReference) -> usize {
        todo!()
    }

    fn get_align_when_copied(_object: ObjectReference) -> usize {
        todo!()
    }

    fn get_align_offset_when_copied(_object: ObjectReference) -> isize {
        todo!()
    }
}
