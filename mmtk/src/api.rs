// Functions in this module are unsafe for one reason:
// They are called by C functions and they need to pass raw pointers to Rust.
#![allow(clippy::missing_safety_doc)]

use std::ffi::CStr;
use std::sync::atomic::Ordering;

use crate::abi;
use crate::abi::HiddenHeader;
use crate::abi::RawVecOfObjRef;
use crate::abi::RubyBindingOptions;
use crate::abi::VALUE;
use crate::binding;
use crate::binding::RubyBinding;
use crate::mmtk;
use crate::Ruby;
use crate::RubySlot;
use crate::BINDING_FAST;
use mmtk::memory_manager;
use mmtk::memory_manager::mmtk_init;
use mmtk::util::alloc::AllocatorInfo;
use mmtk::util::alloc::AllocatorSelector;
use mmtk::util::api_util::NullableObjectReference;
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::options::GCTriggerSelector;
use mmtk::util::options::PlanSelector;
use mmtk::util::{Address, ObjectReference};
use mmtk::util::{VMMutatorThread, VMThread};
use mmtk::AllocationSemantics;
use mmtk::MMTKBuilder;
use mmtk::Mutator;

// For cbindgen to generate simple type names.
/// cbindgen:ignore
pub type RubyMutator = Mutator<Ruby>;

/// Create an MMTKBuilder instance with default options.
/// This instance shall be consumed by `mmtk_init_binding`.
#[no_mangle]
pub extern "C" fn mmtk_builder_default() -> *mut MMTKBuilder {
    let mut builder = MMTKBuilder::new_no_env_vars();
    // We don't use the Java-style finalization framework in mmtk-core.
    builder.options.no_finalizer.set(true);
    Box::into_raw(Box::new(builder))
}

/// Let the MMTKBuilder read options from environment variables,
/// such as `MMTK_THREADS`.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_read_env_var_settings(builder: *mut MMTKBuilder) {
    let builder = unsafe { &mut *builder };
    builder.options.read_env_var_settings();
}

/// Set the GC trigger to dynamically adjust heap size.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_set_dynamic_heap_size(
    builder: *mut MMTKBuilder,
    low: usize,
    high: usize,
) {
    let builder = unsafe { &mut *builder };
    builder
        .options
        .gc_trigger
        .set(GCTriggerSelector::DynamicHeapSize(low, high));
}

/// Set the GC trigger to use a fixed heap size.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_set_fixed_heap_size(
    builder: *mut MMTKBuilder,
    heap_size: usize,
) {
    let builder = unsafe { &mut *builder };
    builder
        .options
        .gc_trigger
        .set(GCTriggerSelector::FixedHeapSize(heap_size));
}

/// Set the plan.  `plan_name` is a case-sensitive C-style ('\0'-terminated) string matching
/// one of the cases of `enum PlanSelector`.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_set_plan(
    builder: *mut MMTKBuilder,
    plan_name: *const libc::c_char,
) {
    let builder = unsafe { &mut *builder };
    let plan_name_cstr = unsafe { CStr::from_ptr(plan_name) };
    let plan_name_str = plan_name_cstr.to_str().unwrap();
    let plan_selector = plan_name_str.parse::<PlanSelector>().unwrap();
    builder.options.plan.set(plan_selector);
}

/// Query if the selected plan is MarkSweep.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_is_mark_sweep(builder: *mut MMTKBuilder) -> bool {
    let builder = unsafe { &mut *builder };
    matches!(*builder.options.plan, PlanSelector::MarkSweep)
}

/// Query if the selected plan is Immix.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_is_immix(builder: *mut MMTKBuilder) -> bool {
    let builder = unsafe { &mut *builder };
    matches!(*builder.options.plan, PlanSelector::Immix)
}

/// Query if the selected plan is StickyImmix.
#[no_mangle]
pub unsafe extern "C" fn mmtk_builder_is_sticky_immix(builder: *mut MMTKBuilder) -> bool {
    let builder = unsafe { &mut *builder };
    matches!(*builder.options.plan, PlanSelector::StickyImmix)
}

/// Build an MMTk instance.
///
/// -   `builder` is the pointer to the `MMTKBuilder` instance created by the
///     `mmtk_builder_default()` function, and the `MMTKBuilder` will be consumed after building
///     the MMTk instance.
/// -   `upcalls` points to the struct that contains upcalls.  It is allocated in C as static.
#[no_mangle]
pub unsafe extern "C" fn mmtk_init_binding(
    builder: *mut MMTKBuilder,
    binding_options: *const RubyBindingOptions,
    upcalls: *const abi::RubyUpcalls,
) {
    crate::set_panic_hook();

    let builder = unsafe { Box::from_raw(builder) };
    let binding_options = unsafe { &*binding_options };
    let mmtk_boxed = mmtk_init(&builder);
    let mmtk_static = Box::leak(Box::new(mmtk_boxed));

    let binding = RubyBinding::new(mmtk_static, binding_options, upcalls);

    crate::BINDING
        .set(binding)
        .unwrap_or_else(|_| panic!("Binding is already initialized"));
}

