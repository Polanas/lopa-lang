use ustr::Ustr;

use crate::{parsing::ast::SyntaxNodePtr, ty};

#[salsa::tracked(debug)]
#[derive(PartialOrd, Ord)]
pub struct Function<'db> {
    pub name: Ustr,
    pub params: Vec<FnParam>,
    pub node_ptr: SyntaxNodePtr,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct FnParam {
    pub name: Ustr,
    pub label: Option<Ustr>,
}

impl FnParam {
    pub fn ty(&self, db: &dyn salsa::Database) -> ty::Type {
        todo!()
    }
}
