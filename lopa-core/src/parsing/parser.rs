use std::{cell::Cell, iter::Peekable, ops::Range};

use itertools::Itertools as _;

use super::lexer;
use crate::parsing::{
    lexer::{
        LexToken, Lexer,
        Syntax::{self, *},
    },
    token_set::TokenSet,
};

pub const EXPR_FIRST: TokenSet = TokenSet::new(&[
    IDENT,
    INT,
    FLOAT,
    STRING,
    T![true],
    T![false],
    T![-],
    T![lua],
    T![nil],
    T![return],
    T![if],
    T!["{"],
    T!["("],
    T![|],
    T![self],
    T![!],
]);
const TYPE_FIRST: TokenSet = TokenSet::new(&[
    IDENT,
    T![fn],
    T!["("],
    T![Self],
    T![dyn],
    T![root],
    T![super],
]);
const PATTERN_FIRST: TokenSet = TokenSet::new(&[IDENT, INT, FLOAT, STRING, T!["("], T!["["]]);
const ITEM_TYPE_FIRST: TokenSet = TokenSet::new(&[T![struct], T![enum]]).union(TYPE_FIRST);
const ITEM_FIRST: TokenSet =
    TokenSet::new(&[T![fn], T![mod], T![struct], T![impl], T![use], T![enum]]);
const ELEMENT_FIRST: TokenSet = TokenSet::new(&[T![fn], IDENT]);
const USE_FIRST: TokenSet = TokenSet::new(&[T!["{"], IDENT, T![*], T![self], T![root], T![super]]);
const PATH_FIRST: TokenSet = TokenSet::new(&[T![root], T![super], IDENT]);

const GENERIC_ANCHOR: TokenSet = TokenSet::new(&[T![;], T!["{"], T!["("], T![")"], T![:], T![?]]);

const USE_RECOVERY: TokenSet = TokenSet::new(&[]).union(ITEM_FIRST);
// const PATTERN_RECOVERY: TokenSet = TokenSet::new(&[T![=], T![=>], T!["{"]]);
const FN_TYPE_PARAM_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![->], T![")"], IDENT]);
const PARAM_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![->], T!["{"], T![;]]).union(ITEM_FIRST);
const RECORD_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T!["}"], T![,]]);
const ELEMENT_RECOVERY: TokenSet = TokenSet::new(&[T!["}"]]).union(ITEM_FIRST);
const CLOSURE_PARAM_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T![|], T!["{"]]);
const STMT_EXPR_RECOVERY: TokenSet = TokenSet::new(&[T![let], T!["{"], T!["}"]]).union(ITEM_FIRST);
const ARG_LIST_RECOVERY: TokenSet = TokenSet::new(&[T![let], T![")"]]);
const COMPILER_ATTRIB_RECOVERY: TokenSet = TokenSet::new(&[T![")"], T![@]]).union(ITEM_FIRST);
const GENERICS_RECOVERY: TokenSet = TokenSet::new(&[T!["{"], T![>]]).union(ITEM_FIRST);
const TUPLE_RECOVERY: TokenSet = TokenSet::new(&[T![")"]]);

pub fn parse(input: &str) -> (Tree, Vec<ParseError>) {
    let mut p = Parser::new(input);
    p.module();
    p.build_tree()
}

#[derive(Debug)]
enum Event {
    Open { node: Syntax },
    Checkpoint,
    Close,
    Advance { token: Syntax },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxErrorKind {
    ExpectedToken(Syntax),
    ExpectedExpression,
    ExpectedOperator,
    ExpectedArgument,
    ExpectedStatement,
    ExpectedGeneric,
    ExpectedType,
    ExpectedParameter,
    ExpectedParent,
    ExpectedStructElement,
    ExpectedEnumElement,
    ExpectedImplElement,
    ExpectedField,
    ExpectedPathSegment,
    ExpectedPath,
    ExpectedAttribute,
    ExpectedPattern,
    ExpectedItem,
    ExpectedUseDeclaration,
    Other(String),
}

impl std::fmt::Display for SyntaxErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExpectedToken(tok) => return write!(f, "expected {}", tok),
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
            Self::ExpectedParent => "expected parent",
            Self::ExpectedStructElement => "expected struct element",
            Self::ExpectedEnumElement => "expected enum element",
            Self::ExpectedImplElement => "expected impl element",
            Self::ExpectedGeneric => "expected generic",
            Self::ExpectedPathSegment => "expected path segment",
            Self::ExpectedPath => "expected path",
            Self::ExpectedUseDeclaration => "expected use declaration",
            Self::Other(text) => text,
        }
        .fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError {
    pub range: Range<usize>,
    pub kind: SyntaxErrorKind,
}

impl ParseError {
    pub fn new(range: Range<usize>, kin: SyntaxErrorKind) -> Self {
        Self { range, kind: kin }
    }
}

struct SavePoint {
    pos: usize,
    event_id: usize,
    error_id: usize,
}

struct Parser<'a> {
    tokens: Vec<LexToken<'a>>,
    pos: usize,
    fuel: Cell<u32>,
    events: Vec<Event>,
    errors: Vec<ParseError>,
    input: &'a str,
    tokens_raw: Peekable<Lexer<'a>>,
    save_point: Option<SavePoint>,
    save_point_errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let tokens_raw = Lexer::new(input);
        let tokens = tokens_raw
            .clone()
            .filter(|t| !t.token.is_whitespace())
            .collect_vec();
        Self {
            tokens,
            pos: 0,
            fuel: 1024.into(),
            events: Default::default(),
            errors: Default::default(),
            input,
            tokens_raw: tokens_raw.peekable(),
            save_point: None,
            save_point_errors: Default::default(),
        }
    }

    fn build_tree(self) -> (Tree, Vec<ParseError>) {
        Builder {
            builder: syntree::Builder::new(),
            tokens_raw: self.tokens_raw,
            errors: self.errors,
        }
        .build_tree(&self.events)
    }

    fn set_save_point(&mut self) {
        self.save_point = Some(SavePoint {
            pos: self.pos,
            event_id: self.events.len(),
            error_id: self.errors.len(),
        });
    }

    fn restore_save_point(&mut self, f: impl FnOnce(&mut Self)) {
        let save_point = self.save_point.take().unwrap();

        self.errors
            .drain((save_point.error_id)..(self.errors.len()))
            .for_each(|err| {
                self.save_point_errors.push(err);
            });

        f(self);

        self.pos = save_point.pos;
        self.events
            .drain((save_point.event_id)..(self.events.len()));
        self.save_point_errors.clear();
    }

    fn with_save_point(&mut self, body: impl FnOnce(&mut Self), restore: impl FnOnce(&mut Self)) {
        self.set_save_point();
        body(self);
        self.restore_save_point(restore);
    }

    fn with<R>(&mut self, syntax: Syntax, body: impl FnOnce(&mut Self) -> R) -> R {
        self.start_node(syntax);
        let res = body(self);
        self.finish_node();
        res
    }

    fn with_at<R>(
        &mut self,
        syntax: Syntax,
        checkpoint: usize,
        body: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.start_node_at(checkpoint, syntax);
        let res = body(self);
        self.finish_node();
        res
    }

    fn start_node_at(&mut self, checkpoint: usize, node: Syntax) {
        self.events.insert(checkpoint, Event::Open { node });
    }

    fn start_node(&mut self, node: Syntax) {
        self.events.push(Event::Open { node });
    }

    fn checkpoint(&mut self) -> usize {
        self.events.push(Event::Checkpoint);
        self.events.len()
    }

    fn finish_node(&mut self) {
        self.events.push(Event::Close);
    }

    fn error(&mut self, kind: SyntaxErrorKind) {
        let range = self
            .tokens
            .get(self.pos)
            .map(|LexToken { range, .. }| range.clone())
            .unwrap_or_else(|| self.input.len() - 1..(self.input.len() - 1));
        self.errors.push(ParseError { range, kind });
    }

    fn expect(&mut self, token: Syntax) {
        if self.eat(token) {
            return;
        }
        self.error(SyntaxErrorKind::ExpectedToken(token));
    }

    fn eat(&mut self, token: Syntax) -> bool {
        if self.at(token) {
            self.advance(token);
            true
        } else {
            false
        }
    }

    fn ate(&mut self, token: Syntax) -> bool {
        if self.at(token) {
            self.expect(token);
            true
        } else {
            false
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn at(&self, token: Syntax) -> bool {
        self.nth(0) == token
    }

    fn peek(&self) -> Syntax {
        self.nth(0)
    }

    fn at_any(&self, tokens: TokenSet) -> bool {
        tokens.contains(self.nth(0))
    }

    fn nth(&self, lookahead: usize) -> Syntax {
        if self.fuel.get() == 0 {
            panic!("parser is stuck");
        }
        self.fuel.set(self.fuel.get() - 1);
        self.tokens
            .get(self.pos + lookahead)
            .map_or(Syntax::EOF, |t| t.token)
    }

    fn nth_span(&self, lookahead: usize) -> Range<usize> {
        self.tokens
            .get(self.pos + lookahead)
            .map(|t| t.range.clone())
            .unwrap_or_else(|| {
                let len = self.input.len();
                (len - 1)..(len - 1)
            })
    }

    fn advance_with_error(&mut self, kind: SyntaxErrorKind) {
        self.advance(ERROR);
        self.error(kind);
    }

    fn advance(&mut self, token: Syntax) {
        self.fuel = 1024.into();
        self.events.push(Event::Advance { token });
        self.pos += 1;
    }
}

pub(super) type Tree = syntree::Tree<u16, syntree::FlavorDefault>;
pub(super) type Node<'a> = syntree::Node<'a, u16, syntree::FlavorDefault>;
pub(super) type NodeId = syntree::pointer::PointerUsize;
pub(super) type Children<'a> = syntree::node::Children<'a, u16, syntree::FlavorDefault>;

struct Builder<'a> {
    builder: syntree::Builder<u16>,
    tokens_raw: Peekable<lexer::Lexer<'a>>,
    errors: Vec<ParseError>,
}

