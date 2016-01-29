extern crate string_cache;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

#[cfg(test)]
mod test {
    use super::{ALPHABET_ATOMS, Alphabet, BorrowedAlphabet};
    use string_cache::atom::BorrowedBaseAtom;
    #[test]
    fn static_atom_set() {
        let a: Alphabet = alphabet!("a");
        assert!(&*a == "a");
        assert!(&*alphabet!("b") == "b");
        assert!(ALPHABET_ATOMS.contains("c"));
        assert!(!ALPHABET_ATOMS.contains("d"));
        assert!(ALPHABET_ATOMS.len() == 3);
        let ba: BorrowedAlphabet = BorrowedBaseAtom(&a);
        assert!(ba == alphabet!("a"));
    }
}
