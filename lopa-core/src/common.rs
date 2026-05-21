#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
}
