use string_cache_codegen;

#[test]
fn test_iteration_order() {

    let x1 = string_cache_codegen::AtomType::new("foo::Atom", "foo_atom!")
        .atoms(&["x", "xlink", "svg", "test"])
        .write_to_string(Vec::new()).unwrap();
    
    let x2 = string_cache_codegen::AtomType::new("foo::Atom", "foo_atom!")
        .atoms(&["x", "xlink", "svg", "test"])
        .write_to_string(Vec::new()).unwrap();

    assert_eq!(x1, x2);
}