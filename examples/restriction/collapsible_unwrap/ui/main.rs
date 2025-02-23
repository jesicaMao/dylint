// run-rustfix

use std::fs;

const PATH: &str = "Cargo.toml";

#[expect(unused_variables)]
fn main() {
    assert!(
        std::path::Path::new(PATH)
            .canonicalize()
            .unwrap()
            .try_exists()
            .unwrap()
    );

    assert!(
        std::path::Path::new(PATH)
            .canonicalize()
            .unwrap()
            .try_exists()
            .expect("could not determine whether path exists")
    );

    // Should not lint, i.e., should not try to collapse an `expect`
    assert!(
        std::path::Path::new(PATH)
            .canonicalize()
            .expect("could not canonicalize path")
            .try_exists()
            .unwrap()
    );

    // Should not lint, error types differ
    let toml = fs::read_to_string(PATH)
        .unwrap()
        .parse::<toml::Value>()
        .unwrap();

    let name = toml
        .as_table()
        .unwrap()
        .get("package")
        .unwrap()
        .as_table()
        .unwrap()
        .get("name")
        .unwrap();

    let name = toml
        .as_table()
        .unwrap()
        .get("package")
        .unwrap()
        .as_table()
        .and_then(|map| map.get("name"))
        .unwrap();

    let name = toml
        .as_table()
        .unwrap()
        .get("package")
        .and_then(|value| value.as_table())
        .unwrap()
        .get("name")
        .unwrap();

    let name = toml
        .as_table()
        .and_then(|map| map.get("package"))
        .unwrap()
        .as_table()
        .unwrap()
        .get("name")
        .unwrap();

    println!("{:?}", name);
}

#[expect(dead_code)]
fn mut_test() {
    let _ = fs::read_dir(".").unwrap().next().unwrap();
}
