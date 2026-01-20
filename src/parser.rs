// use crate::{
//     Token,
//     ast::*,
//     common::*,
//     position::{self, WithSpan},
//     token::{self, Token, TokenKind},
// };
//
// static EOF_TOKEN: WithSpan<Token> = position::WithSpan::empty(Token::EOF);
//
// pub struct Parser<'t> {
//     tokens: &'t [WithSpan<Token>],
//     cursor: usize,
//     diagnostics: Vec<position::Diagnostic>,
//     last_id: AstNodeId,
// }
//
// impl<'t> Parser<'t> {
//     fn new(tokens: &'t [WithSpan<Token>]) -> Self {
//         Self {
//             tokens,
//             cursor: 0,
//             diagnostics: Default::default(),
//             last_id: AstNodeId(0),
//         }
//     }
//
//     fn new_id(&mut self) -> AstNodeId {
//         let id = self.last_id;
//         self.last_id += 1.into();
//         id
//     }
//
//     fn diagnostics(&self) -> &[position::Diagnostic] {
//         &self.diagnostics
//     }
//
//     fn add_error(&mut self, message: &str, span: position::Span) {
//         self.diagnostics.push(position::Diagnostic {
//             span,
//             message: message.to_owned(),
//         });
//     }
//
//     fn peek_token(&self) -> &'t WithSpan<Token> {
//         match self.tokens.get(self.cursor) {
//             Some(token) => token,
//             None => &EOF_TOKEN,
//         }
//     }
//
//     fn peek(&self) -> TokenKind {
//         (&self.peek_token().value).into()
//     }
//
//     fn peek_next(&self) -> TokenKind {
//         match self.tokens.get(self.cursor + 1) {
//             Some(token) => token.value.kind(),
//             None => EOF_TOKEN.value.kind(),
//         }
//     }
//
//     fn check(&self, match_token: TokenKind) -> bool {
//         self.peek() == match_token
//     }
//
//     fn advance(&mut self) -> &'t WithSpan<Token> {
//         match self.tokens.get(self.cursor) {
//             Some(token) => {
//                 self.cursor += 1;
//                 token
//             }
//             None => &EOF_TOKEN,
//         }
//     }
//
//     fn match_token(&mut self, kind: TokenKind) -> Option<&'t WithSpan<Token>> {
//         let check = self.check(kind);
//         if check { Some(self.advance()) } else { None }
//     }
//
//     fn expect(&mut self, expected: TokenKind) -> Option<&'t WithSpan<Token>> {
//         let token = self.advance();
//         if TokenKind::from(&token.value) == expected {
//             Some(token)
//         } else {
//             self.add_error(
//                 &format!(
//                     "Expected {}, got {}",
//                     expected,
//                     TokenKind::from(&token.value)
//                 ),
//                 token.span,
//             );
//             None
//         }
//     }
//
//     fn is_at_end(&self) -> bool {
//         self.peek() == TokenKind::EOF
//     }
// }
//
// #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
// pub enum Precedence {
//     Lowest,
//     NilCoalescing,
//     Or,
//     And,
//     Comparison, // <, <=, >, >=, ==, !=
//     BitwiseOR,
//     BitwiseXOR,
//     BitwiseAND,
//     BitwiseShift,
//     Term,   // + -
//     Factor, // * / %
//     Unary,  // ! -
//     Call,
//     Path,
// }
//
// impl From<TokenKind> for Precedence {
//     fn from(value: TokenKind) -> Self {
//         match value {
//             TokenKind::Bar2 => Self::Or,
//             TokenKind::Ampersand2 => Self::And,
//             TokenKind::Less
//             | TokenKind::LessEqual
//             | TokenKind::Greater
//             | TokenKind::GreaterEqual
//             | TokenKind::BangEqual
//             | TokenKind::Equal2 => Self::Comparison,
//             TokenKind::Plus | TokenKind::Minus => Self::Term,
//             TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Self::Factor,
//             TokenKind::Bang => Self::Unary,
//             TokenKind::LeftParen => Self::Call,
//             _ => Self::Lowest,
//         }
//     }
// }
//
// impl Parser<'_> {
//     fn sync(&mut self) {
//         let mut token = self.advance();
//
//         while !self.is_at_end() {
//             if TokenKind::from(&token.value) == TokenKind::Semicolon {
//                 return;
//             }
//
//             if matches!(
//                 self.peek(),
//                 TokenKind::Struct
//                     | TokenKind::Fn
//                     | TokenKind::Let
//                     | TokenKind::For
//                     | TokenKind::If
//                     | TokenKind::While
//                     | TokenKind::Loop
//                     | TokenKind::Return
//             ) {
//                 return;
//             }
//
//             token = self.advance();
//         }
//     }
//
//     fn parse_ident(&mut self) -> Option<WithSpan<IdentExpr>> {
//         let WithSpan {
//             value: Token::Identifier(ident),
//             span,
//         } = self.expect(TokenKind::Identifier)?
//         else {
//             unreachable!();
//         };
//         Some(WithSpan::new(ident.clone(), *span))
//     }
//
//     fn parse_expr(&mut self, precedence: Precedence) -> Option<WithSpan<Expr>> {
//         let mut expr = self.parse_prefix()?;
//         while !self.is_at_end() {
//             let next_precedence = Precedence::from(self.peek());
//             if precedence >= next_precedence {
//                 break;
//             }
//
//             expr = self.parse_infix(expr)?;
//         }
//
//         Some(expr)
//     }
//
//     fn parse_infix(&mut self, left: WithSpan<Expr>) -> Option<WithSpan<Expr>> {
//         let token = self.peek_token();
//         match token.value {
//             Token::BangEqual
//             | Token::Equal2
//             | Token::Bar2
//             | Token::Ampersand2
//             | Token::Less
//             | Token::LessEqual
//             | Token::Greater
//             | Token::GreaterEqual
//             | Token::Plus
//             | Token::Minus
//             | Token::Star
//             | Token::Slash
//             | Token::Percent => self.parse_binary(left),
//             Token::LeftParen => self.parse_call(left),
//             _ => {
//                 self.add_error(
//                     &format!("Unexpected {}", TokenKind::from(&token.value)),
//                     token.span,
//                 );
//                 None
//             }
//         }
//     }
//
//     fn parse_index(&mut self) -> Option<WithSpan<Expr>> {
//         None
//     }
//
//     fn parse_call(&mut self, left: WithSpan<Expr>) -> Option<WithSpan<Expr>> {
//         let mut span = self.expect(TokenKind::LeftParen)?.span;
//         let mut args = vec![];
//         loop {
//             if self.check(TokenKind::RightParen) {
//                 break;
//             }
//             let arg = self.parse_expr(Precedence::Lowest)?;
//             if self.check(TokenKind::Colon) {
//                 self.expect(TokenKind::Colon)?;
//                 let WithSpan {
//                     value: Expr::Path(ident, _),
//                     ..
//                 } = arg
//                 else {
//                     self.add_error("expected identifier", arg.span);
//                     return None;
//                 };
//
//                 let arg = self.parse_expr(Precedence::Lowest)?;
//                 args.push(FnArg {
//                     name: Some(ident.clone()),
//                     expr: arg.into(),
//                 });
//             } else {
//                 args.push(FnArg {
//                     expr: arg.into(),
//                     name: None,
//                 });
//             }
//
//             if self.check(TokenKind::Comma) {
//                 self.expect(TokenKind::Comma)?;
//             }
//         }
//         span = span.union(self.expect(TokenKind::RightParen)?.span);
//         Some(WithSpan::new(
//             Expr::Call(CallExpr {
//                 func: left.into(),
//                 args,
//                 callee_type: None,
//             }),
//             span,
//         ))
//     }
//
//     fn parse_prefix(&mut self) -> Option<WithSpan<Expr>> {
//         match self.peek() {
//             TokenKind::Number
//             | TokenKind::Nil
//             | TokenKind::True
//             | TokenKind::False
//             | TokenKind::Identifier
//             | TokenKind::String => self.parse_primary(),
//             TokenKind::Bang | TokenKind::Minus => self.parse_unary(),
//             TokenKind::LeftParen => self.parse_grouping(),
//             TokenKind::LeftBrace => self.parse_block(),
//             TokenKind::If => self.parse_if(),
//             TokenKind::Bar => self.parse_closure(),
//             _ => {
//                 self.add_error(
//                     &format!("Unexpected {}", self.peek()),
//                     self.peek_token().span,
//                 );
//                 None
//             }
//         }
//     }
//     fn parse_closure(&mut self) -> Option<WithSpan<Expr>> {
//         let bar = self.expect(TokenKind::Bar)?;
//         let mut params = vec![];
//         while self.peek() != TokenKind::Bar {
//             let ident = self.parse_ident()?;
//             self.expect(TokenKind::Colon)?;
//             let ty = self.parse_type()?;
//             if self.peek() == TokenKind::Comma {
//                 self.expect(TokenKind::Comma)?;
//             }
//             let default_value = if self.peek() == TokenKind::Equal {
//                 self.expect(TokenKind::Equal)?;
//                 Some(self.parse_expr(Precedence::Lowest)?)
//             } else {
//                 None
//             };
//             if self.peek() == TokenKind::Comma {
//                 self.expect(TokenKind::Comma)?;
//             }
//             params.push(FnParam {
//                 kind: FnParamKind::Regular,
//                 ty,
//                 name: WithSpan::new(ident.value, ident.span),
//                 default_value,
//             });
//         }
//         self.expect(TokenKind::Bar)?;
//         let returns = if self.peek() == TokenKind::Arrow {
//             self.expect(TokenKind::Arrow)?;
//             let mut returns = vec![];
//             loop {
//                 let ty = self.parse_type()?;
//                 returns.push(ty);
//                 if self.peek() == TokenKind::Comma {
//                     self.expect(TokenKind::Comma)?;
//                 } else {
//                     break;
//                 }
//             }
//             Some(returns)
//         } else {
//             None
//         };
//         let body = if self.check(TokenKind::LeftBrace) {
//             let WithSpan {
//                 value: Expr::Block(block),
//                 span,
//             } = self.parse_block()?
//             else {
//                 unreachable!()
//             };
//             WithSpan::new(block, span)
//         } else {
//             let stmt = self.parse_stmt_expr()?;
//             let stmt_span = stmt.span;
//             WithSpan::new(
//                 BlockExpr {
//                     body: vec![stmt],
//                     ty: None,
//                 },
//                 stmt_span,
//             )
//         };
//         let body_span = body.span;
//         Some(WithSpan::new(
//             Expr::Closure(ClosureExpr {
//                 params,
//                 body,
//                 returns,
//             }),
//             bar.span.union(body_span),
//         ))
//     }
//
//     fn parse_if(&mut self) -> Option<WithSpan<Expr>> {
//         let if_token = self.expect(TokenKind::If)?;
//         let condition = self.parse_expr(Precedence::Lowest)?;
//         let WithSpan {
//             value: Expr::Block(then_branch),
//             span: then_branch_span,
//         } = self.parse_block()?
//         else {
//             unreachable!()
//         };
//
//         let mut span = if_token.span.union(then_branch_span);
//         let else_branch = if self.match_token(TokenKind::Else).is_some() {
//             let expr = if self.peek() == TokenKind::If {
//                 self.parse_if()?
//             } else {
//                 self.parse_block()?
//             };
//             span = span.union(expr.span);
//             Some(expr.into())
//         } else {
//             None
//         };
//
//         Some(WithSpan::new(
//             Expr::If(IfExpr {
//                 condition: condition.into(),
//                 then_branch: WithSpan::new(then_branch, span),
//                 else_branch,
//                 ty: None,
//             }),
//             span,
//         ))
//     }
//
//     fn parse_binary_op(&mut self) -> Option<WithSpan<BinaryOp>> {
//         let token = self.advance();
//         let op = match &token.value {
//             Token::BangEqual => BinaryOp::NotEqual,
//             Token::Equal2 => BinaryOp::Equal,
//             Token::Less => BinaryOp::Less,
//             Token::LessEqual => BinaryOp::LessEqual,
//             Token::Greater => BinaryOp::Greater,
//             Token::GreaterEqual => BinaryOp::GreaterEqual,
//             Token::Plus => BinaryOp::Add,
//             Token::Minus => BinaryOp::Sub,
//             Token::Star => BinaryOp::Mult,
//             Token::Slash => BinaryOp::Div,
//             Token::Percent => BinaryOp::Modulo,
//             Token::Bar2 => BinaryOp::Or,
//             Token::Ampersand2 => BinaryOp::And,
//             _ => {
//                 self.add_error(
//                     &format!("Unexpected {}", TokenKind::from(&token.value)),
//                     token.span,
//                 );
//                 return None;
//             }
//         };
//
//         Some(WithSpan::new(op, token.span))
//     }
//
//     fn parse_grouping(&mut self) -> Option<WithSpan<Expr>> {
//         let left_paren = self.expect(TokenKind::LeftParen)?;
//         let expr = self.parse_expr(Precedence::Lowest)?;
//         let right_paren = self.expect(TokenKind::RightParen)?;
//
//         let span = left_paren.span.union(right_paren.span);
//         Some(WithSpan::new(Expr::Paren(expr.into()), span))
//     }
//
//     fn parse_unary(&mut self) -> Option<WithSpan<Expr>> {
//         let op = self.parse_unary_op()?;
//         let right = self.parse_expr(Precedence::Unary)?;
//         let span = op.span.union(right.span);
//         Some(WithSpan::new(
//             Expr::Unary(UnaryExpr {
//                 expr: right.into(),
//                 op,
//                 ty: None,
//             }),
//             span,
//         ))
//     }
//
//     fn parse_unary_op(&mut self) -> Option<WithSpan<UnaryOp>> {
//         let token = self.advance();
//         match &token.value {
//             Token::Bang => Some(WithSpan::new(UnaryOp::Not, token.span)),
//             Token::Minus => Some(WithSpan::new(UnaryOp::Negate, token.span)),
//             _ => {
//                 self.add_error(
//                     &format!("Unexpected {}", TokenKind::from(&token.value)),
//                     token.span,
//                 );
//                 None
//             }
//         }
//     }
//
//     fn parse_binary(&mut self, left: WithSpan<Expr>) -> Option<WithSpan<Expr>> {
//         let precedence = Precedence::from(self.peek());
//         let op = self.parse_binary_op()?;
//         let right = self.parse_expr(precedence)?;
//         let span = left.span.union(right.span);
//         Some(WithSpan::new(
//             Expr::Binary(BinaryExpr {
//                 left: left.into(),
//                 right: right.into(),
//                 op,
//                 types: None,
//             }),
//             span,
//         ))
//     }
//
//     fn parse_primary(&mut self) -> Option<WithSpan<Expr>> {
//         let token = self.advance();
//         match &token.value {
//             &Token::Nil => Some(WithSpan::new(Expr::Nil, token.span)),
//             &Token::Number(token::NumberToken::Int(i)) => {
//                 Some(WithSpan::new(Expr::Number(LitNum::Int(i)), token.span))
//             }
//             &Token::Number(token::NumberToken::Float(f)) => {
//                 Some(WithSpan::new(Expr::Number(LitNum::Float(f)), token.span))
//             }
//             &Token::True => Some(WithSpan::new(Expr::Bool(true), token.span)),
//             &Token::False => Some(WithSpan::new(Expr::Bool(false), token.span)),
//             Token::String(kind, s) => {
//                 Some(WithSpan::new(Expr::String(*kind, s.clone()), token.span))
//             }
//             Token::Identifier(s) => Some(WithSpan::new(Expr::Path(s.clone(), None), token.span)),
//             _ => {
//                 self.add_error(
//                     &format!("Unexpected {}", TokenKind::from(&token.value)),
//                     token.span,
//                 );
//                 None
//             }
//         }
//     }
//
//     fn parse_block(&mut self) -> Option<WithSpan<Expr>> {
//         let left_brace = self.expect(TokenKind::LeftBrace)?;
//         let mut stmts = vec![];
//         let right_brace = loop {
//             if self.is_at_end() {
//                 return None;
//             }
//             if let Some(t) = self.match_token(TokenKind::RightBrace) {
//                 break t;
//             }
//             stmts.push(self.parse_stmt()?);
//         };
//
//         Some(WithSpan::new(
//             Expr::Block(BlockExpr {
//                 body: stmts,
//                 ty: None,
//             }),
//             left_brace.span.union(right_brace.span),
//         ))
//     }
//
//     fn parse_item(&mut self) -> Option<WithSpan<Item>> {
//         match self.peek() {
//             TokenKind::Fn => self.parse_fn(),
//             TokenKind::Extern => self.parse_extern(),
//             TokenKind::Inline => self.parse_inline(),
//             TokenKind::Struct => self.parse_struct(),
//             TokenKind::Impl => self.parse_impl(),
//             other => {
//                 let token = self.advance();
//                 self.add_error(&format!("expected item, got {}", other), token.span);
//                 None
//             }
//         }
//     }
//
//     fn parse_impl(&mut self) -> Option<WithSpan<Item>> {
//         let impl_token = self.expect(TokenKind::Impl)?;
//         let target = self.parse_type()?;
//         if target.value.nilable() {
//             self.add_error(
//                 &format!("cannot impl a nilable type {}", target.value),
//                 target.span,
//             );
//             return None;
//         }
//         self.expect(TokenKind::LeftBrace)?;
//         let mut items = vec![];
//         while !self.check(TokenKind::RightBrace) {
//             items.push(self.parse_impl_item(target.as_ref())?);
//         }
//         let right_brace = self.expect(TokenKind::RightBrace)?;
//         Some(WithSpan::new(
//             Item::Impl(ItemImpl { target, items }),
//             impl_token.span.union(right_brace.span),
//         ))
//     }
//
//     fn parse_impl_item(&mut self, target: WithSpan<&Type>) -> Option<WithSpan<ImplItem>> {
//         let func = self.parse_impl_fn(target)?;
//         let span = func.span;
//         Some(WithSpan::new(ImplItem::Fn(func.value), span))
//     }
//
//     fn parse_struct(&mut self) -> Option<WithSpan<Item>> {
//         let struct_token = self.expect(TokenKind::Struct)?;
//         //TODO: parse paths. namespaces?
//         let name = self.parse_ident()?;
//
//         Some(match self.peek() {
//             TokenKind::Semicolon => {
//                 let semi = self.expect(TokenKind::Semicolon)?;
//                 WithSpan::new(
//                     Item::Struct(ItemStruct {
//                         kind: StructKind::GC,
//                         fields: StructFields::Unit,
//                         name,
//                     }),
//                     struct_token.span.union(semi.span),
//                 )
//             }
//             TokenKind::LeftBrace => {
//                 self.expect(TokenKind::LeftBrace)?;
//                 let mut fields = vec![];
//                 while self.peek() != TokenKind::RightBrace {
//                     let name = self.parse_ident()?;
//                     self.expect(TokenKind::Colon)?;
//                     let ty = self.parse_type()?;
//                     let default_value = if self.peek() == TokenKind::Equal {
//                         self.expect(TokenKind::Equal)?;
//                         Some(self.parse_expr(Precedence::Lowest)?)
//                     } else {
//                         None
//                     };
//
//                     if self.peek() == TokenKind::Comma {
//                         self.expect(TokenKind::Comma)?;
//                     }
//
//                     fields.push(Field {
//                         ty,
//                         default_value,
//                         name: Some(name),
//                     })
//                 }
//                 let right_brace = self.expect(TokenKind::RightBrace)?;
//
//                 WithSpan::new(
//                     Item::Struct(ItemStruct {
//                         name,
//                         kind: StructKind::GC,
//                         fields: StructFields::Named(fields),
//                     }),
//                     struct_token.span.union(right_brace.span),
//                 )
//             }
//             TokenKind::LeftParen => {
//                 self.expect(TokenKind::LeftParen)?;
//                 let mut fields = vec![];
//                 while self.peek() != TokenKind::RightParen {
//                     let ty = self.parse_type()?;
//                     let default_value = if self.peek() == TokenKind::Equal {
//                         self.expect(TokenKind::Equal)?;
//                         Some(self.parse_expr(Precedence::Lowest)?)
//                     } else {
//                         None
//                     };
//
//                     if self.peek() == TokenKind::Comma {
//                         self.expect(TokenKind::Comma)?;
//                     }
//
//                     fields.push(Field {
//                         ty,
//                         default_value,
//                         name: None,
//                     })
//                 }
//                 self.expect(TokenKind::RightParen)?;
//                 let semi = self.expect(TokenKind::Semicolon)?;
//                 WithSpan::new(
//                     Item::Struct(ItemStruct {
//                         name,
//                         kind: StructKind::GC,
//                         fields: StructFields::Named(fields),
//                     }),
//                     struct_token.span.union(semi.span),
//                 )
//             }
//             _ => {
//                 let token = self.advance();
//                 self.add_error(
//                     &format!("expected '(', '{{', or ';', got {}", token.value.kind()),
//                     token.span,
//                 );
//                 return None;
//             }
//         })
//     }
//
//     fn parse_inline(&mut self) -> Option<WithSpan<Item>> {
//         let inline_token = self.expect(TokenKind::Inline)?;
//         self.expect(TokenKind::LeftParen)?;
//         let ident = self.parse_ident()?;
//         if ident.value != "lua" {
//             self.add_error(
//                 &format!("expected valid inline variant (lua), got {}", ident.value),
//                 ident.span,
//             );
//             return None;
//         }
//         self.expect(TokenKind::RightParen)?;
//         self.expect(TokenKind::LeftBrace)?;
//
//         let mut defs = vec![];
//         while !self.check(TokenKind::RightBrace) {
//             defs.push(self.parse_inline_def()?);
//         }
//         let span = inline_token
//             .span
//             .union(self.expect(TokenKind::RightBrace)?.span);
//
//         Some(WithSpan::new(Item::Inline(ItemInline { defs }), span))
//     }
//
//     fn parse_inline_def(&mut self) -> Option<WithSpan<InlineFn>> {
//         let fn_token = self.expect(TokenKind::Fn)?;
//         let name = self.parse_ident()?;
//         self.expect(TokenKind::LeftParen)?;
//         let mut params = vec![];
//         while self.peek() != TokenKind::RightParen {
//             let ident = self.parse_ident()?;
//             self.expect(TokenKind::Colon)?;
//             let ty = self.parse_type()?;
//
//             if self.peek() == TokenKind::Comma {
//                 self.expect(TokenKind::Comma)?;
//             }
//
//             params.push(FnParam {
//                 kind: FnParamKind::Regular,
//                 ty,
//                 name: WithSpan::new(ident.value, ident.span),
//                 default_value: None,
//             });
//         }
//         self.expect(TokenKind::RightParen)?;
//
//         let returns = if self.peek() == TokenKind::Arrow {
//             self.expect(TokenKind::Arrow)?;
//             let mut returns = vec![];
//             loop {
//                 let ty = self.parse_type()?;
//                 returns.push(ty);
//                 if self.peek() == TokenKind::Comma {
//                     self.expect(TokenKind::Comma)?;
//                 } else {
//                     break;
//                 }
//             }
//             returns
//         } else {
//             vec![]
//         };
//
//         self.expect(TokenKind::Equal)?;
//         let body = self.advance();
//         let body_string = match &body.value {
//             Token::String(_, s) => s,
//             _ => {
//                 return None;
//             }
//         };
//         self.expect(TokenKind::Semicolon)?;
//         Some(WithSpan::new(
//             InlineFn {
//                 name: name.value,
//                 params,
//                 body: body_string.clone(),
//                 returns,
//                 ty: None,
//             },
//             fn_token.span.union(body.span),
//         ))
//     }
//
//     fn parse_extern(&mut self) -> Option<WithSpan<Item>> {
//         let extern_token = self.expect(TokenKind::Extern)?;
//         self.expect(TokenKind::LeftParen)?;
//         let ident = self.parse_ident()?;
//         let extern_kind = match ident.value.as_str() {
//             "C" => ExternKind::C,
//             "lua" => ExternKind::Lua,
//             other => {
//                 self.add_error(
//                     &format!("expected valid extern variant (lua, C), got {}", other),
//                     ident.span,
//                 );
//                 return None;
//             }
//         };
//         self.expect(TokenKind::RightParen)?;
//         self.expect(TokenKind::LeftBrace)?;
//
//         let mut defs = vec![];
//         while !self.check(TokenKind::RightBrace) {
//             defs.push(self.parse_extern_def()?);
//         }
//         let span = extern_token
//             .span
//             .union(self.expect(TokenKind::RightBrace)?.span);
//
//         Some(WithSpan::new(
//             Item::Extern(ItemExtern {
//                 kind: extern_kind,
//                 defs,
//             }),
//             span,
//         ))
//     }
//
//     fn parse_extern_def(&mut self) -> Option<WithSpan<ExternDefinition>> {
//         match self.peek() {
//             TokenKind::Fn => self.parse_fn_def(),
//             other => {
//                 let token = self.advance();
//                 self.add_error(&format!("expected definition, got {}", other), token.span);
//                 None
//             }
//         }
//     }
//
//     fn parse_fn_def(&mut self) -> Option<WithSpan<ExternDefinition>> {
//         let fn_token = self.expect(TokenKind::Fn)?;
//         let name = self.parse_ident()?;
//         self.expect(TokenKind::LeftParen)?;
//         let mut params = vec![];
//         while self.peek() != TokenKind::RightParen {
//             let ident = self.parse_ident()?;
//
//             self.expect(TokenKind::Colon)?;
//             let ty = self.parse_type()?;
//
//             if self.peek() == TokenKind::Comma {
//                 self.expect(TokenKind::Comma)?;
//             }
//
//             params.push(FnParam {
//                 kind: FnParamKind::Regular,
//                 ty,
//                 name: WithSpan::new(ident.value, ident.span),
//                 default_value: None,
//             });
//         }
//         let right_paren = self.expect(TokenKind::RightParen)?;
//
//         let returns = if self.peek() == TokenKind::Arrow {
//             self.expect(TokenKind::Arrow)?;
//             let mut returns = vec![];
//             loop {
//                 let ty = self.parse_type()?;
//                 returns.push(ty);
//                 if self.peek() == TokenKind::Comma {
//                     self.expect(TokenKind::Comma)?;
//                 } else {
//                     break;
//                 }
//             }
//             returns
//         } else {
//             vec![]
//         };
//         self.expect(TokenKind::Semicolon)?;
//         let span = fn_token
//             .span
//             .union(returns.last().map(|r| r.span).unwrap_or(right_paren.span));
//         Some(WithSpan::new(
//             ExternDefinition::Fn(ExternFn {
//                 name: name.value,
//                 params,
//                 returns,
//                 ty: None,
//             }),
//             span,
//         ))
//     }
//
//     fn parse_stmt(&mut self) -> Option<WithSpan<Stmt>> {
//         match self.peek() {
//             TokenKind::Let | TokenKind::Global => self.parse_binding(),
//             TokenKind::Semicolon => {
//                 let semi = self.expect(TokenKind::Semicolon)?;
//                 Some(WithSpan::new(Stmt::Empty, semi.span))
//             }
//             TokenKind::Return => self.parse_return(),
//             _ => self.parse_stmt_expr(),
//         }
//     }
//
//     fn parse_return(&mut self) -> Option<WithSpan<Stmt>> {
//         let span = self.expect(TokenKind::Return)?.span;
//         if self.check(TokenKind::Semicolon) {
//             let semi = self.expect(TokenKind::Semicolon)?;
//             return Some(WithSpan::new(Stmt::Return(vec![]), span.union(semi.span)));
//         }
//         let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
//         let mut span = span.union(exprs[0].span);
//         while let TokenKind::Comma = self.peek() {
//             self.expect(TokenKind::Comma)?;
//             if let Some(semi) = self.parse_optional_semi() {
//                 span = span.union(semi);
//
//                 return Some(WithSpan::new(Stmt::Return(exprs), span));
//             }
//             let expr = self.parse_expr(Precedence::Lowest)?;
//             span = span.union(expr.span);
//             exprs.push(expr);
//         }
//
//         let span = self
//             .parse_optional_semi()
//             .map(|s| span.union(s))
//             .unwrap_or(span);
//
//         Some(WithSpan::new(Stmt::Return(exprs), span))
//     }
//
//     fn parse_impl_fn(&mut self, target: WithSpan<&Type>) -> Option<WithSpan<ItemFn>> {
//         let fn_token = self.expect(TokenKind::Fn)?;
//         let name = self.parse_ident()?;
//         self.expect(TokenKind::LeftParen)?;
//         let mut params = vec![];
//         while self.peek() != TokenKind::RightParen {
//             match self.peek() {
//                 Token![self] => {
//                     self.expect(Token![self])?;
//                     if self.peek() == TokenKind::Comma {
//                         self.expect(TokenKind::Comma)?;
//                     }
//                     params.push(FnParam {
//                         kind: FnParamKind::Receiver,
//                         ty: target.clone().map(|t| t.clone()),
//                         name: WithSpan::new("self".to_string(), target.span),
//                         default_value: None,
//                     });
//                 }
//                 _ => {
//                     let ident = self.parse_ident()?;
//
//                     self.expect(TokenKind::Colon)?;
//                     let ty = self.parse_type()?;
//
//                     let default_value = if self.peek() == TokenKind::Equal {
//                         self.expect(TokenKind::Equal)?;
//                         Some(self.parse_expr(Precedence::Lowest)?)
//                     } else {
//                         None
//                     };
//
//                     if self.peek() == TokenKind::Comma {
//                         self.expect(TokenKind::Comma)?;
//                     }
//
//                     params.push(FnParam {
//                         kind: FnParamKind::Regular,
//                         ty,
//                         name: WithSpan::new(ident.value, ident.span),
//                         default_value,
//                     });
//                 }
//             }
//         }
//         self.expect(TokenKind::RightParen)?;
//
//         let returns = if self.peek() == TokenKind::Arrow {
//             self.expect(TokenKind::Arrow)?;
//             let mut returns = vec![];
//             loop {
//                 let ty = self.parse_type()?;
//                 returns.push(ty);
//                 if self.peek() == TokenKind::Comma {
//                     self.expect(TokenKind::Comma)?;
//                 } else {
//                     break;
//                 }
//             }
//             returns
//         } else {
//             vec![]
//         };
//
//         let body = self.parse_block()?;
//         Some(WithSpan::new(
//             ItemFn {
//                 name: name.value,
//                 params,
//                 body: match body.value {
//                     Expr::Block(block) => WithSpan::new(block, body.span),
//                     _ => unreachable!(),
//                 },
//                 output: returns,
//                 ty: None,
//             },
//             fn_token.span.union(body.span),
//         ))
//     }
//     fn parse_fn(&mut self) -> Option<WithSpan<Item>> {
//         let fn_token = self.expect(TokenKind::Fn)?;
//         let name = self.parse_ident()?;
//         self.expect(TokenKind::LeftParen)?;
//         let mut params = vec![];
//         while self.peek() != TokenKind::RightParen {
//             let ident = self.parse_ident()?;
//
//             self.expect(TokenKind::Colon)?;
//             let ty = self.parse_type()?;
//
//             let default_value = if self.peek() == TokenKind::Equal {
//                 self.expect(TokenKind::Equal)?;
//                 Some(self.parse_expr(Precedence::Lowest)?)
//             } else {
//                 None
//             };
//
//             if self.peek() == TokenKind::Comma {
//                 self.expect(TokenKind::Comma)?;
//             }
//
//             params.push(FnParam {
//                 kind: FnParamKind::Regular,
//                 ty,
//                 name: WithSpan::new(ident.value, ident.span),
//                 default_value,
//             });
//         }
//         self.expect(TokenKind::RightParen)?;
//
//         let returns = if self.peek() == TokenKind::Arrow {
//             self.expect(TokenKind::Arrow)?;
//             let mut returns = vec![];
//             loop {
//                 let ty = self.parse_type()?;
//                 returns.push(ty);
//                 if self.peek() == TokenKind::Comma {
//                     self.expect(TokenKind::Comma)?;
//                 } else {
//                     break;
//                 }
//             }
//             returns
//         } else {
//             vec![]
//         };
//
//         let body = self.parse_block()?;
//         Some(WithSpan::new(
//             Item::Fn(ItemFn {
//                 name: name.value,
//                 params,
//                 body: match body.value {
//                     Expr::Block(block) => WithSpan::new(block, body.span),
//                     _ => unreachable!(),
//                 },
//                 output: returns,
//                 ty: None,
//             }),
//             fn_token.span.union(body.span),
//         ))
//     }
//
//     fn parse_stmt_expr(&mut self) -> Option<WithSpan<Stmt>> {
//         let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
//         let mut span = exprs[0].span;
//         while let TokenKind::Comma = self.peek() {
//             self.expect(TokenKind::Comma)?;
//             if self.peek() == TokenKind::Semicolon {
//                 let semi = self.expect(TokenKind::Semicolon)?;
//                 let span = span.union(semi.span);
//                 return Some(WithSpan::new(
//                     Stmt::Expr(StmtExpr {
//                         exprs,
//                         semi: Some(semi.span),
//                     }),
//                     span,
//                 ));
//             }
//
//             let expr = self.parse_expr(Precedence::Lowest)?;
//             span = span.union(expr.span);
//             exprs.push(expr);
//         }
//
//         if TokenKind::Equal == self.peek() {
//             self.expect(TokenKind::Equal)?;
//             let idents = exprs
//                 .into_iter()
//                 .map(|e| {
//                     if let WithSpan {
//                         value: Expr::Path(i, None),
//                         span,
//                     } = e
//                     {
//                         WithSpan::new(i, span)
//                     } else {
//                         panic!("expected identifier");
//                     }
//                 })
//                 .collect::<Vec<_>>();
//             let mut values = vec![self.parse_expr(Precedence::Lowest)?];
//             span = span.union(values[0].span);
//             while let TokenKind::Comma = self.peek() {
//                 self.expect(TokenKind::Comma)?;
//                 if self.peek() == TokenKind::Semicolon {
//                     let semi = self.expect(TokenKind::Semicolon)?;
//                     let span = span.union(semi.span);
//                     return Some(WithSpan::new(
//                         Stmt::Assign(Assign {
//                             idents,
//                             values: Some(values),
//                         }),
//                         span,
//                     ));
//                 }
//
//                 let value = self.parse_expr(Precedence::Lowest)?;
//                 span = span.union(value.span);
//                 values.push(value);
//             }
//
//             let semi = self.parse_optional_semi();
//             let span = semi.map(|s| span.union(s)).unwrap_or(span);
//             return Some(WithSpan::new(
//                 Stmt::Assign(Assign {
//                     idents,
//                     values: Some(values),
//                 }),
//                 span,
//             ));
//         }
//
//         let semi = self.parse_optional_semi();
//         let span = semi.map(|s| span.union(s)).unwrap_or(span);
//         Some(WithSpan::new(Stmt::Expr(StmtExpr { exprs, semi }), span))
//     }
//
//     fn parse_optional_mark(&mut self) -> Option<position::Span> {
//         match self.peek() {
//             TokenKind::QuestionMark => {
//                 let mark = self.advance();
//                 Some(mark.span)
//             }
//             _ => None,
//         }
//     }
//
//     fn parse_optional_semi(&mut self) -> Option<position::Span> {
//         match self.peek() {
//             TokenKind::Semicolon => {
//                 let mark = self.advance();
//                 Some(mark.span)
//             }
//             _ => None,
//         }
//     }
//
//     fn parse_type(&mut self) -> Option<WithSpan<Type>> {
//         Some(match self.peek() {
//             TokenKind::Nil => WithSpan::new(
//                 Type::Checked(types::Type::non_nilable(types::TypeKind::Primitive(
//                     Primitive::Nil,
//                 ))),
//                 self.expect(TokenKind::Nil)?.span,
//             ),
//             TokenKind::Fn => {
//                 let fn_token = self.expect(TokenKind::Fn)?;
//                 self.expect(TokenKind::LeftParen)?;
//                 let mut params = vec![];
//                 while self.peek() != TokenKind::RightParen {
//                     if self.peek() == TokenKind::Identifier && self.peek_next() == TokenKind::Colon
//                     {
//                         let ident = self.parse_ident()?;
//
//                         self.expect(TokenKind::Colon)?;
//                         let ty = self.parse_type()?;
//                         if self.peek() == TokenKind::Comma {
//                             self.expect(TokenKind::Comma)?;
//                         }
//
//                         params.push(BareFnParam {
//                             kind: FnParamKind::Regular,
//                             ty,
//                             name: Some(ident.value),
//                             default_value: None,
//                         });
//                     } else {
//                         let ty = self.parse_type()?;
//                         if self.peek() == TokenKind::Comma {
//                             self.expect(TokenKind::Comma)?;
//                         }
//
//                         params.push(BareFnParam {
//                             kind: FnParamKind::Regular,
//                             ty,
//                             name: None,
//                             default_value: None,
//                         });
//                     }
//                 }
//                 let right_paren = self.expect(TokenKind::RightParen)?;
//                 let returns = if self.peek() == TokenKind::Arrow {
//                     self.expect(TokenKind::Arrow)?;
//                     let mut returns = vec![];
//                     loop {
//                         let ty = self.parse_type()?;
//                         returns.push(ty);
//                         if self.peek() == TokenKind::Comma {
//                             self.expect(TokenKind::Comma)?;
//                         } else {
//                             break;
//                         }
//                     }
//                     returns
//                 } else {
//                     vec![]
//                 };
//                 let span = fn_token
//                     .span
//                     .union(returns.last().map(|r| r.span).unwrap_or(right_paren.span));
//                 WithSpan::new(
//                     Type::Ast(AstType::non_nilable(TypeKind::Fn(FnType {
//                         params,
//                         output: returns.into_iter().collect(),
//                     }))),
//                     span,
//                 )
//             }
//             TokenKind::Identifier => {
//                 let ident = self.parse_ident()?;
//                 let mark = self.parse_optional_mark();
//                 let span = mark.map(|s| ident.span.union(s)).unwrap_or(ident.span);
//                 let primitive = Primitive::from_ident_primitive(&ident.value);
//                 WithSpan::new(
//                     primitive
//                         .map(|p| {
//                             Type::Checked(types::Type {
//                                 kind: types::TypeKind::Primitive(p),
//                                 nilable: mark.is_some(),
//                             })
//                         })
//                         .unwrap_or_else(|| {
//                             Type::Ast(AstType {
//                                 kind: TypeKind::Path(ident.clone()),
//                                 nilable: mark.is_some(),
//                             })
//                         }),
//                     span,
//                 )
//             }
//             _ => {
//                 let token = self.advance();
//                 self.add_error(
//                     &format!("expected type, got {}", token.value.kind()),
//                     token.span,
//                 );
//                 return None;
//             }
//         })
//     }
//
//     fn parse_binding(&mut self) -> Option<WithSpan<Stmt>> {
//         let binding_token = self.advance();
//         let binding_type = match binding_token.value {
//             Token::Let => BindingKind::Local,
//             Token::Global => BindingKind::Global,
//             _ => unreachable!(),
//         };
//
//         let mut identifiers = vec![];
//         let mut types = vec![];
//         let mut span = binding_token.span;
//
//         loop {
//             let WithSpan {
//                 value: Token::Identifier(ident),
//                 span: ident_span,
//             } = self.advance()
//             else {
//                 panic!("expected identifier");
//             };
//
//             span = span.union(*ident_span);
//             identifiers.push(WithSpan::new(ident.clone(), *ident_span));
//
//             if self.peek() == TokenKind::Colon {
//                 self.expect(TokenKind::Colon)?;
//                 let ty = self.parse_type()?;
//                 types.push(Some(ty));
//             } else {
//                 types.push(None);
//             }
//
//             if self.peek() == TokenKind::Comma {
//                 self.expect(TokenKind::Comma)?;
//             } else {
//                 break;
//             }
//         }
//
//         if self.match_token(TokenKind::Equal).is_some() {
//             let mut exprs = vec![self.parse_expr(Precedence::Lowest)?];
//             let mut span = span.union(exprs[0].span);
//             while let TokenKind::Comma = self.peek() {
//                 self.expect(TokenKind::Comma)?;
//                 if let Some(semi) = self.parse_optional_semi() {
//                     span = span.union(semi);
//
//                     return Some(WithSpan::new(
//                         Stmt::Binding(Binding {
//                             kind: binding_type,
//                             idents: identifiers,
//                             exprs: Some(exprs),
//                             types,
//                         }),
//                         span,
//                     ));
//                 }
//                 let expr = self.parse_expr(Precedence::Lowest)?;
//                 span = span.union(expr.span);
//                 exprs.push(expr);
//             }
//
//             let span = self
//                 .parse_optional_semi()
//                 .map(|s| span.union(s))
//                 .unwrap_or(span);
//
//             Some(WithSpan::new(
//                 Stmt::Binding(Binding {
//                     kind: binding_type,
//                     idents: identifiers,
//                     exprs: Some(exprs),
//                     types,
//                 }),
//                 span,
//             ))
//         } else {
//             let span = self
//                 .parse_optional_semi()
//                 .map(|s| span.union(s))
//                 .unwrap_or(span);
//
//             Some(WithSpan::new(
//                 Stmt::Binding(Binding {
//                     kind: binding_type,
//                     idents: identifiers,
//                     exprs: None,
//                     types,
//                 }),
//                 span,
//             ))
//         }
//     }
//
//     fn parse_program(&mut self) -> Option<Vec<WithSpan<Item>>> {
//         let mut items = vec![];
//
//         while !self.is_at_end() {
//             if let Some(item) = self.parse_item() {
//                 items.push(item);
//             } else {
//                 self.sync();
//             }
//         }
//
//         if self.diagnostics.is_empty() {
//             Some(items)
//         } else {
//             None
//         }
//     }
// }
//
// pub fn parse_program(
//     tokens: &[WithSpan<Token>],
// ) -> Result<Vec<WithSpan<Item>>, Vec<position::Diagnostic>> {
//     let mut parser = Parser::new(tokens);
//     match parser.parse_program() {
//         Some(output) => Ok(output),
//         None => Err(parser.diagnostics),
//     }
// }
//
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
