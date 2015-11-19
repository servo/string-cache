// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_upper_case_globals)]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::fmt;
use std::mem;
use std::ops;
use std::ptr;
use std::slice;
use std::str;
use std::cmp::Ordering::{self, Equal};
use std::sync::Mutex;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::SeqCst;

use shared::{STATIC_TAG, INLINE_TAG, DYNAMIC_TAG, TAG_MASK, MAX_INLINE_LEN, STATIC_SHIFT_BITS,
             ENTRY_ALIGNMENT, pack_static, StaticAtomSet};
use self::UnpackedAtom::{Dynamic, Inline, Static};

#[cfg(feature = "log-events")]
use event::Event;

include!(concat!(env!("OUT_DIR"), "/static_atom_set.rs"));

#[cfg(not(feature = "log-events"))]
macro_rules! log (($e:expr) => (()));

const NB_BUCKETS: usize = 1 << 12;  // 4096
const BUCKET_MASK: u64 = (1 << 12) - 1;
struct StringCache {
    buckets: [Option<Box<StringCacheEntry>>; NB_BUCKETS],
}

lazy_static! {
    static ref STRING_CACHE: Mutex<StringCache> = Mutex::new(StringCache::new());
}

struct StringCacheEntry {
    next_in_bucket: Option<Box<StringCacheEntry>>,
    hash: u64,
    ref_count: AtomicIsize,
    string: String,
}

impl StringCacheEntry {
    fn new(next: Option<Box<StringCacheEntry>>, hash: u64, string_to_add: &str)
           -> StringCacheEntry {
        StringCacheEntry {
            next_in_bucket: next,
            hash: hash,
            ref_count: AtomicIsize::new(1),
            string: String::from(string_to_add),
        }
    }
}

impl StringCache {
    fn new() -> StringCache {
        StringCache {
            buckets: unsafe { mem::zeroed() },
        }
    }

    fn add(&mut self, string_to_add: &str, hash: u64) -> *mut StringCacheEntry {
        let bucket_index = (hash & BUCKET_MASK) as usize;
        {
            let mut ptr: Option<&mut Box<StringCacheEntry>> =
                self.buckets[bucket_index].as_mut();

            while let Some(entry) = ptr.take() {
                if entry.hash == hash && entry.string == string_to_add {
                    if entry.ref_count.fetch_add(1, SeqCst) > 0 {
                        return &mut **entry;
                    }
                    // Uh-oh. The pointer's reference count was zero, which means someone may try
                    // to free it. (Naive attempts to defend against this, for example having the
                    // destructor check to see whether the reference count is indeed zero, don't
                    // work due to ABA.) Thus we need to temporarily add a duplicate string to the
                    // list.
                    entry.ref_count.fetch_sub(1, SeqCst);
                    break;
                }
                ptr = entry.next_in_bucket.as_mut();
            }
        }
        debug_assert!(mem::align_of::<StringCacheEntry>() >= ENTRY_ALIGNMENT);
        let mut entry = Box::new(StringCacheEntry::new(
            self.buckets[bucket_index].take(), hash, string_to_add));
        let ptr: *mut StringCacheEntry = &mut *entry;
        self.buckets[bucket_index] = Some(entry);
        log!(Event::Insert(ptr as u64, String::from(string_to_add)));

        ptr
    }

    fn remove(&mut self, key: u64) {
        let ptr = key as *mut StringCacheEntry;
        let bucket_index = {
            let value: &StringCacheEntry = unsafe { &*ptr };
            debug_assert!(value.ref_count.load(SeqCst) == 0);
            (value.hash & BUCKET_MASK) as usize
        };


        let mut current: &mut Option<Box<StringCacheEntry>> = &mut self.buckets[bucket_index];

        loop {
            let entry_ptr: *mut StringCacheEntry = match current.as_mut() {
                Some(entry) => &mut **entry,
                None => break,
            };
            if entry_ptr == ptr {
                mem::drop(mem::replace(current, unsafe { (*entry_ptr).next_in_bucket.take() }));
                break;
            }
            current = unsafe { &mut (*entry_ptr).next_in_bucket };
        }

        log!(Event::Remove(key));
    }
}

