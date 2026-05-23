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

const TYPE_FIRST: TokenSet = TokenSet::new(&[IDENT, T![fn], T!["("]]);
const ITEM_FIRST: TokenSet = TokenSet::new(&[T![fn], T![mod], T![@], T![struct]]);
const PATTERN_FIRST: TokenSet = TokenSet::new(&[IDENT]);
const PATTERN_RECOVERY: TokenSet = TokenSet::new(&[T![=]]).union(PARAM_LIST_RECOVERY);

const PARAM_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![->], T!["{"]]).union(ITEM_FIRST);
const RECORD_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T!["}"], T![,]]);
const FIELD_LIST_RECOVERY: TokenSet = TokenSet::new(&[T!["}"]]).union(ITEM_FIRST);
const CLOSURE_PARAM_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T![|], T!["{"]]);
const STMT_EXPR_RECOVERY: TokenSet = TokenSet::new(&[T![let], T!["{"], T!["}"]]);
const ARG_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T![")"]]);
const COMPILER_ATTRIB_RECOVERY: TokenSet = TokenSet::new(&[T![")"], T![@]]).union(ITEM_FIRST);
const EXPR_FIRST: TokenSet = TokenSet::new(&[
    IDENT,
    INT,
    FLOAT,
    STRING,
    T![true],
    T![false],
    T![not],
    T![-],
    T![lua],
    T![nil],
    T![return],
    T![if],
    T!["{"],
    T!["("],
    T![|],
]);

pub struct Parser<'a> {
    input: Input<'a>,
    builder: GreenNodeBuilder<'static>,
    errors: Vec<ParseError>,
}

