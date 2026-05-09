use ustr::Ustr;

use crate::{ide, parsing::ast::{self, LiteralKind}, ty};

#[salsa::tracked]
pub struct Test<'db> {
    pub name: String,
}

#[salsa::tracked]
pub struct Function<'db> {
    pub name: Ustr,
    pub params: Vec<FnParam<'db>>,
    pub output: Option<TypeExpr>,
    pub node_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
}

#[salsa::tracked]
pub struct FnParam<'db> {
    pub name: Ustr,
    pub ty: TypeExpr,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone)]
pub enum TypeExpr {
    NameType(NameType),
    NilableType(NilableType),
    LitType(LitType),
    AnyType(AnyType),
}

#[derive(salsa::Update, PartialEq, Eq, Hash, Clone)]
pub struct NameType {
    pub value: Ustr,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone)]
pub struct NilableType {
    pub value: Box<TypeExpr>,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone)]
pub struct LitType {
    pub kind: LiteralKind,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone)]
pub struct AnyType {}
