#[derive(PartialEq, Debug, Clone)]
pub enum BracketState {
    Open,
    Closed,
}

#[derive(PartialEq, Debug, Clone)]
pub enum TokenKind {
    ValMod(isize),
    PosMod(isize),
    Bracket(BracketState),
    Comment,
    Output,
    Input,
    Clear,
    Copy(isize),
}
impl TokenKind {
    fn from(input: char) -> TokenKind {
        match input {
            '+' => TokenKind::ValMod(1),
            '-' => TokenKind::ValMod(-1),
            '>' => TokenKind::PosMod(1),
            '<' => TokenKind::PosMod(-1),
            '[' => TokenKind::Bracket(BracketState::Open),
            ']' => TokenKind::Bracket(BracketState::Closed),
            '.' => TokenKind::Output,
            ',' => TokenKind::Input,

            _ => TokenKind::Comment,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodePos {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub code_pos: CodePos,
}

// Translates input string to Vec<Token>
pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens: Vec<Token> = vec![];

    let mut line = 1;
    let mut col = 1;
    for command in input.chars() {
        if command == '\n' {
            line += 1;
            col = 1
        }

        let kind = TokenKind::from(command);
        let code_pos = CodePos { line, col };

        if kind == TokenKind::Comment {
            continue;
        };

        tokens.push(Token { kind, code_pos });
        col += 1;
    }

    tokens
}

// Optimizes input
pub fn optimize(input: Vec<Token>) -> Vec<Token> {
    let mut pos = 0usize;
    let mut tokens: Vec<Token> = input;

    // Optimized multiple adds into one. Example: i++;i++;i++; becomes i+=3;
    {
        while pos < tokens.len() - 1 {
            let token = &tokens[pos];
            let next = &tokens[pos + 1];
            match (&token.kind, &next.kind) {
                (TokenKind::PosMod(token_value), TokenKind::PosMod(next_value)) => {
                    tokens[pos].kind = TokenKind::PosMod(token_value + next_value);
                    tokens.remove(pos + 1);
                }
                (TokenKind::ValMod(token_value), TokenKind::ValMod(next_value)) => {
                    tokens[pos].kind = TokenKind::ValMod(token_value + next_value);
                    tokens.remove(pos + 1);
                }
                _ => {
                    pos += 1;
                }
            }
        }
    }

    // Replace while (*ptr) {*ptr += -1} with *ptr = 0;
    {
        pos = 0;
        while pos < tokens.len() - 2 {
            let tokens_for_check = &tokens[pos..pos + 3];
            if (tokens_for_check[0].kind == TokenKind::Bracket(BracketState::Open))
                && (tokens_for_check[1].kind == TokenKind::ValMod(-1))
                && (tokens_for_check[2].kind == TokenKind::Bracket(BracketState::Closed))
            {
                let code_pos = &tokens_for_check[0].code_pos;
                tokens.splice(
                    pos..pos + 3,
                    [Token {
                        kind: TokenKind::Clear,
                        code_pos: code_pos.clone(),
                    }],
                );
            }
            pos += 1;
        }
    }

    // Copy loops
    {
        let mut stage = 0;
        let mut start_code_pos = CodePos { line: 1, col: 1 };
        let mut tokens_optimized = 0;
        let mut should_clear = false;
        let mut current_pos_offset = 0;
        let mut copy_offsets: Vec<isize> = vec![];
        pos = 0;
        while pos < tokens.len() {
            let token = &tokens[pos];
            match stage {
                0 => {
                    if token.kind == TokenKind::Bracket(BracketState::Open) {
                        start_code_pos = token.code_pos.clone();
                        stage += 1;
                        tokens_optimized += 1;
                        pos += 1;
                    } else {
                        pos += 1;
                    }
                }
                1 => {
                    if token.kind == TokenKind::ValMod(-1) {
                        should_clear = true;
                        stage += 1;
                        tokens_optimized += 1;
                        pos += 1;
                    } else {
                        stage += 1;
                    }
                }
                2 => {
                    if let TokenKind::PosMod(value) = token.kind {
                        current_pos_offset += value;
                        stage += 1;
                        tokens_optimized += 1;
                        pos += 1;
                    } else {
                        stage = 100;
                    }
                }
                3 => {
                    if token.kind == TokenKind::ValMod(1) {
                        copy_offsets.push(current_pos_offset);
                        stage = 2;
                        tokens_optimized += 1;
                        pos += 1;
                    } else if token.kind == TokenKind::Bracket(BracketState::Closed)
                        && current_pos_offset == 0
                    {
                        tokens.drain((pos - tokens_optimized)..(pos + 1));

                        for copy_offset in &copy_offsets {
                            tokens.insert(
                                pos - tokens_optimized,
                                Token {
                                    kind: TokenKind::Copy(*copy_offset),
                                    code_pos: start_code_pos.clone(),
                                },
                            )
                        }

                        pos = pos - tokens_optimized + copy_offsets.len();

                        if should_clear {
                            tokens.insert(
                                pos,
                                Token {
                                    kind: TokenKind::Clear,
                                    code_pos: start_code_pos.clone(),
                                },
                            );
                            pos += 1;
                        }

                        stage = 100;
                    } else {
                        stage = 100;
                    }
                }
                100 => {
                    // reset
                    stage = 0;
                    start_code_pos = CodePos { line: 1, col: 1 };
                    tokens_optimized = 0;
                    should_clear = false;
                    current_pos_offset = 0;
                    copy_offsets = vec![];
                    pos += 1;
                }
                _ => {}
            }
        }
    }

    // note: this site has lots of cool optimizations http://calmerthanyouare.org/2015/01/07/optimizing-brainfuck.html

    tokens
}

// Translates Vec<Token> to C
pub fn c_translate(tokens: Vec<Token>) -> String {
    let mut result =
        String::from("#include <stdio.h>\nint main(){char array[30000] = {0}; char *ptr = array;");

    for token in tokens {
        result += &match token.kind {
            TokenKind::Output => "putchar(*ptr);".into(),
            TokenKind::Input => "*ptr = getchar();".into(),
            TokenKind::Clear => "*ptr = 0;".into(),
            TokenKind::ValMod(value) => format!("*ptr += {};", value),
            TokenKind::PosMod(value) => format!("ptr += {};", value),
            TokenKind::Bracket(BracketState::Open) => "while (*ptr) {".into(),
            TokenKind::Bracket(BracketState::Closed) => "}".into(),
            TokenKind::Copy(value) => format!(
                "{{char *tptr = array;tptr = ptr;tptr+={};*tptr += *ptr;}}", // TODO: this can probably be improved
                value
            ),
            TokenKind::Comment => "".into(),
        }
    }

    result + "return 0;}"
}
