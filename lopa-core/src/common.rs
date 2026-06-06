use crate::parsing::ast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum LitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub struct MyAstPtr<T: rowan::ast::AstNode + 'static>(pub ast::AstPtr<T>);
