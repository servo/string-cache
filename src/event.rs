// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::sync::Mutex;
use rustc_serialize::{Encoder, Encodable};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub enum Event {
    Intern(u64),
    Insert(u64, String),
    Remove(u64),
}

lazy_static! {
    pub static ref LOG: Mutex<Vec<Event>>
        = Mutex::new(Vec::with_capacity(50_000));
}

pub fn log(e: Event) {
    LOG.lock().unwrap().push(e);
}

macro_rules! log (($e:expr) => (::event::log($e)));

// Serialize by converting to this private struct,
// which produces more convenient output.

#[derive(RustcEncodable)]
struct SerializeEvent<'a> {
    event: &'static str,
    id: u64,
    string: Option<&'a String>,
}

impl Encodable for Event {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        let (event, id, string) = match *self {
            Event::Intern(id) => ("intern", id, None),
            Event::Insert(id, ref s) => ("insert", id, Some(s)),
            Event::Remove(id) => ("remove", id, None),
        };

        SerializeEvent {
            event: event,
            id: id,
            string: string
        }.encode(s)
    }
}
