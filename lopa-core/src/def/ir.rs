use la_arena::Idx;
use ustr::Ustr;

use crate::{
    ide,
    parsing::ast::{self, BinaryOpKind, LiteralKind, UnaryOpKind},
};

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub params: Vec<FnParam<'db>>,
    pub output: Option<TypeExpr>,
    pub ast_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
}

#[salsa::tracked(debug)]
pub struct FnParam<'db> {
    // pub name: Ustr,
    pub ty: TypeExpr,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum TypeExpr {
    Unknown,
    PathType(PathType),
    NilableType(NilableType),
    LitType(LitType),
    AnyType(AnyType),
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Path {
    pub segments: Vec<Ustr>,
}

#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct PathType {
    pub value: Path,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct NilableType {
    pub value: Box<TypeExpr>,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct LitType {
    pub kind: LiteralKind,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct AnyType {}

pub type ExprId = Idx<Expr>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Expr {
    Missing,
    Name(Ustr),
    Lit(LiteralKind),
    BlockExpr {
        stmts: Vec<Stmt>,
    },
    If {
        if_cond: ExprId,
        if_branch: Vec<Stmt>,
        else_branch: Option<ElseBranch>,
    },
    Unary {
        expr: ExprId,
        kind: UnaryOpKind,
    },
    Binary {
        left: ExprId,
        right: ExprId,
        kind: BinaryOpKind,
    },
    Return {
        expr: ExprId,
    },
    Index {
        base: ExprId,
        index: ExprId,
    },
    Call {
        func: ExprId,
        args: Vec<Arg>,
    },
    Paren {
        expr: ExprId,
    },
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ElseBranch {
    Else { stmts: Vec<Stmt> },
    ElseIf { expr: ExprId },
}

pub type PatternId = Idx<Pattern>;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Pattern {
    Missing,
    Name(Ustr),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Arg {
    Labeled { label: Ustr, value: ExprId },
    NonLabeled { value: ExprId },
}

impl Arg {
    pub fn value(&self) -> ExprId {
        match self {
            Arg::Labeled { value, .. } | Arg::NonLabeled { value } => *value,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Stmt {
    Let {
        pattern: PatternId,
        ty: Option<TypeExpr>,
        expr: ExprId,
    },
    Expr {
        expr: ExprId,
        semi: Option<()>,
    },
}
