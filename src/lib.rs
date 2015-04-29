// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name = "string_cache"]
#![crate_type = "rlib"]

#![feature(plugin, unsafe_no_drop_flag, static_assert)]
#![feature(core, collections, alloc, hash)]
#![deny(warnings)]
#![cfg_attr(test, feature(test))]
#![cfg_attr(bench, feature(rand))]
#![plugin(phf_macros, string_cache_plugin)]

#[cfg(test)]
extern crate test;

extern crate phf;

#[macro_use]
extern crate lazy_static;

extern crate rand;

#[cfg(feature = "log-events")]
extern crate rustc_serialize;

extern crate string_cache_shared;

pub use atom::Atom;
pub use namespace::{Namespace, QualName};

#[macro_export]
macro_rules! qualname (($ns:tt, $local:tt) => (
    ::string_cache::namespace::QualName {
        ns: ns!($ns),
        local: atom!($local),
    }
));

#[cfg(feature = "log-events")]
#[macro_use]
pub mod event;

pub mod atom;
pub mod namespace;

// A private module so that macro-expanded idents like
// `::string_cache::atom::Atom` will also work in this crate.
//
// `libstd` uses the same trick.
#[doc(hidden)]
mod string_cache {
    pub use atom;
    pub use namespace;
}
