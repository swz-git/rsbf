use clap::Parser;
use nohash_hasher::NoHashHasher;
use rsbflib::{BracketState, TokenKind, TokenValue};
use std::{
    collections::HashMap,
    error::Error,
    fs,
    hash::BuildHasherDefault,
    io::{self, Write},
};

/// Brainfuck interpreter
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file
    #[clap(value_parser)]
    file: String,
}

fn generate_jumping_map(
    tokens: &Vec<rsbflib::Token>,
) -> Result<HashMap<usize, usize, BuildHasherDefault<NoHashHasher<usize>>>, Box<dyn Error>> {
    let mut map: HashMap<usize, usize, BuildHasherDefault<NoHashHasher<usize>>> =
        HashMap::with_hasher(BuildHasherDefault::default());
    let mut open_bracket_index_stack: Vec<usize> = vec![];
    for (i, token) in tokens.iter().enumerate() {
        match token.value {
            TokenValue::BracketState(BracketState::Open) => open_bracket_index_stack.push(i),
            TokenValue::BracketState(BracketState::Closed) => {
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
    // Ok(map
    //     .iter()
    //     .map(|x| (*x.0, x.1.expect("Opened bracket never closes")))
    //     .collect())
}

const MEM_SIZE: usize = 30000;

fn interpret(tokens: Vec<rsbflib::Token>) {
    let mut memory = [0isize; MEM_SIZE];
    let mut mempos: usize = 0;
    let mut pos: usize = 0;
    let mut loop_stack: Vec<usize> = vec![];

    let mut stdout = io::stdout();
    let mut temp_stdout_buf = [0u8; 1];

    let jumping_map = generate_jumping_map(&tokens).expect("Couldn't generate jumping tables");

    while tokens.len() > pos {
        let token = &tokens[pos];
        match &token.value {
            TokenValue::None => match token.kind {
                TokenKind::Output => {
                    (memory[mempos] as u8 as char).encode_utf8(&mut temp_stdout_buf);
                    stdout
                        .write(&temp_stdout_buf)
                        .expect("Couldn't write to stdout");
                    stdout.flush().expect("fuck you");
                }
                TokenKind::Input => {
                    todo!("input char (,)")
                }
                TokenKind::Clear => {
                    memory[mempos] = 0;
                }
                _ => panic!("Kind isn't of value None"),
            },
            TokenValue::Int(value) => match &token.kind {
                TokenKind::ValMod => {
                    memory[mempos] += value;
                }
                TokenKind::PosMod => {
                    // TODO: A lot of "as" here, maybe its slow?
                    mempos = (mempos as isize + value) as usize % MEM_SIZE;
                }
                _ => panic!("Kind isn't of value Int"),
            },
            TokenValue::BracketState(s) => match s {
                BracketState::Open => {
                    if memory[mempos] as u8 == 0 {
                        pos = *jumping_map.get(&pos).expect("Opened loop never closed");

                        // pos = find_correct_closing_bracket(pos, &tokens)
                        //     .expect("Opened loop never closed");
                    } else {
                        loop_stack.push(pos);
                    }
                }
                BracketState::Closed => {
                    if memory[mempos] as u8 != 0 {
                        pos = *loop_stack.last().expect("Closed loop never opened")
                    } else {
                        loop_stack.pop();
                    }
                }
            },
        }
        pos += 1;
    }
}

fn main() {
    let args = Args::parse();
    let contents = fs::read_to_string(args.file).expect("Something went wrong reading the file");
    let tokens = rsbflib::tokenize(&contents);
    let optimized_tokens = rsbflib::optimize(tokens);
    interpret(optimized_tokens);
}
