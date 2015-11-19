// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate csv;
extern crate string_cache;
extern crate rustc_serialize;
extern crate phf_shared;

#[path = "../../../src/shared.rs"]
#[allow(dead_code)]
mod shared;

use string_cache::Atom;

use std::{env, cmp};
use std::collections::hash_map::{HashMap, Entry};
use std::path::Path;

#[derive(RustcDecodable, Debug)]
struct Event {
    event: String,
    id: u64,
    string: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Dynamic,
    Inline,
    Static,
}

impl Kind {
    fn from_tag(tag: u8) -> Kind {
        match tag {
            shared::DYNAMIC_TAG => Kind::Dynamic,
            shared::INLINE_TAG => Kind::Inline,
            shared::STATIC_TAG => Kind::Static,
            _ => panic!()
        }
    }

    fn to_tag(self) -> u8 {
        match self {
            Kind::Dynamic => shared::DYNAMIC_TAG,
            Kind::Inline => shared::INLINE_TAG,
            Kind::Static => shared::STATIC_TAG,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Summary {
    kind: Kind,
    times: usize,
}

fn main() {
    let filename = env::args().skip(1).next()
        .expect("Usage: string-cache-summarize-events foo.csv");
    let path = &Path::new(&filename);
    let mut file = csv::Reader::from_file(path).unwrap();

    // Over the lifetime of a program, one dynamic atom might get interned at
    // several addresses, and one address may be used to intern several
    // different strings.  For this reason we must separately track the
    // currently-allocated atoms and the summary of all atoms ever created.
    let mut dynamic: HashMap<u64, String> = HashMap::new();
    let mut peak_dynamic = 0;
    let mut summary: HashMap<String, Summary> = HashMap::new();
    let mut inserts = 0;

    for record in file.decode() {
        let ev: Event = record.unwrap();
        match &ev.event[..] {
            "intern" => {
                let tag = (ev.id & 0xf) as u8;
                assert!(tag <= shared::STATIC_TAG);

                let string = match tag {
                    shared::DYNAMIC_TAG => dynamic[&ev.id].clone(),

                    // FIXME: We really shouldn't be allowed to do this. It's a memory-safety
                    // hazard; the field is only public for the atom!() macro.
                    _ => Atom { data: ev.id }.to_string(),
                };

                match summary.entry(string) {
                    Entry::Occupied(entry) => entry.into_mut().times += 1,
                    Entry::Vacant(entry) => {
                        entry.insert(Summary {
                            kind: Kind::from_tag(tag),
                            times: 1,
                        });
                    }
                }
            },

            "insert" => {
                assert!(!dynamic.contains_key(&ev.id));
                dynamic.insert(ev.id, ev.string.expect("no string to insert"));
                peak_dynamic = cmp::max(peak_dynamic, dynamic.len());
                inserts += 1;
            }

            "remove" => {
                assert!(dynamic.contains_key(&ev.id));
                dynamic.remove(&ev.id);
            }

            e => panic!("unknown event {}", e),
        }
    }

    // Get all records, in a stable order.
    let mut summary: Vec<_> = summary.into_iter().collect();
    summary.sort_by(|&(ref a, _), &(ref b, _)| a.cmp(b));

    // Sort by number of occurrences, descending.
    summary.sort_by(|&(_, a), &(_, b)| b.times.cmp(&a.times));
    let longest_atom = summary.iter().map(|&(ref k, _)| k.len())
        .max().unwrap_or(0);

    let pad = |c, n| {
        for _ in n..longest_atom {
            print!("{}", c);
        }
    };

    let mut total = 0;
    let mut by_kind = [0, 0, 0];
    for &(_, Summary { kind, times }) in &summary {
        total += times;
        by_kind[kind.to_tag() as usize] += times;
    }

    println!("\n");
    println!("kind       times   pct");
    println!("-------  -------  ----");
    for (k, &n) in by_kind.iter().enumerate() {
        let k: Kind = Kind::from_tag(k as u8);
        print!("{:7?}  {:7}  {:4.1}",
            k, n, 100.0 * (n as f64) / (total as f64));

        match k {
            Kind::Dynamic => println!("    {} inserts, peak size {}, miss rate {:4.1}%",
                inserts, peak_dynamic, 100.0 * (inserts as f64) / (n as f64)),
            _ => println!(""),
        }
    }
    println!("");
    println!("total    {:7}", total);
    println!("\n");

    pad(' ', 4);
    println!("atom   times  kind");
    pad('-', 4);
    println!("----  ------  -------");
    for (string, Summary { kind, times }) in summary {
        pad(' ', string.chars().count());
        println!("{}  {:6}  {:?}", string, times, kind);
    }
}
