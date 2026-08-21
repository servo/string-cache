// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use parking_lot::Mutex;
use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::SeqCst;

const NB_BUCKETS: usize = 1 << 12; // 4096
const BUCKET_MASK: u64 = (1 << 12) - 1;

pub(crate) struct Set {
    buckets: Box<[Mutex<Option<NonNull<Entry>>>]>,
}

pub(crate) struct Entry {
    // These fields can be accessed freely by `Atom` methods
    pub(crate) string: Box<str>,
    pub(crate) hash: u64,
    pub(crate) ref_count: AtomicIsize,
    // This field is protected by a `Mutex` in `Set`
    next_in_bucket: UnsafeCell<Option<NonNull<Entry>>>,
}

// SAFETY: Access to the global linked list is strictly guarded by a Mutex,
// and the reference counts are atomic. Even though `NonNull` is strictly
// `!Send` and `!Sync`, the surrounding architecture makes it safe to share.
unsafe impl Send for Entry {}
unsafe impl Sync for Entry {}

unsafe impl Send for Set {}
unsafe impl Sync for Set {}

pub(crate) fn dynamic_set() -> &'static Set {
    // NOTE: Using const initialization for buckets breaks the small-stack test.
    static DYNAMIC_SET: OnceLock<Set> = OnceLock::new();

    DYNAMIC_SET.get_or_init(|| {
        let buckets = (0..NB_BUCKETS).map(|_| Mutex::new(None)).collect();
        Set { buckets }
    })
}

impl Set {
    pub(crate) fn insert(&self, string: Cow<str>, hash: u64) -> NonNull<Entry> {
        let bucket_index = (hash & BUCKET_MASK) as usize;
        let mut linked_list = self.buckets[bucket_index].lock();

        {
            let mut ptr: Option<NonNull<Entry>> = *linked_list;

            while let Some(entry_ptr) = ptr {
                // SAFETY: We hold the Mutex lock for this bucket, so no other thread can mutate
                // the linked list. The `NonNull` pointer is guaranteed to point to a valid Entry.
                let entry = unsafe { entry_ptr.as_ref() };
                if entry.hash == hash && *entry.string == *string {
                    let old_size = entry.ref_count.fetch_add(1, SeqCst);
                    if old_size > 0 {
                        if old_size == isize::MAX {
                            std::process::abort();
                        }
                        return entry_ptr;
                    }
                    // Uh-oh. The pointer's reference count was zero, which means someone may try
                    // to free it. (Naive attempts to defend against this, for example having the
                    // destructor check to see whether the reference count is indeed zero, don't
                    // work due to ABA.) Thus we need to temporarily add a duplicate string to the
                    // list.
                    entry.ref_count.fetch_sub(1, SeqCst);
                    break;
                }
                // SAFETY: We hold the Mutex lock for this bucket, so no other thread can mutate
                // the linked list.
                ptr = unsafe { entry.next_in_bucket.get().read() };
            }
        }
        let string = string.into_owned();
        let entry = Box::new(Entry {
            next_in_bucket: UnsafeCell::new(linked_list.take()),
            hash,
            ref_count: AtomicIsize::new(1),
            string: string.into_boxed_str(),
        });
        let ptr = NonNull::from(Box::leak(entry));
        *linked_list = Some(ptr);
        ptr
    }

    pub(crate) fn remove(&self, ptr: *mut Entry) {
        // SAFETY: The caller provides a pointer derived from a valid Atom. We hold the lock
        // below, and `ptr` is guaranteed to be valid until we drop the `Box` later in this function.
        let value: &Entry = unsafe { &*ptr };
        let bucket_index = (value.hash & BUCKET_MASK) as usize;

        let mut lock_guard = self.buckets[bucket_index].lock();
        debug_assert!(value.ref_count.load(SeqCst) == 0);
        let mut current: &mut Option<NonNull<Entry>> = &mut lock_guard;

        while let Some(entry_ptr) = *current {
            if entry_ptr.as_ptr() == ptr {
                // SAFETY: The reference count has reached 0, and we hold the bucket lock.
                // We have exclusive access to recreate the Box and deallocate the memory.
                let unlinked_entry = unsafe { Box::from_raw(entry_ptr.as_ptr()) };
                *current = unlinked_entry.next_in_bucket.into_inner();
                // We’re done accessing data protected by the mutex
                drop(lock_guard);
                // The `Box` is deallocated here after releasing the lock
                break;
            }
            // SAFETY: We hold the bucket lock, so the pointer remains valid and unaliased here.
            // Still, don’t create `&mut Entry` here because `Atom` methods may have a `&Entry`.
            let entry = unsafe { entry_ptr.as_ref() };
            // SAFETY: The `UnsafeCell` is safe to access here because we hold the `Mutex`
            current = unsafe { &mut *entry.next_in_bucket.get() };
        }
    }
}
