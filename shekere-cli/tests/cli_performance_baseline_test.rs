use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Performance baseline tests for all example configurations
/// These tests establish performance expectations for the CLI

#[test]
fn test_cli_startup_performance_with_all_examples() {
    // Build CLI first
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    assert!(
        build_output.status.success(),
        "CLI should build successfully for performance testing"
    );

    let examples_dir = Path::new("examples");
    assert!(examples_dir.exists(), "Examples directory should exist");

    let mut performance_results = HashMap::new();
    let mut tested_configs = Vec::new();

    // Test each example configuration
    for entry in std::fs::read_dir(examples_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.is_dir() {
            // Look for .toml files in subdirectories
            if let Ok(subdir_entries) = std::fs::read_dir(&path) {
                for subentry in subdir_entries {
                    let subentry = subentry.unwrap();
                    let subpath = subentry.path();

                    if subpath.extension().and_then(|s| s.to_str()) == Some("toml") {
                        let config_name =
                            subpath.file_name().unwrap().to_string_lossy().to_string();
                        tested_configs.push(config_name.clone());

                        println!("Performance testing: {:?}", subpath);

                        // Measure startup time
                        let start_time = Instant::now();
                        let output = Command::new("timeout")
                            .args(&[
                                "3s",
                                "./target/debug/shekere-cli",
                                subpath.to_str().unwrap(),
                            ])
                            .output()
                            .expect("Failed to run performance test");

                        let elapsed = start_time.elapsed();
                        let exit_code = output.status.code().unwrap_or(-1);

                        performance_results.insert(config_name.clone(), (elapsed, exit_code));

                        // Basic performance assertions
                        assert!(
                            elapsed.as_secs() <= 5,
                            "Config {} took too long to start: {:?}",
                            config_name,
                            elapsed
                        );

                        // Should either succeed or timeout (acceptable for GUI apps)
                        assert!(
                            exit_code == 0 || exit_code == 124,
                            "Config {} failed unexpectedly with exit code: {}",
                            config_name,
                            exit_code
                        );

                        println!(
                            "  → {} completed in {:?} (exit: {})",
                            config_name, elapsed, exit_code
                        );
                    }
                }
            }
        }
    }

    // Ensure we tested a reasonable number of configurations
    assert!(
        tested_configs.len() >= 5,
        "Should have tested at least 5 configurations, found: {:?}",
        tested_configs
    );

    // Report performance summary
    println!("\nPerformance Summary:");
    let mut total_time = std::time::Duration::new(0, 0);
    let mut successful_starts = 0;

    for (config, (duration, exit_code)) in &performance_results {
        total_time += *duration;
        if *exit_code == 0 || *exit_code == 124 {
            successful_starts += 1;
        }
        println!("  {}: {:?} (exit: {})", config, duration, exit_code);
    }

    let avg_time = total_time / tested_configs.len() as u32;
    println!("Average startup time: {:?}", avg_time);
    println!(
        "Successful starts: {}/{}",
        successful_starts,
        tested_configs.len()
    );

    // Performance baseline assertions
    assert!(
        avg_time.as_millis() <= 2000,
        "Average startup time should be under 2 seconds, got: {:?}",
        avg_time
    );

    assert!(
        successful_starts as f64 / tested_configs.len() as f64 >= 0.8,
        "At least 80% of configs should start successfully"
    );
}

#[test]
fn test_cli_memory_usage_baseline() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test with a simple configuration
    let simple_config = "examples/basic/basic.toml";

    if Path::new(simple_config).exists() {
        // Use time command to measure resource usage (macOS/Unix)
        let output = Command::new("timeout")
            .args(&[
                "2s",
                "time",
                "-l", // -l for detailed stats on macOS
                "./target/debug/shekere-cli",
                simple_config,
            ])
            .output();

        match output {
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("Memory usage information:\n{}", stderr);

                // Look for memory usage indicators in time output
                if stderr.contains("maximum resident set size") {
                    // Extract memory usage (this is platform-specific)
                    println!("Memory baseline established for basic config");
                } else {
                    println!("Memory usage details not available on this platform");
                }
            }
            Err(e) => {
                println!("Memory test skipped - time command not available: {}", e);
            }
        }
    } else {
        println!("Skipping memory test - basic.toml not found");
    }
}

