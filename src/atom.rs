// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_upper_case_globals)]

use debug_unreachable::debug_unreachable;
use lazy_static::lazy_static;
use phf_shared;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::borrow::Cow;
use std::cmp::Ordering::{self, Equal};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::mem;
use std::num::NonZeroU64;
use std::ops;
use std::slice;
use std::str;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Mutex;

use self::UnpackedAtom::{Dynamic, Inline, Static};

const DYNAMIC_TAG: u8 = 0b_00;
const INLINE_TAG: u8 = 0b_01; // len in upper nybble
const STATIC_TAG: u8 = 0b_10;
const TAG_MASK: u64 = 0b_11;
const ENTRY_ALIGNMENT: usize = 4; // Multiples have TAG_MASK bits unset, available for tagging.

const MAX_INLINE_LEN: usize = 7;

const STATIC_SHIFT_BITS: usize = 32;

const NB_BUCKETS: usize = 1 << 12; // 4096
const BUCKET_MASK: u64 = (1 << 12) - 1;

struct StringCache {
    buckets: Box<[Option<Box<StringCacheEntry>>; NB_BUCKETS]>,
}

lazy_static! {
    static ref STRING_CACHE: Mutex<StringCache> = Mutex::new(StringCache::new());
}

struct StringCacheEntry {
    next_in_bucket: Option<Box<StringCacheEntry>>,
    hash: u64,
    ref_count: AtomicIsize,
    string: Box<str>,
}

impl StringCacheEntry {
    fn new(next: Option<Box<StringCacheEntry>>, hash: u64, string: String) -> StringCacheEntry {
        StringCacheEntry {
            next_in_bucket: next,
            hash: hash,
            ref_count: AtomicIsize::new(1),
            string: string.into_boxed_str(),
        }
    }
}

impl StringCache {
    fn new() -> StringCache {
        type T = Option<Box<StringCacheEntry>>;
        let _static_assert_size_eq = std::mem::transmute::<T, usize>;
        let vec = std::mem::ManuallyDrop::new(vec![0_usize; NB_BUCKETS]);
        StringCache {
            buckets: unsafe { Box::from_raw(vec.as_ptr() as *mut [T; NB_BUCKETS]) },
        }
    }

    fn add(&mut self, string: Cow<str>, hash: u64) -> *mut StringCacheEntry {
        let bucket_index = (hash & BUCKET_MASK) as usize;
        {
            let mut ptr: Option<&mut Box<StringCacheEntry>> = self.buckets[bucket_index].as_mut();

            while let Some(entry) = ptr.take() {
                if entry.hash == hash && &*entry.string == &*string {
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
        let string = string.into_owned();
        let mut entry = Box::new(StringCacheEntry::new(
            self.buckets[bucket_index].take(),
            hash,
            string,
        ));
        let ptr: *mut StringCacheEntry = &mut *entry;
        self.buckets[bucket_index] = Some(entry);

        ptr
    }

    fn remove(&mut self, ptr: *mut StringCacheEntry) {
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
                mem::drop(mem::replace(current, unsafe {
                    (*entry_ptr).next_in_bucket.take()
                }));
                break;
            }
            current = unsafe { &mut (*entry_ptr).next_in_bucket };
        }
    }
}

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
    /// Get the index of the empty string, which is in every set and is used for `Atom::default`.
    fn empty_string_index() -> u32;
}

/// A string set created using a [perfect hash function], specifically
/// [Hash, Displace and Compress].
///
/// See the CHD document for the meaning of the struct fields.
///
/// [perfect hash function]: https://en.wikipedia.org/wiki/Perfect_hash_function
/// [Hash, Displace and Compress]: http://cmph.sourceforge.net/papers/esa09.pdf
pub struct PhfStrSet {
    pub key: u64,
    pub disps: &'static [(u32, u32)],
    pub atoms: &'static [&'static str],
    pub hashes: &'static [u32],
}

/// An empty static atom set for when only dynamic strings will be added
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct EmptyStaticAtomSet;

