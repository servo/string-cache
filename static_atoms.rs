// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A list of static atoms that are pre-hashed at compile time.

pub mod atom {
    use phf::PhfOrderedMap;
    use std::from_str::FromStr;

    #[repr(u32)]
    #[deriving(Eq, PartialEq)]
    pub enum StaticAtom {
        EmptyString,
        Id,
        Class,
        Href,
        Style,
        Span,
        Width,
        Height,
        Type,
        Data,
        New,
        Name,
        Src,
        Rel,
        Div,
    }

    static STATIC_ATOMS: PhfOrderedMap<StaticAtom> = phf_ordered_map!(
        "" => EmptyString,
        "id" => Id,
        "class" => Class,
        "href" => Href,
        "style" => Style,
        "span" => Span,
        "width" => Width,
        "height" => Height,
        "type" => Type,
        "data" => Data,
        "new" => New,
        "name" => Name,
        "src" => Src,
        "rel" => Rel,
        "div" => Div,
    );

    impl FromStr for StaticAtom {
        #[inline]
        fn from_str(string: &str) -> Option<StaticAtom> {
            match STATIC_ATOMS.find(&string) {
                None => None,
                Some(&k) => Some(k)
            }
        }
    }

    impl StaticAtom {
        pub fn as_slice(&self) -> &'static str {
            let (string, _) = STATIC_ATOMS.entries().idx(*self as uint).unwrap();
            string
        }
    }
}
