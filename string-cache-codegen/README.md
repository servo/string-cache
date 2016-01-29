# string-cache-codegen

Example usage:

```
extern crate string_cache_codegen;

use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    let file = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&file).unwrap());

    string_cache_codegen::AtomSetBuilder::new()
        .atom("a")
        .atom("b")
        .atom("c")
        .build(&mut file, "Alphabet", "ALPHABET_ATOMS", "alphabet");
}
```

Then just add `include!(concat!(env!("OUT_DIR"), "/codegen.rs"));` to the root
of your crate.
