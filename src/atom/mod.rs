// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_upper_case_globals)]

use core::prelude::*;

use phf::OrderedSet;
use xxhash::XXHasher;

use core::fmt;
use core::mem;
use core::ptr;
use core::slice::bytes;
use core::str;
use core::atomic::{AtomicInt, SeqCst};
use alloc::heap;
use alloc::boxed::Box;
use collections::string::String;
use collections::hash::{Hash, Hasher};
use sync::Mutex;

use self::repr::{UnpackedAtom, Static, Inline, Dynamic};

#[cfg(feature = "log-events")]
use event;

#[cfg(not(feature = "log-events"))]
macro_rules! log (($e:expr) => (()))

#[path="../../shared/repr.rs"]
pub mod repr;

// Needed for memory safety of the tagging scheme!
const ENTRY_ALIGNMENT: uint = 16;

// Macro-generated table for static atoms.
static static_atom_set: OrderedSet<&'static str> = static_atom_set!();

struct StringCache {
    hasher: XXHasher,
    buckets: [*mut StringCacheEntry, ..4096],
}

lazy_static! {
    static ref STRING_CACHE: Mutex<StringCache> = Mutex::new(StringCache::new());
}

struct StringCacheEntry {
    next_in_bucket: *mut StringCacheEntry,
    hash: u64,
    ref_count: AtomicInt,
    string: String,
}

impl StringCacheEntry {
    fn new(next: *mut StringCacheEntry, hash: u64, string_to_add: &str) -> StringCacheEntry {
        StringCacheEntry {
            next_in_bucket: next,
            hash: hash,
            ref_count: AtomicInt::new(1),
            string: String::from_str(string_to_add),
        }
    }
}

impl StringCache {
    fn new() -> StringCache {
        StringCache {
            hasher: XXHasher::new(),
            buckets: unsafe { mem::zeroed() },
        }
    }

    fn add(&mut self, string_to_add: &str) -> *mut StringCacheEntry {
        let hash = self.hasher.hash(&string_to_add);
        let bucket_index = (hash & (self.buckets.len()-1) as u64) as uint;
        let mut ptr = self.buckets[bucket_index];

        while ptr != ptr::null_mut() {
            let value = unsafe { &*ptr };
            if value.hash == hash && value.string.as_slice() == string_to_add {
                break;
            }
            ptr = value.next_in_bucket;
        }

        let mut should_add = false;
        if ptr != ptr::null_mut() {
            unsafe {
                if (*ptr).ref_count.fetch_add(1, SeqCst) == 0 {
                    // Uh-oh. The pointer's reference count was zero, which means someone may try
                    // to free it. (Naive attempts to defend against this, for example having the
                    // destructor check to see whether the reference count is indeed zero, don't
                    // work due to ABA.) Thus we need to temporarily add a duplicate string to the
                    // list.
                    should_add = true;
                    (*ptr).ref_count.fetch_sub(1, SeqCst);
                }
            }
        } else {
            should_add = true
        }

        if should_add {
            unsafe {
                ptr = heap::allocate(mem::size_of::<StringCacheEntry>(), ENTRY_ALIGNMENT)
                        as *mut StringCacheEntry;
                ptr::write(ptr,
                            StringCacheEntry::new(self.buckets[bucket_index], hash, string_to_add));
            }
            self.buckets[bucket_index] = ptr;
            log!(event::Insert(ptr as u64, String::from_str(string_to_add)));
        }

        debug_assert!(ptr != ptr::null_mut());
        ptr
    }

    fn remove(&mut self, key: u64) {
        let ptr = key as *mut StringCacheEntry;
        let value: &mut StringCacheEntry = unsafe { mem::transmute(ptr) };

        debug_assert!(value.ref_count.load(SeqCst) == 0);

        let bucket_index = (value.hash & (self.buckets.len()-1) as u64) as uint;

        let mut current = self.buckets[bucket_index];
        let mut prev: *mut StringCacheEntry = ptr::null_mut();

        while current != ptr::null_mut() {
            if current == ptr {
                if prev != ptr::null_mut() {
                    unsafe { (*prev).next_in_bucket = (*current).next_in_bucket };
                } else {
                    unsafe { self.buckets[bucket_index] = (*current).next_in_bucket };
                }
                break;
            }
            prev = current;
            unsafe { current = (*current).next_in_bucket };
        }
        debug_assert!(current != ptr::null_mut());

        unsafe {
            ptr::read(ptr as *const StringCacheEntry);
            heap::deallocate(ptr as *mut u8,
                mem::size_of::<StringCacheEntry>(), ENTRY_ALIGNMENT);
        }

        log!(event::Remove(key));
    }
}

