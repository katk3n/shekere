use std::process::Command;

/// Command-line argument validation and help text tests (Red phase)
/// These tests ensure proper CLI argument handling and user experience

#[test]
fn test_cli_help_flag_works() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test --help flag
    let help_output = Command::new("./target/debug/shekere-cli")
        .args(&["--help"])
        .output()
        .expect("Failed to run CLI with --help");

    assert!(help_output.status.success(),
        "CLI should successfully show help");

    let help_text = String::from_utf8_lossy(&help_output.stdout);

    // Verify help contains essential information
    assert!(help_text.contains("shekere"),
        "Help text should contain program name");
    assert!(help_text.contains("FILE") || help_text.contains("config"),
        "Help text should mention config file parameter");
    assert!(help_text.contains("USAGE") || help_text.contains("Usage"),
        "Help text should show usage information");

    println!("Help text:\n{}", help_text);
}

#[test]
fn test_cli_short_help_flag_works() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test -h flag
    let help_output = Command::new("./target/debug/shekere-cli")
        .args(&["-h"])
        .output()
        .expect("Failed to run CLI with -h");

    assert!(help_output.status.success(),
        "CLI should successfully show help with -h");

    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_text.contains("USAGE") || help_text.contains("Usage") || help_text.contains("shekere"),
        "Short help should show usage information");
}

#[test]
fn test_cli_version_flag_works() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test --version flag
    let version_output = Command::new("./target/debug/shekere-cli")
        .args(&["--version"])
        .output()
        .expect("Failed to run CLI with --version");

    assert!(version_output.status.success(),
        "CLI should successfully show version");

    let version_text = String::from_utf8_lossy(&version_output.stdout);
    assert!(version_text.contains("shekere-cli"),
        "Version text should contain binary name 'shekere-cli', got: {}", version_text);

    // Check version format (should contain version number)
    assert!(version_text.contains("0.13.0") || version_text.chars().any(|c| c.is_numeric()),
        "Version text should contain version number, got: {}", version_text);

    println!("Version text: {}", version_text);
}

#[test]
fn test_cli_short_version_flag_works() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test -V flag (clap default for version)
    let version_output = Command::new("./target/debug/shekere-cli")
        .args(&["-V"])
        .output()
        .expect("Failed to run CLI with -V");

    assert!(version_output.status.success(),
        "CLI should successfully show version with -V");

    let version_text = String::from_utf8_lossy(&version_output.stdout);
    assert!(version_text.contains("shekere-cli") || version_text.chars().any(|c| c.is_numeric()),
        "Short version should show version info");
}

#[test]
fn test_cli_rejects_invalid_flags() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test invalid flag
    let invalid_output = Command::new("./target/debug/shekere-cli")
        .args(&["--invalid-flag"])
        .output()
        .expect("Failed to run CLI with invalid flag");

    assert!(!invalid_output.status.success(),
        "CLI should fail with invalid flag");

    let stderr = String::from_utf8_lossy(&invalid_output.stderr);
    assert!(stderr.contains("unrecognized") || stderr.contains("unknown") || stderr.contains("unexpected"),
        "Error should indicate unrecognized flag, got: {}", stderr);
}

#[test]
fn test_cli_requires_config_file_argument() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test with no arguments
    let no_args_output = Command::new("./target/debug/shekere-cli")
        .output()
        .expect("Failed to run CLI with no arguments");

    // Should fail and show helpful error
    assert!(!no_args_output.status.success(),
        "CLI should fail when no config file is provided");

    let stderr = String::from_utf8_lossy(&no_args_output.stderr);
    assert!(stderr.contains("required") || stderr.contains("FILE") || stderr.contains("argument"),
        "Error should indicate missing required argument, got: {}", stderr);
}

#[test]
fn test_cli_accepts_valid_config_file_path() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Use an existing example config
    let config_path = "examples/basic/basic.toml";

    if std::path::Path::new(config_path).exists() {
        // Test with valid config path (use timeout to prevent GUI from hanging tests)
        let output = Command::new("timeout")
            .args(&["2s", "./target/debug/shekere-cli", config_path])
            .output()
            .expect("Failed to run CLI with valid config");

        let exit_code = output.status.code().unwrap_or(-1);

        // Accept either success (0) or timeout (124) for graphical apps
        assert!(exit_code == 0 || exit_code == 124,
            "CLI should either succeed or timeout with valid config, got exit code: {}. stderr: {}",
            exit_code, String::from_utf8_lossy(&output.stderr));
    } else {
        println!("Skipping valid config test - basic.toml not found");
    }
}

#[test]
fn test_cli_handles_multiple_arguments_properly() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test that CLI handles extra arguments appropriately
    let output = Command::new("./target/debug/shekere-cli")
        .args(&["config1.toml", "config2.toml"])
        .output()
        .expect("Failed to run CLI with multiple arguments");

    // Should either accept only the first one or show error about too many arguments
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unexpected") || stderr.contains("too many") || stderr.contains("extra"),
            "Should handle multiple arguments gracefully, got: {}", stderr);
    }
}

#[test]
fn test_cli_help_mentions_supported_features() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Get help text
    let help_output = Command::new("./target/debug/shekere-cli")
        .args(&["--help"])
        .output()
        .expect("Failed to get help");

    let help_text = String::from_utf8_lossy(&help_output.stdout);

    // Should mention what the tool does
    assert!(help_text.contains("config") || help_text.contains("shader") || help_text.contains("visual"),
        "Help should give some indication of what shekere does, got: {}", help_text);
}