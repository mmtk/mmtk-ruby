use mmtk::vm::*;
use mmtk::util::{Address, ObjectReference};
use mmtk::AllocationSemantics;
use mmtk::CopyContext;
use crate::Ruby;

pub struct VMObjectModel {}

mod constants {

}

impl ObjectModel<Ruby> for VMObjectModel {
    const GLOBAL_LOG_BIT_SPEC: VMGlobalLogBitSpec = VMGlobalLogBitSpec::side_first();

    const LOCAL_FORWARDING_POINTER_SPEC: VMLocalForwardingPointerSpec =
            VMLocalForwardingPointerSpec::side_first();

    const LOCAL_FORWARDING_BITS_SPEC: VMLocalForwardingBitsSpec =
            VMLocalForwardingBitsSpec::side_after(Self::LOCAL_FORWARDING_POINTER_SPEC.as_spec());

    const LOCAL_MARK_BIT_SPEC: VMLocalMarkBitSpec =
            VMLocalMarkBitSpec::in_header(0);

    const LOCAL_LOS_MARK_NURSERY_SPEC: VMLocalLOSMarkNurserySpec =
            VMLocalLOSMarkNurserySpec::side_after(Self::LOCAL_FORWARDING_BITS_SPEC.as_spec());

    fn load_metadata(
        metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        object: ObjectReference,
        mask: Option<usize>,
        atomic_ordering: Option<std::sync::atomic::Ordering>,
    ) -> usize {
        todo!()
    }

    fn store_metadata(
        metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        object: ObjectReference,
        val: usize,
        mask: Option<usize>,
        atomic_ordering: Option<std::sync::atomic::Ordering>,
    ) {
        todo!()
    }

    fn compare_exchange_metadata(
        metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        object: ObjectReference,
        old_val: usize,
        new_val: usize,
        mask: Option<usize>,
        success_order: std::sync::atomic::Ordering,
        failure_order: std::sync::atomic::Ordering,
    ) -> bool {
        todo!()
    }

    fn fetch_add_metadata(
        metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        object: ObjectReference,
        val: usize,
        order: std::sync::atomic::Ordering,
    ) -> usize {
        todo!()
    }

    fn fetch_sub_metadata(
        metadata_spec: &mmtk::util::metadata::header_metadata::HeaderMetadataSpec,
        object: ObjectReference,
        val: usize,
        order: std::sync::atomic::Ordering,
    ) -> usize {
        todo!()
    }

    fn copy(
        from: ObjectReference,
        semantics: AllocationSemantics,
        copy_context: &mut impl CopyContext,
    ) -> ObjectReference {
        todo!()
    }

    fn copy_to(from: ObjectReference, to: ObjectReference, region: Address) -> Address {
        todo!()
    }

    fn get_reference_when_copied_to(from: ObjectReference, to: Address) -> ObjectReference {
        todo!()
    }

    fn get_current_size(object: ObjectReference) -> usize {
        todo!()
    }

    fn get_type_descriptor(reference: ObjectReference) -> &'static [i8] {
        todo!()
    }

    fn object_start_ref(object: ObjectReference) -> Address {
        todo!()
    }

    fn ref_to_address(object: ObjectReference) -> Address {
        todo!()
    }

    fn dump_object(object: ObjectReference) {
        todo!()
    }
}
