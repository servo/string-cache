extern crate phf_generator;

#[path = "src/shared.rs"] #[allow(dead_code)] mod shared;
#[path = "src/static_atom_list.rs"] mod static_atom_list;

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let hash_state = generate();
    write_static_atom_set(&hash_state);
    write_atom_macro(&hash_state);
}

fn generate() -> phf_generator::HashState {
    let mut set = std::collections::HashSet::new();
    for atom in static_atom_list::ATOMS {
        if !set.insert(atom) {
            panic!("duplicate static atom `{:?}`", atom);
        }
    }
    phf_generator::generate_hash(static_atom_list::ATOMS)
}

fn write_static_atom_set(hash_state: &phf_generator::HashState) {
    let path = Path::new(&std::env::var("OUT_DIR").unwrap()).join("static_atom_set.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    macro_rules! w {
        ($($arg: expr),+) => { (writeln!(&mut file, $($arg),+).unwrap()) }
    }
    w!("pub static STATIC_ATOM_SET: PhfStrSet = PhfStrSet {{");
    w!("    key: {},", hash_state.key);
    w!("    disps: &[");
    for &(d1, d2) in &hash_state.disps {
        w!("        ({}, {}),", d1, d2);
    }
    w!("    ],");
    w!("    atoms: &[");
    for &idx in &hash_state.map {
        w!("        {:?},", static_atom_list::ATOMS[idx]);
    }
    w!("    ],");
    w!("}};");
}

fn write_atom_macro(hash_state: &phf_generator::HashState) {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("atom_macro.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    writeln!(file, r"#[macro_export]").unwrap();
    writeln!(file, r"macro_rules! atom {{").unwrap();
    for (i, &idx) in hash_state.map.iter().enumerate() {
        writeln!(
            file,
            r"({:?}) => {{ $crate::Atom {{ unsafe_data: 0x{:x}, phantom: ::std::marker::PhantomData }} }};",
            static_atom_list::ATOMS[idx],
            shared::pack_static(i as u32),
        ).unwrap();
    }
    writeln!(file, r"}}").unwrap();
}
