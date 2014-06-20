// Copyright 2013 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_id = "github.com/mozilla-servo/string-cache#string_cache:0.1"]
#![crate_type = "lib"]
#![crate_type = "dylib"]
#![crate_type = "rlib"]

#![feature(phase, macro_rules)]

extern crate sync;
extern crate debug;

#[cfg(test)]
extern crate test;

#[phase(syntax)]
extern crate phf_mac;
extern crate phf;

pub mod atom;
pub mod static_atoms;
