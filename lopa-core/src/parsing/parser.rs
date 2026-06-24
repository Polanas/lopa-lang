use std::{cell::Cell, iter::Peekable, ops::Range};

use crate::{
    ide::TextRange,
    parsing::{
        lexer::{self, LexToken, Lexer},
        token_set::TokenSet,
    },
};

use super::lexer::Syntax::{self, *};
use itertools::Itertools;
use rowan::{GreenNode, GreenNodeBuilder, TextSize};

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

pub fn parse(input: &str) -> (GreenNode, Vec<ParseError>) {
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
    pub range: TextRange,
    pub kind: SyntaxErrorKind,
}

impl ParseError {
    pub fn new(range: TextRange, kin: SyntaxErrorKind) -> Self {
        Self { range, kind: kin }
    }
}

//TODO: figure out what's wrong with the parser.
struct Parser<'a> {
    tokens: Vec<LexToken<'a>>,
    pos: usize,
    fuel: Cell<u32>,
    events: Vec<Event>,
    errors: Vec<ParseError>,
    input: &'a str,
    tokens_raw: Peekable<lexer::Lexer<'a>>,
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
        }
    }
}

struct Builder<'a> {
    builder: GreenNodeBuilder<'static>,
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
                self.builder.token(next.token.into(), next.text);
            }
        }
    }

    fn build_tree(mut self, events: &[Event]) -> (GreenNode, Vec<ParseError>) {
        let mut events_iter = events.iter().peekable();
        while let Some(event) = events_iter.next() {
            match event {
                Event::Open { node } => {
                    //we don't want to insert tokens before the first node (fixes a crash on
                    //builder.finish()
                    if node != &MODULE {
                        self.skip_whitespace();
                    }
                    self.builder.start_node((*node).into());
                }
                Event::Close => {
                    self.builder.finish_node();
                }
                Event::Advance { token } => {
                    self.skip_whitespace();
                    let Some(next) = self.tokens_raw.next() else {
                        continue;
                    };
                    self.builder.token((*token).into(), next.text);

                    if !matches!(events_iter.peek(), Some(Event::Close)) {
                        self.skip_whitespace();
                    }
                }
                Event::Checkpoint => {}
            }
        }
        (self.builder.finish(), self.errors)
    }
}

impl<'a> Parser<'a> {
    fn build_tree(self) -> (GreenNode, Vec<ParseError>) {
        Builder {
            builder: GreenNodeBuilder::new(),
            tokens_raw: self.tokens_raw,
            errors: self.errors,
        }
        .build_tree(&self.events)
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
            .map(|&LexToken { range, .. }| range)
            .unwrap_or_else(|| TextRange::empty(TextSize::from(self.input.len() as u32)));
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
            .map(|t| u32::from(t.range.start())..u32::from(t.range.end()))
            .map(|r| (r.start as usize)..r.end as usize)
            .unwrap_or_else(|| {
                let len = self.input.len();

                len..len
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
            if this.ate(T![:]) {
                this.parents_list();
            }
            this.enum_elem_list();
        });
    }

