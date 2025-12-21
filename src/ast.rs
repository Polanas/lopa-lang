use crate::{
    position::{self, WithSpan},
    token,
};

pub type Identifier = String;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UnaryOp {
    Not,
    Minus,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BinaryOp {
    Div,
    Mult,
    Add,
    Sub,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    Modulo,
    And,
    Or,
}

impl BinaryOp {
    pub fn to_lua(&self) -> &str {
        match self {
            BinaryOp::Div => "/",
            BinaryOp::Mult => "*",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::NotEqual => "~=",
            BinaryOp::Equal => "==",
            BinaryOp::Modulo => "%",
            BinaryOp::And => "and",
            BinaryOp::Or => "||",
        }
    }
    pub fn from_token(token: &token::Token) -> Option<Self> {
        match *token {
            token::Token::Slash => Some(Self::Div),
            token::Token::Star => Some(Self::Mult),
            token::Token::Plus => Some(Self::Add),
            token::Token::Minus => Some(Self::Sub),
            token::Token::Greater => Some(Self::Greater),
            token::Token::GreaterEqual => Some(Self::GreaterEqual),
            token::Token::Less => Some(Self::Less),
            token::Token::LessEqual => Some(Self::LessEqual),
            token::Token::BangEqual => Some(Self::NotEqual),
            token::Token::Equal2 => Some(Self::Equal),
            token::Token::Percent => Some(Self::Modulo),
            _ => None,
        }
    }
}

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
pub enum Assign {
    Binding(WithSpan<String>, Box<WithSpan<Expr>>),
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
    Assign(Vec<Assign>),
    Call(Box<WithSpan<Expr>>, Vec<WithSpan<Expr>>),
    If(IfExpr),
    Block(Vec<WithSpan<Stmt>>),
    Multivalue(Box<WithSpan<Expr>>, Box<WithSpan<Expr>>),
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum BindingType {
    Local,
    Global,
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
pub fn flatten_multivalue(expr: WithSpan<Expr>) -> (Vec<WithSpan<Expr>>, position::Span) {
    let Expr::Multivalue(mut head, first) = expr.value else {
        let span = expr.span;
        return (vec![expr], span);
    };
    let mut span = first.span;
    let mut values = vec![*first];

    while let WithSpan {
        value: Expr::Multivalue(next, current),
        span: current_span,
    } = *head
    {
        values.push(*current.clone());
        span = span.union(current_span);
        head = next.clone();
    }
    values.push(*head);
    values.reverse();

    (values, span)
}
