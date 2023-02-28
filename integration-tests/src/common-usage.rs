/// Test common usage by popular dependents (html5ever, lalrpop, browserlists-rs), to ensure no API-surface breaking changes
/// Created after https://github.com/servo/string-cache/issues/271
use std::collections::HashMap;

use crate::Atom;
use crate::TestAtom;

#[test]
fn usage_with_hashmap() {
    let mut map: HashMap<TestAtom, i32> = HashMap::new();

    map.insert(test_atom!("area"), 1);
    map.insert("str_into".into(), 2);
    map.insert("atom_from".into(), 3);

    assert_eq!(map.get(&"area".into()).unwrap(), &1);
    assert_eq!(map.get(&"str_into".into()).unwrap(), &2);
    assert_eq!(map.get(&Atom::from("atom_from")).unwrap(), &3);
}
