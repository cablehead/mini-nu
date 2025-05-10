use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_simple_command_execution() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Use raw string for Nushell command to avoid escaping quotes
    cmd.arg(r#"'P1: Hello World!' | str upcase"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("P1: HELLO WORLD!"));
}
