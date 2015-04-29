// Copyright 2014 The Servo Project Developers. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name="string_cache_plugin"]
#![crate_type="dylib"]

#![feature(plugin_registrar, quote, box_syntax, static_assert)]
#![feature(rustc_private, slice_patterns)]
#![deny(warnings)]
#![allow(unused_imports)]  // for quotes

extern crate syntax;
extern crate rustc;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate mac;

extern crate string_cache_shared;

use rustc::plugin::Registry;

mod atom;

// NB: This needs to be public or we get a linker error.
#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("static_atom_set", atom::expand_static_atom_set);
    reg.register_macro("atom", atom::expand_atom);
    reg.register_macro("ns", atom::expand_ns);
}
