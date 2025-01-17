use clap::Parser;
use kchfgt::run;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Input fragment shader file. Only wgsl is supported
    #[arg(value_name = "FILE")]
    shader_file: String,
}

fn main() {
    let args = Args::parse();
    pollster::block_on(run(&args.shader_file));
}
