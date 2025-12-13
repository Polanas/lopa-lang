use crate::{
    ast::*,
    position::{self, WithSpan},
    token::{self, Token, TokenKind},
};

static EOF_TOKEN: WithSpan<Token> = position::WithSpan::empty(Token::EOF);

pub struct Parser<'t> {
    tokens: &'t [WithSpan<Token>],
    cursor: usize,
    diagnostics: Vec<position::Diagnostic>,
}

impl<'t> Parser<'t> {
    pub fn new(tokens: &'t [WithSpan<Token>]) -> Self {
        Self {
            tokens,
            cursor: 0,
            diagnostics: Default::default(),
        }
    }

    pub fn diagnostics(&self) -> &[position::Diagnostic] {
        &self.diagnostics
    }

    pub fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    pub fn peek_token(&self) -> &'t WithSpan<Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => token,
            None => &EOF_TOKEN,
        }
    }

    pub fn peek(&self) -> TokenKind {
        (&self.peek_token().value).into()
    }

    pub fn check(&self, match_token: TokenKind) -> bool {
        self.peek() == match_token
    }

    pub fn advance(&mut self) -> &'t WithSpan<Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => {
                self.cursor += 1;
                token
            }
            None => &EOF_TOKEN,
        }
    }

    pub fn match_token(&mut self, kind: TokenKind) -> Option<&'t WithSpan<Token>> {
        let check = self.check(kind);
        if check { Some(self.advance()) } else { None }
    }

    pub fn match_tokens(&mut self, kinds: &[TokenKind]) -> Option<&'t WithSpan<Token>> {
        for kind in kinds {
            if self.check(*kind) {
                return Some(self.advance());
            }
        }
        None
    }

    pub fn previous(&self) -> Option<&'t WithSpan<Token>> {
        self.tokens.get(self.cursor - 1)
    }

    pub fn expect(&mut self, expected: TokenKind) -> Option<&'t WithSpan<Token>> {
        let token = self.advance();
        if TokenKind::from(&token.value) == expected {
            Some(token)
        } else {
            self.add_error(
                &format!(
                    "Expected {}, but got {}",
                    expected,
                    TokenKind::from(&token.value)
                ),
                token.span,
            );
            None
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.peek() != TokenKind::EOF
    }

    // pub fn expect_optional(&mut self, expected: TokenKind) -> Option<bool> {
    //     let token = self.peek();
    //     if token == expected {
    //         self.expect(expected)?;
    //         Some(true)
    //     } else {
    //         Some(false)
    //     }
    // }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Precedence {
    None,
    Assign,
    Or,
    And,
    Equality,   // == !=
    Comparison, // < <= > >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // ()
    List,       // []
    Primary,
}

impl From<TokenKind> for Precedence {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::Equal => Self::Assign,
            TokenKind::Bar2 => Self::Or,
            TokenKind::Ampersand2 => Self::And,
            TokenKind::BangEqual | TokenKind::Equal2 => Self::Equality,
            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Self::Comparison,
            TokenKind::Plus | TokenKind::Minus => Self::Term,
            TokenKind::Star | TokenKind::Slash => Self::Factor,
            TokenKind::Bang => Self::Unary,
            _ => Self::None,
        }
    }
}

macro_rules! parse_binary_fn {
    ($name: ident, $recursive: ident, $tokens: expr) => {
        pub fn $name(&mut self) -> Option<WithSpan<Expr>> {
            let mut expr = self.$recursive()?;

            while let Some(op) = self.match_tokens($tokens) {
                let right = self.$recursive()?;
                let span = expr.span.union(right.span);
                expr = WithSpan::new(
                    Expr::Binary(BinaryExpr {
                        left: expr.into(),
                        right: right.into(),
                        op: WithSpan::new(BinaryOperator::from_token(&op.value).unwrap(), op.span),
                    }),
                    span,
                );
            }
            Some(expr)
        }
    };
}

impl<'t> Parser<'t> {
    pub fn parse_expr(&mut self) -> Option<WithSpan<Expr>> {
        self.parse_eq()
    }