impl StaticAtomSet for EmptyStaticAtomSet {
    fn get() -> &'static PhfStrSet {
        // The name is a lie: this set is not empty (it contains the empty string)
        // but that’s only to avoid divisions by zero in rust-phf.
        static SET: PhfStrSet = PhfStrSet {
            key: 0,
            disps: &[(0, 0)],
            atoms: &[""],
            // "" SipHash'd, and xored with u64_hash_to_u32.
            hashes: &[0x3ddddef3],
        };
        &SET
    }

    fn empty_string_index() -> u32 {
        0
    }
}

/// Use this if you don’t care about static atoms.
pub type DefaultAtom = Atom<EmptyStaticAtomSet>;

/// Represents a string that has been interned.
///
/// While the type definition for `Atom` indicates that it generic on a particular
/// implementation of an atom set, you don't need to worry about this.  Atoms can be static
/// and come from a `StaticAtomSet` generated by the `string_cache_codegen` crate, or they
/// can be dynamic and created by you on an `EmptyStaticAtomSet`.
///
/// `Atom` implements `Clone` but not `Copy`, since internally atoms are reference-counted;
/// this means that you may need to `.clone()` an atom to keep copies to it in different
/// places, or when passing it to a function that takes an `Atom` rather than an `&Atom`.
///
/// ## Creating an atom at runtime
///
/// If you use `string_cache_codegen` to generate a precomputed list of atoms, your code
/// may then do something like read data from somewhere and extract tokens that need to be
/// compared to the atoms.  In this case, you can use `Atom::from(&str)` or
/// `Atom::from(String)`.  These create a reference-counted atom which will be
/// automatically freed when all references to it are dropped.
///
/// This means that your application can safely have a loop which tokenizes data, creates
/// atoms from the tokens, and compares the atoms to a predefined set of keywords, without
/// running the risk of arbitrary memory consumption from creating large numbers of atoms —
/// as long as your application does not store clones of the atoms it creates along the
/// way.
///
/// For example, the following is safe and will not consume arbitrary amounts of memory:
///
/// ```ignore
/// let untrusted_data = "large amounts of text ...";
///
/// for token in untrusted_data.split_whitespace() {
///     let atom = Atom::from(token); // interns the string
///
///     if atom == Atom::from("keyword") {
///         // handle that keyword
///     } else if atom == Atom::from("another_keyword") {
///         // handle that keyword
///     } else {
///         println!("unknown keyword");
///     }
/// } // atom is dropped here, so it is not kept around in memory
/// ```
#[derive(PartialEq, Eq)]
// NOTE: Deriving PartialEq requires that a given string must always be interned the same way.
pub struct Atom<Static> {
    unsafe_data: NonZeroU64,
    phantom: PhantomData<Static>,
}

impl<Static: StaticAtomSet> ::precomputed_hash::PrecomputedHash for Atom<Static> {
    fn precomputed_hash(&self) -> u32 {
        self.get_hash()
    }
}

impl<'a, Static: StaticAtomSet> From<&'a Atom<Static>> for Atom<Static> {
    fn from(atom: &'a Self) -> Self {
        atom.clone()
    }
}

fn u64_hash_as_u32(h: u64) -> u32 {
    // This may or may not be great...
    ((h >> 32) ^ h) as u32
}

// FIXME: bound removed from the struct definition before of this error for pack_static:
// "error[E0723]: trait bounds other than `Sized` on const fn parameters are unstable"
// https://github.com/rust-lang/rust/issues/57563
impl<Static> Atom<Static> {
    /// For the atom!() macros
    #[inline(always)]
    #[doc(hidden)]
    pub const fn pack_static(n: u32) -> Self {
        Self {
            unsafe_data: unsafe {
                // STATIC_TAG ensure this is non-zero
                NonZeroU64::new_unchecked((STATIC_TAG as u64) | ((n as u64) << STATIC_SHIFT_BITS))
            },
            phantom: PhantomData,
        }
    }
}

impl<Static: StaticAtomSet> Atom<Static> {
    #[inline(always)]
    unsafe fn unpack(&self) -> UnpackedAtom {
        UnpackedAtom::from_packed(self.unsafe_data)
    }

    /// Return the internal repersentation. For testing.
    #[doc(hidden)]
    pub fn unsafe_data(&self) -> u64 {
        self.unsafe_data.get()
    }

    /// Return true if this is a static Atom. For testing.
    #[doc(hidden)]
    pub fn is_static(&self) -> bool {
        match unsafe { self.unpack() } {
            Static(..) => true,
            _ => false,
        }
    }

