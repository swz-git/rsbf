use std::fs;

use clap::Parser;
use rsbflib;

/// Brainfuck to c transpiler
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file
    #[clap(value_parser)]
    file: String,
}

fn main() {
    let args = Args::parse();
    let contents = fs::read_to_string(args.file).expect("Something went wrong reading the file");
    let tokens = rsbflib::tokenize(&contents);
    let optimized_tokens = rsbflib::optimize(tokens);
    todo!("Interpretation")
}
