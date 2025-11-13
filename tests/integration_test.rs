use std::process::Command;

#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Frontend Runtime Container"));
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_version_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("frc"));
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_info_node_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "info", "node"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("System:"));
    assert!(stdout.contains("Recommendations for node"));
    assert!(stdout.contains("GB"));
}

#[test]
fn test_info_deno_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "info", "deno"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recommendations for deno"));
}

#[test]
fn test_info_bun_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "info", "bun"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Recommendations for bun"));
    assert!(stdout.contains("automatically"));
}

#[test]
fn test_project_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "project"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Project:"));
    assert!(stdout.contains("fe-run-container"));
}

#[test]
fn test_list_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "list"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Saved Project Configurations")
            || stdout.contains("No saved project configurations")
    );
}

#[test]
fn test_invalid_runtime() {
    let output = Command::new("cargo")
        .args(["run", "--", "info", "invalid-runtime"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Unknown runtime") || stderr.contains("Error"));
}

#[test]
fn test_node_with_invalid_memory() {
    let output = Command::new("cargo")
        .args(["run", "--", "-m", "999999", "info", "node"])
        .output()
        .expect("Failed to execute command");

    // Command should succeed but show warning/error
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Should either succeed with warning or fail with error about memory
    assert!(combined.contains("GB") || combined.contains("memory"));
}
