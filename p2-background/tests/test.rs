use assert_cmd::Command;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid as NixPid;
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use std::time::Duration;
use sysinfo::{Pid, System};

#[test]
fn test_simple_script_completes() {
    // Test running a simple script that returns the number 5
    // This should complete quickly and exit normally
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd.arg("5").assert();
    // Should exit with success
    assert.success().stdout(predicates::str::contains("5"));
}

#[test]
fn test_external_process_with_interrupt() {
    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin(env!("CARGO_PKG_NAME")))
        .arg("^sleep 10; 5")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start background process");
    // Give some time for the sleep command to start
    thread::sleep(Duration::from_millis(1000));

    let our_pid = cmd.id();
    let pids = get_child_pids(our_pid);
    // Assert that we have at least one child process (the sleep command)
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
    sys.refresh_all();

    for pid in pids {
        let process_exists = sys.process(Pid::from_u32(pid)).is_some();
        assert!(
            !process_exists,
            "Child process {} should have been terminated",
            pid
        );
    }
}

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
