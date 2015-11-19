extern crate phf_shared;
extern crate phf_generator;

#[path = "src/shared.rs"] #[allow(dead_code)] mod shared;
#[path = "src/static_atom_list.rs"] mod static_atom_list;

use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::mem;
use std::path::Path;
use std::slice;

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
    w!("pub static STATIC_ATOM_SET: StaticAtomSet = StaticAtomSet {{");
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
    let set = shared::StaticAtomSet {
        key: hash_state.key,
        disps: leak(hash_state.disps.clone()),
        atoms: leak(hash_state.map.iter().map(|&idx| static_atom_list::ATOMS[idx]).collect()),
    };

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("atom_macro.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    writeln!(file, r"#[macro_export]").unwrap();
    writeln!(file, r"macro_rules! atom {{").unwrap();
    for &s in set.iter() {
        if is_ident(s) {
            writeln!(file, r"( {} ) => {{ {} }};", s, atom(&set, s)).unwrap();
        }
        writeln!(file, r"({:?}) => {{ {} }};", s, atom(&set, s)).unwrap();
    }
    writeln!(file, r"}}").unwrap();
}

fn leak<T>(v: Vec<T>) -> &'static [T] {
    let slice = unsafe { slice::from_raw_parts(v.as_ptr(), v.len()) };
    mem::forget(v);
    slice
}

fn atom(set: &shared::StaticAtomSet, s: &str) -> String {
    let data = shared::pack_static(set.get_index_or_hash(s).unwrap() as u32);
    format!("$crate::Atom {{ data: 0x{:x} }}", data)
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    !s.is_empty() && match chars.next().unwrap() {
        'a'...'z' | 'A'...'Z' | '_' => true,
        _ => false
    } && chars.all(|c| match c {
        'a'...'z' | 'A'...'Z' | '_' | '0'...'9' => true,
        _ => false
    })
}
