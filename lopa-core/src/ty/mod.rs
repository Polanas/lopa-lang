#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Unknown,
    Nilable(Box<Self>),
    Lit(LitType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LitType {
    Float,
    Int,
    String,
    Bool,
}
