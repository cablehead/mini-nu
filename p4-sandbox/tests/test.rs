use assert_cmd::Command;
use predicates;

#[test]
fn external_commands_are_blocked() {
    // attempt to run an external process – should _fail_
    Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .arg("^ls")
        .assert()
        .failure()
        .stderr(predicates::str::contains("Compile error")); // error about external commands
}

#[test]
fn filter_command_still_works() {
    // a simple filters-only pipeline
    Command::cargo_bin(env!("CARGO_PKG_NAME"))
        .unwrap()
        .arg(r#"[1 2 3] | length"#)
        .assert()
        .success()
        .stdout(predicates::str::contains("3")); // prints 3
}
