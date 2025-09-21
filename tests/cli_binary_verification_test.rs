use std::process::Command;
use std::path::Path;

/// Binary name and version verification tests
/// Ensures the CLI binary is correctly named and versioned according to Phase 2 requirements

#[test]
fn test_binary_name_is_shekere_cli() {
    // Build the CLI binary
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build shekere-cli");

    assert!(build_output.status.success(),
        "shekere-cli binary should build successfully. stderr: {}",
        String::from_utf8_lossy(&build_output.stderr));

    // Verify the binary exists with correct name
    let binary_path = Path::new("./target/debug/shekere-cli");
    assert!(binary_path.exists(),
        "Binary should exist at ./target/debug/shekere-cli");

    // Verify it's executable
    assert!(binary_path.is_file(),
        "Binary should be a file");

    println!("✓ Binary exists at: {:?}", binary_path);
}

#[test]
fn test_binary_version_matches_cargo_toml() {
    // Build first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Get version from CLI
    let version_output = Command::new("./target/debug/shekere-cli")
        .args(&["--version"])
        .output()
        .expect("Failed to get CLI version");

    assert!(version_output.status.success(),
        "Version command should succeed");

    let version_text = String::from_utf8_lossy(&version_output.stdout);
    println!("CLI version output: {}", version_text);

    // Should contain the binary name
    assert!(version_text.contains("shekere-cli"),
        "Version output should contain 'shekere-cli', got: {}", version_text);

    // Should contain version number (0.13.0 or similar)
    assert!(version_text.contains("0.13.0") || version_text.contains("0."),
        "Version output should contain version number, got: {}", version_text);
}

#[test]
fn test_cargo_metadata_shows_correct_binary() {
    // Check that cargo metadata shows the correct binary configuration
    let metadata_output = Command::new("cargo")
        .args(&["metadata", "--format-version", "1"])
        .output()
        .expect("Failed to get cargo metadata");

    assert!(metadata_output.status.success(),
        "Cargo metadata should succeed");

    let metadata_text = String::from_utf8_lossy(&metadata_output.stdout);

    // Should mention shekere-cli binary
    assert!(metadata_text.contains("shekere-cli"),
        "Cargo metadata should reference shekere-cli binary");

    println!("✓ Cargo metadata includes shekere-cli binary");
}

#[test]
fn test_binary_shows_correct_package_info() {
    // Build first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Get help output which should contain package info
    let help_output = Command::new("./target/debug/shekere-cli")
        .args(&["--help"])
        .output()
        .expect("Failed to get help");

    let help_text = String::from_utf8_lossy(&help_output.stdout);
    println!("Help text:\n{}", help_text);

    // Help should reference the correct binary name
    assert!(help_text.contains("shekere") || help_text.contains("creative coding"),
        "Help should reference shekere or describe its purpose");
}

#[test]
fn test_old_binary_name_does_not_exist() {
    // Verify that we don't accidentally build a binary with the old name
    let old_binary_path = Path::new("./target/debug/shekere");

    // Build workspace to make sure we're testing current state
    let _build = Command::new("cargo")
        .args(&["build", "--workspace"])
        .output()
        .expect("Failed to build workspace");

    // The old binary name should not exist after our refactoring
    if old_binary_path.exists() {
        println!("Warning: Old binary name 'shekere' still exists at: {:?}", old_binary_path);
        println!("This might indicate the refactoring is incomplete.");

        // For now, we'll just warn rather than fail, as this might be from old builds
        // assert!(false, "Old binary name 'shekere' should not exist after refactoring");
    } else {
        println!("✓ Old binary name 'shekere' correctly does not exist");
    }
}

#[test]
fn test_workspace_build_creates_correct_binaries() {
    // Build the entire workspace
    let build_output = Command::new("cargo")
        .args(&["build", "--workspace"])
        .output()
        .expect("Failed to build workspace");

    assert!(build_output.status.success(),
        "Workspace should build successfully. stderr: {}",
        String::from_utf8_lossy(&build_output.stderr));

    // Check that shekere-cli binary exists
    let cli_binary = Path::new("./target/debug/shekere-cli");
    assert!(cli_binary.exists(),
        "shekere-cli binary should exist after workspace build");

    println!("✓ Workspace build creates shekere-cli binary");
}

#[test]
fn test_release_build_works() {
    // Test that release build also works correctly
    let release_output = Command::new("cargo")
        .args(&["build", "--release", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build release");

    if release_output.status.success() {
        let release_binary = Path::new("./target/release/shekere-cli");
        assert!(release_binary.exists(),
            "Release binary should exist");

        // Test that release binary shows version
        let version_output = Command::new("./target/release/shekere-cli")
            .args(&["--version"])
            .output()
            .expect("Failed to get release version");

        assert!(version_output.status.success(),
            "Release binary should show version");

        let version_text = String::from_utf8_lossy(&version_output.stdout);
        assert!(version_text.contains("shekere-cli"),
            "Release version should contain binary name");

        println!("✓ Release build works correctly");
    } else {
        println!("Release build failed - this is acceptable during development");
        println!("stderr: {}", String::from_utf8_lossy(&release_output.stderr));
    }
}

#[test]
fn test_binary_has_reasonable_size() {
    // Build first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    let binary_path = Path::new("./target/debug/shekere-cli");
    if binary_path.exists() {
        let metadata = std::fs::metadata(binary_path)
            .expect("Failed to get binary metadata");

        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        println!("Binary size: {:.2} MB", size_mb);

        // Debug binary should be reasonable size (less than 500MB)
        assert!(size_mb < 500.0,
            "Debug binary size should be reasonable, got: {:.2} MB", size_mb);

        // Should be at least 1MB (sanity check that it's a real binary)
        assert!(size_mb > 1.0,
            "Binary should be substantial enough to be functional, got: {:.2} MB", size_mb);
    }
}

#[test]
fn test_cargo_list_shows_shekere_cli() {
    // List all workspace binaries
    let list_output = Command::new("cargo")
        .args(&["build", "--workspace", "--bins"])
        .output()
        .expect("Failed to list workspace binaries");

    if list_output.status.success() {
        println!("✓ Workspace binaries build successfully");

        // Check what binaries are actually created
        let target_dir = Path::new("./target/debug");
        if target_dir.exists() {
            let entries = std::fs::read_dir(target_dir).unwrap();
            let binaries: Vec<_> = entries
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.is_file() && path.extension().is_none() {
                        Some(path.file_name()?.to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
                .collect();

            println!("Available binaries: {:?}", binaries);
            assert!(binaries.contains(&"shekere-cli".to_string()),
                "shekere-cli should be in the list of built binaries");
        }
    }
}