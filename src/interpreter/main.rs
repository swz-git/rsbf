use clap::Parser;
use nohash_hasher::NoHashHasher;
use rsbflib::{BracketState, TokenKind};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    hash::BuildHasherDefault,
    io::{self, Write},
    path::PathBuf,
};

#[cfg(feature = "jit")]
use rsbflib::codegen;

fn generate_jumping_map(
    tokens: &Vec<rsbflib::Token>,
) -> Result<
    HashMap<usize, usize, BuildHasherDefault<NoHashHasher<usize>>>,
    Box<dyn Error>,
> {
    let mut map: HashMap<
        usize,
        usize,
        BuildHasherDefault<NoHashHasher<usize>>,
    > = HashMap::with_capacity_and_hasher(
        tokens
            .iter()
            .filter(|x| x.kind == TokenKind::Bracket(BracketState::Open))
            .count(),
        BuildHasherDefault::default(),
    );
    let mut open_bracket_index_stack: Vec<usize> = vec![];
    for (i, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Bracket(BracketState::Open) => {
                open_bracket_index_stack.push(i)
            }
            TokenKind::Bracket(BracketState::Closed) => {
                map.insert(
                    open_bracket_index_stack
                        .pop()
                        .ok_or("Too many closing brackets")?,
                    i,
                );
            }
            _ => { /* We don't care */ }
        }
    }

    if open_bracket_index_stack.len() > 0 {
        Err("Too many opening brackets")?
    }

    Ok(map)
}

const MEM_SIZE: usize = 30000;

fn interpret(tokens: Vec<rsbflib::Token>) {
    let mut memory = [0u8; MEM_SIZE];
    let mut mempos: usize = 0;
    let mut pos: usize = 0;
    let mut loop_stack: Vec<usize> = vec![];

    let mut stdout = io::stdout();

    let mut temp_stdio_buf = [0u8; 1];

    let jumping_map = generate_jumping_map(&tokens)
        .expect("Couldn't generate jumping tables");

    while tokens.len() > pos {
        let token = &tokens[pos];
        match &token.kind {
            TokenKind::Output => {
                (memory[mempos] as u8 as char).encode_utf8(&mut temp_stdio_buf);
                stdout
                    .write(&temp_stdio_buf)
                    .expect("Couldn't write to stdout");
                // updates stdout per char but is much slower in a slow terminal
                // stdout.flush().expect("Couldn't flush stdout");
            }
            TokenKind::Input => {
                todo!("input char (,)")
            }
            TokenKind::Clear => {
                memory[mempos] = 0;
            }
            TokenKind::ValMod(value) => {
                memory[mempos] = memory[mempos].wrapping_add(*value as u8);
            }
            TokenKind::PosMod(value) => {
                mempos = mempos.wrapping_add(*value as usize);
                if mempos >= MEM_SIZE {
                    mempos %= MEM_SIZE
                }
            }
            TokenKind::Bracket(BracketState::Open) => {
                if memory[mempos] == 0 {
                    pos = *jumping_map
                        .get(&pos)
                        .expect("Opened loop never closed");
                } else {
                    loop_stack.push(pos);
                }
            }
            TokenKind::Bracket(BracketState::Closed) => {
                if memory[mempos] != 0 {
                    pos = *loop_stack.last().expect("Closed loop never opened")
                } else {
                    loop_stack.pop();
                }
            }
            TokenKind::Copy(offset) => {
                let x = mempos.wrapping_add(*offset as usize);
                // this if statement slows it down so much, and since the bug
                // is extremely rare, this code is commented
                // if x >= MEM_SIZE {
                //     x %= MEM_SIZE
                // }
                memory[x] += memory[mempos];
            }
            TokenKind::Comment => {}
        }
        pos += 1;
    }
}

#[cfg(feature = "jit")]
fn run_bytecode(code: Vec<u8>) -> std::io::Result<()> {
    let mut memory = [0u8; MEM_SIZE];
    let mut buffer = memmap2::MmapOptions::new()
        .len(code.len())
        .map_anon()
        .unwrap();

    buffer.copy_from_slice(code.as_slice());

    let buffer = buffer.make_exec().unwrap();

    unsafe {
        let code_fn: unsafe extern "C" fn(*mut u8) -> *mut std::io::Error =
            std::mem::transmute(buffer.as_ptr());

        let error = code_fn(memory.as_mut_ptr());

        if !error.is_null() {
            return Err(*Box::from_raw(error));
        }
    }

    Ok(())
}

/// Brainfuck interpreter
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file
    #[clap(value_parser)]
    file: PathBuf,

    /// JIT code instead of interpreting
    #[clap(short, long, value_parser)]
    jit: bool,
}

fn main() {
    let args = Args::parse();
    let contents = fs::read_to_string(args.file)
        .expect("Something went wrong reading the file");
    let tokens = rsbflib::tokenize(&contents);
    let optimized_tokens = rsbflib::optimize(tokens);

    if args.jit {
        #[cfg(feature = "jit")]
        {
            let bytecode = codegen::compile(optimized_tokens)
                .expect("JIT compilation failed");
            run_bytecode(bytecode).expect("Couldn't run bytecode");
        }

        #[cfg(not(feature = "jit"))]
        Err("Feature 'jit' was not enabled at compile time").unwrap()
    } else {
        interpret(optimized_tokens);
    }
}
