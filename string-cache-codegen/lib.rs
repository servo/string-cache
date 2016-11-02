// Copyright 2016 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate phf_generator;

use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write, BufWriter};
use std::path::Path;

#[allow(dead_code)]
mod shared;

/// A builder for a static atom set and relevant macros
pub struct AtomType {
    path: String,
    macro_name: String,
    atoms: HashSet<String>,
}

impl AtomType {
    /// Constructs a new static atom set builder
    ///
    /// `path` is a path within a crate of the atom type that will be created.
    /// e.g. `"FooAtom"` at the crate root or `"foo::Atom"` if the generated code
    /// is included in a `foo` module.
    ///
    /// `macro_name` must end with `!`.
    ///
    /// For example, `AtomType::new("foo::FooAtom", "foo_atom!")` will generate:
    ///
    /// ```rust
    /// pub type FooAtom = ::string_cache::Atom<FooAtomStaticSet>;
    /// pub struct FooAtomStaticSet;
    /// impl ::string_cache::StaticAtomSet for FooAtomStaticSet {
    ///     // ...
    /// }
    /// #[macro_export]
    /// macro_rules foo_atom {
    ///    // Expands to: $crate::foo::FooAtom { … }
    /// }
    pub fn new(path: &str, macro_name: &str) -> Self {
        assert!(macro_name.ends_with("!"));
        AtomType {
            path: path.to_owned(),
            macro_name: macro_name[..macro_name.len() - "!".len()].to_owned(),
            atoms: HashSet::new(),
        }
    }

    /// Adds an atom to the builder
    pub fn atom(&mut self, s: &str) -> &mut Self {
        self.atoms.insert(s.to_owned());
        self
    }

    /// Adds multiple atoms to the builder
    pub fn atoms<I>(&mut self, iter: I) -> &mut Self
    where I: IntoIterator, I::Item: AsRef<str> {
        self.atoms.extend(iter.into_iter().map(|s| s.as_ref().to_owned()));
        self
    }

    /// Write generated code to `destination`.
    pub fn write_to<W>(&mut self, mut destination: W) -> io::Result<()> where W: Write {
        // `impl Default for Atom` requires the empty string to be in the static set.
        // This also makes sure the set in non-empty,
        // which would cause divisions by zero in rust-phf.
        self.atoms.insert(String::new());

        let atoms: Vec<&str> = self.atoms.iter().map(|s| &**s).collect();
        let hash_state = phf_generator::generate_hash(&atoms);
        let atoms: Vec<&str> = hash_state.map.iter().map(|&idx| atoms[idx]).collect();
        let empty_string_index = atoms.iter().position(|s| s.is_empty()).unwrap();

        let type_name = if let Some(position) = self.path.rfind("::") {
            &self.path[position + "::".len() ..]
        } else {
            &self.path
        };

        macro_rules! w {
            ($($arg: expr),+) => { try!(writeln!(destination, $($arg),+)) }
        }

        w!("pub type {} = ::string_cache::Atom<{}StaticSet>;", type_name, type_name);
        w!("pub struct {}StaticSet;", type_name);
        w!("impl ::string_cache::StaticAtomSet for {}StaticSet {{", type_name);
        w!("    fn get() -> &'static ::string_cache::PhfStrSet {{");
        w!("        static SET: ::string_cache::PhfStrSet = ::string_cache::PhfStrSet {{");
        w!("            key: {},", hash_state.key);
        w!("            disps: &{:?},", hash_state.disps);
        w!("            atoms: &{:#?},", atoms);
        w!("        }};");
        w!("        &SET");
        w!("    }}");
        w!("    fn empty_string_index() -> u32 {{");
        w!("        {}", empty_string_index);
        w!("    }}");
        w!("}}");
        w!("#[macro_export]");
        w!("macro_rules! {} {{", self.macro_name);
        for (i, atom) in atoms.iter().enumerate() {
            w!("({:?}) => {{ $crate::{} {{ unsafe_data: 0x{:x}, phantom: ::std::marker::PhantomData }} }};",
               atom,
               self.path,
               shared::pack_static(i as u32)
            );
        }
        w!("}}");
        Ok(())
    }

    /// Create a new file at `path` and write generated code there.
    ///
    /// Typical usage:
    /// `.write_to_file(&Path::new(&env::var("OUT_DIR").unwrap()).join("foo_atom.rs"))`
    pub fn write_to_file(&mut self, path: &Path) -> io::Result<()> {
        self.write_to(BufWriter::new(try!(File::create(path))))
    }
}
