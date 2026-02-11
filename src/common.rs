use crate::token;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Primitive {
    Nil,
    Bool,
    Int,
    Float,
    String,
    Any,
}

impl std::fmt::Display for Primitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Primitive::Nil => "nil",
                Primitive::Bool => "bool",
                Primitive::Int => "int",
                Primitive::Float => "float",
                Primitive::String => "string",
                Primitive::Any => "any",
            }
        )
    }
}

impl Primitive {
    pub fn from_ident(ident: &str) -> Option<Self> {
        Some(match ident {
            "nil" => Primitive::Nil,
            "int" => Primitive::Int,
            "float" => Primitive::Float,
            "string" => Primitive::String,
            "bool" => Primitive::Bool,
            "any" => Primitive::Any,
            _ => {
                return None;
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StringKind {
    Regular,
    Multiline,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StructKind {
    GC,
    Native,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Copy)]
pub enum FnParamKind {
    Regular,
    Receiver,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BinaryOp {
    Div,
    DivInt,
    Mult,
    Add,
    Sub,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    Rem,
    And,
    Or,
    Else,
    BitXor,
    BitAnd,
    BitOr,
    Shl,
    Shr,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BinaryAssignOp {
    Add,
    Sub,
    Mul,
    Div,
    DivInt,
    Rem,
    BitXor,
    BitAnd,
    BitOr,
    Shl,
    Shr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Local,
    Global,
}
