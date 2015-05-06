#!/bin/sh

set -xe

(cd ..; cargo build)
gcc -o test test.c -Wall -I ../include -L ../target/debug -lstring_cache_capi -ldl -lpthread -lrt -lgcc_s -lpthread -lc -lm
./test
