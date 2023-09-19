//! This module re-implements some flag-testing macros in Rust

use super::{
    cruby::{ROBJECT_EMBED, RUBY_FL_EXIVAR},
    cruby_extra::{STR_NOFREE, STR_NO_EMBED, STR_SHARED},
};

pub fn all_set(actual: usize, required: usize) -> bool {
    (actual & required) == required
}

#[allow(unused)]
pub fn any_set(actual: usize, required: usize) -> bool {
    (actual & required) != 0
}

pub fn robject_has_exivar(flags: usize) -> bool {
    all_set(flags, RUBY_FL_EXIVAR as usize)
}

pub fn robject_is_embedded(flags: usize) -> bool {
    all_set(flags, ROBJECT_EMBED as usize)
}

pub fn string_no_free(flags: usize) -> bool {
    all_set(flags, STR_NOFREE)
}

#[allow(unused)]
pub fn string_is_shared(flags: usize) -> bool {
    all_set(flags, STR_NO_EMBED | STR_SHARED)
}
