use std::ops::Range;
use std::{fmt, iter::Peekable};

use logos::Logos;
use rowan::{
    GreenNode, GreenNodeBuilder, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, TextRange,
    TextSize,
};

use super::lexer::Syntax;

pub type Cst = GreenNode;

pub trait Prettify {
    fn prettify(&self) -> String;
}

impl Prettify for SyntaxNode<Lang> {
    fn prettify(&self) -> String {
        fn children(
            node_or_token: &NodeOrToken<SyntaxNode<Lang>, SyntaxToken<Lang>>,
            result: &mut String,
            depth: u32,
        ) {
            match node_or_token {
                NodeOrToken::Node(n) => {
                    (0..(depth)).for_each(|_| result.push(' '));
                    result.push_str(&n.kind().to_string());
                    result.push('\n');
                }
                NodeOrToken::Token(t) => {
                    if !matches!(t.kind(), T![' ']) {
                        (0..(depth)).for_each(|_| result.push(' '));
                        result.push('\'');
                        result.push_str(t.text());
                        result.push('\'');
                        result.push('\n');
                    }
                }
            }
            if let NodeOrToken::Node(node) = node_or_token {
                for node in node.children_with_tokens() {
                    children(&node, result, depth + 1);
                }
            }
        }
        let mut result = String::new();
        children(&NodeOrToken::Node(self.clone()), &mut result, 0);
        result
    }
}

pub fn parse(input: &str) -> (Cst, Vec<ParseError>) {
    Parser::new(input).parse()
}

struct Parser<'a> {
    input: Input<'a>,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: Input::new(input),
            builder: GreenNodeBuilder::default(),
            errors: Default::default(),
        }
    }

    fn add_error(&mut self, kind: ErrorKind, span: Range<usize>) {
        self.errors.push(ParseError::new(
            TextRange::new(TextSize::new(span.start as _), TextSize::new(span.end as _)),
            kind,
        ));
    }

    fn whitespace(&mut self) {
        if !self.input.at(T![' ']) {
            return;
        }
        let span = self.input.advance();
        self.builder
            .token(T![' '].into(), &self.input.content[span]);
    }

    fn advance_with_err(&mut self, kind: ErrorKind) {
        let span = self.input.advance();
        self.builder
            .token(Syntax::ERROR.into(), &self.input.content[span.clone()]);
        self.add_error(kind, span);
    }

    fn ate_any(&mut self, tokens: &[Syntax]) -> bool {
        for token in tokens {
            if let Some(str) = self.input.eat(*token) {
                self.builder.token((*token).into(), str);
                self.whitespace();
                return true;
            }
        }
        false
    }

    fn ate(&mut self, token: Syntax) -> bool {
        if let Some(str) = self.input.eat(token) {
            self.builder.token(token.into(), str);
            self.whitespace();
            true
        } else {
            false
        }
    }

    fn with<R>(&mut self, token: Syntax, body: impl FnOnce(&mut Self) -> R) -> R {
        self.builder.start_node(token.into());
        let res = body(self);
        self.builder.finish_node();
        res
    }

    fn expect(&mut self, token: Syntax) {
        if self.ate(token) {
            return;
        }

        self.add_error(ErrorKind::ExpectedToken(token), self.input.nth_span(0));
    }

    fn expect_any(&mut self, tokens: &[Syntax]) {
        if self.ate_any(tokens) {
            return;
        }

        self.add_error(
            ErrorKind::ExpectedTokens(tokens.into()),
            self.input.nth_span(0),
        );
    }
}

impl<'a> Parser<'a> {
    fn parse(mut self) -> (Cst, Vec<ParseError>) {
        self.file();
        (self.builder.finish(), self.errors)
    }
    fn file(&mut self) {
        self.with(Syntax::FILE, |this| {
            this.whitespace();
            while !this.input.at(Syntax::EOF) {
                match this.input.peek() {
                    T!(fn) => this.fn_item(),
                    _ => {
                        this.advance_with_err(ErrorKind::ExpectedToken(T![fn]));
                    }
                };
            }
        });
    }

