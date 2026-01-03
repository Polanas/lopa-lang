use crate::{
    ast::*,
    common::*,
    position::{self, WithSpan},
    token::{self, Token, TokenKind},
    types,
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
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Self::Factor,
            TokenKind::Bang => Self::Unary,
            _ => Self::Lowest,
        }
    }
}

impl<'t> Parser<'t> {
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
            value: Expr::Block(then_branch),
            span: then_branch_span,
        } = self.parse_block()?
        else {
            unreachable!()
        };

        let mut span = if_token.span.union(then_branch_span);
        let else_branch = if self.match_token(TokenKind::Else).is_some() {
            let expr = if self.peek() == TokenKind::If {
                self.parse_if()?
            } else {
                self.parse_block()?
            };
            span = span.union(expr.span);
            Some(expr.into())
        } else {
            None
        };

        Some(WithSpan::new(
            Expr::If(IfExpr {
                condition: condition.into(),
                then_branch: WithSpan::new(then_branch, span),
                else_branch,
                ty: None,
            }),
            span,
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
                types: None,
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
            Expr::Block(Block {
                body: stmts,
                ty: None,
            }),
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
            TokenKind::Fn => self.parse_fn(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_fn(&mut self) -> Option<WithSpan<Stmt>> {
        let fn_token = self.expect(TokenKind::Fn)?;
        let WithSpan {
            value: Token::Identifier(name),
            span: name_span,
        } = self.expect(TokenKind::Identifier)?
        else {
            unreachable!()
        };
        self.expect(TokenKind::LeftParen);
        let mut params = vec![];
        while self.peek() != TokenKind::RightParen {
            let WithSpan {
                value: Token::Identifier(ident),
                span: name_span,
            } = self.expect(TokenKind::Identifier)?
            else {
                unreachable!()
            };

            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;

            if self.peek() == TokenKind::Comma {
                self.expect(TokenKind::Comma)?;
            }

            params.push(FnParam {
                kind: FnParamKind::Regular,
                ty,
                name: WithSpan::new(ident.clone(), *name_span),
            });
        }
        let right_paren = self.expect(TokenKind::RightParen)?;

        let returns = if self.peek() == TokenKind::Arrow {
            self.expect(TokenKind::Arrow);
            let mut returns = vec![];
            loop {
                let ty = self.parse_type()?;
                returns.push(ty);
                if self.peek() == TokenKind::Comma {
                    self.expect(TokenKind::Comma);
                } else {
                    break;
                }
            }
            returns
        } else {
            vec![]
        };

        let body = self.parse_block()?;
        Some(WithSpan::new(
            Stmt::Item(Item::Fn(Fn {
                name: name.clone(),
                params,
                body: match body.value {
                    Expr::Block(block) => WithSpan::new(block, body.span),
                    _ => unreachable!(),
                },
                returns,
            })),
            fn_token.span.union(body.span),
        ))
    }

    fn parse_print(&mut self) -> Option<WithSpan<Stmt>> {
        let print = self.expect(TokenKind::Print)?;
        let expr = self.parse_expr(Precedence::Lowest)?;
        let semi = self.expect(TokenKind::Semicolon)?;
        Some(WithSpan::new(
            Stmt::Print(expr.into()),
            print.span.union(semi.span),
        ))
    }

    fn parse_expr_stmt(&mut self) -> Option<WithSpan<Stmt>> {
        let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
        let mut span = exprs[0].span;
        while let TokenKind::Comma = self.peek() {
            self.expect(TokenKind::Comma);
            if self.peek() == TokenKind::Semicolon {
                let semi = self.expect(TokenKind::Semicolon)?;
                let span = span.union(semi.span);
                return Some(WithSpan::new(
                    Stmt::Expr(StmtExpr {
                        exprs,
                        semi: Some(semi.span),
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
                if self.peek() == TokenKind::Semicolon {
                    let semi = self.expect(TokenKind::Semicolon)?;
                    let span = span.union(semi.span);
                    return Some(WithSpan::new(
                        Stmt::Assign(Assign {
                            idents,
                            values: Some(values),
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
                }),
                span,
            ));
        }

        let semi = self.parse_optional_semi();
        let span = semi.map(|s| span.union(s)).unwrap_or(span);
        Some(WithSpan::new(Stmt::Expr(StmtExpr { exprs, semi }), span))
    }

    fn parse_optional_mark(&mut self) -> Option<position::Span> {
        match self.peek() {
            TokenKind::QuestionMark => {
                let mark = self.advance();
                Some(mark.span)
            }
            _ => None,
        }
    }

    fn parse_optional_semi(&mut self) -> Option<position::Span> {
        match self.peek() {
            TokenKind::Semicolon => {
                let mark = self.advance();
                Some(mark.span)
            }
            _ => None,
        }
    }

    fn parse_type(&mut self) -> Option<WithSpan<types::Type>> {
        let WithSpan {
            value: value @ (Token::Identifier(_) | Token::Nil),
            span,
        } = self.advance()
        else {
            let token = &self.tokens[self.cursor - 1];
            self.add_error(
                &format!("expected type, got {}", token.value.kind()),
                token.span,
            );
            return None;
        };
        let ident = match value {
            Token::Identifier(ident) => ident,
            Token::Nil => "nil",
            _ => unreachable!(),
        };

        let mark = self.parse_optional_mark();
        let span = mark.map(|s| span.union(s)).unwrap_or(*span);
        Some(WithSpan::new(
            types::Type {
                kind: types::TypeKind::from_ident(ident),
                nilable: mark.is_some(),
            },
            span,
        ))
    }

    fn parse_binding(&mut self) -> Option<WithSpan<Stmt>> {
        let binding_token = self.advance();
        let binding_type = match binding_token.value {
            Token::Let => BindingKind::Local,
            Token::Global => BindingKind::Global,
            _ => unreachable!(),
        };

        let mut identifiers = vec![];
        let mut types = vec![];
        let mut span = binding_token.span;

        loop {
            let WithSpan {
                value: Token::Identifier(ident),
                span: ident_span,
            } = self.advance()
            else {
                panic!("expected identifier");
            };

            span = span.union(*ident_span);
            identifiers.push(WithSpan::new(ident.clone(), *ident_span));

            if self.peek() == TokenKind::Colon {
                self.expect(TokenKind::Colon);
                let ty = self.parse_type()?;
                types.push(Some(ty));
            } else {
                types.push(None);
            }

            if self.peek() == TokenKind::Comma {
                self.expect(TokenKind::Comma);
            } else {
                break;
            }
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
                            types,
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
                    types,
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
                    types,
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