    /// Return true if this is a dynamic Atom. For testing.
    #[doc(hidden)]
    pub fn is_dynamic(&self) -> bool {
        match unsafe { self.unpack() } {
            Dynamic(..) => true,
            _ => false,
        }
    }

    /// Return true if this is an inline Atom. For testing.
    #[doc(hidden)]
    pub fn is_inline(&self) -> bool {
        match unsafe { self.unpack() } {
            Inline(..) => true,
            _ => false,
        }
    }

    /// Get the hash of the string as it is stored in the set.
    pub fn get_hash(&self) -> u32 {
        match unsafe { self.unpack() } {
            Static(index) => {
                let static_set = Static::get();
                static_set.hashes[index as usize]
            }
            Dynamic(entry) => {
                let entry = entry as *mut StringCacheEntry;
                u64_hash_as_u32(unsafe { (*entry).hash })
            }
            Inline(..) => u64_hash_as_u32(self.unsafe_data.get()),
        }
    }
}

impl<Static: StaticAtomSet> Default for Atom<Static> {
    #[inline]
    fn default() -> Self {
        Atom::pack_static(Static::empty_string_index())
    }
}

impl<Static: StaticAtomSet> Hash for Atom<Static> {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u32(self.get_hash())
    }
}

impl<Static: StaticAtomSet> PartialEq<str> for Atom<Static> {
    fn eq(&self, other: &str) -> bool {
        &self[..] == other
    }
}

impl<Static: StaticAtomSet> PartialEq<Atom<Static>> for str {
    fn eq(&self, other: &Atom<Static>) -> bool {
        self == &other[..]
    }
}

impl<Static: StaticAtomSet> PartialEq<String> for Atom<Static> {
    fn eq(&self, other: &String) -> bool {
        &self[..] == &other[..]
    }
}

impl<'a, Static: StaticAtomSet> From<Cow<'a, str>> for Atom<Static> {
    #[inline]
    fn from(string_to_add: Cow<'a, str>) -> Self {
        let static_set = Static::get();
        let hash = phf_shared::hash(&*string_to_add, &static_set.key);
        let index = phf_shared::get_index(&hash, static_set.disps, static_set.atoms.len());

        let unpacked = if static_set.atoms[index as usize] == string_to_add {
            Static(index)
        } else {
            let len = string_to_add.len();
            if len <= MAX_INLINE_LEN {
                let mut buf: [u8; 7] = [0; 7];
                buf[..len].copy_from_slice(string_to_add.as_bytes());
                Inline(len as u8, buf)
            } else {
                let hash = (hash.g as u64) << 32 | (hash.f1 as u64);
                Dynamic(STRING_CACHE.lock().unwrap().add(string_to_add, hash) as *mut ())
            }
        };

        unsafe { unpacked.pack() }
    }
}

impl<'a, Static: StaticAtomSet> From<&'a str> for Atom<Static> {
    #[inline]
    fn from(string_to_add: &str) -> Self {
        Atom::from(Cow::Borrowed(string_to_add))
    }
}

impl<Static: StaticAtomSet> From<String> for Atom<Static> {
    #[inline]
    fn from(string_to_add: String) -> Self {
        Atom::from(Cow::Owned(string_to_add))
    }
}

impl<Static: StaticAtomSet> Clone for Atom<Static> {
    #[inline(always)]
    fn clone(&self) -> Self {
        unsafe {
            match from_packed_dynamic(self.unsafe_data.get()) {
                Some(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    (*entry).ref_count.fetch_add(1, SeqCst);
                }
                None => (),
            }
        }
        Atom {
            unsafe_data: self.unsafe_data,
            phantom: PhantomData,
        }
    }
}

impl<Static> Drop for Atom<Static> {
    #[inline]
    fn drop(&mut self) {
        // Out of line to guide inlining.
        fn drop_slow<Static>(this: &mut Atom<Static>) {
            STRING_CACHE
                .lock()
                .unwrap()
                .remove(this.unsafe_data.get() as *mut StringCacheEntry);
        }

        unsafe {
            match from_packed_dynamic(self.unsafe_data.get()) {
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

impl<Static: StaticAtomSet> ops::Deref for Atom<Static> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        unsafe {
            match self.unpack() {
                Inline(..) => {
                    let buf = inline_orig_bytes(&self.unsafe_data);
                    str::from_utf8_unchecked(buf)
                }
                Static(idx) => Static::get()
                    .atoms
                    .get(idx as usize)
                    .expect("bad static atom"),
                Dynamic(entry) => {
                    let entry = entry as *mut StringCacheEntry;
                    &(*entry).string
                }
            }
        }
    }
}

impl<Static: StaticAtomSet> fmt::Display for Atom<Static> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <str as fmt::Display>::fmt(self, f)
    }
}