//TODO: goto's
pub struct LuaParser<'a, 'b> {
    parser: &'b mut Parser<'a>,
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
        loop {
            let next = self.input.peek();
            if !next.is_whitespace() {
                break;
            }
            let span = self.input.advance();
            self.builder.token(next.into(), &self.input.content[span]);
        }
    }

    fn advance_with_err(&mut self, kind: ErrorKind) {
        let span = self.input.advance();
        self.builder
            .token(ERROR.into(), &self.input.content[span.clone()]);
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

    fn expect_advance(&mut self, token: Syntax) {
        if self.ate(token) {
            return;
        }

        self.advance_with_err(ErrorKind::ExpectedToken(token));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Semicolon;

impl<'a> Parser<'a> {
    fn parse(mut self) -> (Cst, Vec<ParseError>) {
        self.file();
        (self.builder.finish(), self.errors)
    }

    fn file(&mut self) {
        self.with(FILE, |this| {
            this.whitespace();
            while !this.input.at(EOF) {
                this.item();
            }
        });
    }

    fn item(&mut self) {
        self.compiler_attrib_list();
        if self.input.at_any(ITEM_FIRST) {
            match self.input.peek() {
                T![fn] => self.fn_item(),
                T![mod] => self.mod_item(),
                T![struct] => self.struct_item(),
                _ => {
                    unreachable!();
                }
            };
        } else {
            self.advance_with_err(ErrorKind::ExpectedItem);
        }
    }

    fn struct_item(&mut self) {
        self.with(STRUCT_ITEM, |this| {
            this.expect(T![struct]);
            this.name();
            this.field_list();
            this.expect(T!["}"]);
        })
    }

    fn mod_item(&mut self) {
        self.with(MOD_ITEM, |this| {
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
        self.with(FN_ITEM, |this| {
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
        if self.input.at(IDENT) {
            self.with(PATH, |this| {
                this.expect(IDENT);
                while this.at_path_sep(0) && !this.input.at(EOF) {
                    this.expect(T![:]);
                    this.expect(T![:]);
                    this.expect(IDENT);
                }
            });
        }
    }

    fn return_type(&mut self) {
        self.with(RETURN_TYPE, |this| {
            this.expect(T![->]);
            this.type_expr();
        });
    }

    fn field_list(&mut self) {
        self.with(FIELD_LIST, |this| {
            this.expect(T!["{"]);

            while !this.input.at(T!["}"]) && !this.input.at(EOF) {
                this.compiler_attrib_list();
                if this.input.at(IDENT) {
                    this.field();
                    this.ate(T![,]);
                } else {
                    if this.input.at_any(FIELD_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedParameter);
                }
            }
        });
    }

    fn field(&mut self) {
        self.with(FIELD, |this| {
            this.name();
            this.expect(T![:]);
            this.type_expr();
            if this.ate(T![=]) {
                this.expr();
            }
        });
    }

    fn param_list(&mut self) {
        self.with(PARAM_LIST, |this| {
            this.expect(T!["("]);
            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                this.compiler_attrib_list();
                if this.input.at_any(PATTERN_FIRST) {
                    this.param();
                    this.ate(T![,]);
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

    fn param(&mut self) {
        self.with(PARAM, |this| {
            this.pattern();
            this.expect(T![:]);
            this.type_expr();
            if this.ate(T![=]) {
                this.expr();
            }
        })
    }

    fn type_expr(&mut self) {
        if self.input.at_any(TYPE_FIRST) {
            let checkpoint = self.builder.checkpoint();

            #[allow(clippy::match_single_binding)]
            match self.input.peek() {
                T![fn] => {
                    self.fn_type();
                }
                T!["("] => {
                    self.expect(T!["("]);
                    if self.input.at(T![")"]) {
                        self.with_at(UNIT_TYPE, checkpoint, |this| {
                            this.expect(T![")"]);
                        })
                    } else {
                        self.with_at(PAREN_TYPE, checkpoint, |this| {
                            this.type_expr();
                            this.expect(T![")"]);
                        })
                    }
                    if self.ate(T![?]) {
                        self.with_at(NILABLE_TYPE, checkpoint, |_| {});
                    }
                }
                _ => {
                    let next_span = self.input.nth_span(0);
                    let is_path = self.at_path_sep(1);
                    self.path();
                    if !is_path {
                        match &self.input.content[next_span] {
                            "int" | "float" | "string" | "bool" => {
                                self.with_at(LIT_TYPE, checkpoint, |_| {})
                            }
                            "any" => self.with_at(ANY_TYPE, checkpoint, |_| {}),
                            _ => {}
                        }
                    }
                    if self.ate(T![?]) {
                        self.with_at(NILABLE_TYPE, checkpoint, |_| {});
                    }
                }
            }
        } else {
            self.advance_with_err(ErrorKind::ExpectedType);
        }
    }

    fn fn_type(&mut self) {
        self.with(FN_TYPE, |this| {
            this.expect(T![fn]);
            this.fn_type_param_list();
            if this.input.at(T![->]) {
                this.return_type();
            }
        });
    }

    fn fn_type_param_list(&mut self) {
        self.with(PARAM_LIST, |this| {
            this.expect(T!["("]);
            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                if this.input.at_any(TYPE_FIRST) || this.input.at(IDENT) {
                    this.fn_type_param();
                    this.ate(T![,]);
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
        self.with(PARAM, |this| {
            if this.input.nth_skip_whitespace(1) == T![:] {
                this.name();
                this.expect(T![:]);
            }
            this.type_expr();
        })
    }

    fn stmt_expr(&mut self) -> Option<Semicolon> {
        self.with(EXPR_STMT, |this| {
            this.expr();
            this.ate(T![;]).then_some(Semicolon)
        })
    }

    fn prefix_expr(&mut self) -> Option<()> {
        let token = self.input.peek();
        let checkpoint = self.builder.checkpoint();
        match token {
            T![return] => self.return_expr(),
            T![if] => self.if_expr(),
            INT | FLOAT | STRING | TRUE_KW | FALSE_KW | NIL_KW | SINGLE_STRING | BRACKET_STRING => {
                self.with(LIT_EXPR, |this| {
                    this.ate(token);
                });
            }
            T!["("] => {
                let checkpoint = self.builder.checkpoint();
                self.expect(T!["("]);

                if self.input.at(T![")"]) {
                    self.with_at(UNIT_EXPR, checkpoint, |this| {
                        this.expect(T![")"]);
                    })
                } else {
                    self.with_at(PATH_EXPR, checkpoint, |this| {
                        this.expr();
                        this.expect(T![")"]);
                    });
                }
            }
            T!["{"] => {
                self.block();
            }
            T![lua] => {
                self.lua_block();
            }
            T![|] => {
                self.closure_expr();
            }
            IDENT => {
                let checkpoint = self.builder.checkpoint();
                self.path();

                if self.input.at(T!["{"])
                    && ((self.input.nth_skip_whitespace(1) == IDENT && self.input.nth_skip_whitespace(2) == T![:])
                        || self.input.nth_skip_whitespace(1) == T!["}"])
                {
                    self.with_at(RECORD_EXPR, checkpoint, |this| this.record_field_list());
                } else {
                    self.with_at(PATH_EXPR, checkpoint, |_| {});
                }
            }
            _ => {
                self.advance_with_err(ErrorKind::ExpectedExpression);
                return None;
            }
        }
        loop {
            match self.input.peek() {
                T!["("] => {
                    self.with_at(CALL_EXPR, checkpoint, |this| this.arg_list());
                }
                T!["["] => {
                    self.with_at(INDEX_EXPR, checkpoint, |this| this.index());
                }
                T![.] => {
                    if self.input.nth(2) == T!["("] {
                        self.with_at(METHOD_EXPR, checkpoint, |this| {
                            this.expect(T![.]);
                            this.name();
                            this.arg_list();
                        })
                    } else {
                        self.with_at(FIELD_EXPR, checkpoint, |this| {
                            this.expect(T![.]);
                            this.name();
                        })
                    }
                }
                _ => break,
            }
        }
        Some(())
    }

    fn record_field_list(&mut self) {
        self.with(RECORD_FIELD_LIST, |this| {
            this.expect(T!["{"]);
            while !this.input.at(T!["}"]) && !this.input.at(EOF) {
                if this.input.at(IDENT) {
                    this.record_field();
                    this.ate(T![,]);
                } else {
                    if this.input.at_any(RECORD_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedField);
                }
            }
            this.expect(T!["}"]);
        });
    }

    fn record_field(&mut self) {
        self.with(RECORD_FIELD, |this| {
            this.name();
            this.expect(T![:]);
            this.expr();
        });
    }

    fn closure_expr(&mut self) {
        self.with(CLOSURE_EXPR, |this| {
            this.closure_param_list();
            if this.input.at(T![->]) {
                this.return_type();
            }
            this.expr();
        })
    }

    fn closure_param_list(&mut self) {
        self.with(CLOSURE_PARAM_LIST, |this| {
            this.expect(T![|]);
            while !this.input.at(T![|]) && !this.input.at(EOF) {
                if this.input.at_any(PATTERN_FIRST) {
                    this.closure_param();
                    this.ate(T![,]);
                } else {
                    if this.input.at_any(CLOSURE_PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedParameter);
                }
            }
            this.expect(T![|]);
        });
    }
    fn closure_param(&mut self) {
        self.with(CLOSURE_PARAM, |this| {
            this.pattern();
            if this.ate(T![:]) {
                this.type_expr();
            }
        });
    }

    fn if_expr(&mut self) {
        self.with(IF_EXPR, |this| {
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
        self.with(RETURN_EXPR, |this| {
            this.expect(T![return]);
            if this.input.at_any(EXPR_FIRST) {
                this.expr();
            }
        })
    }

    fn arg_list(&mut self) {
        self.with(ARG_LIST, |this| {
            this.expect(T!["("]);
            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                if this.input.at_any(EXPR_FIRST) {
                    this.arg();
                    this.ate(T![,]);
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
        self.expr();
        self.expect(T!["]"]);
    }

    fn arg(&mut self) {
        self.with(ARG, |this| {
            let has_label = this.input.nth_skip_whitespace(1) == T![:];
            if has_label {
                this.name();
                this.expect(T![:]);
            }
            if this.input.at_any(EXPR_FIRST) || !has_label {
                this.expr();
            }
        });
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let checkpoint = self.builder.checkpoint();

        match self.input.peek().prefix_bp() {
            Some(rbp) => {
                self.with(UNARY_EXPR, |this| {
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
            let op = self.input.peek();

            if let Some(postfix_bp) = op.postfix_bp() {
                if postfix_bp < min_bp {
                    break;
                }
                match op {
                    T![?] => {
                        self.with_at(TRY_EXPR, checkpoint, |this| {
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
        self.with(LET_STMT, |this| {
            this.expect(T![let]);
            this.pattern();
            if this.ate(T![:]) {
                this.type_expr();
            }
            this.expect(T![=]);
            this.expr();
            this.expect(T![;]);
        })
    }

    fn pattern(&mut self) {
        match self.input.peek() {
            IDENT => {
                self.with(NAME_PATTERN, |this| {
                    this.name();
                });
            }
            _ => {
                self.add_error(ErrorKind::ExpectedPattern, None);
                return;
            }
        }
    }

    fn lua_block(&mut self) {
        LuaParser::new(self).chunk();
    }

    fn block(&mut self) {
        self.with(BLOCK_EXPR, |this| {
            this.expect(T!["{"]);
            while !this.input.at(T!["}"]) && !this.input.at(EOF) {
                this.compiler_attrib_list();
                match this.input.peek() {
                    T![let] => this.stmt_let(),
                    T![;] => {
                        this.expect(T![;]);
                    }
                    _ => {
                        if this.input.at_any(EXPR_FIRST) {
                            if this.stmt_expr().is_none() && !this.input.at(T!["}"]) {
                                this.expect(T![;]);
                            }
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

    fn compiler_attrib_list(&mut self) {
        if !self.input.at(T![@]) {
            return;
        }
        self.with(COMPILER_ATTRIB_LIST, |this| {
            while this.input.at(T![@]) {
                this.compiler_attrib();
            }
        });
    }

    fn compiler_attrib(&mut self) {
        self.with(COMPILER_ATTRIB, |this| {
            this.expect(T![@]);
            this.name();

            if !this.ate(T!["("]) {
                return;
            }

            while !this.input.at(T![")"]) && !this.input.at(EOF) {
                if this.input.at_any(EXPR_FIRST) {
                    this.compiler_attrib_item();
                    this.ate(T![,]);
                } else {
                    if this.input.at_any(COMPILER_ATTRIB_RECOVERY) {
                        break;
                    }
                    this.advance_with_err(ErrorKind::ExpectedAttribute);
                }
            }
            this.expect(T![")"]);
        });
    }

    fn compiler_attrib_item(&mut self) {
        self.with(COMPILER_ATTRIB_ITEM, |this| {
            this.expr();
            if this.ate(T![=]) {
                this.expr();
            }
        });
    }

    fn name(&mut self) {
        self.with(NAME, |this| {
            this.expect(IDENT);
        })
    }

    fn at_path_sep(&self, offset: usize) -> bool {
        self.input.nth(offset) == self.input.nth(1 + offset) && self.input.nth(offset) == T![:]
    }
}

impl<'a, 'b> LuaParser<'a, 'b> {
    fn new(parser: &'b mut Parser<'a>) -> Self {
        Self { parser }
    }

    fn with<R>(&mut self, syntax: Syntax, body: impl FnOnce(&mut Self) -> R) -> R {
        self.parser.builder.start_node(syntax.into());
        let res = body(self);
        self.parser.builder.finish_node();
        res
    }

    fn with_at<R>(
        &mut self,
        syntax: Syntax,
        checkpoint: Checkpoint,
        body: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.parser.builder.start_node_at(checkpoint, syntax.into());
        let res = body(self);
        self.parser.builder.finish_node();
        res
    }

    fn chunk(&mut self) {
        self.with(LUA_BLOCK_EXPR, |this| {
            this.parser.expect(T![lua]);
            this.parser.expect(T!["{"]);

            while !this.parser.input.at(T!["}"]) && !this.parser.input.at(EOF) {
                if this.stmt().is_none() {
                    break;
                }
            }
            this.parser.expect(T!["}"]);
        });
    }

    fn expect_advance_ident(&mut self, ident: &str) {
        let next_span = self.parser.input.nth_span(0);
        if !self.parser.input.at(IDENT) {
            self.parser
                .advance_with_err(ErrorKind::Other(format!("expected {}", ident)));
            return;
        }

        self.parser.expect(IDENT);
        if &self.parser.input.content[next_span] != ident {
            self.parser
                .advance_with_err(ErrorKind::Other(format!("expected {}", ident)));
        }
    }

    fn expect_ident(&mut self, ident: &str) {
        let next_span = self.parser.input.nth_span(0);
        self.parser.expect(IDENT);

        if &self.parser.input.content[next_span] != ident {
            self.parser
                .add_error(ErrorKind::Other(format!("expected {}", ident)), None);
        }
    }

    fn stmt(&mut self) -> Option<Semicolon> {
        match self.parser.input.peek() {
            T![return] => {
                self.return_stmt();
                Some(Semicolon)
            }
            T![break] => {
                self.break_stmt();
                Some(Semicolon)
            }
            T![while] => {
                self.while_stmt();
                Some(Semicolon)
            }
            T![if] => {
                self.if_stmt();
                Some(Semicolon)
            }
            T![for] => {
                self.for_stmt();
                Some(Semicolon)
            }
            T![;] => {
                self.parser.expect(T![;]);
                Some(Semicolon)
            }
            IDENT => {
                let Some(next) = self.parser.input.nth_token_text_skip_whitespace(0) else {
                    //TODO: test this branch
                    self.parser.advance_with_err(ErrorKind::ExpectedExpression);
                    return Some(Semicolon);
                };

                match next {
                    "do" => self.with(LUA_BLOCK_STMT, |this| {
                        this.expect_ident("do");
                        this.block(|t| t == "end");
                        this.expect_ident("end");
                        Some(Semicolon)
                    }),
                    "repeat" => {
                        self.repeat_stmt();
                        Some(Semicolon)
                    }
                    "local" => {
                        self.local_stmt();
                        Some(Semicolon)
                    }
                    "function" => {
                        self.function_stmt();
                        Some(Semicolon)
                    }
                    _ => self.stmt_expr(),
                }
            }
            _ => self.stmt_expr(),
        }
    }

    fn break_stmt(&mut self) {
        self.with(LUA_BREAK_EXPR, |this| {
            this.parser.expect(T![break]);
            this.parser.expect(T![;]);
        });
    }

    fn if_stmt(&mut self) {
        self.with(LUA_IF_STMT, |this| {
            this.parser.expect(T![if]);
            this.expr();
            this.expect_ident("then");
            loop {
                this.block(|t| t == "else" || t == "elseif" || t == "end");
                match this.parser.input.peek_text() {
                    "elseif" => {
                        this.expect_ident("elseif");
                        this.expr();
                        this.expect_ident("then");
                    }
                    "else" => {
                        this.parser.expect_advance(T![else]);
                    }
                    "end" => {
                        this.expect_advance_ident("end");
                        break;
                    }
                    _ => break,
                }
            }
        })
    }
    fn for_stmt(&mut self) {
        self.with(LUA_FOR_STMT, |this| {
            this.parser.expect(T![for]);
            if this.parser.input.nth_skip_whitespace(1) == T![=] {
                this.name();
                this.parser.expect(T![=]);

                this.expr();
                if this.parser.ate(T![,]) {
                    this.expr();
                }
                if this.parser.ate(T![,]) {
                    this.expr();
                }

                this.expect_ident("do");
                this.block(|t| t == "end");
                this.expect_ident("end");
            } else {
                this.name();
                while this.parser.ate(T![,]) && !this.parser.input.at(EOF) {
                    this.name();
                }
                this.parser.expect(T![in]);
                this.expr();
                this.expect_ident("do");
                this.block(|t| t == "end");
                this.expect_ident("end");
            }
        });
    }

    fn while_stmt(&mut self) {
        self.with(LUA_WHILE_STMT, |this| {
            this.parser.expect(T![while]);
            this.expr();
            this.expect_ident("do");
            this.block(|t| t == "end");
            this.expect_ident("end");
        });
    }

    fn repeat_stmt(&mut self) {
        self.with(LUA_REPEAT_STMT, |this| {
            this.expect_ident("repeat");
            this.block(|t| t == "until");
            this.expect_ident("until");
            this.expr();
        });
    }

    fn local_stmt(&mut self) {
        self.with(LUA_LOCAL_STMT, |this| {
            this.expect_ident("local");
            this.name();
            while this.parser.ate(T![,]) && !this.parser.input.at(EOF) {
                this.name();
            }
            this.parser.expect(T![=]);
            this.expr_multi();
            this.parser.expect(T![;]);
        });
    }

    fn function_stmt(&mut self) {
        self.with(LUA_FUNCTION_STMT, |this| {
            this.expect_ident("function");
            this.name();

            if let token @ (T![.] | T![:]) = this.parser.input.peek() {
                this.parser.expect(token);
                this.name();
            }
            this.param_list();
            this.with(LUA_BLOCK_STMT, |this| {
                this.block(|t| t == "end");
                this.expect_ident("end");
            });
            this.parser.ate(T![;]);
        });
    }

    //TODO: implement recovery
    fn param_list(&mut self) {
        self.with(LUA_PARAM_LIST, |this| {
            this.parser.expect(T!["("]);

            if !this.parser.input.at(T![")"]) {
                this.param();
                while this.parser.ate(T![,]) && !this.parser.input.at(EOF) {
                    this.param();
                }
            }
            this.parser.expect(T![")"]);
        });
    }

    fn param(&mut self) {
        self.with(LUA_PARAM, |this| {
            this.parser.expect(IDENT);
        })
    }

    fn return_stmt(&mut self) {
        self.with(LUA_RETURN_STMT, |this| {
            this.parser.expect(T![return]);
            this.expr_multi();
            this.parser.expect(T![;]);
        })
    }

    fn stmt_expr(&mut self) -> Option<Semicolon> {
        self.with(LUA_STMT_EXPR, |this| {
            this.expr_multi();
            if this.parser.ate(T![=]) {
                this.expr_multi();
            }
            this.parser.ate(T![;]).then_some(Semicolon)
        })
    }

    fn expr_multi(&mut self) {
        self.with(LUA_MULTI_EXPR, |this| {
            this.expr();
            if this.parser.ate(T![,]) {
                this.expr();
                while this.parser.ate(T![,]) && !this.parser.input.at(EOF) {
                    this.expr();
                }
            }
        });
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let checkpoint = self.parser.builder.checkpoint();
        match Self::prefix_bp(self.parser.input.peek()) {
            Some(rbp) => self.with(LUA_UNARY_EXPR, |this| {
                this.parser.expect(this.parser.input.peek());
                this.expr_bp(rbp);
            }),
            None => {
                if self.prefix_expr().is_none() {
                    return;
                }
            }
        }

        loop {
            let op = self.parser.input.peek();

            let Some((left_bp, rigth_bp)) = Self::infix_bp(op) else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.parser.expect(op);

            if self.at_expr() {
                self.with_at(LUA_BINARY_EXPR, checkpoint, |this| {
                    this.expr_bp(rigth_bp);
                })
            } else {
                self.parser.advance_with_err(ErrorKind::ExpectedExpression);
            }
        }
    }

    fn at_expr(&self) -> bool {
        let token = self.parser.input.peek();
        matches!(
            token,
            T![nil]
                | T![#]
                | T![true]
                | T![false]
                | T![...]
                | T![-]
                | T![not]
                | T!["{"]
                | T!["("]
                | FLOAT
                | INT
                | STRING
                | SINGLE_STRING
                | BRACKET_STRING
                | IDENT
        )
    }

    fn index(&mut self) {
        self.parser.expect(T!["["]);
        self.expr();
        self.parser.expect(T!["]"]);
    }

    fn arg_list(&mut self) {
        self.with(LUA_ARG_LIST, |this| {
            this.parser.expect(T!["("]);
            if !this.parser.input.at(T![")"]) {
                this.arg();
                while this.parser.ate(T![,]) && !this.parser.input.at(EOF) {
                    this.arg();
                }
            }
            this.parser.expect(T![")"]);
        })
    }

    fn arg(&mut self) {
        self.with(LUA_ARG, |this| {
            this.expr();
        });
    }

    fn prefix_expr(&mut self) -> Option<()> {
        let token = self.parser.input.peek();
        let checkpoint = self.parser.builder.checkpoint();
        match token {
            IDENT
            | INT
            | FLOAT
            | STRING
            | SINGLE_STRING
            | BRACKET_STRING
            | TRUE_KW
            | FALSE_KW
            | NIL_KW
            | T![...] => {
                if self.parser.input.peek_text() == "function" {
                    self.with(LUA_FUNCTION_EXPR, |this| {
                        this.expect_ident("function");
                        this.param_list();
                        this.with(LUA_BLOCK_STMT, |this| {
                            this.block(|t| t == "end");
                            this.expect_ident("end");
                        });
                    });
                } else {
                    self.parser.with(LUA_LIT_EXPR, |this| {
                        this.ate(token);
                    });
                }
            }
            T!["{"] => {
                self.with(LUA_TABLE_EXPR, |this| {
                    this.parser.expect(T!["{"]);
                    while !this.parser.input.at(T!["}"]) && !this.parser.input.at(EOF) {
                        if this.parser.input.nth_skip_whitespace(1) == T![=] {
                            this.name();
                            this.parser.expect(T![=]);
                            this.expr_bp(2);
                        } else if this.parser.input.at(T!["["]) {
                            this.parser.expect(T!["["]);
                            this.expr();
                            this.parser.expect(T!["]"]);
                            this.parser.expect(T![=]);
                            this.expr_bp(2);
                        } else {
                            this.expr_bp(2);
                        }
                        this.parser.ate(T![,]);
                    }
                    this.parser.expect(T!["}"]);
                });
            }
            _ => {
                self.parser.advance_with_err(ErrorKind::ExpectedExpression);
                return None;
            }
        };
        loop {
            match self.parser.input.peek() {
                T!["("] => {
                    self.with_at(LUA_FIELD_CALL_EXPR, checkpoint, |this| this.arg_list());
                }
                T!["["] => {
                    self.with_at(LUA_FIELD_INDEX_EXPR, checkpoint, |this| this.index());
                }
                _ => break,
            }
        }
        while matches!(self.parser.input.peek(), T![.] | T![:]) {
            self.with_at(LUA_FIELD_ACCESS_EXPR, checkpoint, |this| {
                this.parser.expect(this.parser.input.peek());
                this.name();

                loop {
                    match this.parser.input.peek() {
                        T!["("] => {
                            this.with_at(LUA_CALL_EXPR, checkpoint, |this| this.arg_list());
                        }
                        T!["["] => {
                            this.with_at(LUA_INDEX_EXPR, checkpoint, |this| this.index());
                        }
                        _ => break,
                    }
                }
            });
        }
        Some(())
    }

    fn prefix_bp(syntax: Syntax) -> Option<u8> {
        Some(match syntax {
            T![not] => 13,
            T![-] => 14,
            _ => return None,
        })
    }

    fn infix_bp(syntax: Syntax) -> Option<(u8, u8)> {
        Some(match syntax {
            T![or] => (1, 2),
            T![and] => (3, 4),
            T![<] | T![>] | T![<=] | T![>=] | T![==] | T![~=] => (5, 6),
            T![..] => (8, 7),
            T![+] | T![-] => (9, 10),
            T![*] | T![/] => (11, 12),
            _ => return None,
        })
    }

    fn name(&mut self) {
        self.with(LUA_NAME, |this| {
            this.parser.expect(IDENT);
        })
    }

    fn block(&mut self, stop_condition: impl Fn(&str) -> bool) {
        while self
            .parser
            .input
            .nth_token_text(0)
            .map(|t| !stop_condition(t))
            .unwrap_or(false)
            && !self.parser.input.at(EOF)
        {
            if self.stmt().is_none() {
                break;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    ExpectedToken(Syntax),
    ExpectedTokens(Vec<Syntax>),
    ExpectedExpression,
    ExpectedOperator,
    ExpectedArgument,
    ExpectedStatement,
    ExpectedType,
    ExpectedParameter,
    ExpectedField,
    ExpectedAttribute,
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
            Self::ExpectedAttribute => "expected attribute",
            Self::ExpectedExpression => "expected expression",
            Self::ExpectedParameter => "expected parameter",
            Self::ExpectedPattern => "expected pattern",
            Self::ExpectedItem => "expected item",
            Self::ExpectedOperator => "expected operator",
            Self::ExpectedField => "expected field",
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

    fn nth_span_skip_whitespace(&self, amount: usize) -> Range<usize> {
        self.lexer
            .clone()
            .filter(|(t, _)| t.map(|t| !t.is_whitespace()).unwrap_or(true))
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
            .map(|(token, _)| match token {
                Ok(token) => token,
                Err(_) => ERROR,
            })
            .nth(amount)
            .unwrap_or(EOF)
    }

    fn nth_skip_whitespace(&self, amount: usize) -> Syntax {
        if self.fuel == 0 {
            panic!("parser got stuck")
        }
        self.lexer
            .clone()
            .map(|(token, _)| match token {
                Ok(token) => token,
                Err(_) => ERROR,
            })
            .filter(|t| !t.is_whitespace())
            .nth(amount)
            .unwrap_or(EOF)
    }

    fn eat(&mut self, token: Syntax) -> Option<&str> {
        if self.at(token) {
            Some(&self.content[self.advance()])
        } else {
            None
        }
    }

    fn nth_token_text_skip_whitespace(&self, offset: usize) -> Option<&str> {
        if self.nth_skip_whitespace(offset) == EOF {
            return None;
        }
        let next_span = self.nth_span_skip_whitespace(offset);
        Some(&self.content[next_span])
    }

    fn nth_token_text(&self, offset: usize) -> Option<&str> {
        if self.nth(offset) == EOF {
            return None;
        }
        let next_span = self.nth_span_skip_whitespace(offset);
        Some(&self.content[next_span])
    }

    fn peek(&self) -> Syntax {
        self.nth(0)
    }

    fn peek_text(&self) -> &str {
        let span = self.nth_span(0);
        &self.content[span]
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
    use rowan::{GreenNode, NodeOrToken, SyntaxKind, SyntaxToken};

    use crate::parsing::{
        ast::SyntaxNode,
        parser::{Lang, LuaParser, Parser},
    };

    fn parse_rec(
        child: NodeOrToken<SyntaxNode, SyntaxToken<Lang>>,
        result: &mut String,
        depth: usize,
    ) {
        (0..depth).for_each(|_| result.push_str("  "));
        result.push_str(&format!(
            "{:?}: {}..{}{}",
            child.kind(),
            u32::from(child.text_range().start()),
            u32::from(child.text_range().end()),
            match &child {
                NodeOrToken::Token(t) if t.kind() != super::WHITESPACE =>
                    format!(" \"{}\"", t.text()),
                _ => String::from(" "),
            }
        ));
        result.push('\n');
        if let NodeOrToken::Node(node) = child {
            for child in node.children_with_tokens() {
                parse_rec(child, result, depth + 1);
            }
        }
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

        parse_rec(NodeOrToken::Node(node), &mut result, 0);
        result
    }

    #[track_caller]
    fn lua_parse(source: &str, f: impl FnOnce(LuaParser)) -> String {
        let mut parser = Parser::new(source);
        f(LuaParser::new(&mut parser));
        if !parser.errors.is_empty() {
            panic!("{:?}", parser.errors);
        }
        let node = SyntaxNode::new_root(parser.builder.finish());
        let mut result = String::new();

        parse_rec(NodeOrToken::Node(node), &mut result, 0);
        result
    }

    #[track_caller]
    fn try_parse(source: &str, f: impl FnOnce(&mut Parser)) -> (String, Vec<super::ParseError>) {
        let mut parser = Parser::new(source);
        f(&mut parser);
        let node = SyntaxNode::new_root(parser.builder.finish());
        let mut result = String::new();

        parse_rec(NodeOrToken::Node(node), &mut result, 0);
        (result, parser.errors)
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
        insta::assert_snapshot!(parse("(a: int, b : string)", |p| p.param_list()));
    }

    #[test]
    fn type_expr() {
        insta::assert_snapshot!(parse("int", |p| p.type_expr()));
        insta::assert_snapshot!(parse("NotInt", |p| p.type_expr()));
        insta::assert_snapshot!(parse("fn(a : int, string) -> Result", |p| p.type_expr()));
    }

    #[test]
    fn stmt_expr() {
        insta::assert_snapshot!(parse("a;", |p| {
            p.stmt_expr();
        }));
        insta::assert_snapshot!(parse("1+1;", |p| { _ = p.stmt_expr() }));
        insta::assert_snapshot!(parse("print();", |p| { _ = p.stmt_expr() }));
        insta::assert_snapshot!(parse("no_semi % idk", |p| { _ = p.stmt_expr() }));
        insta::assert_snapshot!(parse("vec2 { }", |p| { _ = p.stmt_expr() }));
    }

    #[test]
    fn stmt_let() {
        insta::assert_snapshot!(parse("let x = 1;", |p| p.stmt_let()));
        insta::assert_snapshot!(parse("let y = 1;", |p| p.stmt_let()));
    }

    #[test]
    fn block() {
        insta::assert_snapshot!(parse("{ }", |p| p.block()));
        insta::assert_snapshot!(parse("{ 1 }", |p| p.block()));
        insta::assert_snapshot!(parse("{ something; something_else; }", |p| p.block()));
        insta::assert_snapshot!(parse("{ vec2 { a : 1 }; }", |p| p.block()));
    }

    #[test]
    fn expr() {
        insta::assert_snapshot!(parse("1", |p| p.expr()));
        insta::assert_snapshot!(parse("1+1", |p| p.expr()));
        insta::assert_snapshot!(parse("1+1*3/4%3", |p| p.expr()));
        insta::assert_snapshot!(parse("1=2 or 3 and 4 == 5 != 6 < 7 + 8 * not -9", |p| p.expr()));
        insta::assert_snapshot!(parse("(1)", |p| p.expr()));
        insta::assert_snapshot!(parse("1 + { 1 }", |p| p.expr()));
        insta::assert_snapshot!(parse("if not true {} else {}", |p| p.expr()));
        insta::assert_snapshot!(parse("if true {} else if VALUE { yo_mister_white }", |p| p.expr()));
        insta::assert_snapshot!(parse("\"a string\"", |p| p.expr()));
        insta::assert_snapshot!(parse("a[1](2)[3]", |p| p.expr()));
        insta::assert_snapshot!(parse("a[1] = b = c", |p| p.expr()));
        insta::assert_snapshot!(parse("sort(array, by: callback, something_else:)", |p| p.expr()));
        insta::assert_snapshot!(parse("|x,y: int| lua {x+y}", |p| p.expr()));
        insta::assert_snapshot!(parse("()", |p| p.expr()));
        insta::assert_snapshot!(parse("1.abs()", |p| p.expr()));
        insta::assert_snapshot!(parse("pos[1][2].test().test.len()[0]", |p| p.expr()));
        insta::assert_snapshot!(parse("math::Vec2 { x: 1, y: 2, }", |p| p.expr()));
        insta::assert_snapshot!(parse("Vec2 {}", |p| p.expr()));
    }

    #[test]
    fn strct() {
        insta::assert_snapshot!(parse("struct Vec2 { x: int = 1, y: int = 2 }", |p| p
            .struct_item()));
    }

    #[test]
    fn attribs() {
        insta::assert_snapshot!(parse("@first @second() @third(a=3,b=4, yeah)", |p| p
            .compiler_attrib_list()));
    }

    #[test]
    fn numbers() {
        insta::assert_snapshot!(parse("10.10", |p| p.expr()));
        insta::assert_snapshot!(parse("1_000_000", |p| p.expr()));
    }

    #[test]
    fn lua() {
        insta::assert_snapshot!(parse("lua {}", |p| p.lua_block()));
        insta::assert_snapshot!(parse(
            "lua { x = { [1] = 1, 2, 3, a=1,b=3, [3]=1 }; }",
            |p| p.lua_block()
        ));
        insta::assert_snapshot!(parse(
            "lua { function func(a,b,c) a = 1; b = 2; end }",
            |p| p.lua_block()
        ));
        insta::assert_snapshot!(parse(
            "lua { function test.func(a,b,c) a = 1; b = 2; end }",
            |p| p.lua_block()
        ));
        insta::assert_snapshot!(parse(
            "lua { function test:func(a,b,c) a = 1; b = 2; end }",
            |p| p.lua_block()
        ));
        insta::assert_snapshot!(parse("lua { call(1)[1.5](2)[2.5](3); }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { local string = [[ hello there ]]; }", |p| p
            .lua_block()));
        insta::assert_snapshot!(parse("lua { x = vec2.x.y; }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { x.y = 1; }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { print('1'); }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { x.y()().x.x().y[1][2](),x = 1,2; }", |p| p
            .lua_block()));
        insta::assert_snapshot!(parse("lua { return 1,2,3; }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { break; }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { local a,b,c = 1, 'string', nil; }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { a,b = 1,2; }", |p| p.lua_block()));
        insta::assert_snapshot!(parse("lua { while true do print(); break; end  }", |p| p
            .lua_block()));
        insta::assert_snapshot!(parse("lua { print(...); }", |p| {
            p.lua_block();
        }));
        insta::assert_snapshot!(parse("lua { repeat print() until false }", |p| {
            p.lua_block();
        }));
        insta::assert_snapshot!(parse(
            "lua {
                if true then
                    yan()
                elseif false then
                    yay()
                else 
                    nay()
                end
            }",
            |p| {
                p.lua_block();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                for i=1,2,3 do end
            }",
            |p| {
                p.lua_block();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                for i = 1,10 do end
            }",
            |p| {
                p.lua_block();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                for a,b,c,d in {} do end
            }",
            |p| {
                p.lua_block();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                local x = function() end;
            }",
            |p| {
                p.lua_block();
            }
        ));
        println!("------------------------------");
        insta::assert_snapshot!(parse("lua{1+2}", |p| p.lua_block()));
    }
}
