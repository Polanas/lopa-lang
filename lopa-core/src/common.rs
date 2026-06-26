use crate::parsing::ast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, salsa::Update)]
pub enum LitKind {
    Float,
    Int,
    String,
    Bool,
    Nil,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub struct MyAstPtr<T: rowan::ast::AstNode + 'static>(pub ast::AstPtr<T>);