// NOTE: Deriving Eq here implies that a given string must always
// be interned the same way.
#[cfg_attr(feature = "unstable", unsafe_no_drop_flag)]  // See tests::atom_drop_is_idempotent
#[cfg_attr(feature = "heap_size", derive(HeapSizeOf))]
#[derive(Eq, Hash, PartialEq)]
pub struct Atom {
    /// This field is public so that the `atom!()` macro can use it.
    /// You should not otherwise access this field.
    pub data: u64,
}

impl Atom {
    #[inline(always)]
    unsafe fn unpack(&self) -> UnpackedAtom {
        UnpackedAtom::from_packed(self.data)
    }
}

impl<'a> From<&'a str> for Atom {
    #[inline]
    fn from(string_to_add: &str) -> Atom {
        let unpacked = match STATIC_ATOM_SET.get_index_or_hash(string_to_add) {
            Ok(id) => Static(id as u32),
            Err(hash) => {
                let len = string_to_add.len();
                if len <= MAX_INLINE_LEN {
                    let mut buf: [u8; 7] = [0; 7];
                    copy_memory(string_to_add.as_bytes(), &mut buf);
                    Inline(len as u8, buf)
                } else {
                    Dynamic(STRING_CACHE.lock().unwrap().add(string_to_add, hash) as *mut ())
                }
            }
        };

        let data = unsafe { unpacked.pack() };
        log!(Event::Intern(data));
        Atom { data: data }
    }
}

impl Clone for Atom {
    #[inline(always)]
    fn clone(&self) -> Atom {
        unsafe {
            match from_packed_dynamic(self.data) {
                Some(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    (*entry).ref_count.fetch_add(1, SeqCst);
                },
                None => (),
            }
        }
        Atom {
            data: self.data
        }
    }
}

impl Drop for Atom {
    #[inline]
    fn drop(&mut self) {
        // Out of line to guide inlining.
        fn drop_slow(this: &mut Atom) {
            STRING_CACHE.lock().unwrap().remove(this.data);
        }

        unsafe {
            match from_packed_dynamic(self.data) {
                Some(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    if (*entry).ref_count.fetch_sub(1, SeqCst) == 1 {
                        drop_slow(self);
                    }
                }
                _ => (),
            }
        }
    }
}


impl ops::Deref for Atom {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        unsafe {
            match self.unpack() {
                Inline(..) => {
                    let buf = inline_orig_bytes(&self.data);
                    str::from_utf8(buf).unwrap()
                },
                Static(idx) => STATIC_ATOM_SET.index(idx).expect("bad static atom"),
                Dynamic(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    &(*entry).string
                }
            }
        }
    }
}

impl fmt::Display for Atom {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <str as fmt::Display>::fmt(self, f)
    }
}

impl fmt::Debug for Atom {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ty_str = unsafe {
            match self.unpack() {
                Dynamic(..) => "dynamic",
                Inline(..) => "inline",
                Static(..) => "static",
            }
        };

        write!(f, "Atom('{}' type={})", &*self, ty_str)
    }
}

