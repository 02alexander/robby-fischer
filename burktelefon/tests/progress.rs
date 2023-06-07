#[test]
fn test() {
    let t = trybuild::TestCases::new();
    t.pass("tests/parse.rs");
}
