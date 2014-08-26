// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This code is compiled into both the macros crate and the run-time
//! library, in order to guarantee consistency.

#![allow(dead_code)]

pub static STATIC_TAG: u8 = 2;

static STATIC_SHIFT_BITS: uint = 32;

#[inline(always)]
pub fn add_tag(atom_id: u32) -> u64 {
    (atom_id as u64 << STATIC_SHIFT_BITS) | (STATIC_TAG as u64)
}

/// Undefined to call this on a non-static atom!
#[inline(always)]
pub fn remove_tag(atom_data: u64) -> u32 {
    (atom_data >> STATIC_SHIFT_BITS) as u32
}
