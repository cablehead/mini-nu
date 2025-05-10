use assert_cmd::Command;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid as NixPid;
use predicates::prelude::*;
use std::io::Write;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::Duration;
use sysinfo::{Pid, System};

#[test]
fn test_concurrent_closures_execution_order() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // A Nushell closure that processes each line of input with variable delays
    cmd.arg(r#"{|i| let dur = $in + "ms"; sleep ($dur | into duration); $"job ($i) processed in ($dur)"}"#)
        .write_stdin("300\n200\n100\n") // Corresponds to job 0, 1, 2
        .assert()
        .success()
        // Results should be in order of completion time, not input order
        // Job 2 (100ms) finishes first, then Job 1 (200ms), then Job 0 (300ms)
        .stdout(predicate::str::contains("Thread 2: job 2 processed in 100ms").count(1))
        .stdout(predicate::str::contains("Thread 1: job 1 processed in 200ms").count(1))
        .stdout(predicate::str::contains("Thread 0: job 0 processed in 300ms").count(1));
}

#[test]
fn test_custom_command_execution() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    // Execute a closure that calls our custom 'warble' command
    cmd.arg(r#"{|_| warble}"#)
        .write_stdin("trigger_job\n") // Send input to create a single job in the job queue
        .assert()
        .success()
        .stdout(predicate::str::contains("Thread 0: warble, oh my"));
}

#[test]
fn test_concurrent_external_process_interrupt() {
    // Start the multithreaded engine with a long-running external process to test signal handling
    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin(env!("CARGO_PKG_NAME")))
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

    // Verify all child processes have been terminated
    let mut sys = System::new();
    sys.refresh_all(); // Refresh process list again

    for pid_val in pids {
        let process_exists = sys.process(Pid::from_u32(pid_val)).is_some();
        assert!(
            !process_exists,
            "Child process {} should have been terminated",
            pid_val
        );
    }
}

// Also need to add the get_child_pids function from background_test.rs
fn get_child_pids(target_pid_val: u32) -> Vec<u32> {
    let mut sys = System::new();
    sys.refresh_all();
    let target_sys_pid = Pid::from_u32(target_pid_val);
    sys.processes()
        .iter()
        .filter_map(|(pid, proc)| {
            // Check if this process's parent is our target
            match proc.parent() {
                Some(parent_pid) if parent_pid == target_sys_pid => Some(pid.as_u32()),
                _ => None,
            }
        })
        .collect()
}
