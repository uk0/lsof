use std::process;

#[test]
fn test_list_processes_not_empty() {
    // We can't directly use the platform module from integration tests easily,
    // so we test via the binary
    let output = process::Command::new("cargo")
        .args(["run", "--", "-t"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = stdout.lines().collect();
    assert!(!pids.is_empty(), "Process list should not be empty");
}

#[test]
fn test_pid_filter() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain the header + at least one process line
    assert!(stdout.contains(&my_pid), "Output should contain our PID");
}
