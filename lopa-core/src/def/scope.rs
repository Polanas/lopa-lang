use crate::{
    def::ir,
    parsing::ast::{self},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, salsa::Update)]
pub struct MyAstPtr<T: rowan::ast::AstNode + 'static>(ast::AstPtr<T>);

#[derive(Debug, Default, PartialEq, Eq, salsa::Update)]
pub struct FileSourceMap<'db> {
    fucntions: indexmap::IndexMap<MyAstPtr<ast::FnItem>, ir::Function<'db>>,
}
