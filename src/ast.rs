use crate::{position::WithSpan, token};

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
    Equal,
    Modulo,
    And,
    Or,
}

impl BinaryOperator {
    pub fn to_lua(&self) -> &str {
        match self {
            BinaryOperator::Div => "/",
            BinaryOperator::Mult => "*",
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Greater => ">",
            BinaryOperator::GreaterEqual => ">=",
            BinaryOperator::Less => "<",
            BinaryOperator::LessEqual => "<=",
            BinaryOperator::NotEqual => "~=",
            BinaryOperator::Equal => "==",
            BinaryOperator::Modulo => "%",
            BinaryOperator::And => "and",
            BinaryOperator::Or => "or",
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
    pub op: WithSpan<BinaryOperator>,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UnaryOp {
    Minus,
    Not,
}

impl UnaryOp {
    pub fn from_token(token: &token::Token) -> Option<Self> {
        match *token {
            token::Token::Minus => Some(Self::Minus),
            token::Token::Bang => Some(Self::Not),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfExpr {
    pub condition: Box<WithSpan<Expr>>,
    pub then_branch: Vec<WithSpan<Stmt>>,
    pub else_branch: Option<Vec<WithSpan<Stmt>>>,
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
    Assign(Vec<WithSpan<Identifier>>, Vec<WithSpan<Expr>>),
    Call(Box<WithSpan<Expr>>, Vec<WithSpan<Expr>>),
    If(IfExpr),
    Block(Vec<WithSpan<Stmt>>),
    List(Vec<WithSpan<Expr>>),
}

#[derive(Debug, PartialEq, Clone, Copy, Eq)]
pub enum BindingType {
    Let,
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

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    Expr(Box<Expr>),
    Item(Item),
    Binding(Binding),
    Print(Box<WithSpan<Expr>>),
}
