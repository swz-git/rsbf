#[derive(PartialEq, Debug)]
pub enum TokenKind {
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
pub enum BracketState {
    Open,
    Closed,
}

#[derive(Debug)]
pub enum TokenValue {
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
pub struct Token {
    kind: TokenKind,
    value: TokenValue,
}

// Translates input string to Vec<Token>
pub fn tokenize(input: &str) -> Vec<Token> {
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

// Optimizes input
pub fn optimize(input: Vec<Token>) -> Vec<Token> {
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
pub fn c_translate(tokens: Vec<Token>) -> String {
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