// NOTE: Deriving Eq here implies that a given string must always
// be interned the same way.
#[unsafe_no_drop_flag]
#[deriving(Eq, Hash, PartialEq)]
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

    pub fn from_slice(string_to_add: &str) -> Atom {
        let unpacked = match static_atom_set.get_index(string_to_add) {
            Some(id) => Static(id as u32),
            None => {
                let len = string_to_add.len();
                if len <= repr::MAX_INLINE_LEN {
                    let mut buf: [u8, ..7] = [0, ..7];
                    bytes::copy_memory(buf.as_mut_slice(), string_to_add.as_bytes());
                    Inline(len as u8, buf)
                } else {
                    Dynamic(STRING_CACHE.lock().add(string_to_add) as *mut ())
                }
            }
        };

        let data = unsafe { unpacked.pack() };
        log!(event::Intern(data))
        Atom { data: data }
    }

    pub fn as_slice<'t>(&'t self) -> &'t str {
        unsafe {
            match self.unpack() {
                Inline(..) => {
                    let buf = repr::inline_orig_bytes(&self.data);
                    debug_assert!(str::is_utf8(buf));
                    str::raw::from_utf8(buf)
                },
                Static(idx) => *static_atom_set.iter().idx(idx as uint).expect("bad static atom"),
                Dynamic(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    (*entry).string.as_slice()
                }
            }
        }
    }
}

