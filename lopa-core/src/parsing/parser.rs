use Syntax::*;
use std::ops::Range;
use std::{fmt, iter::Peekable};

use logos::Logos;
use rowan::{
    Checkpoint, GreenNode, GreenNodeBuilder, NodeOrToken, SyntaxNode, SyntaxToken, TextRange,
    TextSize,
};

use crate::parsing::token_set::TokenSet;

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
                    if !matches!(t.kind(), T![" "]) {
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

const STMT_RECOVERY: TokenSet = TokenSet::new(&[T![fn]]);
const PARAM_LIST_RECOVERY: TokenSet =
    TokenSet::new(&[T![->], T!["{"], T![fn]]).union(STMT_RECOVERY);
const STMT_EXPR_RECOVERY: TokenSet =
    TokenSet::new(&[T![let], T!["{"], T!["}"]]).union(STMT_RECOVERY);
const ARG_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T![")"]]);
const EXPR_FIRST: TokenSet = TokenSet::new(&[
    IDENT,
    T![-],
    T![not],
    INT,
    FLOAT,
    STRING,
    T!["["],
    T!["{"],
    T![return],
    T![if],
    T![|],
    T!["("],
]);
const TYPE_FIRST: TokenSet = TokenSet::new(&[IDENT, T![fn]]);
const ITEM_FIRST: TokenSet = TokenSet::new(&[T![fn], T![mod]]);
const PATTERN_FIRST: TokenSet = TokenSet::new(&[IDENT]);
const PATTERN_RECOVERY: TokenSet = TokenSet::new(&[T![=]]).union(PARAM_LIST_RECOVERY);

