use crate::{position, token};
use std::str;
use token::TokenVariant;

pub struct Tokenizer<'a> {
    input: std::iter::Peekable<str::Chars<'a>>,
    current_pos: crate::position::BytePos,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            current_pos: crate::position::BytePos(0),
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let next = self.input.next();
        if let Some(ch) = next {
            self.current_pos.shift(ch);
        }
        next
    }

    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn consume_if(&mut self, func: impl Fn(char) -> bool) -> bool {
        if let Some(&ch) = self.peek() {
            if func(ch) {
                self.next_char().unwrap();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    // Consume next char if the next one after matches
    fn consume_if_next(&mut self, func: impl Fn(char) -> bool) -> bool {
        let mut input = self.input.clone();

        match input.next() {
            Some(_) => (),
            None => return false,
        };

        if let Some(&ch) = input.peek() {
            if func(ch) {
                self.next_char().unwrap();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn consume_while(&mut self, func: impl Fn(char) -> bool) -> Vec<char> {
        let mut chars = Vec::<char>::new();
        while let Some(&ch) = self.input.peek() {
            if func(ch) {
                self.next_char().unwrap();
                chars.push(ch);
            } else {
                break;
            }
        }
        chars
    }

    fn next_token(&mut self) -> Option<TokenVariant> {
        while let Some(ch) = self.next_char() {
            if let Some(t) = match ch {
                ch if ch.is_whitespace() => None,
                ch if ch.is_ascii_digit()
                    || (ch == '.'
                        && self
                            .peek()
                            .map(|ch| ch.is_ascii_digit())
                            .unwrap_or_default()) =>
                {
                    Some(self.number(ch))
                }
                ch if ch.is_alphabetic() || ch == '_' => self.identifier(ch),
                '(' => Some(TokenVariant::LeftParen),
                ')' => Some(TokenVariant::RightParen),
                '{' => Some(TokenVariant::LeftBrace),
                '}' => Some(TokenVariant::RightBrace),
                ']' => Some(TokenVariant::RightBracket),
                '[' => Some(TokenVariant::RightBracket),
                ',' => Some(TokenVariant::Comma),
                '.' => Some(TokenVariant::Dot),
                '-' => Some(self.matches_or('>', TokenVariant::Arrow, TokenVariant::Minus)),
                '+' => Some(TokenVariant::Plus),
                ';' => Some(TokenVariant::Semicolon),
                '/' => {
                    if self.consume_if(|ch| ch == '/') {
                        self.consume_while(|ch| ch != '\n');
                        None
                    } else {
                        Some(TokenVariant::Slash)
                    }
                }
                '*' => Some(TokenVariant::Star),
                '%' => Some(TokenVariant::Percent),
                '#' => Some(TokenVariant::Hash),
                '?' => Some(TokenVariant::QuestionMark),
                ':' => Some(TokenVariant::Colon),

                '=' => {
                    if let Some(next) = self.peek() {
                        match next {
                            '>' => Some(TokenVariant::FatArrow),
                            '=' => Some(TokenVariant::Equal2),
                            _ => Some(TokenVariant::Equal),
                        }
                    } else {
                        Some(TokenVariant::Equal)
                    }
                }
                '!' => Some(self.matches_or('=', TokenVariant::BangEqual, TokenVariant::Bang)),
                '<' => Some(self.matches_or('=', TokenVariant::LessEqual, TokenVariant::Equal)),
                '>' => {
                    Some(self.matches_or('=', TokenVariant::GreaterEqual, TokenVariant::Greater))
                }
                '&' => {
                    Some(self.matches_or('&', TokenVariant::Ampersand2, TokenVariant::Ampersand))
                }
                '|' => Some(self.matches_or('|', TokenVariant::Bar2, TokenVariant::Bar)),
                other => Some(TokenVariant::Unknown(other)),
            } {
                return Some(t);
            }
        }
        None
    }

    fn identifier(&mut self, ch: char) -> Option<TokenVariant> {
        let mut identifier = String::from(ch);
        identifier.push_str(
            &self
                .consume_while(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                .into_iter()
                .collect::<String>(),
        );
        if let Some(token) = Self::keyword(&identifier) {
            Some(token)
        } else {
            Some(TokenVariant::Identifier(identifier))
        }
    }

    fn keyword(identifier: &str) -> Option<TokenVariant> {
        match identifier {
            "let" => Some(TokenVariant::Let),
            "var" => Some(TokenVariant::Var),
            "global" => Some(TokenVariant::Global),
            "true" => Some(TokenVariant::True),
            "false" => Some(TokenVariant::False),
            "fn" => Some(TokenVariant::Fn),
            "if" => Some(TokenVariant::If),
            "else" => Some(TokenVariant::Else),
            "for" => Some(TokenVariant::For),
            "while" => Some(TokenVariant::While),
            "loop" => Some(TokenVariant::Loop),
            "in" => Some(TokenVariant::In),
            "nil" => Some(TokenVariant::Nil),
            "print" => Some(TokenVariant::Print),
            "return" => Some(TokenVariant::Return),
            "super" => Some(TokenVariant::Super),
            "use" => Some(TokenVariant::Use),
            "struct" => Some(TokenVariant::Struct),
            "impl" => Some(TokenVariant::Impl),
            "match" => Some(TokenVariant::Match),
            "self" => Some(TokenVariant::_Self),
            _ => None,
        }
    }

    fn matches_or(
        &mut self,
        to_match: char,
        matched: TokenVariant,
        unmatched: TokenVariant,
    ) -> TokenVariant {
        if self.consume_if(|ch| ch == to_match) {
            matched
        } else {
            unmatched
        }
    }

    fn number(&mut self, ch: char) -> TokenVariant {
        let mut num_str = self
            .consume_while(|ch| ch.is_ascii_digit())
            .into_iter()
            .collect::<String>();
        num_str.insert(0, ch);

        let is_float = self.input.peek() == Some(&'.') || ch == '.';
        if is_float && self.consume_if_next(|ch| ch.is_ascii_digit()) && ch != '.' {
            let num_fract_str = self
                .consume_while(|ch| ch.is_ascii_digit())
                .into_iter()
                .collect::<String>();
            num_str.push('.');
            num_str.push_str(&num_fract_str);
        } else if self.input.peek() == Some(&'.') && ch != '.' {
            self.next_char();
        }

        TokenVariant::Number(if is_float {
            token::NumberToken::Float(num_str.parse::<f64>().unwrap())
        } else {
            token::NumberToken::Int(num_str.parse::<i64>().unwrap())
        })
    }
}

pub fn tokenize(input: &str) -> Vec<position::WithSpan<TokenVariant>> {
    let mut tokenizer = Tokenizer::new(input);
    let mut tokens = vec![];
    while let Some(token) = tokenizer.next_token() {
        let initial_pos = tokenizer.current_pos;
        tokens.push(position::WithSpan::new(
            token,
            position::Span::new(initial_pos, tokenizer.current_pos),
        ));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use crate::{
        token::{self, NumberToken},
        tokenizer::{TokenVariant, Tokenizer},
    };

    fn tokenize(input: &str) -> Vec<TokenVariant> {
        super::tokenize(input)
            .iter()
            .map(|tc| tc.value.clone())
            .collect()
    }
    #[test]
    fn identifier() {
        assert_eq!(
            tokenize("ident"),
            vec![TokenVariant::Identifier(String::from("ident"))]
        );
        assert_eq!(tokenize("let"), vec![TokenVariant::Let]);
    }

    #[test]
    fn number() {
        assert_eq!(
            tokenize("1.2 3 .4 5. .6."),
            vec![
                TokenVariant::Number(NumberToken::Float(1.2)),
                TokenVariant::Number(NumberToken::Int(3)),
                TokenVariant::Number(NumberToken::Float(0.4)),
                TokenVariant::Number(NumberToken::Float(5.0)),
                TokenVariant::Number(NumberToken::Float(0.6)),
                TokenVariant::Dot,
            ],
        );
    }
}
