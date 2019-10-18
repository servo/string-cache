// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(test)]
#![deny(warnings)]
#![allow(non_upper_case_globals)]
#![cfg_attr(feature = "unstable", feature(test))]

#[cfg(feature = "unstable")]
extern crate test;

use std::thread;
use string_cache::atom::StaticAtomSet;

include!(concat!(env!("OUT_DIR"), "/test_atom.rs"));
pub type Atom = TestAtom;

#[test]
fn test_as_slice() {
    let s0 = Atom::from("");
    assert!(s0.as_ref() == "");

    let s1 = Atom::from("class");
    assert!(s1.as_ref() == "class");

    let i0 = Atom::from("blah");
    assert!(i0.as_ref() == "blah");

    let s0 = Atom::from("BLAH");
    assert!(s0.as_ref() == "BLAH");

    let d0 = Atom::from("zzzzzzzzzz");
    assert!(d0.as_ref() == "zzzzzzzzzz");

    let d1 = Atom::from("ZZZZZZZZZZ");
    assert!(d1.as_ref() == "ZZZZZZZZZZ");
}

#[test]
fn test_types() {
    assert!(Atom::from("").is_static());
    assert!(Atom::from("id").is_static());
    assert!(Atom::from("body").is_static());
    assert!(Atom::from("a").is_static());
    assert!(Atom::from("c").is_inline());
    assert!(Atom::from("zz").is_inline());
    assert!(Atom::from("zzz").is_inline());
    assert!(Atom::from("zzzz").is_inline());
    assert!(Atom::from("zzzzz").is_inline());
    assert!(Atom::from("zzzzzz").is_inline());
    assert!(Atom::from("zzzzzzz").is_inline());
    assert!(Atom::from("zzzzzzzz").is_dynamic());
    assert!(Atom::from("zzzzzzzzzzzzz").is_dynamic());
}

#[test]
fn test_equality() {
    let s0 = Atom::from("fn");
    let s1 = Atom::from("fn");
    let s2 = Atom::from("loop");

    let i0 = Atom::from("blah");
    let i1 = Atom::from("blah");
    let i2 = Atom::from("blah2");

    let d0 = Atom::from("zzzzzzzz");
    let d1 = Atom::from("zzzzzzzz");
    let d2 = Atom::from("zzzzzzzzz");

    assert!(s0 == s1);
    assert!(s0 != s2);

    assert!(i0 == i1);
    assert!(i0 != i2);

    assert!(d0 == d1);
    assert!(d0 != d2);

    assert!(s0 != i0);
    assert!(s0 != d0);
    assert!(i0 != d0);
}

#[test]
fn default() {
    assert_eq!(TestAtom::default(), test_atom!(""));
    assert_eq!(&*TestAtom::default(), "");
}

#[test]
fn ord() {
    fn check(x: &str, y: &str) {
        assert_eq!(x < y, Atom::from(x) < Atom::from(y));
        assert_eq!(x.cmp(y), Atom::from(x).cmp(&Atom::from(y)));
        assert_eq!(x.partial_cmp(y), Atom::from(x).partial_cmp(&Atom::from(y)));
    }

    check("a", "body");
    check("asdf", "body");
    check("zasdf", "body");
    check("z", "body");

    check("a", "bbbbb");
    check("asdf", "bbbbb");
    check("zasdf", "bbbbb");
    check("z", "bbbbb");
}

#[test]
fn clone() {
    let s0 = Atom::from("fn");
    let s1 = s0.clone();
    let s2 = Atom::from("loop");

    let i0 = Atom::from("blah");
    let i1 = i0.clone();
    let i2 = Atom::from("blah2");

    let d0 = Atom::from("zzzzzzzz");
    let d1 = d0.clone();
    let d2 = Atom::from("zzzzzzzzz");

    assert!(s0 == s1);
    assert!(s0 != s2);

    assert!(i0 == i1);
    assert!(i0 != i2);

    assert!(d0 == d1);
    assert!(d0 != d2);

    assert!(s0 != i0);
    assert!(s0 != d0);
    assert!(i0 != d0);
}

macro_rules! assert_eq_fmt (($fmt:expr, $x:expr, $y:expr) => ({
    let x = $x;
    let y = $y;
    if x != y {
        panic!("assertion failed: {} != {}",
            format_args!($fmt, x),
            format_args!($fmt, y));
    }
}));

