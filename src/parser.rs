mod conversion;

use crate::cmd::{CellUpdate, Cmd, Point, RgbColor};
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

type Error<T> = Result<T, String>;

#[derive(Debug, PartialEq, Eq)]
pub(in crate::parser) enum Token {
    Word(String),
    Number(u16),
    Char(char),
    StringLiteral(String),
    LeftBracket,
    RightBracket,
    Comma,
    Equal,
}

#[derive(Debug)]
enum ArgValue {
    Single(Token),
    Tuple(Vec<Token>),
}

struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Error<Token> {
        let mut num_str = String::new();

        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() {
                num_str.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }

        let val = num_str.parse::<u16>().map_err(|_|
            format!(
                "Syntax Error: Number '{}' is too large. It must fit within the u16 range (0 - 65535).", 
                num_str
            )
        )?;

        Ok(Token::Number(val))
    }

    fn read_char_literal(&mut self) -> Error<Token> {
        self.chars.next();

        let first_char = self.chars.next().ok_or(
            "Syntax Error: Expected a character after the opening single quote".to_string(),
        )?;

        let c = if first_char == '\\' {
            let escaped = self
                .chars
                .next()
                .ok_or("Syntax Error: Expected an escape character after '\\'.".to_string())?;
            match escaped {
                '\\' => '\\',
                '\'' => '\'',

                _ => {
                    return Err(format!(
                        "Syntax Error: Invalid or unrecognized escape sequence '\\{}'. Only '\\\\' and '\\'' are allowed.",
                        escaped
                    ));
                }
            }
        } else {
            first_char
        };

        let close = self.chars.next().ok_or(
            "Syntax Error: Unterminated character literal. Missing a closing single quote '\''."
                .to_string(),
        )?;

        if close != '\'' {
            return Err(format!(
                "Syntax Error: Expected '\'' to close character literal, but found '{}'.",
                close
            ));
        }

        Ok(Token::Char(c))
    }

    fn read_string_literal(&mut self) -> Error<Token> {
        self.chars.next();
        let mut s = String::new();

        while let Some(&c) = self.chars.peek() {
            if c == '"' {
                break;
            } else if c == '\\' {
                self.chars.next();

                let escaped = self
                    .chars
                    .next()
                    .ok_or("Syntax Error: Expected an escape character after '\\'.".to_string())?;
                let special_char = match escaped {
                    '\\' => '\\',
                    '"' => '"',

                    _ => {
                        return Err(format!(
                            "Syntax Error: Invalid or unrecognized escape sequence '\\{}'. Only '\\\\' and '\\\"' are allowed.",
                            escaped
                        ));
                    }
                };

                s.push(special_char);
            } else {
                s.push(self.chars.next().unwrap());
            }
        }

        self.chars.next().ok_or(
            "Syntax Error: Unterminated string literal. Missing a closing double quote '\"'."
                .to_string(),
        )?;

        Ok(Token::StringLiteral(s))
    }

    fn read_word(&mut self) -> Token {
        let mut word_str = String::new();

        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() || c == '_' {
                word_str.push(self.chars.next().unwrap());
            } else {
                break;
            }
        }

        Token::Word(word_str)
    }

    fn next_token(&mut self) -> Option<Error<Token>> {
        self.skip_whitespace();

        let next_char = self.chars.peek()?;

        let result = match next_char {
            '(' => {
                self.chars.next();
                Ok(Token::LeftBracket)
            }
            ')' => {
                self.chars.next();
                Ok(Token::RightBracket)
            }
            ',' => {
                self.chars.next();
                Ok(Token::Comma)
            }
            '=' => {
                self.chars.next();
                Ok(Token::Equal)
            }
            '\'' => self.read_char_literal(),
            '"' => self.read_string_literal(),
            c if c.is_ascii_digit() => self.read_number(),
            c if c.is_alphabetic() => Ok(self.read_word()),
            _ => {
                return Some(Err(format!(
                    "Syntax Error: Unexpected character '{}'.",
                    next_char
                )));
            }
        };

        Some(result)
    }
}

struct ArgsContext {
    bindings: HashMap<String, ArgValue>,
}

