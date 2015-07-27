extern crate string_cache_shared;

use string_cache_shared::{STATIC_ATOM_SET, ALL_NS, pack_static};

use std::ascii::AsciiExt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    let path = Path::new(env!("OUT_DIR")).join("ns_atom_macros_without_plugin.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    writeln!(file, r"#[macro_export]").unwrap();
    writeln!(file, r"macro_rules! ns {{").unwrap();
    writeln!(file, "(\"\") => {{ $crate::Namespace({}) }};", atom("")).unwrap();
    for &(prefix, url) in ALL_NS {
        if !prefix.is_empty() {
            generate_combination("".to_owned(), prefix, url, &mut file);
        }
    }
    writeln!(file, r"}}").unwrap();

    writeln!(file, r"#[macro_export]").unwrap();
    writeln!(file, r"macro_rules! atom {{").unwrap();
    for &s in STATIC_ATOM_SET.iter() {
        if is_ident(s) {
            writeln!(file, r"( {} ) => {{ {} }};", s, atom(s)).unwrap();
        }
        writeln!(file, r"({:?}) => {{ {} }};", s, atom(s)).unwrap();
    }
    writeln!(file, r"}}").unwrap();
}

fn generate_combination(prefix1: String, suffix: &str, url: &str, file: &mut BufWriter<File>) {
    if suffix.is_empty() {
        writeln!(file, r"({:?}) => {{ $crate::Namespace({}) }};", prefix1, atom(url)).unwrap();
        writeln!(file, r"( {} ) => {{ $crate::Namespace({}) }};", prefix1, atom(url)).unwrap();
    } else {
        let prefix2 = prefix1.clone();
        generate_combination(prefix1 + &*suffix[..1].to_ascii_lowercase(), &suffix[1..], url, file);
        generate_combination(prefix2 + &*suffix[..1].to_ascii_uppercase(), &suffix[1..], url, file);
    }
}

fn atom(s: &str) -> String {
    let data = pack_static(STATIC_ATOM_SET.get_index(s).unwrap() as u32);
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
