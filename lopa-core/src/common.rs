#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum LitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
}