    fn enum_elem_list(&mut self) {
        self.expect(T!["{"]);
        while !self.at(T!["}"]) && !self.at(EOF) {
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
            self.field();
            if !self.at(T!["}"]) {
                self.expect(T![,]);
            }
        }
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
                this.with(IMPL_STRUCT_TYPE, |this| {
                    this.type_expr();
                });
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
            if this.ate(T![:]) {
                this.parents_list();
            }
            this.struct_elem_list();
        })
    }

    fn struct_elem_list(&mut self) {
        self.expect(T!["{"]);
        while !self.at(T!["}"]) && !self.at(EOF) {
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

    fn parents_list(&mut self) {
        self.with(PARENTS_LIST, |this| {
            while !this.at(T!["{"]) && !this.at(EOF) {
                if this.at(IDENT) {
                    this.name();
                    if !this.at(T!["{"]) {
                        this.expect(T![,]);
                    }
                } else {
                    if this.at_any(ARG_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_error(SyntaxErrorKind::ExpectedParent);
                }
            }
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
            while !this.at(T!["}"]) && !this.at(EOF) {
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
                } else {
                    if this.at_any(PARAM_LIST_RECOVERY) {
                        break;
                    }
                    this.advance_with_error(SyntaxErrorKind::ExpectedParameter);
                }
            }
            this.expect(T!(")"));
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
            if !this.at(T![")"]) {
                this.expect(T![,]);
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
                T![struct] => self.with(ITEM_TYPE_STRUCT, |this| this.struct_item()),
                T![enum] => self.with(ITEM_TYPE_ENUM, |this| this.enum_item()),
                _ => self.with(ITEM_TYPE, |this| this.type_expr()),
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
                    this.type_path();
                }),
                T!["("] => {
                    self.expect(T!["("]);
                    if self.at(T![")"]) {
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
                    let next_span = self.nth_span(0);
                    let can_be_lit = self.at_path_sep(1);
                    self.with(PATH_TYPE, |this| {
                        this.type_path();
                    });
                    if !can_be_lit {
                        match &self.input[next_span] {
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
            self.advance_with_error(SyntaxErrorKind::ExpectedType);
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

            while this.at_path_sep(0) && !this.at(EOF) {
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

    fn path_segment(&mut self, kind: PathKind) {
        self.with(PATH_SEGMENT, |this| {
            if this.at(T![root]) {
                this.expect(T![root]);
            } else if this.at(T![super]) {
                this.expect(T![super]);
            } else {
                this.expect(IDENT);
                match kind {
                    PathKind::Type => {
                        if this.at(T![<]) {
                            this.generic_args();
                        }
                    }
                    PathKind::Expr => {
                        if this.at_path_sep(0) && this.nth(2) == T![<] {
                            this.expect(T![:]);
                            this.expect(T![:]);
                            this.generic_args();
                        }
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
            while !this.at(T![")"]) && !this.at(EOF) {
                if this.at_any(TYPE_FIRST) || this.at(IDENT) {
                    this.fn_type_param();
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
            if !this.at(T![")"]) {
                this.expect(T![,]);
            }
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
                    self.with_at(PAREN_EXPR, checkpoint, |this| {
                        this.expr();
                        this.expect(T![")"]);
                    });
                }
            }
            T!["{"] => {
                self.block();
            }
            // T![lua] => {
            //     self.lua_block();
            // }
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
        loop {
            match self.peek() {
                T!["("] => {
                    self.with_at(CALL_EXPR, checkpoint, |this| this.arg_list());
                }
                T!["["] => {
                    self.with_at(INDEX_EXPR, checkpoint, |this| this.index());
                }
                T![?] | T![.] => {
                    let safe_call = self.ate(T![?]);
                    if self.nth(2) == T!["("] || self.nth(2) == T![:] {
                        let method_expr = |this: &mut Self, syntax: Syntax| {
                            this.with_at(syntax, checkpoint, |this| {
                                this.expect(T![.]);
                                this.name();
                                if this.ate(T![:]) {
                                    this.expect(T![:]);
                                    this.generics();
                                }
                                this.arg_list();
                            })
                        };
                        if safe_call {
                            self.with_at(METHOD_EXPR, checkpoint, |this| {
                                method_expr(this, SAFE_METHOD_EXPR)
                            });
                        } else {
                            method_expr(self, METHOD_EXPR);
                        }
                    } else {
                        let field_expr = |this: &mut Self, syntax: Syntax| {
                            this.with_at(syntax, checkpoint, |this| {
                                this.expect(T![.]);
                                this.name();
                            })
                        };
                        if safe_call {
                            self.with_at(FIELD_EXPR, checkpoint, |this| {
                                field_expr(this, SAFE_FIELD_EXPR)
                            });
                        } else {
                            field_expr(self, FIELD_EXPR);
                        }
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

    fn record_field_list(&mut self) {
        self.expect(T!["{"]);
        while !self.at(T!["}"]) && !self.at(EOF) {
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
            while !this.at(T![|]) && !this.at(EOF) {
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
        while !self.at(T![")"]) && !self.at(EOF) {
            if self.at_any(EXPR_FIRST) {
                self.arg();
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
            if !this.at(T![")"]) {
                this.expect(T![,]);
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
                } else {
                    if this.at_any(GENERICS_RECOVERY) {
                        break;
                    } else {
                        this.advance_with_error(SyntaxErrorKind::ExpectedGeneric);
                    }
                }
                if !this.at(T![>]) {
                    this.expect(T![,]);
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

            while !this.at(T![")"]) && !this.at(EOF) {
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
    use crate::parsing::{
        ast::SyntaxNode,
        lexer::Syntax,
        parser::{Lang, Parser, PathKind},
    };

    use rowan::{GreenNodeBuilder, NodeOrToken, SyntaxKind, SyntaxToken};

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
                NodeOrToken::Token(t) if t.kind() != Syntax::WHITESPACE =>
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
    fn try_parse(source: &str, f: impl FnOnce(&mut Parser)) -> (String, Vec<super::ParseError>) {
        let mut parser = Parser::new(source);
        f(&mut parser);
        let (node, errors) = parser.build_tree();
        let node = SyntaxNode::new_root(node);
        let mut result = String::new();

        parse_rec(NodeOrToken::Node(node), &mut result, 0);
        println!("{result}");
        (result, errors)
    }

    #[track_caller]
    fn parse(source: &str, f: impl FnOnce(&mut Parser)) -> String {
        let (result, errs) = try_parse(source, f);
        if !errs.is_empty() {
            let mut output = String::new();
            for err in errs {
                let range =
                    (u32::from(err.range.start()) as usize)..(u32::from(err.range.end()) as usize);
                output.push_str(&format!("{:?}, token: {}", err.kind, &source[range]));
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
        insta::assert_snapshot!(parse("hey::<A,B,C>", |p| p.expr_path()));
        insta::assert_snapshot!(parse("hey<A,B,C>", |p| p.type_path()));
    }

    #[test]
    fn path_segment() {
        insta::assert_snapshot!(parse("A", |p| p.path_segment(PathKind::Expr)));
        insta::assert_snapshot!(parse("root", |p| p.path_segment(PathKind::Expr)));
        insta::assert_snapshot!(parse("super", |p| p.path_segment(PathKind::Expr)));
        insta::assert_snapshot!(parse("G::<T1,T2>", |p| p.path_segment(PathKind::Expr)));
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
        insta::assert_snapshot!(parse("(dyn Debug)?", |p| p.type_expr()));
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
        // insta::assert_snapshot!(parse("|x,y: int| lua {x+y}", |p| p.expr()));
        insta::assert_snapshot!(parse("()", |p| p.expr()));
        insta::assert_snapshot!(parse("1.abs()", |p| p.expr()));
        insta::assert_snapshot!(parse("pos[1][2].test().test.len()[0]", |p| p.expr()));
        insta::assert_snapshot!(parse("math::Vec2 { x: 1, y: 2, }", |p| p.expr()));
        insta::assert_snapshot!(parse("Vec2 {}", |p| p.expr()));
        insta::assert_snapshot!(parse("(20 as float) as int", |p| p.expr()));
        insta::assert_snapshot!(parse("generic_call::<A>()", |p| p.expr()));
        insta::assert_snapshot!(parse("obj.generic_method::<A>()", |p| p.expr()));
        insta::assert_snapshot!(parse("foo?.bar?.baz()", |p| p.expr()));
        insta::assert_snapshot!(parse("x is value and x !is value", |p| p.expr()));
        insta::assert_snapshot!(parse("std::Vec::<int>::new()", |p| p.expr()));
    }

    #[test]
    fn enum_item() {
        insta::assert_snapshot!(parse(
            "enum MyEnum: Parent1, Parent2 {
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
            "struct MyStruct: Parent1, Parent2 {
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
    fn temp() {
        println!(
            "{}",
            parse(
                "fn test(a b c) {}",
                |p| p.module()
            )
        );
    }

    #[test]
    fn lua() {
        // insta::assert_snapshot!(parse("lua {}", |p| p.lua_block()));
        // insta::assert_snapshot!(parse(
        //     "lua { x = { [1] = 1, 2, 3, a=1,b=3, [3]=1 }; }",
        //     |p| p.lua_block()
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua { function func(a,b,c) a = 1; b = 2; end }",
        //     |p| p.lua_block()
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua { function test.func(a,b,c) a = 1; b = 2; end }",
        //     |p| p.lua_block()
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua { function test:func(a,b,c) a = 1; b = 2; end }",
        //     |p| p.lua_block()
        // ));
        // insta::assert_snapshot!(parse("lua { call(1)[1.5](2)[2.5](3); }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { local string = [[ hello there ]]; }", |p| p
        //     .lua_block()));
        // insta::assert_snapshot!(parse("lua { x = vec2.x.y; }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { x.y = 1; }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { print('1'); }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { x.y()().x.x().y[1][2](),x = 1,2; }", |p| p
        //     .lua_block()));
        // insta::assert_snapshot!(parse("lua { return 1,2,3; }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { break; }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { local a,b,c = 1, 'string', nil; }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { a,b = 1,2; }", |p| p.lua_block()));
        // insta::assert_snapshot!(parse("lua { while true do print(); break; end  }", |p| p
        //     .lua_block()));
        // insta::assert_snapshot!(parse("lua { print(...); }", |p| {
        //     p.lua_block();
        // }));
        // insta::assert_snapshot!(parse("lua { repeat print() until false }", |p| {
        //     p.lua_block();
        // }));
        // insta::assert_snapshot!(parse(
        //     "lua {
        //         if true then
        //             yan()
        //         elseif false then
        //             yay()
        //         else
        //             nay()
        //         end
        //     }",
        //     |p| {
        //         p.lua_block();
        //     }
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua {
        //         for i=1,2,3 do end
        //     }",
        //     |p| {
        //         p.lua_block();
        //     }
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua {
        //         for i = 1,10 do end
        //     }",
        //     |p| {
        //         p.lua_block();
        //     }
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua {
        //         for a,b,c,d in {} do end
        //     }",
        //     |p| {
        //         p.lua_block();
        //     }
        // ));
        // insta::assert_snapshot!(parse(
        //     "lua {
        //         local x = function() end;
        //     }",
        //     |p| {
        //         p.lua_block();
        //     }
        // ));
        // println!("------------------------------");
        // insta::assert_snapshot!(parse("lua{1+2}", |p| p.lua_block()));
    }
}
