use crate::{
    common::*,
    position::{self, WithSpan},
    token,
};

pub type Identifier = String;

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpr {
    pub left: Box<WithSpan<Expr>>,
    pub right: Box<WithSpan<Expr>>,
    pub op: WithSpan<BinaryOp>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<WithSpan<Expr>>,
    pub then_branch: Vec<WithSpan<Stmt>>,
    pub else_branch: Option<Box<WithSpan<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnExpr {
    name: WithSpan<Identifier>,
    // args:
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Number {
    Float(f64),
    Int(i64),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Nil,
    Number(Number),
    Bool(bool),
    String(String),
    Grouping(Box<WithSpan<Expr>>),
    Unary(WithSpan<UnaryOp>, Box<WithSpan<Expr>>),
    Binary(BinaryExpr),
    Identifier(Identifier),
    Assign(WithSpan<String>, Box<WithSpan<Expr>>),
    Call(Box<WithSpan<Expr>>, Vec<WithSpan<Expr>>),
    If(IfExpr),
    Block(Vec<WithSpan<Stmt>>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Binding {
    pub binding_type: BindingType,
    pub identifiers: Vec<WithSpan<Identifier>>,
    pub values: Option<Vec<WithSpan<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item {}

#[derive(Debug, Clone, PartialEq)]
pub struct StmtExpr {
    pub expr: Box<Expr>,
    pub semi: Option<position::Span>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Expr(StmtExpr),
    Item(Item),
    Binding(Binding),
    Print(Box<WithSpan<Expr>>),
}
