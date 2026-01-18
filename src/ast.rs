use std::{collections::HashMap, fmt::Display};

use crate::{
    common::{self, *},
    position::{self, WithSpan},
    types,
};

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpr {
    pub left: Box<WithSpan<Expr>>,
    pub right: Box<WithSpan<Expr>>,
    pub op: WithSpan<BinaryOp>,
    pub types: Option<(Type, Type, Type)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<WithSpan<Expr>>,
    pub then_branch: WithSpan<Block>,
    pub else_branch: Option<Box<WithSpan<Expr>>>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Number {
    Float(f64),
    Int(i64),
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Float(float) => write!(f, "{float}"),
            Number::Int(i) => write!(f, "{i}"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct UnaryExpr {
    pub expr: Box<WithSpan<Expr>>,
    pub op: WithSpan<UnaryOp>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub body: Vec<WithSpan<Stmt>>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Arg {
    pub name: Option<Identifier>,
    pub expr: Box<WithSpan<Expr>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Call {
    pub callee: Box<WithSpan<Expr>>,
    pub callee_type: Option<Type>,
    pub args: Vec<Arg>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Closure {
    pub params: Vec<FnParam>,
    pub body: WithSpan<Block>,
    pub returns: Option<Vec<WithSpan<Type>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Nil,
    Number(Number),
    Bool(bool),
    String(common::StringKind, String),
    Grouping(Box<WithSpan<Expr>>),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Identifier(Identifier, Option<Type>),
    Call(Call),
    If(IfExpr),
    Block(Block),
    Closure(Closure),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Binding {
    pub kind: BindingKind,
    pub idents: Vec<WithSpan<Identifier>>,
    pub types: Vec<Option<WithSpan<Type>>>,
    pub values: Option<Vec<WithSpan<Expr>>>,
}

impl Binding {
    pub fn as_ref(&'_ self) -> BindingRef<'_> {
        BindingRef {
            kind: self.kind,
            idents: &self.idents,
            values: self.values.as_deref(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BindingRef<'a> {
    pub kind: BindingKind,
    pub idents: &'a [WithSpan<Identifier>],
    pub values: Option<&'a [WithSpan<Expr>]>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnParam {
    pub kind: FnParamKind,
    pub ty: WithSpan<Type>,
    pub name: WithSpan<Identifier>,
    pub default_value: Option<WithSpan<Expr>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Fn {
    pub name: Identifier,
    pub params: Vec<FnParam>,
    pub body: WithSpan<Block>,
    pub returns: Vec<WithSpan<Type>>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExternFn {
    pub name: Identifier,
    pub params: Vec<FnParam>,
    pub returns: Vec<WithSpan<Type>>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InlineFn {
    pub name: Identifier,
    pub params: Vec<FnParam>,
    pub body: String,
    pub returns: Vec<WithSpan<Type>>,
    pub ty: Option<Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExternKind {
    Lua,
    C,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Extern {
    pub kind: ExternKind,
    pub defs: Vec<WithSpan<ExternDefinition>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Inline {
    pub defs: Vec<WithSpan<InlineFn>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    pub ty: WithSpan<Type>,
    pub default_value: Option<WithSpan<Expr>>,
    pub name: Option<WithSpan<Identifier>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StructFields {
    Unit,
    Tuple(Vec<Field>),
    Named(Vec<Field>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Struct {
    pub name: WithSpan<Identifier>,
    pub kind: StructKind,
    pub fields: StructFields,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item {
    Fn(Fn),
    Extern(Extern),
    Inline(Inline),
    Struct(Struct),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExternDefinition {
    Fn(ExternFn),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StmtExpr {
    pub exprs: Vec<WithSpan<Expr>>,
    pub semi: Option<position::Span>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Assign {
    pub idents: Vec<WithSpan<Identifier>>,
    pub values: Option<Vec<WithSpan<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Expr(StmtExpr),
    Assign(Assign),
    Binding(Binding),
    Return(Vec<WithSpan<Expr>>),
    Empty,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AstType {
    pub kind: TypeKind,
    pub nilable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Ast(AstType),
    Checked(types::Type),
}

impl From<types::TypeKind> for Type {
    fn from(value: types::TypeKind) -> Self {
        Self::Checked(value.into())
    }
}

impl From<types::Type> for Type {
    fn from(value: types::Type) -> Self {
        Self::Checked(value)
    }
}

impl Type {
    pub fn checked(&self) -> Option<&types::Type> {
        match self {
            Type::Checked(checked) => Some(checked),
            _ => None
        }
    }
}

impl AstType {
    pub fn non_nilable(kind: TypeKind) -> Self {
        Self {
            kind,
            nilable: false,
        }
    }

    pub fn nilable(kind: TypeKind) -> Self {
        Self {
            kind,
            nilable: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnParamType {
    pub kind: common::FnParamKind,
    pub name: Option<Identifier>,
    pub ty: WithSpan<Type>,
    pub default_value: Option<()>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FnType {
    pub params: Vec<FnParamType>,
    pub returns: Vec<WithSpan<Type>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeKind {
    Fn(FnType),
    Path(WithSpan<Identifier>),
}
