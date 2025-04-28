use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_threaded_concurrency() {
    let mut cmd = Command::cargo_bin("threaded").unwrap();

    // Simple closure that parses input as milliseconds to sleep
    cmd.arg(r#"{|i| let dur = $in + "ms"; sleep ($dur | into duration); $"finished job ($i) after ($dur)"}"#)
        .write_stdin("300\n200\n100\n")
        .assert()
        .success()
        // Results should be in order of completion time, not input order
        .stdout(predicate::str::contains("finished job 2 after 100ms").count(1))
        .stdout(predicate::str::contains("finished job 1 after 200ms").count(1))
        .stdout(predicate::str::contains("finished job 0 after 300ms").count(1));
}
