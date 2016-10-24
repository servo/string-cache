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

#![cfg_attr(test, deny(warnings))]
#![cfg_attr(all(test, feature = "unstable"), feature(test))]

#[cfg(all(test, feature = "unstable"))] extern crate test;
#[cfg(feature = "log-events")] extern crate rustc_serialize;
#[cfg(feature = "heapsize")] #[macro_use] extern crate heapsize;
#[cfg(test)] extern crate rand;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate debug_unreachable;
extern crate serde;
extern crate phf_shared;

pub use atom::Atom;
pub use namespace::{Namespace, QualName};

#[macro_export]
macro_rules! qualname {
    ("", $local:tt) => {
        $crate::namespace::QualName {
            ns: ns!(),
            local: atom!($local),
        }
    };
    ($ns:tt, $local:tt) => {
        $crate::namespace::QualName {
            ns: ns!($ns),
            local: atom!($local),
        }
    }
}

#[macro_export]
macro_rules! ns {
    () => { $crate::Namespace(atom!("")) };
    (html) => { $crate::Namespace(atom!("http://www.w3.org/1999/xhtml")) };
    (xml) => { $crate::Namespace(atom!("http://www.w3.org/XML/1998/namespace")) };
    (xmlns) => { $crate::Namespace(atom!("http://www.w3.org/2000/xmlns/")) };
    (xlink) => { $crate::Namespace(atom!("http://www.w3.org/1999/xlink")) };
    (svg) => { $crate::Namespace(atom!("http://www.w3.org/2000/svg")) };
    (mathml) => { $crate::Namespace(atom!("http://www.w3.org/1998/Math/MathML")) };
}

include!(concat!(env!("OUT_DIR"), "/atom_macro.rs"));

#[cfg(feature = "log-events")]
#[macro_use]
pub mod event;

pub mod atom;
pub mod namespace;
pub mod shared;

// A private module so that macro-expanded idents like
// `::string_cache::atom::Atom` will also work in this crate.
//
// `libstd` uses the same trick.
#[doc(hidden)]
mod string_cache {
    pub use atom;
    pub use namespace;
}
