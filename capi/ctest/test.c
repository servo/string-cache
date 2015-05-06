// Copyright 2015 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "string_cache.h"

#define CHECK(_x) if (!(_x)) { \
    printf("Assertion failed: %s\n", #_x); \
    exit(1); \
}

int main() {
    scache_atom x = scache_atom_from_buffer("hello", 5);
    CHECK(scache_atom_len(&x) == 5);
    CHECK(!strncmp(scache_atom_data(&x), "hello", 5));

    scache_atom y = scache_atom_from_c_str("blockquote");
    CHECK(scache_atom_len(&y) == 10);
    CHECK(!strncmp(scache_atom_data(&y), "blockquote", 10));
    CHECK(y.unique_id != x.unique_id);

    scache_atom z = scache_atom_from_c_str("zzzzzzzzz");
    CHECK(scache_atom_len(&z) == 9);
    CHECK(!strncmp(scache_atom_data(&z), "zzzzzzzzz", 9));
    CHECK(z.unique_id != x.unique_id);
    CHECK(z.unique_id != y.unique_id);

    scache_atom w = scache_atom_clone(&z);
    CHECK(scache_atom_len(&w) == 9);
    CHECK(!strncmp(scache_atom_data(&w), "zzzzzzzzz", 9));
    CHECK(w.unique_id == z.unique_id);

    scache_atom_destroy(&x);
    scache_atom_destroy(&y);
    scache_atom_destroy(&z);
    scache_atom_destroy(&w);

    return 0;
}
