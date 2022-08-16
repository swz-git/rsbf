use clap::Parser;
use rsbflib::{BracketState, TokenKind, TokenValue};
use std::fs;

/// Brainfuck interpreter
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file
    #[clap(value_parser)]
    file: String,
}

fn find_correct_closing_bracket(startpos: usize, tokens: &Vec<rsbflib::Token>) -> Option<usize> {
    let mut pos = startpos + 1;
    let mut non_closed_openings = 0;
    while pos < tokens.len() {
        let is_found = match &tokens[pos].value {
            TokenValue::BracketState(s) => match s {
                BracketState::Closed => {
                    if non_closed_openings != 0 {
                        non_closed_openings -= 1;
                        false
                    } else {
                        true
                    }
                }
                BracketState::Open => {
                    non_closed_openings += 1;
                    false
                }
            },
            _ => false,
        };
        if is_found {
            return Some(pos);
        }
        pos += 1;
    }
    if pos < tokens.len() {
        Some(pos)
    } else {
        None
    }
}

const MEM_SIZE: usize = 30000;

fn interpret(tokens: Vec<rsbflib::Token>) {
    let mut memory = [0isize; MEM_SIZE];
    let mut mempos: usize = 0;
    let mut pos: usize = 0;
    let mut loop_stack: Vec<usize> = vec![];

    while tokens.len() > pos {
        let token = &tokens[pos];
        match &token.value {
            TokenValue::None => match token.kind {
                TokenKind::Output => {
                    print!("{}", memory[mempos] as u8 as char);
                }
                TokenKind::Input => {
                    todo!("input char (,)")
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
                        // Takes 14167554488ns of mandelbrot when in release mode
                        pos = find_correct_closing_bracket(pos, &tokens)
                            .expect(&*format!("Opened loop never closed: token pos: {}", pos));
                    } else {
                        loop_stack.push(pos);
                    }
                }
                BracketState::Closed => {
                    if memory[mempos] as u8 != 0 {
                        pos = *loop_stack
                            .last()
                            .expect(&*format!("Closed loop never opened: token pos: {}", pos));
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
