use crate::{common, position, token};
use std::str;
use token::Token;

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

    fn peek_next(&mut self) -> Option<char> {
        let mut input = self.input.clone();
        input.nth(1)
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
                self.next_char();
                chars.push(ch);
            } else {
                break;
            }
        }
        chars
    }

    fn next_token(&mut self) -> Option<Token> {
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
                '"' => self.string(),
                '\'' => self.label(),
                ch if ch.is_alphabetic() || ch == '_' => self.identifier(ch),
                '(' => Some(Token::LeftParen),
                ')' => Some(Token::RightParen),
                '{' => Some(Token::LeftBrace),
                '}' => Some(Token::RightBrace),
                ']' => Some(Token::RightBracket),
                '[' => Some(Token::RightBracket),
                ',' => Some(Token::Comma),
                '.' => Some(Token::Dot),
                '-' => Some(self.matches_or('>', Token::Arrow, Token::Minus)),
                '+' => Some(Token::Plus),
                ';' => Some(Token::Semicolon),
                '/' => {
                    if self.consume_if(|ch| ch == '/') {
                        self.consume_while(|ch| ch != '\n');
                        None
                    } else if self.consume_if(|ch| ch == '*') {
                        let mut stack = 1;
                        while let Some(ch) = self.next_char() {
                            if stack == 0 {
                                break;
                            }
                            if ch == '*' && self.peek() == Some(&'/') {
                                stack -= 1;
                            } else if ch == '/' && self.peek() == Some(&'*') {
                                stack += 1;
                            }
                        }
                        self.next_char();
                        None
                    } else {
                        Some(Token::Slash)
                    }
                }
                '*' => Some(Token::Star),
                '%' => Some(Token::Percent),
                '#' => Some(Token::Hash),
                '?' => Some(self.matches_or('.', Token::MarkDot, Token::QuestionMark)),
                ':' => Some(Token::Colon),

                '=' => {
                    if let Some(next) = self.peek().cloned() {
                        match next {
                            '>' => {
                                self.next_char();
                                Some(Token::FatArrow)
                            }
                            '=' => {
                                self.next_char();
                                Some(Token::Equal2)
                            }
                            _ => Some(Token::Equal),
                        }
                    } else {
                        Some(Token::Equal)
                    }
                }
                '!' => Some(self.matches_or('=', Token::BangEqual, Token::Bang)),
                '<' => Some(self.matches_or('=', Token::LessEqual, Token::Less)),
                '>' => Some(self.matches_or('=', Token::GreaterEqual, Token::Greater)),
                '&' => Some(self.matches_or('&', Token::Ampersand2, Token::Ampersand)),
                '|' => Some(self.matches_or('|', Token::Bar2, Token::Bar)),
                other => Some(Token::Unknown(other)),
            } {
                return Some(t);
            }
        }
        None
    }

    fn label(&mut self) -> Option<Token> {
        Some(Token::Label(
            self.consume_while(|ch| ch.is_ascii_alphanumeric())
                .into_iter()
                .collect(),
        ))
    }

    fn string(&mut self) -> Option<Token> {
        if self.peek().map(|&ch| ch == '"').unwrap_or_default()
            && self.peek_next().map(|ch| ch == '"').unwrap_or_default()
        {
            self.next_char();
            self.next_char();
            let mut chars = Vec::<char>::new();
            while let Some(ch) = self.next_char() {
                if ch == '"'
                    && self.peek().map(|&ch| ch == '"').unwrap_or_default()
                    && self.peek_next().map(|ch| ch == '"').unwrap_or_default()
                {
                    self.next_char();
                    self.next_char();
                    break;
                }
                chars.push(ch);
            }
            Some(Token::String(
                common::StringKind::Multiline,
                chars.into_iter().collect::<String>(),
            ))
        } else {
            let string: String = self
                .consume_while(|ch| ch != '"')
                .into_iter()
                .collect::<String>();
            self.next_char();
            Some(Token::String(common::StringKind::Regular, string))
        }
    }

    fn identifier(&mut self, ch: char) -> Option<Token> {
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
            Some(Token::Identifier(identifier))
        }
    }

    fn keyword(identifier: &str) -> Option<Token> {
        match identifier {
            "let" => Some(Token::Let),
            "global" => Some(Token::Global),
            "true" => Some(Token::True),
            "false" => Some(Token::False),
            "fn" => Some(Token::Fn),
            "if" => Some(Token::If),
            "else" => Some(Token::Else),
            "for" => Some(Token::For),
            "while" => Some(Token::While),
            "loop" => Some(Token::Loop),
            "in" => Some(Token::In),
            "nil" => Some(Token::Nil),
            "print" => Some(Token::Print),
            "return" => Some(Token::Return),
            "use" => Some(Token::Use),
            "struct" => Some(Token::Struct),
            "impl" => Some(Token::Impl),
            "match" => Some(Token::Match),
            "self" => Some(Token::_Self),
            "break" => Some(Token::Break),
            "continue" => Some(Token::Continue),
            _ => None,
        }
    }

    fn matches_or(&mut self, to_match: char, matched: Token, unmatched: Token) -> Token {
        if self.consume_if(|ch| ch == to_match) {
            matched
        } else {
            unmatched
        }
    }

    fn number(&mut self, ch: char) -> Token {
        let mut num_str = self
            .consume_while(|ch| ch.is_ascii_digit() || ch == '_')
            .into_iter()
            .collect::<String>();
        num_str.insert(0, ch);
        let mut num_str = num_str.replace('_', "");

        let is_float = (self.peek() == Some(&'.')
            && self
                .peek_next()
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or_default())
            || ch == '.';
        if is_float && self.consume_if_next(|ch| ch.is_ascii_digit()) && ch != '.' {
            let num_fract_str = self
                .consume_while(|ch| ch.is_ascii_digit() || ch == '_')
                .into_iter()
                .collect::<String>();
            let num_fract_str = num_fract_str.replace('_', "");
            num_str.push('.');
            num_str.push_str(&num_fract_str);
        }

        Token::Number(if is_float {
            token::NumberToken::Float(num_str.parse::<f64>().unwrap())
        } else {
            token::NumberToken::Int(num_str.parse::<i64>().unwrap())
        })
    }
}