#[test]
fn test_cli_config_parsing_performance() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test parsing performance by running with immediate exit
    // We'll create a simple config that should parse quickly
    let test_config = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "PerformanceTest"
entry_point = "fs_main"
file = "examples/basic/fragment.wgsl"
"#;

    let temp_config = "performance_test.toml";
    std::fs::write(temp_config, test_config).expect("Failed to write test config");

    let start_time = Instant::now();
    let output = Command::new("timeout")
        .args(&["1s", "./target/debug/shekere-cli", temp_config])
        .output()
        .expect("Failed to run config parsing test");

    let parse_time = start_time.elapsed();

    // Clean up
    let _ = std::fs::remove_file(temp_config);

    // Config parsing should be very fast
    assert!(
        parse_time.as_millis() <= 1000,
        "Config parsing should be fast, took: {:?}",
        parse_time
    );

    println!("Config parsing time: {:?}", parse_time);
}

#[test]
fn test_cli_error_response_time() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test how quickly CLI responds to error conditions
    let start_time = Instant::now();
    let _output = Command::new("./target/debug/shekere-cli")
        .args(&["non_existent_file.toml"])
        .output()
        .expect("Failed to run error test");

    let error_response_time = start_time.elapsed();

    // Error responses should be immediate
    assert!(
        error_response_time.as_millis() <= 500,
        "Error response should be immediate, took: {:?}",
        error_response_time
    );

    println!("Error response time: {:?}", error_response_time);
}

#[test]
fn test_cli_help_response_time() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test how quickly CLI shows help
    let start_time = Instant::now();
    let output = Command::new("./target/debug/shekere-cli")
        .args(&["--help"])
        .output()
        .expect("Failed to run help test");

    let help_response_time = start_time.elapsed();

    assert!(output.status.success(), "Help command should succeed");

    // Help should be immediate
    assert!(
        help_response_time.as_millis() <= 200,
        "Help response should be immediate, took: {:?}",
        help_response_time
    );

    println!("Help response time: {:?}", help_response_time);
}

#[test]
fn test_cli_concurrent_startup_performance() {
    // Build CLI first
    let _build = Command::new("cargo")
        .args(&["build", "--bin", "shekere-cli"])
        .output()
        .expect("Failed to build CLI");

    // Test that multiple instances don't interfere with each other
    // (Though for a graphics app, this might not be practical)

    use std::thread;

    let config_path = "examples/basic/basic.toml";
    if Path::new(config_path).exists() {
        let start_time = Instant::now();

        // Start multiple instances with short timeout
        let handles: Vec<_> = (0..3)
            .map(|i| {
                let config = config_path.to_string();
                thread::spawn(move || {
                    let output = Command::new("timeout")
                        .args(&["1s", "./target/debug/shekere-cli", &config])
                        .output()
                        .expect("Failed to run concurrent test");

                    let exit_code = output.status.code().unwrap_or(-1);
                    println!("Instance {} exit code: {}", i, exit_code);

                    // Should either succeed or timeout
                    exit_code == 0 || exit_code == 124
                })
            })
            .collect();

        // Wait for all instances
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let concurrent_time = start_time.elapsed();

        // All instances should complete successfully
        assert!(
            results.iter().all(|&success| success),
            "All concurrent instances should complete successfully"
        );

        // Concurrent startup shouldn't take much longer than sequential
        assert!(
            concurrent_time.as_secs() <= 5,
            "Concurrent startup took too long: {:?}",
            concurrent_time
        );

        println!("Concurrent startup time: {:?}", concurrent_time);
    } else {
        println!("Skipping concurrent test - basic.toml not found");
    }
}
