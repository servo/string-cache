/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! A list of static atoms that are pre-hashed at compile time.

pub mod atom {
    use phf::PhfOrderedMap;
    use std::from_str::FromStr;

    #[repr(u32)]
    #[deriving(Eq, TotalEq)]
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
