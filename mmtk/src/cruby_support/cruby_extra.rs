//! This module contains extra CRuby definitions not present in the `cruby.rs` or
//! `cruby_bindings.inc.rs` files from CRuby.
//!
//! Those constants should match the definitions in CRuby.

use mmtk::util::Address;

use super::cruby::{
    imemo_type, RBasic, RUBY_Qfalse, RARRAY_EMBED_LEN_MASK, RARRAY_EMBED_LEN_SHIFT, RUBY_FL_USER1,
    RUBY_FL_USER18, RUBY_FL_USER2, RUBY_FL_USHIFT, RUBY_IMMEDIATE_MASK, RUBY_OFFSET_RARRAY_AS_ARY,
    RUBY_OFFSET_RARRAY_AS_HEAP_LEN, VALUE,
};

/// Counterpart of `rb_mmtk_objbuf_t` in C
#[repr(C)]
pub struct IMemoObjBuf {
    pub flags: usize,
    pub capa: usize,
    pub ary: [VALUE; 1],
}

pub const RUBY_OFFSET_ROBJECT_AS_ARY: i32 = 32; // struct RObject, subfield "as.ary"

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
        let rbasic_ptr = cval as *mut RBasic;
        rbasic_ptr
    }
}

pub fn my_special_const_p(value: VALUE) -> bool {
    // This follows the implementation in C.
    // `VALUE.special_const_p` is equivalent to this after the ABI changed upstream,
    // but is slightily more complicated.
    let VALUE(cval) = value;
    let is_immediate = cval & RUBY_IMMEDIATE_MASK as usize != 0;
    let is_false = cval == RUBY_Qfalse as usize;
    let result = is_immediate || is_false;
    result
}

pub fn robject_embed_ary_addr(value: VALUE) -> Address {
    Address::from(value).add(RUBY_OFFSET_ROBJECT_AS_ARY as usize)
}

pub fn rarray_embed_len(flags: usize) -> usize {
    let masked = flags & RARRAY_EMBED_LEN_MASK as usize;
    let shifted = masked >> RARRAY_EMBED_LEN_SHIFT;
    shifted
}

pub fn rarray_embed_ary_addr(value: VALUE) -> Address {
    Address::from(value).add(RUBY_OFFSET_RARRAY_AS_ARY as usize)
}

pub fn get_imemo_type(flags: usize) -> imemo_type {
    // Matches the semantics of the `imemo_type()` function in C.
    (flags >> RUBY_FL_USHIFT) as u32 & IMEMO_MASK
}