pub fn tokenize(input: &str) -> Vec<position::WithSpan<Token>> {
    let mut tokenizer = Tokenizer::new(input);
    let mut tokens = vec![];
    loop {
        let initial_pos = tokenizer.current_pos;
        let Some(token) = tokenizer.next_token() else {
            break;
        };
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
        common,
        token::{self, NumberToken},
        tokenizer::{Token, Tokenizer},
    };

    fn tokenize(input: &str) -> Vec<Token> {
        super::tokenize(input)
            .iter()
            .map(|tc| tc.value.clone())
            .collect()
    }

    #[test]
    fn comments() {
        assert_eq!(tokenize("//a comment"), vec![]);
        assert_eq!(
            tokenize(
                "/* /* */
                 * mutiline comment
                 * */"
            ),
            vec![]
        );
    }

    #[test]
    fn identifier() {
        assert_eq!(
            tokenize("ident"),
            vec![Token::Identifier(String::from("ident"))]
        );
        assert_eq!(tokenize("let"), vec![Token::Let]);
    }

    #[test]
    fn label() {
        assert_eq!(
            tokenize("'label"),
            vec![Token::Label(String::from("label"))]
        )
    }

    #[test]
    fn string() {
        assert_eq!(
            tokenize(" \"str\""),
            vec![Token::String(
                common::StringKind::Regular,
                String::from("str")
            ),]
        )
    }

    #[test]
    fn number() {
        assert_eq!(
            tokenize("1.2 3 .4 5. .6."),
            vec![
                Token::Number(NumberToken::Float(1.2)),
                Token::Number(NumberToken::Int(3)),
                Token::Number(NumberToken::Float(0.4)),
                Token::Number(NumberToken::Int(5)),
                Token::Dot,
                Token::Number(NumberToken::Float(0.6)),
                Token::Dot,
            ],
        );
    }

    #[test]
    fn dot() {
        assert_eq!(
            tokenize("a.b"),
            vec![
                Token::Identifier(String::from("a")),
                Token::Dot,
                Token::Identifier(String::from("b")),
            ]
        );
        assert_eq!(
            tokenize("a?.b"),
            vec![
                Token::Identifier(String::from("a")),
                Token::MarkDot,
                Token::Identifier(String::from("b")),
            ]
        );
        assert_eq!(
            tokenize("a?b"),
            vec![
                Token::Identifier(String::from("a")),
                Token::Unknown('?'),
                Token::Identifier(String::from("b")),
            ]
        );
    }
}