impl<'a> Builder<'a> {
    fn skip_whitespace(&mut self) {
        while self
            .tokens_raw
            .peek()
            .map(|t| t.token.is_whitespace())
            .unwrap_or_default()
        {
            let next = self.tokens_raw.next().unwrap();
            if next.token.is_whitespace() {
                self.builder
                    .token(next.token.into(), next.text.len())
                    .unwrap();
            }
        }
    }

    fn build_tree(mut self, events: &[Event]) -> (Tree, Vec<ParseError>) {
        let mut events_iter = events.iter().peekable();
        while let Some(event) = events_iter.next() {
            match event {
                Event::Open { node } => {
                    //we don't want to insert tokens before the first node (fixes a crash on
                    //builder.finish()
                    if node != &MODULE {
                        self.skip_whitespace();
                    }
                    self.builder.open((*node).into()).unwrap();
                }
                Event::Close => {
                    self.builder.close().unwrap();
                }
                Event::Advance { token } => {
                    self.skip_whitespace();
                    let Some(next) = self.tokens_raw.next() else {
                        continue;
                    };
                    self.builder
                        .token((*token).into(), next.text.len())
                        .unwrap();

                    if !matches!(events_iter.peek(), Some(Event::Close)) {
                        self.skip_whitespace();
                    }
                }
                Event::Checkpoint => {}
            }
        }
        (self.builder.build().unwrap(), self.errors)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathKind {
    Expr,
    Type,
}

impl<'a> Parser<'a> {
    fn module(&mut self) {
        self.with(MODULE, |this| {
            while !this.eof() {
                this.item();
            }
        })
    }

    fn item(&mut self) {
        self.compiler_attrib_list();
        if self.at_any(ITEM_FIRST) {
            match self.peek() {
                T![fn] => self.fn_item(),
                T![mod] => self.mod_item(),
                T![struct] => self.struct_item(),
                T![impl] => self.impl_item(),
                T![enum] => self.enum_item(),
                T![use] => self.use_item(),
                _ => {
                    self.advance_with_error(SyntaxErrorKind::ExpectedItem);
                }
            };
        } else {
            self.advance_with_error(SyntaxErrorKind::ExpectedItem);
        }
    }

    fn use_item(&mut self) {
        self.with(USE_ITEM, |this| {
            this.expect(T![use]);
            this.use_tree();
            this.expect(T![;]);
        });
    }

    fn use_tree(&mut self) {
        match self.peek() {
            T!["{"] => self.use_tree_list(),
            T![*] => self.with(USE_GLOBAL, |this| {
                this.expect(T![*]);
            }),
            T![self] => self.with(USE_SELF_NAME, |this| {
                this.expect(T![self]);
            }),
            T![root] => self.with(USE_ROOT_PATH, |this| {
                this.expect(T![root]);
                this.expect(T![:]);
                this.expect(T![:]);
                this.use_tree();
            }),

            T![super] => self.with(USE_SUPER_PATH, |this| {
                this.expect(T![super]);
                this.expect(T![:]);
                this.expect(T![:]);
                this.use_tree();
            }),
            IDENT => {
                if self.nth(1) == T![:] {
                    self.with(USE_PATH, |this| {
                        this.name();
                        this.expect(T![:]);
                        this.expect(T![:]);
                        this.use_tree();
                    });
                } else {
                    self.with(USE_NAME, |this| {
                        this.name();
                    })
                }
            }
            _ => {
                if self.at_any(USE_RECOVERY) {
                    return;
                }
                self.advance_with_error(SyntaxErrorKind::ExpectedUseDeclaration);
            }
        };
    }

    fn use_tree_list(&mut self) {
        self.with(USE_TREE_LIST, |this| {
            this.expect(T!["{"]);
            while !this.at(T!["}"]) && !this.eof() {
                if this.at_any(USE_FIRST) {
                    this.use_tree();
                } else {
                    if this.at_any(USE_RECOVERY) {
                        break;
                    } else {
                        this.advance_with_error(SyntaxErrorKind::ExpectedUseDeclaration);
                    }
                }
                if !this.at(T!["}"]) {
                    this.expect(T![,]);
                }
            }
            this.expect(T!["}"]);
        });
    }

    fn enum_item(&mut self) {
        self.with(ENUM_ITEM, |this| {
            this.expect(T![enum]);
            this.name();
            this.generics();
            this.enum_elem_list();
        });
    }

    fn enum_elem_list(&mut self) {
        self.expect(T!["{"]);
        while !self.at(T!["}"]) && !self.eof() {
            self.compiler_attrib_list();
            if self.at_any(ELEMENT_FIRST) {
                self.enum_element();
            } else {
                if self.at_any(ELEMENT_RECOVERY) {
                    break;
                }
                self.advance_with_error(SyntaxErrorKind::ExpectedEnumElement);
            }
        }
        self.expect(T!["}"]);
    }

    fn enum_element(&mut self) {
        if self.at(T![fn]) {
            self.fn_item();
        } else {
            self.enum_field();
            if !self.at(T!["}"]) {
                self.expect(T![,]);
            }
        }
    }

    fn enum_field(&mut self) {
        self.with(FIELD, |this| {
            this.name();
            if this.ate(T![:]) {
                this.item_type_expr();
            }
            if this.ate(T![=]) {
                this.expr();
            }
        });
    }

    fn field(&mut self) {
        self.with(FIELD, |this| {
            this.name();
            this.expect(T![:]);
            this.item_type_expr();
            if this.ate(T![=]) {
                this.expr();
            }
        });
    }

    fn impl_item(&mut self) {
        self.with(IMPL_ITEM, |this| {
            this.expect(T![impl]);
            this.generics();
            this.type_expr();
            if this.ate(T![for]) {
                this.type_expr();
            }
            this.expect(T!["{"]);

            while !this.at(T!["}"]) && !this.eof() {
                if this.at(T![fn]) {
                    this.fn_item();
                } else {
                    if this.at_any(ITEM_FIRST) {
                        break;
                    } else {
                        this.advance_with_error(SyntaxErrorKind::ExpectedItem);
                    }
                }
            }
            this.expect(T!["}"]);
        });
    }

    fn struct_item(&mut self) {
        self.with(STRUCT_ITEM, |this| {
            this.expect(T![struct]);
            this.name();
            this.generics();
            if this.at(T![:]) {
                this.parent();
            }
            this.struct_elem_list();
        })
    }

    fn struct_elem_list(&mut self) {
        self.expect(T!["{"]);
        while !self.at(T!["}"]) && !self.eof() {
            self.compiler_attrib_list();
            if self.at_any(ELEMENT_FIRST) {
                self.struct_element();
            } else {
                if self.at_any(ELEMENT_RECOVERY) {
                    break;
                }
                self.advance_with_error(SyntaxErrorKind::ExpectedStructElement);
            }
        }
        self.expect(T!["}"]);
    }

    fn struct_element(&mut self) {
        if self.at(T![fn]) {
            self.fn_item();
        } else {
            self.field();
            if !self.at(T!["}"]) {
                self.expect(T![,]);
            }
        }
    }

    fn parent(&mut self) {
        self.with(PARENT, |this| {
            this.expect(T![:]);
            this.type_path();
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
            while !this.at(T!["}"]) && !this.eof() {
                this.item();
            }
            this.expect(T!["}"]);
        })
    }

    fn fn_item(&mut self) {
        self.with(FN_ITEM, |this| {
            this.expect(T![fn]);
            this.name();
            this.generics();
            this.param_list();
            if this.at(T![->]) {
                this.return_type();
            }

            match this.peek() {
                T!["{"] => {
                    this.block();
                }
                T![;] => {
                    this.expect(T![;]);
                }
                _ => {
                    this.error(SyntaxErrorKind::ExpectedToken(T!["{"]));
                }
            }
        })
    }

    fn param_list(&mut self) {
        self.with(PARAM_LIST, |this| {
            this.expect(T!["("]);
            while !this.at(T![")"]) && !this.eof() {
                this.compiler_attrib_list();
                if this.at_any(PATTERN_FIRST) || this.at(T![self]) {
                    this.param();
                    if !this.at(T![")"]) {
                        this.expect(T![,]);
                    }
                } else {
                    if this.at_any(PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_error(SyntaxErrorKind::ExpectedParameter);
                }
            }
            this.expect(T![")"]);
        })
    }

    fn param(&mut self) {
        self.with(PARAM, |this| {
            if !this.ate(T![self]) {
                this.pattern();
                this.expect(T![:]);
                this.type_expr();
                if this.ate(T![=]) {
                    this.expr();
                }
            }
        })
    }

    fn pattern(&mut self) {
        match self.peek() {
            token
            @ (INT | FLOAT | STRING | TRUE_KW | FALSE_KW | SINGLE_STRING | BRACKET_STRING) => {
                self.with(LIT_PAT, |this| {
                    this.ate(token);
                });
            }
            IDENT => {
                if self.nth(1) == T![:] && self.nth(2) == T![:] {
                    self.with(PATH_PAT, |this| {
                        this.expr_path();
                    });
                } else {
                    self.with(NAME_PAT, |this| {
                        this.name();
                    });
                }
            }
            T![_] => self.with(WILDCARD_PAT, |this| {
                this.expect(T![_]);
            }),
            _ => {
                self.error(SyntaxErrorKind::ExpectedPattern);
            }
        }
    }

    fn return_type(&mut self) {
        self.with(RETURN_TYPE, |this| {
            this.expect(T![->]);
            this.type_expr();
        });
    }

    fn item_type_expr(&mut self) {
        if self.at_any(ITEM_TYPE_FIRST) {
            match self.peek() {
                T![struct] => self.struct_item(),
                T![enum] => self.enum_item(),
                _ => self.type_expr(),
            }
        } else {
            self.advance_with_error(SyntaxErrorKind::ExpectedType);
        }
    }

    fn type_expr(&mut self) {
        if self.at_any(TYPE_FIRST) {
            let checkpoint = self.checkpoint();

            match self.peek() {
                T![fn] => {
                    self.fn_type();
                }
                T![Self] => self.with(SELF_TYPE, |this| {
                    this.expect(T![Self]);
                }),
                T![dyn] => self.with(DYN_TYPE, |this| {
                    this.expect(T![dyn]);
                    this.dyn_bounds();
                }),
                T!["("] => {
                    self.expect(T!["("]);
                    if self.at(T![")"]) {
                        self.with_at(UNIT_TYPE, checkpoint, |this| {
                            this.expect(T![")"]);
                        })
                    } else {
                        self.type_expr();
                        if self.at(T![")"]) {
                            self.with_at(PAREN_TYPE, checkpoint, |this| {
                                this.expect(T![")"]);
                            })
                        } else {
                            self.with_at(TUPLE_TYPE, checkpoint, |this| {
                                this.ate(T![,]);
                                this.tuple_type();
                                this.expect(T![")"]);
                            });
                        }
                    }
                    if self.ate(T![?]) {
                        self.with_at(NILABLE_TYPE, checkpoint, |_| {});
                    }
                }
                _ => {
                    let next_span = self.nth_span(0);
                    let can_be_lit = self.at_path_sep(1);
                    self.with(PATH_TYPE, |this| {
                        this.type_path();
                    });
                    if !can_be_lit {
                        match &self.input[next_span] {
                            "int" => {
                                self.with_at(LIT_TYPE_INT, checkpoint, |this| {
                                    this.with_at(LIT_TYPE, checkpoint, |_| {});
                                });
                            }
                            "float" => {
                                self.with_at(LIT_TYPE_FLOAT, checkpoint, |this| {
                                    this.with_at(LIT_TYPE, checkpoint, |_| {});
                                });
                            }
                            "bool" => {
                                self.with_at(LIT_TYPE_BOOL, checkpoint, |this| {
                                    this.with_at(LIT_TYPE, checkpoint, |_| {});
                                });
                            }
                            "string" => {
                                self.with_at(LIT_TYPE_STRING, checkpoint, |this| {
                                    this.with_at(LIT_TYPE, checkpoint, |_| {});
                                });
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
            self.advance_with_error(SyntaxErrorKind::ExpectedType);
        }
    }

    fn dyn_bounds(&mut self) {
        self.type_expr();
        while self.ate(T![+]) {
            self.type_expr();
        }
    }

    fn tuple_type(&mut self) {
        while !self.at(T![")"]) && !self.eof() {
            if self.at_any(TYPE_FIRST) {
                self.type_expr();
                if !self.at(T![")"]) {
                    self.expect(T![,]);
                }
            } else {
                if self.at_any(TUPLE_RECOVERY) {
                    break;
                } else {
                    self.advance_with_error(SyntaxErrorKind::ExpectedType);
                }
            }
        }
    }

    fn expr_path(&mut self) {
        self.path(PathKind::Expr);
    }

    fn type_path(&mut self) {
        self.path(PathKind::Type);
    }

    fn path(&mut self, kind: PathKind) {
        self.with(PATH, |this| {
            if this.at_any(PATH_FIRST) {
                this.path_segment(kind);
            } else {
                this.advance_with_error(SyntaxErrorKind::ExpectedPath);
                return;
            }

            while this.at_path_sep(0) && !this.eof() {
                this.expect(T![:]);
                this.expect(T![:]);

                if this.at_any(PATH_FIRST) {
                    this.path_segment(kind);
                } else {
                    this.advance_with_error(SyntaxErrorKind::ExpectedPathSegment);
                    break;
                }
            }
        });
        // if self.at_any(TokenSet::new(&[IDENT, ROOT_KW, SUPER_KW])) {
        //     self.with(PATH, |this| {
        //         if this.at(T![root]) {
        //             this.expect(T![root]);
        //         } else if this.at(T![super]) {
        //             this.expect(T![super]);
        //         } else {
        //             this.expect(IDENT);
        //         }
        //         let mut at_super = this.at(T![super]);
        //         while this.at_path_sep(0) && !this.at(EOF) {
        //             this.expect(T![:]);
        //             this.expect(T![:]);
        //             if !is_type_path && this.at(T![<]) {
        //                 this.generic_args();
        //                 return;
        //             }
        //             if this.at(T![super]) && at_super {
        //                 this.expect(T![super]);
        //             } else {
        //                 at_super = false;
        //                 this.expect(IDENT);
        //             }
        //         }
        //         if this.at(T![<]) {
        //             this.generic_args();
        //         }
        //     });
        // }
    }

    fn can_parse_generic_args(&mut self) -> bool {
        let mut result = true;
        self.with_save_point(
            |this| this.generic_args(),
            |this| {
                if !this.save_point_errors.is_empty() {
                    result = false;
                }

                if !GENERIC_ANCHOR.contains(this.nth(0)) {
                    result = false;
                }
            },
        );
        result
    }

    fn path_segment(&mut self, kind: PathKind) {
        self.with(PATH_SEGMENT, |this| {
            if this.at(T![root]) {
                this.expect(T![root]);
            } else if this.at(T![super]) {
                this.expect(T![super]);
            } else {
                this.expect(IDENT);
                if this.at(T![<]) {
                    match kind {
                        PathKind::Expr => {
                            if this.can_parse_generic_args() {
                                this.generic_args();
                            }
                        }
                        PathKind::Type => this.generic_args(),
                    }
                }
            }
        });
    }

    fn fn_type(&mut self) {
        self.with(FN_TYPE, |this| {
            this.expect(T![fn]);
            this.fn_type_param_list();
            if this.at(T![->]) {
                this.return_type();
            }
        });
    }

    fn fn_type_param_list(&mut self) {
        self.with(FN_TYPE_PARAM_LIST, |this| {
            this.expect(T!["("]);
            while !this.at(T![")"]) && !this.eof() {
                if this.at_any(TYPE_FIRST) || this.at(IDENT) {
                    this.fn_type_param();
                    if !this.at(T![")"]) {
                        this.expect(T![,]);
                    }
                } else {
                    if this.at_any(FN_TYPE_PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_error(SyntaxErrorKind::ExpectedParameter);
                }
            }
            this.expect(T!(")"));
        })
    }

    fn fn_type_param(&mut self) {
        self.with(FN_TYPE_PARAM, |this| {
            if this.nth(1) == T![:] {
                this.name();
                this.expect(T![:]);
            }
            this.type_expr();
        })
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let checkpoint = self.checkpoint();

        match self.peek().prefix_bp() {
            Some(rbp) => {
                self.with(UNARY_EXPR, |this| {
                    this.expect(this.peek());
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
            let op = self.peek();

            let Some((left_bp, right_bp)) = op.infix_bp() else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.expect(op);

            if self.at_any(EXPR_FIRST) {
                self.with_at(BINARY_EXPR, checkpoint, |this| this.expr_bp(right_bp));
            } else {
                self.advance_with_error(SyntaxErrorKind::ExpectedExpression);
            }
        }
    }

    fn prefix_expr(&mut self) -> Option<()> {
        let token = self.peek();
        let checkpoint = self.checkpoint();
        match token {
            T![for] => self.for_expr(),
            T![while] => self.while_expr(),
            T![loop] => self.loop_expr(),
            T![return] => self.return_expr(),
            T![if] => self.if_expr(),
            INT | FLOAT | STRING | TRUE_KW | FALSE_KW | NIL_KW | SINGLE_STRING | BRACKET_STRING => {
                self.with(LIT_EXPR, |this| {
                    this.ate(token);
                });
            }
            T!["("] => {
                let checkpoint = self.checkpoint();
                self.expect(T!["("]);

                if self.at(T![")"]) {
                    self.with_at(UNIT_EXPR, checkpoint, |this| {
                        this.expect(T![")"]);
                    })
                } else {
                    self.expr();
                    if self.at(T![")"]) {
                        self.with_at(PAREN_EXPR, checkpoint, |this| {
                            this.expect(T![")"]);
                        });
                    } else {
                        self.with_at(TUPLE_EXPR, checkpoint, |this| {
                            this.ate(T![,]);
                            this.tuple_expr();
                            this.expect(T![")"]);
                        })
                    }
                }
            }
            T!["{"] => {
                self.block();
            }
            T![lua] => {
                self.lua_block_expr();
            }
            T![|] => {
                self.closure_expr();
            }
            T![self] => {
                self.with(SELF_EXPR, |this| this.expect(T![self]));
            }
            IDENT => {
                let checkpoint = self.checkpoint();
                self.expr_path();

                if self.at(T!["{"])
                    && ((self.nth(1) == IDENT && self.nth(2) == T![:]) || self.nth(1) == T!["}"])
                {
                    self.with_at(RECORD_EXPR, checkpoint, |this| this.record_field_list());
                } else {
                    self.with_at(PATH_EXPR, checkpoint, |_| {});
                }
            }
            _ => {
                self.advance_with_error(SyntaxErrorKind::ExpectedExpression);
                return None;
            }
        }
        'outer: loop {
            match self.peek() {
                T!["("] => {
                    self.with_at(CALL_EXPR, checkpoint, |this| this.arg_list());
                }
                T!["["] => {
                    self.with_at(INDEX_EXPR, checkpoint, |this| this.index());
                }
                T![?] | T![.] => {
                    let safe_call = self.ate(T![?]);
                    self.expect(T![.]);
                    self.name();
                    'inner: {
                        if self.at(T![<]) && self.can_parse_generic_args() {
                            self.generic_args();

                            if self.at(T!["("]) {
                                self.arg_list();
                                break 'inner;
                            }
                        }
                        if self.at(T!["("]) {
                            self.arg_list();
                            break 'inner;
                        }
                        if safe_call {
                            self.with_at(FIELD_EXPR, checkpoint, |this| {
                                this.with_at(SAFE_FIELD_EXPR, checkpoint, |_| {});
                            })
                        } else {
                            self.with_at(FIELD_EXPR, checkpoint, |_| {})
                        }
                        continue 'outer;
                    }

                    if safe_call {
                        self.with_at(METHOD_EXPR, checkpoint, |this| {
                            this.with_at(SAFE_METHOD_EXPR, checkpoint, |_| {});
                        })
                    } else {
                        self.with_at(METHOD_EXPR, checkpoint, |_| {})
                    }
                }
                _ => break,
            }
        }

        match self.peek() {
            T![!is] => {
                self.with_at(IS_NOT_EXPR, checkpoint, |this| {
                    this.expect(T![!is]);
                    this.pattern();
                });
            }
            T![is] => {
                self.with_at(IS_EXPR, checkpoint, |this| {
                    this.expect(T![is]);
                    this.pattern();
                });
            }
            T![as] => {
                self.with_at(AS_EXPR, checkpoint, |this| {
                    this.expect(T![as]);
                    this.type_expr();
                });
            }
            _ => {}
        }
        Some(())
    }

    fn loop_expr(&mut self) {
        self.with(LOOP_EXPR, |this| {
            this.expect(T![loop]);
            this.block();
        });
    }

    fn while_expr(&mut self) {
        self.with(WHILE_EXPR, |this| {
            this.expect(T![while]);
            this.expr();
            this.block();
        });
    }

    fn for_expr(&mut self) {
        self.with(FOR_EXPR, |this| {
            this.expect(T![for]);
            this.expr();
            this.expect(T![in]);
            this.expr();
            this.block();
        })
    }

    fn tuple_expr(&mut self) {
        while !self.at(T![")"]) && !self.eof() {
            if self.at_any(EXPR_FIRST) {
                self.expr();
                if !self.at(T![")"]) {
                    self.expect(T![,]);
                }
            } else {
                if self.at_any(TUPLE_RECOVERY) {
                    break;
                } else {
                    self.advance_with_error(SyntaxErrorKind::ExpectedExpression);
                }
            }
        }
    }

    fn lua_block_expr(&mut self) {
        LuaParser::new(self).chunk();
    }

    fn record_field_list(&mut self) {
        self.expect(T!["{"]);
        while !self.at(T!["}"]) && !self.eof() {
            if self.at(IDENT) {
                self.record_field();
            } else {
                if self.at_any(RECORD_LIST_RECOVERY) {
                    break;
                }
                self.advance_with_error(SyntaxErrorKind::ExpectedField);
            }
        }
        self.expect(T!["}"]);
    }

    fn record_field(&mut self) {
        self.with(RECORD_FIELD, |this| {
            this.name();
            this.expect(T![:]);
            this.expr();
            if !this.at(T!["}"]) {
                this.expect(T![,]);
            }
        });
    }

    fn closure_expr(&mut self) {
        self.with(CLOSURE_EXPR, |this| {
            this.closure_param_list();
            if this.at(T![->]) {
                this.return_type();
            }
            this.expr();
        })
    }

    fn closure_param_list(&mut self) {
        self.with(CLOSURE_PARAM_LIST, |this| {
            this.expect(T![|]);
            while !this.at(T![|]) && !this.eof() {
                if this.at_any(PATTERN_FIRST) {
                    this.closure_param();
                } else {
                    if this.at_any(CLOSURE_PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_error(SyntaxErrorKind::ExpectedParameter);
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
            if !this.at(T![|]) {
                this.expect(T![,]);
            }
        });
    }

    fn if_expr(&mut self) {
        self.with(IF_EXPR, |this| {
            this.expect(T![if]);
            this.expr();
            this.block();
            if this.ate(T![else]) {
                if this.at(T![if]) {
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
            if this.at_any(EXPR_FIRST) {
                this.expr();
            }
        })
    }

    fn arg_list(&mut self) {
        self.expect(T!["("]);
        while !self.at(T![")"]) && !self.eof() {
            if self.at_any(EXPR_FIRST) {
                self.arg();
                if !self.at(T![")"]) {
                    self.expect(T![,]);
                }
            } else {
                if self.at_any(ARG_LIST_RECOVERY) {
                    break;
                } else {
                    self.advance_with_error(SyntaxErrorKind::ExpectedArgument);
                }
            }
        }
        self.expect(T![")"]);
    }

    fn index(&mut self) {
        self.expect(T!["["]);
        self.expr();
        self.expect(T!["]"]);
    }

    fn arg(&mut self) {
        self.with(ARG, |this| {
            let has_label = this.nth(1) == T![:];
            if has_label {
                this.name();
                this.expect(T![:]);
            }
            if this.at_any(EXPR_FIRST) || !has_label {
                this.expr();
            }
        });
    }

    fn generic_args(&mut self) {
        self.with(GENERIC_ARGUMENTS, |this| {
            this.expect(T![<]);
            while !this.at(T![>]) && !this.eof() {
                if this.at_any(TYPE_FIRST) {
                    this.type_expr();
                } else {
                    if this.at_any(TokenSet::new(&[T![>], T![;]])) {
                        break;
                    } else {
                        this.advance_with_error(SyntaxErrorKind::ExpectedType);
                    }
                }
                if !this.at(T![>]) {
                    this.expect(T![,]);
                }
            }
            this.expect(T![>]);
        });
    }

    fn generics(&mut self) {
        if !self.at(T![<]) {
            return;
        }
        self.with(GENERICS, |this| {
            this.expect(T![<]);
            while !this.at(T![>]) && !this.eof() {
                if this.at(IDENT) {
                    this.type_param();
                    if !this.at(T![>]) {
                        this.expect(T![,]);
                    }
                } else {
                    if this.at_any(GENERICS_RECOVERY) {
                        break;
                    } else {
                        this.advance_with_error(SyntaxErrorKind::ExpectedGeneric);
                    }
                }
            }
            this.expect(T![>]);
        });
    }

    fn type_param(&mut self) {
        self.with(TYPE_PARAM, |this| {
            this.name();
            if this.at(T![:]) {
                this.expect(T![:]);
                this.type_expr();
                while this.at(T![+]) {
                    this.expect(T![+]);
                    this.type_expr();
                }
            }
        });
    }

    fn block(&mut self) {
        self.with(BLOCK_EXPR, |this| {
            this.expect(T!["{"]);
            while !this.at(T!["}"]) && !this.eof() {
                this.compiler_attrib_list();
                match this.peek() {
                    T![let] => this.stmt_let(),
                    T![;] => {
                        this.expect(T![;]);
                    }
                    _ => {
                        if this.at_any(EXPR_FIRST) {
                            this.stmt_expr();
                        } else {
                            if this.at_any(STMT_EXPR_RECOVERY) {
                                break;
                            }

                            this.advance_with_error(SyntaxErrorKind::ExpectedStatement);
                        }
                    }
                }
            }

            this.expect(T!["}"]);
        });
    }

    fn stmt_expr(&mut self) {
        self.with(EXPR_STMT, |this| {
            this.expr();

            if this.at(T![;]) {
                this.expect(T![;]);
            } else {
                if !this.at(T!["}"]) {
                    this.expect(T![;]);
                }
            }
        })
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

    fn compiler_attrib_list(&mut self) {
        if !self.at(T![@]) {
            return;
        }
        self.with(COMPILER_ATTRIB_LIST, |this| {
            while this.at(T![@]) {
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

            while !this.at(T![")"]) && !this.eof() {
                if this.at_any(EXPR_FIRST) {
                    this.compiler_attrib_item();
                } else {
                    if this.at_any(COMPILER_ATTRIB_RECOVERY) {
                        break;
                    }
                    this.advance_with_error(SyntaxErrorKind::ExpectedAttribute);
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
            if !this.at(T![")"]) {
                this.expect(T![,]);
            }
        });
    }

    fn at_path_sep(&self, lookahead: usize) -> bool {
        self.nth(lookahead) == self.nth(1 + lookahead) && self.nth(lookahead) == T![:]
    }

    fn name(&mut self) {
        self.with(NAME, |this| {
            this.expect(IDENT);
        })
    }
}

pub struct LuaParser<'a, 'b> {
    parser: &'b mut Parser<'a>,
}

impl<'a, 'b> LuaParser<'a, 'b> {
    fn new(parser: &'b mut Parser<'a>) -> Self {
        Self { parser }
    }

    fn with<R>(&mut self, syntax: Syntax, body: impl FnOnce(&mut Self) -> R) -> R {
        self.parser.start_node(syntax);
        let res = body(self);
        self.parser.finish_node();
        res
    }

    fn with_at<R>(
        &mut self,
        syntax: Syntax,
        checkpoint: usize,
        body: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.parser.start_node_at(checkpoint, syntax);
        let res = body(self);
        self.parser.finish_node();
        res
    }

    fn error(&mut self, kind: SyntaxErrorKind) {
        self.parser.error(kind);
    }

    fn expect(&mut self, token: Syntax) {
        self.parser.expect(token);
    }

    fn expect_advance(&mut self, token: Syntax) {
        if self.ate(token) {
            return;
        }

        self.advance_with_err(SyntaxErrorKind::ExpectedToken(token));
    }

    fn at(&mut self, token: Syntax) -> bool {
        self.parser.at(token)
    }

    fn eat(&mut self, token: Syntax) -> bool {
        self.parser.eat(token)
    }

    fn ate(&mut self, token: Syntax) -> bool {
        self.parser.ate(token)
    }

    fn eof(&self) -> bool {
        self.parser.eof()
    }

    fn peek(&self) -> Syntax {
        self.parser.peek()
    }

    fn checkpoint(&mut self) -> usize {
        self.parser.checkpoint()
    }

    fn peek_text(&self) -> &str {
        let span = self.nth_span(0);
        &self.parser.input[span]
    }

    fn nth(&self, lookahead: usize) -> Syntax {
        self.parser.nth(lookahead)
    }

    fn advance_with_err(&mut self, error: SyntaxErrorKind) {
        self.parser.advance_with_error(error);
    }

    fn nth_span(&self, lookahead: usize) -> Range<usize> {
        self.parser.nth_span(lookahead)
    }

    fn nth_text(&self, offset: usize) -> Option<&str> {
        if self.nth(offset) == EOF {
            return None;
        }
        let next_span = self.nth_span(offset);
        Some(&self.parser.input[next_span])
    }

    fn expect_ident(&mut self, ident: &str) {
        let next_span = self.parser.nth_span(0);
        self.parser.expect(IDENT);

        if &self.parser.input[next_span] != ident {
            self.error(SyntaxErrorKind::Other(format!("expected {}", ident)));
        }
    }

    fn expect_advance_ident(&mut self, ident: &str) {
        let next_span = self.parser.nth_span(0);
        if !self.parser.at(IDENT) {
            self.error(SyntaxErrorKind::Other(format!("expected {}", ident)));
            return;
        }

        self.parser.expect(IDENT);
        if &self.parser.input[next_span] != ident {
            self.error(SyntaxErrorKind::Other(format!("expected {}", ident)));
        }
    }

    fn chunk(&mut self) {
        self.with(LUA_CHUNK_EXPR, |this| {
            this.expect(T![lua]);
            this.expect(T!["{"]);

            while !this.at(T!["}"]) && !this.eof() {
                if this.stmt().is_none() {
                    break;
                }
            }

            this.expect(T!["}"]);
        });
    }

    #[must_use]
    fn stmt(&mut self) -> Option<()> {
        match self.parser.peek() {
            T![#] => {
                if self.nth(1) == T![return] {
                    self.hash_return_stmt();
                    None
                } else {
                    self.stmt_expr();
                    Some(())
                }
            }
            T![return] => {
                self.return_stmt();
                Some(())
            }
            T![break] => {
                self.break_stmt();
                Some(())
            }
            T![while] => {
                self.while_stmt();
                Some(())
            }
            T![if] => {
                self.if_stmt();
                Some(())
            }
            T![for] => {
                self.for_stmt();
                Some(())
            }
            T![;] => {
                self.expect(T![;]);
                Some(())
            }
            IDENT => {
                let Some(next) = self.nth_text(0) else {
                    //TODO: test this branch
                    self.advance_with_err(SyntaxErrorKind::ExpectedExpression);
                    return Some(());
                };

                match next {
                    "do" => self.with(LUA_BLOCK_STMT, |this| {
                        this.expect_ident("do");
                        this.block(|t| t == "end");
                        this.expect_ident("end");
                        Some(())
                    }),
                    "repeat" => {
                        self.repeat_stmt();
                        Some(())
                    }
                    "local" => {
                        self.local_stmt();
                        Some(())
                    }
                    "function" => {
                        self.function_stmt();
                        Some(())
                    }
                    _ => {
                        self.stmt_expr();
                        Some(())
                    }
                }
            }
            _ => {
                self.stmt_expr();
                Some(())
            }
        }
    }

    fn stmt_expr(&mut self) {
        self.with(LUA_STMT_EXPR, |this| {
            this.expr_multi();
            if this.ate(T![=]) {
                this.expr_multi();
            }
            this.expect(T![;]);
        })
    }

    fn param_list(&mut self) {
        self.with(LUA_PARAM_LIST, |this| {
            this.expect(T!["("]);
            while !this.at(T![")"]) && !this.eof() {
                if this.at(IDENT) {
                    this.param();
                    if !this.at(T![")"]) {
                        this.expect(T![,]);
                    }
                } else {
                    this.advance_with_err(SyntaxErrorKind::ExpectedParameter);
                    break;
                }
            }
            this.expect(T![")"]);
        });
    }

    fn param(&mut self) {
        self.with(LUA_PARAM, |this| {
            this.name();
        })
    }

    fn hash_return_stmt(&mut self) {
        self.with(LUA_HASH_RETURN_STMT, |this| {
            this.expect(T![#]);
            this.expect(T![return]);
            this.expr_multi();
            this.expect(T![;]);
        });
    }

    fn return_stmt(&mut self) {
        self.with(LUA_RETURN_STMT, |this| {
            this.expect(T![return]);
            this.expr_multi();
            this.expect(T![;]);
        })
    }

    fn break_stmt(&mut self) {
        self.with(LUA_BREAK_STMT, |this| {
            this.expect(T![break]);
            this.expect(T![;]);
        });
    }

    fn while_stmt(&mut self) {
        self.with(LUA_WHILE_STMT, |this| {
            this.expect(T![while]);
            this.expr();
            this.expect_ident("do");
            this.block(|t| t == "end");
            this.expect_ident("end");
        });
    }

    fn if_stmt(&mut self) {
        self.with(LUA_IF_STMT, |this| {
            this.expect(T![if]);
            this.expr();
            this.expect_ident("then");
            this.block(|t| t == "else" || t == "elseif" || t == "end");
            if this.peek_text() == "end" {
                this.expect_advance_ident("end");
                return;
            }
            loop {
                match this.peek_text() {
                    "elseif" => this.with(LUA_ELSEIF, |this| {
                        this.expect_ident("elseif");
                        this.expr();
                        this.expect_ident("then");
                        this.block(|t| t == "else" || t == "elseif" || t == "end");
                    }),
                    "else" => {
                        this.with(LUA_ELSE, |this| {
                            this.expect_advance(T![else]);
                            this.block(|t| t == "else" || t == "elseif" || t == "end");
                        });
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
            if this.nth(2) == T![=] {
                this.with(LUA_NUMERIC_FOR, |this| {
                    this.expect(T![for]);
                    this.name();
                    this.expect(T![=]);

                    this.expr();
                    if this.ate(T![,]) {
                        this.expr();
                    }
                    if this.ate(T![,]) {
                        this.expr();
                    }
                    this.expect_ident("do");
                    this.block(|t| t == "end");
                    this.expect_ident("end");
                });
            } else {
                this.with(LUA_GENERIC_FOR, |this| {
                    this.expect(T![for]);
                    this.name();
                    while this.ate(T![,]) && !this.eof() {
                        this.name();
                    }
                    this.expect(T![in]);
                    this.expr();
                    this.expect_ident("do");
                    this.block(|t| t == "end");
                    this.expect_ident("end");
                })
            }
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
            while this.ate(T![,]) && !this.eof() {
                this.name();
            }
            this.expect(T![=]);
            this.expr_multi();
            this.expect(T![;]);
        });
    }

    fn function_stmt(&mut self) {
        self.with(LUA_FUNCTION_STMT, |this| {
            this.expect_ident("function");
            this.name();

            if let token @ (T![.] | T![:]) = this.peek() {
                this.expect(token);
                this.name();
            }
            this.param_list();
            this.block(|t| t == "end");
            this.expect_ident("end");
        });
    }

    fn expr_multi(&mut self) {
        self.with(LUA_MULTI_EXPR, |this| {
            this.expr();
            if this.ate(T![,]) {
                this.expr();
                while this.ate(T![,]) && !this.eof() {
                    this.expr();
                }
            }
        });
    }

    fn index(&mut self) {
        self.expect(T!["["]);
        self.expr();
        self.expect(T!["]"]);
    }

    fn arg_list(&mut self) {
        self.with(LUA_ARG_LIST, |this| {
            this.expect(T!["("]);
            while !this.at(T![")"]) && !this.eof() {
                this.arg();
                if !this.at(T![")"]) {
                    this.expect(T![,]);
                }
            }
            this.expect(T![")"]);
        })
    }

    fn arg(&mut self) {
        self.with(LUA_ARG, |this| {
            this.expr();
        });
    }

    fn expr(&mut self) {
        self.expr_bp(0);
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let checkpoint = self.checkpoint();
        match Self::prefix_bp(self.peek()) {
            Some(rbp) => self.with(LUA_UNARY_EXPR, |this| {
                this.expect(this.peek());
                this.expr_bp(rbp);
            }),
            None => {
                if self.prefix_expr().is_none() {
                    return;
                }
            }
        }

        loop {
            let op = self.peek();

            let Some((left_bp, right_bp)) = Self::infix_bp(op) else {
                break;
            };

            if left_bp < min_bp {
                break;
            }

            self.expect(op);

            if self.at_expr() {
                self.with_at(LUA_BINARY_EXPR, checkpoint, |this| {
                    this.expr_bp(right_bp);
                })
            } else {
                self.advance_with_err(SyntaxErrorKind::ExpectedExpression);
            }
        }
    }

    fn at_expr(&self) -> bool {
        let token = self.peek();
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

    fn prefix_expr(&mut self) -> Option<()> {
        let token = self.peek();
        let checkpoint = self.checkpoint();
        match token {
            T![#] => {
                self.hash_name();
            }
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
                if self.peek_text() == "function" {
                    self.with(LUA_FUNCTION_EXPR, |this| {
                        this.expect_ident("function");
                        this.param_list();
                        this.block(|t| t == "end");
                        this.expect_ident("end");
                    });
                } else {
                    self.with(LUA_LIT_EXPR, |this| {
                        this.ate(token);
                    });
                }
            }
            T!["{"] => {
                self.with(LUA_TABLE_EXPR, |this| {
                    this.expect(T!["{"]);
                    while !this.at(T!["}"]) && !this.eof() {
                        if this.nth(1) == T![=] {
                            this.with(LUA_ELEM_ASSIGN, |this| {
                                this.name();
                                this.expect(T![=]);
                                this.expr_bp(2);
                            });
                        } else if this.at(T!["["]) {
                            this.with(LUA_ELEM_INDEX_ASSIGN, |this| {
                                this.expect(T!["["]);
                                this.expr();
                                this.expect(T!["]"]);
                                this.expect(T![=]);
                                this.expr_bp(2);
                            });
                        } else {
                            this.with(LUA_ELEM_EXPR, |this| {
                                this.expr_bp(2);
                            });
                        }
                        if !this.at(T!["}"]) {
                            this.expect(T![,]);
                        }
                    }
                    this.expect(T!["}"]);
                });
            }
            _ => {
                self.advance_with_err(SyntaxErrorKind::ExpectedExpression);
                return None;
            }
        };
        loop {
            match self.peek() {
                T!["("] => {
                    self.with_at(LUA_CALL_EXPR, checkpoint, |this| this.arg_list());
                }
                T!["["] => {
                    self.with_at(LUA_INDEX_EXPR, checkpoint, |this| this.index());
                }
                T![.] | T![:] => {
                    self.with_at(LUA_FIELD_ACCESS_EXPR, checkpoint, |this| {
                        this.expect(this.peek());
                        this.name();
                    });
                }
                _ => break,
            }
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
            T![*] | T![/] | T![%] => (11, 12),
            T![^] => (16, 15),
            _ => return None,
        })
    }

    fn block(&mut self, stop_condition: impl Fn(&str) -> bool) {
        while self
            .nth_text(0)
            .map(|t| !stop_condition(t))
            .unwrap_or(false)
            && !self.eof()
        {
            if self.stmt().is_none() {
                break;
            }
        }
    }

    fn hash_name(&mut self) {
        self.with(LUA_HASH_NAME, |this| {
            this.expect(T![#]);
            this.name();
        })
    }

    fn name(&mut self) {
        self.with(LUA_NAME, |this| {
            this.expect(IDENT);
        })
    }
}

#[cfg(test)]
mod test {
    use crate::parsing::ast::NodeExt;

    use super::{Parser, PathKind, Syntax};

    fn parse_rec(source: &str, child: super::Node<'_>, result: &mut String, depth: usize) {
        (0..depth).for_each(|_| result.push_str("  "));
        result.push_str(&format!(
            "{:?}: {}..{}{}",
            child.kind(),
            child.range().start,
            child.range().end,
            if child.is_empty() && child.value() != Syntax::WHITESPACE.into() {
                format!(" \"{}\"", &source[child.range()])
            } else {
                String::from(" ")
            }
        ));
        result.push('\n');
        if child.is_empty() {
            return;
        }
        for child in child.children() {
            parse_rec(source, child, result, depth + 1);
        }
    }

    #[track_caller]
    fn try_parse(source: &str, f: impl FnOnce(&mut Parser)) -> (String, Vec<super::ParseError>) {
        let mut parser = Parser::new(source);
        f(&mut parser);
        let (tree, errors) = parser.build_tree();
        let mut result = String::new();

        parse_rec(source, tree.first().unwrap(), &mut result, 0);
        println!("{result}");
        (result, errors)
    }

    #[track_caller]
    fn parse(source: &str, f: impl FnOnce(&mut Parser)) -> String {
        let (result, errs) = try_parse(source, f);
        if !errs.is_empty() {
            let mut output = String::new();
            for err in errs {
                output.push_str(&format!("{:?}, token: {}", err.kind, &source[err.range]));
                output.push('\n');
            }
            panic!("{output}");
        }
        result
    }

    #[test]
    fn module() {
        insta::assert_snapshot!(parse("fn some_func(){}", |p| p.module()));
        insta::assert_snapshot!(parse(
            "struct X {
        }
        impl X {
              fn test() {}
        }",
            |p| p.module()
        ));
    }

    #[test]
    fn mod_item() {
        insta::assert_snapshot!(parse("mod my_mod { fn some_item() {} }", |p| p.mod_item()));
        insta::assert_snapshot!(parse("mod my_mod;", |p| p.mod_item()));
    }

    #[test]
    fn impl_item() {
        insta::assert_snapshot!(parse(
            "impl Debug for Vec2 {
                fn debug_fmt(self, f: Formatter) {
                }
            }",
            |p| p.impl_item()
        ));
        insta::assert_snapshot!(parse(
            "impl Vec2 {
                fn length();
                fn unit(self) -> Self; 
            }",
            |p| p.impl_item()
        ));
        insta::assert_snapshot!(parse(
            "impl Vec2 {
            }",
            |p| p.impl_item()
        ));
        insta::assert_snapshot!(parse(
            "impl<A: C + B, D> Generic for MyType<A,B> {
            }",
            |p| p.impl_item(),
        ));
    }

    #[test]
    fn fn_item() {
        insta::assert_snapshot!(parse("fn test(a: int, b: string)->int { stmt; }", |p| p.fn_item()));
        insta::assert_snapshot!(parse(
            "fn test() {
              let x: string = { 10           };
        }",
            |p| p.fn_item()
        ));
        insta::assert_snapshot!(parse("fn identity<T>() -> T { }", |p| p.fn_item()));
        insta::assert_snapshot!(parse("fn some_fuc(@label(not_a) a: int) { }", |p| p.fn_item()));
    }

    #[test]
    fn path() {
        insta::assert_snapshot!(parse("a : : b : : c", |p| p.expr_path()));
        insta::assert_snapshot!(parse("root::hello", |p| p.expr_path()));
        insta::assert_snapshot!(parse("hey<A,B,C>", |p| p.expr_path()));
        insta::assert_snapshot!(parse("hey<A,B,C>", |p| p.type_path()));
    }

    #[test]
    fn path_segment() {
        insta::assert_snapshot!(parse("A", |p| p.path_segment(PathKind::Expr)));
        insta::assert_snapshot!(parse("root", |p| p.path_segment(PathKind::Expr)));
        insta::assert_snapshot!(parse("super", |p| p.path_segment(PathKind::Expr)));
        insta::assert_snapshot!(parse("G<T1,T2>", |p| p.path_segment(PathKind::Expr)));
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
        insta::assert_snapshot!(parse("param: type,", |p| p.param()));
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
        insta::assert_snapshot!(parse("(dyn Debug + Foo<T> + Bar<X,Y>)?", |p| p.type_expr()));
        insta::assert_snapshot!(parse("Generic::Path<A,B>::Yay", |p| p.type_expr()));
        insta::assert_snapshot!(parse("(int, string, yo,)", |p| p.type_expr()));
        insta::assert_snapshot!(parse("((),)", |p| p.type_expr()));
    }

    #[test]
    fn item_type_expr() {
        insta::assert_snapshot!(parse(
            "struct Name {
            value: String
        }",
            |p| p.item_type_expr()
        ));
        insta::assert_snapshot!(parse(
            "enum Name {
                    value: String
                }
                ",
            |p| p.item_type_expr()
        ));
    }

    #[test]
    fn stmt_expr() {
        insta::assert_snapshot!(parse("a;", |p| {
            p.stmt_expr();
        }));
        insta::assert_snapshot!(parse("1+1;", |p| { p.stmt_expr() }));
        insta::assert_snapshot!(parse("print();", |p| { p.stmt_expr() }));
        insta::assert_snapshot!(parse("no_semi % idk;", |p| { p.stmt_expr() }));
        insta::assert_snapshot!(parse("vec2 { };", |p| { p.stmt_expr() }));
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
    fn pattern() {
        insta::assert_snapshot!(parse("ident", |p| p.pattern()));
        insta::assert_snapshot!(parse("_", |p| p.pattern()));
        insta::assert_snapshot!(parse("Color::Red", |p| p.pattern()));
        insta::assert_snapshot!(parse("10.1", |p| p.pattern()));
        insta::assert_snapshot!(parse("'hello there!'", |p| p.pattern()));
        insta::assert_snapshot!(parse("true", |p| p.pattern()));
    }

    #[test]
    fn generic_shenanigans() {
        insta::assert_snapshot!(parse("method.call<A,B>()", |p| p.expr()));
        insta::assert_snapshot!(parse("Vec<Vec<int>>::new()", |p| p.expr()));
        insta::assert_snapshot!(parse("BegoneTurbofish<Yay, Finally> {a:b,c:d}", |p| p.expr()));
        insta::assert_snapshot!(parse("let ty = Vec<int>;", |p| p.stmt_let()));
        insta::assert_snapshot!(parse("let ty = (Vec<int>);", |p| p.stmt_let()));
    }

    #[test]
    fn expr() {
        insta::assert_snapshot!(parse("1", |p| p.expr()));
        insta::assert_snapshot!(parse("1+1", |p| p.expr()));
        insta::assert_snapshot!(parse("1+1*3/4%3", |p| p.expr()));
        insta::assert_snapshot!(parse("1=2 or 3 and 4 == 5 != 6 < 7 + 8 * !true", |p| p.expr()));
        insta::assert_snapshot!(parse("(1)", |p| p.expr()));
        insta::assert_snapshot!(parse("1 + { 1 }", |p| p.expr()));
        insta::assert_snapshot!(parse("if !true {} else {}", |p| p.expr()));
        insta::assert_snapshot!(parse("if true {} else if VALUE { yo_mister_white }", |p| p.expr()));
        insta::assert_snapshot!(parse("\"a string\"", |p| p.expr()));
        insta::assert_snapshot!(parse("a[1](2)[3]", |p| p.expr()));
        insta::assert_snapshot!(parse("a[1] = b = c", |p| p.expr()));
        insta::assert_snapshot!(parse("sort(array, by: callback, something_else:)", |p| p.expr()));
        insta::assert_snapshot!(parse("|x,y: int| lua { #return x+y; }", |p| p.expr()));
        insta::assert_snapshot!(parse("()", |p| p.expr()));
        insta::assert_snapshot!(parse("1.abs()", |p| p.expr()));
        insta::assert_snapshot!(parse("pos[1][2].test().test.len()[0]", |p| p.expr()));
        insta::assert_snapshot!(parse("math::Vec2 { x: 1, y: 2, }", |p| p.expr()));
        insta::assert_snapshot!(parse("Vec2 {}", |p| p.expr()));
        insta::assert_snapshot!(parse("(20 as float) as int", |p| p.expr()));
        insta::assert_snapshot!(parse("generic_call<A>()", |p| p.expr()));
        insta::assert_snapshot!(parse("obj.generic_method<A>()", |p| p.expr()));
        insta::assert_snapshot!(parse("foo?.bar?.baz()", |p| p.expr()));
        insta::assert_snapshot!(parse("x is value and x !is value", |p| p.expr()));
        insta::assert_snapshot!(parse("std::Vec<int>::new()", |p| p.expr()));
        //this also parses as "((std::Vec::new < 1) + 2) > ()"
        insta::assert_snapshot!(parse("std::Vec::new<1 + 2>()", |p| p.expr()));
        insta::assert_snapshot!(parse("(1,2,3)", |p| p.expr()));
        insta::assert_snapshot!(parse("((),)", |p| p.expr()));
        insta::assert_snapshot!(parse("Foo<i32>::bar<string>()", |p| p.expr()));
        insta::assert_snapshot!(parse("for x in range(1,2) { print(x); }", |p| p.expr()));
        insta::assert_snapshot!(parse("while true { idk(); }", |p| p.expr()));
        insta::assert_snapshot!(parse("loop { panic(); }", |p| p.expr()));
    }

    #[test]
    fn enum_item() {
        insta::assert_snapshot!(parse(
            "enum MyEnum {
                    A = 0,
                    B,
                    C,
                    foo: Foo,
                    bar: Bar,
                    fn test(self) -> FooBar {
                        self.foo + self.bar
                    }
        }",
            |p| p.enum_item()
        ));
    }

    #[test]
    fn use_item() {
        insta::assert_snapshot!(parse("use foo::bar::baz;", |p| p.use_item()));
        insta::assert_snapshot!(parse("use foo::{bar,baz,*,self};", |p| p.use_item()));
        insta::assert_snapshot!(parse("use root::test;", |p| p.use_item()));
        insta::assert_snapshot!(parse("use super::test;", |p| p.use_item()));
    }

    #[test]
    fn struct_item() {
        insta::assert_snapshot!(parse("struct Vec2 {x: Y, y: Y }", |p| p.struct_item()));
        insta::assert_snapshot!(parse(
            "struct MyStruct: Parent {
                    foo: Foo,
                    bar: Bar,
                    fn test(self) -> FooBar {
                        self.foo + self.bar
                    }
                }
                ",
            |p| p.struct_item()
        ));
        insta::assert_snapshot!(parse("struct X { a: root::a::b::c }", |p| p.struct_item()));
        insta::assert_snapshot!(parse("struct Generic<A,B,C> {}", |p| p.struct_item()));
        insta::assert_snapshot!(parse("struct Y { value: S<int,int,int> }", |p| p.struct_item()));
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
        insta::assert_snapshot!(parse("lua {}", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse(
            "lua { x = { [1] = 1, 2, 3, a=1,b=3, [3]=1 }; }",
            |p| p.lua_block_expr()
        ));
        insta::assert_snapshot!(parse(
            "lua { function func(a,b,c) a = 1; b = 2; end }",
            |p| p.lua_block_expr()
        ));
        insta::assert_snapshot!(parse(
            "lua { function test.func(a,b,c) a = 1; b = 2; end }",
            |p| p.lua_block_expr()
        ));
        insta::assert_snapshot!(parse(
            "lua { function test:func(a,b,c) a = 1; b = 2; end }",
            |p| p.lua_block_expr()
        ));
        insta::assert_snapshot!(parse("lua { local string = [[ hello there ]]; }", |p| p
            .lua_block_expr()));
        insta::assert_snapshot!(parse("lua { call(1)[1.5](2)[2.5](3); }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { x = vec2.x.y().y; }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { x.y = 1; }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { print('1'); }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { x.y()().x.x().y[1][2](),x = 1,2; }", |p| p
            .lua_block_expr()));
        insta::assert_snapshot!(parse("lua { return 1,2,3; }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { #return 1,2,3; }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { break; }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { local a,b,c = 1, 'string', nil; }", |p| p
            .lua_block_expr()));
        insta::assert_snapshot!(parse("lua { a,b = 1,2; }", |p| p.lua_block_expr()));
        insta::assert_snapshot!(parse("lua { while true do print(); break; end  }", |p| p
            .lua_block_expr()));
        insta::assert_snapshot!(parse("lua { repeat print(); until false }", |p| {
            p.lua_block_expr();
        }));
        insta::assert_snapshot!(parse("lua { print(...); }", |p| {
            p.lua_block_expr();
        }));
        insta::assert_snapshot!(parse(
            "lua {
                if true then
                    yan();
                elseif false then
                    yay();
                else
                    nay();
                end
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                if true then
                    yan();
                elseif false then
                    yay();
                end
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                if true then
                    yan();
                end
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                for i=1,2,3 do end
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                for i = 1,10 do end
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                for a,b,c,d in {} do end
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                local x = function() end;
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse(
            "lua {
                local a = 1;
                local b = 2;
                #output = b;
                print(value[#output]);
            }",
            |p| {
                p.lua_block_expr();
            }
        ));
        insta::assert_snapshot!(parse("lua { local x = 1 ^ 2 ^ 3; }", |p| p.lua_block_expr()));
    }
}
