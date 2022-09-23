use clap::Parser;
use std::fs;
use subprocess::{Exec, Redirection};

// Compiles C to machine code
fn cc(input: &str, binary_name: &str) -> String {
    Exec::cmd("clang")
        .args(&["-O3", "-o", binary_name, "-xc", "-"])
        .stdin(input)
        .stdout(Redirection::Pipe)
        .capture()
        .unwrap()
        .stdout_str()
}

/// Brainfuck to c transpiler
#[derive(Parser, Debug)]
#[clap(name="rsbfc", author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file
    #[clap(value_parser)]
    file: String,

    /// Binary (output) path
    #[clap(value_parser, default_value = "a.out")]
    output: String,

    /// Output C code instead of compiling with gcc
    #[clap(short, long, value_parser)]
    code: bool,
}

fn main() {
    let args = Args::parse();
    let contents = fs::read_to_string(args.file).expect("Something went wrong reading the file");
    let tokens = rsbflib::tokenize(&contents);
    let optimized_tokens = rsbflib::optimize(tokens);
    let c_code = rsbflib::c_translate(optimized_tokens);
    if args.code {
        print!("{}", c_code);
    } else {
        print!("{}", cc(&c_code, &(args.output)));
    }
}