pub struct Parser<'a> {
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

    fn add_error(&mut self, kind: ErrorKind, span: Option<Range<usize>>) {
        let span = span.unwrap_or_else(|| self.input.last_token_span.clone());
        self.errors.push(ParseError::new(
            TextRange::new(TextSize::new(span.start as _), TextSize::new(span.end as _)),
            kind,
        ));
    }

    fn whitespace(&mut self) {
        if !self.input.at(T![" "]) {
            return;
        }
        let span = self.input.advance();
        self.builder
            .token(T![" "].into(), &self.input.content[span]);
    }

    fn advance_with_err(&mut self, kind: ErrorKind) {
        let span = self.input.advance();
        self.builder
            .token(Syntax::ERROR.into(), &self.input.content[span.clone()]);
        self.add_error(kind, Some(span));
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

    fn with<R>(&mut self, syntax: Syntax, body: impl FnOnce(&mut Self) -> R) -> R {
        self.builder.start_node(syntax.into());
        let res = body(self);
        self.builder.finish_node();
        res
    }

    fn with_at<R>(
        &mut self,
        syntax: Syntax,
        checkpoint: Checkpoint,
        body: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.builder.start_node_at(checkpoint, syntax.into());
        let res = body(self);
        self.builder.finish_node();
        res
    }

    fn expect(&mut self, token: Syntax) {
        if self.ate(token) {
            return;
        }

        self.add_error(
            ErrorKind::ExpectedToken(token),
            Some(self.input.nth_span(0)),
        );
    }

    fn expect_any(&mut self, tokens: &[Syntax]) {
        if self.ate_any(tokens) {
            return;
        }

        self.add_error(
            ErrorKind::ExpectedTokens(tokens.into()),
            Some(self.input.nth_span(0)),
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
            while !this.input.at(EOF) {
                this.item();
            }
        });
    }

    fn item(&mut self) {
        if self.input.at_any(ITEM_FIRST) {
            match self.input.peek() {
                T![fn] => self.fn_item(),
                T![mod] => self.mod_item(),
                _ => {
                    unreachable!();
                }
            };
        } else {
            self.advance_with_err(ErrorKind::ExpectedItem);
        }
    }

    fn mod_item(&mut self) {
        self.with(Syntax::MOD_ITEM, |this| {
            this.expect(T![mod]);
            this.name();

            if this.ate(T![;]) {
                return;
            }

            this.expect(T!["{"]);
            while !this.input.at(T!["}"]) && !this.input.at(EOF) {
                this.item();
            }
            this.expect(T!["}"]);
        })
    }

    fn fn_item(&mut self) {
        self.with(Syntax::FN_ITEM, |this| {
            this.expect(T![fn]);

            this.name();
            this.param_list();
            if this.input.at(T![->]) {
                this.return_type();
            }
            if this.input.at(T!["{"]) {
                this.block();
            } else {
                this.add_error(ErrorKind::ExpectedToken(T!["{"]), None);
            }
        })
    }

    fn path(&mut self) {
        if self.input.at(Syntax::IDENT) {
            self.with(Syntax::PATH, |this| {
                this.expect(IDENT);
                while this.at_path_sep(0) && !this.input.at(EOF) {
                    this.expect(T![:]);
                    this.expect(T![:]);
                    this.expect(IDENT);
                }
            });
        }
    }

    fn name(&mut self) {
        self.with(Syntax::NAME, |this| {
            this.expect(IDENT);
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
            this.pattern();
            this.expect(T![:]);
            this.type_expr();
            if this.ate(T![=]) {
                this.expect_expr();
            }
            this.ate(T![,]);
        })
    }

    fn param_list(&mut self) {
        self.with(Syntax::PARAM_LIST, |this| {
            this.expect(T!("("));
            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                if this.input.at(IDENT) {
                    this.param();
                } else {
                    if this.input.at_any(PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedParameter);
                }
            }
            this.expect(T!(")"));
        })
    }

    fn type_expr(&mut self) {
        if self.input.at_any(TYPE_FIRST) {
            #[allow(clippy::match_single_binding)]
            match self.input.peek() {
                T![fn] => {
                    self.fn_type();
                }
                _ => {
                    let checkpoint = self.builder.checkpoint();

                    let next_span = self.input.nth_span(0);
                    let is_path = self.at_path_sep(1);
                    self.path();
                    if !is_path {
                        match &self.input.content[next_span] {
                            "int" | "float" | "string" | "bool" => {
                                self.with_at(Syntax::LIT_TYPE, checkpoint, |_| {})
                            }
                            "any" => self.with_at(Syntax::ANY_TYPE, checkpoint, |_| {}),
                            _ => {}
                        }
                    }
                    if self.ate(T![?]) {
                        self.with_at(Syntax::NILABLE_TYPE, checkpoint, |_| {});
                    }
                }
            }
        } else {
            self.advance_with_err(ErrorKind::ExpectedType);
        }
    }

    fn fn_type(&mut self) {
        self.with(Syntax::FN_TYPE, |this| {
            this.expect(T![fn]);
            this.fn_type_param_list();
            if this.input.at(T![->]) {
                this.return_type();
            }
        });
    }

    fn fn_type_param_list(&mut self) {
        self.with(Syntax::PARAM_LIST, |this| {
            this.expect(T!["("]);
            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                if this.input.at_any(TYPE_FIRST) || this.input.at(IDENT) {
                    this.fn_type_param();
                } else {
                    if this.input.at_any(PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedParameter);
                }
            }
            this.expect(T!(")"));
        })
    }

    fn fn_type_param(&mut self) {
        self.with(Syntax::PARAM, |this| {
            if this.input.nth(1) == T![:] {
                this.name();
                this.expect(T![:]);
            }
            this.type_expr();
            this.ate(T![,]);
        })
    }

    fn stmt_expr(&mut self) {
        self.with(Syntax::EXPR_STMT, |this| {
            this.expr();
            if this.input.at(T![;]) {
                this.expect(T![;]);
            }
        });
    }

    fn prefix_expr(&mut self) -> Option<()> {
        match self.input.peek() {
            T![return] => self.return_expr(),
            T![if] => self.if_expr(),
            _ => return self.expr_primary(),
        }
        Some(())
    }

    fn if_expr(&mut self) {
        self.with(Syntax::IF_EXPR, |this| {
            this.expect(T![if]);
            this.expr();
            this.block();
            if this.ate(T![else]) {
                if this.input.at(T![if]) {
                    this.if_expr();
                } else {
                    this.block();
                }
            }
        });
    }

    fn return_expr(&mut self) {
        self.with(Syntax::RETURN_EXPR, |this| {
            this.expect(T![return]);
            if this.input.at_any(EXPR_FIRST) {
                this.expr();
            }
        })
    }

    fn expr_primary(&mut self) -> Option<()> {
        let token = self.input.peek();
        match token {
            INT | FLOAT | STRING | TRUE_KW | FALSE_KW | NIL_KW => {
                self.with(Syntax::LIT_EXPR, |this| {
                    this.ate(token);
                });
            }
            T!["("] => {
                self.with(Syntax::PATH_EXPR, |this| {
                    this.expect(T!["("]);
                    this.expect_expr();
                    this.expect(T![")"]);
                });
            }
            T!["{"] => {
                self.block();
            }
            IDENT => {
                if self.at_path_sep(1) {
                    self.with(Syntax::PATH_EXPR, |this| {
                        this.path();
                    });
                } else {
                    self.with(Syntax::NAME_EXPR, |this| this.name());
                }
            }
            _ => {
                self.advance_with_err(ErrorKind::ExpectedExpression);
                return None;
            }
        }
        Some(())
    }

    fn arg_list(&mut self) {
        self.with(Syntax::ARG_LIST, |this| {
            this.expect(T!["("]);
            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                if this.input.at_any(EXPR_FIRST) {
                    this.arg();
                } else {
                    if this.input.at_any(ARG_LIST_RECOVERY) {
                        break;
                    } else {
                        this.advance_with_err(ErrorKind::ExpectedArgument);
                    }
                }
            }
            this.expect(T![")"]);
        })
    }

    fn index(&mut self) {
        self.expect(T!["["]);
        self.expect_expr();
        self.expect(T!["]"]);
    }

    fn arg(&mut self) {
        self.with(Syntax::ARG, |this| {
            if this.input.nth(1) == T![:] {
                this.name();
                this.expect(T![:]);
            }
            this.expect_expr();
            this.ate(T![,]);
        });
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    fn expect_expr(&mut self) -> Option<()> {
        if self.input.at_any(EXPR_FIRST) {
            self.expr();
            Some(())
        } else {
            self.advance_with_err(ErrorKind::ExpectedExpression);
            None
        }
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let checkpoint = self.builder.checkpoint();

        match self.input.peek().prefix_bp() {
            Some(rbp) => {
                self.with(Syntax::UNARY_EXPR, |this| {
                    this.expect(this.input.peek());
                    this.expr_bp(rbp);
                });
            }
            None => {
                if self.prefix_expr().is_none() {
                    return;
                }
            }
        };

        loop {
            match self.input.peek() {
                T!["("] => {
                    self.with_at(Syntax::CALL_EXPR, checkpoint, |this| this.arg_list());
                }
                T!["["] => {
                    self.with_at(Syntax::INDEX_EXPR, checkpoint, |this| this.index());
                }
                _ => break,
            }
        }
        loop {
            let op = self.input.peek();

            if let Some(postfix_bp) = op.postfix_bp() {
                if postfix_bp < min_bp {
                    break;
                }
                match op {
                    T![?] => {
                        self.with_at(Syntax::TRY_EXPR, checkpoint, |this| {
                            this.expect(op);
                        });
                    }
                    _ => unreachable!(),
                }
                continue;
            }

            let Some((left_bp, right_bp)) = op.infix_bp() else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.expect(op);

            if self.input.at_any(EXPR_FIRST) {
                self.with_at(BINARY_EXPR, checkpoint, |this| this.expr_bp(right_bp));
            } else {
                self.advance_with_err(ErrorKind::ExpectedExpression);
            }
        }
    }

    fn stmt_let(&mut self) {
        self.with(Syntax::LET_STMT, |this| {
            this.expect(T![let]);
            this.pattern();
            if this.ate(T![:]) {
                this.type_expr();
            }
            this.expect(T![=]);
            this.expect_expr();
            this.expect(T![;]);
        })
    }

    fn pattern(&mut self) {
        match self.input.peek() {
            IDENT => {
                self.with(Syntax::NAME_PATTERN, |this| {
                    this.name();
                });
            }
            _ => {
                self.add_error(ErrorKind::ExpectedPattern, None);
                return;
            }
        }
    }

    fn block(&mut self) {
        self.with(Syntax::BLOCK_EXPR, |this| {
            this.expect(T!["{"]);
            while !this.input.at(T!["}"]) && !this.input.at(EOF) {
                match this.input.peek() {
                    T![let] => this.stmt_let(),
                    T![;] => {
                        this.expect(T![;]);
                    }
                    _ => {
                        if this.input.at_any(EXPR_FIRST) {
                            this.stmt_expr()
                        } else {
                            if this.input.at_any(STMT_EXPR_RECOVERY) {
                                break;
                            }

                            this.advance_with_err(ErrorKind::ExpectedStatement);
                        }
                    }
                }
            }

            this.expect(T!["}"]);
        });
    }

    fn at_path_sep(&self, offset: usize) -> bool {
        self.input.nth(offset) == self.input.nth(1 + offset) && self.input.nth(offset) == T![:]
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
    ExpectedParameter,
    ExpectedPattern,
    ExpectedItem,
    Other(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedToken(tok) => return write!(f, "expected {}", tok),
            Self::ExpectedTokens(toks) => return write!(f, "expecteded any of: {:?}", toks),
            Self::ExpectedArgument => "expected argument",
            Self::ExpectedStatement => "expected statement",
            Self::ExpectedType => "expected type",
            Self::ExpectedExpression => "expected expression",
            Self::ExpectedParameter => "expected parameter",
            Self::ExpectedPattern => "expected pattern",
            Self::ExpectedItem => "expected item",
            Self::Other(text) => text,
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
    last_token_span: Range<usize>,
    fuel: u32,
}

impl<'a> Input<'a> {
    pub(super) fn new(content: &'a str) -> Self {
        Self {
            content,
            lexer: Syntax::lexer(content).spanned().peekable(),
            fuel: 256,
            last_token_span: 0..0,
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

    fn at_any(&self, tokens: TokenSet) -> bool {
        tokens.contains(self.peek())
    }

    fn advance(&mut self) -> Range<usize> {
        self.fuel = 256;
        let span = self.lexer.next().map(|(_, span)| span).unwrap_or_else(|| {
            let len = self.content.len();
            len..len
        });
        self.last_token_span = span.clone();
        span
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

#[cfg(test)]
mod test {
    use rowan::{GreenNode, NodeOrToken, SyntaxToken};

    use crate::parsing::{
        ast::SyntaxNode,
        parser::{Lang, ParseError, Parser},
    };

    #[track_caller]
    fn try_parse(source: &str, f: impl FnOnce(&mut Parser)) -> (SyntaxNode, Vec<ParseError>) {
        let mut parser = Parser::new(source);
        f(&mut parser);
        (SyntaxNode::new_root(parser.builder.finish()), parser.errors)
    }

    #[track_caller]
    fn parse(source: &str, f: impl FnOnce(&mut Parser)) -> String {
        let mut parser = Parser::new(source);
        f(&mut parser);
        if !parser.errors.is_empty() {
            panic!("{:?}", parser.errors);
        }
        let node = SyntaxNode::new_root(parser.builder.finish());
        let mut result = String::new();

        fn parse_rec(
            child: NodeOrToken<SyntaxNode, SyntaxToken<Lang>>,
            result: &mut String,
            depth: usize,
        ) {
            (0..depth).for_each(|_| result.push_str("  "));
            result.push_str(&format!(
                "{}: {}..{}",
                child.kind(),
                u32::from(child.text_range().start()),
                u32::from(child.text_range().end())
            ));
            result.push('\n');
            if let NodeOrToken::Node(node) = child {
                for child in node.children_with_tokens() {
                    parse_rec(child, result, depth + 1);
                }
            }
        }

        parse_rec(NodeOrToken::Node(node), &mut result, 0);
        result
    }

    #[test]
    fn file() {
        insta::assert_snapshot!(parse("fn some_func(){}", |p| p.file()));
    }

    #[test]
    fn mod_item() {
        insta::assert_snapshot!(parse("mod my_mod { fn some_item() {} }", |p| p.mod_item()));
        insta::assert_snapshot!(parse("mod my_mod;", |p| p.mod_item()));
    }

    #[test]
    fn func_item() {
        insta::assert_snapshot!(parse("fn test(a: int, b: string)->int { stmt; }", |p| p.fn_item()));
    }

    #[test]
    fn path() {
        insta::assert_snapshot!(parse("a::b::c", |p| p.path()));
        insta::assert_snapshot!(parse("simple_path", |p| p.path()));
    }

    #[test]
    fn name() {
        insta::assert_snapshot!(parse("some_name", |p| p.name()));
    }

    #[test]
    fn return_type() {
        insta::assert_snapshot!(parse("-> SomeType", |p| p.return_type()));
    }

    #[test]
    fn param() {
        insta::assert_snapshot!(parse("param: type", |p| p.param()));
    }

    #[test]
    fn param_list() {
        insta::assert_snapshot!(parse("(a: int, b: string)", |p| p.param_list()));
    }

    #[test]
    fn type_expr() {
        insta::assert_snapshot!(parse("int", |p| p.type_expr()));
        insta::assert_snapshot!(parse("NotInt", |p| p.type_expr()));
        insta::assert_snapshot!(parse("fn(a: int, string) -> Result", |p| p.type_expr()));
    }

    #[test]
    fn stmt_expr() {
        insta::assert_snapshot!(parse("a;", |p| p.stmt_expr()));
        insta::assert_snapshot!(parse("1+1;", |p| p.stmt_expr()));
        insta::assert_snapshot!(parse("print();", |p| p.stmt_expr()));
        insta::assert_snapshot!(parse("no_semi % idk", |p| p.stmt_expr()));
    }

    #[test]
    fn stmt_let() {
        insta::assert_snapshot!(parse("let x = 1;", |p| p.stmt_let()));
    }

    #[test]
    fn block() {
        insta::assert_snapshot!(parse("{ }", |p| p.block()));
        insta::assert_snapshot!(parse("{ 1 }", |p| p.block()));
        insta::assert_snapshot!(parse("{ something; something_else; }", |p| p.block()));
    }

    #[test]
    fn expr() {
        insta::assert_snapshot!(parse("1", |p| p.expr()));
        insta::assert_snapshot!(parse("1+1", |p| p.expr()));
        insta::assert_snapshot!(parse("1+1*3/4%3", |p| p.expr()));
        insta::assert_snapshot!(parse("(1)", |p| p.expr()));
        insta::assert_snapshot!(parse("1 + { 1 }", |p| p.expr()));
        insta::assert_snapshot!(parse("if not true {} else {}", |p| p.expr()));
        insta::assert_snapshot!(parse("if true {} else if VALUE { yo_mister_white }", |p| p.expr()));
        insta::assert_snapshot!(parse("\"a string\"", |p| p.expr()));
        insta::assert_snapshot!(parse("a[1](2)[3]", |p| p.expr()));
        insta::assert_snapshot!(parse("a = b = c", |p| p.expr()));
    }

    #[test]
    fn numbers() {
        insta::assert_snapshot!(parse("10.10", |p| p.expr()));
        insta::assert_snapshot!(parse("1_000_000", |p| p.expr()));
    }
}
