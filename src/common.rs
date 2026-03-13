use crate::token;

#[derive(Clone, Debug, PartialEq, Eq, Copy, PartialOrd, Ord)]
pub enum Primitive {
    Nil,
    Bool,
    Int,
    Float,
    String,
    Any,
}

impl Primitive {
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }
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
    Value,
    C,
}

impl StructKind {
    pub const KIND_GC: &str = "gc";
    pub const KIND_VALUE: &str = "value";
    pub const KIND_C: &str = "C";
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum UnaryOp {
    Not,
    Negate,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum BinaryOp {
    Add,
    Mul,
    Div,
    DivInt,
    Rem,
    Or,
    Shl,
    Shr,
    BitXor,
    BitAnd,
    Sub,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    And,
    BitOr,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    DivIntAssign,
    RemAssign,
    BitXorAssign,
    BitAndAssign,
    BitOrAssign,
    ShlAssign,
    ShrAssign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Local,
    Global,
}
