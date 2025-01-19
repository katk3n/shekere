use clap::Parser;
use kchfgt::run;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Input fragment shader file. Only wgsl is supported.
    #[arg(value_name = "FILE")]
    shader_file: String,

    /// Window width
    #[arg(long("width"), default_value = "1280")]
    width: u32,

    /// Window height
    #[arg(long("height"), default_value = "720")]
    height: u32,
}

fn main() {
    let args = Args::parse();
    pollster::block_on(run(&args.shader_file, args.width, args.height));
}
