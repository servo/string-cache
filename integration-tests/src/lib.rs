// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![deny(warnings)]
#![allow(non_upper_case_globals)]

#[cfg(test)]
use std::thread;
#[cfg(test)]
use string_cache::StaticAtomSet;

include!(concat!(env!("OUT_DIR"), "/test_atom.rs"));
include!(concat!(env!("OUT_DIR"), "/test_atom2.rs"));
pub type Atom = TestAtom;

#[test]
fn test_as_str() {
    let s0 = Atom::from("");
    assert!(s0.as_str() == "");

    let s1 = Atom::from("class");
    assert!(s1.as_str() == "class");

    let i0 = Atom::from("blah");
    assert!(i0.as_str() == "blah");

    let s0 = Atom::from("BLAH");
    assert!(s0.as_str() == "BLAH");

    let d0 = Atom::from("zzzzzzzzzz");
    assert!(d0.as_str() == "zzzzzzzzzz");

    let d1 = Atom::from("ZZZZZZZZZZ");
    assert!(d1.as_str() == "ZZZZZZZZZZ");
}

#[test]
fn test_as_bytes() {
    let s0 = Atom::from("");
    assert!(s0.as_bytes() == b"");

    let s1 = Atom::from("class");
    assert!(s1.as_bytes() == b"class");

    let i0 = Atom::from("blah");
    assert!(i0.as_bytes() == b"blah");

    let s0 = Atom::from("BLAH");
    assert!(s0.as_bytes() == b"BLAH");

    let d0 = Atom::from("zzzzzzzzzz");
    assert!(d0.as_bytes() == b"zzzzzzzzzz");

    let d1 = Atom::from("ZZZZZZZZZZ");
    assert!(d1.as_bytes() == b"ZZZZZZZZZZ");
}

#[test]
#[expect(clippy::comparison_to_empty)]
fn test_as_ref_str() {
    let s0 = Atom::from("");
    assert!(AsRef::<str>::as_ref(&s0) == "");

    let s1 = Atom::from("class");
    assert!(AsRef::<str>::as_ref(&s1) == "class");

    let i0 = Atom::from("blah");
    assert!(AsRef::<str>::as_ref(&i0) == "blah");

    let s0 = Atom::from("BLAH");
    assert!(AsRef::<str>::as_ref(&s0) == "BLAH");

    let d0 = Atom::from("zzzzzzzzzz");
    assert!(AsRef::<str>::as_ref(&d0) == "zzzzzzzzzz");

    let d1 = Atom::from("ZZZZZZZZZZ");
    assert!(AsRef::<str>::as_ref(&d1) == "ZZZZZZZZZZ");
}

#[test]
fn test_as_ref_bytes() {
    let s0 = Atom::from("");
    assert!(AsRef::<[u8]>::as_ref(&s0) == b"");

    let s1 = Atom::from("class");
    assert!(AsRef::<[u8]>::as_ref(&s1) == b"class");

    let i0 = Atom::from("blah");
    assert!(AsRef::<[u8]>::as_ref(&i0) == b"blah");

    let s0 = Atom::from("BLAH");
    assert!(AsRef::<[u8]>::as_ref(&s0) == b"BLAH");

    let d0 = Atom::from("zzzzzzzzzz");
    assert!(AsRef::<[u8]>::as_ref(&d0) == b"zzzzzzzzzz");

    let d1 = Atom::from("ZZZZZZZZZZ");
    assert!(AsRef::<[u8]>::as_ref(&d1) == b"ZZZZZZZZZZ");
}

#[test]
fn test_types() {
    assert!(Atom::from("").is_inline());
    assert!(Atom::from("defaults").is_static());
    assert!(Atom::from("font-weight").is_static());
    assert!(Atom::from("id").is_inline());
    assert!(Atom::from("body").is_inline());
    assert!(Atom::from("a").is_inline());
    assert!(Atom::from("address").is_inline());
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
    #[expect(clippy::cmp_owned)]
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

#[cfg(test)]
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
    #[track_caller]
    fn check_inline(s: &str, mut expected: u64) {
        if cfg!(target_endian = "big") {
            expected = expected.to_le().rotate_left(8)
        }
        assert_eq_fmt!("0x{:016X}", Atom::from(s).unsafe_data(), expected);
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
    check_static("defaults", test_atom!("defaults"));
    check_static("font-weight", test_atom!("font-weight"));

    // Inline atoms
    check_inline("a", 0x0000_0000_0000_6111);
    check_inline("address", 0x7373_6572_6464_6171);
    check_inline("area", 0x0000_0061_6572_6141);
    check_inline("e", 0x0000_0000_0000_6511);
    check_inline("xyzzy", 0x0000_797A_7A79_7851);
    check_inline("xyzzy01", 0x3130_797A_7A79_7871);

    // Dynamic atoms. This is a pointer so we can't verify every bit.
    let tag_mask = 0b11;
    let atom = Atom::from("a dynamic string");
    assert_eq!(0x00, atom.unsafe_data() & tag_mask);
}

#[test]
fn test_threads() {
    let threads = (0_u32..100).map(|_| {
        thread::spawn(move || {
            let _ = Atom::from("a dynamic string");
            let _ = Atom::from("another string");
        })
    });
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn atom_macro() {
    assert_eq!(test_atom!("a"), Atom::from("a"));
    assert_eq!(test_atom!("body"), Atom::from("body"));
    assert_eq!(test_atom!("address"), Atom::from("address"));
    assert_eq!(test_atom!("❤"), Atom::from("❤"));
    assert_eq!(test_atom!("❤💯"), Atom::from("❤💯"));
    assert_eq!(test_atom!("font-weight"), Atom::from("font-weight"));
    assert_eq!(test_atom!("❤💯❤💯"), Atom::from("❤💯❤💯"));
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
    assert!(
        Atom::from("The Quick Brown Fox!")
            .eq_ignore_ascii_case(&Atom::from("THE quick BROWN fox!"))
    );
    assert!(Atom::from("Je vais à Paris").eq_ignore_ascii_case(&Atom::from("je VAIS à PARIS")));
    assert!(!Atom::from("").eq_ignore_ascii_case(&Atom::from("az9")));
    assert!(!Atom::from("aZ9").eq_ignore_ascii_case(&Atom::from("")));
    assert!(!Atom::from("aZ9").eq_ignore_ascii_case(&Atom::from("9Za")));
    assert!(
        !Atom::from("The Quick Brown Fox!")
            .eq_ignore_ascii_case(&Atom::from("THE quick BROWN fox!!"))
    );
    assert!(!Atom::from("Je vais à Paris").eq_ignore_ascii_case(&Atom::from("JE vais À paris")));
}

#[expect(clippy::cmp_owned)]
#[test]
fn test_from_string() {
    assert!(Atom::from("camembert".to_owned()) == Atom::from("camembert"));
}

#[test]
fn test_try_static() {
    assert!(Atom::try_static("defaults").is_some());
    assert!(Atom::try_static("head").is_none());
    assert!(Atom::try_static("not in the static table").is_none());
}

#[test]
fn test_with_empty_static_set() {
    assert_eq!(TestAtom2::from("a").as_str(), "a");
    assert_eq!(
        TestAtom2::from("longer-than-inline").as_str(),
        "longer-than-inline"
    );

    // the dummy string used in `string_cache_codegen::AtomType::to_tokens`
    // to make the static set non-empty
    let dummy = " ".repeat(8);
    let atom = TestAtom2::from(dummy);
    assert!(atom.is_static());
    assert_eq!(atom, test_atom2!("        "));
}

#[cfg(test)]
#[path = "common-usage.rs"]
mod common_usage;