impl Clone for Atom {
    #[inline(always)]
    fn clone(&self) -> Atom {
        unsafe {
            match repr::from_packed_dynamic(self.data) {
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
            STRING_CACHE.lock().remove(this.data);
        }

        unsafe {
            match repr::from_packed_dynamic(self.data) {
                // We use #[unsafe_no_drop_flag] so that Atom will be only 64
                // bits.  That means we need to ignore a NULL pointer here,
                // which represents a value that was moved out.
                Some(entry) if entry.is_not_null() => {
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

impl fmt::Show for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ty_str = unsafe {
            match self.unpack() {
                Dynamic(..) => "dynamic",
                Inline(..) => "inline",
                Static(..) => "static",
            }
        };

        write!(f, "Atom('{}' type={})", self.as_slice(), ty_str)
    }
}

impl PartialOrd for Atom {
    fn partial_cmp(&self, other: &Atom) -> Option<Ordering> {
        if self.data == other.data {
            return Some(Equal);
        }
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl Ord for Atom {
    fn cmp(&self, other: &Atom) -> Ordering {
        if self.data == other.data {
            return Equal;
        }
        self.as_slice().cmp(other.as_slice())
    }
}

#[cfg(test)]
mod bench;

#[cfg(test)]
mod tests {
    use core::prelude::*;

    use std::fmt;
    use std::task::spawn;
    use super::Atom;
    use super::repr::{Static, Inline, Dynamic};

    #[test]
    fn test_as_slice() {
        let s0 = Atom::from_slice("");
        assert!(s0.as_slice() == "");

        let s1 = Atom::from_slice("class");
        assert!(s1.as_slice() == "class");

        let i0 = Atom::from_slice("blah");
        assert!(i0.as_slice() == "blah");

        let s0 = Atom::from_slice("BLAH");
        assert!(s0.as_slice() == "BLAH");

        let d0 = Atom::from_slice("zzzzzzzzzz");
        assert!(d0.as_slice() == "zzzzzzzzzz");

        let d1 = Atom::from_slice("ZZZZZZZZZZ");
        assert!(d1.as_slice() == "ZZZZZZZZZZ");
    }

    macro_rules! unpacks_to (($e:expr, $t:pat) => (
        match unsafe { Atom::from_slice($e).unpack() } {
            $t => (),
            _ => panic!("atom has wrong type"),
        }
    ))

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
        let s0 = Atom::from_slice("fn");
        let s1 = Atom::from_slice("fn");
        let s2 = Atom::from_slice("loop");

        let i0 = Atom::from_slice("blah");
        let i1 = Atom::from_slice("blah");
        let i2 = Atom::from_slice("blah2");

        let d0 = Atom::from_slice("zzzzzzzz");
        let d1 = Atom::from_slice("zzzzzzzz");
        let d2 = Atom::from_slice("zzzzzzzzz");

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
            assert_eq!(x < y, Atom::from_slice(x) < Atom::from_slice(y));
            assert_eq!(x.cmp(y), Atom::from_slice(x).cmp(&Atom::from_slice(y)));
            assert_eq!(x.partial_cmp(y), Atom::from_slice(x).partial_cmp(&Atom::from_slice(y)));
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
        let s0 = Atom::from_slice("fn");
        let s1 = s0.clone();
        let s2 = Atom::from_slice("loop");

        let i0 = Atom::from_slice("blah");
        let i1 = i0.clone();
        let i2 = Atom::from_slice("blah2");

        let d0 = Atom::from_slice("zzzzzzzz");
        let d1 = d0.clone();
        let d2 = Atom::from_slice("zzzzzzzzz");

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
                format_args!(fmt::format, $fmt, x).as_slice(),
                format_args!(fmt::format, $fmt, y).as_slice());
        }
    }))

    #[test]
    fn repr() {
        fn check(s: &str, data: u64) {
            assert_eq_fmt!("0x{:016X}", Atom::from_slice(s).data, data);
        }

        fn check_static(s: &str, x: Atom, data: u64) {
            check(s, data);
            assert_eq_fmt!("0x{:016X}", x.data, data);
        }

        // This test is here to make sure we don't change atom representation
        // by accident.  It may need adjusting if there are changes to the
        // static atom table, the tag values, etc.

        // Static atoms
        check_static("a",       atom!(a),       0x0000_0000_0000_0002);
        check_static("address", atom!(address), 0x0000_0001_0000_0002);
        check_static("area",    atom!(area),    0x0000_0003_0000_0002);

        // Inline atoms
        check("e",       0x0000_0000_0000_6511);
        check("xyzzy",   0x0000_797A_7A79_7851);
        check("xyzzy01", 0x3130_797A_7A79_7871);

        // Dynamic atoms. This is a pointer so we can't verify every bit.
        assert_eq!(0x00, Atom::from_slice("a dynamic string").data & 0xf);
    }

    #[test]
    fn assert_sizes() {
        // Guard against accidental changes to the sizes of things.
        use core::mem;
        assert_eq!(8, mem::size_of::<super::Atom>());
        assert_eq!(48, mem::size_of::<super::StringCacheEntry>());
    }

    #[test]
    fn test_threads() {
        for _ in range(0u32, 100u32) {
            spawn(proc() {
                let _ = Atom::from_slice("a dynamic string");
                let _ = Atom::from_slice("another string");
            });
        }
    }

    #[test]
    fn atom_macro() {
        assert_eq!(atom!(body), Atom::from_slice("body"));
        assert_eq!(atom!("body"), Atom::from_slice("body"));
        assert_eq!(atom!("font-weight"), Atom::from_slice("font-weight"));
    }

    #[test]
    fn match_atom() {
        assert_eq!(2u, match Atom::from_slice("head") {
            atom!(br) => 1u,
            atom!(html) | atom!(head) => 2u,
            _ => 3u,
        });

        assert_eq!(3u, match Atom::from_slice("body") {
            atom!(br) => 1u,
            atom!(html) | atom!(head) => 2u,
            _ => 3u,
        });

        assert_eq!(3u, match Atom::from_slice("zzzzzz") {
            atom!(br) => 1u,
            atom!(html) | atom!(head) => 2u,
            _ => 3u,
        });
    }
}
