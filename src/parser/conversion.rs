use crate::cmd::RgbColor;
use crate::parser::Token;

impl RgbColor {
    pub(in crate::parser) fn from_tokens(tokens: &[Token]) -> Option<Self> {
        if let [Token::Number(r), Token::Number(g), Token::Number(b)] = tokens {
            Some(Self {
                r: *r as u8,
                g: *g as u8,
                b: *b as u8,
            })
        } else {
            None
        }
    }
}
