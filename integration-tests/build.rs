use std::env;
use std::path::Path;

fn main() {
    string_cache_codegen::AtomType::new("TestAtom", "test_atom!")
        .atoms(&[
            "a",
            "b",
            "address",
            "defaults",
            "area",
            "body",
            "font-weight",
            "br",
            "html",
            "head",
            "id",
            "❤",
            "❤💯",
            "❤💯❤💯",
        ])
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("test_atom.rs"))
        .unwrap();

    // All statically-known atoms are short enough to be represented inline,
    // so the static set is empty. Ensure phf doesn’t divide by zero.
    string_cache_codegen::AtomType::new("TestAtom2", "test_atom2!")
        .atoms(&["a"])
        .write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("test_atom2.rs"))
        .unwrap()
}
