use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_basic_string_upcase() {
    let mut cmd = Command::cargo_bin("basic").unwrap();
    
    cmd.arg(r#""Hello from the basic package!" | str upcase"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("HELLO FROM THE BASIC PACKAGE!"));
}