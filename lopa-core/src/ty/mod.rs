pub mod infer;

use std::sync::Arc;

use ustr::Ustr;

use crate::{common::LitKind, def::ir};

#[salsa::tracked(debug)]
pub struct Param<'db> {
    pub name: Option<Ustr>,
    pub ty: Type<'db>,
}

#[derive(Debug, Clone, PartialEq, Eq, salsa::Update, Hash)]
pub enum Type<'db> {
    Unknown,
    Unit,
    Any,
    Nilable(Box<Type<'db>>),
    Lit(LitKind),
    BareFunction {
        params: Vec<Param<'db>>,
        return_type: Option<Box<Type<'db>>>,
    },
    Function(ir::Function<'db>),
    Struct(ir::Struct<'db>),
}

impl<'db> Type<'db> {
    pub fn nilable(&self) -> bool {
        matches!(self, Self::Nilable(_))
    }
}
