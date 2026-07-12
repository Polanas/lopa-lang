#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum LitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum ItemKind {
    Function,
    Mod,
    Impl,
    Struct,
    Enum,
    Use,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum LuaLitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
}

impl LitKind {
    pub fn as_str(&self) -> &str {
        match self {
            LitKind::Float => "float",
            LitKind::Int => "int",
            LitKind::String => "string",
            LitKind::Bool => "bool",
            LitKind::Nil => "nil",
        }
    }
}

#[macro_export]
macro_rules! B {
    (+) => {
        $crate::parsing::ast::BinaryOpKind::Add
    };
    (-) => {
        $crate::parsing::ast::BinaryOpKind::Sub
    };
    (*) => {
        $crate::parsing::ast::BinaryOpKind::Mul
    };
    (/) => {
        $crate::parsing::ast::BinaryOpKind::Div
    };
    ("//") => {
        $crate::parsing::ast::BinaryOpKind::DivInt
    };
    (%) => {
        $crate::parsing::ast::BinaryOpKind::Rem
    };
    (or) => {
        $crate::parsing::ast::BinaryOpKind::Or
    };
    (<<) => {
        $crate::parsing::ast::BinaryOpKind::Shl
    };
    (>>) => {
        $crate::parsing::ast::BinaryOpKind::Shr
    };
    (^) => {
        $crate::parsing::ast::BinaryOpKind::BitXor
    };
    (&) => {
        $crate::parsing::ast::BinaryOpKind::BixAdd
    };
    (>) => {
        $crate::parsing::ast::BinaryOpKind::Greater
    };
    (>=) => {
        $crate::parsing::ast::BinaryOpKind::GreaterEqual
    };
    (<) => {
        $crate::parsing::ast::BinaryOpKind::Less
    };
    (<=) => {
        $crate::parsing::ast::BinaryOpKind::LessEqual
    };
    (!=) => {
        $crate::parsing::ast::BinaryOpKind::NotEqual
    };
    (==) => {
        $crate::parsing::ast::BinaryOpKind::Equal
    };
    (and) => {
        $crate::parsing::ast::BinaryOpKind::And
    };
    (|) => {
        $crate::parsing::ast::BinaryOpKind::BitOr
    };
    (+=) => {
        $crate::parsing::ast::BinaryOpKind::AddAssign
    };
    (-=) => {
        $crate::parsing::ast::BinaryOpKind::SubAssign
    };
    (*=) => {
        $crate::parsing::ast::BinaryOpKind::MulAssign
    };
    (/=) => {
        $crate::parsing::ast::BinaryOpKind::DivAssign
    };
    ("//=") => {
        $crate::parsing::ast::BinaryOpKind::DivIntAssign
    };
    (%=) => {
        $crate::parsing::ast::BinaryOpKind::RemAssign
    };
    (^=) => {
        $crate::parsing::ast::BinaryOpKind::BitXorAssign
    };
    (&=) => {
        $crate::parsing::ast::BinaryOpKind::BitAndAssign
    };
    (|=) => {
        $crate::parsing::ast::BinaryOpKind::BitOrAssign
    };
    (<<=) => {
        $crate::parsing::ast::BinaryOpKind::ShlAssign
    };
    (>>=) => {
        $crate::parsing::ast::BinaryOpKind::ShrAssign
    };
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    DivInt,
    Rem,
    Or,
    Shl,
    Shr,
    BitXor,
    BitAnd,
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum LuaBinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Or,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    NotEqual,
    Equal,
    And,
    Concat,
    Exp,
}

impl BinaryOpKind {
    pub fn is_arithmetic(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::DivInt | Self::Div | Self::Mul | Self::Rem
        )
    }
}

impl std::fmt::Display for BinaryOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOpKind::Add => write!(f, "+"),
            BinaryOpKind::Mul => write!(f, "*"),
            BinaryOpKind::Div => write!(f, "/"),
            BinaryOpKind::DivInt => write!(f, "//"),
            BinaryOpKind::Rem => write!(f, "%"),
            BinaryOpKind::Or => write!(f, "or"),
            BinaryOpKind::Shl => write!(f, "<<"),
            BinaryOpKind::Shr => write!(f, ">>"),
            BinaryOpKind::BitXor => write!(f, "^"),
            BinaryOpKind::BitAnd => write!(f, "&"),
            BinaryOpKind::Sub => write!(f, "-"),
            BinaryOpKind::Greater => write!(f, ">"),
            BinaryOpKind::GreaterEqual => write!(f, ">="),
            BinaryOpKind::Less => write!(f, "<"),
            BinaryOpKind::LessEqual => write!(f, "<="),
            BinaryOpKind::NotEqual => write!(f, "!="),
            BinaryOpKind::Equal => write!(f, "=="),
            BinaryOpKind::And => write!(f, "and"),
            BinaryOpKind::BitOr => write!(f, "|"),
            BinaryOpKind::AddAssign => write!(f, "+="),
            BinaryOpKind::SubAssign => write!(f, "-="),
            BinaryOpKind::MulAssign => write!(f, "*="),
            BinaryOpKind::DivAssign => write!(f, "/="),
            BinaryOpKind::DivIntAssign => write!(f, "//="),
            BinaryOpKind::RemAssign => write!(f, "%="),
            BinaryOpKind::BitXorAssign => write!(f, "^="),
            BinaryOpKind::BitAndAssign => write!(f, "&="),
            BinaryOpKind::BitOrAssign => write!(f, "|="),
            BinaryOpKind::ShlAssign => write!(f, "<<="),
            BinaryOpKind::ShrAssign => write!(f, ">>="),
        }
    }
}

#[macro_export]
macro_rules! U {
    (not) => {
        $crate::parsing::ast::UnaryOpKind::Not
    };
    (neg) => {
        $crate::parsing::ast::UnaryOpKind::Neg
    };
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

impl std::fmt::Display for UnaryOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnaryOpKind::Not => write!(f, "!"),
            UnaryOpKind::Neg => write!(f, "-"),
        }
    }
}
