use clap::Parser;
use kchfg::run;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Input shader file
    #[arg(value_name = "FILE")]
    shader_file: String,
}

fn main() {
    let args = Args::parse();
    println!("{args:#?}");
    pollster::block_on(run(&args.shader_file));
}
