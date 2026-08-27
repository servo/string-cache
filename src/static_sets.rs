// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/// A static `PhfStrSet`
///
/// This trait is implemented by static sets of interned strings generated using
/// `string_cache_codegen`, and `EmptyStaticAtomSet` for when strings will be added dynamically.
///
/// It is used by the methods of [`Atom`] to check if a string is present in the static set.
///
/// [`Atom`]: struct.Atom.html
pub trait StaticAtomSet: Ord {
    /// Get the location of the static string set in the binary.
    fn get() -> &'static PhfStrSet;
}

/// A string set created using a [perfect hash function], specifically
/// [Hash, Displace and Compress].
///
/// See the CHD document for the meaning of the struct fields.
///
/// [perfect hash function]: https://en.wikipedia.org/wiki/Perfect_hash_function
/// [Hash, Displace and Compress]: http://cmph.sourceforge.net/papers/esa09.pdf
pub struct PhfStrSet {
    #[doc(hidden)]
    pub seed: u64,
    #[doc(hidden)]
    pub pilots: &'static [u8],
    #[doc(hidden)]
    pub remap: &'static [u32],
    #[doc(hidden)]
    pub atoms: &'static [&'static str],
    #[doc(hidden)]
    pub hashes: &'static [u64],
}

/// An empty static atom set for when only dynamic strings will be added
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct EmptyStaticAtomSet;

impl StaticAtomSet for EmptyStaticAtomSet {
    fn get() -> &'static PhfStrSet {
        // The name is a lie: this set is not empty (it contains the empty string)
        // but that’s only to avoid divisions by zero in rust-phf.
        static SET: PhfStrSet = PhfStrSet {
            seed: 0,
            pilots: &[0],
            remap: &[0],
            atoms: &[""],
            // "" SipHash'd, and xored with u64_hash_to_u32.
            hashes: &[0x3ddddef3],
        };
        &SET
    }
}
