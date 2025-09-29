use bevy::prelude::*;
use bevy::window::WindowResolution;
use clap::Parser;
use shekere::{Config, ShekerConfig, ShekerPlugin};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Input configuration file.
    #[arg(value_name = "FILE")]
    config_file: String,
}

fn main() {
    env_logger::init();

    println!("Starting shekere...");
    let args = Args::parse();
    println!("Parsed args: {:?}", args);

    let conf_str = std::fs::read_to_string(&args.config_file).unwrap();
    let conf: Config = toml::from_str(&conf_str).unwrap();
    let conf_dir = Path::new(&args.config_file).parent().unwrap().to_path_buf();

    println!("Configuration loaded successfully");
    println!("Window size: {}x{}", conf.window.width, conf.window.height);

    println!("Creating Bevy app...");

    App::new()
        .insert_resource(ShekerConfig {
            config: conf.clone(),
            config_dir: conf_dir
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "shekere".into(),
                    resolution: WindowResolution::new(
                        conf.window.width as f32,
                        conf.window.height as f32
                    ),
                    ..default()
                }),
                ..default()
            }),
            ShekerPlugin,
        ))
        .run();

    println!("App finished.");
}
