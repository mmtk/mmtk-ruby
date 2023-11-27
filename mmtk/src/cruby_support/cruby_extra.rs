//! This module contains extra CRuby definitions not present in the `cruby.rs` or
//! `cruby_bindings.inc.rs` files from CRuby.
//!
//! Those constants should match the definitions in CRuby.

use mmtk::util::Address;

use super::cruby::{
    imemo_type, RBasic, RUBY_Qfalse, OBJ_TOO_COMPLEX_SHAPE_ID, RARRAY_EMBED_LEN_MASK,
    RARRAY_EMBED_LEN_SHIFT, RUBY_FL_USER1, RUBY_FL_USER18, RUBY_FL_USER2, RUBY_FL_USHIFT,
    RUBY_IMMEDIATE_MASK, RUBY_OFFSET_RARRAY_AS_ARY, SHAPE_ID_NUM_BITS, SIZEOF_VALUE, VALUE,
};

/// Counterpart of `rb_mmtk_objbuf_t` in C
#[repr(C)]
pub struct IMemoObjBuf {
    pub flags: usize,
    pub capa: usize,
    pub ary: [VALUE; 1],
}

#[repr(C)]
pub struct RObjectEmbedded {
    basic: RBasic,
    ary: [VALUE; 1],
}

pub const SHAPE_FLAG_SHIFT: usize = (SIZEOF_VALUE * 8) - SHAPE_ID_NUM_BITS as usize;
pub const SHAPE_MASK: usize = (1usize << SHAPE_ID_NUM_BITS) - 1;

pub const STR_NO_EMBED: usize = RUBY_FL_USER1 as usize;
pub const STR_SHARED: usize = RUBY_FL_USER2 as usize;
pub const STR_NOFREE: usize = RUBY_FL_USER18 as usize;

pub const IMEMO_MASK: u32 = 0x0f;

#[allow(non_upper_case_globals)]
pub const imemo_mmtk_strbuf: imemo_type = 14;
#[allow(non_upper_case_globals)]
pub const imemo_mmtk_objbuf: imemo_type = 15;

impl VALUE {
    pub fn as_basic(self) -> *mut RBasic {
        let VALUE(cval) = self;

        cval as *mut RBasic
    }

    pub fn basic_klass(self) -> VALUE {
        unsafe { (*self.as_basic()).klass }
    }
}

pub fn my_special_const_p(value: VALUE) -> bool {
    // This follows the implementation in C.
    // `VALUE.special_const_p` is equivalent to this after the ABI changed upstream,
    // but is slightily more complicated.
    let VALUE(cval) = value;
    let is_immediate = cval & RUBY_IMMEDIATE_MASK as usize != 0;
    let is_false = cval == RUBY_Qfalse as usize;

    is_immediate || is_false
}

pub fn robject_shape_id(flags: usize) -> u32 {
    let shape_id_usize = (flags >> SHAPE_FLAG_SHIFT) & SHAPE_MASK;
    debug_assert!(shape_id_usize <= u32::MAX as usize);
    shape_id_usize as u32
}

pub fn shape_id_is_too_complex(shape_id: u32) -> bool {
    shape_id == OBJ_TOO_COMPLEX_SHAPE_ID
}

pub fn robject_ivptr_embedded(value: VALUE) -> Address {
    unsafe { Address::from_mut_ptr(&mut (*value.as_mut_ptr::<RObjectEmbedded>()).ary as _) }
}

pub fn rarray_embed_len(flags: usize) -> usize {
    let masked = flags & RARRAY_EMBED_LEN_MASK as usize;

    masked >> RARRAY_EMBED_LEN_SHIFT
}

pub fn rarray_embed_ary_addr(value: VALUE) -> Address {
    Address::from(value).add(RUBY_OFFSET_RARRAY_AS_ARY as usize)
}

pub fn get_imemo_type(flags: usize) -> imemo_type {
    // Matches the semantics of the `imemo_type()` function in C.
    (flags >> RUBY_FL_USHIFT) as u32 & IMEMO_MASK
}
