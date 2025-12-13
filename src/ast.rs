use crate::position::WithSpan;

pub type Identifier = String;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UnaryOperator {
    Bang,
    Minus,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BinaryOperator {
    Div,
    Mult,
    Add,
    Sub,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    EqualEqual,
    Modulo,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, PartialEq, Clone)]
pub struct BinaryExpr {
    left: Box<WithSpan<Expr>>,
    right: Box<WithSpan<Expr>>,
    op: WithSpan<BinaryOperator>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LogicalExpr {
    left: Box<WithSpan<Expr>>,
    right: Box<WithSpan<Expr>>,
    op: WithSpan<LogicalOperator>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    condition: Box<WithSpan<Expr>>,
    then_branch: Vec<WithSpan<Stmt>>,
    else_branch: Option<Vec<WithSpan<Stmt>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct FnExpr {
    name: WithSpan<Identifier>,
    // args:
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Nil,
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
    Grouping(Box<WithSpan<Expr>>),
    Unary(Box<WithSpan<Expr>>, Box<WithSpan<Expr>>),
    Binary(BinaryExpr),
    Variable(WithSpan<Identifier>),
    Logical(LogicalExpr),
    Assign(Vec<WithSpan<Identifier>>, Vec<WithSpan<Expr>>),
    Call(Box<WithSpan<Expr>>, Vec<WithSpan<Expr>>),
    If(IfExpr),
    Block(Vec<WithSpan<Stmt>>),
    List(Vec<WithSpan<Expr>>),
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum VariableDefType {
    Let,
    Global,
}

#[derive(Debug, PartialEq, Clone)]
pub struct VariableDef {
    def_type: VariableDefType,
    names: Vec<WithSpan<Identifier>>,
    values: Option<Vec<WithSpan<Expr>>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Item {}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Expr(Box<WithSpan<Expr>>),
    Item(Item),
    VariableDef(VariableDef),
}
