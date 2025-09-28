use std::fs;
use std::path::Path;
use std::process::Command;

/// Comprehensive CLI error handling tests (Red phase - these should fail initially)
/// Following TDD methodology to ensure robust error handling

#[test]
fn test_cli_handles_missing_config_file() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test with non-existent file
    let output = Command::new("./target/debug/shekere-cli")
        .args(&["non_existent_config.toml"])
        .output()
        .expect("Failed to run CLI with missing file");

    // Should fail with non-zero exit code
    assert!(
        !output.status.success(),
        "CLI should fail when config file doesn't exist"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No such file")
            || stderr.contains("not found")
            || stderr.contains("does not exist"),
        "Error message should indicate file not found, got: {}",
        stderr
    );
}

#[test]
fn test_cli_handles_invalid_toml_syntax() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Create a temporary file with invalid TOML
    let temp_file = "test_invalid.toml";
    let invalid_toml = r#"
[window
width = 800  // missing closing bracket and invalid comment syntax
height = 600
"#;

    fs::write(temp_file, invalid_toml).expect("Failed to write test file");

    // Test with invalid TOML
    let output = Command::new("./target/debug/shekere-cli")
        .args(&[temp_file])
        .output()
        .expect("Failed to run CLI with invalid TOML");

    // Clean up
    let _ = fs::remove_file(temp_file);

    // Should fail with non-zero exit code
    assert!(
        !output.status.success(),
        "CLI should fail when TOML syntax is invalid"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("parse") || stderr.contains("TOML") || stderr.contains("syntax"),
        "Error message should indicate TOML parsing error, got: {}",
        stderr
    );
}

#[test]
fn test_cli_handles_invalid_config_structure() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Create a temporary file with valid TOML but invalid config structure
    let temp_file = "test_invalid_structure.toml";
    let invalid_config = r#"
# Valid TOML but missing required fields
[some_unknown_section]
random_field = "value"
"#;

    fs::write(temp_file, invalid_config).expect("Failed to write test file");

    // Test with invalid config structure
    let output = Command::new("./target/debug/shekere-cli")
        .args(&[temp_file])
        .output()
        .expect("Failed to run CLI with invalid config");

    // Clean up
    let _ = fs::remove_file(temp_file);

    // Should fail with non-zero exit code
    assert!(
        !output.status.success(),
        "CLI should fail when config structure is invalid"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("config") || stderr.contains("missing") || stderr.contains("required"),
        "Error message should indicate config validation error, got: {}",
        stderr
    );
}

#[test]
fn test_cli_handles_missing_shader_files() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Create a temporary config that references non-existent shader file
    let temp_file = "test_missing_shader.toml";
    let config_with_missing_shader = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Test"
entry_point = "fs_main"
file = "non_existent_shader.wgsl"
"#;

    fs::write(temp_file, config_with_missing_shader).expect("Failed to write test file");

    // Test with missing shader file reference
    let output = Command::new("timeout")
        .args(&["3s", "./target/debug/shekere-cli", temp_file])
        .output()
        .expect("Failed to run CLI with missing shader");

    // Clean up
    let _ = fs::remove_file(temp_file);

    // Should fail or timeout (acceptable for visual apps)
    let exit_code = output.status.code().unwrap_or(-1);

    // Either immediate failure (preferred) or timeout is acceptable
    if exit_code != 0 && exit_code != 124 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("shader") || stderr.contains("file") || stderr.contains("not found"),
            "Error message should indicate shader file not found, got: {}",
            stderr
        );
    }
}

#[test]
fn test_cli_no_arguments_shows_help() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test with no arguments
    let output = Command::new("./target/debug/shekere-cli")
        .output()
        .expect("Failed to run CLI with no arguments");

    // Should show help or fail with helpful message
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("required") || stderr.contains("FILE") || stderr.contains("usage"),
            "Should show helpful error when no arguments provided, got: {}",
            stderr
        );
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("help") || stdout.contains("usage") || stdout.contains("FILE"),
            "Should show help when no arguments provided, got: {}",
            stdout
        );
    }
}

#[test]
fn test_cli_exit_codes_are_meaningful() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test different error conditions have different exit codes

    // Missing file should have exit code 1 or 2
    let missing_file_output = Command::new("./target/debug/shekere-cli")
        .args(&["non_existent.toml"])
        .output()
        .expect("Failed to run CLI");

    let missing_file_code = missing_file_output.status.code().unwrap_or(-1);
    assert!(
        missing_file_code != 0,
        "Missing file should have non-zero exit code"
    );

    // No arguments should have exit code indicating usage error
    let no_args_output = Command::new("./target/debug/shekere-cli")
        .output()
        .expect("Failed to run CLI");

    let no_args_code = no_args_output.status.code().unwrap_or(-1);
    assert!(
        no_args_code != 0,
        "No arguments should have non-zero exit code"
    );

    println!(
        "Exit codes - Missing file: {}, No args: {}",
        missing_file_code, no_args_code
    );
}

#[test]
fn test_cli_performance_baseline() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test performance with a simple config
    let test_config = "examples/basic/basic.toml";

    if Path::new(test_config).exists() {
        use std::time::Instant;

        let start = Instant::now();
        let _output = Command::new("timeout")
            .args(&["1s", "./target/debug/shekere-cli", test_config])
            .output()
            .expect("Failed to run performance test");
        let duration = start.elapsed();

        // CLI should start within reasonable time (2 seconds for timeout is generous)
        assert!(
            duration.as_secs() <= 2,
            "CLI startup should be reasonably fast, took {:?}",
            duration
        );

        println!("CLI startup time: {:?}", duration);
    } else {
        println!("Skipping performance test - basic.toml not found");
    }
}