#[no_mangle]
pub extern "C" fn mmtk_bind_mutator(tls: VMMutatorThread) -> *mut RubyMutator {
    Box::into_raw(memory_manager::bind_mutator(mmtk(), tls))
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_destroy_mutator(mutator: *mut RubyMutator) {
    let mut boxed_mutator = unsafe { Box::from_raw(mutator) };
    memory_manager::destroy_mutator(boxed_mutator.as_mut())
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_alloc(
    mutator: *mut RubyMutator,
    size: usize,
    align: usize,
    offset: usize,
    semantics: AllocationSemantics,
) -> Address {
    let clamped_size = size.max(MIN_OBJECT_SIZE);
    memory_manager::alloc::<Ruby>(
        unsafe { &mut *mutator },
        clamped_size,
        align,
        offset,
        semantics,
    )
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_post_alloc(
    mutator: *mut RubyMutator,
    refer: ObjectReference,
    bytes: usize,
    semantics: AllocationSemantics,
) {
    memory_manager::post_alloc::<Ruby>(unsafe { &mut *mutator }, refer, bytes, semantics)
}

#[no_mangle]
pub extern "C" fn mmtk_will_never_move(object: ObjectReference) -> bool {
    !object.is_movable()
}

#[no_mangle]
pub extern "C" fn mmtk_initialize_collection(tls: VMThread) {
    memory_manager::initialize_collection(mmtk(), tls)
}

#[no_mangle]
pub extern "C" fn mmtk_prepare_to_fork() {
    mmtk().prepare_to_fork();
    binding().join_all_gc_threads();
}

#[no_mangle]
pub extern "C" fn mmtk_after_fork(tls: VMThread) {
    mmtk().after_fork(tls);
}

#[no_mangle]
pub extern "C" fn mmtk_enable_collection() {
    BINDING_FAST.gc_enabled.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn mmtk_disable_collection() {
    BINDING_FAST.gc_enabled.store(false, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn mmtk_is_collection_enabled() -> bool {
    BINDING_FAST.gc_enabled.load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn mmtk_plan_name() -> *const libc::c_char {
    crate::binding().get_plan_name_c()
}

#[no_mangle]
pub extern "C" fn mmtk_used_bytes() -> usize {
    memory_manager::used_bytes(mmtk())
}

#[no_mangle]
pub extern "C" fn mmtk_free_bytes() -> usize {
    memory_manager::free_bytes(mmtk())
}

#[no_mangle]
pub extern "C" fn mmtk_total_bytes() -> usize {
    memory_manager::total_bytes(mmtk())
}

#[no_mangle]
pub extern "C" fn mmtk_is_reachable(object: ObjectReference) -> bool {
    object.is_reachable()
}

#[no_mangle]
pub extern "C" fn mmtk_is_live_object(object: ObjectReference) -> bool {
    memory_manager::is_live_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_get_forwarded_object(object: ObjectReference) -> NullableObjectReference {
    object.get_forwarded_object().into()
}

#[no_mangle]
pub extern "C" fn mmtk_is_mmtk_object(addr: Address) -> bool {
    debug_assert!(!addr.is_zero());
    debug_assert!(addr.is_aligned_to(mmtk::util::is_mmtk_object::VO_BIT_REGION_SIZE));
    memory_manager::is_mmtk_object(addr).is_some()
}

#[no_mangle]
pub extern "C" fn mmtk_handle_user_collection_request(
    tls: VMMutatorThread,
    force: bool,
    exhaustive: bool,
) {
    crate::mmtk().handle_user_collection_request(tls, force, exhaustive);
}

#[no_mangle]
pub extern "C" fn mmtk_harness_begin(tls: VMMutatorThread) {
    memory_manager::harness_begin(mmtk(), tls)
}

#[no_mangle]
pub extern "C" fn mmtk_harness_end(_tls: VMMutatorThread) {
    memory_manager::harness_end(mmtk())
}

#[no_mangle]
pub extern "C" fn mmtk_starting_heap_address() -> Address {
    memory_manager::starting_heap_address()
}

#[no_mangle]
pub extern "C" fn mmtk_last_heap_address() -> Address {
    memory_manager::last_heap_address()
}

#[no_mangle]
pub extern "C" fn mmtk_add_obj_free_candidate(object: ObjectReference) {
    binding().weak_proc.add_obj_free_candidate(object)
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_add_obj_free_candidates(objects: *const ObjectReference, len: usize) {
    let objects_slice = unsafe { std::slice::from_raw_parts(objects, len) };
    binding().weak_proc.add_obj_free_candidates(objects_slice)
}

#[no_mangle]
pub extern "C" fn mmtk_get_all_obj_free_candidates() -> RawVecOfObjRef {
    let vec = binding().weak_proc.get_all_obj_free_candidates();
    RawVecOfObjRef::from_vec(vec)
}

#[no_mangle]
pub extern "C" fn mmtk_free_raw_vec_of_obj_ref(raw_vec: RawVecOfObjRef) {
    unsafe { raw_vec.into_vec() };
}

#[no_mangle]
pub extern "C" fn mmtk_register_ppp(object: ObjectReference) {
    crate::binding().ppp_registry.register(object)
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_register_ppps(objects: *const ObjectReference, len: usize) {
    let objects_slice = unsafe { std::slice::from_raw_parts(objects, len) };
    crate::binding().ppp_registry.register_many(objects_slice)
}

#[no_mangle]
pub extern "C" fn mmtk_get_backwarded_object(object: ObjectReference) -> ObjectReference {
    let backwarding_table = crate::binding().backwarding_table.lock().unwrap();
    backwarding_table.get(&object).copied().unwrap_or(object)
}

#[no_mangle]
pub extern "C" fn mmtk_get_vo_bit_log_region_size() -> usize {
    // TODO: Fix mmtk-core to make the log region size public
    mmtk::util::is_mmtk_object::VO_BIT_REGION_SIZE.trailing_zeros() as usize
}

#[no_mangle]
pub extern "C" fn mmtk_get_vo_bit_base() -> usize {
    mmtk::util::metadata::side_metadata::VO_BIT_SIDE_METADATA_ADDR.as_usize()
}

#[no_mangle]
pub extern "C" fn mmtk_gc_poll(tls: VMMutatorThread) {
    mmtk::memory_manager::gc_poll(mmtk(), tls)
}

#[no_mangle]
pub extern "C" fn mmtk_get_immix_bump_ptr_offset() -> usize {
    let AllocatorInfo::BumpPointer {
        bump_pointer_offset,
    } = AllocatorInfo::new::<Ruby>(AllocatorSelector::Immix(0))
    else {
        panic!("Expected BumpPointer");
    };
    bump_pointer_offset
}

#[no_mangle]
pub extern "C" fn mmtk_pin_object(object: ObjectReference) -> bool {
    mmtk::memory_manager::pin_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_unpin_object(object: ObjectReference) -> bool {
    mmtk::memory_manager::unpin_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_is_pinned(object: ObjectReference) -> bool {
    mmtk::memory_manager::is_pinned(object)
}

#[no_mangle]
pub extern "C" fn mmtk_register_wb_unprotected_object(object: ObjectReference) {
    crate::binding().register_wb_unprotected_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_is_object_wb_unprotected(object: ObjectReference) -> bool {
    crate::binding().is_object_wb_unprotected(object)
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_object_reference_write_post(
    mutator: *mut RubyMutator,
    object: ObjectReference,
) {
    let ignored_slot = RubySlot::from_address(Address::ZERO);
    let ignored_target = ObjectReference::from_raw_address(Address::ZERO);
    mmtk::memory_manager::object_reference_write_post(
        unsafe { &mut *mutator },
        object,
        ignored_slot,
        ignored_target,
    )
}

/// Enumerate objects.  This function will call `callback(object, data)` for each object. It has
/// undefined behavior if allocation or GC happens while this function is running.
#[no_mangle]
pub extern "C" fn mmtk_enumerate_objects(
    callback: extern "C" fn(ObjectReference, *mut libc::c_void),
    data: *mut libc::c_void,
) {
    crate::mmtk().enumerate_objects(|object| {
        callback(object, data);
    })
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_hidden_header_is_sane(hidden_header: *const HiddenHeader) -> bool {
    let hidden_header = unsafe { &*hidden_header };
    hidden_header.is_sane()
}

#[no_mangle]
pub extern "C" fn mmtk_current_gc_may_move_object() -> bool {
    crate::mmtk().get_plan().current_gc_may_move_object()
}

#[no_mangle]
pub extern "C" fn mmtk_current_gc_is_nursery() -> bool {
    crate::mmtk()
        .get_plan()
        .generational()
        .is_some_and(|gen| gen.is_current_gc_nursery())
}

#[no_mangle]
pub extern "C" fn mmtk_discover_weak_field(field: *mut VALUE) {
    crate::binding().weak_proc.discover_weak_field(field)
}
