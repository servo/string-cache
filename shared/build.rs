extern crate phf_codegen;

mod static_atom_list;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let mut set = phf_codegen::OrderedSet::new();
    for &atom in static_atom_list::ATOMS {
        set.entry(atom);
    }

    let path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("static_atom_set.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    write!(&mut file, "pub static STATIC_ATOM_SET: phf::OrderedSet<&'static str> = ").unwrap();
    set.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();
}
