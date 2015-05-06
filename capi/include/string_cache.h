// Copyright 2015 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#ifndef _STRING_CACHE_H
#define _STRING_CACHE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// A value of type `scache_atom` represents an interned string.
//
// You can compare the `unique_id` field in order to test string equality.
//
// It is *not* safe to implicitly copy this value, and you must
// properly destroy it. See below.
typedef struct {
    uint64_t unique_id;
} scache_atom;

// Get a pointer to the characters of the interned string.
//
// This is *not* NULL-terminated. You can get the length from `scache_atom_len`.
// The string is guaranteed to be well-formed UTF-8.
//
// The pointer is valid until `scache_atom_destroy` is called. In some cases,
// it's an offset from the argument `x`, so the `scache_atom` struct you passed
// in must be valid for long as you're using the character pointer.
const char *scache_atom_data(const scache_atom *x);

// Get the length of the interned string, in bytes.
size_t scache_atom_len(const scache_atom *x);

// Copy an interned string.
//
// This is cheap, but is not equivalent to a shallow bitwise copy.
scache_atom scache_atom_clone(const scache_atom *x);

// Destroy an interned string.
//
// You must not use the string or its character data in any way afterwards!
void scache_atom_destroy(scache_atom *x);

// Create an interned string from a buffer of specified length.
//
// The buffer must be valid UTF-8.
scache_atom scache_atom_from_buffer(const char *buf, size_t len);

// Create an interned string from a C string (NULL-terminated).
//
// The string must be valid UTF-8.
scache_atom scache_atom_from_c_str(const char *buf);

#ifdef __cplusplus
}
#endif

#endif
