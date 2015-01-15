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

#![allow(dead_code, unused_imports)]

use core::{mem, raw, intrinsics};
use core::option::Option::{self, Some, None};
use core::ptr::PtrExt;
use core::slice::{AsSlice, SliceExt};
use core::slice::bytes;

pub use self::UnpackedAtom::{Dynamic, Inline, Static};

// FIXME(rust-lang/rust#18153): generate these from an enum
pub const DYNAMIC_TAG: u8 = 0u8;
pub const INLINE_TAG: u8 = 1u8;  // len in upper nybble
pub const STATIC_TAG: u8 = 2u8;

pub const MAX_INLINE_LEN: usize = 7;

// Atoms use a compact representation which fits this enum in a single u64.
// Inlining avoids actually constructing the unpacked representation in memory.
pub enum UnpackedAtom {
    /// Pointer to a dynamic table entry.  Must be 16-byte aligned!
    Dynamic(*mut ()),

    /// Length + bytes of string.
    Inline(u8, [u8; 7]),

    /// Index in static interning table.
    Static(u32),
}

const STATIC_SHIFT_BITS: usize = 32;

#[inline(always)]
unsafe fn inline_atom_slice(x: &u64) -> raw::Slice<u8> {
    #[static_assert]
    const IS_LITTLE_ENDIAN: bool = cfg!(target_endian = "little");

    raw::Slice {
        data: ((x as *const u64) as *const u8).offset(1),
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
                debug_assert!(0 == n & 0xf);
                n
            }
            Inline(len, buf) => {
                debug_assert!((len as usize) <= MAX_INLINE_LEN);
                let mut data: u64 = (INLINE_TAG as u64) | ((len as u64) << 4);
                {
                    let dest: &mut [u8] = mem::transmute(inline_atom_slice(&mut data));
                    bytes::copy_memory(dest, buf.as_slice());
                }
                data
            }
        }
    }

    #[inline(always)]
    pub unsafe fn from_packed(data: u64) -> UnpackedAtom {
        #[static_assert]
        const DYNAMIC_IS_UNTAGGED: bool = DYNAMIC_TAG == 0;

        match (data & 0xf) as u8 {
            DYNAMIC_TAG => Dynamic(data as *mut ()),
            STATIC_TAG => Static((data >> STATIC_SHIFT_BITS) as u32),
            INLINE_TAG => {
                let len = ((data & 0xf0) >> 4) as usize;
                debug_assert!(len <= MAX_INLINE_LEN);
                let mut buf: [u8; 7] = [0; 7];
                let src: &[u8] = mem::transmute(inline_atom_slice(&data));
                bytes::copy_memory(buf.as_mut_slice(), src);
                Inline(len as u8, buf)
            },

            // intrinsics::unreachable() in release builds?
            // See rust-lang/rust#18152.
            _ => panic!("impossible"),
        }
    }
}

/// Used for a fast path in Clone and Drop.
#[inline(always)]
pub unsafe fn from_packed_dynamic(data: u64) -> Option<*mut ()> {
    if (DYNAMIC_TAG as u64) == (data & 0xf) {
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
            let src: &[u8] = mem::transmute(inline_atom_slice(data));
            src.slice_to(len as uint)
        }
        _ => intrinsics::unreachable(),
    }
}