impl<Static: StaticAtomSet> fmt::Debug for Atom<Static> {
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

impl<Static: StaticAtomSet> PartialOrd for Atom<Static> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.unsafe_data == other.unsafe_data {
            return Some(Equal);
        }
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<Static: StaticAtomSet> Ord for Atom<Static> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.unsafe_data == other.unsafe_data {
            return Equal;
        }
        self.as_ref().cmp(other.as_ref())
    }
}

impl<Static: StaticAtomSet> AsRef<str> for Atom<Static> {
    fn as_ref(&self) -> &str {
        &self
    }
}

impl<Static: StaticAtomSet> Serialize for Atom<Static> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string: &str = self.as_ref();
        string.serialize(serializer)
    }
}

impl<'a, Static: StaticAtomSet> Deserialize<'a> for Atom<Static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        let string: String = Deserialize::deserialize(deserializer)?;
        Ok(Atom::from(string))
    }
}

// AsciiExt requires mutating methods, so we just implement the non-mutating ones.
// We don't need to implement is_ascii because there's no performance improvement
// over the one from &str.
impl<Static: StaticAtomSet> Atom<Static> {
    fn from_mutated_str<F: FnOnce(&mut str)>(s: &str, f: F) -> Self {
        let mut buffer = mem::MaybeUninit::<[u8; 64]>::uninit();
        let buffer = unsafe { &mut *buffer.as_mut_ptr() };

        if let Some(buffer_prefix) = buffer.get_mut(..s.len()) {
            buffer_prefix.copy_from_slice(s.as_bytes());
            let as_str = unsafe { ::std::str::from_utf8_unchecked_mut(buffer_prefix) };
            f(as_str);
            Atom::from(&*as_str)
        } else {
            let mut string = s.to_owned();
            f(&mut string);
            Atom::from(string)
        }
    }

    /// Like [`to_ascii_uppercase`].
    ///
    /// [`to_ascii_uppercase`]: https://doc.rust-lang.org/std/ascii/trait.AsciiExt.html#tymethod.to_ascii_uppercase
    pub fn to_ascii_uppercase(&self) -> Self {
        for (i, b) in self.bytes().enumerate() {
            if let b'a'..=b'z' = b {
                return Atom::from_mutated_str(self, |s| s[i..].make_ascii_uppercase());
            }
        }
        self.clone()
    }

    /// Like [`to_ascii_lowercase`].
    ///
    /// [`to_ascii_lowercase`]: https://doc.rust-lang.org/std/ascii/trait.AsciiExt.html#tymethod.to_ascii_lowercase
    pub fn to_ascii_lowercase(&self) -> Self {
        for (i, b) in self.bytes().enumerate() {
            if let b'A'..=b'Z' = b {
                return Atom::from_mutated_str(self, |s| s[i..].make_ascii_lowercase());
            }
        }
        self.clone()
    }

