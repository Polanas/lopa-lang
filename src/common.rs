use crate::token;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UnaryOp {
    Not,
    Negate,
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

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mult => write!(f, "*"),
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::GreaterEqual => write!(f, ">="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::LessEqual => write!(f, "<="),
            BinaryOp::NotEqual => write!(f, "!="),
            BinaryOp::Equal => write!(f, "=="),
            BinaryOp::Modulo => write!(f, "%"),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
        }
    }
}

impl BinaryOp {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Local,
    Global,
}
