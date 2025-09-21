use clap::Parser;
use shekere_core::{Config, run};
use std::panic;
use std::path::Path;
use std::process;

#[derive(Debug, Parser)]
#[command(
    name = "shekere-cli",
    version = env!("CARGO_PKG_VERSION"),
    about = "shekere - Creative coding tool for real-time visual effects with WebGPU shaders and audio integration",
    long_about = "shekere is a creative coding tool that combines WebGPU-based fragment shaders with audio integration (OSC and spectrum analysis). It creates real-time visual effects driven by sound and user interaction."
)]
struct Args {
    /// Path to the TOML configuration file
    #[arg(
        value_name = "FILE",
        help = "TOML configuration file specifying shaders, audio settings, and window properties"
    )]
    config_file: String,
}

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Validate and process the configuration file
    if let Err(exit_code) = run_with_error_handling(&args.config_file) {
        process::exit(exit_code);
    }
}

fn run_with_error_handling(config_path: &str) -> Result<(), i32> {
    // Check if config file exists
    let config_file_path = Path::new(config_path);
    if !config_file_path.exists() {
        eprintln!(
            "Error: Configuration file '{}' does not exist.",
            config_path
        );
        eprintln!("Please provide a valid TOML configuration file.");
        eprintln!("Example: shekere-cli examples/basic/basic.toml");
        return Err(1);
    }

    // Check if it's actually a file (not a directory)
    if !config_file_path.is_file() {
        eprintln!("Error: '{}' is not a file.", config_path);
        eprintln!("Please provide a valid TOML configuration file.");
        return Err(1);
    }

    // Read the configuration file
    let conf_str = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!(
                "Error: Failed to read configuration file '{}'.",
                config_path
            );
            eprintln!("Reason: {}", err);
            return Err(2);
        }
    };

    // Parse the TOML configuration
    let conf: Config = match toml::from_str(&conf_str) {
        Ok(config) => config,
        Err(err) => {
            eprintln!(
                "Error: Failed to parse TOML configuration file '{}'.",
                config_path
            );
            eprintln!("TOML parsing error: {}", err);
            eprintln!();
            eprintln!("Please check your TOML syntax. Common issues:");
            eprintln!("  - Missing closing brackets ]");
            eprintln!("  - Invalid comment syntax (use # not //)");
            eprintln!("  - Incorrect quotation marks");
            eprintln!("  - Missing required fields");
            return Err(3);
        }
    };

    // Get the directory containing the config file
    let conf_dir = match config_file_path.parent() {
        Some(dir) => dir,
        None => {
            eprintln!(
                "Error: Could not determine directory for configuration file '{}'.",
                config_path
            );
            return Err(4);
        }
    };

    // Validate shader file references (basic check)
    if let Err(validation_error) = validate_shader_files(&conf, conf_dir) {
        eprintln!("Error: Configuration validation failed.");
        eprintln!("{}", validation_error);
        return Err(5);
    }

    // Run the application
    match panic::catch_unwind(|| pollster::block_on(run(&conf, conf_dir))) {
        Ok(_) => {
            println!("shekere completed successfully.");
            Ok(())
        }
        Err(_) => {
            eprintln!("Error: Application crashed during execution.");
            eprintln!("This might be due to:");
            eprintln!("  - Graphics driver issues");
            eprintln!("  - Missing shader files");
            eprintln!("  - Audio device problems");
            eprintln!("  - Invalid shader code");
            Err(6)
        }
    }
}

/// Basic validation of shader file references in the configuration
fn validate_shader_files(config: &Config, base_dir: &Path) -> Result<(), String> {
    // Check if any pipeline configurations reference non-existent shader files
    for pipeline in &config.pipeline {
        let shader_path = base_dir.join(&pipeline.file);
        if !shader_path.exists() {
            return Err(format!(
                "Shader file '{}' referenced in pipeline '{}' does not exist.\n\
                Expected location: {}\n\
                Please ensure the shader file exists or update the configuration.",
                pipeline.file,
                pipeline.label,
                shader_path.display()
            ));
        }
    }
    Ok(())
}