#[test]
fn repr() {
    fn check(s: &str, data: u64) {
        assert_eq_fmt!("0x{:016X}", Atom::from(s).unsafe_data(), data);
    }

    fn check_static(s: &str, x: Atom) {
        assert_eq_fmt!("0x{:016X}", x.unsafe_data(), Atom::from(s).unsafe_data());
        assert_eq!(0x2, x.unsafe_data() & 0xFFFF_FFFF);
        // The index is unspecified by phf.
        assert!((x.unsafe_data() >> 32) <= TestAtomStaticSet::get().atoms.len() as u64);
    }

    // This test is here to make sure we don't change atom representation
    // by accident.  It may need adjusting if there are changes to the
    // static atom table, the tag values, etc.

    // Static atoms
    check_static("a", test_atom!("a"));
    check_static("address", test_atom!("address"));
    check_static("area", test_atom!("area"));

    // Inline atoms
    check("e", 0x0000_0000_0000_6511);
    check("xyzzy", 0x0000_797A_7A79_7851);
    check("xyzzy01", 0x3130_797A_7A79_7871);

    // Dynamic atoms. This is a pointer so we can't verify every bit.
    assert_eq!(0x00, Atom::from("a dynamic string").unsafe_data() & 0xf);
}

#[test]
fn test_threads() {
    for _ in 0_u32..100 {
        thread::spawn(move || {
            let _ = Atom::from("a dynamic string");
            let _ = Atom::from("another string");
        });
    }
}

#[test]
fn atom_macro() {
    assert_eq!(test_atom!("body"), Atom::from("body"));
    assert_eq!(test_atom!("font-weight"), Atom::from("font-weight"));
}

#[test]
fn match_atom() {
    assert_eq!(
        2,
        match Atom::from("head") {
            test_atom!("br") => 1,
            test_atom!("html") | test_atom!("head") => 2,
            _ => 3,
        }
    );

    assert_eq!(
        3,
        match Atom::from("body") {
            test_atom!("br") => 1,
            test_atom!("html") | test_atom!("head") => 2,
            _ => 3,
        }
    );

    assert_eq!(
        3,
        match Atom::from("zzzzzz") {
            test_atom!("br") => 1,
            test_atom!("html") | test_atom!("head") => 2,
            _ => 3,
        }
    );
}

#[test]
fn ensure_deref() {
    // Ensure we can Deref to a &str
    let atom = Atom::from("foobar");
    let _: &str = &atom;
}

#[test]
fn ensure_as_ref() {
    // Ensure we can as_ref to a &str
    let atom = Atom::from("foobar");
    let _: &str = atom.as_ref();
}

#[test]
fn test_ascii_lowercase() {
    assert_eq!(Atom::from("").to_ascii_lowercase(), Atom::from(""));
    assert_eq!(Atom::from("aZ9").to_ascii_lowercase(), Atom::from("az9"));
    assert_eq!(
        Atom::from("The Quick Brown Fox!").to_ascii_lowercase(),
        Atom::from("the quick brown fox!")
    );
    assert_eq!(
        Atom::from("JE VAIS À PARIS").to_ascii_lowercase(),
        Atom::from("je vais À paris")
    );
}

#[test]
fn test_ascii_uppercase() {
    assert_eq!(Atom::from("").to_ascii_uppercase(), Atom::from(""));
    assert_eq!(Atom::from("aZ9").to_ascii_uppercase(), Atom::from("AZ9"));
    assert_eq!(
        Atom::from("The Quick Brown Fox!").to_ascii_uppercase(),
        Atom::from("THE QUICK BROWN FOX!")
    );
    assert_eq!(
        Atom::from("Je vais à Paris").to_ascii_uppercase(),
        Atom::from("JE VAIS à PARIS")
    );
}

#[test]
fn test_eq_ignore_ascii_case() {
    assert!(Atom::from("").eq_ignore_ascii_case(&Atom::from("")));
    assert!(Atom::from("aZ9").eq_ignore_ascii_case(&Atom::from("aZ9")));
    assert!(Atom::from("aZ9").eq_ignore_ascii_case(&Atom::from("Az9")));
    assert!(Atom::from("The Quick Brown Fox!")
        .eq_ignore_ascii_case(&Atom::from("THE quick BROWN fox!")));
    assert!(Atom::from("Je vais à Paris").eq_ignore_ascii_case(&Atom::from("je VAIS à PARIS")));
    assert!(!Atom::from("").eq_ignore_ascii_case(&Atom::from("az9")));
    assert!(!Atom::from("aZ9").eq_ignore_ascii_case(&Atom::from("")));
    assert!(!Atom::from("aZ9").eq_ignore_ascii_case(&Atom::from("9Za")));
    assert!(!Atom::from("The Quick Brown Fox!")
        .eq_ignore_ascii_case(&Atom::from("THE quick BROWN fox!!")));
    assert!(!Atom::from("Je vais à Paris").eq_ignore_ascii_case(&Atom::from("JE vais À paris")));
}

#[test]
fn test_from_string() {
    assert!(Atom::from("camembert".to_owned()) == Atom::from("camembert"));
}

#[cfg(all(test, feature = "unstable"))]
#[path = "bench.rs"]
mod bench;