impl ArgsContext {
    fn expect_bracket(tokenizer: &mut Tokenizer, expected: Token, err_msg: &str) -> Error<()> {
        let token = tokenizer
            .next_token()
            .transpose()?
            .ok_or(format!("Syntax Error: {}", err_msg))?;
        if token != expected {
            return Err(format!("Syntax Error: {}.", err_msg));
        }

        Ok(())
    }

    fn expect_equal(tokenizer: &mut Tokenizer) -> Error<()> {
        let token = tokenizer.next_token().transpose()?.ok_or(
            "Syntax Error: Expected '=' after named argument, but reached the end of the input."
                .to_string(),
        )?;
        if token != Token::Equal {
            return Err(format!(
                "Syntax Error: Expected '=' after argument name, but found '{:?}'.",
                token
            ));
        }

        Ok(())
    }

    fn parse(param_names: &[&str], tokenizer: &mut Tokenizer) -> Error<Self> {
        Self::expect_bracket(
            tokenizer,
            Token::LeftBracket,
            "opening parenthesis '(' after command name.",
        )?;

        let mut bindings = HashMap::new();
        let mut index = 0;
        let mut has_named_arg = false;

        loop {
            let token = match tokenizer.next_token().transpose()? {
                Some(t) => t,
                None => return Err("Syntax Error: Missing closing parenthesis ')'.".to_string()),
            };

            if token == Token::RightBracket {
                break;
            }

            match token {
                Token::Comma => continue,

                Token::LeftBracket => {
                    let mut tuple_tokens = Vec::new();
                    while let Some(sub_res) = tokenizer.next_token() {
                        let sub_token = sub_res?;

                        if sub_token == Token::RightBracket {
                            break;
                        }
                        if sub_token == Token::Comma {
                            continue;
                        }
                        tuple_tokens.push(sub_token);
                    }

                    bindings.insert(
                        param_names[index].to_string(),
                        ArgValue::Tuple(tuple_tokens),
                    );
                    index += 1;
                }

                Token::Word(name) => {
                    has_named_arg = true;

                    if !param_names.contains(&name.as_str()) {
                        return Err(format!(
                            "Syntax Error: Unknown argument name '{}'. Allowed arguments are: {:?}.",
                            name, param_names
                        ));
                    }

                    Self::expect_equal(tokenizer)?;
                    let next = tokenizer.next_token().transpose()?.ok_or(
                        "Syntax Error: Expected a value, but reached the end of input.".to_string(),
                    )?;

                    if next == Token::LeftBracket {
                        let mut tuple_tokens = Vec::new();
                        while let Some(sub_res) = tokenizer.next_token() {
                            let sub_token = sub_res?;

                            if sub_token == Token::RightBracket {
                                break;
                            }
                            if sub_token == Token::Comma {
                                continue;
                            }
                            tuple_tokens.push(sub_token);
                        }
                        bindings.insert(name, ArgValue::Tuple(tuple_tokens));
                    } else {
                        bindings.insert(name, ArgValue::Single(next));
                    }
                }

                any_token => {
                    if index >= param_names.len() {
                        return Err(format!(
                            "Syntax Error: Too many positional arguments. Expected at most {} arguments.",
                            param_names.len()
                        ));
                    }

                    if has_named_arg {
                        return Err(
                            "Syntax Error: Positional argument follows keyword argument."
                                .to_string(),
                        );
                    }

                    bindings.insert(param_names[index].to_string(), ArgValue::Single(any_token));
                    index += 1;
                }
            }
        }

        Ok(Self { bindings })
    }

    fn take(&mut self, arg_name: &str) -> Option<ArgValue> {
        self.bindings.remove(arg_name)
    }
}

pub struct Parser {}

impl Parser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_line(&mut self, line: &str) -> Error<Vec<Cmd>> {
        let mut cmds: Vec<Cmd> = Vec::new();

        let mut tokenizer = Tokenizer::new(line);
        let token = match tokenizer.next_token().transpose()? {
            Some(t) => t,
            None => return Ok(cmds),
        };

