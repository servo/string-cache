// Copyright 2015 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use phf_shared;

// FIXME(rust-lang/rust#18153): generate these from an enum
pub const DYNAMIC_TAG: u8 = 0b_00;
pub const INLINE_TAG: u8 = 0b_01;  // len in upper nybble
pub const STATIC_TAG: u8 = 0b_10;
pub const TAG_MASK: u64 = 0b_11;
pub const ENTRY_ALIGNMENT: usize = 4;  // Multiples have TAG_MASK bits unset, available for tagging.

pub const MAX_INLINE_LEN: usize = 7;

pub const STATIC_SHIFT_BITS: usize = 32;

pub fn pack_static(n: u32) -> u64 {
    (STATIC_TAG as u64) | ((n as u64) << STATIC_SHIFT_BITS)
}

pub struct StaticAtomSet {
    pub key: u64,
    pub disps: &'static [(u32, u32)],
    pub atoms: &'static [&'static str],
}

impl StaticAtomSet {
    #[inline]
    pub fn get_index_or_hash(&self, s: &str) -> Result<u32, u64> {
        let hash = phf_shared::hash(s, self.key);
        let index = phf_shared::get_index(hash, self.disps, self.atoms.len());
        if self.atoms[index as usize] == s {
            Ok(index)
        } else {
            Err(hash)
        }
    }

    #[inline]
    pub fn index(&self, i: u32) -> Option<&'static str> {
        self.atoms.get(i as usize).map(|&s| s)
    }

    #[inline]
    pub fn iter(&self) -> ::std::slice::Iter<&'static str> {
        self.atoms.iter()
    }
}
