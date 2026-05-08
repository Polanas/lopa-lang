use Syntax::*;
use rowan::ast::AstNode;
use std::ops::Range;
use std::{fmt, iter::Peekable};

use logos::Logos;
use rowan::{
    Checkpoint, GreenNode, GreenNodeBuilder, NodeOrToken, SyntaxNode, SyntaxToken, TextRange,
    TextSize,
};

use crate::parsing::ast;
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

#[derive(salsa::Update, PartialEq, Eq, Clone, Debug)]
pub struct Parse {
    pub node: Cst,
    pub errors: Vec<ParseError>,
}

impl Parse {
    pub fn syntax_node(&self) -> ast::SyntaxNode {
        SyntaxNode::new_root(self.node.clone())
    }

    pub fn file(&self) -> ast::File {
        ast::File::cast(self.syntax_node()).unwrap()
    }
}

pub fn parse(input: &str) -> Parse {
    let (node, errors) = Parser::new(input).parse();
    Parse { node, errors }
}

const STMT_RECOVERY: TokenSet = TokenSet::new(&[T![fn]]);
const PARAM_LIST_RECOVREY: TokenSet =
    TokenSet::new(&[T![->], T!["{"], T![fn]]).union(STMT_RECOVERY);
const STMT_EXPR_RECOVERY: TokenSet =
    TokenSet::new(&[T![let], T!["{"], T!["}"]]).union(STMT_RECOVERY);
