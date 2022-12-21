// All functions here are extern function. There is no point for marking them as unsafe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::CStr;

use crate::abi;
use crate::abi::RawVecOfObjRef;
use crate::abi::RubyBindingOptions;
use crate::binding::RubyBinding;
use crate::mmtk;
use crate::Ruby;
use mmtk::memory_manager;
use mmtk::memory_manager::mmtk_init;
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
    Box::into_raw(Box::new(MMTKBuilder::default()))
}

/// Set the GC trigger to dynamically adjust heap size.
#[no_mangle]
pub extern "C" fn mmtk_builder_set_dynamic_heap_size(
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
pub extern "C" fn mmtk_builder_set_fixed_heap_size(builder: *mut MMTKBuilder, heap_size: usize) {
    let builder = unsafe { &mut *builder };
    builder
        .options
        .gc_trigger
        .set(GCTriggerSelector::FixedHeapSize(heap_size));
}

/// Set the plan.  `plan_name` is a case-sensitive C-style ('\0'-terminated) string matching
/// one of the cases of `enum PlanSelector`.
#[no_mangle]
pub extern "C" fn mmtk_builder_set_plan(builder: *mut MMTKBuilder, plan_name: *const libc::c_char) {
    let builder = unsafe { &mut *builder };
    let plan_name_cstr = unsafe { CStr::from_ptr(plan_name) };
    let plan_name_str = plan_name_cstr.to_str().unwrap();
    let plan_selector = plan_name_str.parse::<PlanSelector>().unwrap();
    builder.options.plan.set(plan_selector);
}

/// Build an MMTk instance.
///
/// -   `builder` is the pointer to the `MMTKBuilder` instance created by the
///     `mmtk_builder_default()` function, and the `MMTKBuilder` will be consumed after building
///     the MMTk instance.
/// -   `upcalls` points to the struct that contains upcalls.  It is allocated in C as static.
#[no_mangle]
pub extern "C" fn mmtk_init_binding(
    builder: *mut MMTKBuilder,
    binding_options: *const RubyBindingOptions,
    upcalls: *const abi::RubyUpcalls,
) {
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
pub extern "C" fn mmtk_destroy_mutator(mutator: *mut RubyMutator) {
    let mut boxed_mutator = unsafe { Box::from_raw(mutator) };
    memory_manager::destroy_mutator(boxed_mutator.as_mut())
}

#[no_mangle]
pub extern "C" fn mmtk_alloc(
    mutator: *mut RubyMutator,
    size: usize,
    align: usize,
    offset: isize,
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
pub extern "C" fn mmtk_post_alloc(
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
pub extern "C" fn mmtk_enable_collection() {
    memory_manager::enable_collection(mmtk())
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
pub extern "C" fn mmtk_is_live_object(object: ObjectReference) -> bool {
    memory_manager::is_live_object(object)
}

#[no_mangle]
pub extern "C" fn mmtk_is_mmtk_object(addr: Address) -> bool {
    debug_assert!(!addr.is_zero());
    debug_assert!(addr.is_aligned_to(mmtk::util::is_mmtk_object::ALLOC_BIT_REGION_SIZE));
    memory_manager::is_mmtk_object(addr)
}

#[no_mangle]
pub extern "C" fn mmtk_modify_check(object: ObjectReference) {
    memory_manager::modify_check(mmtk(), object)
}

#[no_mangle]
pub extern "C" fn mmtk_handle_user_collection_request(tls: VMMutatorThread) {
    memory_manager::handle_user_collection_request::<Ruby>(mmtk(), tls);
}

#[no_mangle]
pub extern "C" fn mmtk_add_weak_candidate(reff: ObjectReference) {
    memory_manager::add_weak_candidate(mmtk(), reff)
}

#[no_mangle]
pub extern "C" fn mmtk_add_soft_candidate(reff: ObjectReference) {
    memory_manager::add_soft_candidate(mmtk(), reff)
}

#[no_mangle]
pub extern "C" fn mmtk_add_phantom_candidate(reff: ObjectReference) {
    memory_manager::add_phantom_candidate(mmtk(), reff)
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
pub extern "C" fn mmtk_add_finalizer(reff: ObjectReference) {
    memory_manager::add_finalizer(crate::mmtk(), reff)
}

#[no_mangle]
pub extern "C" fn mmtk_get_finalized_object() -> ObjectReference {
    memory_manager::get_finalized_object(crate::mmtk()).unwrap_or(ObjectReference::NULL)
}

#[no_mangle]
pub extern "C" fn mmtk_get_all_finalizers() -> RawVecOfObjRef {
    memory_manager::get_all_finalizers(crate::mmtk()).into()
}

#[no_mangle]
pub extern "C" fn mmtk_free_raw_vec_of_obj_ref(raw_vec: RawVecOfObjRef) {
    unsafe { raw_vec.into_vec() };
}
