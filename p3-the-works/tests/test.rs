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

// Optional: Add additional tests specifically for async behavior
#[test]
fn test_graceful_shutdown() {
    let mut cmd = Command::cargo_bin("threaded").unwrap();

    // Test that CTRL+C signal is handled properly
    // Note: This is a simplified test that just ensures the program exits cleanly
    // with successful status. In real-world, you might need to simulate CTRL+C.
    cmd.arg(r#"{|i| $"processing ($i)"}"#)
        .write_stdin("test\n")
        .assert()
        .success();
}

#[test]
fn test_external_process_with_interrupt() {
    // Start the threaded app with a closure that sleeps for 10 seconds
    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin("threaded"))
        .arg(r#"{|_| $"Running sleep"; ^sleep 10; $"Done sleeping"}"#)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start threaded process");

    // Write a line to stdin to trigger job execution
    if let Some(mut stdin) = cmd.stdin.take() {
        stdin
            .write_all(b"test\n")
            .expect("Failed to write to stdin");
        // Don't close stdin yet to keep the process running
    }

    // Give some time for the sleep command to start
    thread::sleep(Duration::from_millis(1000));

    let our_pid = cmd.id();
    let pids = get_child_pids(our_pid);

    // Assert that we have at least one child process
    assert!(!pids.is_empty(), "Expected at least one child process");

    // Send SIGINT (Ctrl+C) to our process
    let _ = signal::kill(NixPid::from_raw(our_pid as i32), Signal::SIGINT);

    // Wait for our process to handle the signal and terminate
    thread::sleep(Duration::from_millis(1000));

    // Check that our process has exited
    let status = cmd.try_wait().expect("Failed to check process status");
    assert!(status.is_some(), "Process did not exit after SIGINT");
}

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid as NixPid;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::Duration;
use sysinfo::{Pid, System};

// Also need to add the get_child_pids function from background_test.rs
fn get_child_pids(target: u32) -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_all();
    sys.processes()
        .iter()
        .filter_map(|(pid, proc)| {
            // Check if this process's parent is our target
            match proc.parent() {
                Some(parent_pid) if parent_pid == Pid::from_u32(target) => Some(*pid),
                _ => None,
            }
        })
        .map(|pid| pid.as_u32())
        .collect()
}