    fn fn_item(&mut self) {
        self.with(Syntax::FN_ITEM, |this| {
            this.expect(T!(fn));
            this.expect(T!(ident));
            if this.input.at(T!['(']) {
                this.param_list();
            }
            if this.input.at(T![->]) {
                this.return_type();
            }
            if this.input.at(T!['{']) {
                this.block();
            }
        })
    }

    fn return_type(&mut self) {
        self.with(Syntax::RETURN_TYPE, |this| {
            this.expect(T![->]);
            this.type_expr();
        });
    }

    fn param(&mut self) {
        self.with(Syntax::PARAM, |this| {
            this.expect(T![ident]);
            this.expect(T![:]);
            this.type_expr();
            this.ate(T![,]);
        })
    }

    fn param_list(&mut self) {
        self.with(Syntax::PARAM_LIST, |this| {
            this.expect(T!('('));
            while !this.input.at_any(&[T![')'], T![eof]]) {
                this.param();
            }
            this.expect(T!(')'));
        })
    }

    fn type_expr(&mut self) {
        self.with(Syntax::TYPE_EXPR, |this| {
            this.expect(T![ident]);
        });
    }

    fn stmt_expr(&mut self) {
        self.with(Syntax::EXPR_STMT, |this| {
            this.expr();
            if this.input.at(T![;]) {
                this.expect(T![;]);
            }
        });
    }

    fn expr_prefix(&mut self) {
        match self.input.peek() {
            T![return] => self.ret(),
            _ => self.expr_primary(),
        }
    }

    fn ret(&mut self) {
        self.with(Syntax::RETURN_EXPR, |this| {
            this.expect(T![return]);
            this.expr();
            if this.input.peek() == T![;] {
                this.expect(T![;]);
            }
        })
    }

    fn expr_primary(&mut self) {
        let checkpoint = self.builder.checkpoint();
        match self.input.peek() {
            T![int] | T![float] | T![true] | T![false] | T![nil] => {
                self.builder
                    .start_node_at(checkpoint, Syntax::LIT_EXPR.into());
                self.expect_any(&[T![int], T![float], T![true], T![false], T![nil]]);
                self.builder.finish_node();
            }
            T!['('] => {
                self.builder
                    .start_node_at(checkpoint, Syntax::PAREN_EXPR.into());
                self.expect(T!['(']);
                self.expr();
                self.expect(T![')']);
                self.builder.finish_node();
            }
            T![ident] => {
                self.builder.start_node_at(checkpoint, Syntax::IDENT.into());
                self.expect(T![ident]);
                self.builder.finish_node();
            }
            _ => {
                self.builder.token(
                    Syntax::ERROR.into(),
                    &self.input.content[self.input.advance()],
                );
            }
        }
    }

    fn arg_list(&mut self) {
        self.with(Syntax::ARG_LIST, |this| {
            this.expect(T!['(']);
            while !this.input.at_any(&[T![')'], T![eof]]) {
                this.arg();
            }
            this.expect(T![')']);
        })
    }

    fn index(&mut self) {
        self.expect(T!['[']);
        self.expr();
        self.expect(T![']']);
    }

    fn arg(&mut self) {
        self.with(Syntax::ARG, |this| {
            this.expr();
            this.ate(T![,]);
        });
    }

    fn expr(&mut self) {
        self.expr_rec(0);
    }

    fn expr_rec(&mut self, min_bp: u8) {
        let checkpoint = self.builder.checkpoint();
        self.expr_prefix();

        while self.input.at_any(&[T!['('], T!['[']]) {
            match self.input.nth(0) {
                T!['('] => {
                    self.builder
                        .start_node_at(checkpoint, Syntax::CALL_EXPR.into());
                    self.arg_list();
                }
                T!['['] => {
                    self.builder
                        .start_node_at(checkpoint, Syntax::INDEX_EXPR.into());
                    self.index();
                }
                _ => unreachable!(),
            }
            self.builder.finish_node();
        }

        loop {
            let op = self.input.nth(0);
            //TODO: check that op is valid
            let Some((left_bp, right_bp)) = self.infix_binding_power(op) else {
                break;
            };
            if left_bp < min_bp {
                break;
            }

            self.expect(op);
            self.expr_rec(right_bp);
            self.builder
                .start_node_at(checkpoint, Syntax::BINARY_EXPR.into());
            self.builder.finish_node();
        }
    }

