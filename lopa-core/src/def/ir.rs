use ustr::Ustr;

use crate::{
    ide,
    parsing::ast::{self, LiteralKind},
};

#[salsa::tracked(debug)]
pub struct Function<'db> {
    pub name: Ustr,
    pub params: Vec<FnParam<'db>>,
    pub output: Option<TypeExpr>,
    pub node_ptr: ast::AstPtr<ast::FnItem>,
    pub file: ide::File,
}

#[salsa::tracked(debug)]
pub struct FnParam<'db> {
    pub name: Ustr,
    pub ty: TypeExpr,
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub enum TypeExpr {
    PathType(PathType),
    NilableType(NilableType),
    LitType(LitType),
    AnyType(AnyType),
}

#[derive(salsa::Update, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Path {
    pub segments: Vec<Ustr>,
}

#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct PathType {
    pub value: Path,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct NilableType {
    pub value: Box<TypeExpr>,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct LitType {
    pub kind: LiteralKind,
}
#[derive(salsa::Update, PartialEq, Eq, Hash, Clone, Debug)]
pub struct AnyType {}
