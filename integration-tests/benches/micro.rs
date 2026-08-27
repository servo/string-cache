// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*

A cautionary note about these benchmarks:

Many of the operations we're attempting to measure take less than one
nanosecond. That's why we run them thousands of times in a loop just to get a
single iteration that Rust's statistical benchmarking can work with. At that
scale, any change anywhere in the library can produce durable performance
regressions on the order of half a nanosecond, i.e. "500 ns" in the output for
a test like eq_x_1000.

We can't get anything done if we rachet on these numbers! They are more useful
for selecting between alternatives, and for noticing large regressions or
inconsistencies.

Furthermore, a large part of the point of interning is to make strings small
and cheap to move around, which isn't reflected in these tests.

*/

#![allow(non_upper_case_globals)]

use criterion::{Criterion, criterion_group, criterion_main};
use integration_tests::{TestAtom, test_atom};
use rand::{RngCore, SeedableRng, rngs::SmallRng};
use std::hint::black_box;

// Just shorthand
fn mk(x: &str) -> TestAtom {
    TestAtom::from(x)
}

fn benches_main(c: &mut Criterion) {
    macro_rules! bench_tiny_op {
        ($name1:ident $name2:expr, $op:ident, $ctor_x:expr, $ctor_y:expr) => {
            c.bench_function(concat!(stringify!($name1), " ", $name2), |b| {
                const n: usize = 1000;
                let xs: Vec<_> = std::iter::repeat_n($ctor_x, n).collect();
                let ys: Vec<_> = std::iter::repeat_n($ctor_y, n).collect();

                b.iter(|| {
                    for (x, y) in xs.iter().zip(ys.iter()) {
                        black_box(x.$op(y));
                    }
                })
            })
        };
    }

    macro_rules! bench_one {
        (x_static   $n:ident $x:expr, $y:expr) => ( assert!($x.is_static()); );
        (y_static   $n:ident $x:expr, $y:expr) => ( assert!($y.is_static()); );
        (is_static  $n:ident $x:expr, $y:expr) => ( assert!($x.is_static());
                                                    assert!($y.is_static()); );

        (x_inline   $n:ident $x:expr, $y:expr) => ( assert!($x.is_inline()); );
        (y_inline   $n:ident $x:expr, $y:expr) => ( assert!($y.is_inline()); );
        (is_inline  $n:ident $x:expr, $y:expr) => ( assert!($x.is_inline());
                                                    assert!($y.is_inline()); );

        (x_dynamic  $n:ident $x:expr, $y:expr) => ( assert!($x.is_dynamic()); );
        (y_dynamic  $n:ident $x:expr, $y:expr) => ( assert!($y.is_dynamic()); );
        (is_dynamic $n:ident $x:expr, $y:expr) => ( assert!($x.is_dynamic());
                                                    assert!($y.is_dynamic()); );

        (eq $n:ident $x:expr, $y:expr) => ( bench_tiny_op!($n "eq ×1000", eq, $x, $x); );
        (ne $n:ident $x:expr, $y:expr) => ( bench_tiny_op!($n "ne ×1000", ne, $x, $y); );
        (lt $n:ident $x:expr, $y:expr) => ( bench_tiny_op!($n "lt ×1000", lt, $x, $y); );

        (intern $name: ident $x:expr, $_y:expr) => (
            c.bench_function(concat!(stringify!($name), " intern"), |b| {
                let x = $x.to_string();
                b.iter(|| {
                    black_box(TestAtom::from(&*x));
                })
            })
        );

        (as_str $name: ident $x:expr, $_y:expr) => (
            c.bench_function(concat!(stringify!($name), " as_str ×1000"), |b| {
                let x = $x;
                b.iter(|| {
                    for _ in 0..1000 {
                        black_box(x.as_str());
                    }
                });
            })
        );

        (as_bytes $name: ident $x:expr, $_y:expr) => (
            c.bench_function(concat!(stringify!($name), " as_bytes ×1000"), |b| {
                let x = $x;
                b.iter(|| {
                    for _ in 0..1000 {
                        black_box(x.as_bytes());
                    }
                });
            })
        );

        (clone $name: ident $x:expr, $_y:expr) => (
            c.bench_function(concat!(stringify!($name), " clone ×1000"), |b| {
                let x = $x;
                b.iter(|| {
                    for _ in 0..1000 {
                        black_box(x.clone());
                    }
                });
            })
        );

        (clone_string $name: ident $x:expr, $_y:expr) => (
            c.bench_function(concat!(stringify!($name), " String::clone ×1000"), |b| {
                let x = $x.to_string();
                b.iter(|| {
                    for _ in 0..1000 {
                        black_box(x.clone());
                    }
                });
            })
        );
    }

    macro_rules! benches {
        ($( [ $($which:ident)+ ] for $name:ident = $x:expr, $y:expr; )+) => {
            $($( bench_one!($which $name $x, $y); )+)+
        }
    }

    const LONGER_DYNAMIC_A: &str = "Thee Silver Mt. Zion Memorial Orchestra & Tra-La-La Band";
    const LONGER_DYNAMIC_B: &str = "Thee Silver Mt. Zion Memorial Orchestra & Tra-La-La Ban!";

    benches! {
        [eq ne lt clone_string] for short_string = "e", "f";
        [eq ne lt clone_string] for medium_string = "xyzzy01", "xyzzy02";
        [eq ne lt clone_string] for longer_string = LONGER_DYNAMIC_A, LONGER_DYNAMIC_B;

        [eq ne intern as_str as_bytes clone is_static lt]
            for static_atom = test_atom!("defaults"), test_atom!("font-weight");

        [intern as_str as_bytes clone is_inline]
            for short_inline_atom = mk("e"), mk("f");

        [eq ne intern as_str as_bytes clone is_inline lt]
            for medium_inline_atom = mk("xyzzy01"), mk("xyzzy02");

        [intern as_str as_bytes clone is_dynamic]
            for min_dynamic_atom = mk("xyzzy001"), mk("xyzzy002");

        [eq ne intern as_str as_bytes clone is_dynamic lt]
            for longer_dynamic_atom = mk(LONGER_DYNAMIC_A), mk(LONGER_DYNAMIC_B);

        [intern as_str as_bytes clone is_static]
            for static_at_runtime = mk("defaults"), mk("font-weight");

        [ne lt x_static y_inline]
            for static_vs_inline  = test_atom!("defaults"), mk("f");

        [ne lt x_static y_dynamic]
            for static_vs_dynamic = test_atom!("defaults"), mk(LONGER_DYNAMIC_B);

        [ne lt x_inline y_dynamic]
            for inline_vs_dynamic = mk("e"), mk(LONGER_DYNAMIC_B);
    }
}

fn bench_rand<const LEN: usize>(bencher: &mut criterion::Bencher) {
    let mut rng = SmallRng::from_entropy();
    bencher.iter(|| {
        // We have to generate new atoms on every iter, because
        // the dynamic atom table isn't reset.
        //
        // I measured the overhead of random string generation
        // as about 3-12% at one point.

        let mut buf = [0_u8; LEN];
        rng.fill_bytes(&mut buf);
        for n in buf.iter_mut() {
            // shift into printable ASCII
            *n = (*n % 0x40) + 0x20;
        }
        let s = std::str::from_utf8(&buf[..]).unwrap();
        black_box(TestAtom::from(s));
    })
}

fn benches_rand(c: &mut Criterion) {
    c.bench_function("intern rand 008", bench_rand::<8>);
    c.bench_function("intern rand 032", bench_rand::<32>);
    c.bench_function("intern rand 128", bench_rand::<128>);
    c.bench_function("intern rand 512", bench_rand::<512>);
}

criterion_group!(benches, benches_main, benches_rand);
criterion_main!(benches);