impl PartialOrd for Atom {
    #[inline]
    fn partial_cmp(&self, other: &Atom) -> Option<Ordering> {
        if self.data == other.data {
            return Some(Equal);
        }
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl Ord for Atom {
    #[inline]
    fn cmp(&self, other: &Atom) -> Ordering {
        if self.data == other.data {
            return Equal;
        }
        self.as_ref().cmp(other.as_ref())
    }
}

impl AsRef<str> for Atom {
    fn as_ref(&self) -> &str {
        &self
    }
}

impl Serialize for Atom {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(),S::Error> where S: Serializer {
        let string: &str = self.as_ref();
        string.serialize(serializer)
    }
}

impl Deserialize for Atom {
    fn deserialize<D>(deserializer: &mut D) -> Result<Atom,D::Error> where D: Deserializer {
        let string: String = try!(Deserialize::deserialize(deserializer));
        Ok(Atom::from(&*string))
    }
}

// Atoms use a compact representation which fits this enum in a single u64.
// Inlining avoids actually constructing the unpacked representation in memory.
#[allow(missing_copy_implementations)]
enum UnpackedAtom {
    /// Pointer to a dynamic table entry.  Must be 16-byte aligned!
    Dynamic(*mut ()),

    /// Length + bytes of string.
    Inline(u8, [u8; 7]),

    /// Index in static interning table.
    Static(u32),
}

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

impl UnpackedAtom {
    #[inline(always)]
    unsafe fn pack(self) -> u64 {
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
    unsafe fn from_packed(data: u64) -> UnpackedAtom {
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
unsafe fn from_packed_dynamic(data: u64) -> Option<*mut ()> {
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
unsafe fn inline_orig_bytes<'a>(data: &'a u64) -> &'a [u8] {
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
fn copy_memory(src: &[u8], dst: &mut [u8]) {
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

#[cfg(all(test, feature = "unstable"))]
mod bench;

#[cfg(test)]
mod tests {
    use std::mem;
    use std::thread;
    use super::{Atom, StringCacheEntry, STATIC_ATOM_SET};
    use super::UnpackedAtom::{Dynamic, Inline, Static};
    use shared::ENTRY_ALIGNMENT;

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

    macro_rules! unpacks_to (($e:expr, $t:pat) => (
        match unsafe { Atom::from($e).unpack() } {
            $t => (),
            _ => panic!("atom has wrong type"),
        }
    ));

    #[test]
    fn test_types() {
        unpacks_to!("", Static(..));
        unpacks_to!("id", Static(..));
        unpacks_to!("body", Static(..));
        unpacks_to!("c", Inline(..)); // "z" is a static atom
        unpacks_to!("zz", Inline(..));
        unpacks_to!("zzz", Inline(..));
        unpacks_to!("zzzz", Inline(..));
        unpacks_to!("zzzzz", Inline(..));
        unpacks_to!("zzzzzz", Inline(..));
        unpacks_to!("zzzzzzz", Inline(..));
        unpacks_to!("zzzzzzzz", Dynamic(..));
        unpacks_to!("zzzzzzzzzzzzz", Dynamic(..));
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
            assert_eq_fmt!("0x{:016X}", Atom::from(s).data, data);
        }

        fn check_static(s: &str, x: Atom) {
            assert_eq_fmt!("0x{:016X}", x.data, Atom::from(s).data);
            assert_eq!(0x2, x.data & 0xFFFF_FFFF);
            // The index is unspecified by phf.
            assert!((x.data >> 32) <= STATIC_ATOM_SET.iter().len() as u64);
        }

        // This test is here to make sure we don't change atom representation
        // by accident.  It may need adjusting if there are changes to the
        // static atom table, the tag values, etc.

        // Static atoms
        check_static("a",       atom!("a"));
        check_static("address", atom!("address"));
        check_static("area",    atom!("area"));

        // Inline atoms
        check("e",       0x0000_0000_0000_6511);
        check("xyzzy",   0x0000_797A_7A79_7851);
        check("xyzzy01", 0x3130_797A_7A79_7871);

        // Dynamic atoms. This is a pointer so we can't verify every bit.
        assert_eq!(0x00, Atom::from("a dynamic string").data & 0xf);
    }

    #[test]
    fn assert_sizes() {
        // Guard against accidental changes to the sizes of things.
        use std::mem;
        assert_eq!(if cfg!(feature = "unstable") { 8 } else { 16 }, mem::size_of::<super::Atom>());
        assert_eq!(48, mem::size_of::<super::StringCacheEntry>());
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
        assert_eq!(atom!("body"), Atom::from("body"));
        assert_eq!(atom!("font-weight"), Atom::from("font-weight"));
    }

    #[test]
    fn match_atom() {
        assert_eq!(2, match Atom::from("head") {
            atom!("br") => 1,
            atom!("html") | atom!("head") => 2,
            _ => 3,
        });

        assert_eq!(3, match Atom::from("body") {
            atom!("br") => 1,
            atom!("html") | atom!("head") => 2,
            _ => 3,
        });

        assert_eq!(3, match Atom::from("zzzzzz") {
            atom!("br") => 1,
            atom!("html") | atom!("head") => 2,
            _ => 3,
        });
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

    /// Atom uses #[unsafe_no_drop_flag] to stay small, so drop() may be called more than once.
    /// In calls after the first one, the atom will be filled with a POST_DROP value.
    /// drop() must be a no-op in this case.
    #[cfg(feature = "unstable")]
    #[test]
    fn atom_drop_is_idempotent() {
        use super::from_packed_dynamic;
        unsafe {
            assert_eq!(from_packed_dynamic(mem::POST_DROP_U64), None);
        }
    }

    #[test]
    fn string_cache_entry_alignment_is_sufficient() {
        assert!(mem::align_of::<StringCacheEntry>() >= ENTRY_ALIGNMENT);
    }
}
