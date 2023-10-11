use clap::Parser;
use std::fs;
use subprocess::{Exec, Redirection};

#[cfg(feature = "cranelift-compile")]
use {
    cranelift::prelude::isa, cranelift::prelude::settings,
    cranelift_object::ObjectBuilder, target_lexicon::Triple,
};

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

/// Brainfuck compiler
#[derive(Parser, Debug)]
#[clap(name="rsbfc", author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file
    #[clap(value_parser)]
    file: String,

    /// Binary (output) path
    #[clap(value_parser, default_value = "a.out")]
    output: String,

    /// Use custom cranelift codegen. Cranelift produces
    /// a slower binary than going through a C compiler does.
    /// BF -> CRANELIFT -> BINARY instead of BF -> C -> BINARY
    #[clap(short, long, value_parser)]
    cranelift: bool,

    /// Output C code instead of compiling
    #[clap(
        long,
        conflicts_with = "cranelift",
        conflicts_with = "output",
        value_parser
    )]
    code: bool,
}

fn main() {
    let args = Args::parse();
    let contents = fs::read_to_string(args.file)
        .expect("Something went wrong reading the file");
    let tokens = rsbflib::tokenize(&contents);
    let optimized_tokens = rsbflib::optimize(tokens);
    let c_code = rsbflib::c_translate(optimized_tokens);
    if args.code {
        print!("{}", c_code);
    } else if args.cranelift {
        #[cfg(feature = "cranelift-compile")]
        {
            // let builder = settings::builder();
            // let flags = settings::Flags::new(builder);
            // let isa = match isa::lookup(Triple::host()) {
            //     Err(_) => panic!("x86_64 ISA is not avaliable"),
            //     Ok(isa_builder) => isa_builder.finish(flags).unwrap(),
            // };

            // let obj_builder = ObjectBuilder::new(
            //     isa,
            //     "main",
            //     cranelift_module::default_libcall_names(),
            // )
            // .expect("obj_builder failed");

            // let object_file = obj_builder.finish().object.emit().unwrap();

            todo!("cranelift compiler")
        }

        #[cfg(not(feature = "cranelift-compile"))]
        {
            Err("Feature 'cranelift-compile' was not enabled at compile time")
                .unwrap()
        }
    } else {
        print!("{}", cc(&c_code, &(args.output)));
    }
}