    /// Like [`eq_ignore_ascii_case`].
    ///
    /// [`eq_ignore_ascii_case`]: https://doc.rust-lang.org/std/ascii/trait.AsciiExt.html#tymethod.eq_ignore_ascii_case
    pub fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        (self == other) || self.eq_str_ignore_ascii_case(&**other)
    }

    /// Like [`eq_ignore_ascii_case`], but takes an unhashed string as `other`.
    ///
    /// [`eq_ignore_ascii_case`]: https://doc.rust-lang.org/std/ascii/trait.AsciiExt.html#tymethod.eq_ignore_ascii_case
    pub fn eq_str_ignore_ascii_case(&self, other: &str) -> bool {
        (&**self).eq_ignore_ascii_case(other)
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

#[inline(always)]
fn inline_atom_slice(x: &NonZeroU64) -> &[u8] {
    unsafe {
        let x: *const NonZeroU64 = x;
        let mut data = x as *const u8;
        // All except the lowest byte, which is first in little-endian, last in big-endian.
        if cfg!(target_endian = "little") {
            data = data.offset(1);
        }
        let len = 7;
        slice::from_raw_parts(data, len)
    }
}

#[inline(always)]
fn inline_atom_slice_mut(x: &mut u64) -> &mut [u8] {
    unsafe {
        let x: *mut u64 = x;
        let mut data = x as *mut u8;
        // All except the lowest byte, which is first in little-endian, last in big-endian.
        if cfg!(target_endian = "little") {
            data = data.offset(1);
        }
        let len = 7;
        slice::from_raw_parts_mut(data, len)
    }
}

impl UnpackedAtom {
    /// Pack a key, fitting it into a u64 with flags and data. See `string_cache_shared` for
    /// hints for the layout.
    #[inline(always)]
    unsafe fn pack<Static: StaticAtomSet>(self) -> Atom<Static> {
        match self {
            Static(n) => Atom::pack_static(n),
            Dynamic(p) => {
                let data = p as u64;
                debug_assert!(0 == data & TAG_MASK);
                Atom {
                    // Callers are responsible for calling this with a valid, non-null pointer
                    unsafe_data: NonZeroU64::new_unchecked(data),
                    phantom: PhantomData,
                }
            }
            Inline(len, buf) => {
                debug_assert!((len as usize) <= MAX_INLINE_LEN);
                let mut data: u64 = (INLINE_TAG as u64) | ((len as u64) << 4);
                {
                    let dest = inline_atom_slice_mut(&mut data);
                    dest.copy_from_slice(&buf)
                }
                Atom {
                    // INLINE_TAG ensures this is never zero
                    unsafe_data: NonZeroU64::new_unchecked(data),
                    phantom: PhantomData,
                }
            }
        }
    }

    /// Unpack a key, extracting information from a single u64 into useable structs.
    #[inline(always)]
    unsafe fn from_packed(data: NonZeroU64) -> UnpackedAtom {
        debug_assert!(DYNAMIC_TAG == 0); // Dynamic is untagged

        match (data.get() & TAG_MASK) as u8 {
            DYNAMIC_TAG => Dynamic(data.get() as *mut ()),
            STATIC_TAG => Static((data.get() >> STATIC_SHIFT_BITS) as u32),
            INLINE_TAG => {
                let len = ((data.get() & 0xf0) >> 4) as usize;
                debug_assert!(len <= MAX_INLINE_LEN);
                let mut buf: [u8; 7] = [0; 7];
                let src = inline_atom_slice(&data);
                buf.copy_from_slice(src);
                Inline(len as u8, buf)
            }
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
unsafe fn inline_orig_bytes<'a>(data: &'a NonZeroU64) -> &'a [u8] {
    match UnpackedAtom::from_packed(*data) {
        Inline(len, _) => {
            let src = inline_atom_slice(&data);
            &src[..(len as usize)]
        }
        _ => debug_unreachable!(),
    }
}

// Some minor tests of internal layout here. See ../integration-tests for much
// more.
#[cfg(test)]
mod tests {
    use super::{DefaultAtom, StringCacheEntry, ENTRY_ALIGNMENT};
    use std::mem;

    #[test]
    fn assert_sizes() {
        use std::mem;
        struct EmptyWithDrop;
        impl Drop for EmptyWithDrop {
            fn drop(&mut self) {}
        }
        let compiler_uses_inline_drop_flags = mem::size_of::<EmptyWithDrop>() > 0;

        // Guard against accidental changes to the sizes of things.
        assert_eq!(
            mem::size_of::<DefaultAtom>(),
            if compiler_uses_inline_drop_flags {
                16
            } else {
                8
            }
        );
        assert_eq!(
            mem::size_of::<Option<DefaultAtom>>(),
            mem::size_of::<DefaultAtom>(),
        );
        assert_eq!(
            mem::size_of::<super::StringCacheEntry>(),
            8 + 4 * mem::size_of::<usize>()
        );
    }

    #[test]
    fn string_cache_entry_alignment_is_sufficient() {
        assert!(mem::align_of::<StringCacheEntry>() >= ENTRY_ALIGNMENT);
    }
}