    fn infix_binding_power(&self, op: Syntax) -> Option<(u8, u8)> {
        Some(match op {
            T![+] | T![-] => (1, 2),
            T![*] | T![/] => (3, 4),
            _ => return None,
        })
    }

    fn stmt_let(&mut self) {
        self.with(Syntax::LIT_EXPR, |this| {
            this.expect(T![let]);
            this.expect(T![ident]);
            this.expect(T![=]);
            this.expr();
            this.expect(T![;]);
        })
    }

    fn block(&mut self) {
        self.with(Syntax::BLOCK_EXPR, |this| {
            this.expect(T!['{']);
            while !this.input.at_any(&[T!('}'), T!(eof)]) {
                match this.input.peek() {
                    T![let] => this.stmt_let(),
                    T![;] => {
                        this.expect(T![;]);
                    }
                    _ => this.stmt_expr(),
                }
            }

            this.expect(T!['}']);
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    ExpectedToken(Syntax),
    ExpectedTokens(Vec<Syntax>),
    ExpectedExpression,
    ExpectedArgument,
    ExpectedStatement,
    ExpectedType,
    ExpectedIdentifier,
    ExpectedParameter,
    ExpectedPattern,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedToken(tok) => return write!(f, "expected {}", tok),
            Self::ExpectedTokens(toks) => return write!(f, "expecteded any of: {:?}", toks),
            Self::ExpectedArgument => "expected argument",
            Self::ExpectedIdentifier => "expected identifier",
            Self::ExpectedStatement => "expected statement",
            Self::ExpectedType => "expected type",
            Self::ExpectedExpression => "expected expression",
            Self::ExpectedParameter => "expected parameter",
            Self::ExpectedPattern => "expected pattern",
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    pub range: TextRange,
    pub kind: ErrorKind,
}

impl ParseError {
    pub fn new(range: TextRange, kin: ErrorKind) -> Self {
        Self { range, kind: kin }
    }
}

pub(super) struct Input<'a> {
    content: &'a str,
    lexer: Peekable<logos::SpannedIter<'a, Syntax>>,
    fuel: u32,
}

impl<'a> Input<'a> {
    pub(super) fn new(content: &'a str) -> Self {
        Self {
            content,
            lexer: Syntax::lexer(content).spanned().peekable(),
            fuel: 256,
        }
    }

    fn nth_span(&self, amount: usize) -> Range<usize> {
        self.lexer
            .clone()
            .nth(amount)
            .map(|(_, span)| span)
            .unwrap_or_else(|| {
                let len = self.content.len();

                len..len
            })
    }

    fn nth(&self, amount: usize) -> Syntax {
        if self.fuel == 0 {
            panic!("parser got stuck")
        }
        self.lexer
            .clone()
            .nth(amount)
            .map(|(token, _)| match token {
                Ok(token) => token,
                Err(_) => Syntax::ERROR,
            })
            .unwrap_or(Syntax::EOF)
    }

    fn eat(&mut self, token: Syntax) -> Option<&str> {
        if self.at(token) {
            Some(&self.content[self.advance()])
        } else {
            None
        }
    }

    fn peek(&self) -> Syntax {
        self.nth(0)
    }

    fn at(&self, token: Syntax) -> bool {
        self.peek() == token
    }

    fn at_any(&self, tokens: &[Syntax]) -> bool {
        tokens.contains(&self.peek())
    }

    fn advance(&mut self) -> Range<usize> {
        self.fuel = 256;
        self.lexer.next().map(|(_, span)| span).unwrap_or_else(|| {
            let len = self.content.len();
            len..len
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {}
impl rowan::Language for Lang {
    type Kind = Syntax;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        unsafe { std::mem::transmute::<u16, Syntax>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}
