use crate::{
    common, position,
    token::{self, StringToken},
};
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
                ch if ch.is_ascii_digit() => Some(self.number(ch)),
                '$' => {
                    self.next_char();
                    self.string(true)
                }
                '"' => self.string(false),
                '\'' => self.label(),
                ch if ch.is_alphabetic() || ch == '_' => self.identifier(ch),
                '(' => Some(Token::LeftParen),
                ')' => Some(Token::RightParen),
                '{' => Some(Token::LeftBrace),
                '}' => Some(Token::RightBrace),
                '[' => Some(Token::LeftBracket),
                ']' => Some(Token::RightBracket),
                ',' => Some(Token::Comma),
                '.' => {
                    let mut input = self.input.clone();
                    Some(
                        if let (Some('.'), Some('.')) = (input.next(), input.next()) {
                            self.next_char()?;
                            self.next_char()?;
                            Token::Dot3
                        } else {
                            Token::Dot
                        },
                    )
                }
                '+' => Some(self.matches_or('=', Token::PlusEq, Token::Plus)),
                ';' => Some(Token::Semicolon),
                '/' => Some(if self.consume_if(|ch| ch == '/') {
                    if self.consume_if(|ch| ch == '=') {
                        Token::Slash2Eq
                    } else {
                        Token::Slash2
                    }
                } else {
                    Token::Slash
                }),
                '-' => {
                    if self.consume_if(|ch| ch == '-') {
                        if self.consume_if(|ch| ch == '-') {
                            while let Some(ch) = self.next_char() {
                                if ch == '-'
                                    && self.consume_if(|ch| ch == '-')
                                    && self.consume_if(|ch| ch == '-')
                                {
                                    break;
                                }
                            }
                            None
                        } else {
                            self.consume_while(|ch| ch != '\n');
                            None
                        }
                    } else {
                        Some(if self.consume_if(|ch| ch == '>') {
                            Token::Arrow
                        } else if self.consume_if(|ch| ch == '=') {
                            Token::MinusEq
                        } else {
                            Token::Minus
                        })
                    }
                }
                '*' => Some(self.matches_or('=', Token::StarEq, Token::Star)),
                '%' => Some(self.matches_or('=', Token::PercentEq, Token::Percent)),
                '^' => Some(self.matches_or('=', Token::CaretEq, Token::Caret)),
                '#' => Some(Token::Hash),
                '?' => Some(if self.consume_if(|ch| ch == '?') {
                    Token::Mark2
                } else if self.consume_if(|ch| ch == '.') {
                    Token::MarkDot
                } else {
                    Token::Mark
                }),
                ':' => Some(self.matches_or(':', Token::Colon2, Token::Colon)),
                '=' => {
                    if let Some(next) = self.peek().cloned() {
                        match next {
                            '>' => {
                                self.next_char();
                                Some(Token::FatArrow)
                            }
                            '=' => {
                                self.next_char();
                                Some(Token::Eq2)
                            }
                            _ => Some(Token::Eq),
                        }
                    } else {
                        Some(Token::Eq)
                    }
                }
                '!' => Some(self.matches_or('=', Token::BangEq, Token::Bang)),
                '<' => Some(if self.consume_if(|ch| ch == '<') {
                    if self.consume_if(|ch| ch == '=') {
                        Token::Less2Eq
                    } else {
                        Token::Less2
                    }
                } else if self.consume_if(|ch| ch == '=') {
                    Token::LessEq
                } else {
                    Token::Less
                }),
                '>' => Some(if self.consume_if(|ch| ch == '>') {
                    if self.consume_if(|ch| ch == '=') {
                        Token::Greater2Eq
                    } else {
                        Token::Greater2
                    }
                } else if self.consume_if(|ch| ch == '=') {
                    Token::GreaterEq
                } else {
                    Token::Greater
                }),
                '&' => Some(Token::Ampersand),
                '|' => Some(Token::Bar),
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

    fn string(&mut self, interpolated: bool) -> Option<Token> {
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
            Some(Token::String(StringToken {
                value: chars.into_iter().collect::<String>(),
                kind: common::StringKind::Multiline,
                interpolated,
            }))
        } else {
            let string: String = self
                .consume_while(|ch| ch != '"')
                .into_iter()
                .collect::<String>();
            self.next_char();
            Some(Token::String(StringToken {
                value: string,
                kind: common::StringKind::Regular,
                interpolated,
            }))
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
            Some(Token::Ident(identifier))
        }
    }

    fn keyword(identifier: &str) -> Option<Token> {
        match identifier {
            "let" => Some(Token::Let),
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
            "return" => Some(Token::Return),
            "use" => Some(Token::Use),
            "struct" => Some(Token::Struct),
            "impl" => Some(Token::Impl),
            "match" => Some(Token::Match),
            "self" => Some(Token::SelfValue),
            "Self" => Some(Token::SelfType),
            "break" => Some(Token::Break),
            "continue" => Some(Token::Continue),
            "extern" => Some(Token::Extern),
            "inline" => Some(Token::Inline),
            "and" => Some(Token::And),
            "or" => Some(Token::Or),
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
            .collect::<String>()
            .replace('_', "");
        num_str.insert(0, ch);

        let is_float = self.peek() == Some(&'.');
        if is_float {
            self.next_char();
            let num_fract_str = self
                .consume_while(|ch| ch.is_ascii_digit() || ch == '_')
                .into_iter()
                .collect::<String>()
                .replace('_', "");
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
        assert_eq!(tokenize("ident"), vec![Token::Ident(String::from("ident"))]);
        assert_eq!(tokenize("let"), vec![Token::Let]);
    }

    #[test]
    fn label() {
        assert_eq!(
            tokenize("'label"),
            vec![Token::Label(String::from("label"))]
        )
    }

    // TODO: rewrite tests
    // #[test]
    // fn string() {
    //     assert_eq!(
    //         tokenize(" \"str\""),
    //         vec![Token::String(
    //             common::StringKind::Regular,
    //             String::from("str")
    //         ),]
    //     )
    // }

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
                Token::Ident(String::from("a")),
                Token::Dot,
                Token::Ident(String::from("b")),
            ]
        );
        assert_eq!(
            tokenize("a?.b"),
            vec![
                Token::Ident(String::from("a")),
                Token::MarkDot,
                Token::Ident(String::from("b")),
            ]
        );
        assert_eq!(
            tokenize("a?b"),
            vec![
                Token::Ident(String::from("a")),
                Token::Unknown('?'),
                Token::Ident(String::from("b")),
            ]
        );
    }
}
