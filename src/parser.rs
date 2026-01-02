use crate::{
    ast::{self, *},
    common::*,
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
    fn new(tokens: &'t [WithSpan<Token>]) -> Self {
        Self {
            tokens,
            cursor: 0,
            diagnostics: Default::default(),
        }
    }

    fn diagnostics(&self) -> &[position::Diagnostic] {
        &self.diagnostics
    }

    fn add_error(&mut self, message: &str, span: position::Span) {
        self.diagnostics.push(position::Diagnostic {
            span,
            message: message.to_owned(),
        });
    }

    fn peek_token(&self) -> &'t WithSpan<Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => token,
            None => &EOF_TOKEN,
        }
    }

    fn last_parsed(&self) -> TokenKind {
        self.tokens[self.cursor].value.kind()
    }

    fn peek(&self) -> TokenKind {
        (&self.peek_token().value).into()
    }

    fn check(&self, match_token: TokenKind) -> bool {
        self.peek() == match_token
    }

    fn advance(&mut self) -> &'t WithSpan<Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => {
                self.cursor += 1;
                token
            }
            None => &EOF_TOKEN,
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> Option<&'t WithSpan<Token>> {
        let check = self.check(kind);
        if check { Some(self.advance()) } else { None }
    }

    fn match_tokens(&mut self, kinds: &[TokenKind]) -> Option<&'t WithSpan<Token>> {
        for kind in kinds {
            if self.check(*kind) {
                return Some(self.advance());
            }
        }
        None
    }

    fn previous(&self) -> Option<&'t WithSpan<Token>> {
        self.tokens.get(self.cursor - 1)
    }

    fn expect(&mut self, expected: TokenKind) -> Option<&'t WithSpan<Token>> {
        let token = self.advance();
        if TokenKind::from(&token.value) == expected {
            Some(token)
        } else {
            self.add_error(
                &format!(
                    "Expected {}, got {}",
                    expected,
                    TokenKind::from(&token.value)
                ),
                token.span,
            );
            None
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek() == TokenKind::EOF
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Precedence {
    Lowest,
    NillCoaescing,
    Or,
    And,
    Comparison, // <, <=, >, >=, ==, !=
    BitwiseOR,
    BitwiseXOR,
    BitwiseAND,
    BitwiseShift,
    Term,   // + -
    Factor, // * / %
    Unary,  // ! -
    Path,
}

impl From<TokenKind> for Precedence {
    fn from(value: TokenKind) -> Self {
        match value {
            TokenKind::Bar2 => Self::Or,
            TokenKind::Ampersand2 => Self::And,
            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::BangEqual
            | TokenKind::Equal2 => Self::Comparison,
            TokenKind::Plus | TokenKind::Minus => Self::Term,
            TokenKind::Star | TokenKind::Slash => Self::Factor,
            TokenKind::Bang => Self::Unary,
            _ => Self::Lowest,
        }
    }
}

// macro_rules! parse_binary_fn {
//     ($name: ident, $recursive: ident, $tokens: expr) => {
//         pub fn $name(&mut self) -> Option<WithSpan<Expr>> {
//             let mut expr = self.$recursive()?;
//
//             while let Some(op) = self.match_tokens($tokens) {
//                 let right = self.$recursive()?;
//                 let span = expr.span.union(right.span);
//                 expr = WithSpan::new(
//                     Expr::Binary(BinaryExpr {
//                         left: expr.into(),
//                         right: right.into(),
//                         op: WithSpan::new(BinaryOperator::from_token(&op.value).unwrap(), op.span),
//                     }),
//                     span,
//                 );
//             }
//             Some(expr)
//         }
//     };
// }

impl<'t> Parser<'t> {
    // pub fn parse_expr(&mut self) -> Option<WithSpan<Expr>> {
    //     self.parse_eq()
    // }
    //
    // parse_binary_fn!(
    //     parse_eq,
    //     parse_comparison,
    //     &[TokenKind::BangEqual, TokenKind::Equal2]
    // );
    // parse_binary_fn!(
    //     parse_comparison,
    //     parse_term,
    //     &[
    //         TokenKind::Greater,
    //         TokenKind::GreaterEqual,
    //         TokenKind::Less,
    //         TokenKind::LessEqual,
    //     ]
    // );
    // parse_binary_fn!(
    //     parse_term,
    //     parse_factor,
    //     &[TokenKind::Minus, TokenKind::Plus]
    // );
    // parse_binary_fn!(
    //     parse_factor,
    //     parse_unary,
    //     &[TokenKind::Slash, TokenKind::Star]
    // );
    //
    // pub fn parse_unary(&mut self) -> Option<WithSpan<Expr>> {
    //     if let Some(op) = self.match_tokens(&[TokenKind::Bang, TokenKind::Minus]) {
    //         let right = self.parse_unary()?;
    //         let span = op.span.union(right.span);
    //         Some(WithSpan::new(
    //             Expr::Unary(
    //                 WithSpan::new(UnaryOp::from_token(&op.value).unwrap(), op.span),
    //                 right.into(),
    //             ),
    //             span,
    //         ))
    //     } else {
    //         self.parse_primary()
    //     }
    // }
    //
    // pub fn parse_primary(&mut self) -> Option<WithSpan<Expr>> {
    //     if let Some(t) = self.match_token(TokenKind::False) {
    //         Some(WithSpan::new(Expr::Bool(false), t.span))
    //     } else if let Some(t) = self.match_token(TokenKind::True) {
    //         Some(WithSpan::new(Expr::Bool(true), t.span))
    //     } else if let Some(t) = self.match_token(TokenKind::Nil) {
    //         Some(WithSpan::new(Expr::Nil, t.span))
    //     } else if let Some(t) = self.match_token(TokenKind::String) {
    //         Some(WithSpan::new(
    //             Expr::String(match &t.value {
    //                 Token::String(s) => s.clone(),
    //                 _ => unreachable!(),
    //             }),
    //             t.span,
    //         ))
    //     } else if let Some(t) = self.match_token(TokenKind::Number) {
    //         Some(WithSpan::new(
    //             Expr::Number(match &t.value {
    //                 Token::Number(n) => match n {
    //                     token::NumberToken::Int(i) => Number::Int(*i),
    //                     token::NumberToken::Float(f) => Number::Float(*f),
    //                 },
    //                 _ => unreachable!(),
    //             }),
    //             t.span,
    //         ))
    //     } else if let Some(t) = self.match_token(TokenKind::Identifier) {
    //         Some(WithSpan::new(
    //             Expr::Identifier(if let Token::Identifier(i) = &t.value {
    //                 i.clone()
    //             } else {
    //                 unreachable!()
    //             }),
    //             t.span,
    //         ))
    //     } else if self.match_token(TokenKind::LeftParen).is_some() {
    //         let expr = self.parse_expr()?;
    //         let span = expr.span;
    //         self.expect(TokenKind::RightParen)?;
    //         Some(WithSpan::new(Expr::Grouping(expr.into()), span))
    //     } else if let Some(t) = self.match_token(TokenKind::LeftBrace) {
    //         self.parse_block_expr(t)
    //     } else {
    //         let token = self.advance();
    //         self.add_error(
    //             &format!(
    //                 "Expected expression, got {}",
    //                 TokenKind::from(&token.value)
    //             ),
    //             token.span,
    //         );
    //         None
    //     }
    // }
    //
    // pub fn parse_block_expr(&mut self, left_brace: &WithSpan<Token>) -> Option<WithSpan<Expr>> {
    //     let mut stmts = vec![];
    //     let right_brace = loop {
    //         if self.is_at_end() {
    //             return None;
    //         }
    //         if let Some(t) = self.match_token(TokenKind::RightBrace) {
    //             break t;
    //         }
    //         stmts.push(self.parse_stmt()?);
    //     };
    //
    //     Some(WithSpan::new(
    //         Expr::Block(stmts),
    //         left_brace.span.union(right_brace.span),
    //     ))
    // }
    //

    fn sync(&mut self) {
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

    fn parse_expr(&mut self, precedence: Precedence) -> Option<WithSpan<Expr>> {
        let mut expr = self.parse_prefix()?;
        while !self.is_at_end() {
            let next_precedence = Precedence::from(self.peek());
            if precedence >= next_precedence {
                break;
            }

            expr = self.parse_infix(expr)?;
        }

        Some(expr)
    }

    fn parse_infix(&mut self, left: WithSpan<Expr>) -> Option<WithSpan<Expr>> {
        let token = self.peek_token();
        match token.value {
            Token::BangEqual
            | Token::Equal2
            | Token::Bar2
            | Token::Ampersand2
            | Token::Less
            | Token::LessEqual
            | Token::Greater
            | Token::GreaterEqual
            | Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent => self.parse_binary(left),
            _ => {
                self.add_error(
                    &format!("Unexpected {}", TokenKind::from(&token.value)),
                    token.span,
                );
                None
            }
        }
    }

    fn parse_index(&mut self) -> Option<WithSpan<Expr>> {
        None
    }

    fn parse_call(&mut self) -> Option<WithSpan<Expr>> {
        None
    }

    fn parse_prefix(&mut self) -> Option<WithSpan<Expr>> {
        match self.peek() {
            TokenKind::Number
            | TokenKind::Nil
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Identifier
            | TokenKind::String => self.parse_primary(),
            TokenKind::Bang | TokenKind::Minus => self.parse_unary(),
            TokenKind::LeftParen => self.parse_grouping(),
            TokenKind::LeftBrace => self.parse_block(),
            TokenKind::If => self.parse_if(),
            _ => {
                self.add_error(
                    &format!("Unexpected {}", self.peek()),
                    self.peek_token().span,
                );
                None
            }
        }
    }

    fn parse_if(&mut self) -> Option<WithSpan<Expr>> {
        let if_token = self.expect(TokenKind::If)?;
        let condition = self.parse_expr(Precedence::Lowest)?;
        let WithSpan {
            value: Expr::Block(then_branch, ..),
            span,
        } = self.parse_block()?
        else {
            unreachable!()
        };

        let else_branch = if self.match_token(TokenKind::Else).is_some() {
            Some(self.parse_expr(Precedence::Lowest)?.into())
        } else {
            None
        };

        Some(WithSpan::new(
            Expr::If(IfExpr {
                condition: condition.into(),
                then_branch,
                else_branch,
                ty: None,
            }),
            if_token.span.union(span),
        ))
    }

    fn parse_binary_op(&mut self) -> Option<WithSpan<BinaryOp>> {
        let token = self.advance();
        let op = match &token.value {
            Token::BangEqual => BinaryOp::NotEqual,
            Token::Equal2 => BinaryOp::Equal,
            Token::Less => BinaryOp::Less,
            Token::LessEqual => BinaryOp::LessEqual,
            Token::Greater => BinaryOp::Greater,
            Token::GreaterEqual => BinaryOp::GreaterEqual,
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
            Token::Star => BinaryOp::Mult,
            Token::Slash => BinaryOp::Div,
            Token::Percent => BinaryOp::Modulo,
            Token::Bar2 => BinaryOp::Or,
            Token::Ampersand2 => BinaryOp::And,
            _ => {
                self.add_error(
                    &format!("Unexpected {}", TokenKind::from(&token.value)),
                    token.span,
                );
                return None;
            }
        };

        Some(WithSpan::new(op, token.span))
    }

    fn parse_grouping(&mut self) -> Option<WithSpan<Expr>> {
        let left_paren = self.expect(TokenKind::LeftParen)?;
        let expr = self.parse_expr(Precedence::Lowest)?;
        let right_paren = self.expect(TokenKind::RightParen)?;

        let span = left_paren.span.union(right_paren.span);
        Some(WithSpan::new(Expr::Grouping(expr.into()), span))
    }

    fn parse_unary(&mut self) -> Option<WithSpan<Expr>> {
        let op = self.parse_unary_op()?;
        let right = self.parse_expr(Precedence::Unary)?;
        let span = op.span.union(right.span);
        Some(WithSpan::new(
            Expr::Unary(UnaryExpr {
                expr: right.into(),
                op,
                ty: None,
            }),
            span,
        ))
    }

    fn parse_unary_op(&mut self) -> Option<WithSpan<UnaryOp>> {
        let token = self.advance();
        match &token.value {
            Token::Bang => Some(WithSpan::new(UnaryOp::Not, token.span)),
            Token::Minus => Some(WithSpan::new(UnaryOp::Negate, token.span)),
            _ => {
                self.add_error(
                    &format!("Unexpected {}", TokenKind::from(&token.value)),
                    token.span,
                );
                None
            }
        }
    }

    fn parse_binary(&mut self, left: WithSpan<Expr>) -> Option<WithSpan<Expr>> {
        let precedence = Precedence::from(self.peek());
        let op = self.parse_binary_op()?;
        let right = self.parse_expr(precedence)?;
        let span = left.span.union(right.span);
        Some(WithSpan::new(
            Expr::Binary(BinaryExpr {
                left: left.into(),
                right: right.into(),
                op,
                ty: None,
            }),
            span,
        ))
    }

    fn parse_primary(&mut self) -> Option<WithSpan<Expr>> {
        let token = self.advance();
        match &token.value {
            &Token::Nil => Some(WithSpan::new(Expr::Nil, token.span)),
            &Token::Number(token::NumberToken::Int(i)) => {
                Some(WithSpan::new(Expr::Number(Number::Int(i)), token.span))
            }
            &Token::Number(token::NumberToken::Float(f)) => {
                Some(WithSpan::new(Expr::Number(Number::Float(f)), token.span))
            }
            &Token::True => Some(WithSpan::new(Expr::Bool(true), token.span)),
            &Token::False => Some(WithSpan::new(Expr::Bool(false), token.span)),
            Token::String(s) => Some(WithSpan::new(Expr::String(s.clone()), token.span)),
            Token::Identifier(s) => {
                Some(WithSpan::new(Expr::Identifier(s.clone(), None), token.span))
            }
            _ => {
                self.add_error(
                    &format!("Unexpected {}", TokenKind::from(&token.value)),
                    token.span,
                );
                None
            }
        }
    }

    fn parse_block(&mut self) -> Option<WithSpan<Expr>> {
        let left_brace = self.expect(TokenKind::LeftBrace)?;
        let mut stmts = vec![];
        let right_brace = loop {
            if self.is_at_end() {
                return None;
            }
            if let Some(t) = self.match_token(TokenKind::RightBrace) {
                break t;
            }
            stmts.push(self.parse_stmt()?);
        };

        Some(WithSpan::new(
            Expr::Block(stmts, None),
            left_brace.span.union(right_brace.span),
        ))
    }

    fn parse_stmt(&mut self) -> Option<WithSpan<Stmt>> {
        match self.peek() {
            TokenKind::Let | TokenKind::Global => self.parse_binding(),
            TokenKind::Print => self.parse_print(),
            TokenKind::Semicolon => {
                let semi = self.expect(TokenKind::Semicolon)?;
                Some(WithSpan::new(Stmt::Empty, semi.span))
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_print(&mut self) -> Option<WithSpan<Stmt>> {
        let _print = self.expect(TokenKind::Print);
        let expr = self.parse_expr(Precedence::Lowest)?;
        let span = self
            .parse_optional_semi()
            .map(|s| expr.span.union(s))
            .unwrap_or(expr.span);
        Some(WithSpan::new(Stmt::Print(expr.into()), span))
    }

    fn parse_expr_stmt(&mut self) -> Option<WithSpan<Stmt>> {
        let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
        let mut span = exprs[0].span;
        while let TokenKind::Comma = self.peek() {
            self.expect(TokenKind::Comma);
            if let Some(semi) = self.parse_optional_semi() {
                let span = span.union(semi);
                return Some(WithSpan::new(
                    Stmt::Expr(StmtExpr {
                        exprs,
                        semi: Some(semi),
                    }),
                    span,
                ));
            }

            let expr = self.parse_expr(Precedence::Lowest)?;
            span = span.union(expr.span);
            exprs.push(expr);
        }

        if TokenKind::Equal == self.peek() {
            self.expect(TokenKind::Equal);
            let idents = exprs
                .into_iter()
                .map(|e| {
                    if let WithSpan {
                        value: Expr::Identifier(i, None),
                        span,
                    } = e
                    {
                        WithSpan::new(i, span)
                    } else {
                        panic!("expected identifier");
                    }
                })
                .collect::<Vec<_>>();
            let mut values = vec![self.parse_expr(Precedence::Lowest)?];
            span = span.union(values[0].span);
            while let TokenKind::Comma = self.peek() {
                self.expect(TokenKind::Comma);
                if let Some(semi) = self.parse_optional_semi() {
                    let span = span.union(semi);
                    return Some(WithSpan::new(
                        Stmt::Assign(Assign {
                            idents,
                            values: Some(values),
                            types: None,
                        }),
                        span,
                    ));
                }

                let value = self.parse_expr(Precedence::Lowest)?;
                span = span.union(value.span);
                values.push(value);
            }

            let semi = self.parse_optional_semi();
            let span = semi.map(|s| span.union(s)).unwrap_or(span);
            return Some(WithSpan::new(
                Stmt::Assign(Assign {
                    idents,
                    values: Some(values),
                    types: None,
                }),
                span,
            ));
        }

        let semi = self.parse_optional_semi();
        let span = semi.map(|s| span.union(s)).unwrap_or(span);
        Some(WithSpan::new(Stmt::Expr(StmtExpr { exprs, semi }), span))
    }

    fn parse_optional_semi(&mut self) -> Option<position::Span> {
        match self.peek() {
            TokenKind::Semicolon => {
                let semi = self.advance();
                Some(semi.span)
            }
            _ => None,
        }
    }

    fn parse_binding(&mut self) -> Option<WithSpan<Stmt>> {
        let binding_token = self.advance();
        let binding_type = match binding_token.value {
            Token::Let => BindingKind::Local,
            Token::Global => BindingKind::Global,
            _ => unreachable!(),
        };

        let WithSpan {
            value: Token::Identifier(ident),
            span: ident_span,
        } = self.advance()
        else {
            return None;
        };
        let mut identifiers = vec![WithSpan::new(ident.clone(), *ident_span)];
        let mut span = binding_token.span.union(*ident_span);

        while let TokenKind::Comma = self.peek() {
            self.expect(TokenKind::Comma);
            let WithSpan {
                value: Token::Identifier(ident),
                span: ident_span,
            } = self.advance()
            else {
                panic!("expected identifier");
            };

            span = span.union(*ident_span);
            identifiers.push(WithSpan::new(ident.clone(), *ident_span))
        }

        if self.match_token(TokenKind::Equal).is_some() {
            let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
            let mut span = span.union(exprs[0].span);
            while let TokenKind::Comma = self.peek() {
                self.expect(TokenKind::Comma);
                if let Some(semi) = self.parse_optional_semi() {
                    span = span.union(semi);

                    return Some(WithSpan::new(
                        Stmt::Binding(Binding {
                            kind: binding_type,
                            idents: identifiers,
                            values: Some(exprs),
                            types: None,
                        }),
                        span,
                    ));
                }
                let expr = self.parse_expr(Precedence::Lowest)?;
                span = span.union(expr.span);
                exprs.push(expr);
            }

            let span = self
                .parse_optional_semi()
                .map(|s| span.union(s))
                .unwrap_or(span);

            Some(WithSpan::new(
                Stmt::Binding(Binding {
                    kind: binding_type,
                    idents: identifiers,
                    values: Some(exprs),
                    types: None,
                }),
                span,
            ))
        } else {
            let span = self
                .parse_optional_semi()
                .map(|s| span.union(s))
                .unwrap_or(span);

            Some(WithSpan::new(
                Stmt::Binding(Binding {
                    kind: binding_type,
                    idents: identifiers,
                    values: None,
                    types: None,
                }),
                span,
            ))
        }
    }

    fn parse_program(&mut self) -> Option<Vec<WithSpan<Stmt>>> {
        let mut statements = vec![];

        while !self.is_at_end() {
            if let Some(stmt) = self.parse_stmt() {
                statements.push(stmt);
            } else {
                self.sync();
            }
        }

        if self.diagnostics.is_empty() {
            Some(statements)
        } else {
            None
        }
    }
}

pub fn parse_program(
    tokens: &[WithSpan<Token>],
) -> Result<Vec<WithSpan<Stmt>>, Vec<position::Diagnostic>> {
    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Some(output) => Ok(output),
        None => Err(parser.diagnostics),
    }
}

mod tests {
    use crate::position::Diagnostic;

    use super::*;

    fn parse_str(data: &str) -> Result<WithSpan<Expr>, Vec<Diagnostic>> {
        use super::super::tokenizer::*;

        let tokens = tokenize(data);
        let mut parser = crate::parser::Parser::new(&tokens);
        match parser.parse_expr(Precedence::Lowest) {
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
}
