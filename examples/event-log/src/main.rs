// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate string_cache;

use string_cache::Atom;
use string_cache::event;

use std::io;

fn main() {
    println!("Reading stdin to end of file");
    let stdin = io::stdin().read_to_string().unwrap();
    let mut atoms = vec![];
    for word in stdin.as_slice().split(|c: char| c.is_whitespace()) {
        atoms.push(Atom::from_slice(word));
    }

    let log = event::LOG.lock();

    println!("Created {:u} atoms, logged {:u} events:", atoms.len(), log.len());
    for e in log.iter() {
        println!("{}", e);
    }
}
