use crate::{
    Token,
    ast::*,
    common::*,
    position::{self, Span, Spanned, WithSpan},
    token::{self, StringToken, Token, TokenKind},
    tokenizer::tokenize,
};

static EOF_TOKEN: &Token = &Token::EOF;

pub struct Parser<'t> {
    tokens: &'t [WithSpan<Token>],
    cursor: usize,
    diagnostics: Vec<position::Diagnostic>,
    last_id: AstNodeId,
}

impl<'t> Parser<'t> {
    fn new(tokens: &'t [WithSpan<Token>]) -> Self {
        Self {
            tokens,
            cursor: 0,
            diagnostics: Default::default(),
            last_id: AstNodeId(0),
        }
    }

    fn id(&mut self) -> AstNodeId {
        let id = self.last_id;
        self.last_id += 1.into();
        id
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

    fn peek_token(&self) -> WithSpan<&'t Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => WithSpan::new(&token.value, token.span),
            None => WithSpan::empty(EOF_TOKEN),
        }
    }

    fn peek(&self) -> TokenKind {
        self.peek_token().value.into()
    }

    fn peek_next(&self) -> TokenKind {
        match self.tokens.get(self.cursor + 1) {
            Some(token) => token.value.kind(),
            None => EOF_TOKEN.kind(),
        }
    }

    fn check(&self, match_token: TokenKind) -> bool {
        self.peek() == match_token
    }

    fn advance(&mut self) -> WithSpan<&'t Token> {
        match self.tokens.get(self.cursor) {
            Some(token) => {
                self.cursor += 1;
                WithSpan::new(&token.value, token.span)
            }
            None => WithSpan::empty(EOF_TOKEN),
        }
    }

    fn matches(&mut self, kind: TokenKind) -> Option<WithSpan<&'t Token>> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Option<WithSpan<&'t Token>> {
        let token = self.advance();
        if TokenKind::from(token.value) == expected {
            Some(WithSpan::new(&token.value, token.span))
        } else {
            self.add_error(
                &format!(
                    "Expected {}, got {}",
                    expected,
                    TokenKind::from(token.value)
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
    Else,
    NilCoalescing,
    Or,
    And,
    Comparison,   // <, <=, >, >=, ==, !=
    BitwiseOr,    // |
    BitwiseXor,   // ^
    BitwiseAnd,   // &
    BitwiseShift, // <<, >>
    Term,         // + -
    Factor,       // * / % //
    Unary,        // ! -
    Call,         // a()
    Index,        // []
    Path,
}

impl From<TokenKind> for Precedence {
    fn from(value: TokenKind) -> Self {
        match value {
            Token![else] => Self::Else,
            Token![or] => Self::Or,
            Token![and] => Self::And,
            Token![<] | Token![<=] | Token![>] | Token![>=] | Token![!=] | Token![==] => {
                Self::Comparison
            }
            Token![<<] | Token![>>] => Self::BitwiseShift,
            Token![&] => Self::BitwiseOr,
            Token![^] => Self::BitwiseXor,
            Token![+] | Token![-] => Self::Term,
            Token![*] | Token![/] | Token![%] | TokenKind::Slash2 => Self::Factor,
            Token![!] => Self::Unary,
            TokenKind::LeftParen | Token![.] | Token![?.] => Self::Call,
            TokenKind::LeftBracket => Self::Index,
            _ => Self::Lowest,
        }
    }
}

impl Parser<'_> {
    fn sync(&mut self) {
        let mut token = self.advance();

        while !self.is_at_end() {
            if TokenKind::from(token.value) == TokenKind::Semicolon {
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
                    | TokenKind::Return
            ) {
                return;
            }

            token = self.advance();
        }
    }

    fn parse_ident(&mut self) -> Option<Ident> {
        if let Some(self_token) = self.matches(Token![self]) {
            return Some(Ident {
                value: "self".to_string(),
                span: self_token.span,
                id: self.id(),
            });
        }
        let WithSpan {
            value: Token::Ident(ident),
            span,
        } = self.expect(TokenKind::Ident)?
        else {
            unreachable!();
        };
        Some(Ident {
            span,
            id: self.id(),
            value: ident.clone(),
        })
    }

    fn parse_path(&mut self) -> Option<Path> {
        let ident = self.parse_ident()?;
        let mut span = ident.span;
        let mut segments = vec![PathSegment {
            span: ident.span,
            ident,
        }];
        while self.matches(Token![::]).is_some() {
            let ident = self.parse_ident()?;
            span = span.union(ident.span());
            segments.push(PathSegment {
                span: ident.span(),
                ident,
            });
        }
        Some(Path {
            segments,
            span,
            id: self.id(),
        })
    }

    fn parse_interpolated_string(
        &mut self,
        value: String,
        span: Span,
    ) -> Option<LitInterpolatedString> {
        let mut char_start_pos;
        let mut index_start_pos;
        let mut char_index = 0;
        let mut ranges = vec![];
        let chars_len = value.chars().count();
        loop {
            let ch = value.char_indices().nth(char_index).unwrap();
            if ch.1 == '{' {
                if let Some((_, '{')) = value.char_indices().nth(char_index + 1) {
                    char_index += 1;
                } else {
                    char_start_pos = char_index;
                    index_start_pos = ch.0;
                    let mut char_end_pos = char_start_pos + 1;
                    let index_end_pos;
                    let mut stack = 1;
                    loop {
                        let ch = value.char_indices().nth(char_end_pos).unwrap();
                        match ch.1 {
                            '{' => {
                                if let Some((_, '{')) = value.char_indices().nth(char_index + 1) {
                                    char_index += 1;
                                } else {
                                    stack += 1
                                }
                            }
                            '}' => {
                                if let Some((_, '}')) = value.char_indices().nth(char_index + 1) {
                                    char_index += 1;
                                } else {
                                    stack -= 1
                                }
                            }
                            _ => {}
                        };
                        if stack == 0 {
                            index_end_pos = ch.0;
                            break;
                        }
                        char_end_pos += 1;
                        if char_end_pos == chars_len {
                            self.add_error("expected }", span);
                            return None;
                        }
                    }

                    ranges.push((
                        char_start_pos..=char_end_pos,
                        index_start_pos..=index_end_pos,
                    ));
                }
            }
            char_index += 1;
            if char_index == chars_len {
                break;
            }
        }
        let mut chars = value.chars().collect::<Vec<_>>();
        let mut indices = vec![];
        for (range, _) in ranges.iter().rev() {
            let amount = chars.drain(range.clone()).count();
            indices.push(*range.end());

            for index in &mut indices {
                *index = index.saturating_sub(amount);
            }
        }
        indices.reverse();
        let mut value_stripped: String = chars.into_iter().collect();
        let mut exprs = vec![];
        for ((_, range), index) in ranges.iter().zip(indices.iter()) {
            let range = (range.start() + 1)..=(range.end() - 1);
            let tokens_str = &value[range.clone()];
            let tokens = tokenize(tokens_str);
            let mut ast = Parser::new(&tokens);
            let Some(expr) = ast.parse_expr(Precedence::Lowest) else {
                for err in ast.diagnostics {
                    self.diagnostics.push(err);
                }
                return None;
            };
            exprs.push((*index, expr));
        }

        value_stripped = value_stripped.replace("{{", "{");
        value_stripped = value_stripped.replace("}}", "}");
        Some(LitInterpolatedString {
            exprs: Some(exprs),
            value: value_stripped,
        })
    }

    fn parse_struct_init(&mut self, path: Path) -> Option<Expr> {
        self.expect(TokenKind::LeftBrace)?;

        let mut fields = vec![];
        while !self.check(TokenKind::RightBrace) {
            let name = self.parse_ident()?;
            self.expect(Token![:])?;
            let expr = self.parse_expr(Precedence::Lowest)?;
            self.matches(Token![,]);

            fields.push(FieldValue {
                span: name.span.union(expr.span()),
                id: self.id(),
                name,
                expr: expr.into(),
            });
        }
        let right_brace = self.expect(TokenKind::RightBrace)?;

        Some(Expr::Struct(StructExpr {
            span: path.span.union(right_brace.span),
            id: self.id(),
            path,
            fields,
        }))
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.peek() {
            Token![nil] => {
                let token = self.expect(Token![nil])?;
                Some(Expr::Lit(LitExpr::Nil(LitNil {
                    span: token.span,
                    id: self.id(),
                })))
            }
            Token![true] | Token![false] => {
                let token = self.advance();
                Some(Expr::Lit(LitExpr::Bool(LitBool {
                    value: match &token.value {
                        Token::True => true,
                        Token::False => false,
                        _ => unreachable!(),
                    },
                    span: token.span,
                    id: self.id(),
                })))
            }
            TokenKind::Number => {
                let WithSpan {
                    value: Token::Number(number),
                    span,
                } = self.expect(TokenKind::Number)?
                else {
                    unreachable!();
                };
                Some(Expr::Lit(match number {
                    token::NumberToken::Int(i) => LitExpr::Int(LitInt {
                        value: *i,
                        span,
                        id: self.id(),
                    }),
                    token::NumberToken::Float(f) => LitExpr::Float(LitFloat {
                        value: *f,
                        span,
                        id: self.id(),
                    }),
                }))
            }
            TokenKind::Ident | Token![self] => {
                let mut path = self.parse_path()?;
                Some(if self.check(TokenKind::LeftBrace) {
                    self.parse_struct_init(path)?
                } else if path.segments.len() == 1 {
                    Expr::Ident(path.segments.remove(0).ident)
                } else {
                    Expr::Path(path)
                })
            }
            TokenKind::String => self.parse_string(),
            other => {
                let token = self.advance();
                self.add_error(&format!("unexpected {}", other), token.span);
                None
            }
        }
    }

    fn parse_string(&mut self) -> Option<Expr> {
        let WithSpan {
            value:
                Token::String(StringToken {
                    value,
                    kind,
                    interpolated,
                }),
            span,
        } = self.expect(TokenKind::String)?
        else {
            unreachable!()
        };
        Some(Expr::Lit(LitExpr::String(if *interpolated {
            let interpolated = self.parse_interpolated_string(value.clone(), span)?;
            LitString {
                value: value.clone(),
                kind: *kind,
                span,
                id: self.id(),
                interpolated: Some(interpolated),
            }
        } else {
            LitString {
                interpolated: None,
                value: value.clone(),
                kind: *kind,
                span,
                id: self.id(),
            }
        })))
    }

    fn parse_unary_op(&mut self) -> Option<WithSpan<UnaryOp>> {
        let token = self.advance();
        match &token.value {
            Token::Bang => Some(WithSpan::new(UnaryOp::Not, token.span)),
            Token::Minus => Some(WithSpan::new(UnaryOp::Negate, token.span)),
            _ => {
                self.add_error(
                    &format!("unexpected {}", TokenKind::from(token.value)),
                    token.span,
                );
                None
            }
        }
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        let op = self.parse_unary_op()?;
        let right = self.parse_expr(Precedence::Unary)?;
        let span = op.span.union(right.span());
        Some(Expr::Unary(UnaryExpr {
            expr: right.into(),
            op: op.value,
            id: self.id(),
            span,
        }))
    }

    fn parse_parens(&mut self) -> Option<Expr> {
        self.expect(TokenKind::LeftParen)?;
        let mut exprs = vec![];
        while !self.check(TokenKind::RightParen) {
            let expr = self.parse_expr(Precedence::Lowest)?;
            self.matches(Token![,]);
            exprs.push(expr);
        }
        self.expect(TokenKind::RightParen)?;
        Some(if exprs.len() == 1 {
            let expr = exprs.remove(0);
            Expr::Group(GroupExpr {
                span: expr.span(),
                expr: expr.into(),
                id: self.id(),
            })
        } else {
            Expr::Tuple(TupleExpr {
                span: exprs
                    .iter()
                    .fold(exprs[0].span(), |s, ty| s.union(ty.span())),
                id: self.id(),
                exprs,
            })
        })
    }

    fn parse_block(&mut self) -> Option<Expr> {
        let left_brace = self.expect(TokenKind::LeftBrace)?;
        let mut body = vec![];
        let right_brace = loop {
            if self.is_at_end() {
                return None;
            }
            if let Some(t) = self.matches(TokenKind::RightBrace) {
                break t;
            }
            body.push(self.parse_stmt()?);
        };

        Some(Expr::Block(BlockExpr {
            stmts: body,
            span: left_brace.span.union(right_brace.span),
            id: self.id(),
        }))
    }

    fn parse_if(&mut self) -> Option<Expr> {
        let if_token = self.expect(Token![if])?;
        let condition = self.parse_expr(Precedence::Lowest)?;
        let Expr::Block(value) = self.parse_block()? else {
            unreachable!()
        };
        let span = if_token.span.union(value.span());
        Some(Expr::If(IfExpr {
            condition: condition.into(),
            value,
            id: self.id(),
            span,
        }))
    }

    fn parse_optional_mark(&mut self) -> Option<position::Span> {
        match self.peek() {
            Token![?] => {
                let mark = self.advance();
                Some(mark.span)
            }
            _ => None,
        }
    }

    fn parse_attribs(&mut self) -> Option<Vec<Attrib>> {
        let mut attribs = vec![];
        while self.matches(Token![#]).is_some() {
            self.expect(TokenKind::LeftBracket)?;
            loop {
                let path = self.parse_path()?;
                if let [
                    PathSegment {
                        ident: Ident { value, .. },
                        ..
                    },
                ] = path.segments.as_slice()
                    && value == "operator"
                {
                    self.expect(TokenKind::LeftParen)?;
                    if let Some(op) = self.parse_attrib_op() {
                        let right = self.expect(TokenKind::RightParen)?;
                        attribs.push(Attrib::Operator(OperatorAttrib {
                            op,
                            id: self.id(),
                            span: path.span.union(right.span),
                        }));
                        if self.matches(TokenKind::RightBracket).is_some() {
                            break;
                        }
                        continue;
                    }

                    let token = self.advance();
                    self.add_error(
                        &format!("expected binary operator, got {}", token.value.kind()),
                        token.span,
                    );
                    return None;
                }
                let call_expr = if !self.check(TokenKind::LeftParen) {
                    CallExpr {
                        span: path.span,
                        func: Expr::Path(path).into(),
                        args: Default::default(),
                        id: self.id(),
                    }
                } else {
                    let Expr::Call(call_expr) = self.parse_call(Expr::Path(path))? else {
                        unreachable!();
                    };
                    call_expr
                };
                attribs.push(Attrib::Item(ItemAttrib {
                    span: call_expr.span,
                    id: self.id(),
                    expr: call_expr,
                }));

                self.matches(Token![,]);
                if self.matches(TokenKind::RightBracket).is_some() {
                    break;
                }
            }
        }
        Some(attribs)
    }

    fn parse_type(&mut self) -> Option<TypeExpr> {
        let ty = match self.peek() {
            Token![nil] => {
                let nil = self.expect(Token![nil])?;
                TypeExpr::Primitive(PrimitiveType {
                    span: nil.span,
                    value: Primitive::Nil,
                    id: self.id(),
                })
            }
            Token![fn] => {
                let fn_token = self.expect(Token![fn])?;
                let mut variadic_param = None;
                self.expect(TokenKind::LeftParen)?;
                let mut params = vec![];
                while !self.check(TokenKind::RightParen) {
                    if (self.peek(), self.peek()) == (Token![Self], Token![:]) {
                        let receiver = self.expect(Token![Self])?;
                        self.expect(Token![:])?;
                        params.push(BareFnParam::Receiver(Receiver {
                            span: receiver.span,
                            id: self.id(),
                        }));
                        continue;
                    }
                    let ident = if (self.peek(), self.peek_next()) == (TokenKind::Ident, Token![:])
                    {
                        let ident = self.parse_ident()?;
                        self.expect(Token![:])?;
                        Some(ident)
                    } else {
                        None
                    };
                    let variadic = self.matches(Token![...]);
                    let ty = self.parse_type()?;
                    self.matches(Token![,]);

                    let span = ident
                        .as_ref()
                        .map(|ident| ident.span.union(ty.span()))
                        .unwrap_or_else(|| ty.span());
                    if let Some(variadic) = &variadic {
                        variadic_param = Some(BareVariadic {
                            ident,
                            ty: ty.into(),
                            span: variadic.span.union(span),
                            id: self.id(),
                        });
                        break;
                    }

                    params.push(BareFnParam::Typed(BareFnParamTyped {
                        ident,
                        ty,
                        span,
                        id: self.id(),
                    }));
                }
                let right_paren = self.expect(TokenKind::RightParen)?;
                let output = self.parse_output()?;
                let span = fn_token.span.union(match &output {
                    ReturnType::None => right_paren.span,
                    ReturnType::Type(type_exprs) => type_exprs.last().unwrap().span(),
                });
                TypeExpr::BareFn(BareFnType {
                    params,
                    variadic: variadic_param,
                    output,
                    span,
                    id: self.id(),
                })
            }
            Token![Self] => {
                let self_type = self.expect(Token![Self])?;
                TypeExpr::Receiver(Receiver {
                    span: self_type.span,
                    id: self.id(),
                })
            }
            TokenKind::Ident => {
                let path = self.parse_path()?;
                let mark = self.parse_optional_mark();
                let ty = match &path.segments.as_slice() {
                    [
                        PathSegment {
                            ident: Ident { value, .. },
                            span,
                        },
                    ] if let Some(value) = Primitive::from_ident(value) => {
                        TypeExpr::Primitive(PrimitiveType {
                            span: *span,
                            value,
                            id: self.id(),
                        })
                    }
                    [..] => TypeExpr::Path(path),
                };
                match mark {
                    Some(_) => TypeExpr::Nilable(ty.into()),
                    None => ty,
                }
            }
            TokenKind::LeftBracket => {
                self.expect(TokenKind::LeftBracket)?;
                let ty = self.parse_type()?;
                self.expect(TokenKind::RightBracket)?;
                match self.parse_optional_mark() {
                    Some(_) => TypeExpr::Nilable(ty.into()),
                    None => ty,
                }
            }
            TokenKind::LeftParen => {
                self.expect(TokenKind::LeftParen)?;
                let mut types = vec![];
                while !self.check(TokenKind::RightParen) {
                    let ty = self.parse_type()?;
                    if self.check(TokenKind::Comma) {
                        self.expect(TokenKind::Comma)?;
                    }
                    types.push(ty);
                }
                self.expect(TokenKind::RightParen)?;
                if types.len() == 1 {
                    TypeExpr::Paren(types.remove(0).into())
                } else {
                    TypeExpr::Tuple(TupleType {
                        span: types
                            .iter()
                            .fold(types[0].span(), |s, ty| s.union(ty.span())),
                        types,
                        id: self.id(),
                    })
                }
            }
            _ => {
                let token = self.advance();
                self.add_error(
                    &format!("expected type, got {}", token.value.kind()),
                    token.span,
                );
                return None;
            }
        };
        if self.matches(Token![|]).is_some() {
            let right = self.parse_type()?;
            let span = ty.span().union(right.span());
            Some(TypeExpr::Union(UnionType {
                left: ty.into(),
                right: right.into(),
                span,
                id: self.id(),
            }))
        } else {
            Some(ty)
        }
    }

    fn parse_pat(&mut self) -> Option<Pat> {
        Some(match self.peek() {
            TokenKind::Ident => {
                let mut path = self.parse_path()?;
                if path.segments.len() == 1 {
                    let segment = path.segments.remove(0);
                    Pat::Ident(PatIdent {
                        span: segment.span(),
                        value: segment.ident,
                        id: self.id(),
                    })
                } else {
                    Pat::Path(path)
                }
            }
            TokenKind::LeftParen => {
                let left = self.expect(TokenKind::LeftParen)?;
                let pat = self.parse_pat()?;
                let right = self.expect(TokenKind::RightParen)?;
                Pat::Paren(PatParen {
                    pat: pat.into(),
                    span: left.span.union(right.span),
                    id: self.id(),
                })
            }
            _ => {
                let token = self.advance();
                self.add_error(
                    &format!("expected pattern, got {}", token.value.kind()),
                    token.span,
                );
                return None;
            }
        })
    }

    fn parse_closure(&mut self) -> Option<Expr> {
        let bar = self.expect(Token![|])?;
        let mut params = vec![];
        while !self.check(Token![|]) {
            let pat = self.parse_pat()?;

            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;

            let default_value = if self.matches(Token![=]).is_some() {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };
            self.matches(Token![,]);
            params.push(FnParam::Typed(
                FnParamTyped {
                    span: default_value
                        .as_ref()
                        .map(|d| pat.span().union(d.span()))
                        .unwrap_or_else(|| pat.span().union(ty.span())),
                    pat_type: PatType {
                        span: pat.span().union(ty.span()),
                        pat: pat.into(),
                        ty: ty.into(),
                        id: self.id(),
                    },
                    default_value,
                    id: self.id(),
                    attribs: Default::default(),
                }
                .into(),
            ));
        }
        self.expect(Token![|])?;
        let output = self.parse_output()?;
        let body = self.parse_expr(Precedence::Lowest)?;
        Some(Expr::Closure(ClosureExpr {
            span: bar.span.union(body.span()),
            params,
            body: body.into(),
            output,
            id: self.id(),
        }))
    }

    fn parse_for(&mut self) -> Option<Expr> {
        let for_keyword = self.expect(Token![for])?;
        let pat = self.parse_pat()?;
        self.expect(Token![in])?;
        let expr = self.parse_expr(Precedence::Lowest)?;
        let Expr::Block(body) = self.parse_block()? else {
            unreachable!();
        };

        Some(Expr::For(ForExpr {
            span: for_keyword.span.union(body.span),
            pat,
            expr: expr.into(),
            body,
            id: self.id(),
        }))
    }

    fn parse_while(&mut self) -> Option<Expr> {
        let while_keyword = self.expect(Token![while])?;
        let condition = self.parse_expr(Precedence::Lowest)?;
        let Expr::Block(body) = self.parse_block()? else {
            unreachable!();
        };

        Some(Expr::While(WhileExpr {
            span: while_keyword.span.union(body.span),
            body,
            id: self.id(),
            condition: condition.into(),
        }))
    }

    fn parse_loop(&mut self) -> Option<Expr> {
        let loop_keyword = self.expect(Token![loop])?;
        let Expr::Block(body) = self.parse_block()? else {
            unreachable!();
        };

        Some(Expr::Loop(LoopExpr {
            span: loop_keyword.span.union(body.span),
            body,
            id: self.id(),
        }))
    }

    fn parse_array(&mut self) -> Option<Expr> {
        let left = self.expect(TokenKind::LeftBracket)?;
        let mut elements = vec![];
        while !self.check(TokenKind::RightBracket) {
            elements.push(self.parse_expr(Precedence::Lowest)?);
            self.matches(Token![,]);
        }
        let right = self.expect(TokenKind::RightBracket)?;
        Some(Expr::Array(ArrayExpr {
            elements,
            id: self.id(),
            span: left.span.union(right.span),
        }))
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        match self.peek() {
            TokenKind::Number
            | Token![nil]
            | Token![true]
            | Token![false]
            | Token![self]
            | TokenKind::Ident
            | TokenKind::String => self.parse_primary(),
            Token![!] | Token![-] => self.parse_unary(),
            TokenKind::LeftParen => self.parse_parens(),
            TokenKind::LeftBrace => self.parse_block(),
            TokenKind::LeftBracket => self.parse_array(),
            Token![if] => self.parse_if(),
            Token![for] => self.parse_for(),
            Token![while] => self.parse_while(),
            Token![loop] => self.parse_loop(),
            Token![|] => self.parse_closure(),
            _ => {
                self.add_error(
                    &format!("unexpected {}", self.peek()),
                    self.peek_token().span,
                );
                None
            }
        }
    }

    fn parse_binary_assign_op(&mut self) -> Option<WithSpan<BinaryAssignOp>> {
        let token = self.advance();
        let op = match &token.value.kind() {
            Token![+=] => BinaryAssignOp::Add,
            Token![-=] => BinaryAssignOp::Sub,
            Token![*=] => BinaryAssignOp::Mul,
            Token![/=] => BinaryAssignOp::Div,
            TokenKind::Slash2Eq => BinaryAssignOp::DivInt,
            Token![%=] => BinaryAssignOp::Rem,
            Token![&=] => BinaryAssignOp::BitOr,
            Token![|=] => BinaryAssignOp::BitAnd,
            Token![^=] => BinaryAssignOp::BitXor,
            Token![>>=] => BinaryAssignOp::Shr,
            Token![<<=] => BinaryAssignOp::Shl,
            _ => {
                self.add_error(
                    &format!(
                        "expected binary assign op, got {}",
                        TokenKind::from(token.value)
                    ),
                    token.span,
                );
                return None;
            }
        };
        Some(WithSpan::new(op, token.span))
    }

    fn parse_attrib_op(&mut self) -> Option<BinaryOrAssignOp> {
        let token = self.advance();
        let op = match &token.value.kind() {
            Token![or] => BinaryOrAssignOp::Binary(BinaryOp::Or),
            Token![and] => BinaryOrAssignOp::Binary(BinaryOp::And),
            Token![else] => BinaryOrAssignOp::Binary(BinaryOp::Else),
            Token![!=] => BinaryOrAssignOp::Binary(BinaryOp::NotEqual),
            Token![==] => BinaryOrAssignOp::Binary(BinaryOp::Equal),
            Token![<] => BinaryOrAssignOp::Binary(BinaryOp::Less),
            Token![<=] => BinaryOrAssignOp::Binary(BinaryOp::LessEqual),
            Token![>] => BinaryOrAssignOp::Binary(BinaryOp::Greater),
            Token![>=] => BinaryOrAssignOp::Binary(BinaryOp::GreaterEqual),
            Token![+] => BinaryOrAssignOp::Binary(BinaryOp::Add),
            Token![-] => BinaryOrAssignOp::Binary(BinaryOp::Sub),
            Token![*] => BinaryOrAssignOp::Binary(BinaryOp::Mult),
            Token![/] => BinaryOrAssignOp::Binary(BinaryOp::Div),
            Token![%] => BinaryOrAssignOp::Binary(BinaryOp::Rem),
            Token![|] => BinaryOrAssignOp::Binary(BinaryOp::BitOr),
            Token![&] => BinaryOrAssignOp::Binary(BinaryOp::BitAnd),
            Token![^] => BinaryOrAssignOp::Binary(BinaryOp::BitXor),
            Token![>>] => BinaryOrAssignOp::Binary(BinaryOp::Shr),
            Token![<<] => BinaryOrAssignOp::Binary(BinaryOp::Shl),
            TokenKind::Slash2 => BinaryOrAssignOp::Binary(BinaryOp::DivInt),

            Token![+=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Add),
            Token![-=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Sub),
            Token![*=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Mul),
            Token![/=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Div),
            TokenKind::Slash2Eq => BinaryOrAssignOp::Assign(BinaryAssignOp::DivInt),
            Token![%=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Rem),
            Token![&=] => BinaryOrAssignOp::Assign(BinaryAssignOp::BitOr),
            Token![|=] => BinaryOrAssignOp::Assign(BinaryAssignOp::BitAnd),
            Token![^=] => BinaryOrAssignOp::Assign(BinaryAssignOp::BitXor),
            Token![>>=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Shr),
            Token![<<=] => BinaryOrAssignOp::Assign(BinaryAssignOp::Shl),
            _ => {
                self.add_error(
                    &format!("expected binary op, got {}", TokenKind::from(token.value)),
                    token.span,
                );
                return None;
            }
        };

        Some(op)
    }

    fn parse_binary_op(&mut self) -> Option<WithSpan<BinaryOp>> {
        let token = self.advance();
        let op = match &token.value.kind() {
            Token![or] => BinaryOp::Or,
            Token![and] => BinaryOp::And,
            Token![else] => BinaryOp::Else,
            Token![!=] => BinaryOp::NotEqual,
            Token![==] => BinaryOp::Equal,
            Token![<] => BinaryOp::Less,
            Token![<=] => BinaryOp::LessEqual,
            Token![>] => BinaryOp::Greater,
            Token![>=] => BinaryOp::GreaterEqual,
            Token![+] => BinaryOp::Add,
            Token![-] => BinaryOp::Sub,
            Token![*] => BinaryOp::Mult,
            Token![/] => BinaryOp::Div,
            Token![%] => BinaryOp::Rem,
            Token![|] => BinaryOp::BitOr,
            Token![&] => BinaryOp::BitAnd,
            Token![^] => BinaryOp::BitXor,
            Token![>>] => BinaryOp::Shr,
            Token![<<] => BinaryOp::Shl,
            TokenKind::Slash2 => BinaryOp::DivInt,
            _ => {
                self.add_error(
                    &format!("expected binary op, got {}", TokenKind::from(token.value)),
                    token.span,
                );
                return None;
            }
        };

        Some(WithSpan::new(op, token.span))
    }

    fn parse_binary(&mut self, left: Expr) -> Option<Expr> {
        let precedence = Precedence::from(self.peek());
        let op = self.parse_binary_op()?;
        let right = self.parse_expr(precedence)?;
        let span = left.span().union(right.span());

        Some(Expr::Binary(BinaryExpr {
            left: left.into(),
            right: right.into(),
            op: op.value,
            id: self.id(),
            span,
        }))
    }

    fn parse_call(&mut self, left: Expr) -> Option<Expr> {
        let mut span = self.expect(TokenKind::LeftParen)?.span;
        let mut args = vec![];
        loop {
            if self.check(TokenKind::RightParen) {
                break;
            }
            let arg = self.parse_expr(Precedence::Lowest)?;
            if self.matches(Token![:]).is_some() {
                let Expr::Ident(ident) = arg else {
                    self.add_error(&format!("expected identifier, got {}", &arg), arg.span());
                    return None;
                };
                let arg = self.parse_expr(Precedence::Lowest)?;
                args.push(FnArg {
                    name: Some(ident.clone()),
                    expr: arg.into(),
                });
            } else {
                args.push(FnArg {
                    expr: arg.into(),
                    name: None,
                });
            }

            self.matches(Token![,]);
        }
        span = span.union(self.expect(TokenKind::RightParen)?.span);
        Some(Expr::Call(CallExpr {
            func: left.into(),
            args,
            id: self.id(),
            span,
        }))
    }

    fn parse_index(&mut self, left: Expr) -> Option<Expr> {
        self.expect(TokenKind::LeftBracket)?;
        let expr = self.parse_expr(Precedence::Lowest)?;
        let right_braket = self.expect(TokenKind::RightBracket)?;
        Some(Expr::Index(IndexExpr {
            span: left.span().union(right_braket.span),
            indexed: left.into(),
            index: expr.into(),
            id: self.id(),
        }))
    }

    fn parse_get(&mut self, left: Expr) -> Option<Expr> {
        let mark = self.parse_opt_dot_mark()?;
        let ident = self.parse_ident()?;
        Some(if self.check(TokenKind::LeftParen) {
            let Expr::Call(CallExpr { args, span, .. }) =
                self.parse_call(Expr::Ident(ident.clone()))?
            else {
                unreachable!();
            };

            let span = left.span().union(span);
            Expr::MethodCall(MethodCallExpr {
                receiver: left.into(),
                method: ident,
                args,
                span,
                id: self.id(),
                optional: mark,
            })
        } else {
            let span = left.span().union(ident.span());
            Expr::FieldGet(FieldGetExpr {
                base: left.into(),
                member: Member::Named(NamedMember {
                    span: ident.span,
                    value: ident,
                    id: self.id(),
                }),
                id: self.id(),
                span,
                optional: mark,
            })
        })
    }

    fn parse_opt_dot_mark(&mut self) -> Option<bool> {
        let token = self.advance();
        let mark = match token.value.kind() {
            Token![?.] => true,
            Token![.] => false,
            other => {
                self.add_error(&format!("expected . or ?., got {}", other), token.span);
                return None;
            }
        };
        Some(mark)
    }

    fn parse_infix(&mut self, left: Expr) -> Option<Expr> {
        let token = self.peek_token();
        match token.value.kind() {
            Token![!=]
            | Token![==]
            | Token![or]
            | Token![and]
            | Token![else]
            | Token![<]
            | Token![<=]
            | Token![>]
            | Token![>=]
            | Token![+]
            | Token![-]
            | Token![*]
            | Token![/]
            | Token![%]
            | Token![^]
            | Token![&]
            | Token![|]
            | Token![<<]
            | Token![>>]
            | TokenKind::Slash2 => self.parse_binary(left),
            Token![.] | Token![?.] => self.parse_get(left),
            TokenKind::LeftBracket => self.parse_index(left),
            TokenKind::LeftParen => self.parse_call(left),
            _ => {
                self.add_error(
                    &format!("unexpected {}", TokenKind::from(token.value)),
                    token.span,
                );
                None
            }
        }
    }

    fn parse_fn(&mut self, fn_attribs: Vec<Attrib>) -> Option<Item> {
        let (fn_type, fn_type_span) = {
            if self.check(Token![fn]) {
                let fn_token = self.expect(Token![fn])?;
                (FnType::Sync, fn_token.span)
            } else {
                let co_token = self.expect(Token![co])?;
                (FnType::Coroutine, co_token.span)
            }
        };
        let mut variadic_param = None;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LeftParen)?;
        let mut params = vec![];
        while self.peek() != TokenKind::RightParen {
            let variadic = self.matches(Token![...]);
            if let Some(variadic) = variadic {
                let ty = self.parse_type()?;
                variadic_param = Some(Variadic {
                    ident: None,
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }
            let attribs = self.parse_attribs()?;
            let pat = self.parse_pat()?;

            self.expect(Token![:])?;
            let variadic = self.matches(Token![...]);
            let ty = self.parse_type()?;

            if let Some(variadic) = variadic {
                let Pat::Ident(ident) = pat else {
                    self.add_error(&format!("expected ident, got {}", pat), pat.span());
                    return None;
                };
                variadic_param = Some(Variadic {
                    ident: Some(ident.value),
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }

            let default_value = if self.matches(Token![=]).is_some() {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };

            self.matches(Token![,]);

            params.push(FnParam::Typed(
                FnParamTyped {
                    span: default_value
                        .as_ref()
                        .map(|d| pat.span().union(d.span()))
                        .unwrap_or_else(|| pat.span().union(ty.span())),
                    pat_type: PatType {
                        span: pat.span().union(ty.span()),
                        pat: pat.into(),
                        ty: ty.into(),
                        id: self.id(),
                    },
                    default_value,
                    id: self.id(),
                    attribs,
                }
                .into(),
            ));
        }
        self.expect(TokenKind::RightParen)?;

        let output = self.parse_output()?;
        let body = self.parse_block()?;
        let span = fn_type_span.union(output.span().unwrap_or_else(|| body.span()));

        Some(Item::Fn(ItemFn {
            span,
            name,
            params,
            body: match body {
                Expr::Block(block) => block,
                _ => unreachable!(),
            },
            output,
            id: self.id(),
            variadic: variadic_param,
            fn_type,
            attribs: fn_attribs,
        }))
    }

    fn parse_output(&mut self) -> Option<ReturnType> {
        let output = if self.matches(Token![->]).is_some() {
            let mut returns = vec![];
            loop {
                let ty = self.parse_type()?;
                returns.push(ty);
                if self.matches(Token![,]).is_none() {
                    break;
                }
            }
            if let [
                TypeExpr::Primitive(PrimitiveType {
                    value: Primitive::Nil,
                    ..
                }),
            ] = returns.as_slice()
            {
                return Some(ReturnType::None);
            }
            ReturnType::Type(returns)
        } else {
            ReturnType::None
        };
        Some(output)
    }

    fn parse_optional_semi(&mut self) -> Option<Span> {
        match self.peek() {
            TokenKind::Semicolon => {
                let mark = self.advance();
                Some(mark.span)
            }
            _ => None,
        }
    }

    fn parse_stmt_expr(&mut self) -> Option<Stmt> {
        let mut left = vec![self.parse_expr(Precedence::Lowest)?];
        let mut span = left[0].span();
        while self.matches(Token![,]).is_some() {
            let expr = self.parse_expr(Precedence::Lowest)?;
            span = span.union(expr.span());
            left.push(expr);
        }

        Some(match self.peek() {
            Token![=] => {
                self.expect(Token![=])?;
                let mut right = vec![self.parse_expr(Precedence::Lowest)?];
                span = span.union(right[0].span());
                while self.matches(Token![,]).is_some() {
                    let value = self.parse_expr(Precedence::Lowest)?;
                    span = span.union(value.span());
                    right.push(value);
                }

                let semi = self.expect(Token![;])?;
                Stmt::Assign(AssignStmt {
                    left,
                    right,
                    span: span.union(semi.span),
                    id: self.id(),
                })
            }
            Token![+=]
            | Token![-=]
            | Token![*=]
            | Token![/=]
            | TokenKind::Slash2Eq
            | Token![%=]
            | Token![|=]
            | Token![&=]
            | Token![^=]
            | Token![>>=]
            | Token![<<=] => {
                let assing_op = self.parse_binary_assign_op()?;
                let mut right = vec![self.parse_expr(Precedence::Lowest)?];
                span = span.union(right[0].span());
                while self.matches(Token![,]).is_some() {
                    let value = self.parse_expr(Precedence::Lowest)?;
                    span = span.union(value.span());
                    right.push(value);
                }
                let semi = self.expect(Token![;])?;
                Stmt::BinaryAssign(BinaryAssignStmt {
                    left,
                    right,
                    span: span.union(semi.span),
                    id: self.id(),
                    op: assing_op.value,
                })
            }
            _ => {
                let semi = self.parse_optional_semi();
                let span = semi.map(|s| span.union(s)).unwrap_or(span);
                Stmt::Expr(ExprStmt {
                    exprs: left,
                    semi,
                    span,
                    id: self.id(),
                })
            }
        })
    }

    fn parse_expr(&mut self, precedence: Precedence) -> Option<Expr> {
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

    fn parse_binding(&mut self) -> Option<Stmt> {
        let let_token = self.expect(Token![let])?;

        let mut pats = vec![];
        let mut types = vec![];
        let mut span = let_token.span;

        loop {
            let pat = self.parse_pat()?;
            span = span.union(pat.span());
            pats.push(pat);

            if self.matches(Token![:]).is_some() {
                let ty = self.parse_type()?;
                types.push(Some(ty));
            } else {
                types.push(None);
            }

            if self.matches(Token![,]).is_none() {
                break;
            }
        }

        Some(if self.matches(Token![=]).is_some() {
            let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
            let mut span = span.union(exprs[0].span());
            while let TokenKind::Comma = self.peek() {
                self.expect(TokenKind::Comma)?;
                let expr = self.parse_expr(Precedence::Lowest)?;
                span = span.union(expr.span());
                exprs.push(expr);
            }

            self.expect(Token![;])?;
            Stmt::Binding(BindingStmt {
                pats,
                exprs: Some(exprs),
                span,
                id: self.id(),
                types,
            })
        } else {
            let span = self
                .parse_optional_semi()
                .map(|s| span.union(s))
                .unwrap_or(span);

            Stmt::Binding(BindingStmt {
                pats,
                exprs: None,
                span,
                id: self.id(),
                types,
            })
        })
    }

    fn parse_return(&mut self) -> Option<Stmt> {
        let span = self.expect(Token![return])?.span;
        if self.matches(Token![;]).is_some() {
            return Some(Stmt::Return(ReturnStmt {
                exprs: None,
                span,
                id: self.id(),
            }));
        }
        let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
        let mut span = span.union(exprs[0].span());
        while self.matches(Token![,]).is_some() {
            let expr = self.parse_expr(Precedence::Lowest)?;
            span = span.union(expr.span());
            exprs.push(expr);
        }

        Some(Stmt::Return(ReturnStmt {
            exprs: Some(exprs),
            span,
            id: self.id(),
        }))
    }

    fn parse_yield(&mut self) -> Option<Stmt> {
        let span = self.expect(Token![yield])?.span;
        if self.matches(Token![;]).is_some() {
            return Some(Stmt::Yield(YieldStmt {
                exprs: None,
                span,
                id: self.id(),
            }));
        }
        let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
        let mut span = span.union(exprs[0].span());
        while self.matches(Token![,]).is_some() {
            let expr = self.parse_expr(Precedence::Lowest)?;
            span = span.union(expr.span());
            exprs.push(expr);
        }

        Some(Stmt::Yield(YieldStmt {
            exprs: Some(exprs),
            span,
            id: self.id(),
        }))
    }

    fn parse_continue(&mut self) -> Option<Stmt> {
        let continue_keyword = self.expect(Token![continue])?;
        Some(Stmt::Continue(ContinueStmt {
            span: continue_keyword.span,
            id: self.id(),
        }))
    }

    fn parse_break(&mut self) -> Option<Stmt> {
        let break_keyword = self.expect(Token![break])?;
        Some(Stmt::Continue(ContinueStmt {
            span: break_keyword.span,
            id: self.id(),
        }))
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        Some(match self.peek() {
            Token![let] => self.parse_binding()?,
            Token![;] => {
                let semi = self.expect(Token![;])?;
                Stmt::Empty(EmptyStmt {
                    span: semi.span,
                    id: self.id(),
                })
            }
            Token![return] => self.parse_return()?,
            Token![yield] => self.parse_yield()?,
            Token![continue] => self.parse_continue()?,
            Token![break] => self.parse_break()?,
            _ => self.parse_stmt_expr()?,
        })
    }

    fn parse_enum(&mut self, enum_attribs: Vec<Attrib>) -> Option<Item> {
        let enum_token = self.expect(Token![enum])?;
        let name = self.parse_ident()?;
        let mut variants = vec![];

        self.expect(TokenKind::LeftBrace)?;
        while !self.check(TokenKind::RightBrace) {
            let attribs = self.parse_attribs()?;
            let name = self.parse_ident()?;

            let variant = match self.peek() {
                TokenKind::LeftBrace => {
                    let left = self.expect(TokenKind::LeftBrace)?;
                    let fields = self.parse_fields_named(TokenKind::RightBrace)?;
                    let right = self.expect(TokenKind::RightBrace)?;

                    EnumVariant {
                        span: name.span().union(right.span),
                        name,
                        fields: Fields::Named(FieldsNamed {
                            fields,
                            span: left.span.union(right.span),
                            id: self.id(),
                        }),
                        discriminant: None,
                        id: self.id(),
                        attribs,
                    }
                }
                TokenKind::LeftParen => {
                    let left = self.expect(TokenKind::LeftParen)?;
                    let fields = self.parse_fields_unnamed()?;
                    let right = self.expect(TokenKind::RightParen)?;

                    EnumVariant {
                        span: name.span().union(right.span),
                        name,
                        fields: Fields::Unnamed(FieldsUnnamed {
                            fields,
                            span: left.span.union(right.span),
                            id: self.id(),
                        }),
                        discriminant: None,
                        id: self.id(),
                        attribs,
                    }
                }
                Token![=] => {
                    self.expect(Token![=])?;
                    let expr = self.parse_expr(Precedence::Lowest)?;
                    EnumVariant {
                        span: name.span.union(expr.span()),
                        name,
                        fields: Fields::Unit,
                        id: self.id(),
                        discriminant: Some(expr),
                        attribs,
                    }
                }
                Token![,] => {
                    self.expect(Token![,])?;
                    variants.push(EnumVariant {
                        span: name.span,
                        name,
                        fields: Fields::Unit,
                        discriminant: None,
                        id: self.id(),
                        attribs,
                    });
                    continue;
                }
                _ => {
                    let token = self.advance();
                    self.add_error(
                        &format!("expected '{{', '(' or  '=', got {}", token.value.kind(),),
                        token.span,
                    );
                    return None;
                }
            };
            variants.push(variant);

            self.matches(Token![,]);
        }
        let right_brace = self.expect(TokenKind::RightBrace)?;

        Some(Item::Enum(ItemEnum {
            name,
            variants,
            span: enum_token.span.union(right_brace.span),
            id: self.id(),
            attribs: enum_attribs,
        }))
    }

    fn parse_struct(&mut self, struct_attribs: Vec<Attrib>) -> Option<Item> {
        let struct_token = self.expect(Token![struct])?;
        let name = self.parse_ident()?;

        Some(match self.peek() {
            TokenKind::Semicolon => {
                let semi = self.expect(Token![;])?;
                Item::Struct(ItemStruct {
                    kind: StructKind::GC,
                    fields: Fields::Unit,
                    name,
                    span: struct_token.span.union(semi.span),
                    id: self.id(),
                    attribs: struct_attribs,
                })
            }
            TokenKind::LeftBrace => {
                let left = self.expect(TokenKind::LeftBrace)?;
                let fields = self.parse_fields_named(TokenKind::RightBrace)?;
                let right = self.expect(TokenKind::RightBrace)?;

                Item::Struct(ItemStruct {
                    name,
                    kind: StructKind::GC,
                    fields: Fields::Named(FieldsNamed {
                        span: left.span.union(right.span),
                        fields,
                        id: self.id(),
                    }),
                    span: struct_token.span.union(right.span),
                    id: self.id(),
                    attribs: struct_attribs,
                })
            }
            TokenKind::LeftParen => {
                let left = self.expect(TokenKind::LeftParen)?;
                let fields = self.parse_fields_unnamed()?;
                let right = self.expect(TokenKind::RightParen)?;
                self.expect(TokenKind::Semicolon)?;
                Item::Struct(ItemStruct {
                    name,
                    kind: StructKind::GC,
                    fields: Fields::Unnamed(FieldsUnnamed {
                        fields,
                        span: left.span.union(right.span),
                        id: self.id(),
                    }),
                    span: struct_token.span.union(right.span),
                    id: self.id(),
                    attribs: struct_attribs,
                })
            }
            _ => {
                let token = self.advance();
                self.add_error(
                    &format!("expected '{{', or ';', got {}", token.value.kind()),
                    token.span,
                );
                return None;
            }
        })
    }

    fn parse_fields_named(&mut self, end_token: TokenKind) -> Option<Vec<Field>> {
        let mut fields = vec![];
        while self.peek() != end_token {
            let attribs = self.parse_attribs()?;
            let name = self.parse_ident()?;
            self.expect(Token![:])?;
            let ty = self.parse_type()?;
            let default_value = self
                .matches(Token![=])
                .and_then(|_| self.parse_expr(Precedence::Lowest));
            self.matches(Token![,]);

            let span = default_value
                .as_ref()
                .map(|d| name.span.union(d.span()))
                .unwrap_or_else(|| name.span.union(ty.span()));
            fields.push(Field {
                span,
                default_value,
                name: Some(name),
                id: self.id(),
                ty,
                attribs,
            })
        }
        Some(fields)
    }

    fn parse_fields_unnamed(&mut self) -> Option<Vec<Field>> {
        let mut fields = vec![];
        while self.peek() != TokenKind::RightParen {
            let attribs = self.parse_attribs()?;
            let ty = self.parse_type()?;
            let default_value = if self.peek() == TokenKind::Eq {
                self.expect(TokenKind::Eq)?;
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };

            self.matches(Token![,]);

            let span = default_value
                .as_ref()
                .map(|d| d.span())
                .unwrap_or_else(|| ty.span());
            fields.push(Field {
                ty,
                default_value,
                name: None,
                id: self.id(),
                span,
                attribs,
            });
        }
        Some(fields)
    }

    fn parse_extern_fn(&mut self) -> Option<ExternDefinition> {
        let fn_attribs = self.parse_attribs()?;
        let fn_token = self.expect(Token![fn])?;
        let mut variadic_param = None;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LeftParen)?;
        let mut params = vec![];
        while self.peek() != TokenKind::RightParen {
            if let Some(variadic) = self.matches(Token![...]) {
                let ty = self.parse_type()?;
                variadic_param = Some(Variadic {
                    ident: None,
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }
            let attribs = self.parse_attribs()?;
            let ident = self.parse_ident()?;
            let ident_span = ident.span;

            self.expect(Token![:])?;

            let variadic = self.matches(Token![...]);
            let ty = self.parse_type()?;
            if let Some(variadic) = &variadic {
                variadic_param = Some(Variadic {
                    ident: Some(ident),
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }
            self.matches(Token![,]);

            params.push(FnParam::Typed(
                FnParamTyped {
                    span: ident_span.union(ty.span()),
                    pat_type: PatType {
                        pat: Pat::Ident(PatIdent {
                            id: self.id(),
                            span: ident_span,
                            value: ident,
                        })
                        .into(),
                        span: ident_span,
                        ty: ty.into(),
                        id: self.id(),
                    },
                    default_value: None,
                    id: self.id(),
                    attribs,
                }
                .into(),
            ));
        }
        self.expect(TokenKind::RightParen)?;

        let output = self.parse_output()?;
        let semi = self.expect(TokenKind::Semicolon)?;
        let span = fn_token.span.union(semi.span);

        Some(ExternDefinition::Fn(ExternFn {
            name,
            params,
            output,
            id: self.id(),
            span,
            variadic: variadic_param,
        }))
    }

    fn parse_inline(&mut self) -> Option<Item> {
        let inline_token = self.expect(TokenKind::Inline)?;
        self.expect(TokenKind::LeftParen)?;
        let ident = self.parse_ident()?;
        if ident.value != "lua" {
            self.add_error(
                &format!("expected valid inline variant (lua), got {}", ident.value),
                ident.span,
            );
            return None;
        }
        self.expect(TokenKind::RightParen)?;

        let mut defs = vec![];
        let attribs = self.parse_attribs()?;
        let span = if self.check(TokenKind::Fn) {
            let def = self.parse_inline_fn()?;
            let span = inline_token.span.union(def.span());
            defs.push(def);
            span
        } else {
            self.expect(TokenKind::LeftBrace)?;
            while !self.check(TokenKind::RightBrace) {
                defs.push(self.parse_inline_fn()?);
            }
            inline_token
                .span
                .union(self.expect(TokenKind::RightBrace)?.span)
        };

        Some(Item::Inline(ItemInline {
            defs,
            id: self.id(),
            span,
            attribs,
        }))
    }

    fn parse_inline_fn(&mut self) -> Option<InlineFn> {
        let fn_token = self.expect(Token![fn])?;
        let mut variadic_param = None;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LeftParen)?;
        let mut params = vec![];
        while self.peek() != TokenKind::RightParen {
            if let Some(variadic) = self.matches(Token![...]) {
                let ty = self.parse_type()?;
                variadic_param = Some(Variadic {
                    ident: None,
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }
            let attribs = self.parse_attribs()?;
            let ident = self.parse_ident()?;
            let ident_span = ident.span;

            self.expect(Token![:])?;

            let variadic = self.matches(Token![...]);
            let ty = self.parse_type()?;
            self.matches(Token![,]);
            if let Some(variadic) = &variadic {
                variadic_param = Some(Variadic {
                    ident: Some(ident),
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }

            params.push(FnParam::Typed(
                FnParamTyped {
                    span: ident_span.union(ty.span()),
                    pat_type: PatType {
                        pat: Pat::Ident(PatIdent {
                            id: self.id(),
                            span: ident_span,
                            value: ident,
                        })
                        .into(),
                        span: ident_span,
                        ty: ty.into(),
                        id: self.id(),
                    },
                    default_value: None,
                    id: self.id(),
                    attribs,
                }
                .into(),
            ));
        }
        self.expect(TokenKind::RightParen)?;

        let output = self.parse_output()?;
        self.expect(Token![=])?;
        let body = self.parse_string()?;
        self.expect(Token![;]);
        let span = fn_token.span.union(body.span());
        let Expr::Lit(LitExpr::String(body)) = body else {
            unreachable!();
        };

        Some(InlineFn {
            name,
            params,
            body,
            id: self.id(),
            span,
            output,
            variadic: variadic_param,
        })
    }

    fn parse_extern_def(&mut self) -> Option<ExternDefinition> {
        match self.peek() {
            TokenKind::Fn => self.parse_extern_fn(),
            other => {
                let token = self.advance();
                self.add_error(&format!("expected definition, got {}", other), token.span);
                None
            }
        }
    }

    fn parse_extern(&mut self) -> Option<Item> {
        let extern_token = self.expect(Token![extern])?;
        self.expect(TokenKind::LeftParen)?;
        let ident = self.parse_ident()?;
        let extern_kind = match ident.value.as_str() {
            "C" => ExternKind::C,
            "lua" => ExternKind::Lua,
            other => {
                self.add_error(
                    &format!("expected valid extern variant (lua, C), got {}", other),
                    ident.span,
                );
                return None;
            }
        };
        self.expect(TokenKind::RightParen)?;

        let mut defs = vec![];
        let span = if self.check(TokenKind::Fn) {
            let def = self.parse_extern_def()?;
            let span = extern_token.span.union(def.span());
            defs.push(def);
            span
        } else {
            self.expect(TokenKind::LeftBrace)?;
            while !self.check(TokenKind::RightBrace) {
                defs.push(self.parse_extern_def()?);
            }
            extern_token
                .span
                .union(self.expect(TokenKind::RightBrace)?.span)
        };

        Some(Item::Extern(ItemExtern {
            kind: extern_kind,
            defs,
            id: self.id(),
            span,
        }))
    }

    fn parse_impl_fn(&mut self) -> Option<ImplItem> {
        let fn_attribs = self.parse_attribs()?;
        let (fn_type, fn_type_span) = {
            if self.check(Token![fn]) {
                let fn_token = self.expect(Token![fn])?;
                (FnType::Sync, fn_token.span)
            } else {
                let co_token = self.expect(Token![co])?;
                (FnType::Coroutine, co_token.span)
            }
        };
        let mut variadic_param = None;
        let name = self.parse_ident()?;
        self.expect(TokenKind::LeftParen)?;
        let mut params = vec![];
        while !self.check(TokenKind::RightParen) {
            if let Some(variadic) = self.matches(Token![...]) {
                let ty = self.parse_type()?;
                variadic_param = Some(Variadic {
                    ident: None,
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }
            if let Some(self_token) = self.matches(Token![self]) {
                self.matches(Token![,]);
                params.push(FnParam::Receiver(Receiver {
                    span: self_token.span,
                    id: self.id(),
                }));
                continue;
            }
            let attribs = self.parse_attribs()?;
            let pat = self.parse_pat()?;
            self.expect(Token![:])?;

            let variadic = self.matches(Token![...]);
            let ty = self.parse_type()?;
            if let Some(variadic) = variadic {
                let Pat::Ident(ident) = pat else {
                    self.add_error(&format!("expected ident, got {}", pat), pat.span());
                    return None;
                };
                variadic_param = Some(Variadic {
                    ident: Some(ident.value),
                    span: variadic.span.union(ty.span()),
                    ty: ty.into(),
                    id: self.id(),
                });
                break;
            }

            let default_value = if self.matches(Token![=]).is_some() {
                Some(self.parse_expr(Precedence::Lowest)?)
            } else {
                None
            };

            self.matches(Token![,]);

            params.push(FnParam::Typed(
                FnParamTyped {
                    span: default_value
                        .as_ref()
                        .map(|d| pat.span().union(d.span()))
                        .unwrap_or_else(|| pat.span().union(ty.span())),
                    pat_type: PatType {
                        span: pat.span().union(ty.span()),
                        pat: pat.into(),
                        ty: ty.into(),
                        id: self.id(),
                    },
                    default_value,
                    id: self.id(),
                    attribs,
                }
                .into(),
            ));
        }
        self.expect(TokenKind::RightParen)?;

        let output = self.parse_output()?;
        let body = self.parse_block()?;
        let span = fn_type_span.union(output.span().unwrap_or_else(|| body.span()));
        Some(ImplItem::Fn(ItemFn {
            fn_type,
            name,
            params,
            body: match body {
                Expr::Block(block) => block,
                _ => unreachable!(),
            },
            output,
            variadic: variadic_param,
            id: self.id(),
            span,
            attribs: fn_attribs,
        }))
    }

    fn parse_impl_item(&mut self) -> Option<ImplItem> {
        self.parse_impl_fn()
    }

    fn parse_impl(&mut self) -> Option<Item> {
        let impl_token = self.expect(Token![impl])?;
        let target = self.parse_type()?;
        if target.nilable() {
            self.add_error(
                &format!("cannot impl a nilable type {}", target),
                target.span(),
            );
            return None;
        }
        self.expect(TokenKind::LeftBrace)?;
        let mut items = vec![];
        while !self.check(TokenKind::RightBrace) {
            items.push(self.parse_impl_item()?);
        }
        let right_brace = self.expect(TokenKind::RightBrace)?;
        Some(Item::Impl(ItemImpl {
            target,
            items,
            span: impl_token.span.union(right_brace.span),
            id: self.id(),
        }))
    }

    fn parse_item(&mut self) -> Option<Item> {
        let attribs = self.parse_attribs()?;
        match self.peek() {
            Token![fn] => self.parse_fn(attribs),
            Token![struct] => self.parse_struct(attribs),
            Token![enum] => self.parse_enum(attribs),
            Token![impl] => self.parse_impl(),
            Token![extern] => self.parse_extern(),
            Token![inline] => self.parse_inline(),
            other => {
                let token = self.advance();
                self.add_error(&format!("expected item, got {}", other), token.span);
                None
            }
        }
    }

    fn parse_program(&mut self) -> Option<Vec<Item>> {
        let mut items = vec![];

        while !self.is_at_end() {
            if let Some(item) = self.parse_item() {
                items.push(item);
            } else {
                self.sync();
            }
        }

        if self.diagnostics.is_empty() {
            Some(items)
        } else {
            None
        }
    }
}

pub fn parse_program(tokens: &[WithSpan<Token>]) -> Result<Vec<Item>, Vec<position::Diagnostic>> {
    let mut parser = Parser::new(tokens);
    match parser.parse_program() {
        Some(output) => Ok(output),
        None => Err(parser.diagnostics),
    }
}

// mod tests {
//     use crate::position::Diagnostic;
//
//     use super::*;
//
//     fn parse_str(data: &str) -> Result<WithSpan<Expr>, Vec<Diagnostic>> {
//         use super::super::tokenizer::*;
//
//         let tokens = tokenize(data);
//         let mut parser = crate::parser::Parser::new(&tokens);
//         match parser.parse_expr(Precedence::Lowest) {
//             Some(e) => Ok(e),
//             None => Err(parser.diagnostics().to_vec()),
//         }
//     }
//
//     fn assert_errs(data: &str, errs: &[&str]) {
//         let x = parse_str(data);
//         assert!(x.is_err());
//         let diagnostics = x.unwrap_err();
//         for diag in diagnostics {
//             assert!(errs.contains(&diag.message.as_str()), "{}", diag.message);
//         }
//     }
// }
