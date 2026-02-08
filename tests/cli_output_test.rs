use std::process;

// ---------------------------------------------------------------------------
// Header output tests
// ---------------------------------------------------------------------------

#[test]
fn test_output_has_correct_headers() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("");

    // The header should contain the standard lsof column names
    assert!(first_line.contains("COMMAND"), "Header should contain COMMAND");
    assert!(first_line.contains("PID"), "Header should contain PID");
    assert!(first_line.contains("USER"), "Header should contain USER");
    assert!(first_line.contains("FD"), "Header should contain FD");
    assert!(first_line.contains("TYPE"), "Header should contain TYPE");
    assert!(first_line.contains("DEVICE"), "Header should contain DEVICE");
    assert!(first_line.contains("SIZE/OFF"), "Header should contain SIZE/OFF");
    assert!(first_line.contains("NODE"), "Header should contain NODE");
    assert!(first_line.contains("NAME"), "Header should contain NAME");
}

// ---------------------------------------------------------------------------
// Terse output tests
// ---------------------------------------------------------------------------

#[test]
fn test_terse_output_pids_only() {
    let output = process::Command::new("cargo")
        .args(["run", "--", "-t"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Every line should be a valid PID (numeric)
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        assert!(
            trimmed.parse::<u32>().is_ok(),
            "Terse output line should be a numeric PID, got: '{}'",
            trimmed,
        );
    }
}

#[test]
fn test_terse_output_not_empty() {
    let output = process::Command::new("cargo")
        .args(["run", "--", "-t"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(!pids.is_empty(), "Terse output should not be empty");
}

// ---------------------------------------------------------------------------
// Combined flag tests
// ---------------------------------------------------------------------------

#[test]
fn test_no_hostname_no_portname() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid, "-n", "-P"])
        .output()
        .expect("Failed to run loof");

    assert!(
        output.status.success(),
        "loof -p PID -n -P should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn test_list_uid_flag() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid, "-l"])
        .output()
        .expect("Failed to run loof");

    assert!(
        output.status.success(),
        "loof -p PID -l should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn test_show_ppid_flag() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid, "-R"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("");
    assert!(
        first_line.contains("PPID"),
        "Header should contain PPID when -R is used. Got: {}",
        first_line,
    );
}

// ---------------------------------------------------------------------------
// Field output tests
// ---------------------------------------------------------------------------

#[test]
fn test_field_output_mode() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid, "-F", "pcn"])
        .output()
        .expect("Failed to run loof");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Field output lines should start with field character tags
    let has_p_field = stdout.lines().any(|l| l.starts_with('p'));
    let has_c_field = stdout.lines().any(|l| l.starts_with('c'));
    assert!(
        has_p_field,
        "Field output should contain 'p' (PID) field. Got:\n{}",
        stdout,
    );
    assert!(
        has_c_field,
        "Field output should contain 'c' (command) field. Got:\n{}",
        stdout,
    );
}

// ---------------------------------------------------------------------------
// Plus-prefix flag tests (+D, +d, +c)
// ---------------------------------------------------------------------------

#[test]
fn test_cmd_width_flag() {
    let my_pid = process::id().to_string();
    let output = process::Command::new("cargo")
        .args(["run", "--", "-p", &my_pid, "+c", "15"])
        .output()
        .expect("Failed to run loof");

    assert!(
        output.status.success(),
        "loof -p PID +c 15 should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}
