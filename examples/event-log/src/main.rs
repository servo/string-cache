// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate string_cache;

use string_cache::DefaultAtom as Atom;
use string_cache::event;

use std::io;
use std::io::prelude::*;

fn main() {
    println!("Reading stdin to end of file");
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin).unwrap();
    let mut atoms = vec![];
    for word in stdin.split(|c: char| c.is_whitespace()) {
        atoms.push(Atom::from(word));
    }

    let log = event::LOG.lock().unwrap();

    println!("Created {} atoms, logged {} events:", atoms.len(), log.len());
    for e in log.iter() {
        println!("{:?}", e);
    }
}