const EXPR_FIRST: TokenSet = TokenSet::new(&[
    IDENT,
    T![-],
    T![!],
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
                match this.input.peek() {
                    T!(fn) => this.fn_item(),
                    _ => {
                        this.advance_with_err(ErrorKind::ExpectedItem);
                    }
                };
            }
        });
    }

    fn fn_item(&mut self) {
        self.with(Syntax::FN_ITEM, |this| {
            this.expect(T!(fn));

            if this.input.at(IDENT) {
                this.name();
            } else {
                this.add_error(ErrorKind::ExpectedIdentifier, None);
            }
            if this.input.at(T!["("]) {
                this.param_list();
            } else {
                this.add_error(ErrorKind::ExpectedToken(T!["("]), None);
            }
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

    fn name(&mut self) {
        if self.input.at(Syntax::IDENT) {
            self.with(Syntax::NAME, |this| {
                this.expect(IDENT);
            })
        }
    }

    fn return_type(&mut self) {
        self.with(Syntax::RETURN_TYPE, |this| {
            this.expect(T![->]);
            this.type_expr();
        });
    }

    fn param(&mut self) {
        self.with(Syntax::PARAM, |this| {
            this.name();
            this.expect(T![:]);
            this.type_expr();
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
                    if this.input.at_any(PARAM_LIST_RECOVREY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedParameter);
                }
            }
            this.expect(T!(")"));
        })
    }

    fn type_expr(&mut self) {
        #[allow(clippy::match_single_binding)]
        match self.input.peek() {
            _ => {
                let checkpoint = self.builder.checkpoint();

                let next_span = self.input.nth_span(0);
                self.name();
                match &self.input.content[next_span] {
                    "int" | "float" | "string" | "bool" => {
                        self.with_at(Syntax::LIT_TYPE, checkpoint, |_| {})
                    }
                    "any" => self.with_at(Syntax::ANY_TYPE, checkpoint, |_| {}),
                    _ => {}
                }
                if self.ate(T![?]) {
                    self.with_at(Syntax::NILABLE_TYPE, checkpoint, |_| {});
                }
            }
        }
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
            T![return] => self.ret(),
            _ => return self.expr_primary(),
        }
        Some(())
    }

    fn ret(&mut self) {
        self.with(Syntax::RETURN_EXPR, |this| {
            this.expect(T![return]);
            this.expect_expr();
        })
    }

    fn expr_primary(&mut self) -> Option<()> {
        let token = self.input.peek();
        match token {
            INT | FLOAT | STRING | TRUE_KW | FALSE_KW | NIL_KW => {
                self.builder.start_node(LIT_EXPR.into());
                self.ate(token);
                self.builder.finish_node();
            }
            T!["("] => {
                self.builder.start_node(PAREN_EXPR.into());
                self.expect(T!["("]);
                self.expect_expr();
                self.expect(T![")"]);
                self.builder.finish_node();
            }
            IDENT => {
                self.with(Syntax::NAME_EXPR, |this| this.name());
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
                if this
                    .input
                    .at_any(TokenSet::new(&[Syntax::IDENT]).union(EXPR_FIRST))
                {
                    this.arg();
                } else {
                    break;
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

            let Some((left_bp, right_bp)) = self.infix_binding_power(op) else {
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

    fn infix_binding_power(&self, op: Syntax) -> Option<(u8, u8)> {
        Some(match op {
            T![+] | T![-] => (1, 2),
            T![*] | T![/] => (3, 4),
            _ => return None,
        })
    }

    fn stmt_let(&mut self) {
        self.with(Syntax::LET_STMT, |this| {
            this.expect(T![let]);
            this.name();
            this.expect(T![=]);
            this.expect_expr();
            this.expect(T![;]);
        })
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
    ExpectedItem,
    Other(String),
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

    fn try_parse(source: &str, f: impl FnOnce(&mut Parser)) -> (SyntaxNode, Vec<ParseError>) {
        let mut parser = Parser::new(source);
        f(&mut parser);
        (SyntaxNode::new_root(parser.builder.finish()), parser.errors)
    }

    fn parse(source: &str, f: impl FnOnce(&mut Parser)) -> SyntaxNode {
        let mut parser = Parser::new(source);
        f(&mut parser);
        if !parser.errors.is_empty() {
            panic!("{:?}", parser.errors);
        }
        SyntaxNode::new_root(parser.builder.finish())
    }

    fn children_rec(root: SyntaxNode) -> Vec<String> {
        fn children_recursive(
            child: NodeOrToken<SyntaxNode, SyntaxToken<Lang>>,
            out: &mut Vec<String>,
        ) {
            if child.as_token().is_none() {
                out.push(format!("{:?}: {:?}", child.kind(), child.text_range()));
            }
            if let Some(node) = child.as_node() {
                for child in node.children_with_tokens() {
                    children_recursive(child, out);
                }
            }
        }

        let mut vec = vec![];
        children_recursive(NodeOrToken::Node(root), &mut vec);
        vec
    }

    macro_rules! assert_children_eq {
        ($node:expr, [$($syntax:expr ),* $(,)?] ) => {
            assert_eq!(
                children_rec($node),
                vec![
                    $(
                        $syntax.to_string()
                    ),*
                ]
            );
        };
    }

    #[test]
    #[rustfmt::skip]
    fn func() {
        assert_children_eq!(
            parse("fn test(){}", |p| p.file()),
            [
                "FILE: 0..11",
                    "FN_ITEM: 0..11",
                        "NAME: 3..7",
                        "PARAM_LIST: 7..9",
                        "BLOCK_EXPR: 9..11"
            ]
        );
        assert_children_eq!(
            parse("fn test()->int?{}", |p| p.file()),
            [
                    "FILE: 0..17",
                        "FN_ITEM: 0..17",
                            "NAME: 3..7",
                            "PARAM_LIST: 7..9",
                            "RETURN_TYPE: 9..15",
                                "NILABLE_TYPE: 11..15",
                                    "LIT_TYPE: 11..14",
                                        "NAME: 11..14",
                                "BLOCK_EXPR: 15..17"
                ]
            );
            assert_children_eq!(
                parse("fn test(){
                    let x = (1);
                }", |p| p.file()),
                [
                    "FILE: 0..61",
                        "FN_ITEM: 0..61",
                            "NAME: 3..7",
                            "PARAM_LIST: 7..9",
                            "BLOCK_EXPR: 9..61",
                                "LET_STMT: 31..60",
                                    "NAME: 35..37",
                                    "PAREN_EXPR: 39..42",
                                        "LIT_EXPR: 40..41"
            ]
        );
        assert_children_eq!(
            parse(
                "fn test(){
                    let x = get();
                }",
                |p| p.file()
            ),
            [
                "FILE: 0..63",
                    "FN_ITEM: 0..63",
                        "NAME: 3..7",
                        "PARAM_LIST: 7..9",
                            "BLOCK_EXPR: 9..63",
                            "LET_STMT: 31..62",
                                "NAME: 35..37",
                                "CALL_EXPR: 39..44",
                                    "NAME_EXPR: 39..42",
                                        "NAME: 39..42",
                                "ARG_LIST: 42..44"
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn expr() {
        assert_children_eq!(
            parse("1 + 2", |p| p.expr()),
            ["BINARY_EXPR: 0..5", "LIT_EXPR: 0..2", "LIT_EXPR: 4..5"]
        );
        assert_children_eq!(
            parse("1 + -2 * 3", |p| p.expr()),
            [
                "BINARY_EXPR: 0..10",
                    "LIT_EXPR: 0..2",
                    "BINARY_EXPR: 4..10",
                        "UNARY_EXPR: 4..7",
                            "LIT_EXPR: 5..7",
                        "LIT_EXPR: 9..10"
            ]
        );
        assert_children_eq!(
            parse("a()", |p| p.expr()),
            [
                "CALL_EXPR: 0..3",
                    "NAME_EXPR: 0..1",
                        "NAME: 0..1",
                    "ARG_LIST: 1..3"
            ]
        );
    }
}
