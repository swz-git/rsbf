use std::fs;

use clap::Parser;
use subprocess::{Exec, Redirection};

#[derive(PartialEq, Debug)]
enum TokenKind {
    ValMod,
    PosMod,
    Bracket,
    Comment,
    Output,
    Input,
}
impl TokenKind {
    fn from(input: char) -> TokenKind {
        match input {
            '+' | '-' => TokenKind::ValMod,
            '>' | '<' => TokenKind::PosMod,
            '[' | ']' => TokenKind::Bracket,
            '.' => TokenKind::Output,
            ',' => TokenKind::Input,

            _ => TokenKind::Comment,
        }
    }
}

#[derive(Debug)]
enum BracketState {
    Open,
    Closed,
}

#[derive(Debug)]
enum TokenValue {
    None,
    Int(i8),
    BracketState(BracketState),
}

impl TokenValue {
    fn from(input: char) -> TokenValue {
        return match input {
            '+' | '>' => TokenValue::Int(1),
            '-' | '<' => TokenValue::Int(-1),

            '[' => TokenValue::BracketState(BracketState::Open),
            ']' => TokenValue::BracketState(BracketState::Closed),

            '.' | ',' => TokenValue::None,

            _ => panic!("Input must be valid brainfuck command"),
        };
    }
}

#[derive(Debug)]
struct Token {
    kind: TokenKind,
    value: TokenValue,
}

fn tokenizer(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = vec![];

    for command in input.chars() {
        let kind = TokenKind::from(command);

        if kind == TokenKind::Comment {
            continue;
        };

        let value = TokenValue::from(command);

        tokens.push(Token { kind, value });
    }

    tokens
}

fn optimizer(input: Vec<Token>) -> Vec<Token> {
    let mut pos = 0usize;
    let mut tokens: Vec<Token> = input;

    while pos < tokens.len() - 1 {
        let token = &tokens[pos];
        let next = &tokens[pos + 1];
        if (token.kind == TokenKind::ValMod && next.kind == TokenKind::ValMod)
            || (token.kind == TokenKind::PosMod && next.kind == TokenKind::PosMod)
        {
            let token_value = match token.value {
                TokenValue::None => panic!("Invalid token value for type {:?}", token.kind),
                TokenValue::Int(i) => i,
                TokenValue::BracketState(..) => {
                    panic!("Invalid token value for type {:?}", token.kind)
                }
            };
            let next_value = match next.value {
                TokenValue::None => panic!("Invalid token value for type {:?}", token.kind),
                TokenValue::Int(i) => i,
                TokenValue::BracketState(..) => {
                    panic!("Invalid token value for type {:?}", token.kind)
                }
            };
            tokens[pos].value = TokenValue::Int(token_value + next_value);
            tokens.remove(pos + 1);
        } else {
            pos += 1;
        }
    }

    tokens
}

// Translates Vec<Token> to C
fn translator(tokens: Vec<Token>) -> String {
    let mut result =
        String::from("#include <stdio.h>\nint main(){char array[30000] = {0}; char *ptr = array;");

    for token in tokens {
        match token.value {
            TokenValue::None => {
                result += match token.kind {
                    TokenKind::Output => "putchar(*ptr);",
                    TokenKind::Input => "*ptr = getchar();",
                    _ => panic!("Kind isn't of value None"),
                };
            }
            TokenValue::Int(value) => {
                result += &(match token.kind {
                    TokenKind::ValMod => format!("*ptr += {};", value),
                    TokenKind::PosMod => format!("ptr += {};", value),
                    _ => panic!("Kind isn't of value Int"),
                });
            }
            TokenValue::BracketState(s) => {
                result += match s {
                    BracketState::Open => "while (*ptr) {",
                    BracketState::Closed => "}",
                }
            }
        }
    }

    result + "return 0;}"
}

// Compiles C to machine code
fn cc(input: &str, binary_name: &str) -> String {
    Exec::cmd("gcc")
        .args(&["-O3", "-o", binary_name, "-xc", "-"])
        .stdin(input)
        .stdout(Redirection::Pipe)
        .capture()
        .unwrap()
        .stdout_str()
}

/// Brainfuck to c transpiler
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Brainfuck file to transpile
    #[clap(short, long, value_parser)]
    file: String,

    /// Binary (output) path
    #[clap(short, long, value_parser)]
    output: String,

    /// Output C code instead of compiling with gcc
    #[clap(short, long, value_parser)]
    code: bool,
}

fn main() {
    let args = Args::parse();
    let contents = fs::read_to_string(args.file).expect("Something went wrong reading the file");
    let tokens = tokenizer(&contents);
    let optimized_tokens = optimizer(tokens);
    let c_code = translator(optimized_tokens);
    if args.code {
        print!("{}", c_code);
    } else {
        print!("{}", cc(&c_code, &(args.output)));
    }
}