    parse_binary_fn!(
        parse_eq,
        parse_comparison,
        &[TokenKind::BangEqual, TokenKind::Equal2]
    );
    parse_binary_fn!(
        parse_comparison,
        parse_term,
        &[
            TokenKind::Greater,
            TokenKind::GreaterEqual,
            TokenKind::Less,
            TokenKind::LessEqual,
        ]
    );
    parse_binary_fn!(
        parse_term,
        parse_factor,
        &[TokenKind::Minus, TokenKind::Plus]
    );
    parse_binary_fn!(
        parse_factor,
        parse_unary,
        &[TokenKind::Slash, TokenKind::Star]
    );

    pub fn parse_unary(&mut self) -> Option<WithSpan<Expr>> {
        if let Some(op) = self.match_tokens(&[TokenKind::Bang, TokenKind::Minus]) {
            let right = self.parse_unary()?;
            let span = op.span.union(right.span);
            Some(WithSpan::new(
                Expr::Unary(
                    WithSpan::new(UnaryOp::from_token(&op.value).unwrap(), op.span),
                    right.into(),
                ),
                span,
            ))
        } else {
            self.parse_primary()
        }
    }

    pub fn parse_primary(&mut self) -> Option<WithSpan<Expr>> {
        if let Some(t) = self.match_token(TokenKind::False) {
            Some(WithSpan::new(Expr::Boolean(false), t.span))
        } else if let Some(t) = self.match_token(TokenKind::True) {
            Some(WithSpan::new(Expr::Boolean(true), t.span))
        } else if let Some(t) = self.match_token(TokenKind::Nil) {
            Some(WithSpan::new(Expr::Nil, t.span))
        } else if let Some(t) = self.match_token(TokenKind::String) {
            Some(WithSpan::new(
                Expr::String(match &t.value {
                    Token::String(s) => s.clone(),
                    _ => unreachable!(),
                }),
                t.span,
            ))
        } else if let Some(t) = self.match_token(TokenKind::Number) {
            Some(WithSpan::new(
                Expr::Number(match &t.value {
                    Token::Number(n) => match n {
                        token::NumberToken::Int(i) => Number::Int(*i),
                        token::NumberToken::Float(f) => Number::Float(*f),
                    },
                    _ => unreachable!(),
                }),
                t.span,
            ))
        } else if self.match_token(TokenKind::LeftParen).is_some() {
            let expr = self.parse_expr()?;
            let span = expr.span;
            self.expect(TokenKind::RightParen)?;
            Some(WithSpan::new(Expr::Grouping(expr.into()), span))
        } else {
            let token = self.advance();
            self.add_error(
                &format!(
                    "Expected one of: bool, nil, string, number, (, but got {}",
                    TokenKind::from(&token.value)
                ),
                token.span,
            );
            None
        }
    }

    pub fn sync(&mut self) {
        let mut token = self.advance();

        while !self.is_at_end() {
            if TokenKind::from(&token.value) == TokenKind::Semicolon {
                return;
            }

            if matches!(
                self.peek(),
                TokenKind::Struct
                    | TokenKind::Fn
                    | TokenKind::Let
                    | TokenKind::For
                    | TokenKind::If
                    | TokenKind::While
                    | TokenKind::Loop
                    | TokenKind::Print
                    | TokenKind::Return
            ) {
                return;
            }

            token = self.advance();
        }
    }
}

mod tests {
    use crate::position::Diagnostic;

    use super::*;

    fn parse_str(data: &str) -> Result<WithSpan<Expr>, Vec<Diagnostic>> {
        use super::super::tokenizer::*;

        let tokens = tokenize(data);
        let mut parser = crate::parser::Parser::new(&tokens);
        match parser.parse_expr() {
            Some(e) => Ok(e),
            None => Err(parser.diagnostics().to_vec()),
        }
    }

    fn assert_errs(data: &str, errs: &[&str]) {
        let x = parse_str(data);
        assert!(x.is_err());
        let diagnostics = x.unwrap_err();
        for diag in diagnostics {
            assert!(errs.contains(&diag.message.as_str()), "{}", diag.message);
        }
    }

    #[test]
    fn debug_test() {
        dbg!(parse_str("1)"));
    }
}
