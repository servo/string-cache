extern crate string_cache_codegen;

#[path = "src/static_atom_list.rs"]
mod static_atom_list;

use std::env;
use std::path::Path;

fn main() {
    string_cache_codegen::AtomType::new("atom::tests::TestAtom", "test_atom!")
        .atoms(static_atom_list::ATOMS)
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("test_atom.rs"))
        .unwrap()
}
