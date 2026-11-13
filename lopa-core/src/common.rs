#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::SalsaValue)]
pub enum LitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::SalsaValue)]
pub enum TypeLitKind {
    Float,
    Int,
    String,
    Bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::SalsaValue)]
pub enum ItemKind {
    Function,
    Mod,
    Impl,
    Struct,
    Enum,
    Use,
    Class,
    Trait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::SalsaValue)]
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
        $crate::common::BinaryOpKind::Add
    };
    (-) => {
        $crate::common::BinaryOpKind::Sub
    };
    (*) => {
        $crate::common::BinaryOpKind::Mul
    };
    (/) => {
        $crate::common::BinaryOpKind::Div
    };
    ("//") => {
        $crate::common::BinaryOpKind::DivInt
    };
    (%) => {
        $crate::common::BinaryOpKind::Rem
    };
    (or) => {
        $crate::common::BinaryOpKind::Or
    };
    (<<) => {
        $crate::common::BinaryOpKind::Shl
    };
    (>>) => {
        $crate::common::BinaryOpKind::Shr
    };
    (^) => {
        $crate::common::BinaryOpKind::BitXor
    };
    (&) => {
        $crate::common::BinaryOpKind::BitAnd
    };
    (>) => {
        $crate::common::BinaryOpKind::Greater
    };
    (>=) => {
        $crate::common::BinaryOpKind::GreaterEqual
    };
    (<) => {
        $crate::common::BinaryOpKind::Less
    };
    (<=) => {
        $crate::common::BinaryOpKind::LessEqual
    };
    (!=) => {
        $crate::common::BinaryOpKind::NotEqual
    };
    (==) => {
        $crate::common::BinaryOpKind::Equal
    };
    (and) => {
        $crate::common::BinaryOpKind::And
    };
    (|) => {
        $crate::common::BinaryOpKind::BitOr
    };
    (+=) => {
        $crate::common::BinaryOpKind::AddAssign
    };
    (-=) => {
        $crate::common::BinaryOpKind::SubAssign
    };
    (*=) => {
        $crate::common::BinaryOpKind::MulAssign
    };
    (/=) => {
        $crate::common::BinaryOpKind::DivAssign
    };
    ("//=") => {
        $crate::common::BinaryOpKind::DivIntAssign
    };
    (%=) => {
        $crate::common::BinaryOpKind::RemAssign
    };
    (^=) => {
        $crate::common::BinaryOpKind::BitXorAssign
    };
    (&=) => {
        $crate::common::BinaryOpKind::BitAndAssign
    };
    (|=) => {
        $crate::common::BinaryOpKind::BitOrAssign
    };
    (<<=) => {
        $crate::common::BinaryOpKind::ShlAssign
    };
    (>>=) => {
        $crate::common::BinaryOpKind::ShrAssign
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
        matches!(self, B![+] | B![-] | B!["//"] | B![/] | B![*] | B![%])
    }
}

impl std::fmt::Display for BinaryOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            B![+] => write!(f, "+"),
            B![*] => write!(f, "*"),
            B![/] => write!(f, "/"),
            B!["//"] => write!(f, "//"),
            B![%] => write!(f, "%"),
            B![or] => write!(f, "or"),
            B![<<] => write!(f, "<<"),
            B![>>] => write!(f, ">>"),
            B![^] => write!(f, "^"),
            B![&] => write!(f, "&"),
            B![-] => write!(f, "-"),
            B![>] => write!(f, ">"),
            B![>=] => write!(f, ">="),
            B![<] => write!(f, "<"),
            B![<=] => write!(f, "<="),
            B![!=] => write!(f, "!="),
            B![==] => write!(f, "=="),
            B![and] => write!(f, "and"),
            B![|] => write!(f, "|"),
            B![+=] => write!(f, "+="),
            B![-=] => write!(f, "-="),
            B![*=] => write!(f, "*="),
            B![/=] => write!(f, "/="),
            B!["//="] => write!(f, "//="),
            B![%=] => write!(f, "%="),
            B![^=] => write!(f, "^="),
            B![&=] => write!(f, "&="),
            B![|=] => write!(f, "|="),
            B![<<=] => write!(f, "<<="),
            B![>>=] => write!(f, ">>="),
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum PathNodeType {
    Expr,
    Type,
    Pat,
    Record,
}
