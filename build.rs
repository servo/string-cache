extern crate string_cache_codegen;

#[path = "src/static_atom_list.rs"] mod static_atom_list;

use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("static_atoms.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let mut builder = string_cache_codegen::AtomSetBuilder::new();
    for atom in static_atom_list::ATOMS {
        builder.atom(atom);
    }
    builder.build(&mut file, "ServoAtom", "STATIC_ATOM_SET", "atom");
}
