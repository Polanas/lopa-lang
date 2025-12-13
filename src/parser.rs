use crate::{
    position,
    token::{Token, TokenKind},
};

static EOF_TOKEN: position::WithSpan<Token> = position::WithSpan::empty(Token::EOF);

pub struct Parser<'t> {
    tokens: &'t [position::WithSpan<Token>],
    cursor: usize,
    diagnostics: Vec<position::Diagnostic>,
}

impl<'t> Parser<'t> {
    pub fn new(tokens: &'t [position::WithSpan<Token>]) -> Self {
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

    pub fn peek_token(&self) -> &'t position::WithSpan<Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => token,
            None => &EOF_TOKEN,
        }
    }

    pub fn peek(&self) -> TokenKind {
        (&self.peek_token().value).into()
    }

    pub fn check_token(&self, match_token: TokenKind) -> bool {
        self.peek() == match_token
    }

    pub fn advance(&mut self) -> &'t position::WithSpan<Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => {
                self.cursor += 1;
                token
            }
            None => &EOF_TOKEN,
        }
    }

    pub fn expect(&mut self, expected: TokenKind) -> Option<&'t position::WithSpan<Token>> {
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

    pub fn expect_optional(&mut self, expected: TokenKind) -> Option<bool> {
        let token = self.peek();
        if token == expected {
            self.expect(expected)?;
            Some(true)
        } else {
            Some(false)
        }
    }
}
