// All functions here are extern function. There is no point for marking them as unsafe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::CStr;

use crate::abi;
use crate::binding::RubyBinding;
use crate::mmtk;
use crate::Ruby;
use mmtk::memory_manager;
use mmtk::memory_manager::mmtk_init;
use mmtk::scheduler::{GCController, GCWorker};
use mmtk::util::constants::MIN_OBJECT_SIZE;
use mmtk::util::options::PlanSelector;
use mmtk::util::{Address, ObjectReference};
use mmtk::util::{VMMutatorThread, VMThread, VMWorkerThread};
use mmtk::AllocationSemantics;
use mmtk::MMTKBuilder;
use mmtk::Mutator;

/// Create an MMTKBuilder instance with default options.
/// This instance shall be consumed by `mmtk_init_binding`.
#[no_mangle]
pub extern "C" fn mmtk_builder_default() -> *mut MMTKBuilder {
    Box::into_raw(Box::new(MMTKBuilder::default()))
}

/// Set the `heap_size` option.
#[no_mangle]
pub extern "C" fn mmtk_builder_set_heap_size(builder: *mut MMTKBuilder, heap_size: usize) {
    let builder = unsafe { &mut *builder };
    builder.options.heap_size.set(heap_size);
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
/// -   `builder` is the pointer to the `MMTKBuilder` instance cretaed by the
///     `mmtk_builder_default()` function, and the `MMTKBuilder` will be consumed after building
///     the MMTk instance.
/// -   `upcalls` points to the struct that contains upcalls.  It is allocated in C as static.
#[no_mangle]
pub extern "C" fn mmtk_init_binding(builder: *mut MMTKBuilder, upcalls: *const abi::RubyUpcalls) {
    let builder = unsafe { Box::from_raw(builder) };
    let mmtk_boxed = mmtk_init(&builder);
    let mmtk_static = Box::leak(Box::new(mmtk_boxed));

    let binding = RubyBinding::new(mmtk_static, upcalls);

    crate::BINDING
        .set(binding)
        .unwrap_or_else(|_| panic!("Binding is already initialized"));
}

#[no_mangle]
pub extern "C" fn mmtk_bind_mutator(tls: VMMutatorThread) -> *mut Mutator<Ruby> {
    Box::into_raw(memory_manager::bind_mutator(mmtk(), tls))
}

#[no_mangle]
pub extern "C" fn mmtk_destroy_mutator(mutator: *mut Mutator<Ruby>) {
    memory_manager::destroy_mutator(unsafe { Box::from_raw(mutator) })
}

#[no_mangle]
pub extern "C" fn mmtk_alloc(
    mutator: *mut Mutator<Ruby>,
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
    mutator: *mut Mutator<Ruby>,
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
pub unsafe extern "C" fn mmtk_start_control_collector(
    tls: VMWorkerThread,
    controller: *mut GCController<Ruby>,
) {
    let mut controller = Box::from_raw(controller);
    memory_manager::start_control_collector(mmtk(), tls, &mut controller);
}

#[no_mangle]
pub unsafe extern "C" fn mmtk_start_worker(tls: VMWorkerThread, worker: *mut GCWorker<Ruby>) {
    let mut worker = Box::from_raw(worker);
    memory_manager::start_worker::<Ruby>(mmtk(), tls, &mut worker)
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
pub extern "C" fn mmtk_register_finalizable(reff: ObjectReference) {
    memory_manager::add_finalizer(crate::mmtk(), reff)
}

#[no_mangle]
pub extern "C" fn mmtk_poll_finalizable(_include_live: bool) -> ObjectReference {
    memory_manager::get_finalized_object(crate::mmtk()).unwrap_or(ObjectReference::NULL)
}
