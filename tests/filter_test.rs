use std::process;

// ---------------------------------------------------------------------------
// PID filter tests (via binary invocation)
// ---------------------------------------------------------------------------

#[test]
fn test_pid_filter_include() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid, "-t"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = stdout.lines().collect();
    // Our own PID should appear in the output
    assert!(
        pids.iter().any(|line| line.trim() == my_pid),
        "Terse output should contain our PID {}. Got: {:?}",
        my_pid,
        pids,
    );
}

#[test]
fn test_pid_filter_exclude() {
    // Exclude PID 1 and verify it does not appear
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", "^1", "-t"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = stdout.lines().collect();
    assert!(
        !pids.iter().any(|line| line.trim() == "1"),
        "PID 1 should be excluded from output",
    );
}

// ---------------------------------------------------------------------------
// User filter tests
// ---------------------------------------------------------------------------

#[test]
fn test_user_filter() {
    let output = process::Command::new("cargo")
        .args(["run", "--", "-u", "root", "-t"])
        .output()
        .expect("Failed to run loof");

    // Should succeed (exit 0) even if no root processes are visible
    // The important thing is it doesn't crash
    assert!(
        output.status.success(),
        "loof -u root -t should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ---------------------------------------------------------------------------
// Command filter tests
// ---------------------------------------------------------------------------

#[test]
fn test_command_filter() {
    let output = process::Command::new("cargo")
        .args(["run", "--", "-c", "cargo", "-t"])
        .output()
        .expect("Failed to run loof");

    assert!(
        output.status.success(),
        "loof -c cargo -t should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ---------------------------------------------------------------------------
// Inet filter parsing (via binary, just ensure no crash)
// ---------------------------------------------------------------------------

#[test]
fn test_inet_filter_tcp() {
    let output = process::Command::new("cargo")
        .args(["run", "--", "-i", "TCP", "-t"])
        .output()
        .expect("Failed to run loof");

    assert!(
        output.status.success(),
        "loof -i TCP -t should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn test_inet_filter_empty() {
    let output = process::Command::new("cargo")
        .args(["run", "--", "-i", "-t"])
        .output()
        .expect("Failed to run loof");

    assert!(
        output.status.success(),
        "loof -i -t should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

// ---------------------------------------------------------------------------
// AND mode
// ---------------------------------------------------------------------------

#[test]
fn test_and_mode() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-a", "-p", &my_pid, "-u", "nonexistent_user_xyz", "-t"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // In AND mode, PID match AND user match must both be true.
    // Since user "nonexistent_user_xyz" won't match, output should be empty.
    assert!(
        stdout.trim().is_empty(),
        "AND mode with non-matching user should produce empty output. Got: {}",
        stdout,
    );
}

// ---------------------------------------------------------------------------
// Filter matching unit tests (these don't need the binary)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod unit_tests {
    // These are covered by the unit tests inside src/filter.rs
    // This section is a placeholder for any additional integration-level
    // filter tests that require the full binary.
}
