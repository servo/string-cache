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

use std::cmp::max;
use std::fmt;
use std::mem;
use std::ops;
use std::ptr;
use std::slice::bytes;
use std::str;
use std::rt::heap;
use std::cmp::Ordering::{self, Equal};
use std::hash::{self, Hash, SipHasher};
use std::sync::Mutex;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::SeqCst;

use string_cache_shared::{self, UnpackedAtom, Static, Inline, Dynamic, STATIC_ATOM_SET,
                          ENTRY_ALIGNMENT};

#[cfg(feature = "log-events")]
use event::Event;

#[cfg(not(feature = "log-events"))]
macro_rules! log (($e:expr) => (()));


struct StringCache {
    buckets: [*mut StringCacheEntry; 4096],
}

lazy_static! {
    static ref STRING_CACHE: Mutex<StringCache> = Mutex::new(StringCache::new());
}

struct StringCacheEntry {
    next_in_bucket: *mut StringCacheEntry,
    hash: u64,
    ref_count: AtomicIsize,
    string: String,
}

unsafe impl Send for StringCache { }

impl StringCacheEntry {
    fn new(next: *mut StringCacheEntry, hash: u64, string_to_add: &str) -> StringCacheEntry {
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

    fn add(&mut self, string_to_add: &str) -> *mut StringCacheEntry {
        let hash = hash::hash::<_, SipHasher>(&string_to_add);
        let bucket_index = (hash & (self.buckets.len()-1) as u64) as usize;
        let mut ptr = self.buckets[bucket_index];

        while ptr != ptr::null_mut() {
            let value = unsafe { &*ptr };
            if value.hash == hash && value.string == string_to_add {
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
                ptr = heap::allocate(
                    mem::size_of::<StringCacheEntry>(),
                    max(mem::align_of::<StringCacheEntry>(), ENTRY_ALIGNMENT)
                ) as *mut StringCacheEntry;
                ptr::write(ptr,
                            StringCacheEntry::new(self.buckets[bucket_index], hash, string_to_add));
            }
            self.buckets[bucket_index] = ptr;
            log!(Event::Insert(ptr as u64, String::from(string_to_add)));
        }

        debug_assert!(ptr != ptr::null_mut());
        ptr
    }

    fn remove(&mut self, key: u64) {
        let ptr = key as *mut StringCacheEntry;
        let value: &mut StringCacheEntry = unsafe { mem::transmute(ptr) };

        debug_assert!(value.ref_count.load(SeqCst) == 0);

        let bucket_index = (value.hash & (self.buckets.len()-1) as u64) as usize;

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
            ptr::read(ptr);
            heap::deallocate(ptr as *mut u8,
                mem::size_of::<StringCacheEntry>(),
                max(mem::align_of::<StringCacheEntry>(), ENTRY_ALIGNMENT));
        }

        log!(Event::Remove(key));
    }
}

// NOTE: Deriving Eq here implies that a given string must always
// be interned the same way.
#[unsafe_no_drop_flag]
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

    #[inline]
    pub fn from_slice(string_to_add: &str) -> Atom {
        let unpacked = match STATIC_ATOM_SET.get_index(string_to_add) {
            Some(id) => Static(id as u32),
            None => {
                let len = string_to_add.len();
                if len <= string_cache_shared::MAX_INLINE_LEN {
                    let mut buf: [u8; 7] = [0; 7];
                    bytes::copy_memory(string_to_add.as_bytes(), &mut buf);
                    Inline(len as u8, buf)
                } else {
                    Dynamic(STRING_CACHE.lock().unwrap().add(string_to_add) as *mut ())
                }
            }
        };

        let data = unsafe { unpacked.pack() };
        log!(Event::Intern(data));
        Atom { data: data }
    }

    #[inline]
    pub fn as_slice<'t>(&'t self) -> &'t str {
        unsafe {
            match self.unpack() {
                Inline(..) => {
                    let buf = string_cache_shared::inline_orig_bytes(&self.data);
                    str::from_utf8(buf).unwrap()
                },
                Static(idx) => *STATIC_ATOM_SET.index(idx as usize).expect("bad static atom"),
                Dynamic(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    &(*entry).string
                }
            }
        }
    }
}

impl Clone for Atom {
    #[inline(always)]
    fn clone(&self) -> Atom {
        unsafe {
            match string_cache_shared::from_packed_dynamic(self.data) {
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
            match string_cache_shared::from_packed_dynamic(self.data) {
                // We use #[unsafe_no_drop_flag] so that Atom will be only 64
                // bits.  That means we need to ignore a NULL pointer here,
                // which represents a value that was moved out.
                Some(entry) if !entry.is_null() => {
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
        self.as_slice()
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

        write!(f, "Atom('{}' type={})", self.as_slice(), ty_str)
    }
}

impl PartialOrd for Atom {
    #[inline]
    fn partial_cmp(&self, other: &Atom) -> Option<Ordering> {
        if self.data == other.data {
            return Some(Equal);
        }
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl Ord for Atom {
    #[inline]
    fn cmp(&self, other: &Atom) -> Ordering {
        if self.data == other.data {
            return Equal;
        }
        self.as_slice().cmp(other.as_slice())
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
        Ok(Atom::from_slice(&*string))
    }
}

#[cfg(test)]
mod bench;

#[cfg(test)]
mod tests {
    use std::thread;
    use super::Atom;
    use string_cache_shared::{Static, Inline, Dynamic};

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
                format_args!($fmt, x),
                format_args!($fmt, y));
        }
    }));

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
        use std::mem;
        assert_eq!(8, mem::size_of::<super::Atom>());
        assert_eq!(48, mem::size_of::<super::StringCacheEntry>());
    }

    #[test]
    fn test_threads() {
        for _ in 0_u32..100 {
            thread::spawn(move || {
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
        assert_eq!(2, match Atom::from_slice("head") {
            atom!(br) => 1,
            atom!(html) | atom!(head) => 2,
            _ => 3,
        });

        assert_eq!(3, match Atom::from_slice("body") {
            atom!(br) => 1,
            atom!(html) | atom!(head) => 2,
            _ => 3,
        });

        assert_eq!(3, match Atom::from_slice("zzzzzz") {
            atom!(br) => 1,
            atom!(html) | atom!(head) => 2,
            _ => 3,
        });
    }

    #[test]
    fn ensure_deref() {
        // Ensure we can Deref to a &str
        let atom = Atom::from_slice("foobar");
        let _: &str = &atom;
    }

    #[test]
    fn ensure_as_ref() {
        // Ensure we can as_ref to a &str
        let atom = Atom::from_slice("foobar");
        let _: &str = atom.as_ref();
    }
}
