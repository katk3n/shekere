use clap::Parser;
use kchfgt::run;
use kchfgt::Config;
use std::path::Path;

use toml;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Input configuration file.
    #[arg(value_name = "FILE")]
    config_file: String,
}

fn main() {
    let args = Args::parse();
    let conf_str = std::fs::read_to_string(&args.config_file).unwrap();
    let conf: Config = toml::from_str(&conf_str).unwrap();
    let conf_dir = Path::new(&args.config_file).parent().unwrap();
    pollster::block_on(run(&conf, conf_dir));
}
