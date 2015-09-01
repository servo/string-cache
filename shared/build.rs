extern crate phf_generator;

mod static_atom_list;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let mut set = std::collections::HashSet::new();
    for atom in static_atom_list::ATOMS {
        if !set.insert(atom) {
            panic!("duplicate static atom `{:?}`", atom);
        }
    }

    let state = phf_generator::generate_hash(static_atom_list::ATOMS);

    let path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("static_atom_set.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    macro_rules! w {
        ($($arg: expr),+) => { (writeln!(&mut file, $($arg),+).unwrap()) }
    }
    w!("pub static STATIC_ATOM_SET: StaticAtomSet = StaticAtomSet {{");
    w!("    key: {},", state.key);
    w!("    disps: &[");
    for &(d1, d2) in &state.disps {
        w!("        ({}, {}),", d1, d2);
    }
    w!("    ],");
    w!("    atoms: &[");
    for &idx in &state.map {
        w!("        {:?},", static_atom_list::ATOMS[idx]);
    }
    w!("    ],");
    w!("}};");
}
