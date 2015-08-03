// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Details of the atom representation that need to be shared between
//! the macros crate and the run-time library, in order to guarantee
//! consistency.

#![cfg_attr(test, deny(warnings))]

#[macro_use] extern crate debug_unreachable;
extern crate phf_shared;

use std::ptr;
use std::slice;

pub use self::UnpackedAtom::{Dynamic, Inline, Static};

include!(concat!(env!("OUT_DIR"), "/static_atom_set.rs"));

// FIXME(rust-lang/rust#18153): generate these from an enum
pub const DYNAMIC_TAG: u8 = 0b_00;
pub const INLINE_TAG: u8 = 0b_01;  // len in upper nybble
pub const STATIC_TAG: u8 = 0b_10;
pub const TAG_MASK: u64 = 0b_11;
pub const ENTRY_ALIGNMENT: usize = 4;  // Multiples have TAG_MASK bits unset, available for tagging.

pub const MAX_INLINE_LEN: usize = 7;

pub struct StaticAtomSet {
    key: u64,
    disps: &'static [(u32, u32)],
    atoms: &'static [&'static str],
}

impl StaticAtomSet {
    #[inline]
    pub fn get_index(&self, s: &str) -> Option<u32> {
        let hash = phf_shared::hash(s, self.key);
        let index = phf_shared::get_index(hash, self.disps, self.atoms.len());
        if self.atoms[index as usize] == s {
            Some(index)
        } else {
            None
        }
    }

    #[inline]
    pub fn index(&self, i: u32) -> Option<&'static str> {
        self.atoms.get(i as usize).map(|&s| s)
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<&'static str> {
        self.atoms.iter()
    }
}

// Atoms use a compact representation which fits this enum in a single u64.
// Inlining avoids actually constructing the unpacked representation in memory.
#[allow(missing_copy_implementations)]
pub enum UnpackedAtom {
    /// Pointer to a dynamic table entry.  Must be 16-byte aligned!
    Dynamic(*mut ()),

    /// Length + bytes of string.
    Inline(u8, [u8; 7]),

    /// Index in static interning table.
    Static(u32),
}

const STATIC_SHIFT_BITS: usize = 32;

pub static ALL_NS: &'static [(&'static str, &'static str)] = &[
    ("", ""),
    ("html", "http://www.w3.org/1999/xhtml"),
    ("xml", "http://www.w3.org/XML/1998/namespace"),
    ("xmlns", "http://www.w3.org/2000/xmlns/"),
    ("xlink", "http://www.w3.org/1999/xlink"),
    ("svg", "http://www.w3.org/2000/svg"),
    ("mathml", "http://www.w3.org/1998/Math/MathML"),
];

struct RawSlice {
    data: *const u8,
    len: usize,
}

#[cfg(target_endian = "little")]  // Not implemented yet for big-endian
#[inline(always)]
unsafe fn inline_atom_slice(x: &u64) -> RawSlice {
    let x: *const u64 = x;
    RawSlice {
        data: (x as *const u8).offset(1),
        len: 7,
    }
}

pub fn pack_static(n: u32) -> u64 {
    (STATIC_TAG as u64) | ((n as u64) << STATIC_SHIFT_BITS)
}

impl UnpackedAtom {
    #[inline(always)]
    pub unsafe fn pack(self) -> u64 {
        match self {
            Static(n) => pack_static(n),
            Dynamic(p) => {
                let n = p as u64;
                debug_assert!(0 == n & TAG_MASK);
                n
            }
            Inline(len, buf) => {
                debug_assert!((len as usize) <= MAX_INLINE_LEN);
                let mut data: u64 = (INLINE_TAG as u64) | ((len as u64) << 4);
                {
                    let raw_slice = inline_atom_slice(&mut data);
                    let dest: &mut [u8] = slice::from_raw_parts_mut(
                        raw_slice.data as *mut u8, raw_slice.len);
                    copy_memory(&buf[..], dest);
                }
                data
            }
        }
    }

    #[inline(always)]
    pub unsafe fn from_packed(data: u64) -> UnpackedAtom {
        debug_assert!(DYNAMIC_TAG == 0); // Dynamic is untagged

        match (data & TAG_MASK) as u8 {
            DYNAMIC_TAG => Dynamic(data as *mut ()),
            STATIC_TAG => Static((data >> STATIC_SHIFT_BITS) as u32),
            INLINE_TAG => {
                let len = ((data & 0xf0) >> 4) as usize;
                debug_assert!(len <= MAX_INLINE_LEN);
                let mut buf: [u8; 7] = [0; 7];
                let raw_slice = inline_atom_slice(&data);
                let src: &[u8] = slice::from_raw_parts(raw_slice.data, raw_slice.len);
                copy_memory(src, &mut buf[..]);
                Inline(len as u8, buf)
            },
            _ => debug_unreachable!(),
        }
    }
}

/// Used for a fast path in Clone and Drop.
#[inline(always)]
pub unsafe fn from_packed_dynamic(data: u64) -> Option<*mut ()> {
    if (DYNAMIC_TAG as u64) == (data & TAG_MASK) {
        Some(data as *mut ())
    } else {
        None
    }
}

/// For as_slice on inline atoms, we need a pointer into the original
/// string contents.
///
/// It's undefined behavior to call this on a non-inline atom!!
#[inline(always)]
pub unsafe fn inline_orig_bytes<'a>(data: &'a u64) -> &'a [u8] {
    match UnpackedAtom::from_packed(*data) {
        Inline(len, _) => {
            let raw_slice = inline_atom_slice(&data);
            let src: &[u8] = slice::from_raw_parts(raw_slice.data, raw_slice.len);
            &src[..(len as usize)]
        }
        _ => debug_unreachable!(),
    }
}


/// Copy of std::slice::bytes::copy_memory, which is unstable.
#[inline]
pub fn copy_memory(src: &[u8], dst: &mut [u8]) {
    let len_src = src.len();
    assert!(dst.len() >= len_src);
    // `dst` is unaliasable, so we know statically it doesn't overlap
    // with `src`.
    unsafe {
        ptr::copy_nonoverlapping(src.as_ptr(),
                                 dst.as_mut_ptr(),
                                 len_src);
    }
}
