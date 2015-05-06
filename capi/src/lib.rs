// Copyright 2015 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(core)]

extern crate libc;
extern crate string_cache;

use string_cache::Atom;

use std::{mem, raw, ptr};
use libc::{c_char, size_t, strlen};

#[no_mangle] pub unsafe extern "C"
fn scache_atom_data(x: *const Atom) -> *const c_char {
    let s: &str = &**x;
    s.as_ptr() as *const c_char
}

#[no_mangle] pub unsafe extern "C"
fn scache_atom_len(x: *const Atom) -> size_t {
    (*x).len() as size_t
}

#[no_mangle] pub unsafe extern "C"
fn scache_atom_clone(x: *const Atom) -> Atom {
    (*x).clone()
}

#[no_mangle] pub unsafe extern "C"
fn scache_atom_destroy(x: *mut Atom) {
    let _ = ptr::read(x);
}

#[no_mangle] pub unsafe extern "C"
fn scache_atom_from_buffer(buf: *const c_char, len: size_t) -> Atom {
    let s: &str = mem::transmute(raw::Slice {
        data: buf,
        len: len as usize,
    });
    Atom::from_slice(s)
}

#[no_mangle] pub unsafe extern "C"
fn scache_atom_from_c_str(buf: *const c_char) -> Atom {
    scache_atom_from_buffer(buf, strlen(buf))
}
