use std::{collections::HashMap, fmt::Display};

use crate::{
    common::*,
    position::{self, WithSpan},
    types,
};

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpr {
    pub left: Box<WithSpan<Expr>>,
    pub right: Box<WithSpan<Expr>>,
    pub op: WithSpan<BinaryOp>,
    pub types: Option<(types::Type, types::Type, types::Type)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<WithSpan<Expr>>,
    pub then_branch: WithSpan<Block>,
    pub else_branch: Option<Box<WithSpan<Expr>>>,
    pub ty: Option<types::Type>,
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
    pub ty: Option<types::Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub body: Vec<WithSpan<Stmt>>,
    pub ty: Option<types::Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Arg {
    Named(Identifier, Box<WithSpan<Expr>>),
    Ordered(Box<WithSpan<Expr>>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Call {
    pub callee: Box<WithSpan<Expr>>,
    pub args: Vec<Arg>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Nil,
    Number(Number),
    Bool(bool),
    String(String),
    Grouping(Box<WithSpan<Expr>>),
    Unary(UnaryExpr),
    Binary(BinaryExpr),
    Identifier(Identifier, Option<types::Type>),
    Call(Call),
    If(IfExpr),
    Block(Block),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Binding {
    pub kind: BindingKind,
    pub idents: Vec<WithSpan<Identifier>>,
    pub types: Vec<Option<WithSpan<types::Type>>>,
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
    pub ty: WithSpan<types::Type>,
    pub name: WithSpan<Identifier>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Fn {
    pub name: Identifier,
    pub params: Vec<FnParam>,
    pub body: WithSpan<Block>,
    pub returns: Vec<WithSpan<types::Type>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item {
    Fn(Fn),
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
    Item(Item),
    Assign(Assign),
    Binding(Binding),
    Print(Box<WithSpan<Expr>>),
    Return(Vec<WithSpan<Expr>>),
    Empty,
}
