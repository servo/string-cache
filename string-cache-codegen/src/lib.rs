extern crate phf_codegen;

use std::io::Write;

/// A builder for a static atom set and relevant macros
pub struct AtomSetBuilder {
    atoms: Vec<&'static str>,
}

impl AtomSetBuilder {
    /// Constructs a new static atom set builder
    pub fn new() -> AtomSetBuilder {
        AtomSetBuilder {
            atoms: vec![],
        }
    }

    /// Adds an atom to the builder
    pub fn atom(&mut self, s: &'static str) -> &mut AtomSetBuilder {
        self.atoms.push(s);
        self
    }

    /// Adds multiple atoms to the builder
    pub fn atoms(&mut self, ss: &[&'static str]) -> &mut AtomSetBuilder {
        // `self.atoms.extend_from_slice(ss);` in newer rust
        for s in ss {
            self.atoms.push(s);
        }
        self
    }

    /// Constructs a new atom type with the name `atom_type_name`, a static atom
    /// set with the name `static_set_name` and a macro with the name
    /// `macro_name` for converting strings to static atoms at compile time.
    /// Using the macro requires you to include the generated file in the root
    /// of your crate, likely with the `include!` macro.
    pub fn build<W>(&self, w: &mut W, atom_type_name: &str, static_set_name: &str, macro_name: &str) where W: Write {
        if self.atoms.is_empty() {
            panic!("must have more than one atom of a kind");
        }
        self.build_kind_definition(w, static_set_name, atom_type_name);
        self.build_static_atom_set(w, static_set_name);
        self.build_atom_macro(w, macro_name, atom_type_name);
    }

    fn build_kind_definition<W>(&self, w: &mut W, static_set_name: &str, atom_type_name: &str) where W: Write {
        writeln!(w, "#[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]").unwrap();
        writeln!(w, "pub struct {}Kind;", atom_type_name).unwrap();
        writeln!(w, "
impl ::string_cache::atom::Kind for {atom_type_name}Kind {{
    #[inline]
    fn get_index_or_hash(s: &str) -> Result<u32, u64> {{
        match {static_set_name}.get_index(s) {{
            Some(i) => Ok(i as u32),
            None => Err(::string_cache::shared::dynamic_hash(s)),
        }}
    }}

    #[inline]
    fn index(i: u32) -> Option<&'static str> {{
        {static_set_name}.index(i as usize).map(|&s| s)
    }}
}}
", atom_type_name=atom_type_name, static_set_name=static_set_name).unwrap();
        writeln!(w, "pub type {} = ::string_cache::atom::BaseAtom<{}Kind>;", atom_type_name, atom_type_name).unwrap();
        writeln!(w, "pub type Borrowed{}<'a> = ::string_cache::atom::BorrowedBaseAtom<'a, {}Kind>;", atom_type_name, atom_type_name).unwrap();
    }

    fn build_static_atom_set<W>(&self, w: &mut W, static_set_name: &str) where W: Write {
        writeln!(w, "pub static {}: ::string_cache::shared::phf::OrderedSet<&'static str> = ", static_set_name).unwrap();
        let mut builder = phf_codegen::OrderedSet::new();
        for &atom in &self.atoms {
            builder.entry(atom);
        }
        builder.phf_path("::string_cache::shared::phf").build(w).unwrap();
        writeln!(w, ";").unwrap();
    }

    fn build_atom_macro<W>(&self, w: &mut W, macro_name: &str, atom_type_name: &str) where W: Write {
        writeln!(w, r"#[macro_export]").unwrap();
        writeln!(w, r"macro_rules! {} {{", macro_name).unwrap();
        for (i, s) in self.atoms.iter().enumerate() {
            let data = pack_static(i as u32);
            writeln!(w, r"({:?}) => {{ $crate::{} {{ unsafe_data: 0x{:x}, kind: ::std::marker::PhantomData }} }};", s, atom_type_name, data).unwrap();
        }
        writeln!(w, r"}}").unwrap();
    }
}

// Duplicated from string_cache::shared to lift dependency on string_cache
const STATIC_TAG: u8 = 0b_10;
const STATIC_SHIFT_BITS: usize = 32;
fn pack_static(n: u32) -> u64 {
    (STATIC_TAG as u64) | ((n as u64) << STATIC_SHIFT_BITS)
}