        if let Token::Word(cmd_name) = token {
            match cmd_name.as_str() {
                "Draw" => self.parse_draw(&mut cmds, &mut tokenizer)?,
                "Clear" => self.parse_clear(&mut cmds, &mut tokenizer)?,
                "Flush" => self.parse_flush(&mut cmds, &mut tokenizer)?,
                "QuerySize" => self.parse_query_size(&mut cmds, &mut tokenizer)?,
                _ => return Err(format!("Syntax Error: Unknown command '{}'.", cmd_name)),
            }
        }

        Ok(cmds)
    }
}

impl Parser {
    fn apply_colors(
        update: &mut CellUpdate,
        fg: &Option<ArgValue>,
        bg: &Option<ArgValue>,
    ) -> Error<()> {
        if let Some(ArgValue::Tuple(tokens)) = fg {
            update.fg_color = Some(
                RgbColor::from_tokens(tokens.as_slice()).ok_or(
                    "Syntax Error: 'fg_color' must be a tuple of exactly 3 numbers (R, G, B)."
                        .to_string(),
                )?,
            );
        }
        if let Some(ArgValue::Tuple(tokens)) = bg {
            update.bg_color = Some(
                RgbColor::from_tokens(tokens.as_slice()).ok_or(
                    "Syntax Error: 'bg_color' must be a tuple of exactly 3 numbers (R, G, B)."
                        .to_string(),
                )?,
            );
        }
        Ok(())
    }

    fn parse_draw(&mut self, cmds: &mut Vec<Cmd>, tokenizer: &mut Tokenizer) -> Error<()> {
        let mut arg_content =
            ArgsContext::parse(&["x", "y", "glyph", "fg_color", "bg_color"], tokenizer)?;

        let x = match arg_content.take("x") {
            Some(ArgValue::Single(Token::Number(n))) => n,
            _ => return Err("Syntax Error: Missing or invalid 'x' coordinate.".to_string()),
        };

        let y = match arg_content.take("y") {
            Some(ArgValue::Single(Token::Number(n))) => n,
            _ => return Err("Syntax Error: Missing or invalid 'y' coordinate.".to_string()),
        };

        let target = arg_content
            .take("glyph")
            .ok_or("Syntax Error: Missing 3rd argument (char or string).".to_string())?;

        let result_fg_color = arg_content.take("fg_color");
        let result_bg_color = arg_content.take("bg_color");

        match target {
            ArgValue::Single(Token::Char(c_val)) => {
                let mut update = CellUpdate {
                    pos: Point { x, y },
                    glyph: c_val,
                    ..Default::default()
                };

                Self::apply_colors(&mut update, &result_fg_color, &result_bg_color)?;

                cmds.push(Cmd::Draw(update));
            }
            ArgValue::Single(Token::StringLiteral(s)) => {
                for (i, c_val) in s.chars().enumerate() {
                    let mut update = CellUpdate {
                        pos: Point { x: x + i as u16, y },
                        glyph: c_val,
                        ..Default::default()
                    };

                    Self::apply_colors(&mut update, &result_fg_color, &result_bg_color)?;

                    cmds.push(Cmd::Draw(update));
                }
            }
            _ => return Err("Syntax Error: Third argument must be a char or string.".to_string()),
        }

        Ok(())
    }

    fn parse_flush(&mut self, cmds: &mut Vec<Cmd>, tokenizer: &mut Tokenizer) -> Error<()> {
        let _arg_content = ArgsContext::parse(&[], tokenizer)?;

        cmds.push(Cmd::Flush);
        Ok(())
    }

    fn parse_clear(&mut self, cmds: &mut Vec<Cmd>, tokenizer: &mut Tokenizer) -> Error<()> {
        let _arg_content = ArgsContext::parse(&[], tokenizer)?;

        cmds.push(Cmd::Clear);
        Ok(())
    }

    fn parse_query_size(&mut self, cmds: &mut Vec<Cmd>, tokenizer: &mut Tokenizer) -> Error<()> {
        let _arg_content = ArgsContext::parse(&[], tokenizer)?;

        cmds.push(Cmd::QuerySize);
        Ok(())
    }
}
